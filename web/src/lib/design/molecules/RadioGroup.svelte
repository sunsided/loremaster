<script lang="ts" generics="T extends string">
	import Radio from '../atoms/Radio.svelte';
	import type { RadioOption } from '../types.ts';

	interface Props {
		/** The question that the group answers. It is the fieldset legend. */
		legend: string;
		name: string;
		options: RadioOption<T>[];
		value: T;
		/** `pills`: one line of short choices. `cards`: choices with descriptions. `list`: one choice per line. */
		look?: 'pills' | 'cards' | 'list';
	}

	let { legend, name, options, value = $bindable(), look = 'list' }: Props = $props();
</script>

<fieldset class="group {look}">
	<legend>{legend}</legend>
	<div class="options">
		{#each options as option (option.value)}
			<div class="option" class:on={option.value === value}>
				<Radio {name} value={option.value} checked={option.value === value} onselect={(v) => (value = v as T)}>
					<span class="label">{option.label}</span>
				</Radio>
				{#if option.description && look === 'cards'}
					<p class="description">{option.description}</p>
				{/if}
			</div>
		{/each}
	</div>
</fieldset>

<style>
	.group {
		border: 0;
		margin: 0;
		padding: 0;
		min-width: 0;
	}
	legend {
		padding: 0;
		margin-bottom: var(--space-4);
		font-weight: var(--font-weight-strong);
	}
	.options {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.pills .options {
		flex-direction: row;
		flex-wrap: wrap;
		gap: var(--space-4);
	}
	.pills .option {
		padding: 0 var(--space-5);
		border: var(--size-border) solid var(--color-border-control);
	}
	.cards .options {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: var(--space-5);
	}
	.cards .option {
		padding: var(--space-4) var(--space-5) var(--space-5);
		border: var(--size-border) solid var(--color-border-control);
	}
	.pills .on,
	.cards .on {
		border-color: var(--color-accent);
		box-shadow: inset 0 0 0 1px var(--color-accent);
	}
	.cards .label {
		font-weight: var(--font-weight-strong);
	}
	.description {
		margin: var(--space-2) 0 0;
		color: var(--color-text-muted);
		line-height: var(--line-height-body);
	}
</style>
