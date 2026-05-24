import { writable } from 'svelte/store';

export type PairingRole = 'initiate' | 'join' | null;

export const selectedContactId = writable<string | null>(null);
export const showPairingModal = writable<boolean>(false);
export const showSettings = writable<boolean>(false);
export const pairingRole = writable<PairingRole>(null);
