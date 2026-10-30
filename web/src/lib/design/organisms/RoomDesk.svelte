<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { MapView, RuleView } from '#lib/engine/types.ts';
	import PanelSection from '../molecules/PanelSection.svelte';
	import StateSwitches from '../molecules/StateSwitches.svelte';
	import MapPanel from './MapPanel.svelte';
	import type { RoomReach, RoomSwitch, ThreadChip } from '../types.ts';

	type Tab = 'room' | 'notes' | 'ai';

	interface Props {
		/** The name of the current room. */
		room: string;
		/** The file of the room, for example "rooms/shore.yaml". */
		file?: string | null;
		/** The level of the room. Leave it out for a story on one level. */
		level?: number | null;
		/** What the last explorer run found about this room. Null: the explorer did not run yet. */
		reach?: RoomReach | null;
		/** Run the explorer, or open its results. */
		onexplore: () => void;
		/** The values that the room text depends on. */
		switches: RoomSwitch[];
		texts?: number;
		preview?: Record<string, string>;
		onpreview: (name: string, value: string) => void;
		onreset: () => void;
		onapply: () => void;
		/** The author notes and TODOs of the room and its things. */
		notes: Snippet;
		/** A short fact for the notes title, for example "2 · 1 TODO". */
		notesMeta?: string;
		/** The beats of threads in this room. */
		threads?: ThreadChip[];
		onthread: (id: string) => void;
		/** The AI help for this room, while the AI server runs (usually AiHere). */
		ai?: Snippet;
		rules: RuleView[];
		ruleCount: number;
		onrule: (id: string) => void;
		/** The rooms around, for the small map. Null: no small map, for example when the big map shows on the left. */
		map?: MapView | null;
		/** Open a full view in the Atlas. */
		onopen: (view: 'map' | 'rules' | 'explore' | 'threads') => void;
		/** Show the sections as tabs, for the bottom sheet on a phone. */
		tabs?: boolean;
	}

	let {
		room,
		file = null,
		level = null,
		reach = null,
		onexplore,
		switches,
		texts = 0,
		preview = {},
		onpreview,
		onreset,
		onapply,
		notes,
		notesMeta,
		threads = [],
		onthread,
		ai,
		rules,
		ruleCount,
		onrule,
		map = null,
		onopen,
		tabs = false
	}: Props = $props();

	const uid = $props.id();
	let tab = $state<Tab>('room');
	const tabList = $derived<{ id: Tab; label: string }[]>([
		{ id: 'room', label: 'Room' },
		{ id: 'notes', label: 'Notes' },
		...(ai ? [{ id: 'ai' as const, label: 'AI' }] : [])
	]);
	// The AI tab goes away when the server stops.
	$effect(() => {
		if (tab === 'ai' && !ai) tab = 'room';
	});
	const show = (t: Tab) => !tabs || tab === t;

	/** Arrow keys move between the tabs, as in the ARIA tabs pattern. */
	function onkey(e: KeyboardEvent) {
		const i = tabList.findIndex((t) => t.id === tab);
		const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
		if (!step) return;
		e.preventDefault();
		tab = tabList[(i + step + tabList.length) % tabList.length].id;
		document.getElementById(`${uid}-tab-${tab}`)?.focus();
	}

	const roleLabel: Record<ThreadChip['role'], string> = { setup: 'setup', payoff: 'payoff', mention: 'mention', missing: 'no payoff yet' };
</script>

<!--
	The room desk: the side panel in write mode. It holds what the author can change in the current room:
	the values that the room text depends on, the notes and the thread beats, the AI help, and the rules.
	Things that are about the whole story are in the Atlas. Things about one line of text are in the margin.
	On a phone the sections are tabs, because a long list of folded sections is hard to use with a thumb.
-->
<div class="desk" aria-label="Room desk: {room}" role="region">
	<header class="head">
		<p class="room">{room}</p>
		<p class="where">{file ?? 'no file yet'}{level !== null ? ` · level ${level}` : ''}</p>
		<p class="reach">
			{#if !reach}
				<span class="muted">Not explored yet.</span>
				<button type="button" class="link" onclick={onexplore}>Explore ↗</button>
			{:else if reach.reached}
				<span class="ok">● reachable</span>
				<span class="muted"
					>in {reach.states} explored states{reach.truncated ? ', the walk stopped early' : ''}{reach.neverShown.length
						? ` · ${reach.neverShown.length} ${reach.neverShown.length === 1 ? 'line never shows' : 'lines never show'}`
						: ''}</span
				>
				<button type="button" class="link" onclick={onexplore}>Checks ↗</button>
			{:else}
				<span class="bad">▲ not reached</span>
				<span class="muted">{reach.truncated ? 'The walk stopped early.' : 'No explored path comes here.'}</span>
				<button type="button" class="link" onclick={onexplore}>Checks ↗</button>
			{/if}
		</p>
	</header>

	{#if tabs}
		<div class="tabs" role="tablist" aria-label="Room desk">
			{#each tabList as t (t.id)}
				<button
					type="button"
					role="tab"
					id="{uid}-tab-{t.id}"
					aria-selected={tab === t.id}
					aria-controls="{uid}-panel"
					tabindex={tab === t.id ? 0 : -1}
					onclick={() => (tab = t.id)}
					onkeydown={onkey}>{t.label}</button
				>
			{/each}
		</div>
	{/if}

	<div id="{uid}-panel" role={tabs ? 'tabpanel' : undefined} aria-labelledby={tabs ? `${uid}-tab-${tab}` : undefined}>
		{#if show('room')}
			<PanelSection id="desk-states" title="Depends on" meta={switches.length ? `${switches.length} values · ${texts} texts` : 'nothing'}>
				{#snippet actions()}<button type="button" class="link" onclick={() => onopen('explore')}>All texts ↗</button>{/snippet}
				<StateSwitches {switches} {preview} {onpreview} {onreset} {onapply} />
			</PanelSection>
		{/if}

		{#if show('notes')}
			<PanelSection id="desk-notes" title="Notes and threads" meta={notesMeta}>
				{#snippet actions()}<button type="button" class="link" onclick={() => onopen('threads')}>Threads ↗</button>{/snippet}
				{#if threads.length}
					<ul class="chips" aria-label="Thread beats here">
						{#each threads as c (`${c.thread}:${c.role}`)}
							<li>
								<button type="button" class="chip {c.role}" onclick={() => onthread(c.thread)}
									><span class="role">{roleLabel[c.role]}</span> · {c.name}</button
								>
							</li>
						{/each}
					</ul>
				{/if}
				{@render notes()}
			</PanelSection>
		{/if}

		{#if ai && show('ai')}
			<PanelSection id="desk-ai" title="✦ AI here">{@render ai()}</PanelSection>
		{/if}

		{#if show('room')}
			<PanelSection id="desk-rules" title="Rules here" meta="{rules.length} of {ruleCount}">
				{#snippet actions()}<button type="button" class="link" onclick={() => onopen('rules')}>All ↗</button>{/snippet}
				{#if rules.length}
					<ul class="list">
						{#each rules as r (r.id)}
							<li>
								<button type="button" class="row" onclick={() => onrule(r.id)}>
									<span class="name">{r.id}</span><span class="detail">{r.relevant}</span>
								</button>
							</li>
						{/each}
					</ul>
				{:else}
					<p class="note">No rule matters here now.</p>
				{/if}
			</PanelSection>

			{#if map}
				<PanelSection id="desk-map" title="Around you">
					{#snippet actions()}<button type="button" class="link" onclick={() => onopen('map')}>Map ↗</button>{/snippet}
					<MapPanel {map} compact />
				</PanelSection>
			{/if}
		{/if}
	</div>
</div>

<style>
	.desk {
		display: flex;
		flex-direction: column;
	}
	.head {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-4) var(--space-5);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
	}
	.head p {
		margin: 0;
	}
	.room {
		color: var(--color-text-strong);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-strong);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
	}
	.where,
	.muted {
		color: var(--color-text-muted);
	}
	.reach {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0 var(--space-3);
	}
	.ok {
		color: var(--color-text-accent);
	}
	.bad {
		color: var(--color-text-warning);
	}
	.tabs {
		display: flex;
		border-bottom: var(--size-border) solid var(--color-border-subtle);
	}
	[role='tab'] {
		flex: 1 1 0;
		min-height: var(--size-target);
		border: 0;
		background: none;
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	[role='tab'][aria-selected='true'] {
		background: var(--color-accent);
		color: var(--color-text-on-accent);
	}
	.link {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-2);
		border: 0;
		background: none;
		color: var(--color-text-accent);
		font: inherit;
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	.link:focus-visible,
	.row:focus-visible,
	.chip:focus-visible,
	[role='tab']:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin: 0 0 var(--space-3);
		padding: 0;
		list-style: none;
	}
	.chip {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-3);
		border: var(--size-border) dashed var(--color-border-subtle);
		background: none;
		color: var(--color-text);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
	}
	.chip.setup {
		border-style: solid;
		border-color: var(--color-accent);
	}
	.chip.payoff {
		border-style: solid;
		border-color: var(--color-text-strong);
	}
	.chip.missing {
		border-style: solid;
		border-color: var(--color-warning);
		color: var(--color-text-warning);
	}
	.role {
		color: var(--color-text-muted);
	}
	.chip.missing .role {
		color: inherit;
	}
	.note {
		margin: var(--space-3) 0 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-body);
	}
	.list {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 0 var(--space-4);
		width: 100%;
		min-height: var(--size-target-compact);
		padding: var(--space-1) 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
	}
	.row:hover .name {
		color: var(--color-text-accent);
	}
	.name {
		color: var(--color-text-strong);
	}
	.detail {
		color: var(--color-text-muted);
	}
</style>
