<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { entries, exits, fragments, inventory, things, verbs } from '../fixtures.ts';
	import CommandPrompt from '../molecules/CommandPrompt.svelte';
	import StatusBar from '../molecules/StatusBar.svelte';
	import AppHeader from '../organisms/AppHeader.svelte';
	import AssemblyPanel from '../organisms/AssemblyPanel.svelte';
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

On wide screens the player and the panel are side by side. Each part scrolls on its own.
Below 900px (\`--bp-stack\`) the panel goes under the player, and the page scrolls.

The first element is a skip link. It shows when it gets focus. It moves focus to the command input.
The player screen is the \`main\` landmark. The panel is a complementary landmark.

Use the viewport menu in the toolbar to see the layout at phone width.`
				},
				story: { inline: false, iframeHeight: 720 }
			}
		}
	});
</script>

<Story name="Play and write" asChild>
	<PlayerLayout fxClass="fx-subtle" glow="var(--fx-glow-soft)">
		{#snippet header()}
			<AppHeader
				title="The Lighthouse"
				views={[{ id: 'play', label: 'Play and write' }, { id: 'map', label: 'Map', disabled: true }]}
				current="play"
				now={['day 1 · 20:30 · dusk', 'The Shore']}
				onsettings={() => {}}
			/>
		{/snippet}
		{#snippet effects()}
			<ScreenEffects theme="phosphor" effects="subtle" />
		{/snippet}
		{#snippet player()}
			<StatusBar items={['turn 1', '20:35', 'shore']} />
			<Transcript {entries} room="shore" onthing={() => {}} onexit={() => {}} />
			<CommandPrompt oncommand={() => {}} />
			<VerbBar verbs={verbs} {things} {inventory} {exits} onverb={() => {}} onthing={() => {}} onexit={() => {}} />
		{/snippet}
		{#snippet panel()}
			<AssemblyPanel room="shore" {fragments} time="20:30" phase="dusk" />
		{/snippet}
	</PlayerLayout>
</Story>
