<script lang="ts">
	import { onMount } from 'svelte';
	import { EditorState, RangeSetBuilder, StateEffect, StateField, type Extension } from '@codemirror/state';
	import { Decoration, EditorView, hoverTooltip, keymap, placeholder as placeholderExt, type DecorationSet } from '@codemirror/view';
	import { autocompletion, completionKeymap, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete';
	import { defaultKeymap, history, historyKeymap, insertNewline } from '@codemirror/commands';
	import type { EditorSymbol, MarkupToken, TextProblem } from '../types.ts';

	interface Props {
		/** The text. Bind it to read the changes. */
		value?: string;
		/** The accessible name of the editor. */
		label: string;
		/** The spans to highlight, for the current text. */
		tokenize: (text: string) => MarkupToken[];
		/** The first problem of the current text, or null. */
		problem?: TextProblem | null;
		placeholder?: string;
		/** One line only: Enter does not add a line break. For conditions. */
		singleLine?: boolean;
		/** Called on Enter (on Ctrl+Enter in a multi-line editor too). */
		onsubmit?: () => void;
		/** Called on Escape. */
		oncancel?: () => void;
		/** Put the focus in the editor when it opens. */
		autofocus?: boolean;
		/** The id of an element that describes the editor, for example the problem text. */
		describedby?: string;
		/** Names for completion (Ctrl+Space, or as you type) and for hover. */
		symbols?: EditorSymbol[];
		/** The selection changed. `rect` is null when nothing is selected. */
		onselection?: (from: number, to: number, rect: { left: number; top: number; bottom: number } | null) => void;
		/** Ctrl+. : open the tools for the selection. */
		ontools?: () => void;
		/** A cursor or selection to take; it applies when `n` changes. */
		target?: { from: number; to: number; n: number } | null;
	}

	let {
		value = $bindable(''),
		label,
		tokenize,
		problem = null,
		placeholder = '',
		singleLine = false,
		onsubmit,
		oncancel,
		autofocus = false,
		describedby,
		symbols = [],
		onselection,
		ontools,
		target = null
	}: Props = $props();

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

	/** True when the position is inside `{…}` markup. A condition editor is always "inside". */
	function inMarkup(text: string, pos: number): boolean {
		if (singleLine) return true;
		let depth = 0;
		for (let i = 0; i < pos; i++) {
			if (text[i] === '{' && text[i + 1] === '{') i++;
			else if (text[i] === '}' && text[i + 1] === '}') i++;
			else if (text[i] === '{') depth++;
			else if (text[i] === '}') depth = Math.max(0, depth - 1);
		}
		return depth > 0;
	}

	function complete(context: CompletionContext): CompletionResult | null {
		const text = context.state.doc.toString();
		if (!inMarkup(text, context.pos)) return null;
		const before = text.slice(0, context.pos);
		// After `name ==` or `name !=`: the values of that name.
		const cmp = /([\w.]+)\s*(?:==|!=)\s*(\w*)$/.exec(before);
		if (cmp) {
			const sym = symbols.find((s) => s.name === cmp[1]);
			if (sym?.values.length)
				return {
					from: context.pos - cmp[2].length,
					options: sym.values.map((v) => ({ label: v, type: 'enum', detail: sym.value === v ? 'now' : undefined }))
				};
		}
		const word = context.matchBefore(/[\w.]*/);
		if (!word || (word.from === word.to && !context.explicit)) return null;
		// Text names and `first` belong in markup, not in a condition.
		const usable = symbols.filter((s) => !singleLine || (s.kind !== 'name' && s.kind !== 'markup'));
		return {
			from: word.from,
			options: usable.map((s) => ({
				label: s.name,
				type: s.kind === 'variable' || s.kind === 'property' ? 'variable' : s.kind === 'name' ? 'text' : 'keyword',
				detail: s.value != null && s.value !== '' ? `= ${s.value}` : undefined,
				info: s.detail
			})),
			validFor: /^[\w.]*$/
		};
	}

	const hover = hoverTooltip((view, pos) => {
		const line = view.state.doc.lineAt(pos);
		const text = line.text;
		let from = pos - line.from;
		let to = from;
		while (from > 0 && /[\w.]/.test(text[from - 1])) from--;
		while (to < text.length && /[\w.]/.test(text[to])) to++;
		const name = text.slice(from, to).replace(/\.$/, '');
		const sym = symbols.find((s) => s.name === name);
		if (!sym) return null;
		return {
			pos: line.from + from,
			end: line.from + to,
			above: true,
			create() {
				const dom = document.createElement('div');
				dom.className = 'symbol-tip';
				const head = document.createElement('strong');
				head.textContent = sym.name;
				const detail = document.createElement('div');
				detail.textContent = sym.detail;
				dom.append(head, detail);
				if (sym.value != null && sym.value !== '') {
					const now = document.createElement('div');
					now.textContent = `now: ${sym.value}`;
					dom.append(now);
				}
				return { dom };
			}
		};
	});

	let host: HTMLDivElement | undefined = $state();
	let view: EditorView | undefined;

	const setProblem = StateEffect.define<TextProblem | null>();

	/** Highlights from the tokenizer, and a wavy line under the problem. */
	function decorations(state: EditorState, p: TextProblem | null): DecorationSet {
		const text = state.doc.toString();
		const marks: { from: number; to: number; cls: string }[] = tokenize(text).map((t) => ({
			from: t.from,
			to: t.to,
			cls: `tok-${t.kind}`
		}));
		if (p) {
			const from = Math.min(p.from, text.length);
			const to = Math.min(Math.max(p.to, from + 1), text.length);
			if (to > from) marks.push({ from, to, cls: 'tok-problem' });
		}
		marks.sort((a, b) => a.from - b.from || a.to - b.to);
		const builder = new RangeSetBuilder<Decoration>();
		for (const m of marks) if (m.to > m.from) builder.add(m.from, m.to, Decoration.mark({ class: m.cls }));
		return builder.finish();
	}

	function extensions(): Extension[] {
		const field = StateField.define<{ deco: DecorationSet; problem: TextProblem | null }>({
			create: (state) => ({ deco: decorations(state, null), problem: null }),
			update(prev, tr) {
				let p = prev.problem;
				for (const e of tr.effects) if (e.is(setProblem)) p = e.value;
				if (!tr.docChanged && p === prev.problem) return prev;
				return { deco: decorations(tr.state, p), problem: p };
			},
			provide: (f) => EditorView.decorations.from(f, (v) => v.deco)
		});
		const submit = () => {
			onsubmit?.();
			return true;
		};
		return [
			history(),
			field,
			autocompletion({ override: [complete], icons: false }),
			hover,
			keymap.of([
				...completionKeymap,
				{ key: 'Enter', run: submit, shift: singleLine ? submit : insertNewline },
				{ key: 'Mod-Enter', run: submit },
				{
					key: 'Mod-.',
					run: () => {
						ontools?.();
						return !!ontools;
					}
				},
				{
					key: 'Escape',
					run: () => {
						oncancel?.();
						return !!oncancel;
					}
				},
				...historyKeymap,
				...defaultKeymap
			]),
			singleLine ? EditorState.transactionFilter.of((tr) => (tr.newDoc.lines > 1 ? [] : tr)) : [],
			EditorView.lineWrapping,
			placeholderExt(placeholder),
			EditorView.contentAttributes.of({
				'aria-label': label,
				'aria-multiline': singleLine ? 'false' : 'true',
				...(describedby ? { 'aria-describedby': describedby } : {}),
				spellcheck: singleLine ? 'false' : 'true'
			}),
			EditorView.updateListener.of((u) => {
				if (u.docChanged) value = u.state.doc.toString();
				if (onselection && (u.selectionSet || u.docChanged || u.focusChanged)) report(u.view);
			})
		];
	}

	onMount(() => {
		view = new EditorView({
			parent: host!,
			state: EditorState.create({ doc: value, extensions: extensions() })
		});
		if (autofocus) {
			view.focus();
			view.dispatch({ selection: { anchor: view.state.doc.length } });
		}
		return () => view?.destroy();
	});

	// A value set from outside replaces the text.
	$effect(() => {
		const v = value;
		if (view && v !== view.state.doc.toString()) {
			view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: v } });
		}
	});

	let lastTarget = 0;
	$effect(() => {
		const t = target;
		if (!view || !t || t.n === lastTarget) return;
		lastTarget = t.n;
		const len = view.state.doc.length;
		view.dispatch({ selection: { anchor: Math.min(t.from, len), head: Math.min(t.to, len) } });
		view.focus();
	});

	$effect(() => {
		const p = problem;
		view?.dispatch({ effects: setProblem.of(p) });
	});
</script>

<!--
	A text editor for fragment text with inline markup, or for a condition.
	Highlights come from the runtime tokenizer, so they always agree with the game.
	Enter saves; in a fragment, Shift+Enter adds a line break. Escape cancels.
-->
<div class="editor" class:single={singleLine} class:has-problem={!!problem} bind:this={host}></div>

<style>
	.editor {
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-control);
		color: var(--color-text);
	}
	.editor:focus-within {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.has-problem {
		border-color: var(--color-warning);
	}
	.editor :global(.cm-editor) {
		font-family: var(--font-prose);
		font-size: inherit;
		line-height: var(--line-height-body);
	}
	.single :global(.cm-editor) {
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.editor :global(.cm-editor.cm-focused) {
		outline: none;
	}
	.editor :global(.cm-content) {
		padding: var(--space-3) var(--space-4);
		caret-color: var(--color-text-accent);
	}
	.editor :global(.cm-line) {
		padding: 0;
	}
	.editor :global(.cm-placeholder) {
		color: var(--color-text-muted);
	}
	.editor :global(.cm-selectionBackground),
	.editor :global(.cm-editor.cm-focused .cm-selectionBackground) {
		background: var(--color-accent-tint);
	}
	.editor :global(.cm-cursor) {
		border-left-color: var(--color-text-accent);
	}
	.editor :global(.cm-tooltip) {
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		color: var(--color-text);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.editor :global(.cm-tooltip-autocomplete ul li[aria-selected]) {
		background: var(--color-accent);
		color: var(--color-text-on-accent);
	}
	.editor :global(.cm-completionDetail) {
		margin-left: var(--space-3);
		font-style: normal;
		color: inherit;
		opacity: 0.8;
	}
	.editor :global(.cm-completionInfo) {
		max-width: 32ch;
		padding: var(--space-2) var(--space-3);
	}
	.editor :global(.symbol-tip) {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		max-width: 36ch;
		padding: var(--space-2) var(--space-3);
	}
	.editor :global(.tok-brace),
	.editor :global(.tok-separator),
	.editor :global(.tok-operator),
	.editor :global(.tok-escape) {
		color: var(--color-text-muted);
		font-family: var(--font-ui);
	}
	.editor :global(.tok-name) {
		color: var(--color-text-accent);
		font-family: var(--font-ui);
	}
	.editor :global(.tok-literal) {
		color: var(--color-text-strong);
		font-family: var(--font-ui);
	}
	.editor :global(.tok-emphasis) {
		font-style: italic;
	}
	.editor :global(.tok-invalid),
	.editor :global(.tok-problem) {
		text-decoration: underline wavy var(--color-warning);
		text-underline-offset: 3px;
	}
</style>
