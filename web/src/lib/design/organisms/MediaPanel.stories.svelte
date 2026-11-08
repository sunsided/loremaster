<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import MediaPanel from './MediaPanel.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/MediaPanel',
		component: MediaPanel,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: 'The active illustration and portrait of the story. Without a file for an image, its text shows.'
				}
			}
		}
	});

	const picture = (fill: string) =>
		`data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 100"><rect width="160" height="100" fill="${fill}"/></svg>`)}`;
	const illustration = { src: picture('#5b5448'), alt: 'A stone wall at dusk.' };
	const portrait = { src: picture('#c9a07a'), alt: 'An old keeper with a stern face.' };
</script>

<Story name="Illustration" args={{ illustration }} />
<Story name="With portrait" args={{ illustration, portrait }} />
<Story name="No file" args={{ illustration: { src: null, alt: 'A stone wall at dusk.' } }} />
