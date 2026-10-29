<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Tag from './Tag.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/Tag',
		component: Tag,
		tags: ['autodocs'],
		argTypes: { tone: { control: 'inline-radio', options: ['neutral', 'warning'] } },
		parameters: {
			docs: {
				description: {
					component: `A tag shows a value, such as the time or the room. It is not a control. Do not attach a click to a tag. Use a Button for an action.

Use the **warning** tone for a state that needs attention.`
				}
			}
		}
	});
</script>

<Story name="Neutral">day 1 · 20:30 · dusk</Story>
<Story name="Warning" args={{ tone: 'warning' }}>2 problems</Story>
