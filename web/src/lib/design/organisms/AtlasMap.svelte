<script lang="ts">
	import type { AtlasRoom, MapView } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';
	import MapPanel from './MapPanel.svelte';

	interface Props {
		map: MapView;
		/** What is in each room now, for search and the list view. */
		rooms: AtlasRoom[];
		/** The selected room id. */
		selected?: string | null;
		onselect: (id: string) => void;
	}

	let { map, rooms, selected = $bindable(null), onselect }: Props = $props();

	// Grid cell and room box size, in SVG units, as in MapPanel.
	const CELL_X = 210;
	const CELL_Y = 110;
	const BOX_W = 168;
	const BOX_H = 44;

	const levels = $derived([...new Set(map.rooms.map((r) => r.level))].sort((a, b) => b - a));
	const here = $derived(map.rooms.find((r) => r.current));
	/** The floor on show, or "all" for the unfolded view. */
	let floor = $state<number | 'all' | null>(null);
	const shownFloor = $derived(floor ?? here?.level ?? levels[0] ?? 0);
	let list = $state(false);
	let query = $state('');
	let canvas: HTMLDivElement | undefined = $state();
	let width = $state(800);
	let height = $state(560);

	const floorRooms = $derived(shownFloor === 'all' ? map.rooms : map.rooms.filter((r) => r.level === shownFloor));
	const ids = $derived(new Set(floorRooms.map((r) => r.id)));
	const floorLinks = $derived(map.links.filter((l) => ids.has(l.from) && ids.has(l.to)));

	// The view: a scale and the map point in the top left corner.
	let scale = $state(1);
	let ox = $state(0);
	let oy = $state(0);
	const px = (x: number) => x * CELL_X;
	const py = (y: number) => y * CELL_Y;

	/** Show the whole floor. */
	function fit() {
		if (!floorRooms.length) return;
		const xs = floorRooms.map((r) => px(r.x));
		const ys = floorRooms.map((r) => py(r.y));
		const minX = Math.min(...xs) - BOX_W;
		const maxX = Math.max(...xs) + BOX_W;
		const minY = Math.min(...ys) - BOX_H * 2;
		const maxY = Math.max(...ys) + BOX_H * 2;
		scale = Math.min(2, Math.max(0.2, Math.min(width / (maxX - minX), height / (maxY - minY))));
		ox = minX - (width / scale - (maxX - minX)) / 2;
		oy = minY - (height / scale - (maxY - minY)) / 2;
	}

	/** Put a room in the middle of the view. */
	function center(id: string) {
		const r = map.rooms.find((x) => x.id === id);
		if (!r) return;
		ox = px(r.x) - width / scale / 2;
		oy = py(r.y) - height / scale / 2;
	}

	function zoom(by: number, at = { x: width / 2, y: height / 2 }) {
		const next = Math.min(3, Math.max(0.2, scale * by));
		ox += at.x / scale - at.x / next;
		oy += at.y / scale - at.y / next;
		scale = next;
	}

	// A new floor, or the first show: the selected room (or the player) in the middle at a size to read,
	// else the whole floor.
	let fitted = '';
	$effect(() => {
		const key = `${shownFloor}#${width}x${height}#${floorRooms.length}`;
		if (key !== fitted && width > 0) {
			fitted = key;
			const focus = floorRooms.find((r) => r.id === selected) ?? floorRooms.find((r) => r.current);
			if (focus) {
				scale = 0.9;
				center(focus.id);
			} else fit();
		}
	});

	let drag: { x: number; y: number; ox: number; oy: number; moved: boolean } | null = null;
	function down(e: PointerEvent) {
		drag = { x: e.clientX, y: e.clientY, ox, oy, moved: false };
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}
	function move(e: PointerEvent) {
		if (!drag) return;
		const dx = e.clientX - drag.x;
		const dy = e.clientY - drag.y;
		if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true;
		ox = drag.ox - dx / scale;
		oy = drag.oy - dy / scale;
	}
	function up() {
		setTimeout(() => (drag = null));
	}
	function wheel(e: WheelEvent) {
		e.preventDefault();
		const rect = canvas!.getBoundingClientRect();
		zoom(e.deltaY < 0 ? 1.15 : 1 / 1.15, { x: e.clientX - rect.left, y: e.clientY - rect.top });
	}
	function keys(e: KeyboardEvent) {
		const step = 60 / scale;
		const map: Record<string, () => void> = {
			ArrowLeft: () => (ox -= step),
			ArrowRight: () => (ox += step),
			ArrowUp: () => (oy -= step),
			ArrowDown: () => (oy += step),
			'+': () => zoom(1.2),
			'=': () => zoom(1.2),
			'-': () => zoom(1 / 1.2),
			f: fit
		};
		const run = map[e.key];
		if (run) {
			e.preventDefault();
			run();
		}
	}

	function pick(id: string) {
		if (drag?.moved) return;
		selected = id;
		onselect(id);
	}

	/** Search: rooms by name or id, and rooms with a thing of that name. */
	const results = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return [];
		return rooms
			.filter(
				(r) =>
					r.name.toLowerCase().includes(q) ||
					r.id.includes(q) ||
					r.things.some((t) => t.name.toLowerCase().includes(q) || t.id.includes(q))
			)
			.slice(0, 8);
	});

	function go(id: string) {
		const r = map.rooms.find((x) => x.id === id);
		if (!r) return;
		query = '';
		selected = id;
		onselect(id);
		if (floor === r.level || (floor === null && here?.level === r.level)) center(id);
		floor = r.level;
	}

	const sorted = $derived([...rooms].filter((r) => shownFloor === 'all' || r.level === shownFloor).sort((a, b) => a.name.localeCompare(b.name)));
	const short = (name: string) => (name.length > 21 ? `${name.slice(0, 20).trimEnd()}…` : name);
	const roomById = (id: string) => map.rooms.find((r) => r.id === id);
</script>

<!--
	The Atlas map: the whole world at full size. Floors on the left; the map in the middle can pan (drag, or the
	arrow keys) and zoom (wheel, + and −; F fits the floor). Search finds rooms and things. The list view is the
	same floor as a table: it is the accessible twin of the drawing.
-->
<div class="atlas-map">
	<nav class="floors" aria-label="Floors">
		<ul>
			{#each levels as l (l)}
				{@const count = map.rooms.filter((r) => r.level === l).length}
				<li>
					<button type="button" class="floor" aria-current={shownFloor === l ? 'true' : undefined} onclick={() => (floor = l)}>
						<b>Level {l}</b>
						<span>{count} {count === 1 ? 'room' : 'rooms'}{here?.level === l ? ' · you' : ''}</span>
					</button>
				</li>
			{/each}
			{#if levels.length > 1}
				<li>
					<button type="button" class="floor" aria-current={shownFloor === 'all' ? 'true' : undefined} onclick={() => (floor = 'all')}>
						<b>All floors</b><span>unfolded</span>
					</button>
				</li>
			{/if}
		</ul>
	</nav>

	<div class="main">
		<div class="bar">
			<div class="search">
				<label class="visually-hidden" for="atlas-search">Find a room or a thing</label>
				<input id="atlas-search" type="search" placeholder="Find a room or a thing…  /" bind:value={query} autocomplete="off" />
				{#if results.length}
					<ul class="results" aria-label="Results">
						{#each results as r (r.id)}
							<li><button type="button" onclick={() => go(r.id)}>{r.name} <span>level {r.level}</span></button></li>
						{/each}
					</ul>
				{/if}
			</div>
			{#if !list && shownFloor !== 'all'}
				<Button size="compact" variant="quiet" onclick={() => zoom(1 / 1.2)} aria-label="Zoom out">−</Button>
				<span class="zoom" aria-live="polite">{Math.round(scale * 100)}%</span>
				<Button size="compact" variant="quiet" onclick={() => zoom(1.2)} aria-label="Zoom in">+</Button>
				<Button size="compact" variant="quiet" onclick={fit}>Fit</Button>
			{/if}
			<Button size="compact" pressed={list} onclick={() => (list = !list)}>List view</Button>
		</div>

		{#if list}
			<div class="table-wrap">
				<table>
					<caption class="visually-hidden">Rooms {shownFloor === 'all' ? 'on all floors' : `on level ${shownFloor}`}</caption>
					<thead>
						<tr><th scope="col">Room</th><th scope="col">Level</th><th scope="col">Visited</th><th scope="col">Exits</th><th scope="col">Things</th></tr>
					</thead>
					<tbody>
						{#each sorted as r (r.id)}
							<tr class:current={r.current} class:selected={r.id === selected}>
								<th scope="row"><button type="button" class="room-link" onclick={() => pick(r.id)}>{r.name}</button>{r.current ? ' (you)' : ''}</th>
								<td>{r.level}</td>
								<td>{r.visited ? 'yes' : 'no'}</td>
								<td>{r.exits.map(([d]) => d).join(', ')}</td>
								<td>{r.things.map((t) => t.name).join(', ')}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{:else if shownFloor === 'all'}
			<div class="unfolded"><MapPanel {map} compact view="unfolded" /></div>
		{:else}
			<!-- The drawing is for the pointer and the arrow keys. The list view has the same rooms for screen readers. -->
			<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
			<div
				class="canvas"
				bind:this={canvas}
				bind:clientWidth={width}
				bind:clientHeight={height}
				tabindex="0"
				role="application"
				aria-label="Map of level {shownFloor}. Arrow keys move, plus and minus zoom, F fits. The list view has the same rooms as a table."
				onpointerdown={down}
				onpointermove={move}
				onpointerup={up}
				onwheel={wheel}
				onkeydown={keys}
			>
				<svg {width} {height} aria-hidden="true">
					<g transform="scale({scale}) translate({-ox} {-oy})">
						{#each floorLinks as link (link.from + link.to + link.direction)}
							{@const a = roomById(link.from)}
							{@const b = roomById(link.to)}
							{#if a && b}
								<line x1={px(a.x)} y1={py(a.y)} x2={px(b.x)} y2={py(b.y)} class="link" class:hidden-link={!link.visible} />
							{/if}
						{/each}
						{#each floorRooms as r (r.id)}
							{@const up = map.links.some((l) => (l.from === r.id && l.direction === 'up') || (l.to === r.id && l.direction === 'down'))}
							{@const downStair = map.links.some((l) => (l.from === r.id && l.direction === 'down') || (l.to === r.id && l.direction === 'up'))}
							<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
							<g class="room" class:current={r.current} class:selected={r.id === selected} class:unvisited={!r.visited} onclick={() => pick(r.id)}>
								<rect x={px(r.x) - BOX_W / 2} y={py(r.y) - BOX_H / 2} width={BOX_W} height={BOX_H} />
								<text x={px(r.x)} y={py(r.y) + 4} text-anchor="middle">{short(r.name)}<title>{r.name}</title></text>
								{#if up || downStair}
									<text x={px(r.x) + BOX_W / 2 - 6} y={py(r.y) - BOX_H / 2 + 13} text-anchor="end" class="stairs"
										>{up ? '▲' : ''}{downStair ? '▼' : ''}</text
									>
								{/if}
							</g>
						{/each}
					</g>
				</svg>
			</div>
		{/if}
	</div>
</div>

<style>
	.atlas-map {
		display: grid;
		grid-template-columns: 11rem minmax(0, 1fr);
		height: 100%;
		min-height: 0;
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.floors {
		border-right: var(--size-border) solid var(--color-border-subtle);
		background: var(--color-surface-raised);
		overflow-y: auto;
	}
	.floors ul {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: 0;
		padding: var(--space-4);
		list-style: none;
	}
	.floor {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		width: 100%;
		padding: var(--space-2) var(--space-3);
		border: var(--size-border) solid var(--color-border-subtle);
		background: none;
		color: var(--color-text-muted);
		font: inherit;
		font-size: var(--font-size-xs);
		text-align: left;
	}
	.floor b {
		color: var(--color-text-strong);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-regular);
	}
	.floor[aria-current='true'] {
		border-color: var(--color-accent);
		background: var(--color-accent-tint);
	}
	.floor[aria-current='true'] b {
		color: var(--color-text-accent);
	}
	.floor:focus-visible,
	.room-link:focus-visible,
	.results button:focus-visible,
	input:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.main {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
	}
	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
	}
	.search {
		position: relative;
		flex: 0 1 22rem;
	}
	input {
		box-sizing: border-box;
		width: 100%;
		min-height: var(--size-target-compact);
		padding: 0 var(--space-3);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface);
		color: var(--color-text);
		font: inherit;
	}
	.results {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		z-index: 5;
		margin: 0;
		padding: var(--space-2) 0;
		list-style: none;
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		box-shadow: var(--shadow-overlay);
	}
	.results button {
		display: flex;
		justify-content: space-between;
		width: 100%;
		min-height: var(--size-target-compact);
		padding: 0 var(--space-3);
		border: 0;
		background: none;
		color: var(--color-text);
		font: inherit;
		text-align: left;
	}
	.results button:hover {
		background: var(--color-accent-tint);
	}
	.results span {
		color: var(--color-text-muted);
	}
	.zoom {
		min-width: 4ch;
		text-align: center;
		color: var(--color-text-muted);
	}
	.canvas {
		position: relative;
		flex: 1 1 auto;
		min-height: 20rem;
		overflow: hidden;
		cursor: grab;
		touch-action: none;
		background-color: var(--color-surface);
		background-image:
			linear-gradient(var(--color-border-subtle) 1px, transparent 1px),
			linear-gradient(90deg, var(--color-border-subtle) 1px, transparent 1px);
		background-size: 40px 40px;
	}
	.canvas:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: -2px;
	}
	svg {
		position: absolute;
		inset: 0;
		display: block;
	}
	.link {
		stroke: var(--color-border-control);
		stroke-width: 1;
	}
	.hidden-link {
		stroke-dasharray: 4 5;
	}
	.room {
		cursor: pointer;
	}
	.room rect {
		fill: var(--color-surface-raised);
		stroke: var(--color-border-control);
		stroke-width: 1;
	}
	.room text {
		fill: var(--color-text-muted);
		font-size: 13px;
	}
	.room.unvisited rect {
		stroke-dasharray: 3 4;
	}
	.room.current rect {
		stroke: var(--color-accent);
		stroke-width: 1.5;
	}
	.room.current text {
		fill: var(--color-text-accent);
	}
	.room.selected rect {
		fill: var(--color-accent-tint);
		stroke: var(--color-accent);
		stroke-width: 2;
	}
	.room.selected text {
		fill: var(--color-text-strong);
	}
	.stairs {
		fill: var(--color-text-accent);
		font-size: 11px;
	}
	.unfolded {
		padding: var(--space-4);
		overflow: auto;
	}
	.table-wrap {
		overflow: auto;
		padding: var(--space-4);
	}
	table {
		border-collapse: collapse;
		width: 100%;
	}
	th,
	td {
		padding: var(--space-2) var(--space-4) var(--space-2) 0;
		border-bottom: var(--size-border) solid var(--color-border-subtle);
		text-align: left;
		vertical-align: top;
	}
	thead th {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
		font-weight: var(--font-weight-regular);
	}
	tbody th {
		font-weight: var(--font-weight-regular);
	}
	tr.current .room-link {
		color: var(--color-text-accent);
	}
	tr.selected {
		background: var(--color-accent-tint);
	}
	.room-link {
		min-height: var(--size-target-compact);
		padding: 0;
		border: 0;
		background: none;
		color: var(--color-text-strong);
		font: inherit;
		text-align: left;
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
	@media (max-width: 719px) {
		.atlas-map {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: auto minmax(0, 1fr);
		}
		.floors {
			border-right: 0;
			border-bottom: var(--size-border) solid var(--color-border-subtle);
		}
		.floors ul {
			flex-direction: row;
			overflow-x: auto;
		}
		.floor {
			width: auto;
			white-space: nowrap;
		}
	}
</style>
