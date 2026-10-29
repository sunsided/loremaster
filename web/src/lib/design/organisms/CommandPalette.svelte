<script lang="ts">
	import type { PaletteItem } from '../types.ts';

	interface Props {
		open?: boolean;
		items: PaletteItem[];
		/** Makes an item from the search text, for example "Do “open drawer”". Return null for no item. */
		fallback?: (query: string) => PaletteItem | null;
		placeholder?: string;
	}

	let {
		open = $bindable(false),
		items,
		fallback,
		placeholder = 'Search actions, rooms, rules and states'
	}: Props = $props();

	let dialog: HTMLDialogElement | undefined = $state();
	let input: HTMLInputElement | undefined = $state();
	let list: HTMLUListElement | undefined = $state();
	let query = $state('');
	let active = $state(0);

	// A modal dialog keeps focus inside, closes on Escape and gives focus back to the element that had it.
	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) {
			query = '';
			active = 0;
			dialog.showModal();
			input?.focus();
		}
		if (!open && dialog.open) dialog.close();
	});

	/**
	 * Match: every word of the query must be in the text. Lower is better; null means no match.
	 * A word at the start of a word in the text counts more than one inside a word.
	 */
	function score(text: string, q: string): number | null {
		const t = text.toLowerCase();
		let total = 0;
		for (const word of q.split(/\s+/).filter(Boolean)) {
			const at = t.indexOf(word);
			if (at < 0) return null;
			const startOfWord = at === 0 || /[^a-z0-9]/.test(t[at - 1]);
			total += at + (startOfWord ? 0 : 50);
		}
		return total;
	}

	const results = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const found = q
			? items
					.map((item) => ({ item, s: score(`${item.label} ${item.group} ${item.keywords ?? ''}`, q) }))
					.filter((r) => r.s !== null)
					.sort((a, b) => (a.s ?? 0) - (b.s ?? 0))
					.map((r) => r.item)
			: items;
		// The fallback goes last, so Enter runs the best match. With no match it is the only item.
		const extra = q && fallback ? fallback(query.trim()) : null;
		return extra ? [...found, extra] : found;
	});

	/** The results in groups, in the order the groups first appear. */
	const groups = $derived.by(() => {
		const out: { name: string; items: { item: PaletteItem; index: number }[] }[] = [];
		results.forEach((item, index) => {
			let g = out.find((x) => x.name === item.group);
			if (!g) out.push((g = { name: item.group, items: [] }));
			g.items.push({ item, index });
		});
		return out;
	});

	$effect(() => {
		query;
		active = 0;
	});

	$effect(() => {
		list?.querySelector(`#palette-option-${active}`)?.scrollIntoView({ block: 'nearest' });
	});

	function run(item: PaletteItem | undefined) {
		if (!item) return;
		open = false;
		// Run after the dialog has closed and given focus back, so the action can move focus itself.
		setTimeout(item.run, 0);
	}

	function onkeydown(event: KeyboardEvent) {
		const last = results.length - 1;
		if (event.key === 'ArrowDown') active = active >= last ? 0 : active + 1;
		else if (event.key === 'ArrowUp') active = active <= 0 ? last : active - 1;
		else if (event.key === 'Home' && event.ctrlKey) active = 0;
		else if (event.key === 'End' && event.ctrlKey) active = last;
		else if (event.key === 'Enter') run(results[active]);
		else return;
		event.preventDefault();
	}
</script>

<!--
	The command palette: one search field and a list of actions.
	It follows the WAI-ARIA combobox pattern: focus stays in the search field,
	the arrow keys move the active option, Enter runs it, Escape closes the palette.
-->
<dialog
	bind:this={dialog}
	class="palette"
	aria-label="Command palette"
	onclose={() => (open = false)}
	onclick={(e) => e.target === dialog && (open = false)}
>
	<div class="search">
		<span class="caret" aria-hidden="true">⌘</span>
		<input
			bind:this={input}
			bind:value={query}
			{onkeydown}
			{placeholder}
			role="combobox"
			aria-label="Search the command palette"
			aria-expanded="true"
			aria-controls="palette-list"
			aria-autocomplete="list"
			aria-activedescendant={results.length ? `palette-option-${active}` : undefined}
			autocomplete="off"
			spellcheck="false"
		/>
	</div>
	<ul id="palette-list" role="listbox" aria-label="Actions" bind:this={list}>
		{#each groups as g (g.name)}
			<li role="presentation" class="group" aria-hidden="true">{g.name}</li>
			{#each g.items as { item, index } (item.id)}
				<li
					id="palette-option-{index}"
					role="option"
					aria-selected={index === active}
					class="option"
					onmousemove={() => (active = index)}
					onclick={() => run(item)}
					onkeydown={() => {}}
				>
					<span class="label">{item.label}</span>
					{#if item.hint}<span class="hint">{item.hint}</span>{/if}
					<span class="visually-hidden">, {g.name}</span>
				</li>
			{/each}
		{:else}
			<li class="empty" role="presentation">Nothing matches.</li>
		{/each}
	</ul>
	<p class="keys" aria-hidden="true">↑ ↓ to move · Enter to run · Esc to close</p>
</dialog>

<style>
	.palette {
		box-sizing: border-box;
		width: min(640px, calc(100vw - var(--space-9)));
		max-height: min(560px, calc(100dvh - var(--space-10)));
		margin-top: 12vh;
		padding: 0;
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		color: var(--color-text);
		box-shadow: var(--shadow-overlay);
	}
	.palette[open] {
		display: flex;
		flex-direction: column;
	}
	.palette::backdrop {
		background: rgb(0 0 0 / 45%);
	}
	.search {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-4) var(--space-6);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
	}
	.caret {
		color: var(--color-text-muted);
	}
	input {
		flex: 1 1 auto;
		min-width: 0;
		min-height: var(--size-target);
		border: 0;
		background: transparent;
		color: var(--color-text-strong);
		caret-color: var(--color-accent);
		font-size: var(--font-size-lg);
	}
	input:focus-visible {
		outline: none;
	}
	ul {
		position: relative;
		flex: 1 1 auto;
		min-height: 0;
		margin: 0;
		padding: var(--space-3) 0;
		overflow-y: auto;
		list-style: none;
	}
	.group {
		padding: var(--space-4) var(--space-6) var(--space-2);
		font-size: var(--font-size-xs);
		font-weight: var(--font-weight-strong);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
		color: var(--color-text-muted);
	}
	.option {
		display: flex;
		justify-content: space-between;
		gap: var(--space-5);
		min-height: var(--size-target-compact);
		align-items: center;
		padding: var(--space-2) var(--space-6);
		cursor: pointer;
		border-left: var(--size-border-strong) solid transparent;
	}
	.option[aria-selected='true'] {
		border-left-color: var(--color-accent);
		background: var(--color-accent-tint);
		color: var(--color-text-strong);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		white-space: nowrap;
	}
	.empty {
		padding: var(--space-5) var(--space-6);
		color: var(--color-text-muted);
	}
	.keys {
		margin: 0;
		padding: var(--space-3) var(--space-6);
		border-top: var(--size-border) solid var(--color-border-subtle);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
