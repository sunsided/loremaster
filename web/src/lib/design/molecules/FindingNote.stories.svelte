<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { findings } from '../fixtures.ts';
	import FindingNote from './FindingNote.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/FindingNote',
		component: FindingNote,
		tags: ['autodocs'],
		args: { finding: findings[0], ondismiss: fn() },
		parameters: {
			docs: {
				description: {
					component: `One finding of an AI review ("✦ Review this room"), as a note in the margin of write mode, next to its fragment.

- The severity is a word (HIGH, MEDIUM, LOW), not only a colour. A high finding has a solid line in the warning colour; the others have a dotted line in the ghost colour, and a low one has a dashed tag.
- The fix is in italics under the problem.
- × dismisses the finding: it does not show again for this review.
- A finding about the whole room, or about a fragment that does not show now, goes under the room text.

The review job replies with one line for each finding, in the form \`- high | room:shore #2 | the problem | the fix\`. The editor reads these lines.`
				}
			}
		}
	});
</script>

<Story name="High" />
<Story name="Medium" args={{ finding: findings[1] }} />
<Story name="Low" args={{ finding: findings[2] }} />
<Story name="Without dismiss" args={{ finding: findings[1], ondismiss: undefined }} />
