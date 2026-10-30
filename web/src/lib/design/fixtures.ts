// Sample data from the lighthouse example, for stories. The shapes are the
// same as the runtime output.

import type { AtlasRoom, DialogueOutline, TalkView, Exploration, ExitInfo, RoomMatrix, FragmentView, FrameUse, MapView, RoomParts, RuleView, SourcePiece, ThingInfo, Turn, VerbInfo } from '#lib/engine/types.ts';
import type { MarkupToken, ReviewFinding, StoryStep, TranscriptEntry, WriterFragment, WriterTools } from './types.ts';

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
	{ id: 'go', label: 'Walk to', words: ['go', 'walk'], slots: 'one', preposition: null, bar: false },
	{ id: 'look', label: 'Look around', words: ['look', 'l'], slots: 'none', preposition: null, bar: true },
	{ id: 'examine', label: 'Look at', words: ['examine', 'x', 'look at'], slots: 'one', preposition: null, bar: true },
	{ id: 'take', label: 'Pick up', words: ['take', 'get', 'pick up'], slots: 'one', preposition: null, bar: true },
	{ id: 'use', label: 'Use', words: ['use'], slots: 'two', preposition: 'on', bar: true },
	{ id: 'open', label: 'Open', words: ['open'], slots: 'one', preposition: null, bar: true },
	{ id: 'close', label: 'Close', words: ['close', 'shut'], slots: 'one', preposition: null, bar: true },
	{ id: 'talk', label: 'Talk to', words: ['talk to', 'ask'], slots: 'one', preposition: null, bar: true },
	{ id: 'give', label: 'Give', words: ['give'], slots: 'two', preposition: 'to', bar: true },
	{ id: 'push', label: 'Push', words: ['push', 'press'], slots: 'one', preposition: null, bar: true },
	{ id: 'pull', label: 'Pull', words: ['pull'], slots: 'one', preposition: null, bar: true }
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
		{ id: 'shore', name: 'The Shore', x: 0, y: 0, level: 0, visited: true, current: false },
		{ id: 'cottage', name: "Keeper's Cottage", x: 0, y: -1, level: 0, visited: true, current: true },
		{ id: 'stairwell', name: 'Lighthouse Stairwell', x: 1, y: 0, level: 0, visited: false, current: false },
		{ id: 'lantern_room', name: 'Lantern Room', x: 1, y: -1, level: 1, visited: false, current: false }
	],
	links: [
		{ from: 'shore', to: 'cottage', direction: 'north', visible: true, one_way: false },
		{ from: 'shore', to: 'stairwell', direction: 'east', visible: true, one_way: false },
		{ from: 'stairwell', to: 'lantern_room', direction: 'up', visible: true, one_way: false }
	]
};

/** The lighthouse map with an AI proposal: a new room west of the shore, where a message exit was. */
export const proposalMap: MapView = {
	rooms: [
		{ id: 'shore', name: 'The Shore', x: 0, y: 0, level: 0, visited: true, current: true },
		{ id: 'cottage', name: "Keeper's Cottage", x: 0, y: -1, level: 0, visited: true, current: false },
		{ id: 'stairwell', name: 'Lighthouse Stairwell', x: 1, y: 0, level: 0, visited: false, current: false },
		{ id: 'sea_rocks', name: 'The Sea Rocks', x: -1, y: 0, level: 0, visited: false, current: false, proposed: 'p2' }
	],
	links: [
		{ from: 'shore', to: 'cottage', direction: 'north', visible: true, one_way: false },
		{ from: 'shore', to: 'stairwell', direction: 'east', visible: true, one_way: false },
		{ from: 'shore', to: 'sea_rocks', direction: 'west', visible: true, one_way: false, proposed: 'p2' }
	],
	ends: [
		{ room: 'shore', direction: 'west', visible: true, removed: 'p2' },
		{ room: 'sea_rocks', direction: 'south', visible: true, proposed: 'p2' },
		{ room: 'cottage', direction: 'north', visible: true }
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
	never_fired: ['open_vault'],
	threads: [
		{ thread: 'lost_key', paid_off: true, payoff_before_setup: ['go east'], payoffs: ['room:stairwell#2'] },
		{ thread: 'the_marigold', paid_off: false, payoff_before_setup: [], payoffs: ['rule:marigold_arrives'] }
	]
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

// The spiral tower example: three floors with a ring of four landings each. The west landing leads up to
// the north landing of the next floor.
const sides = [
	['north', 1, 0],
	['east', 2, 1],
	['south', 1, 2],
	['west', 0, 1]
] as const;
const floorNames = ['First', 'Second', 'Third'];
export const towerMap: MapView = {
	rooms: [
		...[1, 2, 3].flatMap((lv) =>
			sides.map(([side, x, y]) => ({
				id: `${side}_${lv}`,
				name: `${side[0].toUpperCase()}${side.slice(1)} Landing, ${floorNames[lv - 1]} Floor`,
				x,
				y,
				level: lv,
				visited: lv < 3,
				current: side === 'north' && lv === 2
			}))
		),
		{ id: 'bell_chamber', name: 'Bell Chamber', x: 1, y: 1, level: 4, visited: false, current: false }
	],
	links: [
		...[1, 2, 3].flatMap((lv) => [
			{ from: `north_${lv}`, to: `east_${lv}`, direction: 'southeast', visible: true, one_way: false },
			{ from: `east_${lv}`, to: `south_${lv}`, direction: 'southwest', visible: true, one_way: false },
			{ from: `south_${lv}`, to: `west_${lv}`, direction: 'northwest', visible: true, one_way: false },
			{ from: `west_${lv}`, to: `north_${lv}`, direction: 'northeast', visible: true, one_way: false },
			{ from: `west_${lv}`, to: lv < 3 ? `north_${lv + 1}` : 'bell_chamber', direction: 'up', visible: true, one_way: false }
		])
	]
};

/** The rooms of the spiral tower for the Atlas: what is in each room now. */
export const towerRooms: AtlasRoom[] = towerMap.rooms.map((r) => ({
	id: r.id,
	name: r.name,
	level: r.level,
	visited: r.visited,
	current: r.current,
	things: r.id === 'bell_chamber' ? [{ id: 'bell', name: 'bronze bell' }] : [],
	exits: towerMap.links.filter((l) => l.from === r.id).map((l) => [l.direction, l.to] as [string, string]),
	fragments: 1
}));

/** The findings of an AI review of the shore. */
export const findings: ReviewFinding[] = [
	{
		key: 'j1#0',
		severity: 'high',
		owner: 'room:shore',
		fragment: 0,
		problem: '"huddles against the wind" and "black against the stars" both lean on wind and dark.',
		fix: 'Cut "against the wind".'
	},
	{ key: 'j1#1', severity: 'medium', owner: 'room:shore', fragment: 2, problem: 'The bell buoy line has no sound word.', fix: 'Add "dull" before "clangs".' },
	{ key: 'j1#2', severity: 'low', owner: 'room:shore', fragment: null, problem: 'The room has no smell.', fix: 'Add salt and weed.' }
];

/** The dialogue of Blackbeard in the cantina example, as an outline. */
export const blackbeardOutline: DialogueOutline = {
	id: 'blackbeard',
	with: 'blackbeard',
	start: ['again', 'meet'],
	nodes: [
		{
			key: 'meet',
			start: 1,
			say: 'The figure looks up. It is a man, grim-looking, with eyes like nails. "Sit, if you like."',
			outcomes: [],
			unreachable: false,
			options: [
				{ say: '"Who are you?"', go: 'names', missing: false, outcomes: [] },
				{ say: '"I am looking for the *Marigold*."', go: 'marigold', missing: false, outcomes: [] },
				{ say: '"Not tonight."', go: 'end', missing: false, outcomes: [] }
			]
		},
		{
			key: 'names',
			say: '"They call me Blackbeard," he says. "And you\'re looking for a ship."',
			outcomes: ['set knows_blackbeard_name = true', 'trust +1'],
			unreachable: false,
			options: [
				{ say: '"The *Marigold*. Where did she go?"', go: 'marigold', missing: false, outcomes: [] },
				{ say: '"Just passing through."', go: 'end', missing: false, outcomes: ['trust -1'] }
			]
		},
		{
			key: 'marigold',
			say: '{blackbeard.trust >= 1 ? "South, past the breakwater," he says quietly. | He shrugs. "Why should I tell a stranger?"}',
			outcomes: ['remember asked_marigold'],
			unreachable: false,
			options: [
				{ say: '"Thank you."', when: 'blackbeard.trust >= 1', go: 'end', missing: false, outcomes: ['trust +1', 'say a line', 'go to street'] },
				{ say: '"Who are you, anyway?"', when: 'not knows_blackbeard_name', go: 'names', missing: false, outcomes: [] },
				{ say: '"Never mind."', go: 'end', missing: false, outcomes: [] }
			]
		},
		{
			key: 'again',
			start: 0,
			when: 'blackbeard.talked > 1',
			say: '{blackbeard.The} looks up from his drink. {remembers(blackbeard, asked_marigold) ? "Still after that ship?" | "Back again."}',
			outcomes: [],
			unreachable: false,
			options: [
				{ say: '"Who are you?"', when: 'not knows_blackbeard_name', go: 'names', missing: false, outcomes: [] },
				{ say: '"Tell me about the *Marigold*."', go: 'marigold', missing: false, outcomes: [] },
				{ say: '"Nothing. Goodbye."', go: 'end', missing: false, outcomes: [] }
			]
		}
	]
};

/** The same dialogue with mistakes: a misspelled target and a node that no start reaches. */
export const brokenOutline: DialogueOutline = {
	...blackbeardOutline,
	nodes: [
		{
			...blackbeardOutline.nodes[0],
			options: [{ ...blackbeardOutline.nodes[0].options[0], go: 'nmes', missing: true }, ...blackbeardOutline.nodes[0].options.slice(1)]
		},
		...blackbeardOutline.nodes.slice(1),
		{ key: 'rumours', say: '"The harbour master drinks here on Fridays."', outcomes: [], unreachable: true, options: [] }
	]
};

/** The talk with Blackbeard after "Who are you?": the line, the outcomes, and one hidden answer. */
export const blackbeardTalk: TalkView = {
	dialogue: 'blackbeard',
	with: 'blackbeard',
	name: 'Blackbeard',
	node: 'marigold',
	say: blackbeardOutline.nodes[2].say,
	outcomes: ['remember asked_marigold'],
	options: [
		{ choice: null, say: '"Thank you."', when: 'blackbeard.trust >= 1', go: 'end', outcomes: ['trust +1', 'say a line', 'go to street'] },
		{ choice: 0, say: '"Who are you, anyway?"', when: 'not knows_blackbeard_name', go: 'names', outcomes: [] },
		{ choice: 1, say: '"Never mind."', go: 'end', outcomes: [] }
	],
	relations: [['trust', 0]],
	memories: ['asked_marigold']
};

/** A talk with Blackbeard: the player chose "Who are you?". */
export const talkTurns: Turn[] = [
	{
		blocks: [
			{ kind: 'command', runs: [{ text: 'talk to shadow' }] },
			{ kind: 'speech', runs: [{ text: 'The figure looks up. It is a man, grim-looking, with eyes like nails. "Sit, if you like."' }] }
		],
		ended: false,
		log: []
	},
	{
		blocks: [
			{ kind: 'said', runs: [{ text: '"Who are you?"' }] },
			{ kind: 'speech', runs: [{ text: '"They call me Blackbeard," he says. "And you\'re looking for a ship."' }] }
		],
		ended: false,
		log: [],
		talk: { dialogue: 'blackbeard', with: 'blackbeard', name: 'Blackbeard', node: 'names' },
		choices: [
			{ index: 0, text: '"The *Marigold*. Where did she go?"' },
			{ index: 1, text: '"Just passing through."' }
		]
	}
];
