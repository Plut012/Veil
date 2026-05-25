# Tauri Rewrite — Overview

**Branch:** `tauri`
**Goal:** Rewrite the Python backend in Rust, wrap in Tauri v2 for native desktop + Android + iOS.

## Why

- **Distribution:** Download and run. No Python, no terminal, no setup.
- **Mobile:** Tauri v2 supports Android and iOS natively.
- **Security:** Rust's memory safety is a genuine advantage for crypto software.
- **Simplicity:** Single binary. No runtime dependencies.
- **Frontend stays the same:** Tauri uses system WebView — the Svelte app is unchanged structurally, just uses Tauri IPC instead of WebSocket.

## Architecture Change

**Before (Python):**
```
Svelte ←→ WebSocket ←→ FastAPI ←→ Python modules
```

**After (Rust + Tauri):**
```
Svelte ←→ Tauri IPC (invoke/listen) ←→ Rust modules
```

Tauri's IPC is simpler than WebSocket — no connection state, no reconnect logic, no server. The frontend calls Rust functions directly via `invoke()` and receives events via `listen()`.

## Crate Mapping

| Python | Rust crate | Notes |
|--------|-----------|-------|
| PyNaCl (libsodium) | `sodiumoxide` | Same libsodium under the hood — ciphertext compatible |
| Telethon | `grammers-client` | Rust-native Telegram client, user auth |
| FastAPI + WebSocket | Tauri commands + events | Direct IPC, no server |
| tomli / tomli_w | `toml` | Standard Rust TOML |
| qrcode (Python) | `qrcode` (Rust) | PNG generation |
| Pillow | `image` | Image handling for QR |
| argon2id (nacl.pwhash) | `argon2` | RustCrypto implementation |

## Project Structure

```
veil/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs              — Tauri entry point
│   │   ├── lib.rs               — module declarations
│   │   ├── crypto/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs        — encrypt/decrypt (XSalsa20-Poly1305)
│   │   │   └── keys.rs          — key generation, base64 encoding
│   │   ├── identity/
│   │   │   ├── mod.rs
│   │   │   ├── contact.rs       — Contact struct
│   │   │   ├── store.rs         — encrypted keyring
│   │   │   └── pairing.rs       — QR ceremony
│   │   ├── bridges/
│   │   │   ├── mod.rs
│   │   │   ├── traits.rs        — Bridge trait
│   │   │   └── telegram.rs      — grammers implementation
│   │   ├── envelope/
│   │   │   ├── mod.rs
│   │   │   └── format.rs        — wrap/unwrap
│   │   └── app/
│   │       ├── mod.rs
│   │       ├── config.rs        — VeilConfig (TOML)
│   │       ├── commands.rs      — Tauri command handlers
│   │       └── state.rs         — shared app state
│   └── icons/
├── frontend/                    — existing Svelte (adapted to Tauri IPC)
├── docs/
├── themes/
└── CLAUDE.md
```

## Phase Order

| Phase | Component | Depends on | Can parallel? |
|-------|-----------|-----------|---------------|
| 1 | Tauri project setup | — | — |
| 2 | Crypto engine | 1 | Yes (with 3) |
| 3 | Envelope | 1 | Yes (with 2) |
| 4 | Identity store | 2 | No |
| 5 | Telegram bridge | 1 | Yes (with 2,3,4) |
| 6 | Pairing ceremony | 2, 4, 5 | No |
| 7 | Tauri app + frontend | All | No |

## Ciphertext Compatibility

Both `sodiumoxide` and PyNaCl use libsodium's `crypto_secretbox`. The sealed format is identical: `nonce (24 bytes) + ciphertext + MAC (16 bytes)`. Messages encrypted by the Python version can be decrypted by the Rust version and vice versa — important for mixed-platform usage during transition.
