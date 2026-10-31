<script lang="ts">
	import type { MachineView } from '#lib/engine/types.ts';
	import Heading from '../atoms/Heading.svelte';

	interface Props {
		machines: MachineView[];
		/** Show the rule of a step in the rules view. */
		onrule?: (id: string) => void;
		/** The state that the details panel shows. */
		selected?: { machine: string; state: string } | null;
		/** Select a state for the details panel. */
		onselect?: (machine: string, state: string) => void;
	}

	let { machines, onrule, selected = null, onselect }: Props = $props();

	/** Width of one state column and the size of a state box, in SVG units. */
	const COL = 150;
	const BOX_W = 116;
	const BOX_H = 34;
	const PAD = 24;
	/** Two badges closer than this move apart. */
	const BADGE_GAP = 24;

	interface Edge {
		/** The number of the step, from 1. */
		n: number;
		from: string;
		to: string;
		path: string;
		label: { x: number; y: number };
		ready: boolean;
		fired: boolean;
	}

	/**
	 * What the pointer (or the focus on a step row) points at: a state lights its steps in and out, and the
	 * states at their other ends; a step lights its arrows and the states at both ends.
	 */
	let lit = $state<{ machine: string; state?: string; step?: number; from?: string } | null>(null);

	function pointState(m: MachineView, state: string) {
		lit = { machine: m.id, state };
	}

	/** A step; `from` narrows it to one of its arrows. */
	function pointStep(m: MachineView, step: number, from?: string) {
		lit = { machine: m.id, step, from };
	}

	function clear() {
		lit = null;
	}

	/** The arrow from `from` to `to` of step `n` is lit. */
	function edgeOn(m: MachineView, n: number, from: string, to: string): boolean {
		if (lit?.machine !== m.id) return false;
		if (lit.state) return from === lit.state || to === lit.state;
		return n === lit.step && (!lit.from || from === lit.from);
	}

	/** The state is lit: the pointed state, or an end of a lit arrow. */
	function stateOn(m: MachineView, s: string): boolean {
		if (lit?.machine !== m.id) return false;
		if (s === lit.state) return true;
		return m.steps.some((st, i) =>
			st.from.some((f) => edgeOn(m, i + 1, f, st.to) && (f === s || st.to === s))
		);
	}

	const isLit = (m: MachineView) => lit?.machine === m.id;

	/**
	 * States in a row; a step to a later state arcs above, a step back arcs below. An arc whose badge
	 * would sit on another badge goes higher (or lower), so every number stays readable.
	 */
	function layout(m: MachineView) {
		const index = new Map(m.states.map((s, i) => [s, i]));
		const cx = (i: number) => PAD + BOX_W / 2 + i * COL;
		const raw = m.steps.flatMap((step, i) =>
			step.from.map((from) => ({
				n: i + 1,
				from,
				to: step.to,
				a: index.get(from) ?? 0,
				b: index.get(step.to) ?? 0,
				ready: step.ready && from === m.state,
				fired: step.fired
			}))
		);
		// The same step from several states to one state shares one badge per arc; first place the arcs.
		const placed: { x: number; side: number; lift: number }[] = [];
		const lifts = raw.map(({ a, b }) => {
			const side = b > a ? -1 : b < a ? 1 : -1;
			const x = a === b ? cx(a) : (cx(a) + cx(b)) / 2;
			let lift = a === b ? 40 : 22 + 16 * Math.abs(b - a);
			while (placed.some((p) => p.side === side && Math.abs(p.x - x) < BADGE_GAP && Math.abs(p.lift - lift) < BADGE_GAP)) {
				lift += BADGE_GAP;
			}
			placed.push({ x, side, lift });
			return lift;
		});
		const above = Math.max(0, ...raw.map((r, i) => (r.b >= r.a ? lifts[i] : 0)));
		const below = Math.max(0, ...raw.map((r, i) => (r.b < r.a ? lifts[i] : 0)));
		const top = PAD + above;
		const edges: Edge[] = raw.map(({ n, from, to, a, b, ready, fired }, i) => {
			const lift = lifts[i];
			if (a === b) {
				const x = cx(a);
				return {
					n,
					from,
					to,
					path: `M ${x - 14} ${top} C ${x - 30} ${top - lift * 1.15}, ${x + 30} ${top - lift * 1.15}, ${x + 14} ${top}`,
					label: { x, y: top - lift },
					ready,
					fired
				};
			}
			const forward = b > a;
			const y = forward ? top : top + BOX_H;
			const x1 = cx(a) + (forward ? 12 : -12);
			const x2 = cx(b) + (forward ? -12 : 12);
			const peak = forward ? y - lift : y + lift;
			// A quadratic curve reaches half way to its control point; aim twice as far.
			const control = forward ? y - 2 * lift : y + 2 * lift;
			return {
				n,
				from,
				to,
				path: `M ${x1} ${y} Q ${(x1 + x2) / 2} ${control}, ${x2} ${y}`,
				label: { x: (x1 + x2) / 2, y: peak },
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
	The puzzle machines of the story, for the Atlas, in the layout of the rules view: one group for each
	machine. The drawing shows the states in a row and the steps as numbered arrows: forward steps above,
	steps back below. The current state is marked; a step that the next matching event takes now is marked
	too. The list under the drawing says the same in words; a row opens the rule of the step.
-->
<section class="machines-view" aria-labelledby="machines-title">
	<Heading level={2} id="machines-title">Machines</Heading>

	{#each machines as m (m.id)}
		{@const g = layout(m)}
		<details class="group" open>
			<summary>
				<span class="group-name">{m.id}</span>
				<span class="count">{m.steps.length} steps</span>
				<span class="now">now <strong>{m.state}</strong></span>
			</summary>
			<div class="graph">
				<svg
					class:pointing={isLit(m)}
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
						<path class="edge" class:ready={e.ready} class:fired={e.fired} class:lit={edgeOn(m, e.n, e.from, e.to)} d={e.path} marker-end="url(#arrow-{m.id})" />
						<!-- A wide, invisible copy of the arrow that is easy to point at. -->
						<path
							class="hit"
							role="presentation"
							d={e.path}
							onpointerenter={() => pointStep(m, e.n, e.from)}
							onpointerleave={clear}
						/>
					{/each}
					{#each g.edges as e, i (i)}
						<g
							class="badge"
							class:ready={e.ready}
							class:lit={edgeOn(m, e.n, e.from, e.to)}
							role="presentation"
							onpointerenter={() => pointStep(m, e.n, e.from)}
							onpointerleave={clear}
						>
							<rect x={e.label.x - 9} y={e.label.y - 9} width="18" height="18" />
							<text x={e.label.x} y={e.label.y}>{e.n}</text>
						</g>
					{/each}
					{#each m.states as s, i (s)}
						<g
							class="state"
							class:current={s === m.state}
							class:lit={stateOn(m, s)}
							class:selected={selected?.machine === m.id && selected.state === s}
							role="button"
							tabindex={onselect ? 0 : -1}
							aria-label="State {s}{s === m.state ? ', now' : ''}. Show its details"
							aria-pressed={selected?.machine === m.id && selected.state === s}
							onpointerenter={() => pointState(m, s)}
							onpointerleave={clear}
							onfocus={() => pointState(m, s)}
							onblur={clear}
							onclick={() => onselect?.(m.id, s)}
							onkeydown={(e) => {
								if (e.key === 'Enter' || e.key === ' ') {
									e.preventDefault();
									onselect?.(m.id, s);
								}
							}}
						>
							<rect x={g.cx(i) - BOX_W / 2} y={g.top} width={BOX_W} height={BOX_H} />
							<text x={g.cx(i)} y={g.top + BOX_H / 2}>{s}</text>
						</g>
					{/each}
				</svg>
			</div>
			<ol class="steps">
				{#each m.steps as step, i (step.rule)}
					<li>
						<button
							type="button"
							class="step"
							class:ready={step.ready}
							class:lit={step.from.some((f) => edgeOn(m, i + 1, f, step.to))}
							disabled={!onrule}
							onpointerenter={() => pointStep(m, i + 1)}
							onpointerleave={clear}
							onfocus={() => pointStep(m, i + 1)}
							onblur={clear}
							onclick={() => onrule?.(step.rule)}
							aria-label="Step {i + 1}: {step.from.join(' or ')} to {step.to}. Show rule {step.rule}"
						>
							<span class="n">{i + 1}</span>
							<span class="text">
								<span class="move">{step.from.join(' or ')} → {step.to}</span>
								<span class="when">
									when {step.when}{#if step.condition}, if <code>{step.condition}</code>{/if}
								</span>
							</span>
							<span class="status" class:on={step.ready || step.fired}>
								{step.ready ? 'ready now' : step.fired ? 'ran' : 'not run yet'}
							</span>
						</button>
					</li>
				{/each}
			</ol>
		</details>
	{:else}
		<p class="empty">
			This story has no puzzle machines. A machine is a puzzle with named states and steps between them, in
			<code>machines/</code> or under <code>machines:</code> in story.yaml.
		</p>
	{/each}
</section>

<style>
	.machines-view {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	/* Groups as in the rules view. */
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
	.now {
		padding: 0 var(--space-3);
		border: var(--size-border) solid var(--color-border-subtle);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.now {
		border-color: var(--color-accent);
	}
	.now strong {
		color: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
	}
	.graph {
		max-width: 100%;
		margin-top: var(--space-3);
		padding: var(--space-4);
		overflow-x: auto;
		border: var(--size-border) solid var(--color-border-subtle);
	}
	svg {
		display: block;
		max-width: none;
	}
	/* While the pointer is on a state or a step, the rest steps back. */
	svg.pointing .edge:not(.lit),
	svg.pointing .badge:not(.lit),
	svg.pointing .state:not(.lit) {
		opacity: 0.3;
	}
	.edge,
	.badge,
	.state {
		transition: opacity var(--duration-fast);
	}
	.hit {
		fill: none;
		stroke: transparent;
		stroke-width: 14;
		pointer-events: stroke;
	}
	.badge {
		cursor: default;
	}
	.state {
		cursor: pointer;
		outline: none;
	}
	.state.selected rect,
	.state:focus-visible rect {
		stroke: var(--color-accent);
		stroke-width: 3;
	}
	.edge.lit {
		stroke: var(--color-accent);
		stroke-width: 2.5;
	}
	.badge.lit rect {
		stroke: var(--color-accent);
	}
	.badge.lit text {
		fill: var(--color-text-accent);
	}
	.state.lit rect {
		stroke: var(--color-accent);
		stroke-width: 2;
	}
	.step.lit {
		border-color: var(--color-accent);
	}
	@media (prefers-reduced-motion: reduce) {
		.edge,
		.badge,
		.state {
			transition: none;
		}
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
		stroke-width: 2;
	}
	.badge rect {
		fill: var(--color-surface);
		stroke: var(--color-border-subtle);
	}
	.badge text {
		fill: var(--color-text-muted);
	}
	.badge.ready rect {
		stroke: var(--color-accent);
	}
	.badge.ready text {
		fill: var(--color-text-accent);
	}
	text {
		fill: var(--color-text);
		font-family: inherit;
		font-size: var(--font-size-sm);
		text-anchor: middle;
		dominant-baseline: central;
	}
	.state rect {
		fill: var(--color-surface);
		stroke: var(--color-border-control);
	}
	.state.current rect {
		fill: var(--color-accent-tint);
		stroke: var(--color-accent);
	}
	.state.current text {
		fill: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
	}
	/* Step rows as the rule rows of the rules view. */
	.steps {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: var(--space-3) 0 0;
		padding: 0;
		list-style: none;
	}
	.step {
		display: grid;
		grid-template-columns: 1.5em minmax(0, 1fr);
		gap: var(--space-1) var(--space-4);
		align-items: baseline;
		width: 100%;
		padding: var(--space-3) var(--space-4);
		border: var(--size-border) solid var(--color-border-subtle);
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.step:disabled {
		cursor: default;
	}
	.step:not(:disabled):hover {
		border-color: var(--color-accent);
	}
	.step:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.step.ready {
		border-left: var(--size-border-strong) solid var(--color-accent);
	}
	.n {
		color: var(--color-text-muted);
		font-variant-numeric: tabular-nums;
	}
	.move {
		color: var(--color-text-accent);
	}
	.move {
		margin-right: var(--space-3);
	}
	.when {
		color: var(--color-text);
		overflow-wrap: anywhere;
	}
	.status {
		grid-column: 2;
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.status.on {
		color: var(--color-text-accent);
	}
	.empty {
		margin: 0;
		color: var(--color-text-muted);
	}
</style>
