<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { StatView } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';
	import StatSheet from './StatSheet.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/StatSheet',
		component: StatSheet,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `The adventure sheet of the player, from \`stats:\` in story.yaml. It shows numbers as "current / initial", the equipment and the codewords.
A changed value has a short mark, and a live region says the change. Below 900px the sheet is one line, and a tap opens it. Select a small viewport to see this.`
				}
			}
		}
	});

	const full: StatView[] = [
		{ var: 'skill', label: 'SKILL', short: 'SK', value: 10, max: 10 },
		{ var: 'stamina', label: 'STAMINA', short: 'ST', value: 14, max: 19 },
		{ var: 'luck', label: 'LUCK', short: 'LK', value: 8, max: 9 },
		{ var: 'gold', label: 'Gold', value: 5 },
		{ var: 'provisions', label: 'Provisions', value: 3 },
		{ var: 'hearth', label: 'HEARTH', value: true },
		{ var: 'ember', label: 'EMBER', value: false }
	];
	const items = [
		{ id: 'sword', name: 'sword', takeable: true, location: 'carried' as const },
		{ id: 'lantern', name: 'brass lantern', takeable: true, location: 'carried' as const }
	];

	let live = $state(full);
	function wound() {
		live = live.map((s) => (s.var === 'stamina' ? { ...s, value: (s.value as number) - 2 } : s));
	}
</script>

<Story name="Full sheet"><StatSheet stats={full} inventory={items} /></Story>
<Story name="No equipment or codewords"><StatSheet stats={full.slice(0, 3)} /></Story>
<Story name="A value changes">
	<Button size="compact" onclick={wound}>Lose 2 STAMINA</Button>
	<StatSheet stats={live} inventory={items} />
</Story>
