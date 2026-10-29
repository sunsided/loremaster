<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { cantinaMatrix, exploration } from '../fixtures.ts';
	import ExplorerView from './ExplorerView.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/ExplorerView',
		component: ExplorerView,
		tags: ['autodocs'],
		args: { mode: 'game', matrix: cantinaMatrix, roomName: 'the Cantina', ontry: fn(), result: exploration, running: false, onrun: fn(), onreplay: fn() },
		parameters: {
			docs: {
				description: {
					component: `The explorer view is the "Explore" tab of the inspector. It has two modes:

- **This room** (the default) shows the room matrix: the text of the current room in each combination of the values that the text depends on. The "Organisms/RoomMatrixView" page tells more.
- **Whole game** runs the state explorer and shows what it found.

The state explorer plays all ways through the story from the start, within a budget (2000 states, 40 commands). It tries the actions that move the story forward: exits, verbs that rules listen to, take, open, turn on, unlock with a key in reach, and wait when the story reacts to time.

The view shows:
1. A summary: the number of states, rooms not reached, endings and stuck states. A warning shows when the budget stopped the walk.
2. **Endings**, each with the shortest command list to it.
3. **Stuck states**: states from which no ending can be reached, with the shortest way into one.
4. **Rooms**: each way the text of a room reads, how many states show it, the facts of the first such state, and the shortest path to it.
5. **Never shown** fragments and **never fired** rules.

"Replay this path" starts the game again and plays the path, so the author can continue from that state.
The terminal command \`lantern explore <folder>\` prints the same report.`
				}
			}
		}
	});
</script>

<Story name="This room" args={{ mode: 'room', result: null }} />
<Story name="After a run" />
<Story name="Before the first run" args={{ result: null }} />
<Story name="Running" args={{ result: null, running: true }} />
