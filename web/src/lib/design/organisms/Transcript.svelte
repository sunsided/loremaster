<script lang="ts">
	import type { Block, Run } from '#lib/engine/types.ts';
	import FragmentMarker from '../atoms/FragmentMarker.svelte';
	import ProseLink from '../atoms/ProseLink.svelte';
	import type { EffectsLevel, TranscriptEntry } from '../types.ts';

	interface Props {
		entries: TranscriptEntry[];
		/** The current room id. Fragment numbers show for this room only. */
		room: string;
		/** Show fragment numbers before room text. */
		showSources?: boolean;
		/** The highlighted fragment index, or null. */
		focus?: number | null;
		effects?: EffectsLevel;
		reduceMotion?: boolean;
		onthing: (id: string, name: string) => void;
		onexit: (direction: string) => void;
		onfocus?: (index: number | null) => void;
	}

	let {
		entries,
		room,
		showSources = true,
		focus = null,
		effects = 'off',
		reduceMotion = true,
		onthing,
		onexit,
		onfocus
	}: Props = $props();

	let scroller: HTMLDivElement | undefined = $state();
	let lastGlitch = 0;
	let glitchId = $state(-1);

	const latest = $derived(entries.at(-1)?.id ?? -1);

	// The glitch shows only on new text, and not more than one time in 2 s.
	$effect(() => {
		const id = latest;
		const now = performance.now();
		if (id > 0 && now - lastGlitch > 2000) {
			lastGlitch = now;
			glitchId = id;
		}
	});

	$effect(() => {
		latest;
		queueMicrotask(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
	});

	function revealDelay(index: number): number {
		if (reduceMotion || effects === 'off') return 0;
		return effects === 'full' ? 120 + index * 520 : 60 + index * 200;
	}

	function isRoomRun(run: Run): boolean {
		return !!run.source && run.source.owner === `room:${room}`;
	}

	/** The fragment number to show before this run, or null. */
	function marker(runs: Run[], i: number): number | null {
		const src = runs[i].source;
		if (!showSources || !src || !isRoomRun(runs[i])) return null;
		const prev = runs[i - 1]?.source;
		if (prev && prev.owner === src.owner && prev.fragment === src.fragment) return null;
		return src.fragment + 1;
	}

	/** `*word*` in story text is emphasis. */
	function pieces(text: string): { text: string; em: boolean }[] {
		return text.split('*').map((t, i) => ({ text: t, em: i % 2 === 1 }));
	}
</script>

<!-- Runs have no whitespace between tags: a gap would show in the prose. -->
{#snippet runs(block: Block)}{#each block.runs as run, i (i)}{@const n = marker(block.runs, i)}{#if n !== null}<FragmentMarker
				number={n}
				active={focus === n - 1}
				onactivate={(on) => onfocus?.(on ? n - 1 : null)}
			/>{/if}{#if run.link?.kind === 'thing'}{@const id = run.link.id}<ProseLink
				kind="thing"
				onclick={() => onthing(id, run.text)}>{run.text}</ProseLink
			>{:else if run.link?.kind === 'exit'}{@const dir = run.link.id}<ProseLink
				kind="exit"
				onclick={() => onexit(dir)}>{run.text}</ProseLink
			>{:else}<span class:focused={focus !== null && run.source?.fragment === focus && isRoomRun(run)}
				>{#each pieces(run.text) as p, j (j)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}</span
			>{/if}{/each}{/snippet}

<!-- The log scrolls, so it must be focusable: keyboard users scroll it with the arrow keys. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="scroller" bind:this={scroller} role="log" aria-label="Story transcript" tabindex="0">
	<div class="transcript">
		{#each entries as entry (entry.id)}
			{@const fresh = entry.id === latest}
			<div class="turn" class:fx-glitch={entry.id === glitchId && fresh}>
				{#each entry.turn.blocks as block, bi (bi)}
					<p
						class="block {block.kind}"
						class:fx-line={fresh && entry.id > 0}
						style:animation-delay={fresh ? `${revealDelay(bi)}ms` : undefined}
					>{#if block.kind === 'command'}<span aria-hidden="true">&gt;&nbsp;</span><span class="visually-hidden"
							>You typed:</span
						>{:else if block.kind === 'author'}<span class="badge">author</span><span class="visually-hidden">:</span
						>{:else if block.kind === 'note'}<span class="visually-hidden">Parser note:</span>{/if}{@render runs(block)}</p>
				{/each}
			</div>
		{/each}
	</div>
</div>

<style>
	.scroller {
		position: relative; /* Keeps screen-reader text inside the scroll area. */
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
	}
	.transcript {
		max-width: var(--measure-prose);
		padding-bottom: var(--space-4);
		font-family: var(--font-prose);
		font-size: var(--font-size-prose);
		line-height: var(--line-height-prose);
		text-shadow: var(--glow, none);
		overflow-wrap: break-word;
	}
	.block {
		margin: var(--space-2) 0 0;
	}
	.turn:first-child .block:first-child {
		margin-top: 0;
	}
	.command {
		margin-top: var(--space-7);
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-lg);
	}
	.title {
		margin-top: var(--space-6);
		color: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
	}
	.exits {
		color: var(--color-text-muted);
		font-size: var(--font-size-lg);
	}
	.narration {
		color: var(--color-text-strong);
	}
	.end {
		margin-top: var(--space-7);
		color: var(--color-text-accent);
		font-style: italic;
	}
	/* A parser note: the game did an extra step by itself. Smaller, in the interface font, so it does not read as prose. */
	.note {
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		font-style: italic;
		color: var(--color-text-muted);
	}
	/* An author action is not part of the game. It looks like an editor message, not like story text. */
	.author {
		margin-top: var(--space-7);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		color: var(--color-text-warning);
	}
	.badge {
		margin-right: var(--space-3);
		padding: 0 var(--space-3);
		border: var(--size-border) dashed var(--color-warning);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
	}
	.focused {
		outline: var(--size-border) solid var(--color-warning);
		outline-offset: 2px;
		color: var(--color-text-strong);
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
