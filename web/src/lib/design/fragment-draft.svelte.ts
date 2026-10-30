// The fragment that the author edits now. The prose view in the game text and the source view in the
// writer both edit this one draft, so they always show the same text.

import { onlyFirst, onlyWhen, useName, variantsFrom, type Edit, type NameForm } from '#lib/project/transform.ts';
import type { WriterFragment } from './types.ts';

/** Where a selection is: in which view, the range in the text, and its place on the screen. */
export interface DraftSelection {
	view: 'prose' | 'source';
	from: number;
	to: number;
	/** The screen rectangle of the selection, for the toolbar. */
	rect: { left: number; top: number; bottom: number };
}

export class FragmentDraft {
	variants = $state<string[]>(['']);
	current = $state(0);
	when = $state('');
	layer = $state<WriterFragment['layer']>('base');
	mode = $state<WriterFragment['mode']>('always');
	replaces = $state(false);
	/** The selection that the toolbar acts on, or null. */
	selection = $state<DraftSelection | null>(null);
	/**
	 * A cursor or selection that a view must take, after a toolbar change. The number changes each time,
	 * so the same range can be set twice.
	 */
	target = $state<{ view: 'prose' | 'source'; from: number; to: number; n: number } | null>(null);
	/** Changes when Ctrl+. asks for the selection toolbar. */
	toolsFocus = $state(0);

	constructor(fragment: WriterFragment, variant = 0) {
		this.variants = [...fragment.variants];
		this.current = Math.min(variant, Math.max(0, fragment.variants.length - 1));
		this.when = fragment.when ?? '';
		this.layer = fragment.layer;
		this.mode = fragment.mode;
		this.replaces = fragment.replaces;
	}

	get text(): string {
		return this.variants[this.current] ?? '';
	}

	set text(value: string) {
		this.variants[this.current] = value;
	}

	/** The fragment as it goes to the file. */
	snapshot(): WriterFragment {
		return {
			variants: [...this.variants],
			when: this.when.trim() || null,
			layer: this.layer,
			mode: this.mode,
			replaces: this.replaces
		};
	}

	#apply(edit: Edit, view: 'prose' | 'source') {
		this.text = edit.text;
		this.selection = null;
		this.target = { view, from: edit.from, to: edit.to, n: (this.target?.n ?? 0) + 1 };
	}

	onlyFirst() {
		const s = this.selection;
		if (s) this.#apply(onlyFirst(this.text, s.from, s.to), s.view);
	}

	/** Wrap the selection in a condition. The cursor goes into the condition, in the source view. */
	onlyWhen() {
		const s = this.selection;
		if (s) this.#apply(onlyWhen(this.text, s.from, s.to), 'source');
	}

	/** The name markup that fits the selection, or null. */
	nameFor(names: NameForm[]): Edit | null {
		const s = this.selection;
		return s ? useName(this.text, s.from, s.to, names) : null;
	}

	useName(names: NameForm[]) {
		const s = this.selection;
		const edit = this.nameFor(names);
		if (s && edit) this.#apply(edit, s.view);
	}

	/** The variants that the selection gives, or null. */
	get split(): string[] | null {
		const s = this.selection;
		return s ? variantsFrom(this.text, s.from, s.to) : null;
	}

	/** Replace the current variant with the variants from the selection. A single-text mode becomes `cycle`. */
	makeVariants() {
		const parts = this.split;
		if (!parts) return;
		this.variants.splice(this.current, 1, ...parts);
		if (this.mode === 'always' || this.mode === 'first_visit' || this.mode === 'once') this.mode = 'cycle';
		this.selection = null;
	}

	addVariant() {
		this.variants.push('');
		this.current = this.variants.length - 1;
		if (this.mode === 'always' || this.mode === 'first_visit' || this.mode === 'once') this.mode = 'cycle';
	}

	removeVariant() {
		if (this.variants.length < 2) return;
		this.variants.splice(this.current, 1);
		this.current = Math.min(this.current, this.variants.length - 1);
	}
}
