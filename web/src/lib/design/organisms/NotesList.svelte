<script lang="ts">
	import Button from '../atoms/Button.svelte';
	import Checkbox from '../atoms/Checkbox.svelte';
	import VisuallyHidden from '../atoms/VisuallyHidden.svelte';
	import type { NoteItem } from '../types.ts';

	interface Props {
		notes: NoteItem[];
		/** The text when there are no notes. */
		empty?: string;
		/** Add a note or a TODO. Leave it out for a list without the add form. */
		onadd?: (note: { text: string; todo: boolean }) => void;
		/** The add form makes TODOs only, for example in a list of TODOs. */
		todoOnly?: boolean;
		/** The placeholder of the add form. */
		placeholder?: string;
		/** Mark a TODO as done, or as open again. */
		ontoggle: (key: string, done: boolean) => void;
		onremove?: (key: string) => void;
		/** Go to the place of the note, for example its room. */
		onselect?: (key: string) => void;
	}

	let {
		notes,
		empty = 'No notes here.',
		onadd,
		todoOnly = false,
		placeholder = 'A note for the authors',
		ontoggle,
		onremove,
		onselect
	}: Props = $props();
	const uid = $props.id();
	let draft = $state('');

	function add(todo: boolean) {
		const text = draft.trim();
		if (!text || !onadd) return;
		onadd({ text, todo });
		draft = '';
	}

	const roles = { setup: 'setup', payoff: 'payoff', mention: 'mention' } as const;
</script>

<!--
	Author notes and TODOs: text for the authors that the player never sees.
	A TODO has a checkbox. A note on a thread shows the thread and the role of the beat (setup, payoff, mention).
-->
<div class="notes-list">
	{#if notes.length}
		<ul>
			{#each notes as n (n.key)}
				<li class:done={n.todo && n.done}>
					{#if n.todo}
						<Checkbox checked={n.done} onchange={(e) => ontoggle(n.key, (e.currentTarget as HTMLInputElement).checked)}
							>{n.text}</Checkbox
						>
					{:else}
						<p class="text"><span class="pin" aria-hidden="true">✎</span>{n.text}</p>
					{/if}
					{#if n.where || n.thread}
						<p class="meta">
							{#if n.thread}<span class="thread">{n.role ? `${roles[n.role]} · ` : ''}{n.thread}</span>{/if}
							{#if n.where}{#if onselect}<button type="button" class="link" onclick={() => onselect(n.key)}>{n.where}</button
									>{:else}<span>{n.where}</span>{/if}{/if}
						</p>
					{/if}
					{#if onremove}
						<Button size="compact" variant="quiet" aria-label="Remove the note: {n.text}" onclick={() => onremove(n.key)}>×</Button>
					{/if}
				</li>
			{/each}
		</ul>
	{:else}
		<p class="empty">{empty}</p>
	{/if}
	{#if onadd}
		<form
			class="add"
			onsubmit={(e) => {
				e.preventDefault();
				add(todoOnly);
			}}
		>
			<VisuallyHidden><label for="{uid}-draft">{todoOnly ? 'A new TODO' : 'A new note or TODO'}</label></VisuallyHidden>
			<input id="{uid}-draft" bind:value={draft} {placeholder} autocomplete="off" />
			<div class="buttons">
				{#if todoOnly}
					<Button size="compact" type="submit" disabled={!draft.trim()}>+ TODO</Button>
				{:else}
					<Button size="compact" type="submit" disabled={!draft.trim()}>+ Note</Button>
					<Button size="compact" type="button" disabled={!draft.trim()} onclick={() => add(true)}>+ TODO</Button>
				{/if}
			</div>
		</form>
	{/if}
</div>

<style>
	.notes-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	ul {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li {
		display: grid;
		grid-template-columns: 1fr auto;
		align-items: start;
		gap: 0 var(--space-3);
	}
	li > :global(*:not(button)) {
		grid-column: 1;
	}
	li > :global(button) {
		grid-column: 2;
		grid-row: 1;
	}
	li.done {
		color: var(--color-text-muted);
	}
	li.done :global(label) {
		text-decoration: line-through;
	}
	.text {
		margin: 0;
		color: var(--color-text);
		line-height: var(--line-height-body);
	}
	.pin {
		margin-right: var(--space-2);
		color: var(--color-accent);
	}
	.meta {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin: var(--space-1) 0 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.thread {
		color: var(--color-accent);
	}
	.link {
		min-height: var(--size-target-compact);
		padding: 0;
		border: 0;
		background: none;
		color: var(--color-text-accent);
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	.empty {
		margin: 0;
		color: var(--color-text-muted);
	}
	.add {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}
	input {
		box-sizing: border-box;
		flex: 1 1 12rem;
		min-width: 0;
		min-height: var(--size-target-compact);
		padding: 0 var(--space-2);
		border: 0;
		border-bottom: var(--size-border) solid var(--color-border-control);
		background: transparent;
		color: var(--color-text-strong);
		caret-color: var(--color-accent);
		font: inherit;
	}
	.buttons {
		display: flex;
		gap: var(--space-2);
	}
</style>
