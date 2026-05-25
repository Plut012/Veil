# Veil — Development Rules

## What This Is
A locally-running encrypted messaging wrapper. Encrypts messages before they hit the platform, decrypts on receive. The platform is a dumb pipe. Telegram bridge first, others later.

## Stack
- **Backend:** Rust (Tauri v2), tokio async runtime
- **Crypto:** sodiumoxide (libsodium) — XSalsa20-Poly1305
- **Frontend:** Svelte + TypeScript, Vite (SvelteKit static adapter)
- **IPC:** Tauri commands (`invoke`) and events (`listen`) — no WebSocket
- **Package mgmt:** cargo (backend), npm (frontend)
- **No database.** Keys and contacts stored as encrypted local files.

## Key Docs
- [docs/overview.md](docs/overview.md) — Vision, principles, scope
- [docs/architecture.md](docs/architecture.md) — Full technical design
- [docs/todo_tasks/tauri_rewrite_overview.md](docs/todo_tasks/tauri_rewrite_overview.md) — Tauri rewrite plan

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
- Rust: use `thiserror` for error types, `serde` for serialization, no `unwrap()` in library code
- TypeScript: strict mode, types mirroring Rust command return types
- Svelte: one component per file, props over context, scoped CSS
- Tauri commands: return `Result<T, String>` for frontend compatibility
- Tests: real crypto operations, no mocking the crypto engine

## Architecture

**IPC pattern:**
```
Svelte ←→ Tauri IPC (invoke/listen) ←→ Rust modules
```

**Crate mapping:**
| Python (old) | Rust crate | Notes |
|---|---|---|
| PyNaCl (libsodium) | `sodiumoxide` | Same libsodium — ciphertext compatible |
| Telethon | `grammers-client` | Rust-native Telegram client |
| FastAPI + WebSocket | Tauri commands + events | Direct IPC, no server |
| tomli / tomli_w | `toml` | Standard Rust TOML |
| argon2id | `argon2` | RustCrypto implementation |

## Directory Reference
```
src-tauri/
  src/
    main.rs          — Tauri entry point
    lib.rs           — module declarations
    crypto/          — encrypt/decrypt, key generation (Phase 2)
    identity/        — contacts, pairing, encrypted keyring (Phase 4)
    bridges/         — platform integrations / Telegram (Phase 5)
    envelope/        — ciphertext wrapping format (Phase 3)
    app/             — Tauri commands, shared state, config (Phase 7)
  Cargo.toml
  tauri.conf.json
  build.rs
frontend/
  src/lib/
    components/      — Svelte UI components
    stores/          — reactive state
    api/             — Tauri IPC client wrappers
themes/              — CSS themes (art-nouveau first)
docs/                — overview, architecture, phase plans
```
