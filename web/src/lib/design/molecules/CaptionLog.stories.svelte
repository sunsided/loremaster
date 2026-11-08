<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import CaptionLog from './CaptionLog.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/CaptionLog',
		component: CaptionLog,
		tags: ['autodocs'],
		parameters: {
			docs: { description: { component: 'The text of the newest sounds, for a player with the sound off or the captions on.' } }
		}
	});
</script>

<Story
	name="Default"
	args={{
		captions: [
			{ id: 1, text: '[Wind across the road.]' },
			{ id: 2, text: '[The gate creaks on its hinges.]' }
		]
	}}
/>
