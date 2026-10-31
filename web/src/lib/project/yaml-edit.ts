// Edits of fragments in YAML text. The edits change the document in place, so comments,
// key order and quoting of the rest of the file stay as they are.

import { isMap, isScalar, isSeq, parseDocument, Scalar, YAMLMap, YAMLSeq, type Document } from 'yaml';
import type { YamlPath } from './source-map.ts';

export type Layer = 'base' | 'ambient' | 'event' | 'objects' | 'characters' | 'exits';
export type Mode = 'always' | 'first_visit' | 'once' | 'cycle' | 'shuffle' | 'sticky';

/** A fragment as the editor sees it. */
export interface FragmentData {
	variants: string[];
	when: string | null;
	mode: Mode;
	layer: Layer;
	replaces: boolean;
}

export const emptyFragment = (layer: Layer = 'base'): FragmentData => ({
	variants: [''],
	when: null,
	mode: 'always',
	layer,
	replaces: false
});

/** Thrown when the YAML text has errors, or a path does not lead to a fragment. */
export class EditError extends Error {}

/** The parsed document, with its original text to keep unchanged lines byte for byte. */
const originals = new WeakMap<Document, string>();

function parse(text: string): Document {
	const doc = parseDocument(text);
	if (doc.errors.length) throw new EditError(doc.errors[0].message);
	originals.set(doc, text);
	return doc;
}

function serialize(doc: Document): string {
	const out = doc.toString({ lineWidth: 0, flowCollectionPadding: false });
	const original = originals.get(doc);
	return original === undefined ? out : keepLines(original, out);
}

/** Spaces inside flow brackets are the only formatting that the serializer changes on unchanged lines. */
const normal = (line: string) => line.replace(/([[{]) +/g, '$1').replace(/ +([\]}])/g, '$1');

/**
 * Put back the original form of each line that the edit did not change.
 * The lines are matched by a longest common subsequence of their normal forms.
 */
export function keepLines(original: string, output: string): string {
	const a = original.split('\n');
	const b = output.split('\n');
	const na = a.map(normal);
	const nb = b.map(normal);
	const n = a.length;
	const m = b.length;
	const lcs: number[][] = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0));
	for (let i = n - 1; i >= 0; i--)
		for (let j = m - 1; j >= 0; j--)
			lcs[i][j] = na[i] === nb[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
	const result = [...b];
	let i = 0;
	let j = 0;
	while (i < n && j < m) {
		if (na[i] === nb[j]) {
			result[j] = a[i];
			i++;
			j++;
		} else if (lcs[i + 1][j] >= lcs[i][j + 1]) i++;
		else j++;
	}
	return result.join('\n');
}

/** The fragment at a list index, as plain data. */
export function readFragment(text: string, list: YamlPath, index: number): FragmentData {
	const node = parse(text).getIn([...list, index], true);
	if (isScalar(node)) return { ...emptyFragment(), variants: [String(node.value ?? '')] };
	if (!isMap(node)) throw new EditError(`No fragment at ${[...list, index].join('.')}.`);
	const get = (key: string) => {
		const v = node.get(key, true);
		return isScalar(v) ? v.value : v;
	};
	const variants: string[] = [];
	const first = get('text');
	if (first != null) variants.push(String(first));
	const more = node.get('variants', true);
	if (isSeq(more)) for (const v of more.items) variants.push(String(isScalar(v) ? v.value : v));
	return {
		variants: variants.length ? variants : [''],
		when: get('when') != null ? String(get('when')) : null,
		mode: (get('mode') as Mode) ?? 'always',
		layer: (get('layer') as Layer) ?? 'base',
		replaces: get('replaces') === true
	};
}

/** Set the text of one variant. A plain-string fragment stays a plain string. */
export function writeVariant(text: string, list: YamlPath, index: number, variant: number, value: string): string {
	const doc = parse(text);
	const path = [...list, index];
	const node = doc.getIn(path, true);
	if (isScalar(node) && variant === 0) {
		setScalar(node, value);
		return serialize(doc);
	}
	if (!isMap(node)) throw new EditError(`No fragment at ${path.join('.')}.`);
	const hasText = node.has('text');
	if (hasText && variant === 0) {
		setValue(doc, node, 'text', value);
	} else {
		const seq = node.get('variants', true);
		const i = hasText ? variant - 1 : variant;
		if (!isSeq(seq) || i < 0 || i >= seq.items.length) throw new EditError(`No variant ${variant} at ${path.join('.')}.`);
		const item = seq.items[i];
		if (isScalar(item)) setScalar(item, value);
		else seq.set(i, doc.createNode(value));
	}
	return serialize(doc);
}

/** Set or remove the condition of a fragment. A plain string becomes a map when it gets a condition. */
export function writeWhen(text: string, list: YamlPath, index: number, when: string | null): string {
	const doc = parse(text);
	const map = asMap(doc, list, index);
	if (when === null || when.trim() === '') map.delete('when');
	else setValue(doc, map, 'when', when.trim());
	return serialize(doc);
}

/**
 * Write all fields of a fragment. When only the texts or the condition changed, only those values change in
 * the file, so the quoting and the key order stay. Else the fragment is written again, without default fields.
 */
export function writeFragment(text: string, list: YamlPath, index: number, data: FragmentData): string {
	const old = readFragment(text, list, index);
	if (old.layer === data.layer && old.mode === data.mode && old.replaces === data.replaces && old.variants.length === data.variants.length) {
		let out = text;
		data.variants.forEach((v, i) => {
			if (v !== old.variants[i]) out = writeVariant(out, list, index, i, v);
		});
		if ((data.when ?? '') !== (old.when ?? '')) out = writeWhen(out, list, index, data.when);
		return out;
	}
	const doc = parse(text);
	const seq = doc.getIn(list, true);
	if (!isSeq(seq)) throw new EditError(`No list at ${list.join('.')}.`);
	seq.set(index, toNode(doc, data));
	return serialize(doc);
}

/** Insert a fragment into a list. The list is made when it does not exist. */
export function insertFragment(text: string, list: YamlPath, index: number, data: FragmentData): string {
	const doc = parse(text);
	let seq = doc.getIn(list, true);
	if (seq == null) {
		doc.setIn(list, doc.createNode([]));
		seq = doc.getIn(list, true);
	}
	if (!isSeq(seq)) throw new EditError(`No list at ${list.join('.')}.`);
	// `description: []` becomes a block list with its first fragment.
	if (seq.items.length === 0) seq.flow = false;
	seq.items.splice(Math.min(index, seq.items.length), 0, toNode(doc, data));
	return serialize(doc);
}

/** Remove a fragment from a list. */
export function removeFragment(text: string, list: YamlPath, index: number): string {
	const doc = parse(text);
	const seq = doc.getIn(list, true);
	if (!isSeq(seq) || index >= seq.items.length) throw new EditError(`No fragment at ${[...list, index].join('.')}.`);
	seq.items.splice(index, 1);
	return serialize(doc);
}

function toNode(doc: Document, data: FragmentData) {
	const plain =
		data.when === null && data.mode === 'always' && data.layer === 'base' && !data.replaces && data.variants.length === 1;
	if (plain) return quoted(data.variants[0]);
	const map = new YAMLMap();
	if (data.layer !== 'base') map.set('layer', data.layer);
	if (data.when) map.set('when', data.when);
	if (data.mode !== 'always') map.set('mode', data.mode);
	if (data.replaces) map.set('replaces', true);
	if (data.variants.length === 1) map.set('text', quoted(data.variants[0]));
	else {
		const seq = new YAMLSeq();
		for (const v of data.variants) seq.add(quoted(v));
		map.set('variants', seq);
	}
	return map;
}

/** Story text in double quotes, like the hand-written files. */
function quoted(text: string): Scalar {
	const node = new Scalar(text);
	node.type = Scalar.QUOTE_DOUBLE;
	return node;
}

/** The fragment as a map. A plain string becomes `{ text: … }` first, with its quoting kept. */
function asMap(doc: Document, list: YamlPath, index: number): YAMLMap {
	const path = [...list, index];
	const node = doc.getIn(path, true);
	if (isMap(node)) return node;
	if (!isScalar(node)) throw new EditError(`No fragment at ${path.join('.')}.`);
	const map = new YAMLMap();
	map.set(doc.createNode('text'), node);
	const seq = doc.getIn(list, true) as YAMLSeq;
	seq.set(index, map);
	return map;
}

function setValue(doc: Document, map: YAMLMap, key: string, value: string) {
	const node = map.get(key, true);
	if (isScalar(node)) setScalar(node, value);
	else map.set(key, doc.createNode(value));
}

/** Change a scalar in place. A plain style changes to double quotes when the text needs it. */
function setScalar(node: Scalar, value: string) {
	node.value = value;
	if (node.type === Scalar.PLAIN && /^[\s{[&*!|>'"%@`#-]|: | #|\n/.test(value)) node.type = Scalar.QUOTE_DOUBLE;
}

/** A new author note: a plain note or a TODO. */
export interface NoteData {
	text: string;
	todo: boolean;
	/** The thread that the note is a beat of, with its role. */
	thread?: string;
	role?: 'setup' | 'payoff' | 'mention';
}

/**
 * The YAML path of a place that the loader names in its own form, for example
 * `description[2].notes[0]` or `frames.look.before[1].notes[0]`.
 */
export function pathOf(at: string): YamlPath {
	const path: YamlPath = [];
	for (const part of at.split('.')) {
		const m = part.match(/^([^[\]]+)((?:\[\d+\])*)$/);
		if (!m) throw new EditError(`Cannot read the place ${at}.`);
		path.push(m[1]);
		for (const i of m[2].matchAll(/\[(\d+)\]/g)) path.push(Number(i[1]));
	}
	return path;
}

/**
 * Add a note to the end of a `notes` list, for example `['notes']` of a room or
 * `['description', 2, 'notes']` of one fragment. A fragment that is a plain string becomes a map first.
 */
export function addNote(text: string, notes: YamlPath, note: NoteData): string {
	const doc = parse(text);
	const owner = notes.slice(0, -1);
	const index = owner.at(-1);
	if (typeof index === 'number') asMap(doc, owner.slice(0, -1), index);
	let seq = doc.getIn(notes, true);
	if (seq == null) {
		doc.setIn(notes, doc.createNode([]));
		seq = doc.getIn(notes, true);
	}
	if (!isSeq(seq)) throw new EditError(`No list at ${notes.join('.')}.`);
	if (seq.items.length === 0) seq.flow = false;
	seq.items.push(noteNode(note));
	return serialize(doc);
}

/** Mark a TODO as done, or as open again. */
export function setNoteDone(text: string, note: YamlPath, done: boolean): string {
	const doc = parse(text);
	const node = doc.getIn(note, true);
	if (!isMap(node) || !node.has('todo')) throw new EditError(`No TODO at ${note.join('.')}.`);
	if (done) node.set('done', true);
	else node.delete('done');
	return serialize(doc);
}

/** Remove a note. An empty `notes` list goes too. */
export function removeNote(text: string, note: YamlPath): string {
	const doc = parse(text);
	const list = note.slice(0, -1);
	const index = note.at(-1);
	const seq = doc.getIn(list, true);
	if (!isSeq(seq) || typeof index !== 'number' || index >= seq.items.length) throw new EditError(`No note at ${note.join('.')}.`);
	seq.items.splice(index, 1);
	if (seq.items.length === 0) doc.deleteIn(list);
	return serialize(doc);
}

function noteNode(note: NoteData) {
	if (!note.todo && !note.thread) return quoted(note.text);
	const map = new YAMLMap();
	map.set(note.todo ? 'todo' : 'text', quoted(note.text));
	if (note.thread) map.set('thread', note.thread);
	if (note.thread && note.role) map.set('role', note.role);
	// A beat is one short line: `- { text: "…", thread: x, role: setup }`.
	if (note.thread) map.flow = true;
	return map;
}

/** Set a top-level scalar, for example the `status` of a thread. The key is added when it is missing. */
export function setField(text: string, key: string, value: string): string {
	const doc = parse(text);
	const node = doc.get(key, true);
	if (isScalar(node)) setScalar(node, value);
	else doc.set(key, value);
	return serialize(doc);
}

/**
 * Set the text at a path, for example `['nodes', 'meet', 'options', 1, 'say']` in a dialogue.
 * The quoting of the old value stays when it can hold the new one.
 */
export function writeScalar(text: string, path: YamlPath, value: string): string {
	const doc = parse(text);
	const node = doc.getIn(path, true);
	if (isScalar(node)) {
		if (node.type === Scalar.QUOTE_SINGLE && value.includes("'") && value.includes('"')) node.type = Scalar.QUOTE_DOUBLE;
		setScalar(node, value);
	} else if (node === undefined) throw new EditError(`Nothing to change at ${path.join('.')}.`);
	else doc.setIn(path, quoted(value));
	return serialize(doc);
}

// ---- puzzle machines --------------------------------------------------------

/** A change to the states of a machine. */
export type StateEdit =
	| { kind: 'add'; name: string }
	| { kind: 'rename'; name: string; to: string }
	| { kind: 'describe'; name: string; description: string }
	| { kind: 'delete'; name: string };

/** A state name is one word, so conditions can name it: `exorcism == lit`. */
export const STATE_NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;

/** The name of a state entry: `idle` or `{ name: idle, description: … }`. */
function stateName(node: unknown): string | null {
	if (isScalar(node)) return String(node.value);
	if (isMap(node)) {
		const n = node.get('name');
		return n === undefined ? null : String(n);
	}
	return null;
}

/** The names in a `from` value: one state, or a list. */
function fromNames(node: unknown): string[] {
	if (isScalar(node)) return [String(node.value)];
	if (isSeq(node)) return node.items.map((i) => (isScalar(i) ? String(i.value) : '')).filter(Boolean);
	return [];
}

/**
 * Change the states of the machine at `at` in a YAML file: the whole document for `machines/<id>.yaml`,
 * or `['machines', i]` in story.yaml. `all` is the list of states as the runtime sees it; a machine
 * without a `states` list gets one first, so a new state or a description has a place.
 */
export function editStates(text: string, at: YamlPath, all: string[], edit: StateEdit): string {
	const doc = parse(text);
	const machine = at.length ? doc.getIn(at, true) : doc.contents;
	if (!isMap(machine)) throw new EditError('This is not a machine.');
	const found: unknown = machine.get('states', true);
	let list: YAMLSeq;
	if (isSeq(found)) {
		list = found;
	} else {
		list = new YAMLSeq();
		for (const s of all) list.items.push(new Scalar(s));
		list.flow = true;
		machine.set('states', list);
	}
	const index = list.items.findIndex((i) => stateName(i) === edit.name);
	const steps = machine.get('steps', true);
	const stepMaps = isSeq(steps) ? steps.items.filter(isMap) : [];
	const used = (name: string) =>
		String(machine.get('start')) === name ||
		stepMaps.some((s) => fromNames(s.get('from', true)).includes(name) || String(s.get('to')) === name);

	switch (edit.kind) {
		case 'add': {
			if (!STATE_NAME.test(edit.name)) throw new EditError('A state name is one word: letters, digits and _.');
			if (index >= 0) throw new EditError(`The state ${edit.name} is already there.`);
			list.items.push(new Scalar(edit.name));
			break;
		}
		case 'rename': {
			if (index < 0) throw new EditError(`There is no state ${edit.name}.`);
			if (!STATE_NAME.test(edit.to)) throw new EditError('A state name is one word: letters, digits and _.');
			if (edit.to !== edit.name && list.items.some((i) => stateName(i) === edit.to))
				throw new EditError(`The state ${edit.to} is already there.`);
			const item = list.items[index];
			if (isMap(item)) item.set('name', edit.to);
			else list.items[index] = new Scalar(edit.to);
			if (String(machine.get('start')) === edit.name) machine.set('start', edit.to);
			for (const s of stepMaps) {
				const from = s.get('from', true);
				if (isScalar(from) && String(from.value) === edit.name) from.value = edit.to;
				if (isSeq(from))
					for (const f of from.items) if (isScalar(f) && String(f.value) === edit.name) f.value = edit.to;
				const to = s.get('to', true);
				if (isScalar(to) && String(to.value) === edit.name) to.value = edit.to;
			}
			break;
		}
		case 'describe': {
			if (index < 0) throw new EditError(`There is no state ${edit.name}.`);
			const description = edit.description.trim();
			const item = list.items[index];
			if (!description) {
				list.items[index] = new Scalar(edit.name);
			} else if (isMap(item)) {
				item.set('description', quoted(description));
			} else {
				const map = new YAMLMap();
				map.set('name', edit.name);
				map.set('description', quoted(description));
				list.items[index] = map;
			}
			// A list with descriptions reads better one state per line.
			list.flow = list.items.every((i) => isScalar(i));
			break;
		}
		case 'delete': {
			if (index < 0) throw new EditError(`There is no state ${edit.name}.`);
			if (used(edit.name)) throw new EditError(`The start or a step uses the state ${edit.name}. Change them first.`);
			list.items.splice(index, 1);
			break;
		}
	}
	return serialize(doc);
}

/** Can the state go? Not while the start or a step uses it. */
export function stateInUse(machine: { start?: string; steps: { from: string[]; to: string }[] }, name: string): boolean {
	return machine.start === name || machine.steps.some((s) => s.from.includes(name) || s.to === name);
}

/** Where machine `id` is in its file: the whole document of `machines/<id>.yaml`, or its item in `machines:`. */
export function machinePath(text: string, file: string, id: string): YamlPath {
	if (file.startsWith('machines/')) return [];
	const doc = parse(text);
	const list = doc.get('machines', true);
	const i = isSeq(list) ? list.items.findIndex((m) => isMap(m) && String(m.get('id')) === id) : -1;
	if (i < 0) throw new EditError(`There is no machine ${id} in ${file}.`);
	return ['machines', i];
}
