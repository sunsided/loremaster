<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import NotesList from './NotesList.svelte';

	const notes = [
		{ key: 'a', text: "Hal's boat must feel abandoned, not wrecked.", todo: false, done: false },
		{ key: 'b', text: 'Decide if the ferryman comes back at dawn.', todo: true, done: false },
		{ key: 'c', text: 'Name the gull.', todo: true, done: true },
		{ key: 'd', text: 'First hint at the key.', todo: false, done: false, where: 'fragment 2', thread: 'lost_key', role: 'setup' as const }
	];

	const { Story } = defineMeta({
		title: 'Organisms/NotesList',
		component: NotesList,
		tags: ['autodocs'],
		args: { notes, ontoggle: fn(), onremove: fn(), onadd: fn() },
		parameters: {
			docs: {
				description: {
					component: `Author notes and TODOs. The player never sees them. The notes are in the YAML files, next to the text that they are about.

- A note shows with a pen mark (✎).
- A TODO has a checkbox. A done TODO is struck through.
- A note on a thread shows the role of the beat (setup, payoff or mention) and the thread.
- The form at the end adds a note or a TODO. Enter adds a note.
- With \`onselect\`, the place of each note is a link, for example to its room. The Atlas list of all TODOs uses this.`
				}
			}
		}
	});
</script>

<Story name="Room notes" />
<Story name="Empty" args={{ notes: [] }} />
<Story
	name="All TODOs"
	args={{
		notes: [
			{ key: 'b', text: 'Decide if the ferryman comes back at dawn.', todo: true, done: false, where: 'The Shore' },
			{ key: 'e', text: 'A second hint on the cliff path.', todo: true, done: false, where: 'thread: The lost key' }
		],
		onadd: undefined,
		onremove: undefined,
		onselect: fn(),
		empty: 'No open TODOs.'
	}}
/>
