<script lang="ts">
	import type { TabItem } from '../types.ts';

	interface Props {
		/** The name of the tab list for screen readers. */
		label: string;
		tabs: TabItem[];
		selected: string;
		/** A prefix for the ids of the tabs and panels. It must be unique in the page. */
		idBase: string;
	}

	let { label, tabs, selected = $bindable(), idBase }: Props = $props();

	let buttons: HTMLButtonElement[] = $state([]);

	// Arrow keys move between tabs and select them, as the WAI-ARIA tabs pattern says.
	function onkeydown(event: KeyboardEvent, index: number) {
		const last = tabs.length - 1;
		const next =
			event.key === 'ArrowRight' ? (index === last ? 0 : index + 1)
			: event.key === 'ArrowLeft' ? (index === 0 ? last : index - 1)
			: event.key === 'Home' ? 0
			: event.key === 'End' ? last
			: -1;
		if (next < 0) return;
		event.preventDefault();
		selected = tabs[next].id;
		buttons[next]?.focus();
	}
</script>

<div class="tab-list" role="tablist" aria-label={label}>
	{#each tabs as tab, i (tab.id)}
		<button
			bind:this={buttons[i]}
			role="tab"
			id="{idBase}-tab-{tab.id}"
			aria-selected={tab.id === selected}
			aria-controls="{idBase}-panel"
			tabindex={tab.id === selected ? 0 : -1}
			onclick={() => (selected = tab.id)}
			onkeydown={(e) => onkeydown(e, i)}>{tab.label}</button
		>
	{/each}
</div>

<style>
	.tab-list {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
	}
	button {
		min-height: var(--size-target-compact);
		padding: var(--space-3) var(--space-5);
		border: 0;
		border-bottom: var(--size-border-strong) solid transparent;
		margin-bottom: calc(-1 * var(--size-border));
		background: transparent;
		color: var(--color-text-muted);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-strong);
	}
	button:hover {
		color: var(--color-text);
	}
	button[aria-selected='true'] {
		border-bottom-color: var(--color-accent);
		color: var(--color-text-accent);
	}
</style>
