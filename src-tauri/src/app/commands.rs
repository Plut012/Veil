use std::collections::HashSet;

use base64::{engine::general_purpose::URL_SAFE, Engine};
use chrono::Utc;
use grammers_client::client::{LoginToken, PasswordToken};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::app::state::VeilState;
use crate::bridges::{Bridge, TelegramBridge};
use crate::crypto::{encrypt, ratchet_encrypt, ratchet_decrypt, init_ratchet};
use crate::envelope::{unwrap, wrap};
use crate::identity::IdentityStore;
use crate::identity::contact::Contact;
use crate::identity::pairing::{build_handshake_message, create_qr_payload, parse_handshake_message};

// ---------------------------------------------------------------------------
// Serializable DTOs sent over IPC
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, Clone)]
pub struct ContactInfo {
    pub contact_id: String,
    pub display_name: String,
    pub telegram_channel_id: i64,
}

#[derive(serde::Serialize, Clone)]
pub struct MessageEvent {
    pub contact_id: String,
    pub text: String,
    pub direction: String, // "in" or "out"
    pub timestamp: String,
}

impl From<&Contact> for ContactInfo {
    fn from(c: &Contact) -> Self {
        ContactInfo {
            contact_id: c.contact_id.clone(),
            display_name: c.display_name.clone(),
            telegram_channel_id: c.telegram_channel_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Setup commands
// ---------------------------------------------------------------------------

/// Return the current setup status.
/// "needs_config"        — API credentials not yet saved
/// "needs_passphrase"    — config present but store not yet unlocked
/// "needs_telegram_auth" — store unlocked but Telegram not yet authenticated
/// "ready"               — fully initialised, chat view can load
#[tauri::command]
pub async fn get_setup_status(state: State<'_, VeilState>) -> Result<String, String> {
    let config = state.config.lock().await;
    if config.telegram.api_id == 0 || config.telegram.api_hash.is_empty() {
        return Ok("needs_config".to_string());
    }
    drop(config);

    let store = state.store.lock().await;
    if store.is_none() {
        return Ok("needs_passphrase".to_string());
    }
    drop(store);

    let bridge = state.bridge.lock().await;
    match bridge.as_ref() {
        None => Ok("needs_telegram_auth".to_string()),
        Some(b) => {
            // Bridge is Some — check if self_user_id has been populated (i.e. connected + authed).
            match b.get_self_user_id().await {
                Ok(_) => Ok("ready".to_string()),
                Err(_) => Ok("needs_telegram_auth".to_string()),
            }
        }
    }
}

/// Save API credentials and display name (step 1).
#[tauri::command]
pub async fn submit_config(
    state: State<'_, VeilState>,
    api_id: i32,
    api_hash: String,
    display_name: String,
) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.telegram.api_id = api_id;
    config.telegram.api_hash = api_hash;
    config.display_name = display_name;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

/// Unlock the keyring with passphrase, initialise IdentityStore and TelegramBridge (step 2).
/// Returns the next status: "needs_telegram_auth" or "ready".
#[tauri::command]
pub async fn submit_passphrase(
    state: State<'_, VeilState>,
    passphrase: String,
) -> Result<String, String> {
    let (api_id, api_hash, session_path) = {
        let config = state.config.lock().await;
        (
            config.telegram.api_id,
            config.telegram.api_hash.clone(),
            config.telegram.session_path.clone(),
        )
    };

    // Initialise identity store.
    let identity_store = IdentityStore::new(&passphrase, Some(state.veil_dir.clone()))
        .map_err(|e| e.to_string())?;

    let monitored: HashSet<i64> = identity_store
        .list_contacts()
        .iter()
        .map(|c| c.telegram_channel_id)
        .collect();

    // Store the identity store.
    {
        let mut store = state.store.lock().await;
        *store = Some(identity_store);
    }

    // Create the bridge (not yet connected).
    let bridge = TelegramBridge::new(
        api_id,
        api_hash,
        std::path::PathBuf::from(&session_path),
        monitored,
        state.message_tx.clone(),
    );

    {
        let mut bridge_lock = state.bridge.lock().await;
        *bridge_lock = Some(bridge);
    }

    // Try to connect — if a valid session exists, we go straight to "ready".
    let is_authorized = {
        let mut bridge_lock = state.bridge.lock().await;
        let bridge = bridge_lock.as_mut().ok_or("bridge not initialised")?;
        bridge.connect_unauthenticated().await.map_err(|e| e.to_string())?
    };

    if is_authorized {
        // Session is valid — finalize the connection (spawns listener).
        {
            let mut bridge_lock = state.bridge.lock().await;
            let bridge = bridge_lock.as_mut().ok_or("bridge not initialised")?;
            bridge.finalize_connection().await.map_err(|e| e.to_string())?;
        }
        Ok("ready".to_string())
    } else {
        Ok("needs_telegram_auth".to_string())
    }
}

/// Request a Telegram login code for the given phone number (step 3a).
#[tauri::command]
pub async fn request_telegram_code(
    state: State<'_, VeilState>,
    phone: String,
) -> Result<(), String> {
    let token: LoginToken = {
        let bridge_lock = state.bridge.lock().await;
        let bridge = bridge_lock.as_ref().ok_or("bridge not initialised")?;
        bridge
            .request_login_code_for_phone(&phone)
            .await
            .map_err(|e: crate::bridges::BridgeError| e.to_string())?
    };

    let mut login_token = state.login_token.lock().await;
    *login_token = Some(token);

    Ok(())
}

/// Submit the Telegram login code (step 3b).
/// Returns "ready" if sign-in succeeded, or "needs_2fa" if 2FA is required.
#[tauri::command]
pub async fn submit_telegram_code(
    state: State<'_, VeilState>,
    code: String,
) -> Result<String, String> {
    let token: LoginToken = {
        let mut login_token = state.login_token.lock().await;
        login_token.take().ok_or("no pending login token — call request_telegram_code first")?
    };

    let password_token: Option<PasswordToken> = {
        let mut bridge_lock = state.bridge.lock().await;
        let bridge = bridge_lock.as_mut().ok_or("bridge not initialised")?;
        bridge
            .sign_in_with_code(token, &code)
            .await
            .map_err(|e: crate::bridges::BridgeError| e.to_string())?
    };

    if let Some(pt) = password_token {
        let mut pw_token = state.password_token.lock().await;
        *pw_token = Some(pt);
        Ok("needs_2fa".to_string())
    } else {
        Ok("ready".to_string())
    }
}

/// Submit 2FA password (step 3c, optional).
#[tauri::command]
pub async fn submit_2fa_password(
    state: State<'_, VeilState>,
    password: String,
) -> Result<(), String> {
    let token: PasswordToken = {
        let mut pw_token = state.password_token.lock().await;
        pw_token.take().ok_or("no pending 2FA token")?
    };

    let mut bridge_lock = state.bridge.lock().await;
    let bridge = bridge_lock.as_mut().ok_or("bridge not initialised")?;
    bridge
        .check_2fa_password(token, &password)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// App commands (only valid after setup)
// ---------------------------------------------------------------------------

/// Return the full contact list.
#[tauri::command]
pub async fn list_contacts(state: State<'_, VeilState>) -> Result<Vec<ContactInfo>, String> {
    let store = state.store.lock().await;
    let store = store.as_ref().ok_or("not initialized")?;
    let contacts: Vec<ContactInfo> = store.list_contacts().iter().map(|c| ContactInfo::from(*c)).collect();
    Ok(contacts)
}

/// Encrypt a message with the contact's shared key, wrap it in the envelope
/// template, send via Telegram, and emit a "veil://message" event.
#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    state: State<'_, VeilState>,
    contact_id: String,
    text: String,
) -> Result<(), String> {
    // 1. Clone contact from store (release lock for network I/O)
    let mut contact = {
        let store = state.store.lock().await;
        let store = store.as_ref().ok_or("not initialized")?;
        store
            .get_contact(&contact_id)
            .ok_or_else(|| format!("contact not found: {contact_id}"))?
            .clone()
    };

    // 2. Encrypt — ratchet or legacy
    let sealed = if let Some(ref mut ratchet) = contact.ratchet {
        ratchet_encrypt(ratchet, text.as_bytes())
    } else {
        encrypt(text.as_bytes(), &contact.key)
    };
    let ciphertext_b64 = URL_SAFE.encode(&sealed);

    // 3. Wrap in envelope template
    let envelope = wrap(&ciphertext_b64, &contact.envelope_template);

    // 4. Persist ratchet state before send (crash-safe: skipped-key cache
    //    handles the gap if the subsequent send fails)
    if contact.ratchet.is_some() {
        let mut store = state.store.lock().await;
        let store = store.as_mut().ok_or("not initialized")?;
        store.update_contact(contact.clone()).map_err(|e| e.to_string())?;
    }

    // 5. Send via bridge
    {
        let bridge = state.bridge.lock().await;
        let bridge = bridge.as_ref().ok_or("not initialized")?;
        bridge
            .send(contact.telegram_channel_id, &envelope)
            .await
            .map_err(|e| e.to_string())?;
    }

    // 6. Emit outgoing message event to frontend
    let event = MessageEvent {
        contact_id: contact_id.clone(),
        text,
        direction: "out".into(),
        timestamp: Utc::now().to_rfc3339(),
    };
    app.emit("veil://message", event).map_err(|e| e.to_string())?;

    Ok(())
}

/// Generate a QR code for the initiator pairing role.
/// Returns a base64-encoded PNG image.
#[tauri::command]
pub async fn initiate_pairing(state: State<'_, VeilState>) -> Result<String, String> {
    // Get our own Telegram user ID and config values
    let (self_user_id, display_name, envelope_template) = {
        let bridge = state.bridge.lock().await;
        let bridge = bridge.as_ref().ok_or("not initialized")?;
        let uid = bridge.get_self_user_id().await.map_err(|e| e.to_string())?;
        let config = state.config.lock().await;
        (uid, config.display_name.clone(), config.envelope_template.clone())
    };

    // Generate QR payload + PNG bytes
    let (payload, png_bytes) =
        create_qr_payload(self_user_id, &display_name, &envelope_template)
            .map_err(|e| e.to_string())?;

    // Store pending pairing so the incoming message handler can detect the handshake
    {
        let mut pending = state.pending_pairing.lock().await;
        *pending = Some(payload);
    }

    // Enable passthrough so we receive messages from unknown channels
    {
        let bridge = state.bridge.lock().await;
        let bridge = bridge.as_ref().ok_or("not initialized")?;
        let mut passthrough = bridge.passthrough.lock().await;
        *passthrough = true;
    }

    Ok(URL_SAFE.encode(&png_bytes))
}

/// Complete pairing (joiner role): parse the QR data, create a Telegram group,
/// send the handshake message, save the contact.
#[tauri::command]
pub async fn complete_pairing(
    app: AppHandle,
    state: State<'_, VeilState>,
    qr_data: String,
) -> Result<ContactInfo, String> {
    use crate::identity::pairing::parse_qr_payload;

    // 1. Parse QR
    let qr = parse_qr_payload(&qr_data).map_err(|e| e.to_string())?;

    let (display_name, envelope_template) = {
        let config = state.config.lock().await;
        (config.display_name.clone(), config.envelope_template.clone())
    };

    // 2. Create Telegram group with the initiator
    let channel_id = {
        let bridge = state.bridge.lock().await;
        let bridge = bridge.as_ref().ok_or("not initialized")?;
        bridge
            .create_channel(&[qr.telegram_user_id], &format!("veil:{}", qr.display_name))
            .await
            .map_err(|e| e.to_string())?
    };

    // 3. Send encrypted handshake
    let handshake = build_handshake_message(&qr.key, &display_name, &envelope_template)
        .map_err(|e| e.to_string())?;
    {
        let bridge = state.bridge.lock().await;
        let bridge = bridge.as_ref().ok_or("not initialized")?;
        bridge
            .send(channel_id, &handshake)
            .await
            .map_err(|e| e.to_string())?;
    }

    // 4. Save contact — init ratchet for v2 QR payloads
    let ratchet = if qr.version == 2 {
        Some(init_ratchet(&qr.key, false))
    } else {
        None
    };

    let contact = Contact {
        contact_id: Uuid::new_v4().to_string(),
        display_name: qr.display_name.clone(),
        key: qr.key,
        telegram_channel_id: channel_id,
        telegram_user_id: qr.telegram_user_id,
        created_at: Utc::now(),
        envelope_template: qr.envelope_template.clone(),
        is_initiator: false,
        ratchet,
    };

    {
        let mut store = state.store.lock().await;
        let store = store.as_mut().ok_or("not initialized")?;
        store.add_contact(contact.clone()).map_err(|e| e.to_string())?;
    }

    // 5. Add to monitored channels
    {
        let bridge = state.bridge.lock().await;
        let bridge = bridge.as_ref().ok_or("not initialized")?;
        bridge.add_monitored_channel(channel_id).await;
    }

    let info = ContactInfo::from(&contact);

    // 6. Emit pairing_complete event
    app.emit("veil://pairing_complete", info.clone())
        .map_err(|e| e.to_string())?;

    Ok(info)
}

/// Update the envelope template and persist to config.
#[tauri::command]
pub async fn update_envelope(
    state: State<'_, VeilState>,
    template: String,
) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.envelope_template = template;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

/// Update the active theme and persist to config.
#[tauri::command]
pub async fn set_theme(state: State<'_, VeilState>, theme_id: String) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.theme = theme_id;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Incoming message handler (called from main.rs setup)
// ---------------------------------------------------------------------------

/// Process a single incoming message from the bridge mpsc channel.
///
/// - If a pending pairing is active and the message is a valid handshake,
///   complete the initiator role: save the contact, disable passthrough, emit event.
/// - Otherwise, look up the contact by channel, unwrap/decrypt, emit "veil://message".
pub async fn handle_incoming_message(
    app: &AppHandle,
    state: &VeilState,
    channel_id: i64,
    text: &str,
) {
    // Check for pending pairing (initiator role)
    let pending = {
        let lock = state.pending_pairing.lock().await;
        lock.clone()
    };

    if let Some(qr_payload) = pending {
        if let Some(handshake) = parse_handshake_message(text, &qr_payload.key) {
            // Complete initiator side pairing
            let ratchet = if qr_payload.version == 2 {
                Some(init_ratchet(&qr_payload.key, true))
            } else {
                None
            };

            let contact = Contact {
                contact_id: Uuid::new_v4().to_string(),
                display_name: handshake.name.clone(),
                key: qr_payload.key,
                telegram_channel_id: channel_id,
                telegram_user_id: 0, // unknown at initiator side
                created_at: Utc::now(),
                envelope_template: handshake.env.clone(),
                is_initiator: true,
                ratchet,
            };

            {
                let mut store = state.store.lock().await;
                if let Some(store) = store.as_mut() {
                    if let Err(e) = store.add_contact(contact.clone()) {
                        log::error!("failed to save contact after handshake: {e}");
                        return;
                    }
                } else {
                    log::error!("store not initialized when receiving handshake");
                    return;
                }
            }

            // Add to monitored channels and disable passthrough
            {
                let bridge = state.bridge.lock().await;
                if let Some(bridge) = bridge.as_ref() {
                    bridge.add_monitored_channel(channel_id).await;
                    let mut passthrough = bridge.passthrough.lock().await;
                    *passthrough = false;
                }
            }

            // Clear pending pairing
            {
                let mut pending_lock = state.pending_pairing.lock().await;
                *pending_lock = None;
            }

            let info = ContactInfo::from(&contact);
            if let Err(e) = app.emit("veil://pairing_complete", info) {
                log::error!("failed to emit pairing_complete: {e}");
            }
            return;
        }
    }

    // Normal encrypted message — look up contact by channel
    let mut contact = {
        let store = state.store.lock().await;
        match store.as_ref().and_then(|s| s.get_contact_by_channel(channel_id).cloned()) {
            Some(c) => c,
            None => {
                log::debug!("received message from unknown channel {channel_id}, ignoring");
                return;
            }
        }
    };

    // Unwrap envelope
    let ciphertext_b64 = match unwrap(text, &[contact.envelope_template.as_str(), ""]) {
        Some(ct) => ct,
        None => {
            log::debug!("could not unwrap message from channel {channel_id}");
            return;
        }
    };

    // Decode base64
    let sealed = match URL_SAFE.decode(&ciphertext_b64) {
        Ok(b) => b,
        Err(e) => {
            log::debug!("base64 decode failed for channel {channel_id}: {e}");
            return;
        }
    };

    // Decrypt — ratchet or legacy
    let plaintext = if let Some(ref mut ratchet) = contact.ratchet {
        match ratchet_decrypt(ratchet, &sealed) {
            Ok(p) => p,
            Err(e) => {
                log::debug!("ratchet decrypt failed for channel {channel_id}: {e}");
                return;
            }
        }
    } else {
        match crate::crypto::decrypt(&sealed, &contact.key) {
            Ok(p) => p,
            Err(e) => {
                log::debug!("decrypt failed for channel {channel_id}: {e}");
                return;
            }
        }
    };

    // Persist updated ratchet state
    if contact.ratchet.is_some() {
        let mut store = state.store.lock().await;
        if let Some(store) = store.as_mut() {
            if let Err(e) = store.update_contact(contact.clone()) {
                log::error!("failed to persist ratchet state: {e}");
            }
        }
    }

    let plain_text = match String::from_utf8(plaintext) {
        Ok(s) => s,
        Err(e) => {
            log::debug!("message is not valid UTF-8 from channel {channel_id}: {e}");
            return;
        }
    };

    let event = MessageEvent {
        contact_id: contact.contact_id.clone(),
        text: plain_text,
        direction: "in".into(),
        timestamp: Utc::now().to_rfc3339(),
    };

    if let Err(e) = app.emit("veil://message", event) {
        log::error!("failed to emit veil://message: {e}");
    }
}
