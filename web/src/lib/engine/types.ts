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

export interface FragmentView {
	index: number;
	layer: Layer;
	mode: FragmentMode;
	condition: string | null;
	replaces: boolean;
	text: string;
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
