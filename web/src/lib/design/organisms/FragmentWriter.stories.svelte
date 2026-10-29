<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { shoreFragment, waveFragment, writerTools } from '../fixtures.ts';
	import FragmentWriter from './FragmentWriter.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/FragmentWriter',
		component: FragmentWriter,
		tags: ['autodocs'],
		args: {
			fragment: shoreFragment,
			title: 'Room text of The Shore, fragment 1',
			tools: writerTools,
			onsave: fn(async () => null),
			oncancel: fn(),
			onremove: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `The fragment writer edits one fragment in place, in the story text. It opens in write mode, when the author selects a fragment in the transcript or "Edit" in the assembly panel.

The writer has these parts:
1. The title: what the author edits, for example "Room text of The Shore, fragment 1".
2. The layer, the mode ("Shows") and "replaces earlier text in its layer".
3. The condition ("When"). The tag next to it tells whether the condition holds now. Empty means always.
4. The variants, for cycling and random fragments. "Add a variant" changes the mode to "cycle variants" when the mode had only one text.
5. The text, in the markup editor.
6. "Now it reads": the text as the player reads it in the current state. A problem in the text shows here instead, with a "Use …" quick fix when the editor finds the name that the author meant.
7. "Other values" (closed at first): how the text reads in each combination of the values that it depends on, one line for each different text, with the values that give it.
8. The actions: Save (or Add for a new fragment), Cancel, Add a variant, Remove this fragment.

Enter saves, Shift+Enter adds a line break, Escape cancels. Save is not available while the text or the condition has a problem.
\`onsave\` returns a problem text when the story does not build with the change. The writer then stays open and shows it.

The writer uses the interface font for its controls and the prose font for the text, so the text looks as it does in the game.`
				}
			}
		}
	});
</script>

<Story name="Room text" />
<Story name="Variants and a condition" args={{ fragment: waveFragment, variant: 1, title: 'Room text of The Shore, fragment 3' }} />
<Story
	name="New fragment"
	args={{ fragment: { ...shoreFragment, variants: [''] }, fresh: true, title: 'Room text of The Shore: new fragment' }}
/>
<Story name="Save refused" args={{ onsave: fn(async () => 'There is no thing `lamp`.') }} />
