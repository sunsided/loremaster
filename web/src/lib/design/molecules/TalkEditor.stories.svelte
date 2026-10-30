<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { blackbeardTalk } from '../fixtures.ts';
	import TalkEditor from './TalkEditor.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/TalkEditor',
		component: TalkEditor,
		tags: ['autodocs'],
		args: { talk: blackbeardTalk, onsave: fn(async () => null), oncancel: fn() },
		parameters: {
			docs: {
				description: {
					component: `The talk editor lets the author change a talk in place. It shows in the game text, in write mode (Ctrl+E), while a talk is in progress. It takes the place of the newest line and the list of answers.

The author can change:
- The line of the other character, as markup.
- The text of each answer. Hidden answers are editable too. They have a "hidden" tag; its tooltip gives the condition that is false now.

To save, push "Save" or Ctrl+Enter. To cancel, push "Cancel" or Escape.
"Save" is off until the text changes. When the story does not build with the change, the problem shows under the answers (\`role="alert"\`), and the editor stays open.

Each field has a label. The numbers are the numbers the player sees; a hidden answer has "–".`
				}
			}
		}
	});
</script>

<Story name="Blackbeard at the Marigold node" />
<Story name="A problem after Save" args={{ onsave: fn(async () => 'unknown name `blackbeard.trsut`; did you mean `blackbeard.trust`?') }} />
