import { writable, derived } from 'svelte/store';
import { selectedContactId } from './ui.js';

export interface Message {
	contact_id: string;
	text: string;
	direction: 'in' | 'out';
	timestamp: string;
}

// Map from contact_id to array of messages
export const messages = writable<Map<string, Message[]>>(new Map());

export function addMessage(message: Message): void {
	messages.update((map) => {
		const key = message.contact_id;
		const existing = map.get(key) ?? [];
		const updated = new Map(map);
		updated.set(key, [...existing, message]);
		return updated;
	});
}

export function getMessagesForContact(contactId: string, map: Map<string, Message[]>): Message[] {
	return map.get(contactId) ?? [];
}

// Derived store: messages for currently selected contact
export const currentMessages = derived(
	[messages, selectedContactId],
	([$messages, $selectedId]) => {
		if (!$selectedId) return [];
		return $messages.get($selectedId) ?? [];
	}
);
