<script lang="ts">
	import type { RoomMatrix } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';
	import Tag from '../atoms/Tag.svelte';

	interface Props {
		matrix: RoomMatrix;
		/** The name of the room, for the headings. */
		roomName: string;
		/** The full room texts that the whole-game explorer saw in this room. Null before a whole-game run. */
		seen?: string[] | null;
		/** Put the game into one combination. */
		ontry: (values: [string, string][]) => void;
	}

	let { matrix, roomName, seen = null, ontry }: Props = $props();

	interface Group {
		text: string;
		warnings: string[];
		reached: boolean | null;
		rows: string[][];
	}

	/** One group for each different text. Problems first, then texts that no play reached, then the rest. */
	const groups = $derived.by(() => {
		const byText = new Map<string, Group>();
		for (const cell of matrix.cells) {
			let g = byText.get(cell.text);
			if (!g) {
				g = { text: cell.text, warnings: [], reached: seen ? seen.includes(cell.text) : null, rows: [] };
				byText.set(cell.text, g);
			}
			for (const w of cell.warnings) if (!g.warnings.includes(w)) g.warnings.push(w);
			g.rows.push(cell.values);
		}
		const rank = (g: Group) => (g.warnings.length ? 0 : g.reached === false ? 1 : 2);
		return [...byText.values()].sort((a, b) => rank(a) - rank(b));
	});
	const problems = $derived(groups.filter((g) => g.warnings.length).length);
	const unreached = $derived(groups.filter((g) => g.reached === false).length);

	/** The values that are the same in all combinations of a group: the condition for the text. */
	function common(g: Group): [string, string][] {
		return matrix.dimensions.flatMap((d, i): [string, string][] => {
			const values = new Set(g.rows.map((r) => r[i]));
			return values.size === 1 && d.values.length > 1 ? [[d.name, g.rows[0][i]]] : [];
		});
	}

	function pairs(row: string[]): [string, string][] {
		return matrix.dimensions.map((d, i) => [d.name, row[i]]);
	}

	/** `*word*` in story text is emphasis. */
	function pieces(text: string): { text: string; em: boolean }[] {
		return text.split('*').map((t, i) => ({ text: t, em: i % 2 === 1 }));
	}
</script>

{#snippet prose(text: string)}{#each pieces(text) as p, i (i)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}{/snippet}

<!--
	The room matrix: the text of the current room in every combination of the values that the text depends on,
	reachable or not. Each different text shows one time, with the combinations that give it.
-->
<div class="matrix">
	{#if matrix.dimensions.length}
		<div class="depends">
			<span class="muted">The text of {roomName} depends on:</span>
			<ul class="chips" aria-label="Values">
				{#each matrix.dimensions as d (d.name)}<li><Tag><code>{d.name}</code></Tag></li>{/each}
			</ul>
		</div>
	{:else}
		<p class="muted">The text of {roomName} does not depend on a value. It always reads the same.</p>
	{/if}
	<p class="summary" role="status">
		{matrix.combinations}
		{matrix.combinations === 1 ? 'combination' : 'combinations'}, {groups.length}
		{groups.length === 1 ? 'text' : 'different texts'}.
		{problems ? `${problems} with a possible problem.` : 'No problems found.'}
		{#if seen}{unreached
				? `${unreached} not seen in the whole-game run.`
				: 'The whole-game run saw each text.'}{:else}Run "Whole game" to mark texts that a player can reach.{/if}
	</p>
	{#if matrix.combinations > matrix.cells.length}
		<p class="warning">There are too many combinations. The list shows the first {matrix.cells.length}.</p>
	{/if}

	<ol class="groups">
		{#each groups as g, gi (g.text)}
			<li class="group" class:problem={g.warnings.length}>
				<div class="marks">
					{#each g.warnings as w (w)}<Tag tone="warning">{w}</Tag>{/each}
					{#if g.reached === false}<Tag tone="warning">not seen in the whole-game run</Tag>{:else if g.reached}<Tag
							>seen in the whole-game run</Tag
						>{/if}
				</div>
				{#if g.text.trim()}
					<p class="text">{@render prose(g.text)}</p>
				{:else}
					<p class="text muted">(no text)</p>
				{/if}
				{#if matrix.dimensions.length}
					{#if g.rows.length > 1}<p class="when">
						{#if common(g).length}<span class="muted">When</span>
							{#each common(g) as [name, value], i (name)}{i ? ', ' : ''}<code>{name}</code> = {value}{/each}{:else}<span
								class="muted">In combinations with no shared value</span
							>{/if}
					</p>{/if}
					<details class="rows" open={g.rows.length === 1}>
						<summary>{g.rows.length === 1 ? '1 combination' : `${g.rows.length} combinations`}</summary>
						<ul>
							{#each g.rows as row, ri (ri)}
								<li>
									<span class="values"
										>{#each pairs(row) as [name, value], i (name)}{i ? ', ' : ''}<code>{name}</code> = {value}{/each}</span
									>
									<Button size="compact" variant="quiet" onclick={() => ontry(pairs(row))}
										>Try this<span class="visually-hidden"
											>: {pairs(row)
												.map(([n, v]) => `${n} ${v}`)
												.join(', ')}</span
										></Button
									>
								</li>
							{/each}
						</ul>
					</details>
				{/if}
			</li>
		{/each}
	</ol>
</div>

<style>
	.matrix {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}
	p {
		margin: 0;
	}
	.muted {
		color: var(--color-text-muted);
	}
	.summary {
		color: var(--color-text-strong);
	}
	.warning {
		color: var(--color-text-warning);
	}
	.depends {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.groups {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-4) var(--space-5);
		border: var(--size-border) solid var(--color-border-subtle);
	}
	.problem {
		border-color: var(--color-warning);
	}
	.marks {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}
	.marks:empty {
		display: none;
	}
	.text {
		font-family: var(--font-prose);
		line-height: var(--line-height-body);
		white-space: pre-line;
	}
	.when {
		font-size: var(--font-size-sm);
		overflow-wrap: anywhere;
	}
	.rows > summary {
		display: flex;
		align-items: center;
		min-height: var(--size-target-compact);
		cursor: pointer;
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.rows ul {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: var(--space-2) 0 0;
		padding: 0;
		list-style: none;
	}
	.rows li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2) var(--space-4);
		font-size: var(--font-size-sm);
	}
	.values {
		overflow-wrap: anywhere;
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
