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
