<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { blackbeardTalk, history, map, rules } from '../fixtures.ts';
	import HerePanel from './HerePanel.svelte';

	const switches = [
		{ name: 'beacon_lit', value: 'false', options: ['false', 'true'] },
		{ name: 'time.phase', value: 'dusk', options: ['night', 'dawn', 'day', 'dusk'] },
		{ name: 'first_visit', value: 'true', options: ['true', 'false'] }
	];

	const { Story } = defineMeta({
		title: 'Organisms/HerePanel',
		component: HerePanel,
		tags: ['autodocs'],
		args: {
			map,
			switches,
			texts: 10,
			preview: {},
			rules: rules.filter((r) => r.relevant),
			ruleCount: rules.length,
			last: history.at(-1),
			onpreview: fn(),
			onreset: fn(),
			onapply: fn(),
			onrule: fn(),
			onopen: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `The Here panel is the side panel in play mode. It is a playtest inspector for the current room. It answers one question: what is true here, now? It has sections that fold (PanelSection), not tabs. The folded state stays across visits.

During a talk, a **Talk** section comes first (TalkInspector): the current node, the answers with the hidden ones and their false conditions, and the relations and memories of the character.

1. **Around you**: a small map of the current room and its neighbours on the same level. "Map ↗" opens the full map. Set \`map\` to null when the big map shows on the left (ultrawide). Then this section does not show.
2. **Depends on**: the values that the room text depends on, as switches (StateSwitches). A change shows a **preview** of the room text in place: the game does not move. The status line shows "PREVIEW · …", and a live region tells screen readers. "Make it so" sets the values in the game for real; "Reset" ends the preview. A turn or another room also ends it. "All texts ↗" opens the explorer.
3. **Rules here**: the rules that matter here and now, with the reason. A click shows the rule in the rules view. "All ↗" opens all rules.
4. **What changed**: the command of the last turn, the rules that fired and the changes. "Story ↗" opens the timeline.

The panel has no tools of the author. Notes, threads and AI help are in the room desk (RoomDesk) in write mode. The things and the exits are in the room text and in the verb bar.

The panel is a region with the name "Here and now". Each section has a heading with a button that folds it.`
				}
			}
		}
	});
</script>

<Story name="Here and now" />
<Story name="Next to the big map" args={{ map: null }} />
<Story name="Preview" args={{ preview: { 'time.phase': 'night' } }} />
<Story name="A room without values" args={{ switches: [], texts: 1, rules: [] }} />
<Story name="In a talk" args={{ talk: blackbeardTalk }} />
