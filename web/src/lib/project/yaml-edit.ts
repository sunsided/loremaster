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
