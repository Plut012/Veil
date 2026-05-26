<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { getSetupStatus, listContacts, onMessage, onPairingComplete } from '$lib/api/tauri.js';
	import { setContacts, addContact } from '$lib/stores/contacts.js';
	import { addMessage } from '$lib/stores/messages.js';
	import { showPairingModal, showSettings } from '$lib/stores/ui.js';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import ChatView from '$lib/components/ChatView.svelte';
	import PairingModal from '$lib/components/PairingModal.svelte';
	import SettingsPanel from '$lib/components/SettingsPanel.svelte';
	import SetupView from '$lib/components/SetupView.svelte';

	let setupComplete = false;
	let setupStatus = 'loading';

	let unlistenMessage: (() => void) | null = null;
	let unlistenPairing: (() => void) | null = null;

	async function initApp() {
		try {
			const contacts = await listContacts();
			setContacts(contacts);
		} catch (err) {
			console.error('[veil] listContacts failed:', err);
		}

		const unlistenMsgPromise = onMessage((msg) => {
			addMessage(msg);
		});

		const unlistenPairPromise = onPairingComplete((contact) => {
			addContact(contact);
			showPairingModal.set(false);
		});

		unlistenMessage = await unlistenMsgPromise;
		unlistenPairing = await unlistenPairPromise;
	}

	onMount(async () => {
		try {
			setupStatus = await getSetupStatus();
		} catch (err) {
			console.error('[veil] getSetupStatus failed:', err);
			setupStatus = 'needs_passphrase';
		}

		if (setupStatus === 'ready') {
			setupComplete = true;
			await initApp();
		}
	});

	async function onSetupDone() {
		setupComplete = true;
		await initApp();
	}

	onDestroy(() => {
		if (unlistenMessage) unlistenMessage();
		if (unlistenPairing) unlistenPairing();
	});
</script>

{#if setupComplete}
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
{:else if setupStatus !== 'loading'}
	<SetupView status={setupStatus} on:complete={onSetupDone} />
{/if}

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
