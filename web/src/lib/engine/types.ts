// Mirrors the serde types of lantern-runtime as they cross the WASM boundary.

export type Layer = 'base' | 'ambient' | 'event' | 'objects' | 'characters' | 'exits';
export type FragmentMode = 'always' | 'first_visit' | 'once' | 'cycle' | 'shuffle' | 'sticky';

export interface Source {
	owner: string;
	fragment: number;
	layer: Layer;
	condition?: string;
	variant: number;
}

export interface Link {
	kind: 'thing' | 'exit';
	id: string;
}

export interface Run {
	text: string;
	source?: Source;
	link?: Link;
}

export type BlockKind =
	| 'command'
	| 'title'
	| 'prose'
	| 'objects'
	| 'exits'
	| 'response'
	| 'narration'
	| 'end'
	/** A parser note about an action the game did by itself, for example "(first opening the door)". */
	| 'note'
	/** An author action from the command palette. It is not part of the game. */
	| 'author';

export interface Block {
	kind: BlockKind;
	runs: Run[];
}

export interface LogEntry {
	kind: string;
	text: string;
}

export interface Turn {
	blocks: Block[];
	ended: boolean;
	log: LogEntry[];
}

export interface ThingInfo {
	id: string;
	name: string;
}

export interface ExitInfo {
	direction: string;
	to: string;
	to_name: string;
}

export interface Scene {
	room: string;
	room_name: string;
	time: string;
	phase: string;
	day: number;
	turns: number;
	dark: boolean;
	things: ThingInfo[];
	inventory: ThingInfo[];
	exits: ExitInfo[];
	ended: boolean;
}

export interface VerbInfo {
	id: string;
	label: string;
	slots: 'none' | 'one' | 'two';
	preposition: string | null;
	bar: boolean;
}

export interface SourcePiece {
	role: 'text' | 'syntax' | 'insert';
	text: string;
	/** False when the piece is in an inline branch whose condition does not hold now. */
	active: boolean;
	/** For an insert: the value it shows now. */
	value?: string;
}

export interface FragmentView {
	index: number;
	layer: Layer;
	mode: FragmentMode;
	condition: string | null;
	replaces: boolean;
	text: string;
	/** The source of each variant in pieces. */
	variants: SourcePiece[][];
	shown: boolean;
	why: string | null;
}

export type Command =
	| { type: 'verb'; verb: string; objects: string[] }
	| { type: 'go'; direction: string };

export interface Diagnostic {
	severity: 'error' | 'warning';
	file: string;
	at: string;
	message: string;
	line: number | null;
	column: number | null;
}

export interface SourceFile {
	path: string;
	text: string;
}

export interface MapRoom {
	id: string;
	name: string;
	x: number;
	y: number;
	visited: boolean;
	current: boolean;
}

export interface MapLink {
	from: string;
	to: string;
	direction: string;
	visible: boolean;
	one_way: boolean;
}

export interface MapView {
	rooms: MapRoom[];
	links: MapLink[];
}

export interface PartView {
	/** A thing id, or an exit direction. */
	id: string;
	label: string;
	/** The presence text of a thing, or the generated text. */
	text: string;
	detail: string | null;
	shown: boolean;
	why: string | null;
}

export interface RoomParts {
	objects: PartView[];
	exits: PartView[];
}

export interface RuleView {
	id: string;
	/** The event, in words. */
	when: string;
	condition: string | null;
	condition_holds: boolean;
	/** Each effect, in words. */
	then: string[];
	once: boolean;
	instead: boolean;
	fired: boolean;
	/** The kind of event: start, turn, enter, leave, time or verb. */
	trigger: 'start' | 'turn' | 'enter' | 'leave' | 'time' | 'verb';
	/** Why the rule matters here and now, or null. */
	relevant: string | null;
}

/** What a command that is still being typed refers to. */
export interface Preview {
	verb: string | null;
	/** Things the command names without doubt, in order. */
	objects: string[];
	/** Things the last, unfinished noun phrase could still mean. */
	candidates: string[];
	direction: string | null;
	complete: boolean;
}

export type Value = boolean | number | string;

/** A command for the author: it changes the game state directly. */
export type AuthorCommand =
	| { type: 'goto'; room: string }
	| { type: 'set'; name: string; value: Value }
	| { type: 'time'; time: string }
	| { type: 'give'; thing: string }
	| { type: 'situation'; room: string; values: [string, string][] };

export interface VarView {
	name: string;
	value: Value;
	/** For an enum variable: all values. */
	options: string[];
}

export interface PhaseView {
	name: string;
	from: string;
}

/** Facts about a thing or an exit, for an author tooltip. */
export interface Inspect {
	kind: 'thing' | 'exit';
	id: string;
	title: string;
	/** Label and value pairs, in display order. */
	facts: [string, string][];
	/** Ids of related rules. */
	rules: string[];
}

/** One fragment of an action frame, with its state for the last action. */
export interface FrameFragment {
	index: number;
	mode: FragmentMode;
	condition: string | null;
	text: string;
	shown: boolean;
	why: string | null;
}

/** The action frame used for the last action. */
export interface FrameUse {
	verb: string;
	/** `story`, `room:<id>` or `thing:<id>`. */
	scope: string;
	repeat: number;
	count: number;
	total: number;
	parts: { part: 'before' | 'instead' | 'after'; fragments: FrameFragment[] }[];
}

export interface TextVariant {
	text: string;
	states: number;
	/** Commands from the start, as `go north` or `<verb> <thing ids>`. */
	path: string[];
	facts: string[];
}

export interface RoomCoverage {
	id: string;
	name: string;
	reached: boolean;
	variants: TextVariant[];
	/** Every full room text that the explorer saw, with the things in the room. */
	seen: string[];
}

/** One value that the room text depends on, and the values worth trying. */
export interface Dimension {
	name: string;
	values: string[];
}

/** The room text for one combination of values. */
export interface MatrixCell {
	/** One value for each dimension, in the order of the dimensions. */
	values: string[];
	text: string;
	warnings: string[];
}

/** The text of a room in all combinations of the values that the text depends on. */
export interface RoomMatrix {
	room: string;
	dimensions: Dimension[];
	cells: MatrixCell[];
	/** All combinations. When this is more than the cells, the matrix shows only the first ones. */
	combinations: number;
}

/** The result of the state explorer. */
export interface Exploration {
	states: number;
	truncated: boolean;
	rooms: RoomCoverage[];
	never_shown: string[];
	never_fired: string[];
	endings: { text: string; path: string[] }[];
	stuck: number;
	stuck_path: string[];
}
