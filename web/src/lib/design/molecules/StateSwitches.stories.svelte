<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import StateSwitches from './StateSwitches.svelte';

	const switches = [
		{ name: 'beacon_lit', value: 'false', options: ['false', 'true'] },
		{ name: 'time.phase', value: 'dusk', options: ['night', 'dawn', 'day', 'dusk'] },
		{ name: 'first_visit', value: 'true', options: ['true', 'false'] }
	];

	const { Story } = defineMeta({
		title: 'Molecules/StateSwitches',
		component: StateSwitches,
		tags: ['autodocs'],
		args: { switches, preview: {}, onpreview: fn(), onreset: fn(), onapply: fn() },
		parameters: {
			docs: {
				description: {
					component: `The values that the room text depends on, as switches (StateSwitch). The Here panel, the room desk and the pinned room over the fragment writer use this block. Thus the three places always look and work the same.

- A change is a **preview**. The room text changes in place. The game does not move. A changed switch shows in the warning colour.
- **Make it so** sets the previewed values in the game. **Reset** ends the preview. Leave out \`onapply\` and \`onreset\` for no buttons, for example over the fragment writer.
- A live region tells screen readers about the preview.
- Set \`hint\` to false to remove the help line.`
				}
			}
		}
	});
</script>

<Story name="Values now" />
<Story name="Preview" args={{ preview: { 'time.phase': 'night' } }} />
<Story name="Pinned over the writer" args={{ preview: { 'time.phase': 'night' }, onapply: undefined, onreset: undefined, hint: false }} />
<Story name="No values" args={{ switches: [] }} />
