import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface Contact {
	contact_id: string;
	display_name: string;
	telegram_channel_id: number;
}

export interface MessageEvent {
	contact_id: string;
	text: string;
	direction: 'in' | 'out';
	timestamp: string;
}

// ---------------------------------------------------------------------------
// Setup commands
// ---------------------------------------------------------------------------

export async function getSetupStatus(): Promise<string> {
	return invoke('get_setup_status');
}

export async function submitConfig(
	apiId: number,
	apiHash: string,
	displayName: string
): Promise<void> {
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

// ---------------------------------------------------------------------------
// Commands (frontend → Rust)
// ---------------------------------------------------------------------------

export async function listContacts(): Promise<Contact[]> {
	return invoke('list_contacts');
}

export async function sendMessage(contactId: string, text: string): Promise<void> {
	return invoke('send_message', { contactId, text });
}

export async function initiatePairing(): Promise<string> {
	return invoke('initiate_pairing');
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

// ---------------------------------------------------------------------------
// Events (Rust → frontend)
// ---------------------------------------------------------------------------

export function onMessage(handler: (msg: MessageEvent) => void) {
	return listen<MessageEvent>('veil://message', (e) => handler(e.payload));
}

export function onPairingComplete(handler: (contact: Contact) => void) {
	return listen<Contact>('veil://pairing_complete', (e) => handler(e.payload));
}
