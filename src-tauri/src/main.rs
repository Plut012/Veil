#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bridges;
mod crypto;
mod envelope;
mod identity;

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use app::commands::handle_incoming_message;
use app::state::VeilState;
use app::{
    complete_pairing, initiate_pairing, list_contacts, send_message, set_theme, update_envelope,
    VeilConfig,
};
use bridges::{Bridge, IncomingMessage, TelegramBridge};
use identity::IdentityStore;
use tauri::Manager;
use tokio::sync::mpsc;

fn prompt(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().ok();
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok();
    line.trim_end_matches('\n')
        .trim_end_matches('\r')
        .to_string()
}

fn main() {
    // Initialise libsodium — must be called before any crypto operations.
    sodiumoxide::init().expect("sodiumoxide::init() failed");

    let veil_dir: PathBuf = dirs::home_dir()
        .expect("could not determine home directory")
        .join(".veil");
    let config_path = veil_dir.join("config.toml");

    // Load (or create default) config.
    let mut config = VeilConfig::load(&config_path);

    // First-run: prompt for Telegram API credentials if not configured.
    if config.telegram.api_id == 0 || config.telegram.api_hash.is_empty() {
        println!("--- Veil first-run setup ---");
        let id_str = prompt("Telegram API ID: ");
        config.telegram.api_id = id_str.parse().unwrap_or(0);
        config.telegram.api_hash = prompt("Telegram API hash: ");
        config
            .save(&config_path)
            .expect("failed to save config after first-run setup");
    }

    // Prompt for passphrase (no echo).
    let passphrase =
        rpassword::read_password_from_tty(Some("Veil passphrase: ")).unwrap_or_default();

    // Initialise identity store.
    let store = IdentityStore::new(&passphrase, Some(veil_dir.clone()))
        .expect("failed to open identity store");

    // Collect channel IDs already in the store so the bridge monitors them.
    let monitored: HashSet<i64> = store
        .list_contacts()
        .iter()
        .map(|c| c.telegram_channel_id)
        .collect();

    // Create mpsc channel for incoming bridge messages.
    let (message_tx, message_rx) = mpsc::unbounded_channel::<IncomingMessage>();

    // Initialise the Telegram bridge.
    let bridge = TelegramBridge::new(
        config.telegram.api_id,
        config.telegram.api_hash.clone(),
        PathBuf::from(&config.telegram.session_path),
        monitored,
        message_tx,
    );

    let veil_state = VeilState::new(config, store, bridge, config_path);

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
            let app_handle = app.handle().clone();

            // Connect bridge (interactive sign-in if needed).
            {
                let state = app.state::<VeilState>();
                tauri::async_runtime::block_on(async {
                    let mut bridge = state.bridge.lock().await;
                    bridge
                        .connect()
                        .await
                        .expect("Telegram bridge failed to connect");
                });
            }

            // Spawn task that reads incoming messages and processes them.
            let app_handle_rx = app_handle.clone();
            let mut rx = message_rx;

            tauri::async_runtime::spawn(async move {
                while let Some(incoming) = rx.recv().await {
                    let state = app_handle_rx.state::<VeilState>();
                    handle_incoming_message(
                        &app_handle_rx,
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
