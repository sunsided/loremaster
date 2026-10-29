<script lang="ts">
	import Button from '../atoms/Button.svelte';
	import type { ActionItem } from '../types.ts';

	interface Props {
		/** The name of the list, for example "here" or "carrying". */
		label: string;
		items: ActionItem[];
		/** The text when the list has no items. */
		empty: string;
		onpick: (item: ActionItem) => void;
		/** Ids of items to mark, for example the things the typed command names. */
		marked?: string[];
	}

	let { label, items, empty, onpick, marked = [] }: Props = $props();
	const id = $props.id();
</script>

<!-- A labelled row of compact buttons. Each button is one thing or exit that the player can use. -->
<div class="action-list" role="group" aria-labelledby="{id}-label">
	<span id="{id}-label" class="label">{label}</span>
	{#each items as item (item.id)}
		<Button size="compact" variant="quiet" marked={marked.includes(item.id)} onclick={() => onpick(item)}
			>{item.label}{#if item.detail}<span class="detail">({item.detail})</span>{/if}{#if marked.includes(item.id)}<span
					class="visually-hidden">, in the command</span
				>{/if}</Button
		>
	{:else}
		<span class="empty">{empty}</span>
	{/each}
</div>

<style>
	.action-list {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
	}
	.label,
	.empty,
	.detail {
		color: var(--color-text-muted);
	}
	.label {
		margin-right: var(--space-1);
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
