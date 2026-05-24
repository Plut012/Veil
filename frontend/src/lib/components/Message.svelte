<script lang="ts">
	import type { Message } from '$lib/stores/messages.js';

	export let message: Message;

	function formatTime(ts: string): string {
		try {
			const d = new Date(ts);
			return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		} catch {
			return '';
		}
	}
</script>

<div class="message" class:outbound={message.direction === 'out'} class:inbound={message.direction === 'in'}>
	<p class="text">{message.text}</p>
	<span class="time">{formatTime(message.timestamp)}</span>
</div>

<style>
	.message {
		display: flex;
		flex-direction: column;
		max-width: 72%;
		padding: var(--spacing-sm) var(--spacing-md);
		border-radius: var(--radius-md);
		margin-bottom: var(--spacing-xs);
		word-break: break-word;
	}

	.inbound {
		align-self: flex-start;
		background: var(--color-msg-in);
		border-bottom-left-radius: var(--radius-sm);
	}

	.outbound {
		align-self: flex-end;
		background: var(--color-msg-out);
		border-bottom-right-radius: var(--radius-sm);
	}

	.text {
		font-size: 14px;
		line-height: 1.55;
		color: var(--color-text);
	}

	.time {
		font-size: 10px;
		font-family: var(--font-mono);
		color: var(--color-text-muted);
		margin-top: var(--spacing-xs);
		align-self: flex-end;
	}

	.inbound .time {
		align-self: flex-start;
	}
</style>
