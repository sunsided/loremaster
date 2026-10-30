<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { towerMap, towerRooms } from '../fixtures.ts';
	import AppHeader from '../organisms/AppHeader.svelte';
	import AtlasMap from '../organisms/AtlasMap.svelte';
	import RoomCard from '../molecules/RoomCard.svelte';
	import AtlasLayout from './AtlasLayout.svelte';

	const tabs = [
		{ id: 'map', label: 'Map' },
		{ id: 'rules', label: 'Rules' },
		{ id: 'state', label: 'State' },
		{ id: 'story', label: 'Story' },
		{ id: 'checks', label: 'Checks' },
		{ id: 'proposals', label: 'Proposals 1' }
	];

	const { Story } = defineMeta({
		title: 'Templates/AtlasLayout',
		component: AtlasLayout,
		tags: ['autodocs'],
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The Atlas layout is the frame of the world view. It takes the place of the player screen: the app opens it with Ctrl+M or the room name in the status line, and Escape or "Back to …" in the header returns to the game at the same turn.

It has the header, a row of tabs (a \`nav\` with \`aria-current\`), the view, and an optional \`aside\` on the right, for example the room card of the map. Each view gets the full width, so tabs are fine here.
Below 900px the aside goes under the view.`
				}
			},
			story: { inline: false, iframeHeight: 640 }
		}
	});
</script>

<Story name="Map with a room card" asChild>
	<AtlasLayout {tabs} selected="map" onselect={() => {}}>
		{#snippet header()}
			<AppHeader title="The Spiral" saved="bundled example" back={{ label: 'Back to North Landing', run: () => {} }} onsettings={() => {}} onpalette={() => {}} />
		{/snippet}
		<AtlasMap map={towerMap} rooms={towerRooms} selected="bell_chamber" onselect={() => {}} />
		{#snippet aside()}
			<RoomCard room={towerRooms.at(-1)!} ongo={() => {}} onwrite={() => {}} onexplore={() => {}} />
		{/snippet}
	</AtlasLayout>
</Story>
