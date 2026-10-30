<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	interface Props extends HTMLButtonAttributes {
		/** The accessible name. It is also the tooltip. */
		label: string;
		/** The keyboard shortcut, for the tooltip and aria-keyshortcuts, for example "Ctrl K". */
		shortcut?: string;
		/** The icon: a small SVG or one or two characters. It is hidden from screen readers. */
		children: Snippet;
	}

	let { label, shortcut, children, ...rest }: Props = $props();
	const keys = $derived(shortcut ? shortcut.replace(/\bCtrl\b/g, 'Control').replace(/ /g, '+') : undefined);
</script>

<!--
	A small square button with an icon. The name shows as a tooltip on hover and focus,
	and screen readers read it as the name of the button.
-->
<span class="wrap">
	<button type="button" class="icon-button" aria-label={label} aria-keyshortcuts={keys} {...rest}>
		<span aria-hidden="true">{@render children()}</span>
	</button>
	<span class="tip" aria-hidden="true">{label}{#if shortcut}<kbd>{shortcut}</kbd>{/if}</span>
</span>

<style>
	.wrap {
		position: relative;
		display: inline-flex;
	}
	.icon-button {
		display: inline-grid;
		place-items: center;
		min-width: var(--size-target-compact);
		height: var(--size-target-compact);
		padding: 0 var(--space-3);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-control);
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.icon-button:hover {
		border-color: var(--color-accent);
		color: var(--color-text-accent);
	}
	.icon-button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.icon-button[aria-expanded='true'] {
		border-color: var(--color-accent);
		color: var(--color-text-accent);
	}
	.tip {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		z-index: 30;
		display: none;
		gap: var(--space-3);
		align-items: center;
		padding: var(--space-2) var(--space-3);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		color: var(--color-text);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		white-space: nowrap;
		pointer-events: none;
	}
	.wrap:hover .tip,
	.icon-button:focus-visible + .tip {
		display: flex;
	}
	kbd {
		padding: 0 var(--space-2);
		border: var(--size-border) solid var(--color-border-control);
		font: inherit;
		color: var(--color-text-muted);
	}
</style>
