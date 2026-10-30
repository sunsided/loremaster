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
	}

	let { room, rules = [], ongo, onwrite, onexplore }: Props = $props();
</script>

<!-- One room in the Atlas: where it is, what is in it now, and what you can do there. -->
<section class="room-card" aria-labelledby="room-card-title">
	<h2 id="room-card-title">{room.name}</h2>
	<p class="where">level {room.level}{room.current ? ' · you are here' : ''}</p>
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
		<Button size="compact" variant="primary" onclick={ongo} disabled={room.current}>Go there</Button>
		<Button size="compact" onclick={onwrite}>Write here</Button>
		{#if onexplore}<Button size="compact" variant="quiet" onclick={onexplore}>Explore</Button>{/if}
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
