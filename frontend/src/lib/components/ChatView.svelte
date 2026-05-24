<script lang="ts">
	import { selectedContactId } from '$lib/stores/ui.js';
	import { contacts } from '$lib/stores/contacts.js';
	import MessageList from './MessageList.svelte';
	import Compose from './Compose.svelte';

	$: selectedContact = $contacts.find((c) => c.contact_id === $selectedContactId) ?? null;
</script>

<div class="chat-view">
	{#if selectedContact}
		<header class="chat-header">
			<span class="contact-name">{selectedContact.display_name}</span>
		</header>
		<MessageList />
		<Compose />
	{:else}
		<div class="no-contact">
			<p>Select a contact to begin.</p>
		</div>
	{/if}
</div>

<style>
	.chat-view {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		height: 100%;
		background: var(--color-bg);
	}

	.chat-header {
		display: flex;
		align-items: center;
		padding: var(--spacing-md) var(--spacing-lg);
		border-bottom: 1px solid var(--color-border);
		background: var(--color-bg-surface);
		min-height: 52px;
	}

	.contact-name {
		font-size: 14px;
		font-weight: 500;
		letter-spacing: 0.03em;
		color: var(--color-text);
	}

	.no-contact {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		font-size: 13px;
		font-style: italic;
	}
</style>
