# Veil — Development Rules

## What This Is
A locally-running encrypted messaging wrapper. Encrypts messages before they hit the platform, decrypts on receive. The platform is a dumb pipe. Telegram bridge first, others later.

## Stack
- **Backend:** Python 3.12+, FastAPI, WebSockets, Telethon, uvicorn
- **Crypto:** PyNaCl (libsodium) — XChaCha20-Poly1305
- **Frontend:** Svelte + TypeScript, Vite
- **Package mgmt:** uv (backend), npm (frontend)
- **No database.** Keys and contacts stored as encrypted local files.

## Key Docs
- [docs/overview.md](docs/overview.md) — Vision, principles, scope
- [docs/architecture.md](docs/architecture.md) — Full technical design

## Core Rules

- **Simple, robust, clever — in that order.** Always.
- **Plaintext never leaves the machine.** The crypto module is a hard boundary. Everything crossing a network is ciphertext in an envelope.
- **Bridges are dumb pipes.** A bridge moves envelopes. It never sees, touches, or logs plaintext.
- **Crypto is pure.** No side effects, no platform awareness. Two functions and a key generator.
- **UI is intuitive, not hand-holding.** No wizards, no tooltips for obvious actions. The interface teaches through form. If it needs a label, redesign it.
- **Themes are visual, not structural.** CSS-level theming. The messaging UI structure stays stable across themes.
- **Modules are independent.** Crypto knows nothing about Telegram. Bridges know nothing about encryption. Envelopes know nothing about UI. Each module has a clean interface boundary.
- **Keys are ephemeral in transit.** QR codes containing keys exist only on screen during pairing. Never persisted, never transmitted digitally.
- **Keys are encrypted at rest.** The local keyring and Telegram session are encrypted with a user passphrase.

## What NOT to Build
- No cloud sync or remote storage
- No account system or user registration
- No message persistence (platform stores ciphertext, Veil is stateless)
- No group encryption until 1-to-1 is rock solid
- No additional bridges until Telegram bridge is complete
- No asymmetric crypto / Double Ratchet until symmetric MVP is proven
- No mobile until desktop is solid
- No onboarding tutorials or guided setup flows

## Code Style
- Python: dataclasses over dicts, type hints everywhere, no classes where functions suffice
- TypeScript: strict mode, types mirroring server models
- Svelte: one component per file, props over context, scoped CSS
- WebSocket messages: JSON with `type` field, validated on receive
- Tests: real crypto operations, no mocking the crypto engine

## Directory Reference
```
src/veil/
  crypto/          — encrypt/decrypt, key generation
  identity/        — contacts, pairing, encrypted keyring
  bridges/         — platform integrations (Telegram)
  envelope/        — configurable ciphertext wrapping
  app/             — FastAPI server, WebSocket API
frontend/
  src/lib/
    components/    — Svelte UI components
    stores/        — reactive state
    api/           — WebSocket client
themes/            — CSS themes (art-nouveau first)
docs/              — overview, architecture
```
