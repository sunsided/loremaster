// Tab completion for the command prompt: verbs, things in reach and exits, as the parser knows them.

/** What a Tab press gives: the new text, and all words that fit. */
export interface Completion {
	text: string;
	/** All words that fit the end of the text, for the hint under the prompt. */
	matches: string[];
}

/**
 * Complete the end of `text` from `words`. A word can have spaces, for example "pick up" or
 * "oak door"; it can complete the last one or more typed words. One match completes the word and
 * adds a space. More matches complete to their common start. `cycle` picks the next match when
 * the author pushes Tab again. Null when nothing fits.
 */
export function complete(text: string, words: string[], cycle = -1): Completion | null {
	const typed = text.toLowerCase();
	const parts = typed.split(' ');
	// The longest typed end that a word starts with: "pick u" fits "pick up" with two typed words.
	for (let k = parts.length; k >= 1; k--) {
		const end = parts.slice(-k).join(' ');
		if (!end.trim()) continue;
		const matches = [...new Set(words.map((w) => w.toLowerCase()))].filter((w) => w.startsWith(end) && w !== end).sort();
		if (!matches.length) continue;
		const head = text.slice(0, text.length - end.length);
		if (cycle >= 0) return { text: head + matches[cycle % matches.length] + ' ', matches };
		if (matches.length === 1) return { text: head + matches[0] + ' ', matches };
		const common = matches.reduce((a, b) => {
			let i = 0;
			while (i < a.length && a[i] === b[i]) i++;
			return a.slice(0, i);
		});
		return { text: head + common, matches };
	}
	return null;
}
