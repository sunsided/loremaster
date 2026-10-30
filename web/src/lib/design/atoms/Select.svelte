<script lang="ts" generics="T extends string">
	interface Props {
		/** The visible label. */
		label: string;
		value: T;
		options: { value: T; label: string }[];
		/** Hide the label visually. Screen readers still read it. */
		hideLabel?: boolean;
		/** Called when the user picks an option. */
		onchange?: (event: Event) => void;
	}

	let { label, value = $bindable(), options, hideLabel = false, onchange }: Props = $props();
</script>

<!-- A native select with a label. The browser gives it keyboard and screen-reader support. -->
<label class="select">
	<span class="label" class:visually-hidden={hideLabel}>{label}</span>
	<select bind:value {onchange}>
		{#each options as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
	</select>
</label>

<style>
	.select {
		display: inline-flex;
		align-items: center;
		gap: var(--space-3);
		font-size: var(--font-size-sm);
	}
	.label {
		color: var(--color-text-muted);
	}
	select {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-3);
		border: var(--size-border) solid var(--color-border-control);
		border-radius: 0;
		background: var(--color-surface-control);
		color: var(--color-text);
		font: inherit;
	}
	select:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
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
