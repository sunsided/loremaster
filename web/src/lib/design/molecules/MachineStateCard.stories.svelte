<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import MachineStateCard from './MachineStateCard.svelte';

	const machine = {
		id: 'exorcism',
		state: 'rung',
		states: ['idle', 'rung', 'lit', 'done'],
		descriptions: { rung: 'The bell is red hot and the spirits are stunned. The timer exorcism_window runs.' },
		steps: [
			{ rule: 'exorcism#1', from: ['idle'], to: 'rung', when: 'the player uses "ring bell"', condition: null, fired: true, ready: false },
			{ rule: 'exorcism#2', from: ['rung'], to: 'lit', when: 'the player uses "turn on candles"', condition: null, fired: false, ready: true },
			{ rule: 'exorcism#4', from: ['rung', 'lit'], to: 'idle', when: 'the timer exorcism_window runs out', condition: null, fired: false, ready: true }
		]
	};

	const { Story } = defineMeta({
		title: 'Molecules/MachineStateCard',
		component: MachineStateCard,
		tags: ['autodocs'],
		args: { machine, state: 'rung', onrule: fn() },
		parameters: {
			docs: {
				description: {
					component: 'One state of a puzzle machine, in the details panel of Atlas › Machines: its description and the steps into it and out of it.'
				}
			}
		}
	});
</script>

<Story name="With description" />
<Story name="Without description" args={{ state: 'lit' }} />
