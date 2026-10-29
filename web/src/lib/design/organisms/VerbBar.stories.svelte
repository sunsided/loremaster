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

The verb bar and the command prompt use the same text. When the player types, the runtime reads the partial command:
- The verb that the text names is pressed. "look " selects "Look at".
- The things that the text names, or could still name, are marked. "look lighth" marks "lighthouse door".
When the pointer is on a thing or an exit, or it has keyboard focus, the verb bar marks the verb that a click would use (\`hintedVerb\`): "Walk to" for an exit, the verb in the prompt for a thing, or "Look at" when the prompt has no verb.
When the player clicks a verb, the verb goes into the prompt. When the player clicks a thing, the thing goes into the prompt. A complete command runs immediately.
"Walk to" replaces the rows with the exits of the room.
A two-object verb, such as "Use", waits for a second thing. The "just use it" button runs it with one thing.

The verbs are toggle buttons (\`aria-pressed\`). The sentence line is a live region, so screen readers announce the command as it grows.
The verb grid changes its number of columns with the width of the screen.`
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
<Story name="Walk to" args={{ selected: 'go', sentence: 'Walk to _' }} />
<Story name="Empty hands" args={{ inventory: [] }} />
