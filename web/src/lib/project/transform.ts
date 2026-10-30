// "Type first, tune later": changes to a selection in fragment text.
// Each function takes the text and the selection, and returns the new text and where the cursor goes.

export interface Edit {
	text: string;
	/** The cursor or the selection after the change. */
	from: number;
	to: number;
}

/** The markup depth at a position: 0 outside `{…}`. */
function depthAt(text: string, pos: number): number {
	let depth = 0;
	for (let i = 0; i < pos; i++) {
		if ((text[i] === '{' && text[i + 1] === '{') || (text[i] === '}' && text[i + 1] === '}')) {
			i++;
			continue;
		}
		if (text[i] === '{') depth++;
		else if (text[i] === '}') depth = Math.max(0, depth - 1);
	}
	return depth;
}

/**
 * True when the selection starts or ends inside `{…}` markup, where these changes do not apply.
 * A selection around whole markup is fine: the markup moves into the new markup.
 */
export function inMarkup(text: string, from: number, to: number): boolean {
	return depthAt(text, from) > 0 || depthAt(text, to) > 0;
}

/** `{first: …}`: the selection shows only on the first visit. */
export function onlyFirst(text: string, from: number, to: number): Edit {
	const out = `${text.slice(0, from)}{first: ${text.slice(from, to)}}${text.slice(to)}`;
	return { text: out, from: from + 8, to: from + 8 + (to - from) };
}

/** `{ ? …}`: the selection shows only when a condition holds. The cursor goes where the condition goes. */
export function onlyWhen(text: string, from: number, to: number): Edit {
	const out = `${text.slice(0, from)}{ ? ${text.slice(from, to)}}${text.slice(to)}`;
	return { text: out, from: from + 1, to: from + 1 };
}

/** A name of a thing as text, for example `the oil lamp` for `oil_lamp.the`. */
export interface NameForm {
	/** The markup name, for example `oil_lamp.the`. */
	name: string;
	/** The text now, for example `the oil lamp`. */
	value: string;
}

/**
 * The markup for the selected words when they are the name of a thing: "the oil lamp" becomes `{oil_lamp.the}`.
 * The name then follows the thing, for example its name stages. Null when no name fits.
 */
export function useName(text: string, from: number, to: number, names: NameForm[]): Edit | null {
	const sel = text.slice(from, to).trim();
	if (!sel) return null;
	const exact = names.find((n) => n.value === sel);
	const loose = exact ?? names.find((n) => n.value.toLowerCase() === sel.toLowerCase());
	if (!loose) return null;
	const lead = text.slice(from, to).length - text.slice(from, to).trimStart().length;
	const start = from + lead;
	const end = start + sel.length;
	const markup = `{${loose.name}}`;
	return { text: `${text.slice(0, start)}${markup}${text.slice(end)}`, from: start + markup.length, to: start + markup.length };
}

/**
 * Variants from alternatives in the selection, split at " / " or at line breaks:
 * in "The wind howls / drops.", the selection "howls / drops" gives "The wind howls." and "The wind drops.".
 * Null when the selection has fewer than two alternatives.
 */
export function variantsFrom(text: string, from: number, to: number): string[] | null {
	const parts = text
		.slice(from, to)
		.split(/\s+\/\s+|\n+/)
		.map((p) => p.trim())
		.filter(Boolean);
	if (parts.length < 2) return null;
	const before = text.slice(0, from);
	const after = text.slice(to);
	return parts.map((p) => `${before}${p}${after}`);
}
