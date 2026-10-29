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
	| 'end';

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
}
