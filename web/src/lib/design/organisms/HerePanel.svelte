<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ExitInfo, MapView, RuleView, ThingInfo } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';
	import ProseLink from '../atoms/ProseLink.svelte';
	import StateSwitch from '../atoms/StateSwitch.svelte';
	import PanelSection from '../molecules/PanelSection.svelte';
	import MapPanel from './MapPanel.svelte';
	import type { StoryStep } from '../types.ts';

	/** A value that the room text depends on. */
	interface Switch {
		name: string;
		/** The value in the game now. */
		value: string;
		options: string[];
	}

	interface Props {
		/** The rooms around the player, for the small map. */
		map: MapView;
		/** The values that the room text depends on. */
		switches: Switch[];
		/** The number of different texts of the room, for the section title. */
		texts?: number;
		/** The values that the preview shows instead of the game, by name. Empty: no preview. */
		preview?: Record<string, string>;
		/** The rules that matter here and now. */
		rules: RuleView[];
		/** The number of all rules, for the section title. */
		ruleCount: number;
		/** The last turn. */
		last?: StoryStep | null;
		things: ThingInfo[];
		exits: ExitInfo[];
		onpreview: (name: string, value: string) => void;
		/** Leave the preview: the room text is the game again. */
		onreset: () => void;
		/** Set the previewed values in the game for real. */
		onapply: () => void;
		onrule: (id: string) => void;
		/** Open a full view: map, story, rules or explore. */
		onopen: (view: 'map' | 'story' | 'rules' | 'explore') => void;
		onthing: (id: string, name: string) => void;
		onexit: (direction: string) => void;
		/** The AI section, while the AI server runs. It shows first. */
		ai?: Snippet;
	}

	let {
		map,
		switches,
		texts = 0,
		preview = {},
		rules,
		ruleCount,
		last = null,
		things,
		exits,
		onpreview,
		onreset,
		onapply,
		onrule,
		onopen,
		onthing,
		onexit,
		ai
	}: Props = $props();

	const previewing = $derived(Object.keys(preview).length > 0);
	const announce = $derived(
		previewing ? `Preview: ${Object.entries(preview).map(([n, v]) => `${n} ${v}`).join(', ')}. The game did not move.` : ''
	);
</script>

<!--
	The Here panel: what is true here and now. It has sections that fold, not tabs:
	the rooms around, the values that the room text depends on (with a preview), the rules that matter here,
	the last turn, and the things and exits. The whole world is one step away in the full views.
-->
<div class="here" aria-label="Here and now" role="region">
	{#if ai}
		<PanelSection id="here-ai" title="✦ AI">{@render ai()}</PanelSection>
	{/if}
	<PanelSection id="here-map" title="Around you">
		{#snippet actions()}<button type="button" class="link" onclick={() => onopen('map')}>Map ↗</button>{/snippet}
		<MapPanel {map} compact />
	</PanelSection>

	<PanelSection id="here-states" title="Depends on" meta={switches.length ? `${switches.length} values · ${texts} texts` : 'nothing'}>
		{#snippet actions()}<button type="button" class="link" onclick={() => onopen('explore')}>All texts ↗</button>{/snippet}
		{#if switches.length}
			<div class="switches">
				{#each switches as sw (sw.name)}
					<StateSwitch
						name={sw.name}
						value={preview[sw.name] ?? sw.value}
						options={sw.options.includes(sw.value) ? sw.options : [sw.value, ...sw.options]}
						changed={sw.name in preview}
						onchange={(v) => onpreview(sw.name, v)}
					/>
				{/each}
			</div>
			{#if previewing}
				<div class="preview-actions">
					<Button size="compact" variant="primary" onclick={onapply}>Make it so</Button>
					<Button size="compact" onclick={onreset}>Reset</Button>
				</div>
			{:else}
				<p class="note">Change a value to preview the room text in place. The game does not move.</p>
			{/if}
		{:else}
			<p class="note">The text of this room does not depend on a value.</p>
		{/if}
		<p class="visually-hidden" aria-live="polite">{announce}</p>
	</PanelSection>

	<PanelSection id="here-rules" title="Rules here" meta="{rules.length} of {ruleCount}">
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

	<PanelSection id="here-last" title="Last turn" meta={last ? `turn ${last.turn}` : undefined}>
		{#snippet actions()}<button type="button" class="link" onclick={() => onopen('story')}>Story ↗</button>{/snippet}
		{#if last}
			<dl class="facts">
				<dt>command</dt>
				<dd>{last.command || '(the opening)'}</dd>
				<dt>rules fired</dt>
				<dd>{last.rules.length ? last.rules.join(', ') : 'none'}</dd>
				<dt>changes</dt>
				<dd>{last.changes.length ? last.changes.join(', ') : 'none'}</dd>
			</dl>
		{/if}
	</PanelSection>

	<PanelSection id="here-reach" title="Things and exits" meta="{things.length} · {exits.length}">
		<ul class="list">
			{#each things as t (t.id)}
				<li class="row-static">
					<ProseLink inspect="thing:{t.id}" onclick={() => onthing(t.id, t.name)}>{t.name}</ProseLink>
				</li>
			{/each}
			{#each exits as e (e.direction)}
				<li class="row-static">
					<ProseLink kind="exit" inspect="exit:{e.direction}" onclick={() => onexit(e.direction)}>{e.direction}</ProseLink>
					<span class="detail">to {e.to_name}</span>
				</li>
			{/each}
			{#if !things.length && !exits.length}<li class="note">Nothing in reach, and no exits.</li>{/if}
		</ul>
	</PanelSection>
</div>

<style>
	.here {
		display: flex;
		flex-direction: column;
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
	.row:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.switches {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.preview-actions {
		display: flex;
		gap: var(--space-3);
		margin-top: var(--space-3);
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
	.row-static {
		display: flex;
		flex-wrap: wrap;
		gap: 0 var(--space-4);
		align-items: baseline;
		min-height: var(--size-target-compact);
		padding: var(--space-1) 0;
	}
	.name {
		color: var(--color-text-strong);
	}
	.detail {
		color: var(--color-text-muted);
	}
	.facts {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: var(--space-1) var(--space-4);
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
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
