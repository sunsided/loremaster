<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Die from './Die.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/Die',
		component: Die,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `One die. The phosphor theme shows ASCII art in the phosphor colour. The other themes show a drawn die.
The die is decoration: \`DiceDisplay\` gives the result as text. Change the theme in the toolbar to see each look.`
				}
			}
		}
	});
</script>

<Story name="All faces">
	<div style="display: flex; flex-wrap: wrap; gap: var(--space-4)">
		{#each [1, 2, 3, 4, 5, 6] as face (face)}<Die {face} />{/each}
	</div>
</Story>

<Story name="Number over 6"><Die face={17} /></Story>
