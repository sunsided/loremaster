<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** `neutral` for facts. `warning` for a state that needs attention. */
		tone?: 'neutral' | 'warning';
		children: Snippet;
	}

	let { tone = 'neutral', children }: Props = $props();
</script>

<!-- A label that shows a value. It is not a control. -->
<span class="tag {tone}">{@render children()}</span>

<style>
	.tag {
		display: inline-flex;
		align-items: center;
		min-height: var(--size-target-compact);
		box-sizing: border-box;
		padding: var(--space-2) var(--space-5);
		border: var(--size-border) solid var(--color-border-subtle);
		background: var(--color-surface-control);
		color: var(--color-text);
	}
	.warning {
		color: var(--color-text-warning);
	}
</style>
