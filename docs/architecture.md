# Veil — Architecture

## Stack

| Layer | Tech | Why |
|-------|------|-----|
| Backend | Python + FastAPI + WebSockets | Proven stack, fast iteration, async-native |
| Telegram | Telethon | Most mature Python Telegram client, full user API |
| Crypto | PyNaCl (libsodium) | Audited, simple, correct — XChaCha20-Poly1305 |
| QR | qrcode + OpenCV/pyzbar | Generation and scanning |
| Frontend | Svelte + TypeScript | Lightweight, excellent transitions, compiles to vanilla JS |
| Package mgmt | uv (Python), npm (JS) | Standard tooling |
| Dev | uvicorn (backend), Vite (frontend) | Hot reload on both sides |

No database. No cloud. Contacts and keys stored as encrypted local files.

## Core Principle: Plaintext Never Leaves

The crypto engine is a hard boundary. Plaintext exists only in memory on your machine, between the UI and the crypto module. Everything that crosses a network is ciphertext wrapped in an envelope.

```
                        YOUR MACHINE
  ┌──────────────────────────────────────────────────┐
  │                                                  │
  │   ┌────────┐  plaintext  ┌────────┐  ciphertext │    envelope     ┌──────────┐
  │   │   UI   │────────────▶│ Crypto │────────────▶│───────────────▶│ Telegram │
  │   │(Svelte)│◀────────────│ Engine │◀────────────│◀───────────────│   API    │
  │   └────────┘  plaintext  └────────┘  ciphertext │    envelope     └──────────┘
  │       ▲                      ▲                   │
  │       │ WebSocket            │ keys              │
  │       ▼                      ▼                   │
  │   ┌────────┐            ┌────────┐               │
  │   │ FastAPI│            │Keyring │               │
  │   │ Server │            │(local) │               │
  │   └────────┘            └────────┘               │
  │                                                  │
  └──────────────────────────────────────────────────┘
```

## Module Boundaries

### Crypto (`src/veil/crypto/`)

Pure encryption logic. No side effects, no platform awareness.

- `engine.py` — encrypt/decrypt with XChaCha20-Poly1305, per-message random nonce
- `keys.py` — 256-bit symmetric key generation, key serialization

The crypto module exposes two functions and a key generator. That's it.

```python
def encrypt(plaintext: bytes, key: bytes) -> bytes:
    """Returns nonce + ciphertext + auth tag."""

def decrypt(sealed: bytes, key: bytes) -> bytes:
    """Verifies auth tag, returns plaintext. Raises on tamper."""

def generate_key() -> bytes:
    """Returns 256-bit random symmetric key."""
```

### Identity (`src/veil/identity/`)

Key storage, contact management, and the pairing ceremony.

- `contact.py` — contact model: display name, encryption key, telegram channel ID
- `pairing.py` — QR code generation/scanning, ceremony orchestration
- `store.py` — encrypted local storage for contacts and keys

**Pairing Ceremony (in person, one scan):**

```
Alice: "New Contact"
  → Generates 256-bit symmetric key
  → Generates QR: { key, telegram_user_id, display_name }
  → Displays QR on screen (ephemeral — dismissed after scan)

Bob: "Pair" → scans QR
  → Saves key + Alice's info
  → Creates Telegram group, adds Alice
  → Sends encrypted handshake: { bob_display_name, confirmation }

Alice's Veil: detects new group, decrypts handshake
  → Pairing complete. Channel live.
```

The QR contains the encryption key — it is sensitive. The UI must treat it as ephemeral: display only for scanning, never persist, prompt dismissal.

### Bridges (`src/veil/bridges/`)

Platform integrations. Each bridge implements a simple interface.

- `base.py` — abstract bridge interface
- `telegram.py` — Telethon implementation

```python
class Bridge(Protocol):
    async def send(self, channel_id: int, envelope: str) -> None: ...
    async def on_receive(self, callback: Callable[[int, str], Awaitable[None]]) -> None: ...
    async def create_channel(self, user_ids: list[int], name: str) -> int: ...
    async def connect(self) -> None: ...
    async def disconnect(self) -> None: ...
```

The bridge never sees plaintext. It moves envelopes.

### Envelope (`src/veil/envelope/`)

Configurable ciphertext wrapping. User-defined templates that give encrypted messages personality.

- `format.py` — template-based wrap/parse

```python
# User configures their envelope style:
template = "~~ {ciphertext} ~~"
template = "[ veil ] {ciphertext}"
template = "{ciphertext}"

def wrap(ciphertext_b64: str, template: str) -> str: ...
def unwrap(message: str, known_templates: list[str]) -> str | None: ...
```

The parser tries known templates from paired contacts to extract ciphertext. Unrecognized messages are ignored (not Veil messages).

### App (`src/veil/app/`)

FastAPI server that connects all modules and serves the frontend.

- `server.py` — WebSocket server, routes, lifecycle management

Runs locally. Serves the Svelte frontend and provides a WebSocket API for real-time message flow.

### Frontend (`frontend/`)

Svelte app that renders the messaging UI. Communicates with the backend exclusively via WebSocket.

The frontend is a renderer with input — same philosophy as Dominion. It never touches crypto, keys, or Telegram directly.

### Themes (`themes/`)

CSS-level theming. Each theme is a directory with styles and assets that transform the visual identity without changing component structure.

```
themes/
  art-nouveau/
    theme.css          — colors, typography, spacing, decorative elements
    assets/            — ornamental SVGs, textures, fonts
    theme.json         — { name, description, preview }
```

The messaging UI structure (conversation list, messages, compose) stays stable across themes. Themes change how it looks and feels, not how it's laid out.

## WebSocket Protocol

```
Client → Server:
  { type: "send_message", contact_id: str, text: str }
  { type: "list_contacts" }
  { type: "initiate_pairing" }
  { type: "complete_pairing", qr_data: str }
  { type: "update_envelope", template: str }
  { type: "set_theme", theme_id: str }

Server → Client:
  { type: "message", contact_id: str, text: str, timestamp: str, direction: "in"|"out" }
  { type: "contacts", contacts: [...] }
  { type: "pairing_qr", qr_image: str }
  { type: "pairing_complete", contact: {...} }
  { type: "connected", status: "ok" }
  { type: "error", message: str }
```

## Local Storage

```
~/.veil/
  config.toml          — envelope template, theme, Telegram session path
  keyring/             — encrypted contact keys (one file per contact)
  session.enc          — Telegram session (encrypted at rest with passphrase)
```

No plaintext keys on disk. The keyring is encrypted with a passphrase entered at Veil startup. The Telegram session file is similarly encrypted.

## Directory Structure

```
veil/
├── docs/
│   ├── overview.md
│   └── architecture.md
├── src/
│   └── veil/
│       ├── __init__.py
│       ├── crypto/
│       │   ├── __init__.py
│       │   ├── engine.py        — encrypt/decrypt
│       │   └── keys.py          — key generation, serialization
│       ├── identity/
│       │   ├── __init__.py
│       │   ├── contact.py       — contact model
│       │   ├── pairing.py       — QR ceremony orchestration
│       │   └── store.py         — encrypted local keyring
│       ├── bridges/
│       │   ├── __init__.py
│       │   ├── base.py          — abstract bridge interface
│       │   └── telegram.py      — Telethon implementation
│       ├── envelope/
│       │   ├── __init__.py
│       │   └── format.py        — template wrap/parse
│       └── app/
│           ├── __init__.py
│           └── server.py        — FastAPI + WebSocket server
├── frontend/
│   ├── src/
│   │   ├── lib/
│   │   │   ├── components/      — Chat, Message, Compose, Pairing, ContactList
│   │   │   ├── stores/          — connection, contacts, messages, theme
│   │   │   └── api/
│   │   │       └── websocket.ts — WebSocket client
│   │   ├── routes/
│   │   │   └── +page.svelte
│   │   └── app.css
│   ├── static/
│   └── package.json
├── themes/
│   └── art-nouveau/
│       ├── theme.css
│       ├── theme.json
│       └── assets/
├── CLAUDE.md
├── README.md
├── pyproject.toml
├── title.art
└── .gitignore
```
