// Sample data from the lighthouse example, for stories. The shapes are the
// same as the runtime output.

import type { ExitInfo, FragmentView, MapView, RoomParts, RuleView, SourcePiece, ThingInfo, Turn, VerbInfo } from '#lib/engine/types.ts';
import type { StoryStep, TranscriptEntry } from './types.ts';

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

const t = (text: string, active = true): SourcePiece => ({ role: 'text', text, active });
const x = (text: string): SourcePiece => ({ role: 'syntax', text, active: true });

/** The shore base text on a second visit at dusk: `{first: ...}` and the night branch do not apply. */
export const shoreBasePieces: SourcePiece[] = [
	t('Shingle crunches underfoot. To the north a whitewashed cottage huddles against the wind'),
	x('{first: '),
	t(', its windows dark', false),
	x('}'),
	t('. East, the lighthouse rises '),
	x('{night: '),
	t('black against the stars', false),
	x(' | '),
	t('pale against the '),
	{ role: 'insert', text: '{time.phase}', active: true, value: 'dusk' },
	t(' sky'),
	x('}'),
	t('.')
];

export const fragments: FragmentView[] = [
	{
		index: 0,
		layer: 'base',
		mode: 'always',
		condition: null,
		replaces: false,
		text: 'Shingle crunches underfoot. To the north a whitewashed cottage huddles against the wind{first: , its windows dark}. East, the lighthouse rises {night: black against the stars | pale against the {time.phase} sky}.',
		variants: [shoreBasePieces],
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
		variants: [[t('The sun is a red coal sinking into the sea.')]],
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
		variants: [[t('Waves hiss on the stones.')], [t('Somewhere out at sea a bell buoy clangs.')]],
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
		variants: [[t('Far out, a single masthead light rocks on the swell.')]],
		shown: false,
		why: 'night and not beacon_lit is false'
	}
];

export const map: MapView = {
	rooms: [
		{ id: 'shore', name: 'The Shore', x: 0, y: 0, visited: true, current: false },
		{ id: 'cottage', name: "Keeper's Cottage", x: 0, y: -1, visited: true, current: true },
		{ id: 'stairwell', name: 'Lighthouse Stairwell', x: 1, y: 0, visited: false, current: false },
		{ id: 'lantern_room', name: 'Lantern Room', x: 1, y: -1, visited: false, current: false }
	],
	links: [
		{ from: 'shore', to: 'cottage', direction: 'north', visible: true, one_way: false },
		{ from: 'shore', to: 'stairwell', direction: 'east', visible: true, one_way: false },
		{ from: 'stairwell', to: 'lantern_room', direction: 'up', visible: true, one_way: false }
	]
};

export const parts: RoomParts = {
	objects: [
		{ id: 'cottage_door', label: 'cottage door', text: 'There is a cottage door here.', detail: null, shown: false, why: 'not listed: the room text describes it' },
		{ id: 'lighthouse_door', label: 'lighthouse door', text: 'There is a lighthouse door here.', detail: null, shown: false, why: 'not listed: the room text describes it' }
	],
	exits: [
		{ id: 'north', label: "north to Keeper's Cottage", text: 'north', detail: 'door: cottage door, closed', shown: true, why: null },
		{ id: 'east', label: 'east to Lighthouse Stairwell', text: 'east', detail: 'door: lighthouse door, locked', shown: true, why: null }
	]
};

export const rules: RuleView[] = [
	{
		id: 'find_iron_key',
		when: 'the player uses "open drawer"',
		condition: 'not drawer_searched',
		condition_holds: false,
		then: ['set drawer_searched = true', 'move iron key to cottage', 'say "Under a fold of oilcloth lies an iron key."'],
		once: false,
		instead: false,
		fired: true
	},
	{
		id: 'lamp_needs_matches',
		when: 'the player uses "turn on oil lamp"',
		condition: 'not matches.carried',
		condition_holds: true,
		then: ['say "You have nothing to light it with."'],
		once: false,
		instead: true,
		fired: false
	},
	{
		id: 'dusk_falls',
		when: 'the clock reaches 21:00',
		condition: null,
		condition_holds: true,
		then: ['say "The last of the light drains out of the sky."'],
		once: false,
		instead: false,
		fired: false
	}
];

export const history: StoryStep[] = [
	{ turn: 0, time: '20:30', phase: 'dusk', room: 'The Shore', command: '', rules: [], changes: [], ended: false },
	{ turn: 1, time: '20:35', phase: 'dusk', room: "Keeper's Cottage", command: 'n', rules: [], changes: ['cottage_door.open = true'], ended: false },
	{
		turn: 2,
		time: '20:40',
		phase: 'dusk',
		room: "Keeper's Cottage",
		command: 'open drawer',
		rules: ['find_iron_key'],
		changes: ['drawer.open = true', 'drawer_searched = true', 'iron_key -> cottage'],
		ended: false
	}
];
