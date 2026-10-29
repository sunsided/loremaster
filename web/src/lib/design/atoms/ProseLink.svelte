<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** `thing`: an object in the story. `exit`: a way out of the room. */
		kind?: 'thing' | 'exit';
		onclick: () => void;
		children: Snippet;
	}

	let { kind = 'thing', onclick, children }: Props = $props();
</script>

<!-- A word in the story text that the player can click. It is a real button, so the keyboard can reach it. -->
<button class="prose-link {kind}" {onclick}>{@render children()}</button>

<style>
	.prose-link {
		display: inline;
		border: 0;
		padding: 0;
		background: none;
		font: inherit;
		line-height: inherit;
		color: var(--color-text-strong);
		text-shadow: inherit;
		text-decoration: underline;
		text-decoration-color: var(--color-border-control);
		text-underline-offset: 3px;
	}
	.exit {
		color: var(--color-text);
	}
	.prose-link:hover {
		color: var(--color-text-accent);
		text-decoration-color: var(--color-accent);
	}
</style>
