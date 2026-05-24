<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { veilSocket } from '$lib/api/websocket.js';
	import { showPairingModal } from '$lib/stores/ui.js';
	import { showSettings } from '$lib/stores/ui.js';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import ChatView from '$lib/components/ChatView.svelte';
	import PairingModal from '$lib/components/PairingModal.svelte';
	import SettingsPanel from '$lib/components/SettingsPanel.svelte';

	onMount(() => {
		veilSocket.connect();
	});

	onDestroy(() => {
		veilSocket.disconnect();
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
