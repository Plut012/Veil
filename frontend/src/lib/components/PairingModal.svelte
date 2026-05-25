<script lang="ts">
	import { onDestroy } from 'svelte';
	import { showPairingModal, pairingRole } from '$lib/stores/ui.js';
	import { addContact } from '$lib/stores/contacts.js';
	import { initiatePairing, completePairing } from '$lib/api/tauri.js';

	let qrImage: string | null = null;
	let scanning = false;
	let scannerEl: HTMLDivElement;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let html5QrScanner: any = null;
	let scanError: string | null = null;
	let pairingStatus: 'idle' | 'waiting' | 'done' = 'idle';

	function close() {
		stopScanner();
		qrImage = null;
		pairingStatus = 'idle';
		scanError = null;
		pairingRole.set(null);
		showPairingModal.set(false);
	}

	function selectRole(role: 'initiate' | 'join') {
		pairingRole.set(role);
		if (role === 'initiate') {
			startInitiate();
		} else {
			startJoin();
		}
	}

	async function startInitiate() {
		pairingStatus = 'waiting';
		try {
			const pngBase64 = await initiatePairing();
			qrImage = `data:image/png;base64,${pngBase64}`;
		} catch (err) {
			console.error('[pairing] initiatePairing failed:', err);
			pairingStatus = 'idle';
		}
	}

	function startJoin() {
		scanning = true;
	}

	async function mountScanner() {
		if (!scannerEl) return;
		try {
			const { Html5Qrcode } = await import('html5-qrcode');
			html5QrScanner = new Html5Qrcode('qr-scanner-container');
			await html5QrScanner.start(
				{ facingMode: 'environment' },
				{ fps: 10, qrbox: { width: 220, height: 220 } },
				async (decodedText: string) => {
					stopScanner();
					scanning = false;
					pairingStatus = 'waiting';
					try {
						const contact = await completePairing(decodedText);
						addContact(contact);
						pairingStatus = 'done';
						setTimeout(close, 1200);
					} catch (err) {
						console.error('[pairing] completePairing failed:', err);
						pairingStatus = 'idle';
					}
				},
				undefined
			);
		} catch (err) {
			scanError = 'Camera unavailable. Check browser permissions.';
			console.error('[pairing] scanner error', err);
		}
	}

	async function stopScanner() {
		if (html5QrScanner) {
			try {
				await html5QrScanner.stop();
				html5QrScanner.clear();
			} catch {
				// ignore
			}
			html5QrScanner = null;
		}
	}

	onDestroy(() => {
		stopScanner();
	});

	// Start scanner after DOM is ready
	$: if (scanning && scannerEl) {
		mountScanner();
	}

	function onBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) close();
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') close();
	}
</script>

<svelte:window on:keydown={onKeydown} />

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="backdrop" on:click={onBackdropClick}>
	<div class="modal" role="dialog" aria-modal="true" aria-label="Pair a contact">
		<button class="close-btn" on:click={close} aria-label="Close">
			<svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5">
				<path d="M1 1l10 10M11 1L1 11" stroke-linecap="round" />
			</svg>
		</button>

		{#if !$pairingRole}
			<!-- Role selection -->
			<div class="role-select">
				<p class="modal-title">New contact</p>
				<div class="role-buttons">
					<button class="role-btn" on:click={() => selectRole('initiate')}>
						<span class="role-icon">
							<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.3">
								<rect x="3" y="3" width="14" height="14" rx="1" />
								<rect x="6" y="6" width="3" height="3" />
								<rect x="11" y="6" width="3" height="3" />
								<rect x="6" y="11" width="3" height="3" />
								<rect x="11" y="11" width="2" height="2" />
							</svg>
						</span>
						<span class="role-label">Show QR</span>
					</button>
					<button class="role-btn" on:click={() => selectRole('join')}>
						<span class="role-icon">
							<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.3">
								<path d="M2 10a8 8 0 1 0 16 0 8 8 0 0 0-16 0z" />
								<circle cx="10" cy="10" r="3" />
							</svg>
						</span>
						<span class="role-label">Scan QR</span>
					</button>
				</div>
			</div>

		{:else if $pairingRole === 'initiate'}
			<!-- Initiator view -->
			<div class="initiate-view">
				<p class="modal-title">Show this QR to your contact</p>
				{#if pairingStatus === 'done'}
					<p class="status-msg success">Paired.</p>
				{:else if qrImage}
					<div class="qr-wrapper">
						<img src={qrImage} alt="Pairing QR code" class="qr-image" />
					</div>
					<p class="qr-hint">The key is ephemeral. Dismiss after scanning.</p>
				{:else}
					<div class="qr-placeholder">
						<span class="loading-dots">...</span>
					</div>
				{/if}
			</div>

		{:else if $pairingRole === 'join'}
			<!-- Joiner view -->
			<div class="join-view">
				{#if pairingStatus === 'idle' || scanning}
					<p class="modal-title">Scan your contact's QR</p>
					<div
						class="scanner-container"
						bind:this={scannerEl}
					>
						<div id="qr-scanner-container"></div>
						{#if scanError}
							<p class="scan-error">{scanError}</p>
						{/if}
					</div>
				{:else if pairingStatus === 'waiting'}
					<p class="modal-title">Completing pairing...</p>
					<div class="qr-placeholder"><span class="loading-dots">...</span></div>
				{:else if pairingStatus === 'done'}
					<p class="status-msg success">Paired.</p>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.65);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.modal {
		position: relative;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--spacing-lg);
		width: 320px;
		max-width: 90vw;
	}

	.close-btn {
		position: absolute;
		top: var(--spacing-md);
		right: var(--spacing-md);
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

	.modal-title {
		font-size: 13px;
		color: var(--color-text-muted);
		margin-bottom: var(--spacing-md);
		letter-spacing: 0.03em;
	}

	/* Role selection */
	.role-select {
		display: flex;
		flex-direction: column;
	}

	.role-buttons {
		display: flex;
		gap: var(--spacing-md);
	}

	.role-btn {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
		transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
	}

	.role-btn:hover {
		background: var(--color-bg-elevated);
		color: var(--color-text);
		border-color: var(--color-accent-dim);
	}

	.role-icon {
		display: flex;
		align-items: center;
		color: var(--color-accent);
	}

	.role-label {
		font-size: 12px;
		letter-spacing: 0.04em;
	}

	/* QR display */
	.qr-wrapper {
		display: flex;
		justify-content: center;
		margin: var(--spacing-md) 0;
	}

	.qr-image {
		width: 220px;
		height: 220px;
		border-radius: var(--radius-sm);
		image-rendering: pixelated;
	}

	.qr-hint {
		font-size: 11px;
		color: var(--color-text-muted);
		text-align: center;
		font-style: italic;
	}

	.qr-placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 220px;
		color: var(--color-text-muted);
	}

	.loading-dots {
		font-family: var(--font-mono);
		font-size: 18px;
		letter-spacing: 0.2em;
		animation: pulse 1.4s ease infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 0.3; }
		50% { opacity: 1; }
	}

	/* Scanner */
	.scanner-container {
		min-height: 260px;
		display: flex;
		flex-direction: column;
		align-items: center;
	}

	#qr-scanner-container {
		width: 280px;
	}

	:global(#qr-scanner-container video) {
		border-radius: var(--radius-sm);
	}

	.scan-error {
		margin-top: var(--spacing-md);
		font-size: 12px;
		color: var(--color-error);
		text-align: center;
	}

	.status-msg {
		text-align: center;
		padding: var(--spacing-lg) 0;
		font-size: 13px;
	}

	.status-msg.success {
		color: var(--color-success);
		letter-spacing: 0.05em;
	}
</style>
