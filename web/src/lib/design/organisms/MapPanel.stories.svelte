<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import { map, proposalMap, towerMap } from '../fixtures.ts';
	import MapPanel from './MapPanel.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/MapPanel',
		component: MapPanel,
		tags: ['autodocs'],
		args: { map },
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `**Levels.** When the rooms are on more than one level, the map has a "Map view" choice. "One level" shows the level of the player (or the level that "Up" and "Down" pick) in full, with the levels next to it faint behind it; ▲ and ▼ mark rooms with stairs. "All levels, unfolded" puts each level in one row, the highest at the top, with the rooms in ring order; each row starts one column further right, so a spiral reads as a staircase. Long names are cut to fit; the full name is in the tooltip and in the list for screen readers.

The map panel shows all rooms of the story and the links between them.

Each room has its grid position from the story file (\`position\`). A room without a position goes on a row under the map.
- The current room has the accent colour.
- A visited room has a solid edge.
- A room that the player did not visit yet has a dashed edge and muted text.
- A hidden exit has a dashed line. A one-way exit has an arrow.
- Up, down, in, out and named exits have a label, because the line alone does not show the direction.
- An exit that only prints a message (it leads nowhere) is a short line out of the room that ends in a bar.
- **AI proposals**: while \`lantern serve\` has open proposals that add or remove rooms or exits, the map shows them. What a proposal adds is dashed in the ghost colour, and a new room has ✦ before its name. What it removes is in the warning colour, and a removed room is struck through. The legend explains the marks, and the list for screen readers says "proposed by AI" or "an AI proposal removes it".

The drawing is for sighted users only. The same facts are in a list for screen readers: each room with its state, then each connection.

The map shows in the Map tab of the inspector. On ultrawide screens (21:9 and wider) it has its own column on the left; then set \`framed\`, so it gets its own padding, background and scroll.`
				}
			}
		}
	});
</script>

{#snippet template(args: ComponentProps<typeof MapPanel>)}
	<div style="height: 520px; max-width: 640px; display: flex">
		<MapPanel {...args} />
	</div>
{/snippet}

<Story name="Lighthouse" {template} />
<Story
	name="Hidden and one-way exits"
	args={{
		map: {
			rooms: [
				{ id: 'hall', name: 'Hall', x: 0, y: 0, level: 0, visited: true, current: true },
				{ id: 'study', name: 'Study', x: 1, y: 0, level: 0, visited: false, current: false },
				{ id: 'cellar', name: 'Cellar', x: 0, y: 1, level: -1, visited: false, current: false },
				{ id: 'mirror', name: 'Behind the Mirror', x: 1, y: 1, level: 0, visited: false, current: false }
			],
			links: [
				{ from: 'hall', to: 'study', direction: 'east', visible: true, one_way: false },
				{ from: 'hall', to: 'cellar', direction: 'down', visible: true, one_way: false },
				{ from: 'study', to: 'mirror', direction: 'through the mirror', visible: false, one_way: true }
			]
		}
	}}
	{template}
/>
<Story name="Spiral tower, one level" args={{ map: towerMap }} {template} />
<Story name="Spiral tower, unfolded" args={{ map: towerMap }} {template} play={async ({ canvasElement }) => {
	canvasElement.querySelector<HTMLInputElement>('input[value="unfolded"]')?.click();
}} />
<Story name="Message exits and an AI proposal" args={{ map: proposalMap }} {template} />
