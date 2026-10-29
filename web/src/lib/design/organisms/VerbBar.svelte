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
		things: ThingInfo[];
		inventory: ThingInfo[];
		exits: ExitInfo[];
		onverb: (verb: VerbInfo) => void;
		onthing: (id: string, name: string) => void;
		onexit: (direction: string) => void;
		onrun?: () => void;
		oncancel?: () => void;
	}

	let {
		verbs,
		selected = null,
		sentence = '',
		canRunPartial = false,
		marked = [],
		markedExit = null,
		things,
		inventory,
		exits,
		onverb,
		onthing,
		onexit,
		onrun,
		oncancel
	}: Props = $props();

	const selectedVerb = $derived(verbs.find((v) => v.id === selected));
	const toItem = (t: ThingInfo) => ({ id: t.id, label: t.name });
</script>

<!--
	The SCUMM-style controls. The player selects a verb, then clicks a thing.
	A click on a thing with no verb looks at it.
-->
<div class="verb-bar">
	<div class="verbs" role="group" aria-label="Verbs">
		{#each verbs as verb (verb.id)}
			<Button block pressed={verb.id === selected} onclick={() => onverb(verb)}>{verb.label}</Button>
		{/each}
	</div>

	<div class="reach">
		{#if selected === 'go'}
			<ActionList
				label="to"
				empty="nowhere"
				items={exits.map((e) => ({ id: e.direction, label: e.to_name, detail: e.direction }))}
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
		{:else}
			<span class="hint">Select a verb, or type one. Then click or type a thing. A click on a thing with no verb looks at it.</span>
		{/if}
	</div>
</div>

<style>
	.verb-bar {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		padding-top: var(--space-6);
		border-top: var(--size-border) solid var(--color-border-subtle);
	}
	.verbs {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
		gap: var(--space-3);
		max-width: 820px;
	}
	.reach {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4) var(--space-7);
	}
	.sentence {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
		min-height: var(--size-target-compact);
	}
	.text {
		color: var(--color-text-warning);
	}
	.hint {
		color: var(--color-text-muted);
	}
</style>
