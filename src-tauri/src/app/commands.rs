use base64::{engine::general_purpose::URL_SAFE, Engine};
use chrono::Utc;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::app::state::VeilState;
use crate::bridges::Bridge;
use crate::crypto::encrypt;
use crate::envelope::{unwrap, wrap};
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
// Commands
// ---------------------------------------------------------------------------

/// Return the full contact list.
#[tauri::command]
pub async fn list_contacts(state: State<'_, VeilState>) -> Result<Vec<ContactInfo>, String> {
    let store = state.store.lock().await;
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
    // 1. Look up contact and grab needed fields
    let (key, channel_id, template) = {
        let store = state.store.lock().await;
        let contact = store
            .get_contact(&contact_id)
            .ok_or_else(|| format!("contact not found: {contact_id}"))?;
        (contact.key, contact.telegram_channel_id, contact.envelope_template.clone())
    };

    // 2. Encrypt
    let sealed = encrypt(text.as_bytes(), &key);
    let ciphertext_b64 = URL_SAFE.encode(&sealed);

    // 3. Wrap in envelope template
    let envelope = wrap(&ciphertext_b64, &template);

    // 4. Send via bridge
    {
        let bridge = state.bridge.lock().await;
        bridge
            .send(channel_id, &envelope)
            .await
            .map_err(|e| e.to_string())?;
    }

    // 5. Emit outgoing message event to frontend
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
        bridge
            .send(channel_id, &handshake)
            .await
            .map_err(|e| e.to_string())?;
    }

    // 4. Save contact
    let contact = Contact {
        contact_id: Uuid::new_v4().to_string(),
        display_name: qr.display_name.clone(),
        key: qr.key,
        telegram_channel_id: channel_id,
        telegram_user_id: qr.telegram_user_id,
        created_at: Utc::now(),
        envelope_template: qr.envelope_template.clone(),
    };

    {
        let mut store = state.store.lock().await;
        store.add_contact(contact.clone()).map_err(|e| e.to_string())?;
    }

    // 5. Add to monitored channels
    {
        let bridge = state.bridge.lock().await;
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
            let contact = Contact {
                contact_id: Uuid::new_v4().to_string(),
                display_name: handshake.name.clone(),
                key: qr_payload.key,
                telegram_channel_id: channel_id,
                telegram_user_id: 0, // unknown at initiator side
                created_at: Utc::now(),
                envelope_template: handshake.env.clone(),
            };

            {
                let mut store = state.store.lock().await;
                if let Err(e) = store.add_contact(contact.clone()) {
                    log::error!("failed to save contact after handshake: {e}");
                    return;
                }
            }

            // Add to monitored channels and disable passthrough
            {
                let bridge = state.bridge.lock().await;
                bridge.add_monitored_channel(channel_id).await;
                let mut passthrough = bridge.passthrough.lock().await;
                *passthrough = false;
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
    let contact_opt = {
        let store = state.store.lock().await;
        store.get_contact_by_channel(channel_id).map(|c| (c.contact_id.clone(), c.key, c.envelope_template.clone()))
    };

    let (contact_id, key, template) = match contact_opt {
        Some(t) => t,
        None => {
            log::debug!("received message from unknown channel {channel_id}, ignoring");
            return;
        }
    };

    // Unwrap envelope
    let ciphertext_b64 = match unwrap(text, &[template.as_str(), ""]) {
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

    // Decrypt
    let plaintext = match crate::crypto::decrypt(&sealed, &key) {
        Ok(p) => p,
        Err(e) => {
            log::debug!("decrypt failed for channel {channel_id}: {e}");
            return;
        }
    };

    let plain_text = match String::from_utf8(plaintext) {
        Ok(s) => s,
        Err(e) => {
            log::debug!("message is not valid UTF-8 from channel {channel_id}: {e}");
            return;
        }
    };

    let event = MessageEvent {
        contact_id,
        text: plain_text,
        direction: "in".into(),
        timestamp: Utc::now().to_rfc3339(),
    };

    if let Err(e) = app.emit("veil://message", event) {
        log::error!("failed to emit veil://message: {e}");
    }
}
