<script lang="ts">
	import type { TalkView } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';

	/** The new text of the line and of each answer, in the order of the node's answers. */
	export interface TalkEdit {
		say: string;
		options: string[];
	}

	interface Props {
		talk: TalkView;
		/** Write the change. Resolves to a problem text, or null when the story builds with it. */
		onsave: (edit: TalkEdit) => Promise<string | null>;
		oncancel: () => void;
	}

	let { talk, onsave, oncancel }: Props = $props();

	const uid = $props.id();
	// The editor starts from the node as written; a new node is a new editor (keyed by the caller).
	// svelte-ignore state_referenced_locally
	let say = $state(talk.say);
	// svelte-ignore state_referenced_locally
	let options = $state(talk.options.map((o) => o.say));
	let problem = $state<string | null>(null);
	let saving = $state(false);
	const changed = $derived(say !== talk.say || options.some((o, i) => o !== talk.options[i]?.say));

	async function save() {
		if (!changed || saving) return;
		saving = true;
		problem = await onsave({ say, options });
		saving = false;
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			void save();
		} else if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			oncancel();
		}
	}
</script>

<!--
	Write mode during a talk: the line of the other character and the answers of the player are editable in place,
	in the game text. Hidden answers are editable too; they are marked. Ctrl+Enter saves, Escape cancels.
	The change goes to the dialogue file. The game rebuilds and the talk stays at the same node.
-->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<form
	class="talk-editor"
	aria-label="Edit the talk at node {talk.node}"
	onsubmit={(e) => {
		e.preventDefault();
		void save();
	}}
	{onkeydown}
>
	<p class="head"><span class="node">{talk.node}</span> in dialogues/{talk.dialogue}.yaml</p>
	<label class="label" for="{uid}-say">Line of {talk.name}</label>
	<textarea id="{uid}-say" class="say" rows="3" bind:value={say}></textarea>
	<fieldset>
		<legend class="label">Answers</legend>
		{#each options as _, i (i)}
			{@const o = talk.options[i]}
			<div class="option">
				<label for="{uid}-o{i}" class="n"
					>{o.choice === null ? '–' : o.choice + 1}<span class="visually-hidden"
						>Answer {i + 1}{o.choice === null ? ', hidden now' : ''}</span
					></label
				>
				<input id="{uid}-o{i}" class="answer" class:hidden={o.choice === null} bind:value={options[i]} />
				{#if o.choice === null}<span class="tag" title={o.when ? `false now: ${o.when}` : 'hidden now'}>hidden</span>{/if}
			</div>
		{/each}
	</fieldset>
	{#if problem}<p class="problem" role="alert">{problem}</p>{/if}
	<div class="actions">
		<Button type="submit" variant="primary" size="compact" disabled={!changed || saving}>Save</Button>
		<Button size="compact" variant="quiet" onclick={oncancel}>Cancel</Button>
		<span class="keys">Ctrl+Enter saves · Esc cancels</span>
	</div>
</form>

<style>
	.talk-editor {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin-top: var(--space-4);
		padding: var(--space-4);
		border: var(--size-border) dashed var(--color-border-control);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		text-shadow: none;
	}
	.head {
		margin: 0;
		color: var(--color-text-muted);
	}
	.node {
		color: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
	}
	.label {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
	}
	.say,
	.answer {
		box-sizing: border-box;
		width: 100%;
		min-width: 0;
		min-height: var(--size-target-compact);
		padding: var(--space-2) var(--space-3);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface);
		color: var(--color-text-strong);
		caret-color: var(--color-accent);
		font-family: var(--font-prose);
		font-size: var(--font-size-prose);
		line-height: var(--line-height-body);
	}
	.say {
		resize: vertical;
	}
	.say:focus-visible,
	.answer:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 1px;
	}
	.answer.hidden {
		color: var(--color-text-muted);
	}
	fieldset {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		border: 0;
		min-width: 0;
	}
	.option {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.n {
		flex: none;
		min-width: 1.5em;
		color: var(--color-text-accent);
	}
	.tag {
		flex: none;
		padding: 0 var(--space-2);
		border: var(--size-border) solid var(--color-border-control);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.problem {
		margin: 0;
		color: var(--color-text-warning);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
	}
	.keys {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
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
