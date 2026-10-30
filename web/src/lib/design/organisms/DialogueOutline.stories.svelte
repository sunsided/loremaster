<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { blackbeardOutline, brokenOutline } from '../fixtures.ts';
	import DialogueOutline from './DialogueOutline.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/DialogueOutline',
		component: DialogueOutline,
		tags: ['autodocs'],
		args: { outline: blackbeardOutline, name: 'Blackbeard', file: 'dialogues/blackbeard.yaml', current: null },
		parameters: {
			docs: {
				description: {
					component: `The dialogue outline shows a dialogue as a list. It is in the "Dialogue" section of the room desk, one outline for each thing in the room that has a dialogue.

Each node shows:
- Its key, a "start" tag with its place in the \`start\` list, and "now" when the talk in progress is at this node.
- Its condition (\`when\`), when it has one. A start node with a condition starts the talk only when the condition holds.
- The line of the other character, as written (with markup).
- What entering the node changes, for example "trust +1".
- The answers of the player, numbered as in the game. Each answer shows its condition, where it goes, and what it changes.

A link to a node ("→ names") moves the keyboard focus to that node. The node heading takes focus for this (\`tabindex="-1"\`).

The outline shows two problems as warnings:
1. An answer goes to a node that does not exist.
2. No start node leads to a node.

\`loremaster check\` reports the same problems, through the dramaturge graph validator.

The outline uses lists and headings, so screen readers can move from node to node. Its width follows the side panel; long text wraps.`
				}
			}
		}
	});
</script>

<Story name="Cantina: Blackbeard" />
<Story name="In a talk" args={{ current: 'marigold' }} />
<Story name="With problems" args={{ outline: brokenOutline }} />
<Story name="No dialogue" args={{ outline: null, name: 'the barkeeper' }} />
