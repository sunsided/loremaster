<script lang="ts">
	interface Props {
		/** The fragment number, starting at 1. */
		number: number;
		/** Set when the fragment is highlighted. */
		active?: boolean;
		/** Called with true when the marker gets hover or focus, and with false when it loses them. */
		onactivate?: (on: boolean) => void;
	}

	let { number, active = false, onactivate }: Props = $props();
</script>

<!-- The number of a fragment, shown before its text. Hover or focus highlights the fragment in the assembly panel. -->
<button
	class="marker"
	class:active
	aria-label="Fragment {number}"
	aria-pressed={active}
	onmouseenter={() => onactivate?.(true)}
	onmouseleave={() => onactivate?.(false)}
	onfocus={() => onactivate?.(true)}
	onblur={() => onactivate?.(false)}
	onclick={() => onactivate?.(!active)}>{number}</button
>

<style>
	.marker {
		vertical-align: super;
		border: 0;
		padding: 0 var(--space-1) 0 0;
		background: none;
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		line-height: 1;
		color: var(--color-text-warning);
		text-shadow: none;
	}
	.active {
		font-weight: var(--font-weight-strong);
		color: var(--color-warning);
	}
</style>
