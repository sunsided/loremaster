<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import StateSwitch from './StateSwitch.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/StateSwitch',
		component: StateSwitch,
		tags: ['autodocs'],
		args: { name: 'time.phase', value: 'dusk', options: ['night', 'dawn', 'day', 'dusk'], onchange: fn() },
		parameters: {
			docs: {
				description: {
					component: `One value that the room text depends on, as \`name | value ▾\`. It is a native select; the value name is its label.

With \`changed\`, the switch shows a value that differs from the game now (a preview): the border and the value are in the warning colour.`
				}
			}
		}
	});
</script>

<Story name="Value now" />
<Story name="Preview value" args={{ value: 'night', changed: true }} />
