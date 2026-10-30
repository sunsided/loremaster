<script lang="ts">
	interface Props {
		/** The value name, for example `time.phase` or `beacon_lit`. */
		name: string;
		/** The value now. */
		value: string;
		/** The values to choose from. */
		options: string[];
		/** True when the switch shows a value that differs from the game now (a preview). */
		changed?: boolean;
		onchange: (value: string) => void;
	}

	let { name, value, options, changed = false, onchange }: Props = $props();
	const id = $props.id();
</script>

<!--
	One value that the room text depends on, with a choice of its values: `name | value ▾`.
	It is a native select with the value name as its label.
-->
<span class="state-switch" class:changed>
	<label for={id}>{name}</label>
	<select {id} {value} onchange={(e) => onchange(e.currentTarget.value)}>
		{#each options as o (o)}<option value={o}>{o}</option>{/each}
	</select>
</span>

<style>
	.state-switch {
		display: inline-flex;
		align-items: stretch;
		border: var(--size-border) solid var(--color-border-control);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
	}
	.changed {
		border-color: var(--color-warning);
	}
	label {
		display: inline-flex;
		align-items: center;
		padding: 0 var(--space-3);
		border-right: var(--size-border) solid var(--color-border-control);
		color: var(--color-text-strong);
	}
	select {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-2);
		border: 0;
		border-radius: 0;
		background: var(--color-surface-control);
		color: var(--color-text-accent);
		font: inherit;
	}
	.changed select {
		color: var(--color-text-warning);
	}
	select:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 1px;
	}
</style>
