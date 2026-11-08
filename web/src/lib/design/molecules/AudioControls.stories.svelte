<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import AudioControls from './AudioControls.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/AudioControls',
		component: AudioControls,
		tags: ['autodocs'],
		parameters: {
			docs: { description: { component: 'The sound switch, the captions switch and one volume for music, ambience and effects.' } }
		}
	});
</script>

<Story name="Off" />
<Story name="On with captions" args={{ sound: true, captions: true }} />
