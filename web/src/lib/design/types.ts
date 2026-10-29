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
