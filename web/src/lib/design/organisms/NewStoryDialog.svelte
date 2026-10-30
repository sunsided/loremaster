<script lang="ts">
	import Button from '../atoms/Button.svelte';
	import Heading from '../atoms/Heading.svelte';
	import TextInput from '../atoms/TextInput.svelte';

	interface Props {
		open?: boolean;
		/**
		 * Why the AI cannot seed the story now, for example "Start loremaster serve to use AI.".
		 * Null when it can. The idea field shows always, so that the author sees the option.
		 */
		aiUnavailable?: string | null;
		/** Make the story. `idea` is empty for an empty story. */
		oncreate: (story: { title: string; idea: string }) => void;
	}

	let { open = $bindable(false), aiUnavailable = null, oncreate }: Props = $props();
	const id = $props.id();

	let dialog: HTMLDialogElement | undefined = $state();
	let title = $state('');
	let idea = $state('');
	let titleInput: HTMLInputElement | undefined = $state();

	// A modal dialog keeps focus inside, closes on Escape and gives focus back to the button that opened it.
	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) {
			title = '';
			idea = '';
			dialog.showModal();
			queueMicrotask(() => titleInput?.focus());
		}
		if (!open && dialog.open) dialog.close();
	});

	const useIdea = $derived(idea.trim() !== '' && aiUnavailable === null);

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!title.trim()) return;
		oncreate({ title: title.trim(), idea: useIdea ? idea.trim() : '' });
		open = false;
	}
</script>

<!--
	Start a new story: a title, and an optional idea. Without an idea, the story has one empty room.
	With an idea, the AI drafts two or three connected rooms as a proposal.
-->
<dialog bind:this={dialog} class="new-story" aria-labelledby="{id}-title" onclose={() => (open = false)}>
	<form onsubmit={submit}>
		<div class="head">
			<Heading level={2} look="title" id="{id}-title">New story</Heading>
			<Button size="compact" variant="quiet" type="button" onclick={() => (open = false)}>Cancel</Button>
		</div>

		<div class="field">
			<TextInput id="{id}-name" label="Title" required autocomplete="off" bind:value={title} bind:element={titleInput} />
		</div>

		<div class="field">
			<label for="{id}-idea">Idea <span class="optional">(optional)</span></label>
			<textarea
				id="{id}-idea"
				rows="4"
				bind:value={idea}
				aria-describedby="{id}-idea-help"
				placeholder="A lighthouse keeper vanished three nights ago, and the ferry is due after dark."
			></textarea>
			<p class="help" id="{id}-idea-help">
				{#if aiUnavailable}
					{aiUnavailable} Without the AI, the story starts with one empty room.
				{:else}
					The AI drafts two or three connected rooms from your idea, as a proposal that you can accept or reject.
					Leave it empty to start with one empty room.
				{/if}
			</p>
		</div>

		<div class="actions">
			<Button variant="primary" type="submit" disabled={!title.trim()}>
				{useIdea ? '✦ Create and draft with AI' : 'Create the story'}
			</Button>
		</div>
	</form>
</dialog>

<style>
	.new-story {
		box-sizing: border-box;
		width: min(var(--width-dialog), calc(100vw - var(--space-9)));
		max-height: calc(100vh - var(--space-9));
		padding: var(--space-7) var(--space-8);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		color: var(--color-text);
		box-shadow: var(--shadow-overlay);
	}
	.new-story::backdrop {
		background: rgb(0 0 0 / 40%);
	}
	form {
		display: flex;
		flex-direction: column;
		gap: var(--space-7);
	}
	.head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-5);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	label {
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.optional {
		color: var(--color-text-muted);
	}
	textarea {
		box-sizing: border-box;
		width: 100%;
		min-height: calc(var(--size-target) * 2);
		padding: var(--space-3);
		border: var(--size-border) solid var(--color-border-control);
		background: transparent;
		color: var(--color-text-strong);
		caret-color: var(--color-accent);
		font-family: var(--font-ui);
		font-size: var(--font-size-md);
		resize: vertical;
	}
	.help {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.actions {
		display: flex;
		justify-content: flex-end;
	}
</style>
