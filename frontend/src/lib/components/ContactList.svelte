<script lang="ts">
	import { contacts } from '$lib/stores/contacts.js';
	import { selectedContactId } from '$lib/stores/ui.js';
	import ContactItem from './ContactItem.svelte';
</script>

<div class="contact-list">
	{#if $contacts.length === 0}
		<p class="empty">No contacts yet.</p>
	{:else}
		{#each $contacts as contact (contact.contact_id)}
			<ContactItem
				{contact}
				selected={$selectedContactId === contact.contact_id}
				onClick={() => selectedContactId.set(contact.contact_id)}
			/>
		{/each}
	{/if}
</div>

<style>
	.contact-list {
		flex: 1;
		overflow-y: auto;
		padding: var(--spacing-xs) 0;
	}

	.empty {
		padding: var(--spacing-md);
		color: var(--color-text-muted);
		font-size: 12px;
		font-style: italic;
	}
</style>
