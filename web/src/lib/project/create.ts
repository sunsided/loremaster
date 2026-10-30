// New rooms, exits, things and rules, as YAML text in the style of the hand-written files.

import { isSeq, parseDocument, YAMLMap, YAMLSeq } from 'yaml';
import { keepLines } from './yaml-edit.ts';

export const directions = [
	'north',
	'northeast',
	'east',
	'southeast',
	'south',
	'southwest',
	'west',
	'northwest',
	'up',
	'down',
	'in',
	'out'
] as const;
export type Direction = (typeof directions)[number];

const shorts: Record<string, Direction> = {
	n: 'north',
	ne: 'northeast',
	e: 'east',
	se: 'southeast',
	s: 'south',
	sw: 'southwest',
	w: 'west',
	nw: 'northwest',
	u: 'up',
	d: 'down'
};

/** The direction that a word names, for example `n` or `north`. */
export function directionOf(word: string): Direction | null {
	const w = word.trim().toLowerCase().replace(/^(go|walk|walk to)\s+/, '');
	if ((directions as readonly string[]).includes(w)) return w as Direction;
	return shorts[w] ?? null;
}

const opposites: Record<Direction, Direction> = {
	north: 'south',
	northeast: 'southwest',
	east: 'west',
	southeast: 'northwest',
	south: 'north',
	southwest: 'northeast',
	west: 'east',
	northwest: 'southeast',
	up: 'down',
	down: 'up',
	in: 'out',
	out: 'in'
};

export const opposite = (d: Direction) => opposites[d];

/** The map step for a direction: north is up on the map (y − 1). Up and down change the level instead. */
export function step(d: Direction): [number, number] {
	const x = d.includes('east') ? 1 : d.includes('west') ? -1 : 0;
	const y = d.startsWith('north') ? -1 : d.startsWith('south') ? 1 : 0;
	if (d === 'in') return [1, 1];
	if (d === 'out') return [-1, -1];
	return [x, y];
}

/** An id from a name: "Brass Key" becomes `brass_key`. A number is added when the id is taken. */
export function slug(name: string, taken: Set<string>): string {
	const base =
		name
			.toLowerCase()
			.normalize('NFKD')
			.replace(/[̀-ͯ]/g, '')
			.replace(/[^a-z0-9]+/g, '_')
			.replace(/^_+|_+$/g, '') || 'new';
	const start = /^[a-z_]/.test(base) ? base : `x_${base}`;
	let id = start;
	for (let n = 2; taken.has(id); n++) id = `${start}_${n}`;
	return id;
}

function flow(entries: Record<string, unknown>): YAMLMap {
	const map = new YAMLMap();
	for (const [k, v] of Object.entries(entries)) map.set(k, v);
	map.flow = true;
	return map;
}

/** A new room file. The room has no text yet; the author writes it in the game. */
export function roomFile(o: { name: string; position: [number, number]; level?: number; exits: { direction: Direction; to: string }[] }): string {
	const lines = [`name: ${JSON.stringify(o.name)}`, `position: [${o.position[0]}, ${o.position[1]}]`];
	if (o.level) lines.push(`level: ${o.level}`);
	lines.push('description: []');
	if (o.exits.length) {
		lines.push('exits:');
		for (const e of o.exits) lines.push(`  - { direction: ${e.direction}, to: ${e.to} }`);
	}
	return `${lines.join('\n')}\n`;
}

/** Add an exit to a room file. A message exit (no `to`) in that direction becomes the new exit. */
export function addExit(text: string, direction: Direction, to: string): string {
	const doc = parseDocument(text);
	let exits = doc.get('exits', true);
	if (!isSeq(exits)) {
		exits = new YAMLSeq();
		doc.set('exits', exits);
	}
	const message = (exits as YAMLSeq).items.find((e) => e instanceof YAMLMap && e.get('direction') === direction && !e.has('to'));
	if (message instanceof YAMLMap) {
		message.delete('blocked');
		message.set('to', to);
	} else {
		(exits as YAMLSeq).add(flow({ direction, to }));
	}
	return keepLines(text, doc.toString({ lineWidth: 0, flowCollectionPadding: true }));
}

/** A new thing file, in a room. */
export function thingFile(o: { name: string; room: string; proper?: boolean; portable?: boolean; listed?: boolean; props?: Record<string, boolean | number | string> }): string {
	const lines = [`name: ${JSON.stringify(o.name)}`, `location: { room: ${o.room} }`];
	if (o.portable) lines.push('portable: true');
	if (o.proper) lines.push('proper: true');
	// A thing that the room text names is not listed again as "There is … here."
	if (o.listed === false) lines.push('listed: false');
	const props = Object.entries(o.props ?? {});
	if (props.length) lines.push(`props: { ${props.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(', ')} }`);
	return `${lines.join('\n')}\n`;
}

/** A plain YAML scalar when it is safe, else a double-quoted one. */
const scalar = (text: string) => (/^[\w][\w .=<>!()]*$/.test(text) && !/: |#/.test(text) ? text : JSON.stringify(text));

/** Add a rule that answers a verb on a thing with a text, instead of the default response. */
export function addResponseRule(text: string | null, o: { id: string; verb: string; thing: string | null; when: string | null; say: string }): string {
	const rule = [
		`- id: ${o.id}`,
		`  on: { verb: [${o.thing ? `${o.verb}, ${o.thing}` : o.verb}] }`,
		...(o.when ? [`  when: ${scalar(o.when)}`] : []),
		'  instead: true',
		'  then:',
		`    - say: ${JSON.stringify(o.say)}`
	].join('\n');
	const old = (text ?? '').trimEnd();
	if (!old) return `${rule}\n`;
	// A file with one rule becomes a list of rules.
	const list = old.trimStart().startsWith('-')
		? old
		: old
				.split('\n')
				.map((line, i) => (i === 0 ? `- ${line}` : line ? `  ${line}` : line))
				.join('\n');
	return `${list}\n\n${rule}\n`;
}
