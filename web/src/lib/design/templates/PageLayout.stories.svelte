<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { rules } from '../fixtures.ts';
	import AppHeader from '../organisms/AppHeader.svelte';
	import RulesView from '../organisms/RulesView.svelte';
	import PageLayout from './PageLayout.svelte';

	const { Story } = defineMeta({
		title: 'Templates/PageLayout',
		component: PageLayout,
		tags: ['autodocs'],
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The page layout is the frame of the editor views Map, Story and Rules: the header and one main region.

The main region scrolls. Its content stays in the center, not wider than \`maxWidth\`. On screens wider than \`--width-content-max\` the whole frame stays in the center.
The first element is a skip link to the main region.`
				},
				story: { inline: false, iframeHeight: 640 }
			}
		}
	});
</script>

<Story name="Rules view" asChild>
	<PageLayout>
		{#snippet header()}
			<AppHeader
				title="The Lighthouse"
				views={[
					{ id: 'play', label: 'Play and write' },
					{ id: 'map', label: 'Map' },
					{ id: 'story', label: 'Story' },
					{ id: 'rules', label: 'Rules' }
				]}
				current="rules"
				onsettings={() => {}}
			/>
		{/snippet}
		<RulesView {rules} />
	</PageLayout>
</Story>
