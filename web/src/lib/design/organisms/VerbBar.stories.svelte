<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { exits, inventory, things, verbs } from '../fixtures.ts';
	import VerbBar from './VerbBar.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/VerbBar',
		component: VerbBar,
		tags: ['autodocs'],
		args: {
			verbs,
			selected: null,
			sentence: '',
			canRunPartial: false,
			things,
			inventory,
			exits,
			onverb: fn(),
			onthing: fn(),
			onexit: fn(),
			onrun: fn(),
			oncancel: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `The verb bar is the SCUMM-style way to play. It uses the same commands as the typed parser.

To use it:
1. Select a verb, for example "Open".
2. Click a thing in the story text or in the "here" or "carrying" row.

A click on a thing with no verb looks at the thing.
A verb without things, such as "Look around", runs at once.

The look is quiet: the verbs are text in a grid of four columns (three on narrow screens, one scrolling row on a phone), and the things in reach are on the right (under the verbs on narrow screens). The selected verb is in inverse video, like the status line; the hinted verb is underlined in the accent colour. Each verb is a toggle button with \`aria-pressed\`.
The help line shows only while \`hint\` is true. The app sets it until the player gives the first command.

The verb bar and the command prompt use the same text. When the player types, the runtime reads the partial command:
- The verb that the text names is pressed. "look " selects "Look at".
- The things that the text names, or could still name, are marked. "look lighth" marks "lighthouse door".
When the pointer is on a thing or an exit, or it has keyboard focus, the verb bar marks the verb that a click would use (\`hintedVerb\`): "Walk to" for an exit, the verb in the prompt for a thing, or "Look at" when the prompt has no verb.
When the player clicks a verb, the verb goes into the prompt. When the player clicks a thing, the thing goes into the prompt. A complete command runs immediately.
"Walk to" replaces the rows with the exits of the room.
A two-object verb, such as "Use", waits for a second thing. The "just use it" button runs it with one thing.

The verbs are toggle buttons (\`aria-pressed\`). The sentence line is a live region, so screen readers announce the command as it grows.
The verb grid changes its number of columns with the width of the screen.

During a talk (\`talkingTo\` is set), the answers that the player can choose show above the verbs, as a numbered list of buttons, with "Leave" after them. This is the only place of the answers: the transcript shows only what is said. When a talk starts, the focus goes to the first answer. After a choice, the focus goes to the new first answer when it was on an old one. The verbs and the things dim and are inert: they do not take focus, and screen readers skip them. Only an answer or "Leave" ends the turn. The number keys and Escape do the same as the buttons; \`aria-keyshortcuts\` tells assistive technology.`
				}
			}
		}
	});
</script>

<Story name="No verb selected" />
<Story name="Verb selected" args={{ selected: 'open', sentence: 'Open _' }} />
<Story name="Two-object verb" args={{ selected: 'use', sentence: 'Use oil lamp on _', canRunPartial: true }} />
<Story name="Typed: look lighthouse" args={{ selected: 'examine', sentence: 'Look at lighthouse door', marked: ['cottage_door'] }} />
<Story name="Pointer on a thing" args={{ hintedVerb: 'examine' }} />
<Story
	name="In a talk"
	args={{
		talkingTo: 'the grim-looking man',
		choices: [
			{ index: 0, text: '"Who are you?"' },
			{ index: 1, text: '"I am looking for the *Marigold*."' },
			{ index: 2, text: '"Not tonight."' }
		],
		onchoose: fn(),
		onleave: fn()
	}}
/>
<Story name="Walk to" args={{ selected: 'go', sentence: 'Walk to _' }} />
<Story name="Empty hands" args={{ inventory: [] }} />
