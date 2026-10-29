<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { history } from '../fixtures.ts';
	import StoryTimeline from './StoryTimeline.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/StoryTimeline',
		component: StoryTimeline,
		tags: ['autodocs'],
		args: { steps: history },
		parameters: {
			docs: {
				description: {
					component: `The story timeline shows the playthrough of the current session. Each step is one turn.

Each step shows:
1. The time and the phase of the day.
2. The command and the room.
3. The rules that fired.
4. The state changes, for example \`drawer.open = true\`.

A step with rules or changes has a filled marker on the line. Use the timeline to find out why the story went one way.
The timeline is an ordered list, oldest first. Scenes come later; then the timeline also shows scenes as bars.`
				}
			}
		}
	});
</script>

<Story name="Three turns" />
<Story name="Empty" args={{ steps: [] }} />
