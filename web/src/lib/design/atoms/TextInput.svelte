<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	interface Props extends Omit<HTMLInputAttributes, 'value'> {
		id: string;
		/** The label. It is always in the page. Set `hideLabel` to hide it visually only. */
		label: string;
		hideLabel?: boolean;
		value?: string;
		/** The input element, for focus from outside. */
		element?: HTMLInputElement;
	}

	let {
		id,
		label,
		hideLabel = false,
		value = $bindable(''),
		element = $bindable(),
		...rest
	}: Props = $props();
</script>

<label for={id} class:visually-hidden={hideLabel}>{label}</label>
<input {id} class="input" bind:value bind:this={element} {...rest} />

<style>
	.input {
		box-sizing: border-box;
		min-height: var(--size-target);
		min-width: 0;
		flex: 1 1 auto;
		padding: var(--space-3) var(--space-1);
		border: 0;
		border-bottom: var(--size-border) solid var(--color-border-control);
		background: transparent;
		color: var(--color-text-strong);
		caret-color: var(--color-accent);
		caret-shape: var(--caret-shape, auto);
		font-family: var(--font-ui);
		font-size: var(--font-size-lg);
	}
	.input::placeholder {
		color: var(--color-text-muted);
	}
	.input:focus-visible {
		outline: none;
		border-bottom: var(--size-border-strong) solid var(--color-focus);
	}
	label {
		color: var(--color-text-muted);
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
