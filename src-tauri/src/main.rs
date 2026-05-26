#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bridges;
mod crypto;
mod envelope;
mod identity;

use std::path::PathBuf;

use app::commands::handle_incoming_message;
use app::state::VeilState;
use app::{
    complete_pairing, get_setup_status, initiate_pairing, list_contacts, request_telegram_code,
    send_message, set_theme, submit_2fa_password, submit_config, submit_passphrase,
    submit_telegram_code, update_envelope, VeilConfig,
};
use tauri::Manager;
use tokio::sync::mpsc;

fn main() {
    // Initialise libsodium — must be called before any crypto operations.
    sodiumoxide::init().expect("sodiumoxide::init() failed");

    let veil_dir: PathBuf = dirs::home_dir()
        .expect("could not determine home directory")
        .join(".veil");
    let config_path = veil_dir.join("config.toml");

    // Load (or create default) config.
    let config = VeilConfig::load(&config_path);

    // Channel for forwarding incoming messages from the bridge listener task.
    let (message_tx, mut message_rx) = mpsc::unbounded_channel();

    let veil_state = VeilState::new(config, config_path, veil_dir, message_tx);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(veil_state)
        .invoke_handler(tauri::generate_handler![
            // Setup commands
            get_setup_status,
            submit_config,
            submit_passphrase,
            request_telegram_code,
            submit_telegram_code,
            submit_2fa_password,
            // App commands (only valid after setup)
            list_contacts,
            send_message,
            initiate_pairing,
            complete_pairing,
            update_envelope,
            set_theme,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Spawn task that reads incoming messages and processes them.
            tauri::async_runtime::spawn(async move {
                while let Some(incoming) = message_rx.recv().await {
                    let state = app_handle.state::<VeilState>();
                    handle_incoming_message(
                        &app_handle,
                        &state,
                        incoming.channel_id,
                        &incoming.text,
                    )
                    .await;
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Veil");
}
