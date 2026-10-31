// Mirrors the serde types of loremaster-runtime as they cross the WASM boundary.

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
	| 'author'
	/** The player's line in a talk. */
	| 'said'
	/** The line of the character the player talks to. */
	| 'speech';

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
	/** The talk in progress after this turn. Absent outside a talk. */
	talk?: TalkInfo;
	/** The answers the player can choose now, during a talk. */
	choices?: Choice[];
}

/** Who the player is talking to. */
export interface TalkInfo {
	dialogue: string;
	/** The thing id. */
	with: string;
	/** Its current name, for example "Blackbeard". */
	name: string;
	/** The key of the current node. */
	node: string;
}

/** An answer the player can choose. */
export interface Choice {
	/** The number to pass to `choose`, from 0. The player sees it plus 1. */
	index: number;
	text: string;
}

/** An option of the current node, for the inspector. */
export interface TalkOptionView {
	/** The index to choose it with, or null when its condition does not hold. */
	choice: number | null;
	say: string;
	when?: string;
	/** A node key, `end` or `back`. */
	go: string;
	outcomes: string[];
}

/** The talk in progress, for the inspector and the in-place editor. */
export interface TalkView {
	dialogue: string;
	with: string;
	name: string;
	node: string;
	/** The line as written. */
	say: string;
	when?: string;
	outcomes: string[];
	options: TalkOptionView[];
	relations: [string, number][];
	memories: string[];
}

/** A dialogue as an outline: nodes, their options, and where each option goes. */
export interface DialogueOutline {
	id: string;
	/** The thing id. */
	with: string;
	start: string[];
	nodes: OutlineNode[];
}

export interface OutlineNode {
	key: string;
	/** The place of the node in `start`, if it is a start node. */
	start?: number;
	when?: string;
	say: string;
	outcomes: string[];
	options: OutlineOption[];
	/** No start node leads here. */
	unreachable: boolean;
}

export interface OutlineOption {
	say: string;
	when?: string;
	/** A node key, `end` or `back`. */
	go: string;
	/** `go` names a node that does not exist. */
	missing: boolean;
	outcomes: string[];
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
	/** The words that the parser accepts, for example `take` and `pick up`. */
	words: string[];
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
	| { type: 'go'; direction: string }
	| { type: 'choose'; index: number }
	| { type: 'leave' };

export interface Diagnostic {
	severity: 'error' | 'warning';
	file: string;
	at: string;
	message: string;
	line: number | null;
	column: number | null;
}

/** An author note. The player never sees it. */
export interface Note {
	id: string | null;
	text: string;
	/** A task for the authors. */
	todo: boolean;
	done: boolean;
	/** The thread that the note is a beat of. */
	thread: string | null;
	role: 'setup' | 'payoff' | 'mention' | null;
}

/** A note and where it is. */
export interface NoteRef {
	/** `story`, `room:shore`, `thing:lamp`, `rule:find_key` or `thread:lost_key`. */
	entity: string;
	file: string;
	/** Where in the entity, for example `notes[0]` or `description[2].notes[1]`. */
	at: string;
	fragment_id: string | null;
	line: number | null;
	note: Note;
}

/** A long-range thread with its beats. */
export interface ThreadView {
	id: string;
	name: string;
	status: 'open' | 'resolved' | 'dropped';
	summary: string | null;
	file: string;
	beats: NoteRef[];
	setups: number;
	payoffs: number;
	open_todos: number;
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
	/** The level, for towers and dungeons. Up exits go one level higher. */
	level: number;
	visited: boolean;
	current: boolean;
	/** Editor only: the AI proposal that adds the room. */
	proposed?: string;
	/** Editor only: the AI proposal that removes the room. */
	removed?: string;
}

export interface MapLink {
	from: string;
	to: string;
	direction: string;
	visible: boolean;
	one_way: boolean;
	/** Editor only: the AI proposal that adds the link. */
	proposed?: string;
	/** Editor only: the AI proposal that removes the link. */
	removed?: string;
}

/** An exit that leads nowhere and only prints a message. */
export interface MapEnd {
	room: string;
	direction: string;
	visible: boolean;
	/** Editor only: the AI proposal that adds the exit. */
	proposed?: string;
	/** Editor only: the AI proposal that removes the exit. */
	removed?: string;
}

export interface MapView {
	rooms: MapRoom[];
	links: MapLink[];
	/** The message exits. */
	ends?: MapEnd[];
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

/** A puzzle machine: its states, where it is now and its steps. */
export interface MachineView {
	id: string;
	state: string;
	states: string[];
	/** Author text about states, by state name. */
	descriptions: Record<string, string>;
	steps: MachineStepView[];
}

/** One step of a puzzle machine. */
export interface MachineStepView {
	/** The id of the rule that the step became. */
	rule: string;
	from: string[];
	to: string;
	/** The event, in words. */
	when: string;
	condition: string | null;
	/** The step ran at least once in this game. */
	fired: boolean;
	/** The machine is in a `from` state and the condition holds. */
	ready: boolean;
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
	/** The kind of event. `check` runs before a verb, `verb` instead of or after it. */
	trigger:
		| 'start'
		| 'turn'
		| 'enter'
		| 'leave'
		| 'time'
		| 'verb'
		| 'check'
		| 'timer'
		| 'event'
		| 'talk';
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
	/** For each open or resolved thread with a payoff: whether a payoff showed, and in which order. */
	threads: ThreadReach[];
}

/** What the explorer found for one thread. */
export interface ThreadReach {
	thread: string;
	/** True when an explored path showed a payoff. */
	paid_off: boolean;
	/** The shortest explored path on which a payoff shows before any setup; empty when there is none. */
	payoff_before_setup: string[];
	/** Where the payoffs are, for example `room:vault#2`. */
	payoffs: string[];
}

/** The check of text that the author types. Offsets are UTF-16. */
export interface MarkupCheck {
	error: { start: number; end: number; message: string } | null;
	/** The rendered text in the current state. Empty for a condition. */
	text: string;
	pieces: SourcePiece[];
	/** For a condition: whether it holds now. */
	holds: boolean | null;
}

/** One room for the Atlas: where it is, what is in it now, and its exits. */
export interface AtlasRoom {
	id: string;
	name: string;
	level: number;
	visited: boolean;
	current: boolean;
	things: ThingInfo[];
	/** Each exit as [direction, room id]. The room id is empty for an exit that only prints a message. */
	exits: [string, string][];
	/** The number of room fragments. */
	fragments: number;
	/** Editor only: the AI proposal that adds the room. */
	proposed?: string;
	/** Editor only: the AI proposal that removes the room. */
	removed?: string;
}
