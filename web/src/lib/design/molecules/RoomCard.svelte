<script lang="ts">
	import type { AtlasRoom, RuleView } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';

	interface Props {
		room: AtlasRoom;
		/** The rules that name this room. */
		rules?: RuleView[];
		/** Move the player to the room (an author move). */
		ongo: () => void;
		/** Move the player to the room and start write mode. */
		onwrite: () => void;
		/** Show the texts of the room in all states. */
		onexplore?: () => void;
		/** For a room that an AI proposal adds: the proposal. The card then offers Walk in, Accept and Reject. */
		proposal?: { id: string; summary: string } | null;
		/** For a room that an AI proposal removes: the summary of the proposal. */
		removedBy?: string | null;
		/** Play the proposal in a sandbox, in this room. */
		onwalk?: () => void;
		onaccept?: () => void;
		onreject?: () => void;
	}

	let { room, rules = [], ongo, onwrite, onexplore, proposal = null, removedBy = null, onwalk, onaccept, onreject }: Props = $props();
	const id = $props.id();
</script>

<!-- One room in the Atlas: where it is, what is in it now, and what you can do there. -->
<section class="room-card" class:proposed={!!proposal} aria-labelledby="{id}-title">
	<h2 id="{id}-title">{proposal ? '✦ ' : ''}{room.name}</h2>
	<p class="where">level {room.level}{room.current ? ' · you are here' : ''}</p>
	{#if proposal}<p class="ai">Proposed by AI{proposal.summary ? `: ${proposal.summary}` : ''}</p>{/if}
	{#if removedBy}<p class="ai removed">An AI proposal removes this room: {removedBy}</p>{/if}
	<dl>
		<dt>visited</dt>
		<dd>{room.visited ? 'yes' : 'no'}</dd>
		<dt>exits</dt>
		<dd>{room.exits.length ? room.exits.map(([d, to]) => (to ? `${d} (${to})` : `${d} (message)`)).join(', ') : 'none'}</dd>
		<dt>things</dt>
		<dd>{room.things.length ? room.things.map((t) => t.name).join(', ') : 'none'}</dd>
		<dt>fragments</dt>
		<dd>{room.fragments}</dd>
		<dt>rules</dt>
		<dd>{rules.length ? rules.map((r) => r.id).join(', ') : 'none'}</dd>
	</dl>
	<div class="actions">
		{#if proposal}
			{#if onwalk}<Button size="compact" variant="primary" onclick={onwalk}>Walk in</Button>{/if}
			{#if onaccept}<Button size="compact" onclick={onaccept}>Accept</Button>{/if}
			{#if onreject}<Button size="compact" variant="quiet" onclick={onreject}>Reject</Button>{/if}
		{:else}
			<Button size="compact" variant="primary" onclick={ongo} disabled={room.current}>Go there</Button>
			<Button size="compact" onclick={onwrite}>Write here</Button>
			{#if onexplore}<Button size="compact" variant="quiet" onclick={onexplore}>Explore</Button>{/if}
		{/if}
	</div>
</section>

<style>
	.room-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-5);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	h2 {
		margin: 0;
		font-size: var(--font-size-md);
		color: var(--color-text-accent);
	}
	.where {
		margin: 0;
		color: var(--color-text-muted);
	}
	.proposed h2,
	.ai {
		color: var(--color-ghost);
		font-style: italic;
	}
	.ai {
		margin: 0;
		padding-left: var(--space-3);
		border-left: var(--size-border) dashed var(--color-ghost);
	}
	.ai.removed {
		color: var(--color-text-warning);
		border-left-color: var(--color-warning);
	}
	dl {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: var(--space-2) var(--space-4);
		margin: 0;
	}
	dt {
		color: var(--color-text-muted);
	}
	dd {
		margin: 0;
		color: var(--color-text-strong);
		overflow-wrap: anywhere;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin-top: var(--space-2);
	}
</style>
