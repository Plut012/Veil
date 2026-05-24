<script lang="ts">
	import type { Contact } from '$lib/stores/contacts.js';

	export let contact: Contact;
	export let selected: boolean = false;
	export let onClick: () => void = () => {};
</script>

<button
	class="contact-item"
	class:selected
	on:click={onClick}
	aria-current={selected ? 'true' : undefined}
>
	<span class="indicator" class:active={selected}></span>
	<span class="name">{contact.display_name}</span>
</button>

<style>
	.contact-item {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		width: 100%;
		padding: var(--spacing-sm) var(--spacing-md);
		text-align: left;
		border-radius: var(--radius-sm);
		transition: background var(--transition-fast);
		color: var(--color-text-muted);
	}

	.contact-item:hover {
		background: var(--color-bg-elevated);
		color: var(--color-text);
	}

	.contact-item.selected {
		background: var(--color-bg-elevated);
		color: var(--color-text);
	}

	.indicator {
		flex-shrink: 0;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		border: 1px solid var(--color-text-muted);
		transition: border-color var(--transition-fast), background var(--transition-fast);
	}

	.contact-item.selected .indicator,
	.indicator.active {
		border-color: var(--color-accent);
		background: var(--color-accent);
	}

	.name {
		font-size: 13px;
		letter-spacing: 0.01em;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
