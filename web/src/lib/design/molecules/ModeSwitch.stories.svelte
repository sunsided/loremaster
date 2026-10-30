<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import ModeSwitch from './ModeSwitch.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/ModeSwitch',
		component: ModeSwitch,
		tags: ['autodocs'],
		args: { value: 'play', onchange: fn() },
		parameters: {
			docs: {
				description: {
					component: `The two modes of the editor as one switch: **Play** shows the game as the player sees it, **Write** opens the same screen for editing.

It is a radio group with the name "Mode". Tab moves to the checked mode, and the arrow keys switch. Ctrl+E (Cmd+E on a Mac) switches from anywhere in the page; the page handles that key.
The checked mode is in inverse video, like the status line.`
				}
			}
		}
	});
</script>

<Story name="Play" />
<Story name="Write" args={{ value: 'write' }} />
