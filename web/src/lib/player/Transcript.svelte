<script lang="ts">
	import type { Block, Run } from '#lib/engine/index.ts';
	import { settings } from '#lib/settings.svelte.ts';
	import type { Session } from './session.svelte.ts';

	let { session }: { session: Session } = $props();

	let scroller: HTMLDivElement | undefined = $state();
	let lastGlitch = 0;
	let glitchId = $state(-1);

	const latest = $derived(session.entries.at(-1)?.id ?? -1);

	// Glitch only on new text, and never more than once every 2 s.
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
		if (settings.reduceMotion || settings.effects === 'off') return 0;
		return settings.effects === 'full' ? 120 + index * 520 : 60 + index * 200;
	}

	/** Show a fragment number where a new room fragment starts. */
	function marker(runs: Run[], i: number): number | null {
		const src = runs[i].source;
		if (!settings.sources || !src || !src.owner.startsWith('room:')) return null;
		const prev = runs[i - 1]?.source;
		if (prev && prev.owner === src.owner && prev.fragment === src.fragment) return null;
		return src.fragment + 1;
	}

	/** `*word*` in story text is emphasis. */
	function pieces(text: string): { text: string; em: boolean }[] {
		return text.split('*').map((t, i) => ({ text: t, em: i % 2 === 1 }));
	}

	function isRoomRun(run: Run): boolean {
		return !!run.source && run.source.owner === `room:${session.scene.room}`;
	}
</script>

<!-- Runs are written without whitespace between tags: any gap would show in the prose. -->
{#snippet runs(block: Block)}{#each block.runs as run, i (i)}{@const n = marker(block.runs, i)}{#if n !== null}<button
				class="marker"
				class:focus={session.focus === n - 1 && isRoomRun(run)}
				aria-label="Fragment {n}"
				onmouseenter={() => isRoomRun(run) && (session.focus = n - 1)}
				onmouseleave={() => (session.focus = null)}
				onfocus={() => isRoomRun(run) && (session.focus = n - 1)}
				onblur={() => (session.focus = null)}>{n}</button
			>{/if}{#if run.link?.kind === 'thing'}{@const id = run.link.id}<button
				class="link thing"
				onclick={() => session.pickThing(id, run.text)}>{run.text}</button
			>{:else if run.link?.kind === 'exit'}{@const dir = run.link.id}<button
				class="link exit"
				onclick={() => session.pickExit(dir)}>{run.text}</button
			>{:else}<span
				class:focused={session.focus !== null && run.source?.fragment === session.focus && isRoomRun(run)}
				>{#each pieces(run.text) as p, j (j)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}</span
			>{/if}{/each}{/snippet}

<div class="scroller" bind:this={scroller}>
	<div class="transcript" aria-live="polite" aria-relevant="additions">
		{#each session.entries as entry (entry.id)}
			{@const fresh = entry.id === latest}
			<div class="turn" class:fx-glitch={entry.id === glitchId && fresh}>
				{#each entry.turn.blocks as block, bi (bi)}
					<p
						class="block {block.kind}"
						class:fx-line={fresh && entry.id > 0}
						style:animation-delay={fresh ? `${revealDelay(bi)}ms` : undefined}
					>{#if block.kind === 'command'}&gt;&nbsp;{/if}{@render runs(block)}</p>
				{/each}
			</div>
		{/each}
	</div>
</div>

<style>
	.scroller {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		scrollbar-color: var(--chip-line) transparent;
	}
	.transcript {
		max-width: 70ch;
		font-family: var(--prose);
		font-size: 17px;
		line-height: 1.65;
		text-shadow: var(--glow);
		padding-bottom: 8px;
	}
	.block {
		margin: 4px 0 0;
	}
	.command {
		margin-top: 20px;
		color: var(--muted);
		font-family: var(--ui);
		font-size: 14px;
	}
	.turn:first-child .block:first-child {
		margin-top: 0;
	}
	.title {
		margin-top: 14px;
		color: var(--accent);
		font-weight: 600;
	}
	.exits,
	.objects {
		color: var(--text);
	}
	.exits {
		color: var(--muted);
		font-size: 15px;
	}
	.narration {
		color: var(--strong);
	}
	.end {
		margin-top: 18px;
		color: var(--accent);
		font-style: italic;
	}
	.link {
		background: none;
		border: 0;
		padding: 0;
		font: inherit;
		color: var(--strong);
		text-decoration: underline;
		text-decoration-color: var(--chip-line);
		text-underline-offset: 3px;
		text-shadow: inherit;
	}
	.link:hover {
		color: var(--accent);
		text-decoration-color: var(--accent);
	}
	.exit {
		color: var(--text);
	}
	.marker {
		vertical-align: super;
		font-family: var(--ui);
		font-size: 10px;
		line-height: 1;
		color: var(--warn-text);
		background: none;
		border: 0;
		padding: 0 2px 0 0;
		text-shadow: none;
	}
	.marker.focus {
		color: var(--warn);
		font-weight: 600;
	}
	.focused {
		outline: 1px solid var(--warn);
		outline-offset: 2px;
		color: var(--strong);
	}
</style>
