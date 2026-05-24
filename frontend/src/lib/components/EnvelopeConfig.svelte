<script lang="ts">
	import { veilSocket } from '$lib/api/websocket.js';

	let template = '{ciphertext}';
	let saved = false;
	let saveTimer: ReturnType<typeof setTimeout>;

	function save() {
		veilSocket.updateEnvelope(template);
		saved = true;
		clearTimeout(saveTimer);
		saveTimer = setTimeout(() => {
			saved = false;
		}, 2000);
	}
</script>

<div class="envelope-config">
	<label class="field-label" for="envelope-template">Envelope template</label>
	<textarea
		id="envelope-template"
		class="template-input"
		bind:value={template}
		rows="2"
		spellcheck="false"
	></textarea>
	<p class="hint">Use <code>{'{ciphertext}'}</code> as the placeholder. Example: <code>~~ {'{ciphertext}'} ~~</code></p>
	<button class="save-btn" on:click={save}>
		{saved ? 'saved' : 'apply'}
	</button>
</div>

<style>
	.envelope-config {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	.field-label {
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-text-muted);
	}

	.template-input {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		padding: var(--spacing-sm);
		color: var(--color-text);
		font-family: var(--font-mono);
		font-size: 12px;
		resize: vertical;
		min-height: 52px;
		transition: border-color var(--transition-fast);
	}

	.template-input:focus {
		outline: none;
		border-color: var(--color-accent-dim);
	}

	.hint {
		font-size: 11px;
		color: var(--color-text-muted);
		line-height: 1.6;
	}

	code {
		font-family: var(--font-mono);
		font-size: 11px;
		background: var(--color-bg-elevated);
		padding: 1px 3px;
		border-radius: 2px;
	}

	.save-btn {
		align-self: flex-start;
		font-size: 12px;
		color: var(--color-accent);
		padding: var(--spacing-xs) var(--spacing-sm);
		border: 1px solid var(--color-accent-dim);
		border-radius: var(--radius-sm);
		transition: background var(--transition-fast), color var(--transition-fast);
		letter-spacing: 0.03em;
	}

	.save-btn:hover {
		background: var(--color-accent-dim);
		color: var(--color-bg);
	}
</style>
