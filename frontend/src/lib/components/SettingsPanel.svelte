<script lang="ts">
	import { showSettings } from '$lib/stores/ui.js';
	import { veilSocket } from '$lib/api/websocket.js';
	import EnvelopeConfig from './EnvelopeConfig.svelte';

	const themes = [
		{ id: 'art-nouveau', label: 'Art Nouveau' }
	];

	let selectedTheme = 'art-nouveau';

	function applyTheme(themeId: string) {
		selectedTheme = themeId;
		veilSocket.setTheme(themeId);
	}

	function closeSettings() {
		showSettings.set(false);
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') closeSettings();
	}
</script>

<svelte:window on:keydown={onKeydown} />

<div class="settings-panel" role="complementary" aria-label="Settings">
	<header class="settings-header">
		<span class="settings-title">Settings</span>
		<button class="close-btn" on:click={closeSettings} aria-label="Close settings">
			<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
				<path d="M1 1l10 10M11 1L1 11" stroke-linecap="round" />
			</svg>
		</button>
	</header>

	<div class="settings-body">
		<section class="settings-section">
			<h3 class="section-title">Envelope</h3>
			<EnvelopeConfig />
		</section>

		<section class="settings-section">
			<h3 class="section-title">Theme</h3>
			<div class="theme-list">
				{#each themes as theme (theme.id)}
					<button
						class="theme-option"
						class:active={selectedTheme === theme.id}
						on:click={() => applyTheme(theme.id)}
					>
						<span class="theme-swatch" data-theme={theme.id}></span>
						<span class="theme-label">{theme.label}</span>
					</button>
				{/each}
			</div>
		</section>
	</div>
</div>

<style>
	.settings-panel {
		position: fixed;
		top: 0;
		right: 0;
		bottom: 0;
		width: 300px;
		background: var(--color-bg-surface);
		border-left: 1px solid var(--color-border);
		display: flex;
		flex-direction: column;
		z-index: 50;
	}

	.settings-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border);
	}

	.settings-title {
		font-size: 12px;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--color-text-muted);
	}

	.close-btn {
		color: var(--color-text-muted);
		padding: var(--spacing-xs);
		border-radius: var(--radius-sm);
		display: flex;
		align-items: center;
		transition: color var(--transition-fast);
	}

	.close-btn:hover {
		color: var(--color-text);
	}

	.settings-body {
		flex: 1;
		overflow-y: auto;
		padding: var(--spacing-md);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
	}

	.settings-section {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}

	.section-title {
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--color-accent);
		font-weight: 500;
	}

	/* Theme selector */
	.theme-list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}

	.theme-option {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		text-align: left;
		transition: background var(--transition-fast), border-color var(--transition-fast), color var(--transition-fast);
	}

	.theme-option:hover {
		background: var(--color-bg-elevated);
		color: var(--color-text);
	}

	.theme-option.active {
		border-color: var(--color-accent-dim);
		color: var(--color-text);
	}

	.theme-swatch {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.theme-swatch[data-theme="art-nouveau"] {
		background: linear-gradient(135deg, #c4956a 40%, #2a2520 40%);
		border: 1px solid var(--color-border);
	}

	.theme-label {
		font-size: 12px;
	}
</style>
