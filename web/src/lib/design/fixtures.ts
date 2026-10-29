// Sample data from the lighthouse example, for stories. The shapes are the
// same as the runtime output.

import type { ExitInfo, FragmentView, ThingInfo, Turn, VerbInfo } from '#lib/engine/types.ts';
import type { TranscriptEntry } from './types.ts';

const shore = (fragment: number, text: string, condition?: string) => ({
	text,
	source: { owner: 'room:shore', fragment, layer: fragment === 0 ? 'base' : 'ambient', condition, variant: 0 } as const
});

export const openingTurn: Turn = {
	ended: false,
	log: [],
	blocks: [
		{
			kind: 'narration',
			runs: [
				{
					text: 'The ferry left you on the shingle an hour ago. Old Hal has not lit the lighthouse in three nights, and the *Marigold* is due in after dark.'
				}
			]
		},
		{ kind: 'title', runs: [{ text: 'The Shore' }] },
		{
			kind: 'prose',
			runs: [
				shore(0, 'Shingle crunches underfoot. To the north a whitewashed cottage huddles against the wind, its windows dark. East, the lighthouse rises pale against the dusk sky.'),
				{ text: ' ' },
				shore(1, 'The sun is a red coal sinking into the sea.', 'time.phase == dusk')
			]
		},
		{
			kind: 'exits',
			runs: [
				{ text: 'Exits: ' },
				{ text: 'north', link: { kind: 'exit', id: 'north' } },
				{ text: ', ' },
				{ text: 'east', link: { kind: 'exit', id: 'east' } },
				{ text: '.' }
			]
		}
	]
};

export const drawerTurn: Turn = {
	ended: false,
	log: [{ kind: 'rule', text: 'find_iron_key' }],
	blocks: [
		{ kind: 'command', runs: [{ text: 'open drawer' }] },
		{
			kind: 'response',
			runs: [{ text: 'You open the ' }, { text: 'drawer', link: { kind: 'thing', id: 'drawer' } }, { text: '.' }]
		},
		{
			kind: 'narration',
			runs: [
				{ text: 'Under a fold of oilcloth lies an ' },
				{ text: 'iron key', link: { kind: 'thing', id: 'iron_key' } },
				{ text: '.' }
			]
		}
	]
};

export const endTurn: Turn = {
	ended: true,
	log: [],
	blocks: [
		{ kind: 'command', runs: [{ text: 'light beacon' }] },
		{
			kind: 'narration',
			runs: [
				{
					text: "You touch the lamp's flame to the wick. It catches, steadies, and then the lens takes it and throws it out across the water."
				}
			]
		},
		{
			kind: 'end',
			runs: [{ text: "Far below, the *Marigold*'s masthead light turns, and steadies, and comes home." }]
		}
	]
};

export const entries: TranscriptEntry[] = [
	{ id: 0, turn: openingTurn },
	{ id: 1, turn: drawerTurn }
];

export const verbs: VerbInfo[] = [
	{ id: 'go', label: 'Walk to', slots: 'one', preposition: null, bar: false },
	{ id: 'examine', label: 'Look at', slots: 'one', preposition: null, bar: true },
	{ id: 'take', label: 'Pick up', slots: 'one', preposition: null, bar: true },
	{ id: 'use', label: 'Use', slots: 'two', preposition: 'on', bar: true },
	{ id: 'open', label: 'Open', slots: 'one', preposition: null, bar: true },
	{ id: 'close', label: 'Close', slots: 'one', preposition: null, bar: true },
	{ id: 'talk', label: 'Talk to', slots: 'one', preposition: null, bar: true },
	{ id: 'give', label: 'Give', slots: 'two', preposition: 'to', bar: true },
	{ id: 'push', label: 'Push', slots: 'one', preposition: null, bar: true },
	{ id: 'pull', label: 'Pull', slots: 'one', preposition: null, bar: true }
];

export const things: ThingInfo[] = [
	{ id: 'drawer', name: 'drawer' },
	{ id: 'oil_lamp', name: 'oil lamp' },
	{ id: 'matches', name: 'box of matches' },
	{ id: 'cottage_door', name: 'cottage door' }
];

export const inventory: ThingInfo[] = [{ id: 'iron_key', name: 'iron key' }];

export const exits: ExitInfo[] = [
	{ direction: 'north', to: 'cottage', to_name: "Keeper's Cottage" },
	{ direction: 'east', to: 'stairwell', to_name: 'Lighthouse Stairwell' }
];

export const fragments: FragmentView[] = [
	{
		index: 0,
		layer: 'base',
		mode: 'always',
		condition: null,
		replaces: false,
		text: 'Shingle crunches underfoot. To the north a whitewashed cottage huddles against the wind{first: , its windows dark}. East, the lighthouse rises {night: black against the stars | pale against the {time.phase} sky}.',
		shown: true,
		why: null
	},
	{
		index: 1,
		layer: 'ambient',
		mode: 'always',
		condition: 'time.phase == dusk',
		replaces: false,
		text: 'The sun is a red coal sinking into the sea.',
		shown: true,
		why: null
	},
	{
		index: 2,
		layer: 'ambient',
		mode: 'shuffle',
		condition: 'time.phase == night',
		replaces: false,
		text: 'Waves hiss on the stones. / Somewhere out at sea a bell buoy clangs.',
		shown: false,
		why: 'time.phase == night is false'
	},
	{
		index: 3,
		layer: 'event',
		mode: 'always',
		condition: 'night and not beacon_lit',
		replaces: false,
		text: 'Far out, a single masthead light rocks on the swell.',
		shown: false,
		why: 'night and not beacon_lit is false'
	}
];
