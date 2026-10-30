<script lang="ts">
	import type { MapView, RuleView } from '#lib/engine/types.ts';
	import PanelSection from '../molecules/PanelSection.svelte';
	import StateSwitches from '../molecules/StateSwitches.svelte';
	import MapPanel from './MapPanel.svelte';
	import type { RoomSwitch, StoryStep } from '../types.ts';

	interface Props {
		/** The rooms around the player, for the small map. Null: no small map, for example when the big map shows on the left. */
		map: MapView | null;
		/** The values that the room text depends on. */
		switches: RoomSwitch[];
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
		onpreview: (name: string, value: string) => void;
		/** Leave the preview: the room text is the game again. */
		onreset: () => void;
		/** Set the previewed values in the game for real. */
		onapply: () => void;
		onrule: (id: string) => void;
		/** Open a full view: map, story, rules or explore. */
		onopen: (view: 'map' | 'story' | 'rules' | 'explore') => void;
	}

	let { map, switches, texts = 0, preview = {}, rules, ruleCount, last = null, onpreview, onreset, onapply, onrule, onopen }: Props = $props();
</script>

<!--
	The Here panel in play mode: a playtest inspector for the current room. It has sections that fold, not tabs:
	the rooms around, the values that the room text depends on (with a preview), the rules that matter here,
	and what the last turn changed. The tools of the author are in the room desk of write mode.
	The whole world is one step away in the Atlas.
-->
<div class="here" aria-label="Here and now" role="region">
	{#if map}
		<PanelSection id="here-map" title="Around you">
			{#snippet actions()}<button type="button" class="link" onclick={() => onopen('map')}>Map ↗</button>{/snippet}
			<MapPanel {map} compact />
		</PanelSection>
	{/if}

	<PanelSection id="here-states" title="Depends on" meta={switches.length ? `${switches.length} values · ${texts} texts` : 'nothing'}>
		{#snippet actions()}<button type="button" class="link" onclick={() => onopen('explore')}>All texts ↗</button>{/snippet}
		<StateSwitches {switches} {preview} {onpreview} {onreset} {onapply} />
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

	<PanelSection id="here-last" title="What changed" meta={last ? `turn ${last.turn}` : undefined}>
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
		{:else}
			<p class="note">Nothing yet. Play a turn.</p>
		{/if}
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
</style>
