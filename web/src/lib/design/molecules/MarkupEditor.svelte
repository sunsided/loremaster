<script lang="ts">
	import { onMount } from 'svelte';
	import { EditorState, RangeSetBuilder, StateEffect, StateField, type Extension } from '@codemirror/state';
	import { Decoration, EditorView, keymap, placeholder as placeholderExt, type DecorationSet } from '@codemirror/view';
	import { defaultKeymap, history, historyKeymap, insertNewline } from '@codemirror/commands';
	import type { MarkupToken, TextProblem } from '../types.ts';

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
		describedby
	}: Props = $props();

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
			keymap.of([
				{ key: 'Enter', run: submit, shift: singleLine ? submit : insertNewline },
				{ key: 'Mod-Enter', run: submit },
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
