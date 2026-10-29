<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import { map } from '../fixtures.ts';
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
					component: `The map panel shows all rooms of the story and the links between them.

Each room has its grid position from the story file (\`position\`). A room without a position goes on a row under the map.
- The current room has the accent colour.
- A visited room has a solid edge.
- A room that the player did not visit yet has a dashed edge and muted text.
- A hidden exit has a dashed line. A one-way exit has an arrow.
- Up, down, in, out and named exits have a label, because the line alone does not show the direction.

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
				{ id: 'hall', name: 'Hall', x: 0, y: 0, visited: true, current: true },
				{ id: 'study', name: 'Study', x: 1, y: 0, visited: false, current: false },
				{ id: 'cellar', name: 'Cellar', x: 0, y: 1, visited: false, current: false },
				{ id: 'mirror', name: 'Behind the Mirror', x: 1, y: 1, visited: false, current: false }
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
