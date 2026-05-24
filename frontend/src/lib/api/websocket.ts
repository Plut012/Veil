import { connectionState } from '$lib/stores/connection.js';
import { setContacts, addContact } from '$lib/stores/contacts.js';
import { addMessage } from '$lib/stores/messages.js';
import { showPairingModal } from '$lib/stores/ui.js';

const WS_URL = 'ws://127.0.0.1:8900/ws';
const RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000, 30000];

type Handler = (data: unknown) => void;

export class VeilSocket {
	private ws: WebSocket | null = null;
	private handlers: Map<string, Handler[]> = new Map();
	private reconnectAttempt = 0;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private intentionalClose = false;
	private qrImageStore = { set: (_v: string | null) => {} };

	// Store reference for QR image, set after construction
	setQrStore(store: { set: (v: string | null) => void }): void {
		this.qrImageStore = store;
	}

	connect(): void {
		this.intentionalClose = false;
		this._connect();
	}

	private _connect(): void {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) return;

		connectionState.set('connecting');

		try {
			this.ws = new WebSocket(WS_URL);
		} catch {
			this._scheduleReconnect();
			return;
		}

		this.ws.onopen = () => {
			this.reconnectAttempt = 0;
			connectionState.set('connected');
		};

		this.ws.onmessage = (event: MessageEvent) => {
			try {
				const msg = JSON.parse(event.data as string) as { type: string; [key: string]: unknown };
				this._dispatch(msg);
			} catch {
				// malformed message — ignore
			}
		};

		this.ws.onclose = () => {
			connectionState.set('disconnected');
			if (!this.intentionalClose) {
				this._scheduleReconnect();
			}
		};

		this.ws.onerror = () => {
			connectionState.set('error');
			// onclose will fire after onerror; reconnect handled there
		};
	}

	private _scheduleReconnect(): void {
		if (this.intentionalClose) return;
		if (this.reconnectTimer) clearTimeout(this.reconnectTimer);

		const delay = RECONNECT_DELAYS[Math.min(this.reconnectAttempt, RECONNECT_DELAYS.length - 1)];
		this.reconnectAttempt++;

		connectionState.set('connecting');
		this.reconnectTimer = setTimeout(() => {
			this._connect();
		}, delay);
	}

	disconnect(): void {
		this.intentionalClose = true;
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}
		connectionState.set('disconnected');
	}

	send(message: object): void {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(message));
		}
	}

	// Typed send helpers

	sendMessage(contactId: string, text: string): void {
		this.send({ type: 'send_message', contact_id: contactId, text });
	}

	requestContacts(): void {
		this.send({ type: 'list_contacts' });
	}

	initiatePairing(): void {
		this.send({ type: 'initiate_pairing' });
	}

	completePairing(qrData: string): void {
		this.send({ type: 'complete_pairing', qr_data: qrData });
	}

	updateEnvelope(template: string): void {
		this.send({ type: 'update_envelope', template });
	}

	setTheme(themeId: string): void {
		this.send({ type: 'set_theme', theme_id: themeId });
	}

	// Event registration

	on(type: string, handler: Handler): void {
		const existing = this.handlers.get(type) ?? [];
		this.handlers.set(type, [...existing, handler]);
	}

	off(type: string, handler: Handler): void {
		const existing = this.handlers.get(type) ?? [];
		this.handlers.set(
			type,
			existing.filter((h) => h !== handler)
		);
	}

	private _dispatch(msg: { type: string; [key: string]: unknown }): void {
		// Built-in handlers that update stores
		switch (msg.type) {
			case 'connected':
				connectionState.set('connected');
				break;

			case 'contacts':
				setContacts((msg.contacts as import('$lib/stores/contacts.js').Contact[]) ?? []);
				break;

			case 'message':
				addMessage({
					contact_id: msg.contact_id as string,
					text: msg.text as string,
					direction: msg.direction as 'in' | 'out',
					timestamp: (msg.timestamp as string) ?? new Date().toISOString()
				});
				break;

			case 'pairing_qr':
				// Dispatched to external handlers (PairingModal)
				break;

			case 'pairing_complete':
				addContact(msg.contact as import('$lib/stores/contacts.js').Contact);
				showPairingModal.set(false);
				break;

			case 'error':
				console.error('[veil]', msg.message);
				break;
		}

		// Dispatch to registered external handlers
		const handlers = this.handlers.get(msg.type) ?? [];
		for (const h of handlers) {
			h(msg);
		}
	}
}

// Singleton instance
export const veilSocket = new VeilSocket();
