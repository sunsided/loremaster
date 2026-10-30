<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { Block, Run, Source } from '#lib/engine/types.ts';
	import FragmentMarker from '../atoms/FragmentMarker.svelte';
	import ProseLink from '../atoms/ProseLink.svelte';
	import { lineReveal, revealLines } from '../reveal.ts';
	import type { EffectsLevel, RevealMode, GhostLine, MarginInfo, ProposalSummary, ReviewFinding, RoomBrief, TranscriptEntry, WritingAt } from '../types.ts';
	import GhostText from '../atoms/GhostText.svelte';
	import ProposalCard from '../molecules/ProposalCard.svelte';
	import FindingNote from '../molecules/FindingNote.svelte';

	interface Props {
		entries: TranscriptEntry[];
		/** The current room id. Fragment numbers show for this room only. */
		room: string;
		/** Show fragment numbers before room text. The app shows them in write mode only. */
		showSources?: boolean;
		/** The highlighted fragment index, or null. */
		focus?: number | null;
		effects?: EffectsLevel;
		reduceMotion?: boolean;
		/** How new text appears with effects on: line by line (`lines`) or a paragraph at once (`blocks`). */
		reveal?: RevealMode;
		onthing: (id: string, name: string) => void;
		onexit: (direction: string) => void;
		onfocus?: (index: number | null) => void;
		/** Where the player is and what is in reach, for the room brief under the newest turn. */
		brief?: RoomBrief | null;
		/** Write mode: the id of the entry whose text the author can edit, or null. */
		writable?: number | null;
		/** The fragment that the writer edits now, or null. */
		writing?: WritingAt | null;
		/** The writer with all the tools. It shows after the paragraph of the fragment that it edits. Leave it out when the writer shows elsewhere, for example next to the game text. */
		writer?: Snippet;
		/** The prose view of the fragment under edit. It shows in place of the fragment, in the game text. */
		editor?: Snippet;
		/** Open the writer for a fragment. */
		onwrite?: (source: Source) => void;
		/** Open the writer for a new room fragment. */
		onadd?: () => void;
		/**
		 * Write mode: notes next to each fragment of the room text (layer, condition, why it is hidden), and the
		 * fragments that do not show now. Each fragment gets its own line, with its note in the margin.
		 */
		margin?: MarginInfo | null;
		/** Write mode: ghost lines of AI proposals for the room text, with their proposals for the cards in the margin. */
		ghosts?: GhostLine[];
		proposals?: ProposalSummary[];
		onproposal?: (id: string, action: 'accept' | 'reject', note?: string) => void;
		/** Play a proposal in a sandbox before the author accepts it. */
		onwalk?: (id: string) => void;
		/** Write mode with the margin: the findings of an AI review, next to their fragments. */
		findings?: ReviewFinding[];
		ondismiss?: (key: string) => void;
		/** A problem from the last proposal action, by proposal id. */
		proposalProblems?: Record<string, string>;
		/** A preview of the room text in other values. It shows in place of the newest room text, marked. */
		preview?: Block[] | null;
		/** In write mode: the last command, when it got the default response. It offers "Write a response". */
		respond?: string | null;
		onrespond?: () => void;
	}

	let {
		entries,
		room,
		showSources = true,
		focus = null,
		effects = 'off',
		reduceMotion = true,
		reveal = 'lines',
		onthing,
		onexit,
		onfocus,
		brief = null,
		writable = null,
		writing = null,
		writer,
		editor,
		onwrite,
		onadd,
		respond = null,
		onrespond,
		preview = null,
		margin = null,
		ghosts = [],
		proposals = [],
		onproposal,
		onwalk,
		findings = [],
		ondismiss,
		proposalProblems = {}
	}: Props = $props();

	/** The first ghost line of each proposal gets its card in the margin. */
	const firstGhost = $derived(new Set(proposals.map((p) => ghosts.find((g) => g.proposal === p.id)).filter(Boolean)));

	/** The findings next to a fragment that shows; the others (the whole room, or a hidden fragment) go under the room text. */
	const shownIndexes = $derived(new Set(margin?.fragments.filter((f) => f.shown).map((f) => f.index) ?? []));
	const findingsAt = (owner: string, index: number) => findings.filter((f) => f.owner === owner && f.fragment === index && shownIndexes.has(index));
	const roomFindings = $derived(
		findings.filter((f) => !(margin && f.owner === margin.owner && f.fragment !== null && shownIndexes.has(f.fragment)))
	);

	/** Show the room fragments that do not show now, in place. */
	let hiddenOpen = $state(false);
	const hiddenFragments = $derived(margin?.fragments.filter((f) => !f.shown) ?? []);
	const uid = $props.id();

	/** The lines of a block in the margin view: one for each fragment; generated text (exits, "There is …") stays together. */
	function marginGroups(block: Block): { source?: Source; runs: Run[] }[] {
		if (block.kind === 'exits') return [{ runs: block.runs }];
		const out: { source?: Source; runs: Run[] }[] = [];
		for (const g of groups(block.runs)) {
			const last = out.at(-1);
			if (!g.source && last && !last.source) last.runs.push(...g.runs);
			else out.push({ source: g.source, runs: [...g.runs] });
		}
		// Spaces between fragments are not a line of their own.
		return out.filter((g) => g.source || g.runs.some((r) => r.text.trim()));
	}

	/** The margin note for a fragment: a tag and a detail. */
	function noteFor(source: Source | undefined): { tag: string; detail: string; generated?: boolean } | null {
		if (!source) return null;
		if (margin && source.owner === margin.owner) {
			const f = margin.fragments.find((x) => x.index === source.fragment);
			return f ? { tag: f.layer, detail: f.detail } : { tag: source.layer, detail: '' };
		}
		const [entity, part] = source.owner.split('/');
		if (part === 'presence') return { tag: 'presence', detail: entity.replace('thing:', '') };
		if (part === 'frames') return { tag: 'frame', detail: source.owner.split('/').slice(2).join(' · ') };
		return { tag: source.layer, detail: entity };
	}

	/** The newest turn that shows a room: a preview replaces its room text. */
	const previewAt = $derived(preview ? (entries.findLast((e) => e.turn.blocks.some((b) => b.kind === 'title'))?.id ?? -1) : -1);

	/** The blocks of a turn, with the preview in place of the room text, and where the preview starts. */
	function shownBlocks(entry: TranscriptEntry): { blocks: Block[]; from: number } {
		if (entry.id !== previewAt || !preview) return { blocks: entry.turn.blocks, from: Infinity };
		const blocks = entry.turn.blocks;
		const start = blocks.findLastIndex((b) => b.kind === 'title');
		if (start < 0) return { blocks, from: Infinity };
		let end = start + 1;
		while (end < blocks.length && ['prose', 'objects', 'exits'].includes(blocks[end].kind)) end++;
		return { blocks: [...blocks.slice(0, start), ...preview, ...blocks.slice(end)], from: start, to: start + preview.length } as {
			blocks: Block[];
			from: number;
		};
	}

	const same = (a: Source | undefined, w: WritingAt | null) => !!a && !!w && a.owner === w.owner && a.fragment === w.fragment;
	const editable = (kind: string) => kind === 'prose' || kind === 'objects';

	/** The runs of a block in groups: one group for each fragment, and one for each run without a source. */
	function groups(runs: Run[]): { source?: Source; runs: Run[] }[] {
		const out: { source?: Source; runs: Run[] }[] = [];
		for (const run of runs) {
			const last = out.at(-1);
			if (run.source && last?.source && last.source.owner === run.source.owner && last.source.fragment === run.source.fragment)
				last.runs.push(run);
			else out.push({ source: run.source, runs: [run] });
		}
		return out;
	}

	/** The last block of room text, or the room title when the room has no text yet. */
	function lastRoomText(blocks: Block[]): number {
		const prose = blocks.findLastIndex((b) => b.kind === 'prose');
		return prose >= 0 ? prose : blocks.findIndex((b) => b.kind === 'title');
	}

	/** The block after which the writer shows: the block with the fragment, else the last block of room text. */
	function writerAfter(blocks: Block[]): number {
		if (writing && writing.fragment === -2) return -1;
		const at = blocks.findIndex((b) => b.runs.some((r) => same(r.source, writing)));
		if (at >= 0) return at;
		const prose = blocks.findLastIndex((b) => b.kind === 'prose');
		return prose >= 0 ? prose : blocks.findIndex((b) => b.kind === 'title');
	}

	/** Show the brief only when the newest turn did not print the room and the game goes on. */
	const showBrief = $derived.by(() => {
		const last = entries.at(-1)?.turn;
		return !!last && !last.ended && !last.blocks.some((b) => b.kind === 'title');
	});

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

	/** True where a run of action frame text starts. */
	function isFrameRun(runs: Run[], i: number): boolean {
		const src = runs[i].source;
		if (!src || !src.owner.includes('/frames/')) return false;
		const prev = runs[i - 1]?.source;
		return !(prev && prev.owner === src.owner && prev.fragment === src.fragment);
	}

	/** "story frame for look, before", from an owner such as `story/frames/look/before`. */
	function frameScope(run: Run): string {
		const [scope, , verb, part] = (run.source?.owner ?? '').split('/');
		return `${scope} frame for ${verb}, ${part}`;
	}

	/** `*word*` in story text is emphasis. */
	function pieces(text: string): { text: string; em: boolean }[] {
		return text.split('*').map((t, i) => ({ text: t, em: i % 2 === 1 }));
	}
</script>

<!-- Runs have no whitespace between tags: a gap would show in the prose. -->
{#snippet runs(block: Block)}{#each block.runs as run, i (i)}{@const n = marker(block.runs, i)}{#if showSources && isFrameRun(block.runs, i)}<span
				class="frame-mark"
				title="Action frame: {frameScope(run)}"
				><span aria-hidden="true">frame</span><span class="visually-hidden">Action frame text:</span></span
			>{/if}{#if n !== null}<FragmentMarker
				number={n}
				active={focus === n - 1}
				onactivate={(on) => onfocus?.(on ? n - 1 : null)}
			/>{/if}{#if run.link?.kind === 'thing'}{@const id = run.link.id}<ProseLink
				kind="thing"
				inspect="thing:{id}"
				onclick={() => onthing(id, run.text)}>{run.text}</ProseLink
			>{:else if run.link?.kind === 'exit'}{@const dir = run.link.id}<ProseLink
				kind="exit"
				inspect="exit:{dir}"
				onclick={() => onexit(dir)}>{run.text}</ProseLink
			>{:else}<span class:focused={focus !== null && run.source?.fragment === focus && isRoomRun(run)}
				>{#each pieces(run.text) as p, j (j)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}</span
			>{/if}{/each}{/snippet}

<!--
	In write mode, each fragment of the room text is a button that opens the writer.
	Links are plain text there: a click edits, it does not play.
-->
{#snippet writableRuns(block: Block, part: 'all' | 'before' | 'after' = 'all')}{@const gs = groups(block.runs)}{@const cut = gs.findIndex((g) => same(g.source, writing))}{#each gs as g, gi (gi)}{#if (part === 'before' && gi >= cut) || (part === 'after' && gi <= cut)}{:else if same(g.source, writing)}<span class="being-written"
				>{#each g.runs as run, i (i)}{run.text}{/each}</span
			>{:else if g.source}{@const src = g.source}<button
				type="button"
				class="writable"
				onclick={() => onwrite?.(src)}
				><span class="visually-hidden">Edit: </span>{#each g.runs as run, i (i)}{#each pieces(run.text) as p, j (j)}{#if p.em}<em
								>{p.text}</em
							>{:else}{p.text}{/if}{/each}{/each}</button
			>{:else}{#each g.runs as run, i (i)}{run.text}{/each}{/if}{/each}{/snippet}

{#snippet ghostRow(g: GhostLine)}
	{@const card = firstGhost.has(g) ? proposals.find((p) => p.id === g.proposal) : undefined}
	<div class="mrow ghost-row">
		<p class="block prose mtext">
			{#if g.kind === 'removed'}<GhostText look="block">removes the line above</GhostText>{:else}<GhostText look="block"
					>{g.text}</GhostText
				>{/if}
		</p>
		{#if card}
			<div class="mnote">
				<ProposalCard
					proposal={card}
					problem={proposalProblems[card.id] ?? null}
					onaccept={() => onproposal?.(card.id, 'accept')}
					onreject={(note) => onproposal?.(card.id, 'reject', note)}
					onwalk={onwalk ? () => onwalk(card.id) : undefined}
				/>
			</div>
		{:else}
			<p class="mnote ghost-note"><span class="tag">✦ {g.proposal}</span>{g.kind}</p>
		{/if}
	</div>
{/snippet}

<!--
	Write mode with the margin: each fragment on its own line, and its note in the margin next to it.
	The note describes the fragment for screen readers too (aria-describedby).
-->
{#snippet marginBlock(block: Block, bi: number)}{@const gs = marginGroups(block)}
	{#each gs as g, gi (gi)}
		{@const note = noteFor(g.source) ?? (block.kind === 'exits' ? { tag: 'exits', detail: `generated from ${margin?.exits ?? 0} exits`, generated: true } : block.kind === 'objects' ? { tag: 'objects', detail: `generated from ${margin?.things ?? 0} things`, generated: true } : null)}
		{@const noteId = `${uid}-n${bi}-${gi}`}
		<div class="mrow">
			<p class="block {block.kind} mtext">
				{#if same(g.source, writing) && editor}{@render editor()}{:else if g.source && editable(block.kind)}{@const src = g.source}<button
						type="button"
						class="writable"
						aria-describedby={note ? noteId : undefined}
						onclick={() => onwrite?.(src)}
						><span class="visually-hidden">Edit: </span>{#each g.runs as run, i (i)}{#each pieces(run.text) as p, j (j)}{#if p.em}<em
										>{p.text}</em
									>{:else}{p.text}{/if}{/each}{/each}</button
					>{:else}{@render runs({ kind: block.kind, runs: g.runs })}{/if}
			</p>
			{#if note || (g.source && findings.length)}
				<div class="mside">
					{#if note}
						<p class="mnote" class:generated={note.generated} id={noteId}>
							<span class="tag">{note.tag}</span>{note.detail}
						</p>
					{/if}
					{#if g.source && margin && g.source.owner === margin.owner}
						{#each findingsAt(g.source.owner, g.source.fragment) as f (f.key)}
							<FindingNote finding={f} ondismiss={ondismiss ? () => ondismiss(f.key) : undefined} />
						{/each}
					{/if}
				</div>
			{/if}
		</div>
		{#if margin && g.source && g.source.owner === margin.owner}
			{#each ghosts.filter((x) => x.kind !== 'added' && x.before === g.source?.fragment) as gh, k (k)}{@render ghostRow(gh)}{/each}
		{/if}
	{/each}
	{#if block.kind === 'prose'}
		{#each ghosts.filter((x) => x.kind === 'added') as gh, k (k)}{@render ghostRow(gh)}{/each}
	{/if}
	{#if block.kind === 'prose' && roomFindings.length}
		<div class="mrow">
			<p class="mtext review-label">The AI review, about the whole room or a hidden fragment:</p>
			<div class="mside">
				{#each roomFindings as f (f.key)}
					<FindingNote
						finding={{ ...f, problem: f.fragment === null ? f.problem : `Fragment ${f.fragment + 1}: ${f.problem}` }}
						ondismiss={ondismiss ? () => ondismiss(f.key) : undefined}
					/>
				{/each}
			</div>
		</div>
	{/if}
	{#if block.kind === 'prose' && hiddenFragments.length}
		<div class="mrow">
			<p class="mtext">
				<button type="button" class="hidden-chip" aria-expanded={hiddenOpen} onclick={() => (hiddenOpen = !hiddenOpen)}
					>{hiddenOpen ? '−' : '+'} {hiddenFragments.length} hidden here</button
				>
			</p>
		</div>
		{#if hiddenOpen}
			{#each hiddenFragments as f (f.index)}
				{@const noteId = `${uid}-h${f.index}`}
				<div class="mrow hidden-row">
					<p class="block prose mtext">
						{#if writing && margin && writing.owner === margin.owner && writing.fragment === f.index && editor}{@render editor()}{:else}<button
								type="button"
								class="writable struck"
								aria-describedby={noteId}
								onclick={() =>
									onwrite?.({ owner: margin!.owner, fragment: f.index, layer: f.layer as Source['layer'], variant: 0 })}
								><span class="visually-hidden">Edit, does not show now: </span>{f.text}</button
							>{/if}
					</p>
					<p class="mnote warn" id={noteId}><span class="tag">{f.layer}</span>{f.why ?? 'hidden'}</p>
				</div>
			{/each}
		{/if}
	{/if}
{/snippet}

<!-- The log scrolls, so it must be focusable: keyboard users scroll it with the arrow keys. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="scroller" bind:this={scroller} role="region" aria-label="Story" tabindex="0">
	<div class="transcript" role="log" aria-label="Story transcript">
		{#each entries as entry (entry.id)}
			{@const fresh = entry.id === latest}
			{@const writeHere = entry.id === writable}
			{@const writerAt = writeHere && writing ? writerAfter(entry.turn.blocks) : -1}
			{@const addAt = writeHere && !writing && onadd ? lastRoomText(entry.turn.blocks) : -1}
			{@const shown = shownBlocks(entry)}
			<div
				{@attach revealLines(fresh && reveal === 'lines' && entry.id > 0 ? lineReveal(effects, reduceMotion) : null)}
				class="turn"
				class:fx-glitch={entry.id === glitchId && fresh} class:write={writeHere} class:margined={writeHere && !!margin}>
				{#if shown.from !== Infinity}<p class="visually-hidden">Preview of the room text in other values. The game did not move.</p>{/if}
				{#each shown.blocks as block, bi (bi)}
					{@const inPlace = !!editor && writeHere && editable(block.kind) && block.runs.some((r) => same(r.source, writing))}
					{#if writeHere && margin && (editable(block.kind) || block.kind === 'exits')}
						{@render marginBlock(block, bi)}
					{:else if inPlace}
						<!-- The fragment under edit: the prose view in place, between the text before and after it. -->
						<div class="block {block.kind} editing">
							<span>{@render writableRuns(block, 'before')}</span>
							{@render editor?.()}
							<span>{@render writableRuns(block, 'after')}</span>
						</div>
					{:else}
					<p
						class="block {block.kind}"
						class:previewed={bi >= shown.from && bi < shown.from + (preview?.length ?? 0)}
						class:fx-line={fresh && entry.id > 0 && !writeHere && reveal === 'blocks'}
						data-reveal={fresh && entry.id > 0 && !writeHere && reveal === 'lines' ? '' : undefined}
						style:animation-delay={fresh && reveal === 'blocks' ? `${revealDelay(bi)}ms` : undefined}
					>{#if block.kind === 'command'}<span aria-hidden="true">&gt;&nbsp;</span><span class="visually-hidden"
							>You typed:</span
						>{:else if block.kind === 'author'}<span class="badge">author</span><span class="visually-hidden">:</span
						>{:else if block.kind === 'note'}<span class="visually-hidden">Parser note:</span>{/if}{#if writeHere && editable(block.kind)}{@render writableRuns(
								block
							)}{:else}{@render runs(block)}{/if}</p>
					{/if}
					{#if bi === writerAt && editor && writing && !entry.turn.blocks.some((b) => b.runs.some((r) => same(r.source, writing))) && !(margin && hiddenOpen && writing.owner === margin.owner && hiddenFragments.some((f) => f.index === writing?.fragment))}
						<!-- A new fragment, or one that does not show now: the prose view after the room text. -->
						<div class="block prose editing">{@render editor()}</div>
					{/if}
					{#if bi === writerAt}{@render writer?.()}{/if}
					{#if bi === addAt}
						<!-- New room text goes after the room text, so the button is there too, above the things and exits. -->
						<p class="add"><button type="button" class="add-button" onclick={onadd}>+ Add room text</button></p>
					{/if}
				{/each}
				{#if entry.rewritten}<p class="rewritten">Written again after an edit of the story.</p>{/if}
				{#if respond && onrespond && !writing && entry.id === latest}
					<p class="add">
						<button type="button" class="add-button" onclick={onrespond}>+ Write a response for “{respond}”</button>
					</p>
				{/if}
				{#if writing?.fragment === -2 && entry.id === latest}{@render writer?.()}{/if}
			</div>
		{/each}
	</div>
	{#if showBrief && brief}
		<!--
			The room brief: where the player is and what they can act on. It is not part of the log,
			so it is not read again after each turn. It follows the newest turn when that turn did not
			print the room.
		-->
		<p class="brief" aria-label="Room brief">
			<span class="brief-room">{brief.room}</span>{#if brief.things.length}{' · here: '}{#each brief.things as t, i (t.id)}{#if i > 0}{', '}{/if}<ProseLink
						inspect="thing:{t.id}"
						onclick={() => onthing(t.id, t.name)}>{t.name}</ProseLink
					>{/each}{/if}{#if brief.exits.length}{' · exits: '}{#each brief.exits as e, i (e.direction)}{#if i > 0}{', '}{/if}<ProseLink
						kind="exit"
						inspect="exit:{e.direction}"
						onclick={() => onexit(e.direction)}>{e.direction}</ProseLink
					>{/each}{/if}
		</p>
	{/if}
</div>

<style>
	.scroller {
		position: relative; /* Keeps screen-reader text inside the scroll area. */
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
	}
	.scroller {
		container-type: inline-size;
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
	.frame-mark {
		margin-right: var(--space-2);
		padding: 0 var(--space-2);
		border: var(--size-border) dashed var(--color-border-control);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
		vertical-align: middle;
		text-shadow: none;
		user-select: none;
	}
	/* Write mode: each fragment is a button that looks like text, with an outline on hover and focus. */
	.writable {
		display: inline;
		margin: 0;
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: inherit;
		text-shadow: inherit;
		cursor: text;
		box-decoration-break: clone;
		-webkit-box-decoration-break: clone;
		border-bottom: var(--size-border) dashed var(--color-border-control);
	}
	.writable:hover {
		background: var(--color-accent-tint);
		border-bottom-color: var(--color-accent);
	}
	.writable:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 1px;
	}
	/* Write mode with the margin: the turn is wider than the prose, for the notes on the right. */
	.transcript:has(.margined) {
		max-width: calc(var(--measure-prose) + var(--width-margin) + var(--space-7));
	}
	.transcript:has(.margined) > .turn:not(.margined) {
		max-width: var(--measure-prose);
	}
	.mrow {
		display: grid;
		grid-template-columns: minmax(0, var(--measure-prose)) var(--width-margin);
		gap: 0 var(--space-7);
		align-items: start;
	}
	.mtext,
	.mside {
		min-width: 0;
	}
	.mtext .writable {
		display: inline;
	}
	.mnote {
		margin: var(--space-2) 0 0;
		padding-left: var(--space-4);
		border-left: var(--size-border) solid var(--color-border-subtle);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-body);
		color: var(--color-text-muted);
		text-shadow: none;
	}
	.mnote .tag {
		display: inline-block;
		margin-right: var(--space-2);
		padding: 0 var(--space-2);
		border: var(--size-border) solid var(--color-border-control);
		color: var(--color-text-strong);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
	}
	.review-label {
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
	}
	.mnote.generated .tag {
		border-style: dashed;
		color: var(--color-text-muted);
	}
	.mnote.warn {
		color: var(--color-text-warning);
	}
	.ghost-row .mnote:not(.ghost-note) {
		padding-left: 0;
		border-left: 0;
	}
	.ghost-note .tag {
		border-color: var(--color-ghost);
		color: var(--color-ghost);
		text-transform: none;
	}
	.hidden-chip {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-3);
		border: var(--size-border) dashed var(--color-border-control);
		background: none;
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
	}
	.hidden-chip:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.writable.struck {
		color: var(--color-text-muted);
		text-decoration: line-through;
		text-decoration-color: var(--color-border-control);
	}
	/* Narrow: the note goes under its fragment. */
	@container (max-width: 760px) {
		.mrow {
			grid-template-columns: minmax(0, 1fr);
		}
		.mnote {
			margin: 0 0 var(--space-3);
		}
	}
	/* A preview of the room text: a line on the left in the warning colour, like the PREVIEW mark. */
	.previewed {
		margin-left: calc(-1 * var(--space-4));
		padding-left: calc(var(--space-4) - var(--size-border-strong));
		border-left: var(--size-border-strong) solid var(--color-warning);
	}
	.editing > span:empty {
		display: none;
	}
	.being-written {
		color: var(--color-text-muted);
		text-decoration: line-through;
		text-decoration-color: var(--color-border-control);
	}
	.add {
		margin: var(--space-3) 0 0;
	}
	.add-button {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-4);
		border: var(--size-border) dashed var(--color-border-control);
		background: none;
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.add-button:hover {
		border-color: var(--color-accent);
		color: var(--color-text-accent);
	}
	.add-button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.rewritten {
		margin: var(--space-2) 0 0;
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.brief {
		max-width: var(--measure-prose);
		margin: var(--space-6) 0 var(--space-2);
		padding-top: var(--space-4);
		border-top: var(--size-border) dashed var(--color-border-subtle);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-body);
		color: var(--color-text-muted);
	}
	.brief-room {
		color: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
	}
	/* Labels for screen readers only. user-select: none keeps them out of copied text. */
	.visually-hidden {
		user-select: none;
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
