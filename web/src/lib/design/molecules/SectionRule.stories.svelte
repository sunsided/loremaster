<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import SectionRule from './SectionRule.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/SectionRule',
		component: SectionRule,
		tags: ['autodocs'],
		args: { title: 'ambient', level: 3 },
		parameters: {
			docs: {
				description: {
					component: `A section rule is a small heading with a line that fills the row. Use it to divide a panel into sections.

The line is decoration. Screen readers ignore it. Set \`level\` to the correct place in the document outline.`
				}
			}
		}
	});
</script>

<Story name="Default" />
