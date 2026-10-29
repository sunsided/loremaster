<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Select from './Select.svelte';

	const options = ['base', 'ambient', 'event'].map((v) => ({ value: v, label: v }));

	const { Story } = defineMeta({
		title: 'Atoms/Select',
		component: Select,
		tags: ['autodocs'],
		args: { label: 'Layer', value: 'ambient', options },
		parameters: {
			docs: {
				description: {
					component: `A native select with a visible label. Use it for a short, fixed list of choices, for example the layer of a fragment.

The browser gives the select its keyboard and screen-reader support. The control is 32px high. Set \`hideLabel\` only when the context gives the name; screen readers still read the label.`
				}
			}
		}
	});
</script>

<Story name="Default" />
<Story name="Hidden label" args={{ hideLabel: true }} />
