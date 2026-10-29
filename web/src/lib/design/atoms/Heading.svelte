<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** The heading level in the document outline. It does not change the look. */
		level: 1 | 2 | 3 | 4;
		/** `title`: a dialog or page title. `section`: a small heading in capitals. */
		look?: 'title' | 'section';
		id?: string;
		children: Snippet;
	}

	let { level, look = 'section', id, children }: Props = $props();
</script>

<svelte:element this={`h${level}`} {id} class="heading {look}">{@render children()}</svelte:element>

<style>
	.heading {
		margin: 0;
		font-weight: var(--font-weight-strong);
		letter-spacing: var(--letter-spacing-caps);
	}
	.title {
		font-size: var(--font-size-heading);
		color: var(--color-text-accent);
		text-transform: uppercase;
	}
	.section {
		font-size: var(--font-size-md);
		color: var(--color-text);
		text-transform: uppercase;
	}
</style>
