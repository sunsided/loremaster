// Shared prop types of the design components.

export interface RadioOption<V extends string = string> {
	value: V;
	label: string;
	/** A short description. The `cards` look shows it under the label. */
	description?: string;
}

export interface ActionItem {
	id: string;
	label: string;
	/** Text after the label in a lighter colour, for example a direction. */
	detail?: string;
	/** What an author tooltip shows facts about, for example `thing:oil_lamp`. */
	inspect?: string;
}

/** In order of preference: phosphor is the default, then night, then paper. */
export type Theme = 'phosphor' | 'night' | 'paper';
export type EffectsLevel = 'full' | 'subtle' | 'off';
export type MotionPreference = 'system' | 'reduce';

export interface TranscriptEntry {
	id: number;
	turn: import('#lib/engine/types.ts').Turn;
	/** True when an edit of the story wrote the room text of this turn again. */
	rewritten?: boolean;
}

/** The fragment that the writer edits: its owner and index. Index -1 is a new fragment. */
export interface WritingAt {
	owner: string;
	fragment: number;
}

export interface View {
	id: string;
	label: string;
	/** A view that is not available yet. */
	disabled?: boolean;
}

/** One turn of a playthrough, for the story timeline. */
export interface StoryStep {
	turn: number;
	time: string;
	phase: string;
	room: string;
	/** The command as the player gave it. Empty for the opening. */
	command: string;
	/** Ids of the rules that fired. */
	rules: string[];
	/** State changes, for example "drawer.open = true". */
	changes: string[];
	ended: boolean;
}

export interface TabItem {
	id: string;
	label: string;
}

/** One action in the command palette. */
export interface PaletteItem {
	/** Unique in the palette. */
	id: string;
	/** The group heading, for example "Rooms" or "Rules". */
	group: string;
	label: string;
	/** Short extra text on the right, for example the current value. */
	hint?: string;
	/** More words that the search matches. */
	keywords?: string;
	run: () => void;
}

/** Where the player is and what is in reach, for the room brief in the transcript. */
export interface RoomBrief {
	room: string;
	things: import('#lib/engine/types.ts').ThingInfo[];
	exits: import('#lib/engine/types.ts').ExitInfo[];
}

/** A highlighted span of fragment text or of a condition, in UTF-16 offsets. */
export interface MarkupToken {
	from: number;
	to: number;
	kind: 'brace' | 'separator' | 'name' | 'operator' | 'literal' | 'escape' | 'emphasis' | 'invalid';
}

/** A problem in text that the author types, in UTF-16 offsets. */
export interface TextProblem {
	from: number;
	to: number;
	message: string;
}

/** A fragment in the writer: the same shape as in the project files. */
export interface WriterFragment {
	variants: string[];
	when: string | null;
	mode: 'always' | 'first_visit' | 'once' | 'cycle' | 'shuffle' | 'sticky';
	layer: 'base' | 'ambient' | 'event' | 'objects' | 'characters' | 'exits';
	replaces: boolean;
}

/** A name that conditions and markup can use, for completion and hover. */
export interface EditorSymbol {
	name: string;
	/** `variable`, `property`, `name`, `derived`, `phase` or `markup`. */
	kind: string;
	detail: string;
	/** The value now, as text. */
	value: string | null;
	/** The values to compare with, for example the values of an enum. */
	values: string[];
}

/** What the writer needs from the runtime. */
export interface WriterTools {
	/** The names for completion and hover. */
	symbols: EditorSymbol[];
	/** The text in each combination of the values that it depends on. */
	textMatrix?: (text: string, when: string | null) => import('#lib/engine/types.ts').RoomMatrix | null;
	tokenize: (text: string, condition: boolean) => MarkupToken[];
	/** Check fragment text and render it in the current state. */
	tryMarkup: (text: string) => { problem: TextProblem | null; text: string; pieces: import('#lib/engine/types.ts').SourcePiece[] };
	/** Check a condition and tell whether it holds now. */
	tryCondition: (text: string) => { problem: TextProblem | null; holds: boolean | null };
}
