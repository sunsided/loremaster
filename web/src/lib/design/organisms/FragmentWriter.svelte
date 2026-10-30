<script lang="ts">
	import type { Snippet } from 'svelte';
	import { untrack } from 'svelte';
	import Button from '../atoms/Button.svelte';
	import Checkbox from '../atoms/Checkbox.svelte';
	import Select from '../atoms/Select.svelte';
	import RadioGroup from '../molecules/RadioGroup.svelte';
	import Tag from '../atoms/Tag.svelte';
	import MarkupEditor from '../molecules/MarkupEditor.svelte';
	import SelectionTools from '../molecules/SelectionTools.svelte';
	import { FragmentDraft } from '../fragment-draft.svelte.ts';
	import { inMarkup } from '#lib/project/transform.ts';
	import type { TextProblem, WriterFragment, WriterTools } from '../types.ts';

	interface Props {
		/** The fragment as it is in the file. */
		fragment: WriterFragment;
		/** The variant to edit first, for cycling and random fragments. */
		variant?: number;
		/** What the author edits, for example "Room text of The Shore, fragment 2". */
		title: string;
		tools: WriterTools;
		/** True for a fragment that is not in the file yet. */
		fresh?: boolean;
		/** Save the fragment. Returns a problem text when the story does not build with it. */
		onsave: (fragment: WriterFragment) => Promise<string | null>;
		oncancel: () => void;
		onremove?: () => void;
		/** `fragment`: a text with layer, mode and variants. `response`: one text and a condition, for a rule. */
		kind?: 'fragment' | 'response';
		/** Create a thing for a name that the story does not know. When set, the writer offers it. */
		oncreate?: (id: string, property: string | null) => void;
		/**
		 * The draft to edit. Give the same draft to a prose view in the game text, and both views edit one text.
		 * Without it, the writer makes its own draft from `fragment`.
		 */
		draft?: FragmentDraft;
		/** "Make it a thing" for selected words. When set, the selection toolbar offers it. */
		onthing?: (name: string) => void;
		/**
		 * True when the text is edited in the game text (a prose view with the same draft). Then the writer is a
		 * slim fragment panel: properties, and how the text reads in other states. The markup is one key away.
		 */
		prose?: boolean;
		/** The author notes of this fragment, for example a NotesList and the form that ties it to a thread. */
		notes?: Snippet;
	}

	let {
		fragment,
		variant = 0,
		title,
		tools,
		fresh = false,
		onsave,
		oncancel,
		onremove,
		kind = 'fragment',
		oncreate,
		draft: given,
		onthing,
		prose = false,
		notes
	}: Props = $props();



	// The draft starts as a copy of the fragment and is not changed from outside.
	const draft = untrack(() => given ?? new FragmentDraft($state.snapshot(fragment), variant));
	let saving = $state(false);
	let saveProblem = $state<string | null>(null);

	const text = $derived(draft.text);
	const check = $derived(tools.tryMarkup(text));
	const condition = $derived(tools.tryCondition(draft.when));
	const blocked = $derived(!!check.problem || !!condition.problem || saving);
	const id = $props.id();

	const layers = ['base', 'ambient', 'event', 'objects', 'characters', 'exits'].map((v) => ({ value: v as WriterFragment['layer'], label: v }));
	const modes = [
		{ value: 'always' as const, label: 'always' },
		{ value: 'first_visit' as const, label: 'first visit' },
		{ value: 'once' as const, label: 'once' },
		{ value: 'cycle' as const, label: 'cycle variants' },
		{ value: 'shuffle' as const, label: 'shuffle variants' },
		{ value: 'sticky' as const, label: 'sticky variant' }
	];

	/**
	 * A better name for the name at a problem: the hint of the runtime ("did you mean `x`?"),
	 * else a known name that ends with the typed name (`lamp.lit` to `oil_lamp.lit`).
	 */
	function fixFor(source: string, problem: TextProblem | null): { from: number; to: number; name: string } | null {
		if (!problem) return null;
		const typed = source.slice(problem.from, problem.to);
		const hint = /did you mean `([^`]+)`/.exec(problem.message)?.[1];
		let name = hint;
		if (name && !name.includes('.') && typed.includes('.')) name = `${name}${typed.slice(typed.indexOf('.'))}`;
		if (!name) name = tools.symbols.find((s) => typed.length > 2 && (s.name.endsWith(`_${typed}`) || s.name.endsWith(`.${typed}`)))?.name;
		return name && name !== typed ? { from: problem.from, to: problem.to, name } : null;
	}
	const textFix = $derived(fixFor(text, check.problem));

	/** A name to create: `brass_key` from "unknown name `brass_key.the`", with the property when there is one. */
	function newName(problem: TextProblem | null): { id: string; property: string | null } | null {
		const m = problem && /^unknown name `([a-z_][\w]*)(?:\.(\w+))?`/.exec(problem.message);
		if (!m) return null;
		const forms = ['name', 'the', 'The', 'a', 'A'];
		return { id: m[1], property: m[2] && !forms.includes(m[2]) ? m[2] : null };
	}
	const textNew = $derived(oncreate ? newName(check.problem) : null);
	const whenNew = $derived(oncreate ? newName(condition.problem) : null);
	const whenFix = $derived(fixFor(draft.when, condition.problem));

	function applyFix(fix: { from: number; to: number; name: string }, inWhen: boolean) {
		if (inWhen) draft.when = draft.when.slice(0, fix.from) + fix.name + draft.when.slice(fix.to);
		else draft.text = text.slice(0, fix.from) + fix.name + text.slice(fix.to);
	}

	/** The names of things as they read now, for "Use the name". */
	const names = $derived(
		tools.symbols.filter((s) => s.kind === 'name' && s.value).map((s) => ({ name: s.name, value: s.value as string }))
	);
	/** The actions for the selected text. */
	const actions = $derived.by(() => {
		const sel = draft.selection;
		if (!sel) return [];
		const words = text.slice(sel.from, sel.to).trim();
		const out: { id: string; label: string; run: () => void }[] = [];
		const named = draft.nameFor(names);
		if (named) out.push({ id: 'name', label: 'Use the name', run: () => draft.useName(names) });
		else if (onthing && /^[\p{L}][\p{L}\p{N} '-]{0,40}$/u.test(words))
			out.push({ id: 'thing', label: 'Make it a thing', run: () => (onthing(words), (draft.selection = null)) });
		out.push({ id: 'first', label: 'Only the first time', run: () => draft.onlyFirst() });
		out.push({ id: 'when', label: 'Only when…', run: () => draft.onlyWhen() });
		if (kind === 'fragment' && draft.split) out.push({ id: 'variants', label: 'Make variants', run: () => draft.makeVariants() });
		return out;
	});

	/** Report a selection from the source view. */
	function selectSource(from: number, to: number, rect: { left: number; top: number; bottom: number } | null) {
		draft.selection = rect && !inMarkup(text, from, to) ? { view: 'source', from, to, rect } : null;
	}

	/** The text in each combination of the values that it depends on, one entry for each different text. */
	let showOther = $state(untrack(() => prose));
	const other = $derived.by(() => {
		if (!(showOther || prose) || check.problem || condition.problem || !tools.textMatrix) return null;
		const m = tools.textMatrix(text, draft.when.trim() || null);
		if (!m) return null;
		const groups = new Map<string, string[][]>();
		for (const c of m.cells) groups.set(c.text, [...(groups.get(c.text) ?? []), c.values]);
		return {
			names: m.dimensions.map((d) => d.name),
			total: m.combinations,
			shown: m.cells.length,
			groups: [...groups.entries()].map(([t, rows]) => ({
				text: t,
				// The values that are the same in all combinations of this text.
				when: m.dimensions
					.map((d, i) => ({ name: d.name, values: new Set(rows.map((r) => r[i])), all: d.values.length }))
					.filter((d) => d.values.size === 1 && d.all > 1)
					.map((d) => `${d.name} = ${[...d.values][0]}`),
				count: rows.length
			}))
		};
	});

	/**
	 * The part of a text that differs from the text now, with "…" for the rest: for the slim panel, where the
	 * game text already shows the whole text. The cut is at word boundaries.
	 */
	function difference(other: string, now: string): string {
		if (!prose || !now || other === now) return other;
		let a = 0;
		while (a < other.length && a < now.length && other[a] === now[a]) a++;
		let b = 0;
		while (b < other.length - a && b < now.length - a && other[other.length - 1 - b] === now[now.length - 1 - b]) b++;
		let start = other.lastIndexOf(' ', a - 1) + 1;
		let end = other.indexOf(' ', other.length - b);
		if (end < 0) end = other.length;
		start = Math.max(0, start);
		return `${start > 0 ? '…' : ''}${other.slice(start, end).trim()}${end < other.length ? '…' : ''}`;
	}

	/** `*word*` in story text is emphasis. */
	function emphasis(text: string): { text: string; em: boolean }[] {
		return text.split('*').map((t, i) => ({ text: t, em: i % 2 === 1 }));
	}

	let section: HTMLElement | undefined = $state();
	$effect(() => {
		// With a prose view the author types in the game text, so the panel does not scroll the page.
		if (!prose) section?.scrollIntoView({ block: 'nearest' });
	});

	/** Save the draft. The prose view in the game text calls it on Enter too. */
	export async function save() {
		if (blocked) return;
		saving = true;
		saveProblem = await onsave(draft.snapshot());
		saving = false;
	}
</script>

<!--
	The fragment writer opens in the story text, in place of the fragment.
	The author types the text with its markup and sees at once how it reads now.
	Enter saves, Shift+Enter adds a line break, Escape cancels.
-->
{#if draft.selection && actions.length}
	<SelectionTools rect={draft.selection.rect} {actions} focus={draft.toolsFocus} ondismiss={() => (draft.selection = null)} onclose={() => (draft.target = { ...draft.selection!, view: draft.selection!.view, n: (draft.target?.n ?? 0) + 1 })} />
{/if}

<section class="writer" aria-labelledby="{id}-title" bind:this={section}>
	<div class="head">
		<h2 id="{id}-title">{title}</h2>
		{#if kind === 'fragment'}
			<Select label="Layer" bind:value={draft.layer} options={layers} />
			{#if prose}
				<RadioGroup legend="Shows" name="{id}-mode" look="pills" options={modes} bind:value={draft.mode} />
			{:else}
				<Select label="Shows" bind:value={draft.mode} options={modes} />
			{/if}
			<Checkbox bind:checked={draft.replaces}>replaces earlier text in its layer</Checkbox>
		{/if}
	</div>

	<div class="when">
		<span class="label" id="{id}-when-label">When</span>
		<div class="when-editor">
			<MarkupEditor
				bind:value={draft.when}
				label={kind === 'fragment'
					? 'Condition: the fragment shows only when it holds. Empty means always.'
					: 'Condition: the response applies only when it holds. Empty means always.'}
				tokenize={(t) => tools.tokenize(t, true)}
				problem={condition.problem}
				placeholder="always"
				singleLine
				symbols={tools.symbols}
				onsubmit={save}
				{oncancel}
				describedby="{id}-when-state"
			/>
		</div>
		<span id="{id}-when-state" class="when-state">
			{#if condition.problem}<Tag tone="warning">{condition.problem.message}</Tag>{#if whenFix}<Button
						size="compact"
						variant="quiet"
						onclick={() => applyFix(whenFix, true)}>Use {whenFix.name}</Button
					>{:else if whenNew}<Button size="compact" variant="quiet" onclick={() => oncreate?.(whenNew.id, whenNew.property)}
						>Create thing {whenNew.id}</Button
					>{/if}{:else if condition.holds === true}<Tag>holds now</Tag>{:else if condition.holds === false}<Tag
					tone="warning">does not hold now</Tag
				>{/if}
		</span>
	</div>

	{#if draft.variants.length > 1}
		<div class="variants" role="group" aria-label="Variants">
			{#each draft.variants as _, i (i)}
				<Button size="compact" variant="quiet" pressed={i === draft.current} onclick={() => (draft.current = i)}>Variant {i + 1}</Button>
			{/each}
			<Button size="compact" variant="quiet" onclick={() => draft.removeVariant()}>Remove this variant</Button>
		</div>
	{/if}

	{#if prose}
		<div class="markup-toggle">
			<Button size="compact" variant="quiet" pressed={draft.showMarkup} aria-keyshortcuts="Control+/" onclick={() => (draft.showMarkup = !draft.showMarkup)}
				>&lt;/&gt; {draft.showMarkup ? 'Hide' : 'Show'} markup</Button
			>
		</div>
	{/if}
	{#key draft.current}
		<div class="text" hidden={prose && !draft.showMarkup}>
		<MarkupEditor
			bind:value={draft.variants[draft.current]}
			label="Text, with markup{draft.variants.length > 1 ? `, variant ${draft.current + 1}` : ''}"
			tokenize={(t) => tools.tokenize(t, false)}
			problem={check.problem}
			placeholder="Write what the player reads…"
			symbols={tools.symbols}
			onsubmit={save}
			{oncancel}
			autofocus={!given}
			describedby="{id}-reads"
			onselection={selectSource}
			ontools={() => draft.toolsFocus++}
			target={draft.target?.view === 'source' ? draft.target : null}
		/>
		</div>
	{/key}

	<div class="reads" id="{id}-reads" aria-live="polite">
		{#if check.problem}
			<p class="problem">
				{check.problem.message}
				{#if textFix}<Button size="compact" variant="quiet" onclick={() => applyFix(textFix, false)}>Use {textFix.name}</Button
					>{:else if textNew}<Button size="compact" variant="quiet" onclick={() => oncreate?.(textNew.id, textNew.property)}
						>Create thing {textNew.id}</Button
					>{/if}
			</p>
		{:else if text.trim() && !prose}
			<p><span class="label">Now it reads</span> {#each emphasis(check.text) as p, i (i)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}</p>
		{/if}
		{#if saveProblem}<p class="problem" role="alert">{saveProblem}</p>{/if}
	</div>

	{#if tools.textMatrix}
		<details class="other" bind:open={showOther}>
			<summary>Reads in other states</summary>
			{#if other}
				{#if other.names.length === 0}
					<p class="muted">The text does not depend on a value. It always reads the same.</p>
				{:else}
					<p class="muted">
						{other.groups.length}
						{other.groups.length === 1 ? 'text' : 'different texts'} for {other.total}
						{other.total === 1 ? 'combination' : 'combinations'} of {other.names.join(', ')}{other.total > other.shown
							? ` (the first ${other.shown})`
							: ''}.
					</p>
					<ul>
						{#each other.groups as g, i (i)}
							<li>
								<span class="muted">{g.when.length ? `When ${g.when.join(', ')}` : `In ${g.count} combinations`}:</span>
								{#if g.text}{#each emphasis(difference(g.text, check.text)) as p, j (j)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}{:else}<span
										class="muted">(does not show)</span
									>{/if}
							</li>
						{/each}
					</ul>
				{/if}
			{/if}
		</details>
	{/if}

	<div class="actions">
		<Button variant="primary" size="compact" onclick={save} disabled={blocked}>{fresh ? 'Add' : 'Save'}</Button>
		<Button size="compact" onclick={oncancel}>Cancel</Button>
		{#if kind === 'fragment'}<Button size="compact" variant="quiet" onclick={() => draft.addVariant()}>Add a variant</Button>{/if}
		{#if onremove && !fresh}<Button size="compact" variant="quiet" onclick={onremove}>Remove this fragment</Button>{/if}
		<span class="keys" aria-hidden="true">Enter saves · Shift+Enter new line · Esc cancels</span>
	</div>
	{#if notes && !fresh}
		<details class="notes">
			<summary>Notes and threads of this fragment</summary>
			{@render notes()}
		</details>
	{/if}
</section>

<style>
	.notes {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.notes summary {
		min-height: var(--size-target-compact);
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.writer {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		margin: var(--space-3) 0;
		padding: var(--space-4) var(--space-5);
		border: var(--size-border) solid var(--color-accent);
		background: var(--color-surface-raised);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		text-shadow: none;
	}
	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3) var(--space-5);
	}
	h2 {
		margin: 0;
		flex: 1 1 100%;
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-strong);
		color: var(--color-text-accent);
	}
	.when {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: center;
		gap: var(--space-2) var(--space-4);
	}
	.when > :last-child {
		grid-column: 2;
	}
	.when > :last-child:empty {
		display: none;
	}
	.when-editor {
		min-width: 0;
	}
	.label {
		color: var(--color-text-muted);
	}
	.variants,
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
	}
	.text {
		font-size: var(--font-size-prose);
	}
	.text[hidden] {
		display: none;
	}
	.reads {
		font-size: var(--font-size-prose);
		line-height: var(--line-height-body);
	}
	.reads p {
		margin: 0;
	}
	.reads .label {
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.problem {
		color: var(--color-text-warning);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.when-state {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
	}
	.other > summary {
		display: flex;
		align-items: center;
		min-height: var(--size-target-compact);
		cursor: pointer;
		color: var(--color-text-muted);
	}
	.other ul {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin: var(--space-2) 0 0;
		padding: 0;
		list-style: none;
		font-family: var(--font-prose);
		font-size: var(--font-size-prose);
		line-height: var(--line-height-body);
	}
	.other .muted,
	.muted {
		margin: 0;
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.keys {
		margin-left: auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
