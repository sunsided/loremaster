<script lang="ts">
	import type { Snippet } from 'svelte';
	import { help } from '../help.svelte.ts';

	interface Props {
		/** A stable id. The collapsed state of each id stays across visits. */
		id: string;
		/** The name of the region, for the button label, for example "Assembly". */
		topic: string;
		children: Snippet;
	}

	let { id, topic, children }: Props = $props();

	/** Open for this visit only, after the user collapsed the note. */
	let peek = $state(false);
	const collapsed = $derived(help.isCollapsed(id));
	const textId = $derived(`help-${id}`);
</script>

<!--
	A help text that experienced users can collapse into a (?) button. The choice stays across visits.
	Put it in a flex row with the heading: the button sits next to the heading, and the text takes a full line under it.
-->
{#if collapsed}
	<button
		class="toggle"
		aria-expanded={peek}
		aria-controls={textId}
		aria-label="{peek ? 'Hide' : 'Show'} help: {topic}"
		onclick={() => (peek = !peek)}>?</button
	>
{/if}
{#if !collapsed || peek}
	<p class="note" id={textId}>
		{@render children()}
		{#if collapsed}
			<button class="link" onclick={() => { help.set(id, false); peek = false; }}>Always show</button>
		{:else}
			<button class="link" onclick={() => help.set(id, true)}>Hide</button>
		{/if}
	</p>
{/if}

<style>
	.toggle {
		width: 24px;
		height: 24px;
		padding: 0;
		border: var(--size-border) solid var(--color-border-control);
		border-radius: 50%;
		background: transparent;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		line-height: 1;
	}
	.toggle:hover,
	.toggle[aria-expanded='true'] {
		border-color: var(--color-accent);
		color: var(--color-text-accent);
	}
	.note {
		flex-basis: 100%;
		margin: 0;
		color: var(--color-text-muted);
		line-height: var(--line-height-body);
	}
	.link {
		margin-left: var(--space-2);
		padding: 0;
		border: 0;
		background: none;
		color: var(--color-text-accent);
		text-decoration: underline;
		text-underline-offset: 2px;
	}
</style>
