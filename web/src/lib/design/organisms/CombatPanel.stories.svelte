<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { CombatView } from '#lib/engine/types.ts';
	import CombatPanel from './CombatPanel.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/CombatPanel',
		component: CombatPanel,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `The combat in progress: each enemy with SKILL and STAMINA, the attack strengths of the last round, and a log of the last rounds.
The round choices are in the await prompt, under the panel.`
				}
			}
		}
	});

	const round: CombatView = {
		round: 2,
		enemies: [{ id: 'warden', name: 'GATE WARDEN', skill: 7, stamina: 4, strength: 13 }],
		player_strength: 16,
		log: ['Round 1: you 15, GATE WARDEN 12. You hit GATE WARDEN.', 'Round 2: you 16, GATE WARDEN 13. You hit GATE WARDEN.']
	};
	const start: CombatView = { round: 0, enemies: [{ id: 'warden', name: 'GATE WARDEN', skill: 7, stamina: 8 }], log: [] };
	const two: CombatView = {
		round: 3,
		enemies: [
			{ id: 'hound', name: 'HOUND', skill: 5, stamina: 0 },
			{ id: 'warden', name: 'GATE WARDEN', skill: 7, stamina: 6, strength: 14 }
		],
		player_strength: 12,
		log: ['Round 3: you 12, GATE WARDEN 14. GATE WARDEN hits you.']
	};
</script>

<Story name="In a round"><CombatPanel combat={round} /></Story>
<Story name="Before the first round"><CombatPanel combat={start} /></Story>
<Story name="One enemy beaten"><CombatPanel combat={two} /></Story>
