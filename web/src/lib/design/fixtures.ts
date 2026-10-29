// Sample data from the lighthouse example, for stories. The shapes are the
// same as the runtime output.

import type { Exploration, ExitInfo, RoomMatrix, FragmentView, FrameUse, MapView, RoomParts, RuleView, SourcePiece, ThingInfo, Turn, VerbInfo } from '#lib/engine/types.ts';
import type { MarkupToken, StoryStep, TranscriptEntry, WriterFragment, WriterTools } from './types.ts';

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

export const authorTurn: Turn = {
	ended: false,
	log: [{ kind: 'author', text: 'go to Lantern Room' }],
	blocks: [
		{ kind: 'author', runs: [{ text: 'go to Lantern Room' }] },
		{ kind: 'title', runs: [{ text: 'Lantern Room' }] }
	]
};

export const doorTurn: Turn = {
	ended: false,
	log: [{ kind: 'set', text: 'cottage_door.open = true' }],
	blocks: [
		{ kind: 'command', runs: [{ text: 'n' }] },
		{ kind: 'note', runs: [{ text: '(first opening the ' }, { text: 'cottage door', link: { kind: 'thing', id: 'cottage_door' } }, { text: ')' }] },
		{ kind: 'title', runs: [{ text: "Keeper's Cottage" }] }
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
	{ id: 'look', label: 'Look around', slots: 'none', preposition: null, bar: true },
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
		{ id: 'cottage_door', label: 'cottage door', text: 'There is a cottage door here.', detail: null, shown: false, why: 'not listed: it is the door of the north exit' },
		{ id: 'lighthouse_door', label: 'lighthouse door', text: 'There is a lighthouse door here.', detail: null, shown: false, why: 'not listed: it is the door of the east exit' }
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
		fired: true,
		trigger: 'verb',
		relevant: 'fired in the last turn'
	},
	{
		id: 'lamp_needs_matches',
		when: 'the player uses "turn on oil lamp"',
		condition: 'not matches.carried',
		condition_holds: true,
		then: ['say "You have nothing to light it with."'],
		once: false,
		instead: true,
		fired: false,
		trigger: 'verb',
		relevant: 'uses things in reach'
	},
	{
		id: 'dusk_falls',
		when: 'the clock reaches 21:00',
		condition: null,
		condition_holds: true,
		then: ['say "The last of the light drains out of the sky."'],
		once: false,
		instead: false,
		fired: false,
		trigger: 'time',
		relevant: 'fires in 30 minutes'
	},
	{
		id: 'first_climb',
		when: 'the player enters Lantern Room',
		condition: null,
		condition_holds: true,
		then: ['say "You are out of breath, and the wind leans on the glass."'],
		once: true,
		instead: false,
		fired: false,
		trigger: 'enter',
		relevant: null
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

/** The look frame after the second "Look around". */
export const lookFrame: FrameUse = {
	verb: 'look',
	scope: 'story',
	repeat: 2,
	count: 2,
	total: 2,
	parts: [
		{
			part: 'before',
			fragments: [
				{ index: 0, mode: 'always', condition: 'action.repeat == 1', text: 'You look around.', shown: false, why: 'action.repeat == 1 is false' },
				{ index: 1, mode: 'always', condition: 'action.repeat == 2', text: 'You look around, again.', shown: true, why: null },
				{ index: 2, mode: 'always', condition: 'action.repeat >= 3', text: 'You keep looking around, and nothing seems to change.', shown: false, why: 'action.repeat >= 3 is false' }
			]
		}
	]
};

export const exploration: Exploration = {
	states: 287,
	truncated: false,
	endings: [
		{
			text: "Far below, the *Marigold*'s masthead light turns, and steadies, and comes home.",
			path: ['go north', 'open drawer', 'take iron_key', 'take matches', 'take oil_lamp', 'go south', 'turn_on oil_lamp', 'unlock lighthouse_door iron_key', 'go east', 'go up', 'turn_on lens']
		}
	],
	stuck: 0,
	stuck_path: [],
	rooms: [
		{
			id: 'shore',
			name: 'The Shore',
			reached: true,
			variants: [
				{ text: 'Shingle crunches underfoot. … pale against the dusk sky. The sun is a red coal sinking into the sea.', states: 40, path: [], facts: ['lighthouse_door.locked = true'] },
				{ text: 'Shingle crunches underfoot. … black against the stars. Waves hiss on the stones.', states: 120, path: ['go north', 'go south', 'wait', 'wait'], facts: ['cottage_door.open = true', 'lighthouse_door.locked = true'] }
			],
			seen: []
		},
		{ id: 'vault', name: 'The Vault', reached: false, variants: [], seen: [] }
	],
	never_shown: ['room:vault#1: Cold stone and the smell of old coins.'],
	never_fired: ['open_vault']
};

// The cantina of the cantina example: the presence text of Blackbeard depends on what the player knows.
const bar = 'Low beams, smoke, a bar polished by elbows.';
const turns = ' Every head turns when you come in, then turns away again.';
const looms = 'A shadow looms in the corner of the cantina.';
const staring = 'The grim-looking man is still staring at you from the corner.';
const sitting = 'Blackbeard is still sitting in the corner, nodding at you as you enter.';

export const cantinaMatrix: RoomMatrix = {
	room: 'cantina',
	dimensions: [
		{ name: 'knows_blackbeard_name', values: ['false', 'true'] },
		{ name: 'first_visit', values: ['true', 'false'] },
		{ name: 'blackbeard.talked', values: ['0', '1'] }
	],
	combinations: 8,
	cells: [
		{ values: ['false', 'true', '0'], text: `${bar}${turns}\n${looms}`, warnings: [] },
		{ values: ['true', 'true', '0'], text: `${bar}${turns}\nBlackbeard looms in the corner of the cantina. ${sitting}`, warnings: ['2 presence texts of blackbeard show'] },
		{ values: ['false', 'false', '0'], text: `${bar}\n${looms}`, warnings: [] },
		{ values: ['true', 'false', '0'], text: `${bar}\nBlackbeard looms in the corner of the cantina. ${sitting}`, warnings: ['2 presence texts of blackbeard show'] },
		{ values: ['false', 'true', '1'], text: `${bar}${turns}\n${staring}`, warnings: [] },
		{ values: ['true', 'true', '1'], text: `${bar}${turns}\n${sitting}`, warnings: [] },
		{ values: ['false', 'false', '1'], text: `${bar}\n${staring}`, warnings: [] },
		{ values: ['true', 'false', '1'], text: `${bar}\n${sitting}`, warnings: [] }
	]
};

/** The full texts that the whole-game run saw in the cantina. */
export const cantinaSeen: string[] = cantinaMatrix.cells.filter((c) => !c.warnings.length).map((c) => c.text);

// A small tokenizer and checker in JavaScript, for stories. The app uses the runtime in WebAssembly.
function fakeTokens(text: string, condition: boolean): MarkupToken[] {
	const out: MarkupToken[] = [];
	const names = (from: number, src: string) => {
		for (const m of src.matchAll(/[A-Za-z_][\w.]*|==|!=|>=|<=|\d+/g)) {
			const kind = /^\d/.test(m[0]) ? 'literal' : /^[=!<>]/.test(m[0]) || ['and', 'or', 'not'].includes(m[0]) ? 'operator' : 'name';
			out.push({ from: from + m.index, to: from + m.index + m[0].length, kind });
		}
	};
	if (condition) {
		names(0, text);
		return out;
	}
	for (const m of text.matchAll(/\{([^{}]*)\}?/g)) {
		const start = m.index;
		out.push({ from: start, to: start + 1, kind: 'brace' });
		const inner = m[1];
		const sep = inner.search(/[:?]/);
		names(start + 1, sep < 0 ? inner : inner.slice(0, sep));
		if (sep >= 0) out.push({ from: start + 1 + sep, to: start + 2 + sep, kind: 'separator' });
		const bar = inner.indexOf('|');
		if (bar >= 0) out.push({ from: start + 1 + bar, to: start + 2 + bar, kind: 'separator' });
		if (m[0].endsWith('}')) out.push({ from: start + m[0].length - 1, to: start + m[0].length, kind: 'brace' });
	}
	for (const m of text.matchAll(/\*[^*]+\*/g)) out.push({ from: m.index, to: m.index + m[0].length, kind: 'emphasis' });
	return out.sort((a, b) => a.from - b.from);
}

export const writerTools: WriterTools = {
	symbols: [
		{ name: 'oil_lamp.lit', kind: 'property', detail: 'property of oil_lamp, a flag', value: 'false', values: [] },
		{ name: 'oil_lamp.the', kind: 'name', detail: 'the name with "the" of oil_lamp, for text', value: 'the oil lamp', values: [] },
		{ name: 'time.phase', kind: 'derived', detail: 'the phase of the day', value: 'dusk', values: ['night', 'dawn', 'day', 'dusk'] },
		{ name: 'night', kind: 'phase', detail: 'true when time.phase is night', value: 'false', values: [] },
		{ name: 'first', kind: 'markup', detail: 'in markup: {first: …} shows only on the first visit', value: null, values: [] }
	],
	textMatrix: (text) => ({
		room: 'shore',
		dimensions: [{ name: 'time.phase', values: ['night', 'dawn', 'day', 'dusk'] }],
		combinations: 4,
		cells: ['night', 'dawn', 'day', 'dusk'].map((p) => ({
			values: [p],
			text: text.replace(/\{night:\s*([^|}]*)\|\s*([^}]*)\}/g, p === 'night' ? '$1' : '$2').replace(/\{time\.phase\}/g, p).replace(/[{}]/g, ''),
			warnings: []
		}))
	}),
	tokenize: fakeTokens,
	tryMarkup: (text) => {
		const open = text.lastIndexOf('{');
		if (open > text.lastIndexOf('}')) return { problem: { from: open, to: text.length, message: '`{` is never closed' }, text: '', pieces: [] };
		const rendered = text.replace(/\{night:\s*([^|}]*)\|\s*([^}]*)\}/g, '$2').replace(/\{first:([^}]*)\}/g, '$1').replace(/\{([^}]*)\}/g, '$1');
		return { problem: null, text: rendered, pieces: [{ role: 'text', text: rendered, active: true }] };
	},
	tryCondition: (text) => (text.trim() ? { problem: null, holds: !text.includes('night') } : { problem: null, holds: null })
};

export const shoreFragment: WriterFragment = {
	variants: [
		'Shingle crunches underfoot. To the north a whitewashed cottage huddles against the wind{first: , its windows dark}. East, the lighthouse rises {night: black against the stars | pale against the {time.phase} sky}.'
	],
	when: null,
	mode: 'always',
	layer: 'base',
	replaces: false
};

export const waveFragment: WriterFragment = {
	variants: ['Waves hiss on the stones.', 'Somewhere out at sea a bell buoy clangs.'],
	when: 'time.phase == night',
	mode: 'shuffle',
	layer: 'ambient',
	replaces: false
};
