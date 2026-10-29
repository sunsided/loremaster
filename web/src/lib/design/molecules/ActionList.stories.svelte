<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import ActionList from './ActionList.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/ActionList',
		component: ActionList,
		tags: ['autodocs'],
		args: {
			label: 'here',
			empty: 'nothing in reach',
			items: [
				{ id: 'drawer', label: 'drawer' },
				{ id: 'oil_lamp', label: 'oil lamp' },
				{ id: 'matches', label: 'box of matches' }
			],
			onpick: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `An action list is a labelled row of compact buttons. Each button is one thing or one exit.

The row is a group. Its label, for example "here", names the group for screen readers.
When the list has no items, it shows the \`empty\` text. Write the empty text as a fact, for example "nothing in reach".
The row wraps on narrow screens.

\`marked\` marks items, for example the things that the typed command names. Screen readers hear ", in the command" after a marked item.`
				}
			}
		}
	});
</script>

<Story name="Things" />
<Story name="With detail" args={{ label: 'to', items: [{ id: 'north', label: "Keeper's Cottage", detail: 'north' }, { id: 'east', label: 'Lighthouse Stairwell', detail: 'east' }] }} />
<Story name="With a marked item" args={{ marked: ['oil_lamp'] }} />
<Story name="Empty" args={{ label: 'carrying', items: [], empty: 'nothing' }} />
