import { readable, derived } from 'svelte/store';

// Tauri IPC is always available — no connection handshake needed.
export type ConnectionState = 'connected';

export const connectionState = readable<ConnectionState>('connected');

export const isConnected = derived(connectionState, () => true);
