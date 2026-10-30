<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import StatusLine from './StatusLine.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/StatusLine',
		component: StatusLine,
		tags: ['autodocs'],
		args: {
			room: 'The Shore',
			day: 1,
			time: '20:30',
			phase: 'dusk',
			turn: 0,
			turnItems: [{ id: 'restart', label: 'Restart the game', run: fn() }]
		},
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The status line is the top line of the screen, in inverse video, like the Infocom games. It is the one place for the state of the game.

- Left: the current room, and its level when the story has more than one level. Short marks can follow, for example a running AI job.
- Right: the day, the time and the phase, then "Turn N ▾". The turn menu has the game actions, for example "Restart the game".

The colours are \`--color-status-bg\` and \`--color-status-text\`. In each theme they are the accent and the text on the accent.
The line is a group with the name "Game status", so screen readers can find it.`
				}
			}
		}
	});
</script>

<Story name="One level" />
<Story name="With a level" args={{ room: 'North Landing, Second Floor', level: 2, time: '23:08', phase: 'night', turn: 4 }} />
