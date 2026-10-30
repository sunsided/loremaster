// The "typing" reveal of new game text, one line after the other.
//
// The CSS reveal (`.fx-line`) wipes a whole paragraph at once: all its lines grow together.
// This reveal measures the lines of each paragraph and wipes them one by one, like a
// terminal that prints line after line. The text is in the page from the start, so screen
// readers and copy and paste do not wait for it.

import type { Attachment } from 'svelte/attachments';

export interface LineReveal {
	/** Time for one line, in ms. */
	line: number;
	/** Steps for one line: a low number gives a coarse, typed look. */
	steps: number;
	/** Time before the first line, in ms. */
	delay: number;
}

/** The reveal for an effects level, or null when the text shows at once. */
export function lineReveal(effects: 'full' | 'subtle' | 'off', reduceMotion: boolean): LineReveal | null {
	if (reduceMotion || effects === 'off') return null;
	return effects === 'full' ? { line: 420, steps: 28, delay: 120 } : { line: 160, steps: 8, delay: 60 };
}

/** The clip of a paragraph with `done` full lines, and the next line shown to `part` (0 to 1). */
function clip(done: number, part: number, height: number): string {
	const top = done * height;
	const bottom = top + height;
	const x = `${(part * 100).toFixed(2)}%`;
	return `polygon(0 0, 100% 0, 100% ${top}px, ${x} ${top}px, ${x} ${bottom}px, 0 ${bottom}px)`;
}

/**
 * Reveal the paragraphs with `[data-reveal]` in the element, line by line, in their order.
 * Each paragraph waits for the paragraphs before it.
 */
export function revealLines(reveal: LineReveal | null): Attachment<HTMLElement> {
	return (element) => {
		if (!reveal || typeof element.animate !== 'function') return;
		const animations: Animation[] = [];
		let start = reveal.delay;
		for (const p of element.querySelectorAll<HTMLElement>('[data-reveal]')) {
			const height = parseFloat(getComputedStyle(p).lineHeight) || p.getBoundingClientRect().height;
			const lines = Math.max(1, Math.round(p.getBoundingClientRect().height / height));
			// Each line grows from the left in steps; then the next line starts at once.
			const frames: Keyframe[] = [];
			const gap = 0.001 / lines;
			for (let i = 0; i < lines; i++) {
				frames.push({ clipPath: clip(i, 0, height), offset: i / lines, easing: `steps(${reveal.steps}, end)` });
				frames.push({ clipPath: clip(i, 1, height), offset: (i + 1) / lines - (i + 1 < lines ? gap : 0) });
			}
			// Hidden before its start ("backwards"); no clip after the end, so a later reflow cannot cut the text.
			animations.push(p.animate(frames, { duration: lines * reveal.line, delay: start, fill: 'backwards' }));
			start += lines * reveal.line;
		}
		return () => animations.forEach((a) => a.cancel());
	};
}
