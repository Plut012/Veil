import { writable } from 'svelte/store';

export interface Contact {
	contact_id: string;
	display_name: string;
	telegram_channel_id: number;
}

export const contacts = writable<Contact[]>([]);

export function setContacts(list: Contact[]): void {
	contacts.set(list);
}

export function addContact(contact: Contact): void {
	contacts.update((list) => {
		const exists = list.find((c) => c.contact_id === contact.contact_id);
		if (exists) {
			return list.map((c) => (c.contact_id === contact.contact_id ? contact : c));
		}
		return [...list, contact];
	});
}
