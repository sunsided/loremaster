<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** Short facts, for example the turn, the time and the room. */
		items: string[];
		/** Controls at the end of the bar. */
		actions?: Snippet;
	}

	let { items, actions }: Props = $props();
</script>

<div class="status">
	<ul aria-label="Game status">
		{#each items as item, i (i)}<li>{item}</li>{/each}
	</ul>
	{#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
	.status {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-4) var(--space-7);
		color: var(--color-text-muted);
	}
	ul {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-7);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-6);
		margin-left: auto;
	}
</style>
