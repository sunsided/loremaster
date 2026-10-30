<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** `inline` in a line of text; `block` for a line of its own. */
		look?: 'inline' | 'block';
		children: Snippet;
	}

	let { look = 'inline', children }: Props = $props();
</script>

<!--
	Text that an AI proposal suggests and nobody accepted yet. It has three cues, so colour is not the only one:
	the ghost colour, italics with a dotted line, and a ✦ mark. Screen readers hear "proposal:" before it.
-->
<span class="ghost {look}"><span class="mark" aria-hidden="true">✦ </span><span class="visually-hidden">proposal: </span>{@render children()}</span>

<style>
	.ghost {
		color: var(--color-ghost);
		font-style: italic;
		text-decoration: underline dotted var(--color-ghost);
		text-underline-offset: 3px;
		background: var(--color-ghost-tint);
	}
	.block {
		display: block;
		padding: var(--space-1) var(--space-3);
	}
	.mark {
		font-style: normal;
	}
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
