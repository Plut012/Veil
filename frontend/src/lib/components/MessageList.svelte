<script lang="ts">
	import { afterUpdate } from 'svelte';
	import { currentMessages } from '$lib/stores/messages.js';
	import Message from './Message.svelte';

	let container: HTMLDivElement;

	afterUpdate(() => {
		if (container) {
			container.scrollTop = container.scrollHeight;
		}
	});
</script>

<div class="message-list" bind:this={container}>
	{#each $currentMessages as msg, i (i + msg.timestamp + msg.direction)}
		<Message message={msg} />
	{/each}
</div>

<style>
	.message-list {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		padding: var(--spacing-md);
		gap: var(--spacing-xs);
	}
</style>
