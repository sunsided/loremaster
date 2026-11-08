<script lang="ts">
	import type { ChartStep, StepChart } from '#lib/engine/types.ts';
	import Heading from '../atoms/Heading.svelte';

	interface Props {
		charts: StepChart[];
		/** How many steps in a row with width 1 count as "in series only". */
		serialRun?: number;
	}

	let { charts, serialRun = 5 }: Props = $props();
	const uid = $props.id();

	/** Width of one depth column and the size of a step box, in SVG units. */
	const COL = 170;
	const ROW = 50;
	const BOX_W = 132;
	const BOX_H = 32;
	const PAD = 16;
	/** The strip of widths under the chart. */
	const BAR_W = 14;
	const BAR_GAP = 4;
	const BAR_UNIT = 10;

	/** The step under the pointer: its needs and the steps that need it light up. The list under the drawing says the same for keyboard and touch. */
	let lit = $state<{ chart: string; step: string } | null>(null);

	function layout(c: StepChart) {
		const rows = new Map<number, number>();
		const pos = new Map<string, { x: number; y: number }>();
		for (const s of c.steps) {
			const row = rows.get(s.depth) ?? 0;
			rows.set(s.depth, row + 1);
			pos.set(s.id, { x: PAD + s.depth * COL, y: PAD + row * ROW });
		}
		const depth = Math.max(0, ...c.steps.map((s) => s.depth));
		const height = PAD * 2 + (Math.max(1, ...rows.values()) - 1) * ROW + BOX_H;
		const edges = c.steps.flatMap((s) =>
			s.needs.flatMap((n) => {
				const a = pos.get(n);
				const b = pos.get(s.id);
				if (!a || !b) return [];
				const x1 = a.x + BOX_W;
				const y1 = a.y + BOX_H / 2;
				const x2 = b.x;
				const y2 = b.y + BOX_H / 2;
				const mid = (x1 + x2) / 2;
				return [{ from: n, to: s.id, path: `M ${x1} ${y1} C ${mid} ${y1}, ${mid} ${y2}, ${x2} ${y2}` }];
			})
		);
		return { width: PAD * 2 + depth * COL + BOX_W, height, pos, edges };
	}

	/** The steps of each run of at least `serialRun` steps with width 1. */
	function serial(c: StepChart): Set<string> {
		const out = new Set<string>();
		let run: string[] = [];
		for (const s of [...c.steps, null]) {
			if (s && s.width === 1) {
				run.push(s.id);
				continue;
			}
			if (run.length >= serialRun) run.forEach((id) => out.add(id));
			run = [];
		}
		return out;
	}

	function on(c: StepChart, s: ChartStep): boolean {
		if (lit?.chart !== c.walkthrough) return false;
		if (lit.step === s.id) return true;
		const at = c.steps.find((x) => x.id === lit?.step);
		return !!at?.needs.includes(s.id) || s.needs.includes(lit.step);
	}

	function edgeOn(c: StepChart, from: string, to: string): boolean {
		return lit?.chart === c.walkthrough && (lit.step === from || lit.step === to);
	}

	const point = (c: StepChart, s: ChartStep) => (lit = { chart: c.walkthrough, step: s.id });
	const clear = () => (lit = null);
</script>

<!--
	The dependency charts of the walkthroughs, for the Atlas (CK-7, US-41, US-43). Each walkthrough with step ids
	gets a drawing: one column for each depth of needs, a box for each step, and an arrow from each step to the
	steps that need it. A strip of bars shows the puzzle width: how many steps the player can work on when each
	step comes up. A long run of width 1 is marked: there the game is in series only. The list under the drawing
	says the same in words.
-->
<section class="chart-view" aria-labelledby="{uid}-title">
	<Heading level={2} id="{uid}-title">Steps</Heading>

	{#each charts as c, ci (c.walkthrough)}
		{@const g = layout(c)}
		{@const series = serial(c)}
		<details class="group" open>
			<summary>
				<span class="group-name">{c.name}</span>
				<span class="count">{c.steps.length} steps</span>
				<span class="count">width at most {c.max_width}</span>
				{#if series.size}<span class="warn">{series.size} steps in series only</span>{/if}
			</summary>
			<div class="graph">
				<svg
					class:pointing={lit?.chart === c.walkthrough}
					viewBox="0 0 {g.width} {g.height}"
					width={g.width}
					height={g.height}
					role="img"
					aria-label="{c.name}: {c.steps.length} steps, width at most {c.max_width}"
				>
					<defs>
						<marker id="{uid}-need-{ci}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto">
							<path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke" />
						</marker>
					</defs>
					{#each g.edges as e (e.from + '>' + e.to)}
						<path class="edge" class:lit={edgeOn(c, e.from, e.to)} d={e.path} marker-end="url(#{uid}-need-{ci})" />
					{/each}
					{#each c.steps as s (s.id)}
						{@const p = g.pos.get(s.id)}
						{#if p}
							<g
								class="step"
								class:lit={on(c, s)}
								class:series={series.has(s.id)}
								role="presentation"
								onpointerenter={() => point(c, s)}
								onpointerleave={clear}
							>
								<rect x={p.x} y={p.y} width={BOX_W} height={BOX_H} />
								<text x={p.x + BOX_W / 2} y={p.y + BOX_H / 2}>{s.id}{s.hints ? ' ?' : ''}</text>
							</g>
						{/if}
					{/each}
				</svg>
			</div>
			<div class="widths" aria-hidden="true">
				<svg
					viewBox="0 0 {c.steps.length * (BAR_W + BAR_GAP)} {c.max_width * BAR_UNIT}"
					width={c.steps.length * (BAR_W + BAR_GAP)}
					height={c.max_width * BAR_UNIT}
				>
					{#each c.steps as s, i (s.id)}
						<rect
							class="bar"
							class:series={series.has(s.id)}
							class:lit={on(c, s)}
							x={i * (BAR_W + BAR_GAP)}
							y={(c.max_width - s.width) * BAR_UNIT}
							width={BAR_W}
							height={s.width * BAR_UNIT}
						/>
					{/each}
				</svg>
				<span class="legend">width of each step, in walkthrough order</span>
			</div>
			<ol class="steps">
				{#each c.steps as s (s.id)}
					<li class:lit={on(c, s)} class:series={series.has(s.id)}>
						<span class="id">{s.id}</span>
						<code>{s.command}</code>
						<span class="detail">
							step {s.index + 1}{#if s.at}, at {s.at}{/if}{#if s.needs.length}, needs {s.needs.join(', ')}{/if}, width {s.width}{#if s.hints}, {s.hints}
								{s.hints === 1 ? 'hint' : 'hints'}{/if}
						</span>
					</li>
				{/each}
			</ol>
		</details>
	{:else}
		<p class="empty">
			No walkthrough has step ids yet. Give the steps of a walkthrough in <code>tests/</code> an <code>id</code>, and
			<code>needs</code> for the steps that must come first: <code>{'{ id: key, do: take key, needs: drawer }'}</code>.
			The chart and the hint ladder come from them.
		</p>
	{/each}
</section>

<style>
	.chart-view {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	summary {
		list-style: none;
	}
	summary::-webkit-details-marker {
		display: none;
	}
	.group > summary {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-4);
		min-height: var(--size-target-compact);
		cursor: pointer;
	}
	.group > summary::before {
		content: '▸';
		color: var(--color-text-muted);
		transition: transform var(--duration-fast);
	}
	.group[open] > summary::before {
		transform: rotate(90deg);
	}
	.group-name {
		font-weight: var(--font-weight-strong);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
	}
	.count,
	.warn {
		padding: 0 var(--space-3);
		border: var(--size-border) solid var(--color-border-subtle);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.warn {
		border-color: var(--color-warning);
		color: var(--color-text-warning);
	}
	.graph,
	.widths {
		max-width: 100%;
		margin-top: var(--space-3);
		overflow-x: auto;
	}
	.graph {
		padding: var(--space-4);
		border: var(--size-border) solid var(--color-border-subtle);
	}
	.widths {
		display: flex;
		align-items: flex-end;
		gap: var(--space-4);
	}
	svg {
		display: block;
		max-width: none;
	}
	svg.pointing .edge:not(.lit),
	svg.pointing .step:not(.lit) {
		opacity: 0.3;
	}
	.edge,
	.step {
		transition: opacity var(--duration-fast);
	}
	@media (prefers-reduced-motion: reduce) {
		.edge,
		.step,
		.group > summary::before {
			transition: none;
		}
	}
	.edge {
		fill: none;
		stroke: var(--color-border-control);
		stroke-width: 1.5;
	}
	.edge.lit {
		stroke: var(--color-accent);
		stroke-width: 2.5;
	}
	.step rect {
		fill: var(--color-surface-raised);
		stroke: var(--color-border-control);
	}
	.step.series rect {
		stroke: var(--color-warning);
	}
	.step.lit rect {
		stroke: var(--color-accent);
		stroke-width: 2;
	}
	text {
		fill: var(--color-text);
		font-family: inherit;
		font-size: var(--font-size-sm);
		text-anchor: middle;
		dominant-baseline: central;
	}
	.bar {
		fill: var(--color-border-control);
	}
	.bar.series {
		fill: var(--color-warning);
	}
	.bar.lit {
		fill: var(--color-accent);
	}
	.legend,
	.detail,
	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.steps {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: var(--space-4) 0 0;
		padding: 0;
		list-style: none;
	}
	.steps li {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
		align-items: baseline;
		padding: var(--space-2) var(--space-4);
		border-left: 3px solid transparent;
	}
	.steps li.series {
		border-left-color: var(--color-warning);
	}
	.steps li.lit {
		border-left-color: var(--color-accent);
	}
	.id {
		font-weight: var(--font-weight-strong);
	}
	.empty {
		line-height: var(--line-height-body);
	}
</style>
