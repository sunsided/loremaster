<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import ThreadsView from './ThreadsView.svelte';

	const threads = [
		{
			id: 'lost_key',
			name: 'The lost key',
			status: 'open' as const,
			summary: 'Hal hid the key of the lamp room before he went away. The gull knows.',
			setups: 1,
			payoffs: 1,
			warnings: ['The payoff shows before any setup after: go east.'],
			beats: [
				{ key: 'a', text: 'First hint at the key.', todo: false, done: false, where: 'The Shore · fragment 2', role: 'setup' as const },
				{ key: 'b', text: 'The key fits the lamp room door.', todo: false, done: false, where: 'Lighthouse Stairwell · fragment 2', role: 'payoff' as const },
				{ key: 'c', text: 'A second hint on the cliff path.', todo: true, done: false, where: 'thread lost_key', role: null }
			]
		},
		{
			id: 'the_marigold',
			name: 'The Marigold',
			status: 'resolved' as const,
			summary: null,
			setups: 0,
			payoffs: 0,
			warnings: [],
			beats: []
		}
	];

	const { Story } = defineMeta({
		title: 'Organisms/ThreadsView',
		component: ThreadsView,
		tags: ['autodocs'],
		args: { threads, onstatus: fn(), oncreate: fn(), onbeat: fn(), onexplore: fn() },
		parameters: {
			docs: {
				description: {
					component: `Atlas › Threads: the long-range threads of the story. A thread is a promise to the reader that resolves later.

- Each thread shows its name, the number of setups and payoffs, its status (open, resolved or dropped) and its summary.
- The beats are the notes with \`thread: <id>\` all over the story: setups, mentions and payoffs, and the notes and TODOs of the thread file. The place of a beat goes to it.
- Warnings come from the checker (an open thread without payoff, a payoff without setup, a resolved thread with open TODOs) and, after a run of the state explorer, from the explorer (a payoff that never shows, or shows before any setup).
- "+ Thread" makes a new file in \`threads/\`.`
				}
			}
		}
	});
</script>

<Story name="Threads" />
<Story name="Explored" args={{ explored: true }} />
<Story name="Empty" args={{ threads: [] }} />
