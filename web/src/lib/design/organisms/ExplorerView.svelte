<script lang="ts">
	import type { Exploration } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';
	import Heading from '../atoms/Heading.svelte';
	import SectionRule from '../molecules/SectionRule.svelte';

	interface Props {
		/** The last result, or null before the first run. */
		result?: Exploration | null;
		/** True while the explorer runs. */
		running?: boolean;
		onrun: () => void;
		/** Start again and play a path. */
		onreplay: (path: string[]) => void;
	}

	let { result = null, running = false, onrun, onreplay }: Props = $props();

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
	<Heading level={2} id="explore-title">Explore</Heading>
	<p class="muted">
		The explorer plays all ways through the story from the start, within a budget. It tries the actions that move the
		story forward. Use it to see how complicated text reads in each state, and to find text that never shows.
	</p>
	<div><Button variant="primary" onclick={onrun} disabled={running}>{running ? 'Exploring…' : result ? 'Explore again' : 'Explore all states'}</Button></div>

	{#if result}
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
