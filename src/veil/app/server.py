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
