<script lang="ts">
	import '../app.css';

	// Load the active theme CSS dynamically.
	// The art-nouveau theme is the default; switching themes swaps this link tag.
	import { onMount } from 'svelte';

	let themeLink: HTMLLinkElement | null = null;

	onMount(() => {
		themeLink = document.createElement('link');
		themeLink.rel = 'stylesheet';
		themeLink.href = '/themes/art-nouveau/theme.css';
		document.head.appendChild(themeLink);

		return () => {
			if (themeLink) document.head.removeChild(themeLink);
		};
	});

	// Exposed for theme switching (called by SettingsPanel indirectly)
	export function switchTheme(themeId: string) {
		if (themeLink) {
			themeLink.href = `/themes/${themeId}/theme.css`;
		}
	}
</script>

<slot />
