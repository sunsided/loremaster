<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	interface Props extends HTMLButtonAttributes {
		/** `primary`: the main action. `secondary`: a normal action. `quiet`: a low-priority action. */
		variant?: 'primary' | 'secondary' | 'quiet';
		/** `default` is 44px high. `compact` is 32px high. */
		size?: 'default' | 'compact';
		/** Set for a toggle button. The value goes to `aria-pressed`. */
		pressed?: boolean;
		/** Make the button fill the width of its grid cell. */
		block?: boolean;
		/** Show that the button is part of something else, for example a thing named in the typed command. */
		marked?: boolean;
		children: Snippet;
	}

	let {
		variant = 'secondary',
		size = 'default',
		pressed,
		block = false,
		marked = false,
		type = 'button',
		children,
		...rest
	}: Props = $props();
</script>

<button
	{type}
	class="button {variant} {size}"
	class:block
	class:marked
	aria-pressed={pressed}
	{...rest}>{@render children()}</button
>

<style>
	.button {
		display: inline-flex;
		align-items: center;
		gap: var(--space-3);
		box-sizing: border-box;
		border: var(--size-border) solid var(--color-border-control);
		border-radius: var(--radius-control);
		background: var(--color-surface-control);
		color: var(--color-text);
		text-align: left;
		transition:
			border-color var(--duration-fast),
			background var(--duration-fast);
	}
	.default {
		min-height: var(--size-target);
		padding: var(--space-4) var(--space-5);
	}
	.compact {
		min-height: var(--size-target-compact);
		padding: var(--space-2) var(--space-5);
	}
	.block {
		width: 100%;
	}
	.primary,
	.button[aria-pressed='true'] {
		border-color: var(--color-accent);
		background: var(--color-accent);
		color: var(--color-text-on-accent);
		font-weight: var(--font-weight-strong);
	}
	.quiet {
		background: transparent;
	}
	.marked {
		border-color: var(--color-accent);
		box-shadow: inset 0 0 0 1px var(--color-accent);
		background: var(--color-accent-tint);
	}
	.button:not(:disabled):hover {
		border-color: var(--color-accent);
	}
	.button:disabled {
		color: var(--color-text-muted);
		border-style: dashed;
	}
</style>
