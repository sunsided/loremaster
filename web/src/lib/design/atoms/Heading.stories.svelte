<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Heading from './Heading.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/Heading',
		component: Heading,
		tags: ['autodocs'],
		args: { level: 2, look: 'section' },
		argTypes: {
			level: { control: 'inline-radio', options: [1, 2, 3, 4] },
			look: { control: 'inline-radio', options: ['title', 'section'] }
		},
		parameters: {
			docs: {
				description: {
					component: `A heading names a region or a section.

\`level\` sets the place in the document outline (h1 to h4). \`look\` sets the appearance. The two are separate, so the outline stays correct when the design changes.
Do not skip levels: an h3 must be under an h2.`
				}
			}
		}
	});
</script>

<Story name="Section">Ambient</Story>
<Story name="Title" args={{ look: 'title' }}>Look and motion</Story>
