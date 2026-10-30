<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { proposalMap, towerMap, towerRooms } from '../fixtures.ts';
	import type { AtlasRoom } from '#lib/engine/types.ts';

	const proposalRooms: AtlasRoom[] = proposalMap.rooms.map((r) => ({
		...r,
		things: [],
		exits: proposalMap.links.filter((l) => l.from === r.id).map((l) => [l.direction, l.to] as [string, string]),
		fragments: 1
	}));
	import AtlasMap from './AtlasMap.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/AtlasMap',
		component: AtlasMap,
		tags: ['autodocs'],
		args: { map: towerMap, rooms: towerRooms, selected: 'north_2', onselect: fn() },
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The Atlas map shows the whole world at full size.

- **Floors** on the left: each level with its number of rooms, and "you" on the level of the player. "All floors, unfolded" shows the unfolded map of MapPanel.
- **The map**: drag or use the arrow keys to move, the wheel or + and − to zoom, F (or "Fit") to see the whole floor. A floor opens with the selected room (or the player) in the middle, at a size to read. ▲ and ▼ mark rooms with stairs. A click selects a room.
- **Search** ("/" focuses it) finds rooms by name or id, and rooms with a thing of that name. A result shows its floor and selects the room.
- **List view** shows the rooms of the floor as a table: name, level, visited, exits and things. It is the accessible twin of the drawing; the drawing itself is for the pointer and the arrow keys.

The selected room goes to the room card (RoomCard) next to the map.

**AI proposals**: rooms and exits that open proposals add are dashed in the ghost colour, with ✦ before the name; what they remove is in the warning colour. A proposed room can be selected: its room card offers Walk in, Accept and Reject. An exit that only prints a message is a short line that ends in a bar.`
				}
			},
			story: { iframeHeight: 560 }
		}
	});
</script>

<Story name="One floor" asChild>
	<div style="height: 540px">
		<AtlasMap map={towerMap} rooms={towerRooms} selected="north_2" onselect={() => {}} />
	</div>
</Story>
<Story name="With an AI proposal" asChild>
	<div style="height: 540px">
		<AtlasMap map={proposalMap} rooms={proposalRooms} selected="sea_rocks" onselect={() => {}} />
	</div>
</Story>
