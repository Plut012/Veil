<script lang="ts">
	import { selectedContactId } from '$lib/stores/ui.js';
	import { isConnected } from '$lib/stores/connection.js';
	import { sendMessage } from '$lib/api/tauri.js';

	let text = '';

	async function send() {
		const trimmed = text.trim();
		if (!trimmed || !$selectedContactId || !$isConnected) return;
		try {
			await sendMessage($selectedContactId, trimmed);
			text = '';
		} catch (err) {
			console.error('[veil] sendMessage failed:', err);
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			send();
		}
	}
</script>

<div class="compose">
	<textarea
		class="input"
		bind:value={text}
		on:keydown={onKeydown}
		placeholder="write..."
		rows="1"
		disabled={!$isConnected || !$selectedContactId}
	></textarea>
	<button class="send-btn" on:click={send} disabled={!text.trim() || !$isConnected || !$selectedContactId} aria-label="Send">
		<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
			<path d="M1 7h12M8 2l5 5-5 5" stroke-linecap="round" stroke-linejoin="round" />
		</svg>
	</button>
</div>

<style>
	.compose {
		display: flex;
		align-items: flex-end;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		border-top: 1px solid var(--color-border);
		background: var(--color-bg-surface);
	}

	.input {
		flex: 1;
		resize: none;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--spacing-sm) var(--spacing-md);
		color: var(--color-text);
		font-size: 14px;
		line-height: 1.5;
		min-height: 38px;
		max-height: 140px;
		overflow-y: auto;
		transition: border-color var(--transition-fast);
	}

	.input:focus {
		outline: none;
		border-color: var(--color-accent-dim);
	}

	.input::placeholder {
		color: var(--color-text-muted);
		font-style: italic;
	}

	.input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.send-btn {
		flex-shrink: 0;
		width: 36px;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-md);
		background: var(--color-accent-dim);
		color: var(--color-bg);
		transition: background var(--transition-fast), opacity var(--transition-fast);
	}

	.send-btn:hover:not(:disabled) {
		background: var(--color-accent);
	}

	.send-btn:disabled {
		opacity: 0.35;
		cursor: not-allowed;
	}
</style>
