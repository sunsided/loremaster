<script lang="ts">
	import type { Exploration, RoomMatrix } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';
	import Heading from '../atoms/Heading.svelte';
	import HelpNote from '../molecules/HelpNote.svelte';
	import RadioGroup from '../molecules/RadioGroup.svelte';
	import SectionRule from '../molecules/SectionRule.svelte';
	import RoomMatrixView from './RoomMatrixView.svelte';

	interface Props {
		/** `room`: all combinations of the current room. `game`: all states that the player can reach from the start. */
		mode?: 'room' | 'game';
		/** The room matrix of the current room. */
		matrix?: RoomMatrix | null;
		/** The name of the current room. */
		roomName?: string;
		/** Put the game into one combination of the room matrix. */
		ontry?: (values: [string, string][]) => void;
		/** The last result, or null before the first run. */
		result?: Exploration | null;
		/** True while the explorer runs. */
		running?: boolean;
		onrun: () => void;
		/** Start again and play a path. */
		onreplay: (path: string[]) => void;
	}

	let {
		mode = $bindable('room'),
		matrix = null,
		roomName = '',
		ontry = () => {},
		result = null,
		running = false,
		onrun,
		onreplay
	}: Props = $props();

	const modes = [
		{ value: 'room' as const, label: 'This room' },
		{ value: 'game' as const, label: 'Whole game' }
	];
	const seen = $derived(result?.rooms.find((r) => r.id === matrix?.room)?.seen ?? null);

	const unreached = $derived(result?.rooms.filter((r) => !r.reached) ?? []);

	/** `*word*` in story text is emphasis. */
	function pieces(text: string): { text: string; em: boolean }[] {
		return text.split('*').map((t, i) => ({ text: t, em: i % 2 === 1 }));
	}
</script>

{#snippet prose(text: string)}{#each pieces(text) as p, i (i)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}{/snippet}

{#snippet path(steps: string[])}
	<ol class="path" aria-label="Commands">
		{#each steps as step, i (i)}<li><code>{step}</code></li>{:else}<li class="muted">the start</li>{/each}
	</ol>
{/snippet}

<!--
	The state explorer: it plays every meaningful way through the story, from the start, and shows
	how the text reads in each state it found, which text never shows, and whether the player can get stuck.
-->
<section class="explorer" aria-labelledby="explore-title">
	<div class="head">
		<Heading level={2} id="explore-title">Explore</Heading>
		<HelpNote id="explore" topic="Explore">
			"This room" shows the text of the current room in each combination of the values that the text depends on, also
			combinations that the player can not reach. Use it to find a text that reads badly in one condition. "Whole game"
			plays all ways through the story from the start, within a budget, and finds text that never shows.
		</HelpNote>
	</div>
	<RadioGroup legend="What to explore" name="explore-mode" look="pills" options={modes} bind:value={mode} />

	{#if mode === 'room'}
		{#if matrix}
			<RoomMatrixView {matrix} {roomName} {seen} {ontry} />
		{/if}
	{:else}
		<div>
			<Button variant="primary" onclick={onrun} disabled={running}
				>{running ? 'Exploring…' : result ? 'Explore again' : 'Explore all states'}</Button
			>
		</div>
	{/if}

	{#if mode === 'game' && result}
		<p class="summary" role="status">
			{result.states} states.
			{unreached.length ? `${unreached.length} rooms not reached.` : 'Every room reached.'}
			{result.endings.length} {result.endings.length === 1 ? 'ending' : 'endings'}.
			{result.stuck ? `${result.stuck} stuck states.` : 'No stuck states.'}
		</p>
		{#if result.truncated}
			<p class="warning">The budget stopped the walk. The results are incomplete: more states can exist.</p>
		{/if}

		{#if result.endings.length || result.stuck}
			<section class="group">
				<SectionRule title="endings" />
				{#each result.endings as ending, i (i)}
					<div class="item">
						<p class="text">{@render prose(ending.text)}</p>
						{@render path(ending.path)}
						<Button size="compact" variant="quiet" onclick={() => onreplay(ending.path)}>Replay this path</Button>
					</div>
				{/each}
				{#if result.stuck}
					<div class="item stuck">
						<p>{result.stuck} states cannot reach an ending. The shortest way into one:</p>
						{@render path(result.stuck_path)}
						<Button size="compact" variant="quiet" onclick={() => onreplay(result.stuck_path)}>Replay this path</Button>
					</div>
				{/if}
			</section>
		{/if}

		<section class="group">
			<SectionRule title="rooms" />
			{#each result.rooms as room (room.id)}
				<details class="room" open={!room.reached || room.variants.length > 1}>
					<summary>
						<span class="name">{room.name}</span>
						<span class="muted">{room.reached ? `${room.variants.length} ${room.variants.length === 1 ? 'way' : 'ways'} the text reads` : 'not reached'}</span>
					</summary>
					<ol class="variants">
						{#each room.variants as v, i (i)}
							<li class="item">
								<p class="text">{@render prose(v.text)}</p>
								<p class="muted">
									{v.states} {v.states === 1 ? 'state' : 'states'}{v.facts.length ? ` · ${v.facts.join(', ')}` : ''}
								</p>
								{@render path(v.path)}
								<Button size="compact" variant="quiet" onclick={() => onreplay(v.path)}>Replay this path</Button>
							</li>
						{/each}
					</ol>
				</details>
			{/each}
		</section>

		<section class="group">
			<SectionRule title="never shown" />
			{#each result.never_shown as f (f)}<p class="warning-text"><code>{f}</code></p>{:else}<p class="muted">Every room and presence fragment showed.</p>{/each}
		</section>
		<section class="group">
			<SectionRule title="never fired" />
			{#each result.never_fired as r (r)}<p class="warning-text"><code>{r}</code></p>{:else}<p class="muted">Every rule fired.</p>{/each}
		</section>
		{#if result.threads?.length}
			<section class="group">
				<SectionRule title="threads" />
				{#each result.threads as t (t.thread)}
					<div class="item">
						{#if !t.paid_off}
							<p class="warning-text">
								<code>{t.thread}</code>: no explored path shows its payoff ({t.payoffs.join(', ')}).
							</p>
						{:else if t.payoff_before_setup.length}
							<p class="warning-text"><code>{t.thread}</code>: the payoff shows before any setup on this path:</p>
							{@render path(t.payoff_before_setup)}
							<Button size="compact" variant="quiet" onclick={() => onreplay(t.payoff_before_setup)}>Replay this path</Button>
						{:else}
							<p class="muted"><code>{t.thread}</code>: the payoff shows, after a setup.</p>
						{/if}
					</div>
				{/each}
			</section>
		{/if}
	{/if}
</section>

<style>
	.explorer {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}
	p {
		margin: 0;
	}
	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3) var(--space-4);
	}
	.muted {
		color: var(--color-text-muted);
	}
	.summary {
		color: var(--color-text-strong);
	}
	.warning,
	.warning-text {
		color: var(--color-text-warning);
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.item {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-3);
		padding: var(--space-4) var(--space-5);
		border: var(--size-border) solid var(--color-border-subtle);
	}
	.stuck {
		border-color: var(--color-warning);
	}
	.text {
		font-family: var(--font-prose);
		line-height: var(--line-height-body);
		white-space: pre-line;
	}
	.path {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2) var(--space-3);
		margin: 0;
		padding: 0;
		list-style: none;
		font-size: var(--font-size-sm);
	}
	.path li + li::before {
		content: '→ ';
		color: var(--color-text-muted);
	}
	.room > summary {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
		min-height: var(--size-target-compact);
		align-items: center;
		cursor: pointer;
	}
	.name {
		font-weight: var(--font-weight-strong);
	}
	.variants {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin: var(--space-3) 0 0;
		padding: 0;
		list-style: none;
	}
</style>
