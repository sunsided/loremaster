<script lang="ts">
	import type { ExitInfo, ThingInfo, VerbInfo } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';
	import ActionList from '../molecules/ActionList.svelte';

	interface Props {
		verbs: VerbInfo[];
		/** The id of the selected verb, or null. */
		selected?: string | null;
		/** The command so far, for example "Use oil lamp on _". Empty when no verb is selected. */
		sentence?: string;
		/** True when a two-object verb has its first object and can also run without a second. */
		canRunPartial?: boolean;
		/** Ids of things that the typed command names or could still name. They are marked. */
		marked?: string[];
		/** The exit direction that the typed command names. It is marked. */
		markedExit?: string | null;
		/** The verb that a click on the thing or exit under the pointer would use. It is marked. */
		hintedVerb?: string | null;
		things: ThingInfo[];
		inventory: ThingInfo[];
		exits: ExitInfo[];
		onverb: (verb: VerbInfo) => void;
		onthing: (id: string, name: string) => void;
		onexit: (direction: string) => void;
		onrun?: () => void;
		oncancel?: () => void;
		/** Show the help line when no verb is selected. The app shows it until the first command. */
		hint?: boolean;
	}

	let {
		verbs,
		selected = null,
		sentence = '',
		canRunPartial = false,
		marked = [],
		markedExit = null,
		hintedVerb = null,
		things,
		inventory,
		exits,
		onverb,
		onthing,
		onexit,
		onrun,
		oncancel,
		hint = true
	}: Props = $props();

	const selectedVerb = $derived(verbs.find((v) => v.id === selected));
	const toItem = (t: ThingInfo) => ({ id: t.id, label: t.name, inspect: `thing:${t.id}` });
</script>

<!--
	The SCUMM-style controls, quiet: the verbs are text in a grid of four columns, and the things in reach are
	on the right. The selected verb is in inverse video. The player selects a verb, then clicks a thing.
	A click on a thing with no verb looks at it.
-->
<div class="verb-bar">
	<div class="verbs" role="group" aria-label="Verbs">
		{#each verbs as verb (verb.id)}
			<button
				type="button"
				class="verb"
				aria-pressed={verb.id === selected}
				class:hinted={verb.id === hintedVerb && verb.id !== selected}
				onclick={() => onverb(verb)}>{verb.label}</button
			>
		{/each}
	</div>

	<div class="reach">
		{#if selected === 'go'}
			<ActionList
				label="to"
				empty="nowhere"
				items={exits.map((e) => ({ id: e.direction, label: e.to_name, detail: e.direction, inspect: `exit:${e.direction}` }))}
				marked={markedExit ? [markedExit] : []}
				onpick={(item) => onexit(item.id)}
			/>
		{:else}
			<ActionList label="here" empty="nothing in reach" items={things.map(toItem)} {marked} onpick={(i) => onthing(i.id, i.label)} />
			<ActionList label="carrying" empty="nothing" items={inventory.map(toItem)} {marked} onpick={(i) => onthing(i.id, i.label)} />
		{/if}
	</div>

	<div class="sentence" aria-live="polite">
		{#if selectedVerb}
			<span class="text">{sentence}</span>
			{#if canRunPartial}
				<Button size="compact" variant="quiet" onclick={() => onrun?.()}>just {selectedVerb.label.toLowerCase()} it</Button>
			{/if}
			<Button size="compact" variant="quiet" onclick={() => oncancel?.()}>cancel</Button>
		{:else if hint}
			<span class="hint">Select a verb, or type one. Then click or type a thing. A click on a thing with no verb looks at it.</span>
		{/if}
	</div>
</div>

<style>
	.verb-bar {
		display: grid;
		grid-template-columns: minmax(0, 30rem) minmax(0, 1fr);
		grid-template-areas:
			'verbs reach'
			'sentence sentence';
		gap: var(--space-4) var(--space-8);
		padding-top: var(--space-5);
		border-top: var(--size-border) solid var(--color-border-subtle);
		font-family: var(--font-ui);
	}
	.verbs {
		grid-area: verbs;
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 0 var(--space-4);
		align-content: start;
	}
	.verb {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-2);
		margin-left: calc(-1 * var(--space-2));
		border: 0;
		background: none;
		color: var(--color-text-muted);
		font: inherit;
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.verb:hover {
		color: var(--color-text-strong);
	}
	.verb.hinted {
		color: var(--color-text-accent);
		text-decoration: underline;
		text-underline-offset: 3px;
	}
	.verb[aria-pressed='true'] {
		background: var(--color-accent);
		color: var(--color-text-on-accent);
		font-weight: var(--font-weight-strong);
	}
	.verb:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 1px;
	}
	.reach {
		grid-area: reach;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		min-width: 0;
	}
	.sentence {
		grid-area: sentence;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
		min-height: var(--size-target-compact);
		font-size: var(--font-size-sm);
	}
	.sentence:empty {
		display: none;
	}
	.text {
		color: var(--color-text-warning);
	}
	.hint {
		color: var(--color-text-muted);
	}
	/* Narrow: the verbs over the things, three columns. */
	@media (max-width: 719px) {
		.verb-bar {
			grid-template-columns: minmax(0, 1fr);
			grid-template-areas:
				'verbs'
				'reach'
				'sentence';
		}
		.verbs {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}
	}
</style>
