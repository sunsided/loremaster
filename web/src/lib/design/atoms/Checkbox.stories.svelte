<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Checkbox from './Checkbox.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/Checkbox',
		component: Checkbox,
		tags: ['autodocs'],
		args: { checked: true },
		parameters: {
			docs: {
				description: {
					component: `A checkbox turns one option on or off.

The label is part of the control: a click on the label changes the value. Write the label as the option, not as a question.
Use \`hint\` for a short fact after the label, such as the current system setting.`
				}
			}
		}
	});
</script>

<Story name="Checked">Show fragment numbers in the transcript</Story>
<Story name="Unchecked" args={{ checked: false }}>Show fragment numbers in the transcript</Story>
<Story name="With hint" args={{ hint: '(now: on)' }}>Reveal new text line by line</Story>
