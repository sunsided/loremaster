<script lang="ts">
	import type { Snippet } from 'svelte';
	import { sections } from '../sections.svelte.ts';

	interface Props {
		/** A stable id. The open or closed state of each id stays across visits. */
		id: string;
		title: string;
		/** A short fact after the title, for example "1 of 6". */
		meta?: string;
		/** Links or buttons at the right of the title row, for example "Atlas ↗". */
		actions?: Snippet;
		children: Snippet;
	}

	let { id, title, meta, actions, children }: Props = $props();
	const open = $derived(!sections.isClosed(id));
	const uid = $props.id();
</script>

<!--
	One section of a side panel, with a heading and a button that folds it.
	The folded state of each section stays across visits (browser storage).
	The heading is a real heading, so screen readers can move from section to section.
-->
<section class="panel-section" aria-labelledby="{uid}-title">
	<div class="head">
		<h2 id="{uid}-title">
			<button type="button" aria-expanded={open} aria-controls="{uid}-body" onclick={() => sections.set(id, open)}>
				<span class="caret" aria-hidden="true">{open ? '▾' : '▸'}</span>
				<span class="title">{title}</span>
				{#if meta}<span class="meta">{meta}</span>{/if}
			</button>
		</h2>
		{#if actions}<span class="actions">{@render actions()}</span>{/if}
	</div>
	<div class="body" id="{uid}-body" hidden={!open}>{@render children()}</div>
</section>

<style>
	.panel-section {
		border-bottom: var(--size-border) solid var(--color-border-subtle);
		padding: var(--space-3) var(--space-5) var(--space-4);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	h2 {
		flex: 1 1 auto;
		margin: 0;
		min-width: 0;
		font-size: inherit;
	}
	button {
		display: flex;
		align-items: baseline;
		gap: var(--space-3);
		width: 100%;
		min-height: var(--size-target-compact);
		padding: 0;
		border: 0;
		background: none;
		color: var(--color-text-strong);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		text-align: left;
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
		font-weight: var(--font-weight-strong);
	}
	button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.caret {
		width: 1ch;
		color: var(--color-text-muted);
	}
	.meta {
		margin-left: auto;
		color: var(--color-text-muted);
		font-weight: var(--font-weight-regular);
		text-transform: none;
		letter-spacing: normal;
	}
	.actions {
		display: inline-flex;
		gap: var(--space-2);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
	}
	.body {
		padding-top: var(--space-2);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.body[hidden] {
		display: none;
	}
</style>
