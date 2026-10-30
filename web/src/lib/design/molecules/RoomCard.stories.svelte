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
- **Explore** goes there and opens the checks with the texts of the room in all states.`
				}
			}
		}
	});
</script>

<Story name="Bell chamber" />
