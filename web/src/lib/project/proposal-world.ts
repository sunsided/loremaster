// The world after an AI proposal: its files, and the rooms and exits that it adds or removes.
// The map draws them as ghosts, and "Walk in" plays them in a sandbox before the author accepts.

import type { MapView, SourceFile } from '#lib/engine/types.ts';
import type { ProposalView } from './proposals.svelte.ts';
import { applyChanges } from './workspace.svelte.ts';

/** The files of the editor as the proposal leaves them. A file with `after: null` is removed. */
export function filesAfter(files: SourceFile[], view: ProposalView): SourceFile[] {
	return applyChanges(
		files,
		view.files.map((f) => ({ path: f.path, text: f.after }))
	);
}

/** True when a proposal can change the map: it adds, removes or changes an entity, not only text. */
export function changesWorld(view: ProposalView): boolean {
	return view.changes.some((c) => c.kind !== 'fragment');
}

/** A link between the same two rooms, in either order. A two-way link is listed once, from either side. */
const pairKey = (from: string, to: string) => [from, to].sort().join('|');

/**
 * The map with the rooms, links and message exits of the proposals, marked with the proposal that adds
 * (`proposed`) or removes (`removed`) them. The positions come from the files, so they match.
 * The first proposal that adds a room or a link wins.
 */
export function mapWithProposals(map: MapView, after: { id: string; map: MapView }[]): MapView {
	const rooms = [...map.rooms];
	const links = [...map.links];
	const ends = [...(map.ends ?? [])];
	const roomIds = new Set(rooms.map((r) => r.id));
	const pairs = new Set(links.map((l) => pairKey(l.from, l.to)));
	const endKey = (e: { room: string; direction: string }) => `${e.room}>${e.direction}`;
	const endKeys = new Set(ends.map(endKey));
	for (const { id, map: m } of after) {
		for (const r of m.rooms)
			if (!roomIds.has(r.id)) {
				roomIds.add(r.id);
				rooms.push({ ...r, current: false, visited: false, proposed: id });
			}
		for (const l of m.links)
			if (!pairs.has(pairKey(l.from, l.to))) {
				pairs.add(pairKey(l.from, l.to));
				links.push({ ...l, proposed: id });
			}
		for (const e of m.ends ?? [])
			if (!endKeys.has(endKey(e))) {
				endKeys.add(endKey(e));
				ends.push({ ...e, proposed: id });
			}
		// What the proposal removes: a room, a link or a message exit that its map does not have.
		const keepRooms = new Set(m.rooms.map((r) => r.id));
		for (const [i, r] of rooms.entries()) if (!r.proposed && !r.removed && !keepRooms.has(r.id)) rooms[i] = { ...r, removed: id };
		const keep = new Set(m.links.map((l) => pairKey(l.from, l.to)));
		const keepEnds = new Set((m.ends ?? []).map(endKey));
		for (const [i, l] of links.entries())
			if (!l.proposed && !l.removed && !keep.has(pairKey(l.from, l.to))) links[i] = { ...l, removed: id };
		for (const [i, e] of ends.entries()) if (!e.proposed && !e.removed && !keepEnds.has(endKey(e))) ends[i] = { ...e, removed: id };
	}
	return { rooms, links, ends };
}
