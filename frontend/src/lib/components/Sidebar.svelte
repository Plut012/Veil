<script lang="ts">
	import ContactList from './ContactList.svelte';
	import { showPairingModal, showSettings, pairingRole } from '$lib/stores/ui.js';
	import { connectionState } from '$lib/stores/connection.js';

	function openPairing() {
		pairingRole.set(null);
		showPairingModal.set(true);
	}

	function toggleSettings() {
		showSettings.update((v) => !v);
	}

	const stateLabel: Record<string, string> = {
		connected: 'live',
		connecting: '...',
		disconnected: 'off',
		error: 'err'
	};
</script>

<aside class="sidebar">
	<header class="sidebar-header">
		<span class="wordmark">Veil</span>
		<span class="conn-state" data-state={$connectionState}>{stateLabel[$connectionState] ?? '?'}</span>
	</header>

	<ContactList />

	<footer class="sidebar-footer">
		<button class="pair-btn" on:click={openPairing}>+ pair</button>
		<button class="settings-btn" on:click={toggleSettings} aria-label="Settings">
			<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
				<circle cx="7" cy="7" r="2.5" />
				<path d="M7 1v1.5M7 11.5V13M1 7h1.5M11.5 7H13M2.93 2.93l1.06 1.06M10.01 10.01l1.06 1.06M2.93 11.07l1.06-1.06M10.01 3.99l1.06-1.06" />
			</svg>
		</button>
	</footer>
</aside>

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		width: 220px;
		min-width: 180px;
		max-width: 260px;
		background: var(--color-bg-surface);
		border-right: 1px solid var(--color-border);
		height: 100%;
	}

	.sidebar-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-md) var(--spacing-md) var(--spacing-sm);
		border-bottom: 1px solid var(--color-border);
	}

	.wordmark {
		font-size: 15px;
		font-weight: 500;
		color: var(--color-accent);
		letter-spacing: 0.08em;
		text-transform: lowercase;
	}

	.conn-state {
		font-size: 10px;
		font-family: var(--font-mono);
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		padding: 2px 5px;
		border-radius: var(--radius-sm);
		border: 1px solid var(--color-border);
		transition: color var(--transition-fast), border-color var(--transition-fast);
	}

	.conn-state[data-state="connected"] {
		color: var(--color-success);
		border-color: var(--color-success);
	}

	.conn-state[data-state="error"] {
		color: var(--color-error);
		border-color: var(--color-error);
	}

	.sidebar-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-sm) var(--spacing-md);
		border-top: 1px solid var(--color-border);
	}

	.pair-btn {
		font-size: 12px;
		color: var(--color-accent);
		padding: var(--spacing-xs) var(--spacing-sm);
		border: 1px solid var(--color-accent-dim);
		border-radius: var(--radius-sm);
		transition: background var(--transition-fast), color var(--transition-fast);
		letter-spacing: 0.03em;
	}

	.pair-btn:hover {
		background: var(--color-accent-dim);
		color: var(--color-bg);
	}

	.settings-btn {
		color: var(--color-text-muted);
		padding: var(--spacing-xs);
		border-radius: var(--radius-sm);
		display: flex;
		align-items: center;
		transition: color var(--transition-fast);
	}

	.settings-btn:hover {
		color: var(--color-text);
	}
</style>
