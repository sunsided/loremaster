<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { Awaiting, AwaitRequest } from '#lib/engine/types.ts';
	import AwaitPrompt from './AwaitPrompt.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/AwaitPrompt',
		component: AwaitPrompt,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `The story waits for an answer: a rule paused at an \`await\` effect, or the death prompt after a death ending.
The prompt names what the story asks and offers the answers as buttons. The answer goes back to the session with \`resume\`.`
				}
			}
		}
	});

	const at = (request: AwaitRequest): Awaiting => ({ id: 'await@3', rule: 'ferry', next: 1, request });
	const answer = (value: unknown) => console.log('answer', value);
</script>

<Story name="Choice">
	<AwaitPrompt
		awaiting={at({
			kind: 'choice',
			options: [
				{ id: 'pay', text: 'Pay the ferryman' },
				{ id: 'swim', text: 'Swim across' }
			]
		})}
		onanswer={answer}
	/>
</Story>

<Story name="Roll">
	<AwaitPrompt awaiting={at({ kind: 'roll', dice: '2d6', min: 2, max: 12 })} onanswer={answer} />
</Story>

<Story name="Roll for an enemy">
	<AwaitPrompt awaiting={at({ kind: 'roll', dice: '2d6', min: 2, max: 12, who: 'GATE WARDEN' })} onanswer={answer} />
</Story>

<Story name="Your roll">
	<AwaitPrompt awaiting={at({ kind: 'roll', dice: '2d6', min: 2, max: 12, who: 'You' })} onanswer={answer} />
</Story>

<Story name="Roll against Luck">
	<AwaitPrompt awaiting={at({ kind: 'roll', dice: '2d6', min: 2, max: 12, who: 'LUCK', against: 'luck' })} onanswer={answer} />
</Story>

<Story name="Action">
	<AwaitPrompt awaiting={at({ kind: 'action', name: 'pick_lock', results: ['opened', 'broke'] })} onanswer={answer} />
</Story>

<Story name="Walk to">
	<AwaitPrompt awaiting={at({ kind: 'walk_to', room: 'pier' })} onanswer={answer} />
</Story>

<Story name="Death prompt">
	<AwaitPrompt awaiting={at({ kind: 'death_prompt', ending: 'drowned', options: ['undo', 'restart', 'restore'] })} onanswer={answer} />
</Story>
