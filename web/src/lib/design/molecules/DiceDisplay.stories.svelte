<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { RollDetail } from '#lib/engine/types.ts';
	import DiceDisplay from './DiceDisplay.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/DiceDisplay',
		component: DiceDisplay,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `The dice of the newest turn. The text always gives the result and the target, so the dice are not the only source.
The phosphor theme rolls the ASCII dice for a short time. With reduced motion, the result shows at once.`
				}
			}
		}
	});

	const luck: RollDetail[] = [{ dice: '2d6', total: 7, faces: [3, 4], who: 'LUCK', target: 9, result: 'lucky' }];
	const round: RollDetail[] = [
		{ dice: '2d6', total: 9, faces: [4, 5], who: 'GATE WARDEN' },
		{ dice: '2d6', total: 6, faces: [1, 5], who: 'You' }
	];
</script>

<Story name="Test with a target"><DiceDisplay rolls={luck} reduceMotion={false} /></Story>
<Story name="A combat round"><DiceDisplay rolls={round} reduceMotion={false} /></Story>
<Story name="Reduced motion"><DiceDisplay rolls={luck} reduceMotion /></Story>
<Story name="Total only"><DiceDisplay rolls={[{ dice: '2d6', total: 8 }]} /></Story>
<Story name="No dice"><DiceDisplay rolls={[]} /></Story>
