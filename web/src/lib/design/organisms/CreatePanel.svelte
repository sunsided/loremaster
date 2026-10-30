<script lang="ts">
	import { untrack } from 'svelte';
	import Button from '../atoms/Button.svelte';
	import Checkbox from '../atoms/Checkbox.svelte';
	import Select from '../atoms/Select.svelte';
	import TextInput from '../atoms/TextInput.svelte';
	import type { CreateRequest } from '../types.ts';

	interface Props {
		/** `exit`: a new exit, to a new or an existing room. `thing` or `character`: a new thing in the current room. */
		kind: 'exit' | 'thing' | 'character';
		/** The current room, for the title. */
		here: string;
		/** For an exit: the first direction. */
		direction?: string;
		/** For an exit: the directions that have no exit yet. */
		directions?: string[];
		/** For an exit: the rooms that the exit can lead to. */
		rooms?: { id: string; name: string }[];
		/** For a thing: the first name, for example from an unknown name in the text. */
		name?: string;
		/** Create it. Returns a problem text when the story does not build with it. */
		onsubmit: (request: CreateRequest) => Promise<string | null>;
		oncancel: () => void;
		/** For an exit: let the AI draft options for this direction instead. When set, the panel offers it. */
		ondraft?: (direction: string) => void;
	}

	let { kind, here, direction = '', directions = [], rooms = [], name = '', onsubmit, oncancel, ondraft }: Props = $props();

	const id = $props.id();
	let dir = $state(untrack(() => direction || directions[0] || 'north'));
	let target = $state<string>('new');
	let title = $state(untrack(() => name));
	let back = $state(true);
	/** For a message exit: what a move in that direction prints. */
	let message = $state('');
	let portable = $state(untrack(() => kind === 'thing'));
	let proper = $state(false);
	let busy = $state(false);
	let problem = $state<string | null>(null);
	let nameInput: HTMLInputElement | undefined = $state();
	let section: HTMLElement | undefined = $state();

	$effect(() => {
		section?.scrollIntoView({ block: 'nearest' });
		(nameInput ?? section?.querySelector('select'))?.focus();
	});

	const heading = $derived(
		kind === 'exit' ? `New exit from ${here}` : kind === 'character' ? `New character in ${here}` : `New thing in ${here}`
	);
	const needsName = $derived(kind !== 'exit' || target === 'new');
	const isMessage = $derived(kind === 'exit' && target === 'message');

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (needsName && !title.trim()) {
			problem = 'Write a name.';
			nameInput?.focus();
			return;
		}
		if (isMessage && !message.trim()) {
			problem = 'Write the message.';
			return;
		}
		busy = true;
		problem = await onsubmit(
			kind === 'exit'
				? {
						kind,
						direction: dir,
						to: target === 'new' ? { name: title.trim() } : target === 'message' ? { message: message.trim() } : { id: target },
						back: back && !isMessage
					}
				: { kind, name: title.trim(), portable: kind === 'thing' && portable, proper }
		);
		busy = false;
	}
</script>

<!--
	A small form in the story text, for something new: an exit (to a new or an existing room), a thing or a character.
	It opens in write mode, for example when the player walks where there is no exit.
	Escape cancels.
-->
<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape' && section?.contains(document.activeElement)) oncancel();
	}}
/>

<section class="create" aria-labelledby="{id}-title" bind:this={section}>
	<form onsubmit={submit}>
		<h2 id="{id}-title">{heading}</h2>
		{#if kind === 'exit'}
			<div class="row">
				<Select label="Direction" bind:value={dir} options={(directions.length ? directions : [dir]).map((d) => ({ value: d, label: d }))} />
				<Select
					label="Leads to"
					bind:value={target}
					options={[
						{ value: 'new', label: 'a new room' },
						{ value: 'message', label: 'nowhere, only a message' },
						...rooms.map((r) => ({ value: r.id, label: r.name }))
					]}
				/>
			</div>
		{/if}
		{#if isMessage}
			<div class="name">
				<TextInput
					id="{id}-message"
					label="What a move there prints"
					bind:value={message}
					autocomplete="off"
					placeholder="Cold surf breaks over the rocks."
				/>
			</div>
		{/if}
		{#if needsName}
			<div class="name">
				<TextInput
					id="{id}-name"
					label={kind === 'exit' ? 'Name of the new room' : 'Name'}
					bind:value={title}
					bind:element={nameInput}
					autocomplete="off"
					placeholder={kind === 'exit' ? 'The Beach' : kind === 'character' ? 'shadow' : 'brass key'}
				/>
			</div>
		{/if}
		<div class="row">
			{#if kind === 'exit' && !isMessage}<Checkbox bind:checked={back}>with the way back</Checkbox>{/if}
			{#if kind === 'thing'}<Checkbox bind:checked={portable}>the player can take it</Checkbox>{/if}
			{#if kind === 'character'}<Checkbox bind:checked={proper}>proper name, without "the"</Checkbox>{/if}
		</div>
		{#if problem}<p class="problem" role="alert">{problem}</p>{/if}
		<div class="row">
			<Button variant="primary" size="compact" type="submit" disabled={busy}>Create</Button>
			<Button size="compact" type="button" onclick={oncancel}>Cancel</Button>
			{#if kind === 'exit' && ondraft}
				<Button size="compact" type="button" variant="quiet" onclick={() => ondraft(dir)}>✦ Draft "{dir}" with AI</Button>
			{/if}
		</div>
	</form>
</section>

<style>
	.create {
		margin: var(--space-4) 0;
		padding: var(--space-4) var(--space-5);
		border: var(--size-border) solid var(--color-accent);
		background: var(--color-surface-raised);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		text-shadow: none;
	}
	form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	h2 {
		margin: 0;
		font-size: var(--font-size-sm);
		color: var(--color-text-accent);
	}
	.row,
	.name {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3) var(--space-5);
	}
	.name :global(label) {
		color: var(--color-text-muted);
	}
	.problem {
		margin: 0;
		color: var(--color-text-warning);
	}
</style>
