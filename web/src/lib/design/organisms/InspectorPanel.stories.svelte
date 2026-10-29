<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fragments, history, map, parts, rules } from '../fixtures.ts';
	import AssemblyPanel from './AssemblyPanel.svelte';
	import InspectorPanel from './InspectorPanel.svelte';
	import MapPanel from './MapPanel.svelte';
	import RulesView from './RulesView.svelte';
	import StoryTimeline from './StoryTimeline.svelte';

	const tabs = [
		{ id: 'assembly', label: 'Assembly' },
		{ id: 'map', label: 'Map' },
		{ id: 'story', label: 'Story' },
		{ id: 'rules', label: 'Rules' }
	];

	const { Story } = defineMeta({
		title: 'Organisms/InspectorPanel',
		component: InspectorPanel,
		tags: ['autodocs'],
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The inspector panel is the side panel next to the player. Its tabs select one inspector:

- **Assembly** shows how the runtime makes the text of the current room.
- **Map** shows all rooms and exits.
- **Story** shows the playthrough so far.
- **Rules** shows all rules with their state.

The player stays visible for all tabs. Thus the author can play and examine the story at the same time.
On ultrawide screens the map has its own column. Then the page removes the Map tab.
The tab panel scrolls on its own and can get keyboard focus.`
				},
				story: { inline: false, iframeHeight: 720 }
			}
		}
	});
</script>

<script lang="ts">
	let selected = $state('assembly');
</script>

<Story name="All tabs" asChild>
	<div style="height: 700px; max-width: 480px; display: flex; flex-direction: column">
		<InspectorPanel {tabs} bind:selected>
			{#if selected === 'map'}
				<MapPanel {map} />
			{:else if selected === 'story'}
				<StoryTimeline steps={history} />
			{:else if selected === 'rules'}
				<RulesView {rules} />
			{:else}
				<AssemblyPanel room="shore" {fragments} {parts} time="20:30" phase="dusk" />
			{/if}
		</InspectorPanel>
	</div>
</Story>
