// Shared drawing helpers of the map views (MapPanel and AtlasMap).

/** The way out of a room for each direction. A named direction points down and right. */
const END_DIR: Record<string, [number, number]> = {
	north: [0, -1],
	south: [0, 1],
	east: [1, 0],
	west: [-1, 0],
	northeast: [1, -1],
	northwest: [-1, -1],
	southeast: [1, 1],
	southwest: [-1, 1],
	up: [0.6, -1],
	down: [-0.6, 1]
};

/**
 * The mark of a message exit: a short line from the edge of the room box outward, and a bar at its end.
 * `x` and `y` are the middle of the box.
 */
export function endLine(x: number, y: number, boxW: number, boxH: number, direction: string, length = 30) {
	const [dx, dy] = END_DIR[direction] ?? [1, 1];
	const n = Math.hypot(dx, dy);
	const ux = dx / n;
	const uy = dy / n;
	const t = Math.min(ux === 0 ? Infinity : boxW / 2 / Math.abs(ux), uy === 0 ? Infinity : boxH / 2 / Math.abs(uy));
	const x1 = x + ux * t;
	const y1 = y + uy * t;
	const x2 = x1 + ux * length;
	const y2 = y1 + uy * length;
	return { x1, y1, x2, y2, bar: `M${x2 - uy * 6} ${y2 + ux * 6}L${x2 + uy * 6} ${y2 - ux * 6}` };
}

/** Words for screen readers about an AI proposal that adds or removes a part of the map. */
export const aiNote = (x: { proposed?: string; removed?: string }) =>
	x.proposed ? ', proposed by AI' : x.removed ? ', an AI proposal removes it' : '';
