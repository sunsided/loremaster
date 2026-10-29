<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		name: string;
		value: string;
		checked: boolean;
		onselect: (value: string) => void;
		children: Snippet;
	}

	let { name, value, checked, onselect, children }: Props = $props();
</script>

<!-- One choice of a group. Use RadioGroup to make a group. -->
<label class="radio">
	<input type="radio" {name} {value} {checked} onchange={() => onselect(value)} />
	{@render children()}
</label>

<style>
	.radio {
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
</style>
