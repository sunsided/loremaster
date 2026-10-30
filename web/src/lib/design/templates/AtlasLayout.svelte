<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		header: Snippet;
		/** The views of the Atlas, for example Map, Rules, State, Story and Checks. */
		tabs: { id: string; label: string }[];
		selected: string;
		onselect: (id: string) => void;
		/** The content of the selected view. */
		children: Snippet;
		/** A panel on the right, for example the room card of the map. */
		aside?: Snippet;
	}

	let { header, tabs, selected, onselect, children, aside }: Props = $props();
</script>

<!--
	The Atlas: the whole world at full width. It takes the place of the player screen (Ctrl+M or the room name in
	the status line), and Escape or "Back to …" returns to the game at the same turn.
	Each view gets the full page, so tabs are fine here.
-->
<div class="atlas-layout">
	<div class="frame">
		{@render header()}
		<nav class="tabs" aria-label="Atlas views">
			{#each tabs as t (t.id)}
				<button type="button" aria-current={t.id === selected ? 'page' : undefined} onclick={() => onselect(t.id)}>{t.label}</button>
			{/each}
		</nav>
		<div class="body" class:has-aside={!!aside}>
			<main class="content" aria-label="Atlas">{@render children()}</main>
			{#if aside}<aside class="aside" aria-label="Details">{@render aside()}</aside>{/if}
		</div>
	</div>
</div>

<style>
	.atlas-layout {
		height: 100dvh;
		background: var(--color-surface-outer);
	}
	.frame {
		display: flex;
		flex-direction: column;
		height: 100%;
		max-width: var(--width-content-max);
		margin: 0 auto;
		background: var(--color-surface);
	}
	.tabs {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-5);
		padding: 0 var(--space-6);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
	}
	.tabs button {
		min-height: var(--size-target);
		padding: 0;
		border: 0;
		border-bottom: var(--size-border-strong) solid transparent;
		background: none;
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
	}
	.tabs button[aria-current='page'] {
		border-bottom-color: var(--color-accent);
		color: var(--color-text-accent);
	}
	.tabs button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.body {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		flex: 1 1 auto;
		min-height: 0;
	}
	.body.has-aside {
		grid-template-columns: minmax(0, 1fr) 20rem;
	}
	.content {
		position: relative;
		min-height: 0;
		overflow: auto;
	}
	.aside {
		border-left: var(--size-border) solid var(--color-border-subtle);
		background: var(--color-surface-raised);
		overflow-y: auto;
	}
	@media (max-width: 899px) {
		.body.has-aside {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: minmax(0, 1fr) auto;
		}
		.aside {
			border-left: 0;
			border-top: var(--size-border) solid var(--color-border-subtle);
			max-height: 40dvh;
		}
	}
</style>
