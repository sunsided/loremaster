<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { blackbeardTalk } from '../fixtures.ts';
	import TalkInspector from './TalkInspector.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/TalkInspector',
		component: TalkInspector,
		tags: ['autodocs'],
		args: { talk: blackbeardTalk },
		parameters: {
			docs: {
				description: {
					component: `The talk inspector shows the talk in progress in the playtest inspector (the side panel in play mode). It shows only during a talk.

It shows:
- The current node and the file of the dialogue.
- The condition of the node, and what entering it changed.
- All answers of the node, also the hidden ones. A visible answer has its number, as the player sees it. A hidden answer is struck out and shows the condition that is false now.
- The relations of the player to the character (for example trust) and the memories that the character keeps.

The room matrix and the state explorer do not show relations and memories. This is the place to see them.`
				}
			}
		}
	});
</script>

<Story name="Blackbeard, wary" />
<Story
	name="Blackbeard, trusting"
	args={{
		talk: {
			...blackbeardTalk,
			options: blackbeardTalk.options.map((o, i) => ({ ...o, choice: i === 1 ? null : i === 0 ? 0 : 1 })),
			relations: [['trust', 1]]
		}
	}}
/>
