<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import TextInput from './TextInput.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/TextInput',
		component: TextInput,
		tags: ['autodocs'],
		args: { id: 'demo-input', label: 'Command', placeholder: 'type a command' },
		parameters: {
			docs: {
				description: {
					component: `A text input lets the player or the author type one line of text.

Every input has a label. If the design does not show a label, set \`hideLabel\`. The label stays in the page for screen readers.
Do not use the placeholder as the label. The placeholder goes away when the user types.

The focus look is a thicker line under the input in the focus colour.`
				}
			}
		}
	});
</script>

<Story name="Visible label" />
<Story name="Hidden label" args={{ hideLabel: true }} />
<Story name="With value" args={{ value: 'unlock iron door with key' }} />
