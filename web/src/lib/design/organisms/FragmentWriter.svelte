<script lang="ts">
	import { untrack } from 'svelte';
	import Button from '../atoms/Button.svelte';
	import Checkbox from '../atoms/Checkbox.svelte';
	import Select from '../atoms/Select.svelte';
	import Tag from '../atoms/Tag.svelte';
	import MarkupEditor from '../molecules/MarkupEditor.svelte';
	import type { WriterFragment, WriterTools } from '../types.ts';

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
	}

	let { fragment, variant = 0, title, tools, fresh = false, onsave, oncancel, onremove }: Props = $props();

	// The draft starts as a copy of the fragment and is not changed from outside.
	const start = untrack(() => $state.snapshot(fragment));
	let variants = $state([...start.variants]);
	let current = $state(untrack(() => Math.min(variant, start.variants.length - 1)));
	let when = $state(start.when ?? '');
	let layer = $state(start.layer);
	let mode = $state(start.mode);
	let replaces = $state(start.replaces);
	let saving = $state(false);
	let saveProblem = $state<string | null>(null);

	const text = $derived(variants[current] ?? '');
	const check = $derived(tools.tryMarkup(text));
	const condition = $derived(tools.tryCondition(when));
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

	/** `*word*` in story text is emphasis. */
	function emphasis(text: string): { text: string; em: boolean }[] {
		return text.split('*').map((t, i) => ({ text: t, em: i % 2 === 1 }));
	}

	let section: HTMLElement | undefined = $state();
	$effect(() => {
		section?.scrollIntoView({ block: 'nearest' });
	});

	async function save() {
		if (blocked) return;
		saving = true;
		saveProblem = await onsave({ variants: [...variants], when: when.trim() || null, layer, mode, replaces });
		saving = false;
	}

	function addVariant() {
		variants.push('');
		current = variants.length - 1;
		if (mode === 'always' || mode === 'first_visit' || mode === 'once') mode = 'cycle';
	}

	function removeVariant() {
		if (variants.length < 2) return;
		variants.splice(current, 1);
		current = Math.min(current, variants.length - 1);
	}
</script>

<!--
	The fragment writer opens in the story text, in place of the fragment.
	The author types the text with its markup and sees at once how it reads now.
	Enter saves, Shift+Enter adds a line break, Escape cancels.
-->
<section class="writer" aria-labelledby="{id}-title" bind:this={section}>
	<div class="head">
		<h2 id="{id}-title">{title}</h2>
		<Select label="Layer" bind:value={layer} options={layers} />
		<Select label="Shows" bind:value={mode} options={modes} />
		<Checkbox bind:checked={replaces}>replaces earlier text in its layer</Checkbox>
	</div>

	<div class="when">
		<span class="label" id="{id}-when-label">When</span>
		<div class="when-editor">
			<MarkupEditor
				bind:value={when}
				label="Condition: the fragment shows only when it holds. Empty means always."
				tokenize={(t) => tools.tokenize(t, true)}
				problem={condition.problem}
				placeholder="always"
				singleLine
				onsubmit={save}
				{oncancel}
				describedby="{id}-when-state"
			/>
		</div>
		<span id="{id}-when-state">
			{#if condition.problem}<Tag tone="warning">{condition.problem.message}</Tag>{:else if condition.holds === true}<Tag
					>holds now</Tag
				>{:else if condition.holds === false}<Tag tone="warning">does not hold now</Tag>{/if}
		</span>
	</div>

	{#if variants.length > 1}
		<div class="variants" role="group" aria-label="Variants">
			{#each variants as _, i (i)}
				<Button size="compact" variant="quiet" pressed={i === current} onclick={() => (current = i)}>Variant {i + 1}</Button>
			{/each}
			<Button size="compact" variant="quiet" onclick={removeVariant}>Remove this variant</Button>
		</div>
	{/if}

	{#key current}
		<div class="text">
		<MarkupEditor
			bind:value={variants[current]}
			label="Text{variants.length > 1 ? `, variant ${current + 1}` : ''}"
			tokenize={(t) => tools.tokenize(t, false)}
			problem={check.problem}
			placeholder="Write what the player reads…"
			onsubmit={save}
			{oncancel}
			autofocus
			describedby="{id}-reads"
		/>
		</div>
	{/key}

	<div class="reads" id="{id}-reads" aria-live="polite">
		{#if check.problem}
			<p class="problem">{check.problem.message}</p>
		{:else if text.trim()}
			<p><span class="label">Now it reads</span> {#each emphasis(check.text) as p, i (i)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}</p>
		{/if}
		{#if saveProblem}<p class="problem" role="alert">{saveProblem}</p>{/if}
	</div>

	<div class="actions">
		<Button variant="primary" size="compact" onclick={save} disabled={blocked}>{fresh ? 'Add' : 'Save'}</Button>
		<Button size="compact" onclick={oncancel}>Cancel</Button>
		<Button size="compact" variant="quiet" onclick={addVariant}>Add a variant</Button>
		{#if onremove && !fresh}<Button size="compact" variant="quiet" onclick={onremove}>Remove this fragment</Button>{/if}
		<span class="keys" aria-hidden="true">Enter saves · Shift+Enter new line · Esc cancels</span>
	</div>
</section>

<style>
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
	.keys {
		margin-left: auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
