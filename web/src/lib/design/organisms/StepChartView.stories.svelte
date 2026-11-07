<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import StepChartView from './StepChartView.svelte';
	import type { StepChart } from '#lib/engine/types.ts';

	const lighthouse: StepChart = {
		walkthrough: 'tests/win.yaml',
		name: 'Light the beacon',
		max_width: 3,
		steps: [
			{ id: 'drawer', command: 'open drawer', index: 1, at: 'cottage', needs: [], depth: 0, width: 3, hints: 2 },
			{ id: 'key', command: 'take key', index: 2, needs: ['drawer'], depth: 1, width: 3, hints: 1 },
			{ id: 'lamp', command: 'take lamp', index: 3, needs: [], depth: 0, width: 3, hints: 1 },
			{ id: 'matches', command: 'take matches', index: 4, needs: [], depth: 0, width: 2, hints: 1 },
			{ id: 'lit', command: 'light lamp', index: 5, needs: ['lamp', 'matches'], depth: 1, width: 2, hints: 1 },
			{ id: 'door', command: 'unlock iron door with key', index: 7, at: 'shore', needs: ['key'], depth: 2, width: 1, hints: 1 },
			{ id: 'beacon', command: 'light beacon', index: 10, at: 'lantern_room', needs: ['lit', 'door'], depth: 3, width: 1, hints: 2 }
		]
	};

	const chain: StepChart = {
		walkthrough: 'tests/chain.yaml',
		name: 'One after the other',
		max_width: 1,
		steps: ['a', 'b', 'c', 'd', 'e'].map((id, i, all) => ({
			id,
			command: `step ${id}`,
			index: i,
			needs: i ? [all[i - 1]] : [],
			depth: i,
			width: 1,
			hints: 0
		}))
	};

	const { Story } = defineMeta({
		title: 'Organisms/StepChartView',
		component: StepChartView,
		tags: ['autodocs'],
		args: { charts: [lighthouse] },
		parameters: {
			docs: {
				description: {
					component: `The dependency charts of the walkthroughs, in the Atlas (CK-7, US-41, US-43). They come from the step ids and \`needs\` of the walkthrough files.

- One column for each depth of needs, a box for each step (\`?\` marks a step with hints), and an arrow from each step to the steps that need it. The pointer on a step lights its needs and the steps that need it.
- The bars show the puzzle width: how many steps the player can work on when each step comes up.
- A run of at least five steps with width 1 is marked: the game is in series only there. The checker warns about it too.`
				}
			}
		}
	});
</script>

<Story name="Lighthouse" />
<Story name="In series only" args={{ charts: [chain] }} />
<Story name="No step ids" args={{ charts: [] }} />
