<script lang="ts">
	import { onMount } from 'svelte';
	import { EditorSelection, EditorState, RangeSetBuilder, StateEffect, StateField, type Extension } from '@codemirror/state';
	import { Decoration, EditorView, WidgetType, keymap, placeholder as placeholderExt, type DecorationSet } from '@codemirror/view';
	import { defaultKeymap, history, historyKeymap, insertNewline } from '@codemirror/commands';

	interface Props {
		/** The text with its markup. Bind it to read the changes. */
		value?: string;
		label: string;
		/** How a piece of markup reads now, for example `{night: dark | pale}` reads "pale". */
		render: (markup: string) => string | null;
		placeholder?: string;
		onsubmit?: () => void;
		oncancel?: () => void;
		/** The selection changed. `rect` is null when nothing is selected. */
		onselection?: (from: number, to: number, rect: { left: number; top: number; bottom: number } | null) => void;
		/** Ctrl+. : open the tools for the selection. */
		ontools?: () => void;
		/** Ctrl+/ : show or hide the markup in the fragment panel. */
		onmarkup?: () => void;
		/** Change it to render the chips again, for example when the values of a preview change. */
		stamp?: unknown;
		/** A cursor or selection to take; it applies when `n` changes. */
		target?: { from: number; to: number; n: number } | null;
		autofocus?: boolean;
		describedby?: string;
	}

	let {
		value = $bindable(''),
		label,
		render,
		placeholder = '',
		onsubmit,
		oncancel,
		onselection,
		ontools,
		onmarkup,
		stamp,
		target = null,
		autofocus = false,
		describedby
	}: Props = $props();

	let host: HTMLDivElement | undefined = $state();
	let view: EditorView | undefined;
	const refresh = StateEffect.define<null>();

	/** Markup as the player reads it. A click opens the markup in place. */
	class Chip extends WidgetType {
		text: string;
		source: string;
		at: number;
		constructor(text: string, source: string, at: number) {
			super();
			this.text = text;
			this.source = source;
			this.at = at;
		}
		eq(other: Chip) {
			return other.text === this.text && other.source === this.source && other.at === this.at;
		}
		toDOM(v: EditorView) {
			const el = document.createElement('span');
			el.className = this.text ? 'chip' : 'chip empty';
			el.textContent = this.text || '◦';
			el.title = this.text ? `${this.source} (click to edit)` : `Not shown now: ${this.source} (click to edit)`;
			el.addEventListener('mousedown', (e) => {
				e.preventDefault();
				v.dispatch({ selection: { anchor: this.at + 1 } });
				v.focus();
			});
			return el;
		}
		ignoreEvent() {
			return false;
		}
	}

	/** Top-level markup ranges `{…}`, without the escapes `{{` and `}}`. */
	function markup(text: string): { from: number; to: number }[] {
		const out: { from: number; to: number }[] = [];
		let depth = 0;
		let start = -1;
		for (let i = 0; i < text.length; i++) {
			const c = text[i];
			if (depth === 0 && (c === '{' || c === '}') && text[i + 1] === c) {
				i++;
				continue;
			}
			if (c === '{') {
				if (depth === 0) start = i;
				depth++;
			} else if (c === '}' && depth > 0) {
				depth--;
				if (depth === 0) out.push({ from: start, to: i + 1 });
			}
		}
		return out;
	}

	function decorate(state: EditorState): DecorationSet {
		const text = state.doc.toString();
		const sel = state.selection.main;
		const marks: { from: number; to: number; deco: Decoration }[] = [];
		const ranges = markup(text);
		for (const r of ranges) {
			// The markup under the cursor shows as source, so the author can edit it.
			if (sel.from <= r.to && sel.to >= r.from && sel.head > r.from && sel.head < r.to) continue;
			const shown = render(text.slice(r.from, r.to));
			if (shown === null) continue;
			marks.push({ from: r.from, to: r.to, deco: Decoration.replace({ widget: new Chip(shown, text.slice(r.from, r.to), r.from) }) });
		}
		// *Emphasis* reads in italics, and the stars do not show unless the cursor is next to them.
		for (const m of text.matchAll(/\*([^*\n]+)\*/g)) {
			const from = m.index;
			const to = from + m[0].length;
			if (ranges.some((r) => from < r.to && to > r.from)) continue;
			marks.push({ from, to, deco: Decoration.mark({ class: 'em' }) });
			if (!(sel.head >= from && sel.head <= to)) {
				marks.push({ from, to: from + 1, deco: Decoration.replace({}) });
				marks.push({ from: to - 1, to, deco: Decoration.replace({}) });
			}
		}
		marks.sort((a, b) => a.from - b.from || a.to - b.to);
		const b = new RangeSetBuilder<Decoration>();
		for (const m of marks) b.add(m.from, m.to, m.deco);
		return b.finish();
	}

	function extensions(): Extension[] {
		const field = StateField.define<DecorationSet>({
			create: (s) => decorate(s),
			update: (deco, tr) => (tr.docChanged || tr.selection || tr.effects.some((e) => e.is(refresh)) ? decorate(tr.state) : deco),
			provide: (f) => [EditorView.decorations.from(f), EditorView.atomicRanges.of((v) => v.state.field(f))]
		});
		const submit = () => {
			onsubmit?.();
			return true;
		};
		return [
			history(),
			field,
			keymap.of([
				{ key: 'Enter', run: submit, shift: insertNewline },
				{ key: 'Mod-Enter', run: submit },
				{
					key: 'Escape',
					run: () => {
						oncancel?.();
						return !!oncancel;
					}
				},
				{
					key: 'Mod-.',
					run: () => {
						ontools?.();
						return true;
					}
				},
				{
					key: 'Mod-/',
					run: () => {
						onmarkup?.();
						return !!onmarkup;
					}
				},
				...historyKeymap,
				...defaultKeymap
			]),
			EditorView.lineWrapping,
			placeholderExt(placeholder),
			EditorView.contentAttributes.of({
				'aria-label': label,
				'aria-multiline': 'true',
				spellcheck: 'true',
				...(describedby ? { 'aria-describedby': describedby } : {})
			}),
			EditorView.updateListener.of((u) => {
				if (u.docChanged) value = u.state.doc.toString();
				if (u.selectionSet || u.docChanged || u.focusChanged) report(u.view);
			})
		];
	}

	function report(v: EditorView) {
		const { from, to } = v.state.selection.main;
		if (from === to || !v.hasFocus) {
			if (v.hasFocus) onselection?.(from, to, null);
			return;
		}
		const a = v.coordsAtPos(from);
		const b = v.coordsAtPos(to);
		if (!a || !b) return;
		onselection?.(from, to, { left: Math.min(a.left, b.left), top: Math.min(a.top, b.top), bottom: Math.max(a.bottom, b.bottom) });
	}

	onMount(() => {
		view = new EditorView({ parent: host!, state: EditorState.create({ doc: value, extensions: extensions() }) });
		if (autofocus) {
			view.focus();
			view.dispatch({ selection: { anchor: view.state.doc.length } });
		}
		return () => view?.destroy();
	});

	// A value set from outside, for example from the source view, replaces the text.
	$effect(() => {
		const v = value;
		if (view && v !== view.state.doc.toString()) {
			view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: v } });
		}
		// The chips read the state of the game: render them again when it changes.
		render;
		stamp;
		view?.dispatch({ effects: refresh.of(null) });
	});

	let lastTarget = 0;
	$effect(() => {
		const t = target;
		if (!view || !t || t.n === lastTarget) return;
		lastTarget = t.n;
		const len = view.state.doc.length;
		view.dispatch({ selection: EditorSelection.single(Math.min(t.from, len), Math.min(t.to, len)) });
		view.focus();
	});
</script>

<!--
	The prose view of a fragment: the author types in the game text, and it reads as the player reads it.
	Markup shows as chips with the text it gives now; a click on a chip (or the cursor in it) shows its source.
	*Emphasis* shows in italics. Enter saves, Shift+Enter adds a line break, Escape cancels, Ctrl+. opens the tools.
-->
<div class="prose-editor" bind:this={host}></div>

<style>
	.prose-editor {
		margin: 0 calc(-1 * var(--space-3));
		padding: 0 var(--space-3);
		border-left: var(--size-border-strong) solid var(--color-accent);
		background: var(--color-accent-tint);
	}
	.prose-editor :global(.cm-editor) {
		font: inherit;
		line-height: inherit;
		background: transparent;
		color: inherit;
	}
	.prose-editor :global(.cm-editor.cm-focused) {
		outline: none;
	}
	.prose-editor:focus-within {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.prose-editor :global(.cm-scroller) {
		font: inherit;
		line-height: inherit;
	}
	.prose-editor :global(.cm-content) {
		padding: 0;
		caret-color: var(--color-text-accent);
	}
	.prose-editor :global(.cm-line) {
		padding: 0;
	}
	.prose-editor :global(.cm-cursor) {
		border-left-color: var(--color-text-accent);
	}
	.prose-editor :global(.cm-selectionBackground),
	.prose-editor :global(.cm-editor.cm-focused .cm-selectionBackground) {
		background: var(--color-accent-tint);
	}
	/* On the tint, the muted colour is too faint: the placeholder is in italics in the text colour instead. */
	.prose-editor :global(.cm-placeholder) {
		color: var(--color-text);
		font-style: italic;
	}
	.prose-editor :global(.chip) {
		border-bottom: var(--size-border) dotted var(--color-text-accent);
		cursor: pointer;
	}
	.prose-editor :global(.chip.empty) {
		color: var(--color-text-muted);
		border-bottom-style: dashed;
	}
	.prose-editor :global(.em) {
		font-style: italic;
	}
</style>
