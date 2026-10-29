<script lang="ts">
	import type { MapLink, MapRoom, MapView } from '#lib/engine/types.ts';
	import Heading from '../atoms/Heading.svelte';

	interface Props {
		map: MapView;
	}

	let { map }: Props = $props();

	// Grid cell and room box size, in SVG units.
	const CELL_X = 210;
	const CELL_Y = 110;
	const BOX_W = 168;
	const BOX_H = 44;
	const PAD = 24;

	const bounds = $derived.by(() => {
		const xs = map.rooms.map((r) => r.x);
		const ys = map.rooms.map((r) => r.y);
		return {
			minX: Math.min(0, ...xs),
			minY: Math.min(0, ...ys),
			maxX: Math.max(0, ...xs),
			maxY: Math.max(0, ...ys)
		};
	});
	// Extra room on the right for exit labels such as "through the mirror".
	const LABEL_ROOM = 120;
	const width = $derived((bounds.maxX - bounds.minX) * CELL_X + BOX_W + 2 * PAD + LABEL_ROOM);
	const height = $derived((bounds.maxY - bounds.minY) * CELL_Y + BOX_H + 2 * PAD);

	const cx = (r: MapRoom) => (r.x - bounds.minX) * CELL_X + PAD + BOX_W / 2;
	const cy = (r: MapRoom) => (r.y - bounds.minY) * CELL_Y + PAD + BOX_H / 2;
	const room = (id: string) => map.rooms.find((r) => r.id === id);

	/** The point where the line from `a` to `b` leaves the box of `b`, so an arrow head stays visible. */
	function edge(a: MapRoom, b: MapRoom): { x: number; y: number } {
		const dx = cx(b) - cx(a);
		const dy = cy(b) - cy(a);
		const t = Math.min(
			dx === 0 ? Infinity : (BOX_W / 2 + 4) / Math.abs(dx),
			dy === 0 ? Infinity : (BOX_H / 2 + 4) / Math.abs(dy)
		);
		return { x: cx(b) - dx * t, y: cy(b) - dy * t };
	}
	const current = $derived(map.rooms.find((r) => r.current));

	/** Vertical and named exits get a label, because the line alone does not show the direction. */
	function label(link: MapLink): string | null {
		return ['north', 'south', 'east', 'west', 'northeast', 'northwest', 'southeast', 'southwest'].includes(link.direction)
			? null
			: link.direction;
	}

	function describe(link: MapLink): string {
		const from = room(link.from)?.name ?? link.from;
		const to = room(link.to)?.name ?? link.to;
		const parts = [`${from} ${link.one_way ? 'to' : 'and'} ${to}, ${link.direction}`];
		if (link.one_way) parts.push('one way');
		if (!link.visible) parts.push('hidden now');
		return parts.join(', ');
	}
</script>

<!--
	The map shows all rooms on their grid positions and the links between them.
	The SVG is decoration for sighted users. The same facts are in a list for screen readers.
-->
<section class="map-panel" aria-labelledby="map-title">
	<Heading level={2} id="map-title">Map</Heading>
	{#if current}<p class="where">You are in {current.name}.</p>{/if}

	<svg viewBox="0 0 {width} {height}" class="map" aria-hidden="true">
		<defs>
			<marker id="map-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
				<path d="M0 0L10 5L0 10z" class="arrow" />
			</marker>
		</defs>
		{#each map.links as link (link.from + link.to + link.direction)}
			{@const a = room(link.from)}
			{@const b = room(link.to)}
			{#if a && b}
				{@const end = link.one_way ? edge(a, b) : { x: cx(b), y: cy(b) }}
				<line
					x1={cx(a)}
					y1={cy(a)}
					x2={end.x}
					y2={end.y}
					class="link"
					class:hidden-link={!link.visible}
					marker-end={link.one_way ? 'url(#map-arrow)' : undefined}
				/>
				{@const text = label(link)}
				{#if text}
					<text x={(cx(a) + cx(b)) / 2 + 8} y={(cy(a) + cy(b)) / 2} class="link-label">{text}</text>
				{/if}
			{/if}
		{/each}
		{#each map.rooms as r (r.id)}
			<g class="room" class:current={r.current} class:unvisited={!r.visited}>
				<rect x={cx(r) - BOX_W / 2} y={cy(r) - BOX_H / 2} width={BOX_W} height={BOX_H} />
				<text x={cx(r)} y={cy(r) + 4} text-anchor="middle">{r.name}</text>
			</g>
		{/each}
	</svg>

	<div class="legend" aria-hidden="true">
		<span><i class="swatch current-swatch"></i>you are here</span>
		<span><i class="swatch visited-swatch"></i>visited</span>
		<span><i class="swatch unvisited-swatch"></i>not visited yet</span>
	</div>

	<div class="visually-hidden">
		<h3>Rooms</h3>
		<ul>
			{#each map.rooms as r (r.id)}
				<li>{r.name}{r.current ? ', you are here' : r.visited ? ', visited' : ', not visited yet'}</li>
			{/each}
		</ul>
		<h3>Connections</h3>
		<ul>
			{#each map.links as link (link.from + link.to + link.direction)}
				<li>{describe(link)}</li>
			{/each}
		</ul>
	</div>
</section>

<style>
	.map-panel {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		box-sizing: border-box;
		min-height: 0;
		padding: var(--space-7);
		overflow-y: auto;
		background: var(--color-surface-raised);
	}
	.where {
		margin: 0;
		color: var(--color-text-muted);
	}
	.map {
		width: 100%;
		height: auto;
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.link {
		stroke: var(--color-border-control);
		stroke-width: 1.5;
	}
	.hidden-link {
		stroke-dasharray: 4 5;
	}
	.arrow {
		fill: var(--color-border-control);
	}
	.link-label {
		fill: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.room rect {
		fill: var(--color-surface);
		stroke: var(--color-border-control);
		stroke-width: 1;
	}
	.room text {
		fill: var(--color-text);
	}
	.room.unvisited rect {
		stroke-dasharray: 3 4;
		fill: var(--color-surface-raised);
	}
	.room.unvisited text {
		fill: var(--color-text-muted);
	}
	.room.current rect {
		fill: var(--color-accent);
		stroke: var(--color-accent);
	}
	.room.current text {
		fill: var(--color-text-on-accent);
		font-weight: var(--font-weight-strong);
	}
	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3) var(--space-6);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.legend span {
		display: inline-flex;
		align-items: center;
		gap: var(--space-3);
	}
	.swatch {
		display: inline-block;
		width: 14px;
		height: 10px;
		border: var(--size-border) solid var(--color-border-control);
	}
	.current-swatch {
		background: var(--color-accent);
		border-color: var(--color-accent);
	}
	.visited-swatch {
		background: var(--color-surface);
	}
	.unvisited-swatch {
		border-style: dashed;
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
