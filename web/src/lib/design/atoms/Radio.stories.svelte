<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import Radio from './Radio.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/Radio',
		component: Radio,
		tags: ['autodocs'],
		args: { name: 'demo', value: 'paper', checked: false, onselect: fn() },
		parameters: {
			docs: {
				description: {
					component: `A radio is one choice in a group of choices. Only one choice in a group can be selected.

Do not use a Radio alone. Use RadioGroup. RadioGroup gives the group a legend and keeps the value.`
				}
			}
		}
	});
</script>

<Story name="Unselected">Paper</Story>
<Story name="Selected" args={{ checked: true }}>Paper</Story>
