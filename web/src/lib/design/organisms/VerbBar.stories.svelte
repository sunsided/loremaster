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
<Story name="Walk to" args={{ selected: 'go', sentence: 'Walk to _' }} />
<Story name="Empty hands" args={{ inventory: [] }} />
