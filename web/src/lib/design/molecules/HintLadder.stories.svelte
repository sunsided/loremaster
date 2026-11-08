<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import HintLadder from './HintLadder.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/HintLadder',
		component: HintLadder,
		tags: ['autodocs'],
		args: {
			hints: [
				{ step: 'drawer', shown: [], levels: 3 },
				{ step: 'lamp', shown: ['It will be dark where you are going. Something on the table could show you the way.'], levels: 2 }
			],
			onhint: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `The hint ladder of the current room (UI-1, CK-6). It comes from the walkthrough files: each step with \`hints\` is a ladder. Only the steps of this room show that the player has not done and whose \`needs\` are done.

- A ladder goes one level at a time: the hints of the step, from a nudge to almost the command, and then the command.
- The ladders say "Puzzle 1", "Puzzle 2", not the ids of their steps, so a ladder gives away no more than its levels.
- The typed word \`hint\` shows the next level too.`
				}
			}
		}
	});
</script>

<Story name="Two puzzles here" />
<Story
	name="The whole ladder"
	args={{
		hints: [{ step: 'drawer', shown: ['Hal kept his few things in the cottage. Look at the furniture.', 'The dresser has a deep drawer.'], command: 'open drawer', levels: 3 }]
	}}
/>
<Story name="No hint here" args={{ hints: [] }} />
