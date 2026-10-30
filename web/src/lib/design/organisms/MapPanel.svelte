<script lang="ts">
	import type { MapLink, MapRoom, MapView } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';
	import Heading from '../atoms/Heading.svelte';
	import RadioGroup from '../molecules/RadioGroup.svelte';
	import { aiNote, endLine } from '../map-geometry.ts';

	interface Props {
		map: MapView;
		/** Give the map its own padding, background and scroll. Use it when the map is not in a panel. */
		framed?: boolean;
		/** Only the drawing of the current level, without heading, view choice and legend. For a small map in a panel. */
		compact?: boolean;
		/** The view for a story on more than one level: one level, or all levels unfolded. */
		view?: 'stacked' | 'unfolded';
	}

	let { map, framed = false, compact = false, view = $bindable('stacked') }: Props = $props();
	const uid = $props.id();

	// Levels: towers and dungeons. With more than one level, the map shows one level in full (stacked)
	// or all levels side by side in ring order (unfolded).
	const levels = $derived([...new Set(map.rooms.map((r) => r.level))].sort((a, b) => a - b));
	const multi = $derived(levels.length > 1);
	const here = $derived(map.rooms.find((r) => r.current));
	/** The level that the author picked. Null follows the player. */
	let picked = $state<number | null>(null);
	let lastLevel: number | undefined;
	$effect(() => {
		// When the player changes level, the map follows again.
		const l = here?.level;
		if (l !== lastLevel) picked = null;
		lastLevel = l;
	});
	const shown = $derived(picked ?? here?.level ?? levels[0] ?? 0);

	/** Rooms of one level, sorted around the middle of the level: north first, then clockwise. */
	function ringOrder(level: number): MapRoom[] {
		const rs = map.rooms.filter((r) => r.level === level);
		const mx = rs.reduce((a, r) => a + r.x, 0) / (rs.length || 1);
		const my = rs.reduce((a, r) => a + r.y, 0) / (rs.length || 1);
		const angle = (r: MapRoom) => (Math.atan2(r.y - my, r.x - mx) + Math.PI * 2.5) % (Math.PI * 2);
		return rs.sort((a, b) => angle(a) - angle(b) || a.id.localeCompare(b.id));
	}

	/** Grid positions for the current view. Rooms that the view does not show have no position. */
	const layout = $derived.by(() => {
		const pos = new Map<string, { x: number; y: number }>();
		if (!multi || view === 'stacked') {
			for (const r of map.rooms) if (!multi || r.level === shown) pos.set(r.id, { x: r.x, y: r.y });
			return pos;
		}
		// One row for each level, the highest at the top, the rooms in ring order. Each row is shifted so that
		// the room at the top of a stair is right above the room at its foot: a spiral reads as a staircase,
		// and a stair is a straight line up.
		const top = levels.at(-1) ?? 0;
		let shift = 0;
		levels.forEach((l, li) => {
			const ring = ringOrder(l);
			if (li > 0) {
				const stair = map.links.find((k) => {
					const a = map.rooms.find((r) => r.id === k.from);
					const b = map.rooms.find((r) => r.id === k.to);
					return a && b && ((a.level === l - 1 && b.level === l) || (b.level === l - 1 && a.level === l)) && (pos.has(a.id) || pos.has(b.id));
				});
				if (stair) {
					const lower = pos.get(stair.from) ?? pos.get(stair.to)!;
					const upper = ring.findIndex((r) => r.id === stair.from || r.id === stair.to);
					shift = lower.x - upper;
				}
			}
			ring.forEach((r, i) => pos.set(r.id, { x: shift + i, y: top - l }));
		});
		return pos;
	});
	/** The rooms of the levels next to the shown level, drawn faint behind it in the stacked view. */
	const ghosts = $derived(
		multi && view === 'stacked' ? map.rooms.filter((r) => Math.abs(r.level - shown) === 1).map((r) => ({ r, d: r.level - shown })) : []
	);
	const unfolded = $derived(multi && view === 'unfolded');
	let drawing: HTMLDivElement | undefined = $state();
	// In the unfolded view, scroll so that the current room is in the middle.
	$effect(() => {
		const el = drawing;
		const me = here;
		if (!el || !unfolded || !me || !layout.has(me.id)) return;
		const scale = el.scrollWidth / width;
		el.scrollLeft = Math.max(0, cx(me) * scale - el.clientWidth / 2);
	});
	const placed = $derived(map.rooms.filter((r) => layout.has(r.id)));
	const stairs = (r: MapRoom) => ({
		up: map.links.some((l) => (l.from === r.id && l.direction === 'up') || (l.to === r.id && l.direction === 'down' && !l.one_way)),
		down: map.links.some((l) => (l.from === r.id && l.direction === 'down') || (l.to === r.id && l.direction === 'up' && !l.one_way))
	});

	// Grid cell and room box size, in SVG units.
	const CELL_X = 210;
	const CELL_Y = 110;
	const BOX_W = 168;
	const BOX_H = 44;
	const PAD = 24;

	const bounds = $derived.by(() => {
		const ps = [...layout.values()];
		const xs = ps.map((p) => p.x);
		const ys = ps.map((p) => p.y);
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

	/** A long name is cut to fit the box. The full name is in the title and in the list for screen readers. */
	const short = (name: string) => (name.length > 21 ? `${name.slice(0, 20).trimEnd()}…` : name);
	const at = (r: MapRoom) => layout.get(r.id) ?? { x: r.x, y: r.y };
	const cx = (r: MapRoom) => (at(r).x - bounds.minX) * CELL_X + PAD + BOX_W / 2;
	const cy = (r: MapRoom) => (at(r).y - bounds.minY) * CELL_Y + PAD + BOX_H / 2;
	/** In the stacked view, the levels next to the shown one sit a little up and right (above) or down and left (below). */
	const GHOST = 14;
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

	/** A link to draw in the current view: both rooms placed, and not a stair in the stacked view. */
	function drawn(link: MapLink): boolean {
		if (!layout.has(link.from) || !layout.has(link.to)) return false;
		return !(multi && view === 'stacked' && (link.direction === 'up' || link.direction === 'down'));
	}

	/** A path for a link. In the unfolded view, a link between rooms that are not next to each other bends. */
	function path(a: MapRoom, b: MapRoom, end: { x: number; y: number }): string {
		const pa = at(a);
		const pb = at(b);
		if (multi && view === 'unfolded') {
			if (pa.y !== pb.y) {
				// A stair: straight up or down when the rooms are above each other, else up and then across.
				if (pa.x === pb.x) return `M${cx(a)} ${cy(a)}L${end.x} ${end.y}`;
				const side = cx(a) + (cx(b) >= cx(a) ? BOX_W / 2 : -BOX_W / 2) * 0.6;
				return `M${side} ${cy(a)}V${end.y}H${end.x}`;
			}
			if (Math.abs(pa.x - pb.x) > 1) {
				// The ring closes: an arc under the row.
				const y = cy(a) + BOX_H / 2 + 18;
				return `M${cx(a)} ${cy(a) + BOX_H / 2}V${y}H${end.x}V${end.y}`;
			}
		}
		return `M${cx(a)} ${cy(a)}L${end.x} ${end.y}`;
	}

	const ends = $derived((map.ends ?? []).filter((e) => layout.has(e.room) && (!(multi && view === 'stacked') || (e.direction !== 'up' && e.direction !== 'down'))));
	const hasProposals = $derived(map.rooms.some((r) => r.proposed || r.removed) || map.links.some((l) => l.proposed || l.removed) || (map.ends ?? []).some((e) => e.proposed || e.removed));

	function describe(link: MapLink): string {
		const from = room(link.from)?.name ?? link.from;
		const to = room(link.to)?.name ?? link.to;
		const parts = [`${from} ${link.one_way ? 'to' : 'and'} ${to}, ${link.direction}`];
		if (link.one_way) parts.push('one way');
		if (!link.visible) parts.push('hidden now');
		return parts.join(', ') + aiNote(link);
	}
</script>

<!--
	The map shows all rooms on their grid positions and the links between them.
	The SVG is decoration for sighted users. The same facts are in a list for screen readers.
-->
<section class="map-panel" class:framed aria-labelledby="{uid}-title">
	{#if compact}<h2 class="visually-hidden" id="{uid}-title">Map</h2>{:else}<Heading level={2} id="{uid}-title">Map</Heading>{/if}
	{#if current && !compact}<p class="where">You are in {current.name}{multi ? `, on level ${current.level}` : ''}.</p>{/if}
	{#if multi && !compact}
		<div class="levels">
			<RadioGroup legend="Map view" name="{uid}-view" look="pills" options={[{ value: 'stacked', label: 'One level' }, { value: 'unfolded', label: 'All levels, unfolded' }]} bind:value={view} />
			{#if view === 'stacked'}
				<div class="stepper" role="group" aria-label="Level">
					<Button size="compact" variant="quiet" disabled={shown >= (levels.at(-1) ?? 0)} onclick={() => (picked = levels[levels.indexOf(shown) + 1])}
						>Up<span class="visually-hidden"> one level</span></Button
					>
					<span class="level-name" aria-live="polite">Level {shown}{shown === here?.level ? ' (you)' : ''}</span>
					<Button size="compact" variant="quiet" disabled={shown <= levels[0]} onclick={() => (picked = levels[levels.indexOf(shown) - 1])}
						>Down<span class="visually-hidden"> one level</span></Button
					>
				</div>
			{/if}
		</div>
	{/if}

	<!-- The unfolded view can be wider than the panel: it scrolls sideways, so the names stay readable. -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="drawing" class:wide={unfolded} bind:this={drawing} tabindex={unfolded ? 0 : undefined} role={unfolded ? 'group' : undefined} aria-label={unfolded ? 'Unfolded map, scrolls sideways' : undefined}>
	<svg viewBox="0 0 {width} {height}" class="map" aria-hidden="true" style:min-width={unfolded ? `${Math.round(width * 0.7)}px` : undefined}>
		<defs>
			<marker id="{uid}-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
				<path d="M0 0L10 5L0 10z" class="arrow" />
			</marker>
		</defs>
		{#each ghosts as { r, d } (r.id)}
			{@const gx = (r.x - bounds.minX) * CELL_X + PAD + d * GHOST}
			{@const gy = (r.y - bounds.minY) * CELL_Y + PAD - d * GHOST}
			<rect class="ghost" class:above={d > 0} x={gx} y={gy} width={BOX_W} height={BOX_H}><title>{r.name}, level {r.level}</title></rect>
		{/each}
		{#each map.links.filter(drawn) as link (link.from + link.to + link.direction)}
			{@const a = room(link.from)}
			{@const b = room(link.to)}
			{#if a && b}
				{@const end = link.one_way ? edge(a, b) : { x: cx(b), y: cy(b) }}
				<path
					d={path(a, b, end)}
					class="link"
					class:stair={link.direction === 'up' || link.direction === 'down'}
					class:hidden-link={!link.visible}
					class:proposed={!!link.proposed}
					class:removed={!!link.removed}
					marker-end={link.one_way ? `url(#${uid}-arrow)` : undefined}
				/>
				{@const text = multi && view === 'unfolded' ? null : label(link)}
				{#if text}
					<text x={(cx(a) + cx(b)) / 2 + 8} y={(cy(a) + cy(b)) / 2} class="link-label">{text}</text>
				{/if}
			{/if}
		{/each}
		{#each ends as e (e.room + '>' + e.direction)}
			{@const r = room(e.room)}
			{#if r}
				{@const l = endLine(cx(r), cy(r), BOX_W, BOX_H, e.direction)}
				<g class="end" class:hidden-link={!e.visible} class:proposed={!!e.proposed} class:removed={!!e.removed}>
					<line x1={l.x1} y1={l.y1} x2={l.x2} y2={l.y2} />
					<path d={l.bar} />
					<title>{e.direction}: only a message</title>
				</g>
			{/if}
		{/each}
		{#each placed as r (r.id)}
			{@const st = multi ? stairs(r) : { up: false, down: false }}
			<g class="room" class:current={r.current} class:unvisited={!r.visited} class:proposed={!!r.proposed} class:removed={!!r.removed}>
				<rect x={cx(r) - BOX_W / 2} y={cy(r) - BOX_H / 2} width={BOX_W} height={BOX_H} />
				{#if r.current}
					<rect class="tint" x={cx(r) - BOX_W / 2} y={cy(r) - BOX_H / 2} width={BOX_W} height={BOX_H} />
				{/if}
				<text x={cx(r)} y={cy(r) + 4} text-anchor="middle">{r.proposed ? '✦ ' : ''}{short(r.name)}<title>{r.name}{aiNote(r)}</title></text>
				{#if r.removed}<line class="strike" x1={cx(r) - BOX_W / 2} y1={cy(r)} x2={cx(r) + BOX_W / 2} y2={cy(r)} />{/if}
				{#if st.up || st.down}
					<text x={cx(r) + BOX_W / 2 - 6} y={cy(r) - BOX_H / 2 + 13} text-anchor="end" class="stairs"
						>{st.up ? '▲' : ''}{st.down ? '▼' : ''}</text
					>
				{/if}
			</g>
		{/each}
	</svg>
	</div>

	<div class="legend" aria-hidden="true" hidden={compact}>
		<span><i class="swatch current-swatch"></i>you are here</span>
		<span><i class="swatch visited-swatch"></i>visited</span>
		<span><i class="swatch unvisited-swatch"></i>not visited yet</span>
		{#if ends.length}<span><i class="end-swatch"></i>only a message</span>{/if}
		{#if hasProposals}<span><i class="swatch proposed-swatch"></i>✦ proposed by AI</span>{/if}
	</div>

	<div class="visually-hidden">
		<h3>Rooms</h3>
		<ul>
			{#each map.rooms as r (r.id)}
				<li>{r.name}{multi ? `, level ${r.level}` : ''}{r.current ? ', you are here' : r.visited ? ', visited' : ', not visited yet'}{aiNote(r)}</li>
			{/each}
		</ul>
		<h3>Connections</h3>
		<ul>
			{#each map.links as link (link.from + link.to + link.direction)}
				<li>{describe(link)}</li>
			{/each}
			{#each map.ends ?? [] as e (e.room + '>' + e.direction)}
				<li>{room(e.room)?.name ?? e.room}, {e.direction}: only a message{e.visible ? '' : ', hidden now'}{aiNote(e)}</li>
			{/each}
		</ul>
	</div>
</section>

<style>
	.map-panel {
		/* The background behind the map. Room boxes use it, so lines do not show through. */
		--map-bg: var(--color-surface-raised);
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
	}
	/* Standalone use, for example the map column on ultrawide screens. */
	.map-panel.framed {
		position: relative; /* Keeps screen-reader text inside the scroll area. */
		box-sizing: border-box;
		min-height: 0;
		padding: var(--space-7);
		overflow-y: auto;
		/* Next to the game text, the map steps back: darker than the player, quiet colours. */
		--map-bg: var(--color-surface-outer);
		background: var(--map-bg);
	}
	.where {
		margin: 0;
		color: var(--color-text-muted);
	}
	.drawing.wide {
		position: relative;
		overflow-x: auto;
	}
	.drawing.wide:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.map {
		display: block;
		width: 100%;
		height: auto;
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.link {
		stroke: var(--color-border-control);
		stroke-width: 1;
	}
	.link {
		fill: none;
	}
	.stair {
		stroke: var(--color-accent);
		stroke-width: 1.5;
	}
	.ghost {
		fill: none;
		stroke: var(--color-border-subtle);
		stroke-width: 1;
		stroke-dasharray: 2 4;
	}
	.ghost.above {
		stroke: var(--color-border-control);
	}
	.stairs {
		fill: var(--color-text-accent);
		font-size: var(--font-size-xs);
	}
	.levels {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3) var(--space-6);
	}
	.stepper {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.level-name {
		min-width: 9ch;
		text-align: center;
		color: var(--color-text-strong);
		font-size: var(--font-size-sm);
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
		fill: var(--map-bg);
		stroke: var(--color-border-control);
		stroke-width: 1;
	}
	.room text {
		fill: var(--color-text-muted);
	}
	.room.unvisited rect {
		stroke-dasharray: 3 4;
	}
	/* The current room: an accent edge and a faint tint, not a solid block. */
	.room.current rect {
		stroke: var(--color-accent);
		stroke-width: 1.5;
	}
	/* A second rectangle over the opaque one, so the links under the room do not show through the tint. */
	.room .tint {
		fill: var(--color-accent-tint);
	}
	.room.current text {
		fill: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
	}
	.end line,
	.end path {
		stroke: var(--color-border-control);
		stroke-width: 1.5;
	}
	/* AI proposals: the ghost colour and a long dash; the name has a ✦, so the colour is not the only cue. */
	.link.proposed,
	.end.proposed line,
	.end.proposed path {
		stroke: var(--color-ghost);
		stroke-width: 1.5;
		stroke-dasharray: 6 4;
	}
	.room.proposed rect {
		fill: var(--map-bg);
		stroke: var(--color-ghost);
		stroke-width: 1.5;
		stroke-dasharray: 6 4;
	}
	.room.proposed text {
		fill: var(--color-ghost);
		font-style: italic;
	}
	/* What a proposal removes: the warning colour and a line through it. */
	.link.removed,
	.end.removed line,
	.end.removed path,
	.room.removed rect,
	.room .strike {
		stroke: var(--color-warning);
	}
	.room.removed text {
		fill: var(--color-text-warning);
		text-decoration: line-through;
	}
	.legend[hidden] {
		display: none;
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
		background: var(--color-accent-tint);
		border-color: var(--color-accent);
	}
	.unvisited-swatch {
		border-style: dashed;
	}
	.proposed-swatch {
		border-color: var(--color-ghost);
		border-style: dashed;
	}
	.end-swatch {
		display: inline-block;
		width: 14px;
		height: 10px;
		border-right: 2px solid var(--color-border-control);
		background: linear-gradient(var(--color-border-control), var(--color-border-control)) left center / 100% 1.5px no-repeat;
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
