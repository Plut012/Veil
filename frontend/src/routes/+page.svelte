<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { listContacts, onMessage, onPairingComplete } from '$lib/api/tauri.js';
	import { setContacts, addContact } from '$lib/stores/contacts.js';
	import { addMessage } from '$lib/stores/messages.js';
	import { showPairingModal } from '$lib/stores/ui.js';
	import { showSettings } from '$lib/stores/ui.js';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import ChatView from '$lib/components/ChatView.svelte';
	import PairingModal from '$lib/components/PairingModal.svelte';
	import SettingsPanel from '$lib/components/SettingsPanel.svelte';

	let unlistenMessage: (() => void) | null = null;
	let unlistenPairing: (() => void) | null = null;

	onMount(async () => {
		// Load contacts from Rust
		try {
			const contacts = await listContacts();
			setContacts(contacts);
		} catch (err) {
			console.error('[veil] listContacts failed:', err);
		}

		// Listen for incoming messages
		const unlistenMsgPromise = onMessage((msg) => {
			addMessage(msg);
		});

		// Listen for pairing completions
		const unlistenPairPromise = onPairingComplete((contact) => {
			addContact(contact);
			showPairingModal.set(false);
		});

		unlistenMessage = await unlistenMsgPromise;
		unlistenPairing = await unlistenPairPromise;
	});

	onDestroy(() => {
		if (unlistenMessage) unlistenMessage();
		if (unlistenPairing) unlistenPairing();
	});
</script>

<div class="app-shell">
	<Sidebar />
	<main class="main-area">
		<ChatView />
	</main>

	{#if $showSettings}
		<SettingsPanel />
	{/if}

	{#if $showPairingModal}
		<PairingModal />
	{/if}
</div>

<style>
	.app-shell {
		display: flex;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
		position: relative;
	}

	.main-area {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
</style>
