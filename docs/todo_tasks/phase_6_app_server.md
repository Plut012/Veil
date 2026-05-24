# Phase 6: App Server

**Status:** Not Started
**Dependencies:** Phase 1-5 (all backend modules)
**Output:** `src/veil/app/server.py`, `src/veil/app/config.py`, `tests/test_server.py`

---

## Purpose

The app server is the orchestrator. It connects all modules — crypto, identity, bridge, envelope — and exposes them to the frontend via a WebSocket API. It also serves the built Svelte frontend as static files.

This is the only module that knows about all other modules. Everything else is decoupled.

---

## Files

### `src/veil/app/config.py`

Configuration loading from `~/.veil/config.toml`.

```python
from dataclasses import dataclass, field
from pathlib import Path
import tomli
import tomli_w

VEIL_DIR = Path.home() / ".veil"
CONFIG_PATH = VEIL_DIR / "config.toml"


@dataclass
class TelegramConfig:
    api_id: int = 0
    api_hash: str = ""
    session_path: str = str(VEIL_DIR / "telegram.session")


@dataclass
class VeilConfig:
    telegram: TelegramConfig = field(default_factory=TelegramConfig)
    envelope_template: str = ""
    display_name: str = "Veil User"
    theme: str = "art-nouveau"
    host: str = "127.0.0.1"
    port: int = 8900

    @classmethod
    def load(cls, path: Path = CONFIG_PATH) -> "VeilConfig":
        """Load config from TOML file. Returns defaults if file doesn't exist."""
        if not path.exists():
            return cls()
        with open(path, "rb") as f:
            data = tomli.load(f)
        config = cls()
        if "telegram" in data:
            config.telegram = TelegramConfig(**data["telegram"])
        config.envelope_template = data.get("envelope_template", "")
        config.display_name = data.get("display_name", "Veil User")
        config.theme = data.get("theme", "art-nouveau")
        config.host = data.get("host", "127.0.0.1")
        config.port = data.get("port", 8900)
        return config

    def save(self, path: Path = CONFIG_PATH) -> None:
        """Write current config to TOML file."""
        data = {
            "display_name": self.display_name,
            "envelope_template": self.envelope_template,
            "theme": self.theme,
            "host": self.host,
            "port": self.port,
            "telegram": {
                "api_id": self.telegram.api_id,
                "api_hash": self.telegram.api_hash,
                "session_path": self.telegram.session_path,
            },
        }
        path.parent.mkdir(mode=0o700, exist_ok=True)
        with open(path, "wb") as f:
            tomli_w.dump(data, f)
```

### `src/veil/app/server.py`

The main server. Connects everything.

```python
import asyncio
import json
import logging
import base64
from pathlib import Path
from contextlib import asynccontextmanager

from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.staticfiles import StaticFiles

from veil.app.config import VeilConfig
from veil.crypto import encrypt, decrypt, key_to_base64, key_from_base64
from veil.identity import Contact, IdentityStore
from veil.identity.pairing import (
    initiate_pairing,
    complete_pairing_as_joiner,
    complete_pairing_as_initiator,
    parse_handshake_message,
    QRPayload,
)
from veil.bridges.telegram import TelegramBridge
from veil.envelope import wrap, unwrap

logger = logging.getLogger(__name__)


class VeilApp:
    """Main application. Wires all modules together."""

    def __init__(self, config: VeilConfig, passphrase: str):
        self.config = config
        self.store = IdentityStore(passphrase)
        self.bridge = TelegramBridge(
            api_id=config.telegram.api_id,
            api_hash=config.telegram.api_hash,
            session_path=Path(config.telegram.session_path),
            monitored_channels={
                c.telegram_channel_id for c in self.store.list_contacts()
            },
        )
        self._pending_pairing: QRPayload | None = None
        self._ws_clients: set[WebSocket] = set()

    async def start(self) -> None:
        """Connect bridge and register message handler."""
        await self.bridge.connect()
        await self.bridge.on_receive(self._handle_incoming)

    async def stop(self) -> None:
        """Disconnect bridge."""
        await self.bridge.disconnect()

    async def _handle_incoming(self, channel_id: int, message_text: str) -> None:
        """
        Handle an incoming message from Telegram.

        1. Check if it's a handshake (if we have a pending pairing)
        2. Otherwise, try to unwrap and decrypt as a normal message
        3. Forward decrypted plaintext to connected WebSocket clients
        """
        # Check for pending pairing handshake
        if self._pending_pairing:
            handshake = parse_handshake_message(
                message_text, self._pending_pairing.key
            )
            if handshake:
                contact = await complete_pairing_as_initiator(
                    self._pending_pairing, channel_id, handshake,
                    self.store, self.bridge,
                )
                self._pending_pairing = None
                await self._broadcast({
                    "type": "pairing_complete",
                    "contact": {
                        "contact_id": contact.contact_id,
                        "display_name": contact.display_name,
                        "telegram_channel_id": contact.telegram_channel_id,
                    },
                })
                return

        # Normal message — find contact by channel
        contact = self.store.get_contact_by_channel(channel_id)
        if not contact:
            return  # unknown channel, ignore

        # Collect known templates for unwrapping
        templates = [contact.envelope_template]

        # Try to extract ciphertext
        ciphertext_b64 = unwrap(message_text, templates)
        if ciphertext_b64 is None:
            return  # not a Veil message

        # Decrypt
        try:
            sealed = base64.urlsafe_b64decode(ciphertext_b64.encode("ascii"))
            plaintext = decrypt(sealed, contact.key)
            text = plaintext.decode("utf-8")
        except Exception:
            logger.warning(f"Failed to decrypt message from {contact.display_name}")
            return

        # Forward to UI
        await self._broadcast({
            "type": "message",
            "contact_id": contact.contact_id,
            "text": text,
            "direction": "in",
        })

    async def send_message(self, contact_id: str, text: str) -> None:
        """Encrypt and send a message to a contact."""
        contact = self.store.get_contact(contact_id)
        if not contact:
            raise ValueError(f"Unknown contact: {contact_id}")

        # Encrypt
        plaintext = text.encode("utf-8")
        sealed = encrypt(plaintext, contact.key)
        ciphertext_b64 = base64.urlsafe_b64encode(sealed).decode("ascii")

        # Wrap in envelope
        envelope = wrap(ciphertext_b64, self.config.envelope_template)

        # Send via bridge
        await self.bridge.send(contact.telegram_channel_id, envelope)

        # Echo to UI
        await self._broadcast({
            "type": "message",
            "contact_id": contact_id,
            "text": text,
            "direction": "out",
        })

    async def _broadcast(self, message: dict) -> None:
        """Send a JSON message to all connected WebSocket clients."""
        data = json.dumps(message)
        disconnected = set()
        for ws in self._ws_clients:
            try:
                await ws.send_text(data)
            except Exception:
                disconnected.add(ws)
        self._ws_clients -= disconnected

    async def handle_websocket(self, ws: WebSocket) -> None:
        """Handle a WebSocket connection from the frontend."""
        await ws.accept()
        self._ws_clients.add(ws)

        try:
            # Send initial state
            await ws.send_text(json.dumps({
                "type": "connected",
                "status": "ok",
            }))
            await ws.send_text(json.dumps({
                "type": "contacts",
                "contacts": [
                    {
                        "contact_id": c.contact_id,
                        "display_name": c.display_name,
                        "telegram_channel_id": c.telegram_channel_id,
                    }
                    for c in self.store.list_contacts()
                ],
            }))

            # Message loop
            while True:
                data = await ws.receive_text()
                msg = json.loads(data)
                await self._handle_ws_message(msg, ws)

        except WebSocketDisconnect:
            pass
        finally:
            self._ws_clients.discard(ws)

    async def _handle_ws_message(self, msg: dict, ws: WebSocket) -> None:
        """Route a WebSocket message from the frontend."""
        msg_type = msg.get("type")

        if msg_type == "send_message":
            await self.send_message(msg["contact_id"], msg["text"])

        elif msg_type == "list_contacts":
            await ws.send_text(json.dumps({
                "type": "contacts",
                "contacts": [
                    {
                        "contact_id": c.contact_id,
                        "display_name": c.display_name,
                        "telegram_channel_id": c.telegram_channel_id,
                    }
                    for c in self.store.list_contacts()
                ],
            }))

        elif msg_type == "initiate_pairing":
            payload, qr_bytes = await initiate_pairing(
                self.store, self.bridge,
                self.config.display_name,
                self.config.envelope_template,
            )
            self._pending_pairing = payload
            await ws.send_text(json.dumps({
                "type": "pairing_qr",
                "qr_image": base64.b64encode(qr_bytes).decode("ascii"),
            }))

        elif msg_type == "complete_pairing":
            contact = await complete_pairing_as_joiner(
                msg["qr_data"],
                self.store, self.bridge,
                self.config.display_name,
                self.config.envelope_template,
            )
            await self._broadcast({
                "type": "pairing_complete",
                "contact": {
                    "contact_id": contact.contact_id,
                    "display_name": contact.display_name,
                    "telegram_channel_id": contact.telegram_channel_id,
                },
            })

        elif msg_type == "update_envelope":
            self.config.envelope_template = msg["template"]
            self.config.save()

        elif msg_type == "set_theme":
            self.config.theme = msg["theme_id"]
            self.config.save()

        else:
            await ws.send_text(json.dumps({
                "type": "error",
                "message": f"Unknown message type: {msg_type}",
            }))


def create_app(config: VeilConfig, passphrase: str) -> FastAPI:
    """Create the FastAPI application."""
    veil = VeilApp(config, passphrase)

    @asynccontextmanager
    async def lifespan(app: FastAPI):
        await veil.start()
        yield
        await veil.stop()

    app = FastAPI(lifespan=lifespan)

    @app.websocket("/ws")
    async def websocket_endpoint(ws: WebSocket):
        await veil.handle_websocket(ws)

    # Serve built frontend (production)
    frontend_dist = Path(__file__).parent.parent.parent.parent / "frontend" / "dist"
    if frontend_dist.exists():
        app.mount("/", StaticFiles(directory=str(frontend_dist), html=True))

    return app


def main():
    """Entry point: load config, prompt for passphrase, start server."""
    import getpass
    import uvicorn

    config = VeilConfig.load()

    if not config.telegram.api_id:
        print("First run — configure Telegram API credentials.")
        config.telegram.api_id = int(input("API ID: "))
        config.telegram.api_hash = input("API Hash: ")
        config.display_name = input("Your display name: ")
        config.save()

    passphrase = getpass.getpass("Veil passphrase: ")

    app = create_app(config, passphrase)
    uvicorn.run(app, host=config.host, port=config.port)
```

**Implementation notes:**

**Startup flow:**
1. Load `~/.veil/config.toml` (or create defaults on first run)
2. Prompt for passphrase (never stored — only lives in memory)
3. Initialize `IdentityStore` with passphrase (loads and decrypts contacts)
4. Initialize `TelegramBridge` with config values
5. Connect bridge (first time: interactive Telegram auth via terminal)
6. Start FastAPI server on `127.0.0.1:8900`

**Security:**
- Server binds to `127.0.0.1` only — not accessible from the network.
- Passphrase is entered via `getpass` (not echoed to terminal) and held in memory only.
- No CORS needed — frontend is served from the same origin.

**Message flow (outgoing):**
```
Frontend → WebSocket { type: "send_message", contact_id, text }
  → VeilApp.send_message()
    → encrypt(text, contact.key)
    → wrap(ciphertext, template)
    → bridge.send(channel_id, envelope)
    → broadcast echo to UI { type: "message", direction: "out" }
```

**Message flow (incoming):**
```
Telegram API → bridge callback(channel_id, message_text)
  → VeilApp._handle_incoming()
    → unwrap(message_text, templates)
    → decrypt(sealed, contact.key)
    → broadcast to UI { type: "message", direction: "in" }
```

### `src/veil/app/__init__.py`

```python
from veil.app.server import create_app, main
```

### `tests/test_server.py`

```python
"""Tests for app server. Tests config loading and message routing logic."""
import pytest
import json
from pathlib import Path
from veil.app.config import VeilConfig, TelegramConfig


class TestConfig:
    def test_default_config(self):
        config = VeilConfig()
        assert config.host == "127.0.0.1"
        assert config.port == 8900
        assert config.theme == "art-nouveau"

    def test_save_and_load(self, tmp_path):
        path = tmp_path / "config.toml"
        config = VeilConfig(
            display_name="Alice",
            envelope_template="~~ {ciphertext} ~~",
            telegram=TelegramConfig(api_id=123, api_hash="abc"),
        )
        config.save(path)
        loaded = VeilConfig.load(path)
        assert loaded.display_name == "Alice"
        assert loaded.envelope_template == "~~ {ciphertext} ~~"
        assert loaded.telegram.api_id == 123
        assert loaded.telegram.api_hash == "abc"

    def test_load_missing_file(self, tmp_path):
        """Missing config file returns defaults."""
        config = VeilConfig.load(tmp_path / "nonexistent.toml")
        assert config.host == "127.0.0.1"

    def test_binds_localhost_only(self):
        """Default config binds to localhost — not network-accessible."""
        config = VeilConfig()
        assert config.host == "127.0.0.1"
```

---

## Acceptance Criteria

- [ ] `VeilConfig` loads from and saves to TOML file
- [ ] Missing config file returns sensible defaults
- [ ] Server binds to `127.0.0.1` only (not `0.0.0.0`)
- [ ] First-run flow prompts for Telegram API credentials and display name
- [ ] Passphrase is entered via `getpass` and never persisted
- [ ] WebSocket `/ws` endpoint accepts connections and sends initial state
- [ ] `send_message` encrypts, wraps, sends via bridge, and echoes to UI
- [ ] Incoming messages are unwrapped, decrypted, and broadcast to WebSocket clients
- [ ] Pairing initiation generates QR and returns PNG to frontend
- [ ] Pairing completion (both roles) creates contacts and broadcasts confirmation
- [ ] Config updates (envelope template, theme) are persisted to disk
- [ ] All tests pass: `uv run pytest tests/test_server.py -v`
