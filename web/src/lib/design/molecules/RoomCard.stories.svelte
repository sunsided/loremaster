<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { towerRooms } from '../fixtures.ts';
	import RoomCard from './RoomCard.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/RoomCard',
		component: RoomCard,
		tags: ['autodocs'],
		args: { room: towerRooms.at(-1)!, ongo: fn(), onwrite: fn(), onexplore: fn() },
		parameters: {
			docs: {
				description: {
					component: `One room in the Atlas, next to the map: its level, whether the player saw it, its exits (an exit that only prints a message says so), the things in it now, its fragments and the rules that name it.

- **Go there** moves the player to the room (an author move) and returns to the game. It is off for the room of the player.
- **Write here** goes there and starts write mode.
- **Explore** goes there and opens the checks with the texts of the room in all states.

A room that an AI proposal adds has ✦ and the summary of the proposal, in the ghost colour. In place of Go there and Write here, it offers **Walk in** (play the proposal in a sandbox, from this room), **Accept** and **Reject**. A room that a proposal removes says so in the warning colour.`
				}
			}
		}
	});
</script>

<Story name="Bell chamber" />
<Story
	name="Proposed by AI"
	args={{
		room: { id: 'sea_rocks', name: 'The Sea Rocks', level: 0, visited: false, current: false, things: [], exits: [['east', 'shore'], ['south', '']], fragments: 1, proposed: 'p2' },
		proposal: { id: 'p2', summary: 'Rocks west of the shore' },
		onwalk: fn(),
		onaccept: fn(),
		onreject: fn()
	}}
/>
<Story name="Removed by AI" args={{ removedBy: 'Fold the bell chamber into the stair' }} />
