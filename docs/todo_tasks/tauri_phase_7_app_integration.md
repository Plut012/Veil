# Tauri Phase 7: App Integration + Frontend Adaptation

**Status:** Not Started
**Dependencies:** All previous phases (2-6)
**Output:** `src-tauri/src/app/`, updated frontend with Tauri IPC

---

## Purpose

Wire all Rust modules together through Tauri commands and events. Replace the frontend's WebSocket layer with Tauri's IPC (`invoke` + `listen`). This is the final phase — after this, Veil is a working native app.

---

## Dependencies (Cargo.toml)

```toml
toml = "0.8"
```

---

## Architecture: Tauri IPC replaces WebSocket

**Before (Python):**
```
Frontend → WebSocket → FastAPI → Python modules
```

**After (Rust):**
```
Frontend → invoke("send_message") → Tauri Command → Rust modules
Frontend ← listen("veil://message") ← Tauri Event ← Rust modules
```

No server, no connection state, no reconnect logic. The frontend calls Rust directly.

---

## Rust Files

### `src-tauri/src/app/config.rs`

Port of VeilConfig. Same TOML format as Python version for compatibility.

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: String,
    pub session_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeilConfig {
    pub display_name: String,
    pub envelope_template: String,
    pub theme: String,
    pub telegram: TelegramConfig,
}

impl VeilConfig {
    pub fn load(path: &Path) -> Self { ... }
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> { ... }
}

impl Default for VeilConfig {
    fn default() -> Self {
        Self {
            display_name: "Veil User".into(),
            envelope_template: String::new(),
            theme: "art-nouveau".into(),
            telegram: TelegramConfig {
                api_id: 0,
                api_hash: String::new(),
                session_path: dirs::home_dir()
                    .unwrap()
                    .join(".veil/telegram.session")
                    .to_string_lossy()
                    .into(),
            },
        }
    }
}
```

### `src-tauri/src/app/state.rs`

Shared application state, managed by Tauri.

```rust
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

use crate::app::config::VeilConfig;
use crate::identity::IdentityStore;
use crate::bridges::TelegramBridge;
use crate::identity::pairing::QrPayload;

pub struct VeilState {
    pub config: Mutex<VeilConfig>,
    pub store: Mutex<IdentityStore>,
    pub bridge: Mutex<TelegramBridge>,
    pub pending_pairing: Mutex<Option<QrPayload>>,
}
```

### `src-tauri/src/app/commands.rs`

Tauri command handlers — these are the functions the frontend calls via `invoke()`.

```rust
use tauri::{command, AppHandle, State, Emitter};
use crate::app::state::VeilState;

#[derive(serde::Serialize, Clone)]
struct ContactInfo {
    contact_id: String,
    display_name: String,
    telegram_channel_id: i64,
}

#[derive(serde::Serialize, Clone)]
struct MessageEvent {
    contact_id: String,
    text: String,
    direction: String,  // "in" or "out"
    timestamp: String,
}

/// List all contacts
#[command]
async fn list_contacts(state: State<'_, VeilState>) -> Result<Vec<ContactInfo>, String> {
    let store = state.store.lock().await;
    Ok(store.list_contacts().iter().map(|c| ContactInfo {
        contact_id: c.contact_id.clone(),
        display_name: c.display_name.clone(),
        telegram_channel_id: c.telegram_channel_id,
    }).collect())
}

/// Send an encrypted message
#[command]
async fn send_message(
    app: AppHandle,
    state: State<'_, VeilState>,
    contact_id: String,
    text: String,
) -> Result<(), String> {
    // 1. Look up contact
    // 2. Encrypt text with contact's key
    // 3. Wrap in envelope
    // 4. Send via bridge
    // 5. Emit "veil://message" event with direction "out"
    ...
}

/// Initiate pairing (show QR)
#[command]
async fn initiate_pairing(
    state: State<'_, VeilState>,
) -> Result<String, String> {
    // 1. Generate QR payload
    // 2. Store as pending_pairing
    // 3. Set bridge passthrough = true
    // 4. Return base64 PNG image
    ...
}

/// Complete pairing (scan QR — joiner role)
#[command]
async fn complete_pairing(
    app: AppHandle,
    state: State<'_, VeilState>,
    qr_data: String,
) -> Result<ContactInfo, String> {
    // 1. Parse QR payload
    // 2. Create Telegram channel
    // 3. Send handshake
    // 4. Save contact
    // 5. Add to monitored channels
    // 6. Emit "veil://pairing_complete"
    ...
}

/// Update envelope template
#[command]
async fn update_envelope(
    state: State<'_, VeilState>,
    template: String,
) -> Result<(), String> { ... }

/// Set theme
#[command]
async fn set_theme(
    state: State<'_, VeilState>,
    theme_id: String,
) -> Result<(), String> { ... }
```

### `src-tauri/src/main.rs`

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod crypto;
mod identity;
mod bridges;
mod envelope;
mod app;

use app::state::VeilState;
use app::commands::*;

fn main() {
    // Load config
    // Prompt for passphrase (first run: also API credentials)
    // Initialize store, bridge
    // Create VeilState

    tauri::Builder::default()
        .manage(veil_state)
        .invoke_handler(tauri::generate_handler![
            list_contacts,
            send_message,
            initiate_pairing,
            complete_pairing,
            update_envelope,
            set_theme,
        ])
        .setup(|app| {
            // Connect bridge
            // Spawn incoming message handler task
            // The handler reads from mpsc receiver, decrypts, emits Tauri events
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Veil");
}
```

### Incoming Message Handler (in `setup`)

```rust
// Spawn task that reads from bridge's mpsc receiver
let app_handle = app.handle().clone();
tokio::spawn(async move {
    while let Some(incoming) = message_rx.recv().await {
        // Check for pending pairing handshake
        // Otherwise: find contact by channel, unwrap envelope, decrypt
        // Emit "veil://message" event to frontend
        app_handle.emit("veil://message", MessageEvent { ... }).unwrap();
    }
});
```

---

## Frontend Changes

### Replace WebSocket with Tauri IPC

**Delete:** `src/lib/api/websocket.ts`

**Create:** `src/lib/api/tauri.ts`

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Commands (frontend → backend)
export async function listContacts(): Promise<Contact[]> {
    return invoke('list_contacts');
}

export async function sendMessage(contactId: string, text: string): Promise<void> {
    return invoke('send_message', { contactId, text });
}

export async function initiatePairing(): Promise<string> {
    return invoke('initiate_pairing');  // returns base64 QR PNG
}

export async function completePairing(qrData: string): Promise<Contact> {
    return invoke('complete_pairing', { qrData });
}

export async function updateEnvelope(template: string): Promise<void> {
    return invoke('update_envelope', { template });
}

export async function setTheme(themeId: string): Promise<void> {
    return invoke('set_theme', { themeId });
}

// Events (backend → frontend)
export function onMessage(handler: (msg: MessageEvent) => void) {
    return listen('veil://message', (event) => handler(event.payload));
}

export function onPairingComplete(handler: (contact: Contact) => void) {
    return listen('veil://pairing_complete', (event) => handler(event.payload));
}
```

### Update stores

- `connection.ts` — simplify or remove. No WebSocket connection to manage. Tauri is always "connected".
- `contacts.ts` — load via `listContacts()` on mount instead of waiting for WebSocket message.
- `messages.ts` — listen via `onMessage()` instead of WebSocket handler.

### Update components

- `Compose.svelte` — call `sendMessage()` instead of `veilSocket.sendMessage()`
- `PairingModal.svelte` — call `initiatePairing()` / `completePairing()` instead of WebSocket sends
- `SettingsPanel.svelte` — call `updateEnvelope()` / `setTheme()`
- `+page.svelte` — initialize with `listContacts()` on mount, set up `onMessage()` listener

---

## Acceptance Criteria

- [ ] VeilConfig loads/saves TOML (compatible format with Python version)
- [ ] All Tauri commands wired and callable from frontend
- [ ] `list_contacts` returns contact list
- [ ] `send_message` encrypts, wraps, sends via bridge, emits "out" event
- [ ] Incoming messages decrypted and emitted as "veil://message" events
- [ ] `initiate_pairing` returns QR PNG, sets passthrough
- [ ] `complete_pairing` creates channel, sends handshake, saves contact
- [ ] Pairing handshake detection works for initiator role
- [ ] Frontend uses `invoke()` / `listen()` — no WebSocket
- [ ] App compiles and runs: `cargo tauri dev`
- [ ] Theme switching works
- [ ] Envelope template updates persist
