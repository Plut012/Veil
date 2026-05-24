import { writable, derived } from 'svelte/store';

export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

export const connectionState = writable<ConnectionState>('disconnected');

export const isConnected = derived(
	connectionState,
	($state) => $state === 'connected'
);
