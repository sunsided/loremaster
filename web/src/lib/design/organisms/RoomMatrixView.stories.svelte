<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { cantinaMatrix, cantinaSeen } from '../fixtures.ts';
	import RoomMatrixView from './RoomMatrixView.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/RoomMatrixView',
		component: RoomMatrixView,
		tags: ['autodocs'],
		args: { matrix: cantinaMatrix, roomName: 'the Cantina', seen: null, ontry: fn() },
		parameters: {
			docs: {
				description: {
					component: `The room matrix view is the "This room" mode of the "Explore" tab. It shows the text of the current room in each combination of the values that the text depends on. It also shows combinations that the player can not reach. Use it to find a text that reads badly in one condition.

The runtime finds the values in the conditions of the room fragments, in the inline markup, and in the presence texts and name stages of the things in the room. For a number, it tries the values next to each limit in a condition. It renders not more than 256 combinations.

The view shows:
1. The values that the text depends on.
2. A summary: the number of combinations, of different texts and of possible problems.
3. Each different text one time, with a table of the combinations that give it. Texts with a possible problem show first.

The runtime marks these possible problems:
- No room text.
- Two spaces in a row, a space before punctuation, or a brace in the text.
- Two replacing fragments of one layer that apply at the same time.
- Two or more presence texts of one thing that show at the same time.

After a "Whole game" run, each text has a mark: "seen in the whole-game run", or "not seen in the whole-game run". A text that the run did not see can be unreachable. It can also be outside the budget of the run.

"Try this" puts the game into the combination: it moves the player to the room and sets each value. It does not play a turn.`
				}
			}
		}
	});
</script>

<Story name="Before a whole-game run" />
<Story name="After a whole-game run" args={{ seen: cantinaSeen }} />
<Story
	name="Text that does not change"
	args={{
		matrix: { room: 'stairwell', dimensions: [], combinations: 1, cells: [{ values: [], text: 'Stone steps wind up into the dark.', warnings: [] }] },
		roomName: 'the Stairwell'
	}}
/>
