<script lang="ts">
	import type { MachineView } from '#lib/engine/types.ts';
	import Heading from '../atoms/Heading.svelte';

	interface Props {
		machines: MachineView[];
		/** Show the rule of a step in the rules view. */
		onrule?: (id: string) => void;
	}

	let { machines, onrule }: Props = $props();

	/** Width of one state column and the size of a state box, in SVG units. */
	const COL = 150;
	const BOX_W = 116;
	const BOX_H = 34;
	const PAD = 24;

	interface Edge {
		/** The number of the step, from 1. */
		n: number;
		path: string;
		label: { x: number; y: number };
		ready: boolean;
		fired: boolean;
	}

	/** States in a row; a step to a later state arcs above, a step back arcs below. */
	function layout(m: MachineView) {
		const index = new Map(m.states.map((s, i) => [s, i]));
		let above = 0;
		let below = 0;
		const raw = m.steps.flatMap((step, i) =>
			step.from.map((from) => {
				const a = index.get(from) ?? 0;
				const b = index.get(step.to) ?? 0;
				const lift = 22 + 16 * Math.abs(b - a);
				if (b > a) above = Math.max(above, lift);
				else if (b < a) below = Math.max(below, lift);
				else above = Math.max(above, 40);
				return { n: i + 1, a, b, lift, ready: step.ready && from === m.state, fired: step.fired };
			})
		);
		const top = PAD + above;
		const cx = (i: number) => PAD + BOX_W / 2 + i * COL;
		const edges: Edge[] = raw.map(({ n, a, b, lift, ready, fired }) => {
			if (a === b) {
				const x = cx(a);
				const y = top;
				return {
					n,
					path: `M ${x - 14} ${y} C ${x - 30} ${y - 46}, ${x + 30} ${y - 46}, ${x + 14} ${y}`,
					label: { x, y: y - 40 },
					ready,
					fired
				};
			}
			const forward = b > a;
			const y = forward ? top : top + BOX_H;
			const x1 = cx(a) + (forward ? 12 : -12);
			const x2 = cx(b) + (forward ? -12 : 12);
			const peak = forward ? y - lift : y + lift;
			const mid = (x1 + x2) / 2;
			return {
				n,
				path: `M ${x1} ${y} Q ${mid} ${peak + (forward ? -lift / 3 : lift / 3)}, ${x2} ${y}`,
				label: { x: mid, y: peak },
				ready,
				fired
			};
		});
		return {
			width: PAD * 2 + BOX_W + Math.max(0, m.states.length - 1) * COL,
			height: top + BOX_H + below + PAD,
			top,
			cx,
			edges
		};
	}
</script>

<!--
	The puzzle machines of the story, for the Atlas. Each machine shows its states in a row and its steps as
	numbered arrows: forward steps above, steps back below. The current state is filled; a step that the next
	matching event takes now is marked. The list under the drawing says the same in words.
-->
<section class="machines-view" aria-label="Puzzle machines">
	{#each machines as m (m.id)}
		{@const g = layout(m)}
		<article class="machine">
			<Heading level={3}>{m.id}</Heading>
			<p class="now">Now: <strong>{m.state}</strong></p>
			<div class="graph">
				<svg
					viewBox="0 0 {g.width} {g.height}"
					width={g.width}
					height={g.height}
					role="img"
					aria-label="{m.id}: {m.states.length} states, {m.steps.length} steps, now {m.state}"
				>
					<defs>
						<marker id="arrow-{m.id}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
							<path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke" />
						</marker>
					</defs>
					{#each g.edges as e, i (i)}
						<path class="edge" class:ready={e.ready} class:fired={e.fired} d={e.path} marker-end="url(#arrow-{m.id})" />
						<g class="badge" class:ready={e.ready}>
							<circle cx={e.label.x} cy={e.label.y} r="10" />
							<text x={e.label.x} y={e.label.y}>{e.n}</text>
						</g>
					{/each}
					{#each m.states as s, i (s)}
						<g class="state" class:current={s === m.state}>
							<rect x={g.cx(i) - BOX_W / 2} y={g.top} width={BOX_W} height={BOX_H} rx="6" />
							<text x={g.cx(i)} y={g.top + BOX_H / 2}>{s}</text>
						</g>
					{/each}
				</svg>
			</div>
			<ol class="steps">
				{#each m.steps as step, i (step.rule)}
					<li class:ready={step.ready}>
						<span class="n">{i + 1}</span>
						<span class="move">{step.from.join(' or ')} → {step.to}</span>
						<span class="when">when {step.when}{#if step.condition}, if <code>{step.condition}</code>{/if}</span>
						{#if step.ready}<span class="mark">ready</span>{/if}
						{#if step.fired}<span class="mark muted">ran</span>{/if}
						{#if onrule}<button type="button" class="link" onclick={() => onrule(step.rule)}>rule</button>{/if}
					</li>
				{/each}
			</ol>
		</article>
	{:else}
		<p class="muted">
			No puzzle machines. A machine is a puzzle with named states and steps between them, in <code>machines/</code> or under
			<code>machines:</code> in story.yaml.
		</p>
	{/each}
</section>

<style>
	.machines-view {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.machine {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding-bottom: var(--space-5);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
	}
	p {
		margin: 0;
	}
	.now,
	.muted {
		color: var(--color-text-muted);
	}
	.now strong {
		color: var(--color-text-accent);
	}
	.graph {
		max-width: 100%;
		overflow-x: auto;
	}
	svg {
		display: block;
		max-width: none;
	}
	.edge {
		fill: none;
		stroke: var(--color-border-control);
		stroke-width: 1.5;
	}
	.edge.fired {
		stroke: var(--color-text-muted);
	}
	.edge.ready {
		stroke: var(--color-accent);
		stroke-width: 2.5;
	}
	.badge circle {
		fill: var(--color-surface-raised);
		stroke: var(--color-border-control);
	}
	.badge.ready circle {
		stroke: var(--color-accent);
	}
	text {
		fill: var(--color-text);
		font-family: var(--font-ui);
		font-size: 12px;
		text-anchor: middle;
		dominant-baseline: central;
	}
	.state rect {
		fill: var(--color-surface-control);
		stroke: var(--color-border-control);
	}
	.state.current rect {
		fill: var(--color-accent);
		stroke: var(--color-accent);
	}
	.state.current text {
		fill: var(--color-text-on-accent);
		font-weight: 600;
	}
	.steps {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.steps li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-1) var(--space-3);
		line-height: var(--line-height-body);
	}
	.n {
		min-width: 1.5em;
		color: var(--color-text-muted);
		font-variant-numeric: tabular-nums;
	}
	.move {
		color: var(--color-text-strong);
	}
	.when {
		color: var(--color-text);
	}
	.mark {
		color: var(--color-text-accent);
	}
	.mark.muted {
		color: var(--color-text-muted);
	}
	.link {
		padding: 0;
		border: 0;
		background: none;
		color: var(--color-text-accent);
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	.link:focus-visible {
		outline: var(--size-focus, 2px) solid var(--color-focus);
		outline-offset: 2px;
	}
</style>
