# Tauri Phase 8: UI-Based Setup Flow

**Status:** Not Started
**Dependencies:** Phase 7
**Output:** Setup commands in Rust, SetupView component in frontend

---

## Purpose

Replace terminal stdin prompts with an in-app setup flow. Required for Android (no terminal) and better UX on desktop too. The app launches instantly — if setup is incomplete, the frontend shows a minimal setup screen instead of the chat view.

---

## Design

Three sequential steps, shown only when needed:

```
Step 1: Config         Step 2: Passphrase      Step 3: Telegram Auth
┌──────────────────┐   ┌──────────────────┐    ┌──────────────────┐
│                  │   │                  │    │                  │
│  API ID: [____]  │   │                  │    │  Phone: [______] │
│  API Hash:[____] │   │  Passphrase:     │    │  [Send Code]     │
│  Name:   [____]  │   │  [••••••••••]    │    │                  │
│                  │   │                  │    │  Code: [______]  │
│  [Continue]      │   │  [Unlock]        │    │  [Verify]        │
│                  │   │                  │    │                  │
└──────────────────┘   └──────────────────┘    └──────────────────┘
```

- Step 1 shown only on first run (no config / api_id == 0)
- Step 2 shown every launch (passphrase unlocks keyring)
- Step 3 shown only when Telegram session doesn't exist or is expired

After all steps: setup view disappears, chat view appears.

---

## Rust Changes

### Remove from `main.rs`
- Remove all `prompt()` calls and `rpassword` usage
- App starts with NO stdin interaction
- `VeilState` is created in a "pending" state — store and bridge are initialized lazily after the frontend provides credentials

### New state model

```rust
pub struct VeilState {
    pub config: Mutex<VeilConfig>,
    pub config_path: PathBuf,
    pub veil_dir: PathBuf,
    // These are None until setup completes
    pub store: Mutex<Option<IdentityStore>>,
    pub bridge: Mutex<Option<TelegramBridge>>,
    pub pending_pairing: Mutex<Option<QrPayload>>,
    pub message_tx: mpsc::UnboundedSender<IncomingMessage>,
}
```

### New Tauri commands

```rust
/// Check what setup step is needed.
/// Returns: "needs_config" | "needs_passphrase" | "needs_telegram_auth" | "ready"
#[tauri::command]
async fn get_setup_status(state: State<'_, VeilState>) -> Result<String, String>

/// Save API credentials and display name (step 1).
#[tauri::command]
async fn submit_config(
    state: State<'_, VeilState>,
    api_id: i32,
    api_hash: String,
    display_name: String,
) -> Result<(), String>

/// Unlock the keyring with passphrase, initialize bridge (step 2).
/// After this, checks if Telegram auth is needed.
#[tauri::command]
async fn submit_passphrase(
    state: State<'_, VeilState>,
    passphrase: String,
) -> Result<String, String>  // returns next status

/// Request Telegram login code (step 3a).
#[tauri::command]
async fn request_telegram_code(
    state: State<'_, VeilState>,
    phone: String,
) -> Result<(), String>

/// Submit Telegram login code (step 3b).
#[tauri::command]
async fn submit_telegram_code(
    state: State<'_, VeilState>,
    code: String,
) -> Result<String, String>  // returns "ready" or "needs_2fa"

/// Submit 2FA password if enabled (step 3c, optional).
#[tauri::command]
async fn submit_2fa_password(
    state: State<'_, VeilState>,
    password: String,
) -> Result<(), String>
```

### Updated `main.rs`

```rust
fn main() {
    sodiumoxide::init().expect("sodiumoxide::init() failed");

    let veil_dir = dirs::home_dir().unwrap().join(".veil");
    let config_path = veil_dir.join("config.toml");
    let config = VeilConfig::load(&config_path);

    let (message_tx, message_rx) = mpsc::unbounded_channel();

    let veil_state = VeilState {
        config: Mutex::new(config),
        config_path,
        veil_dir,
        store: Mutex::new(None),      // initialized after passphrase
        bridge: Mutex::new(None),     // initialized after passphrase
        pending_pairing: Mutex::new(None),
        message_tx,
    };

    tauri::Builder::default()
        .manage(veil_state)
        .invoke_handler(tauri::generate_handler![
            // Setup commands
            get_setup_status,
            submit_config,
            submit_passphrase,
            request_telegram_code,
            submit_telegram_code,
            submit_2fa_password,
            // App commands (only work after setup)
            list_contacts,
            send_message,
            initiate_pairing,
            complete_pairing,
            update_envelope,
            set_theme,
        ])
        .setup(|app| {
            // Spawn incoming message handler (reads when bridge is connected)
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(incoming) = message_rx.recv().await {
                    let state = app_handle.state::<VeilState>();
                    handle_incoming_message(&app_handle, &state, incoming.channel_id, &incoming.text).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Veil");
}
```

### Updated existing commands

Existing commands (`list_contacts`, `send_message`, etc.) need to handle the `Option` wrappers:
```rust
let store = state.store.lock().await;
let store = store.as_ref().ok_or("not initialized")?;
```

---

## Frontend Changes

### `src/lib/api/tauri.ts` — add setup functions

```typescript
export async function getSetupStatus(): Promise<string> { return invoke('get_setup_status'); }
export async function submitConfig(apiId: number, apiHash: string, displayName: string): Promise<void> {
    return invoke('submit_config', { apiId, apiHash, displayName });
}
export async function submitPassphrase(passphrase: string): Promise<string> {
    return invoke('submit_passphrase', { passphrase });
}
export async function requestTelegramCode(phone: string): Promise<void> {
    return invoke('request_telegram_code', { phone });
}
export async function submitTelegramCode(code: string): Promise<string> {
    return invoke('submit_telegram_code', { code });
}
export async function submit2faPassword(password: string): Promise<void> {
    return invoke('submit_2fa_password', { password });
}
```

### `src/lib/components/SetupView.svelte`

Single component with reactive step progression. No wizard chrome — just centered fields on the dark background with minimal labeling.

```
- Step "config": three inputs (API ID, API Hash, Display Name) + continue button
- Step "passphrase": one password input + unlock button
- Step "phone": phone number input + send code button
- Step "code": code input + verify button
- Step "2fa": password input + submit button
```

Each step calls the corresponding command. On success, advances to next step (or transitions to main app).

Styling: same art nouveau variables. Centered vertically and horizontally. Inputs are minimal — bottom-border only, warm text on dark background. No labels — placeholder text only.

### `src/routes/+page.svelte` — gate on setup

```svelte
<script>
  let setupComplete = false;
  let setupStatus = 'loading';

  onMount(async () => {
    setupStatus = await getSetupStatus();
    if (setupStatus === 'ready') {
      setupComplete = true;
      // load contacts, set up listeners...
    }
  });

  function onSetupDone() {
    setupComplete = true;
    // load contacts, set up listeners...
  }
</script>

{#if setupComplete}
  <!-- normal app -->
{:else}
  <SetupView status={setupStatus} on:complete={onSetupDone} />
{/if}
```

---

## Acceptance Criteria

- [ ] App launches with no terminal interaction
- [ ] `get_setup_status` correctly detects missing config, missing store, missing telegram auth
- [ ] Config step saves API credentials and display name
- [ ] Passphrase step initializes IdentityStore and TelegramBridge
- [ ] Telegram auth step handles phone → code → (optional 2FA) flow
- [ ] After setup completes, main chat view appears with contacts loaded
- [ ] Subsequent launches skip config step, show only passphrase
- [ ] Setup UI is minimal — no wizard chrome, no hand-holding, just fields
- [ ] `cargo build` and `npm run build` both succeed
- [ ] All existing `cargo test` tests still pass
