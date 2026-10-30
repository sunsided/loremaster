<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { entries, exits, fragments, inventory, map as lighthouseMap, parts, things, verbs } from '../fixtures.ts';
	import InspectorPanel from '../organisms/InspectorPanel.svelte';
	import CommandPrompt from '../molecules/CommandPrompt.svelte';
	import StatusLine from '../organisms/StatusLine.svelte';
	import AppHeader from '../organisms/AppHeader.svelte';
	import AssemblyPanel from '../organisms/AssemblyPanel.svelte';
	import MapPanel from '../organisms/MapPanel.svelte';
	import ScreenEffects from '../organisms/ScreenEffects.svelte';
	import Transcript from '../organisms/Transcript.svelte';
	import VerbBar from '../organisms/VerbBar.svelte';
	import PlayerLayout from './PlayerLayout.svelte';

	const { Story } = defineMeta({
		title: 'Templates/PlayerLayout',
		component: PlayerLayout,
		tags: ['autodocs'],
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The player layout is the frame of the play-and-write page. It has a header, the player screen and the side panel.

The screen starts with the status line (the \`status\` snippet), edge to edge, then the player.

The layout has four steps:
1. Below 900px (\`--bp-stack\`): one column. The panel goes under the player, and the page scrolls.
2. From 900px: the player and the panel are side by side. Each part scrolls on its own.
3. From 2200px with an aspect ratio of 21:9 or more (\`--bp-ultrawide\`): the map shows on the left. This is for 21:9, 24:9 and 32:9 screens.
4. Wider than \`--width-content-max\` (2400px): the content stays at that width, in the center.

The first element is a skip link. It shows when it gets focus. It moves focus to the command input.
The player screen is the \`main\` landmark. The panel holds the InspectorPanel with its tabs. The player stays on the screen for all inspector tabs.

Use the viewport menu in the toolbar to see the layout at phone width. To see the map, open the story in a new tab on a screen that is 2200px wide or more.`
				},
				story: { inline: false, iframeHeight: 720 }
			}
		}
	});
</script>

<Story name="Play and write" asChild>
	<PlayerLayout fxClass="fx-subtle" glow="var(--fx-glow-soft)">
		{#snippet header()}
			<AppHeader title="The Lighthouse" saved="bundled example" onmode={() => {}} onsettings={() => {}} onpalette={() => {}} />
		{/snippet}
		{#snippet effects()}
			<ScreenEffects theme="phosphor" effects="subtle" />
		{/snippet}
		{#snippet status()}
			<StatusLine room="The Shore" day={1} time="20:35" phase="dusk" turn={1} turnItems={[{ id: 'restart', label: 'Restart the game', run: () => {} }]} />
		{/snippet}
		{#snippet player()}
			<Transcript {entries} room="shore" onthing={() => {}} onexit={() => {}} />
			<CommandPrompt oncommand={() => {}} />
			<VerbBar verbs={verbs} {things} {inventory} {exits} onverb={() => {}} onthing={() => {}} onexit={() => {}} />
		{/snippet}
		{#snippet map()}
			<MapPanel map={lighthouseMap} framed />
		{/snippet}
		{#snippet panel()}
			<InspectorPanel tabs={[{ id: 'assembly', label: 'Assembly' }, { id: 'story', label: 'Story' }, { id: 'rules', label: 'Rules' }]} selected="assembly">
				<AssemblyPanel room="shore" {fragments} {parts} time="20:30" phase="dusk" />
			</InspectorPanel>
		{/snippet}
	</PlayerLayout>
</Story>
