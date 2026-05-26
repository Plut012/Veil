<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import {
		submitConfig,
		submitPassphrase,
		requestTelegramCode,
		submitTelegramCode,
		submit2faPassword
	} from '$lib/api/tauri.js';
	import { open } from '@tauri-apps/plugin-shell';

	export let status: string;

	const dispatch = createEventDispatcher();

	// Map initial status to step
	type Step = 'config' | 'passphrase' | 'phone' | 'code' | '2fa';

	function statusToStep(s: string): Step {
		if (s === 'needs_config') return 'config';
		if (s === 'needs_telegram_auth') return 'phone';
		return 'passphrase';
	}

	let step: Step = statusToStep(status);
	let error = '';
	let loading = false;

	// Config step
	let apiId = '';
	let apiHash = '';
	let displayName = '';

	// Passphrase step
	let passphrase = '';

	// Phone step
	let phone = '';

	// Code step
	let code = '';

	// 2FA step
	let twoFaPassword = '';

	function clearError() {
		error = '';
	}

	async function handleConfig() {
		clearError();
		const id = parseInt(apiId, 10);
		if (!id || isNaN(id)) {
			error = 'API ID must be a number.';
			return;
		}
		if (!apiHash.trim()) {
			error = 'API Hash is required.';
			return;
		}
		if (!displayName.trim()) {
			error = 'Display name is required.';
			return;
		}
		loading = true;
		try {
			await submitConfig(id, apiHash.trim(), displayName.trim());
			step = 'passphrase';
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function handlePassphrase() {
		clearError();
		if (!passphrase) {
			error = 'Passphrase is required.';
			return;
		}
		loading = true;
		try {
			const next = await submitPassphrase(passphrase);
			if (next === 'ready') {
				dispatch('complete');
			} else {
				step = 'phone';
			}
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function handlePhone() {
		clearError();
		if (!phone.trim()) {
			error = 'Phone number is required.';
			return;
		}
		loading = true;
		try {
			await requestTelegramCode(phone.trim());
			step = 'code';
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function handleCode() {
		clearError();
		if (!code.trim()) {
			error = 'Code is required.';
			return;
		}
		loading = true;
		try {
			const next = await submitTelegramCode(code.trim());
			if (next === 'needs_2fa') {
				step = '2fa';
			} else {
				dispatch('complete');
			}
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function handle2fa() {
		clearError();
		if (!twoFaPassword) {
			error = 'Password is required.';
			return;
		}
		loading = true;
		try {
			await submit2faPassword(twoFaPassword);
			dispatch('complete');
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function openTelegramApps() {
		try {
			await open('https://my.telegram.org/apps');
		} catch {
			// Fallback: if shell plugin isn't available, let browser handle it
			window.open('https://my.telegram.org/apps', '_blank');
		}
	}

	function handleKeydown(e: KeyboardEvent, handler: () => void) {
		if (e.key === 'Enter') handler();
	}
</script>

<div class="setup-overlay">
	<div class="setup-card">
		{#if step === 'config'}
			<div class="setup-guidance">
				<p class="guidance-text">
					Create an app at
					<!-- svelte-ignore a11y-invalid-attribute -->
					<a href="#" class="guidance-link" on:click|preventDefault={() => openTelegramApps()}>my.telegram.org/apps</a>
					and paste your credentials below.
				</p>
				<p class="guidance-hint">App title and platform can be anything.</p>
			</div>
			<input
				type="number"
				placeholder="API ID"
				bind:value={apiId}
				on:input={clearError}
				on:keydown={(e) => handleKeydown(e, handleConfig)}
				disabled={loading}
				class="setup-input"
				autofocus
			/>
			<input
				type="text"
				placeholder="API Hash"
				bind:value={apiHash}
				on:input={clearError}
				on:keydown={(e) => handleKeydown(e, handleConfig)}
				disabled={loading}
				class="setup-input"
			/>
			<input
				type="text"
				placeholder="Display Name"
				bind:value={displayName}
				on:input={clearError}
				on:keydown={(e) => handleKeydown(e, handleConfig)}
				disabled={loading}
				class="setup-input"
			/>
			{#if error}<span class="setup-error">{error}</span>{/if}
			<button class="setup-btn" on:click={handleConfig} disabled={loading}>
				{loading ? '...' : 'Continue'}
			</button>

		{:else if step === 'passphrase'}
			<input
				type="password"
				placeholder="Passphrase"
				bind:value={passphrase}
				on:input={clearError}
				on:keydown={(e) => handleKeydown(e, handlePassphrase)}
				disabled={loading}
				class="setup-input"
				autofocus
			/>
			{#if error}<span class="setup-error">{error}</span>{/if}
			<button class="setup-btn" on:click={handlePassphrase} disabled={loading}>
				{loading ? '...' : 'Unlock'}
			</button>

		{:else if step === 'phone'}
			<input
				type="tel"
				placeholder="+1 415 555 0132"
				bind:value={phone}
				on:input={clearError}
				on:keydown={(e) => handleKeydown(e, handlePhone)}
				disabled={loading}
				class="setup-input"
				autofocus
			/>
			{#if error}<span class="setup-error">{error}</span>{/if}
			<button class="setup-btn" on:click={handlePhone} disabled={loading}>
				{loading ? '...' : 'Send Code'}
			</button>

		{:else if step === 'code'}
			<input
				type="text"
				placeholder="Login code"
				bind:value={code}
				on:input={clearError}
				on:keydown={(e) => handleKeydown(e, handleCode)}
				disabled={loading}
				class="setup-input"
				autofocus
			/>
			{#if error}<span class="setup-error">{error}</span>{/if}
			<button class="setup-btn" on:click={handleCode} disabled={loading}>
				{loading ? '...' : 'Verify'}
			</button>

		{:else if step === '2fa'}
			<input
				type="password"
				placeholder="2FA password"
				bind:value={twoFaPassword}
				on:input={clearError}
				on:keydown={(e) => handleKeydown(e, handle2fa)}
				disabled={loading}
				class="setup-input"
				autofocus
			/>
			{#if error}<span class="setup-error">{error}</span>{/if}
			<button class="setup-btn" on:click={handle2fa} disabled={loading}>
				{loading ? '...' : 'Submit'}
			</button>
		{/if}
	</div>
</div>

<style>
	.setup-overlay {
		position: fixed;
		inset: 0;
		background: var(--color-bg);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.setup-card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		width: 280px;
	}

	.setup-input {
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-text);
		padding: var(--spacing-sm) 0;
		font-size: 14px;
		width: 100%;
		outline: none;
		transition: border-color var(--transition-fast);
	}

	.setup-input:focus {
		border-bottom-color: var(--color-accent);
	}

	.setup-input::placeholder {
		color: var(--color-text-muted);
	}

	.setup-input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Hide number input spinners */
	.setup-input[type='number']::-webkit-inner-spin-button,
	.setup-input[type='number']::-webkit-outer-spin-button {
		-webkit-appearance: none;
		margin: 0;
	}
	.setup-input[type='number'] {
		-moz-appearance: textfield;
	}

	.setup-guidance {
		margin-bottom: var(--spacing-sm);
	}

	.guidance-text {
		font-size: 13px;
		color: var(--color-text-muted);
		line-height: 1.5;
	}

	.guidance-link {
		color: var(--color-accent);
		text-decoration: none;
		border-bottom: 1px solid var(--color-accent-dim);
		transition: color var(--transition-fast), border-color var(--transition-fast);
		cursor: pointer;
	}

	.guidance-link:hover {
		color: var(--color-text);
		border-color: var(--color-text);
	}

	.guidance-hint {
		font-size: 11px;
		color: var(--color-text-muted);
		opacity: 0.6;
		margin-top: var(--spacing-xs);
	}

	.setup-error {
		font-size: 12px;
		color: var(--color-error);
		margin-top: calc(var(--spacing-xs) * -1);
	}

	.setup-btn {
		align-self: flex-start;
		background: none;
		border: none;
		color: var(--color-accent);
		font-size: 13px;
		letter-spacing: 0.04em;
		padding: var(--spacing-xs) 0;
		cursor: pointer;
		transition: color var(--transition-fast);
	}

	.setup-btn:hover:not(:disabled) {
		color: var(--color-text);
	}

	.setup-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
</style>
