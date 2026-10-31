<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import MachinesView from './MachinesView.svelte';

	const exorcism = {
		id: 'exorcism',
		state: 'rung',
		start: 'idle',
		states: ['idle', 'rung', 'lit', 'done'],
		descriptions: { idle: 'The spirits block the gate and jeer.', rung: 'The bell is red hot; the spirits are stunned.' },
		steps: [
			{ rule: 'exorcism#1', from: ['idle'], to: 'rung', when: 'the player uses "ring bell"', condition: null, fired: true, ready: false },
			{ rule: 'exorcism#2', from: ['rung'], to: 'lit', when: 'the player uses "turn on candles"', condition: 'carried(candles)', fired: false, ready: true },
			{ rule: 'exorcism#3', from: ['lit'], to: 'done', when: 'the player uses "read book"', condition: null, fired: false, ready: false },
			{ rule: 'exorcism#4', from: ['rung', 'lit'], to: 'idle', when: 'the timer exorcism_window runs out', condition: null, fired: false, ready: true }
		]
	};

	const { Story } = defineMeta({
		title: 'Organisms/MachinesView',
		component: MachinesView,
		tags: ['autodocs'],
		args: { machines: [exorcism], onrule: fn(), onselect: fn(), selected: { machine: 'exorcism', state: 'rung' }, onaddstate: fn(async () => null) },
		parameters: {
			docs: {
				description: {
					component:
						'The puzzle machines of the story, in the Atlas. States are in a row. Steps forward arc above, steps back arc below. The current state is filled. A step that the next matching event takes now is marked.'
				}
			}
		}
	});
</script>

<Story name="Exorcism" />
<Story name="Empty" args={{ machines: [] }} />
