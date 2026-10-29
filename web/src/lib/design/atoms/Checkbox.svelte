<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		checked?: boolean;
		/** Extra text after the label. Use it for a short hint. */
		hint?: string;
		children: Snippet;
	}

	let { checked = $bindable(false), hint, children }: Props = $props();
</script>

<label class="checkbox">
	<input type="checkbox" bind:checked />
	<span>{@render children()}</span>
	{#if hint}<span class="hint">{hint}</span>{/if}
</label>

<style>
	.checkbox {
		display: inline-flex;
		align-items: center;
		gap: var(--space-4);
		min-height: var(--size-target-compact);
	}
	input {
		width: 18px;
		height: 18px;
		margin: 0;
		accent-color: var(--color-accent);
	}
	.hint {
		color: var(--color-text-muted);
	}
</style>
