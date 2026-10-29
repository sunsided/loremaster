<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import { fn } from 'storybook/test';
	import { endTurn, entries, openingTurn } from '../fixtures.ts';
	import Transcript from './Transcript.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/Transcript',
		component: Transcript,
		tags: ['autodocs'],
		args: {
			entries,
			room: 'shore',
			showSources: true,
			focus: null,
			effects: 'off',
			reduceMotion: true,
			onthing: fn(),
			onexit: fn(),
			onfocus: fn()
		},
		argTypes: {
			effects: { control: 'inline-radio', options: ['full', 'subtle', 'off'] }
		},
		parameters: {
			docs: {
				description: {
					component: `The transcript shows the story as the player reads it: the commands and the text that each command gives.

Each paragraph is one block from the runtime. Things and exits in the text are buttons (ProseLink). Fragment numbers show before the room text when \`showSources\` is on.

The transcript is a \`log\` region. Screen readers announce new text when it arrives, but they do not read the full history again.
Each echoed command starts with "You typed:" for screen readers.

New text can show with a line reveal and a glitch. The glitch shows not more than one time in 2 s. With \`reduceMotion\` or with effects off, new text shows immediately.

The transcript scrolls to the newest text. Put it in a container with a fixed height.`
				}
			}
		}
	});
</script>

{#snippet template(args: ComponentProps<typeof Transcript>)}
	<div class="fx-{args.effects}" style="height: 420px; display: flex; flex-direction: column">
		<Transcript {...args} />
	</div>
{/snippet}

<Story name="Opening and a command" {template} />
<Story name="Fragment highlighted" args={{ focus: 1 }} {template} />
<Story name="Without sources" args={{ showSources: false }} {template} />
<Story name="Ending" args={{ entries: [{ id: 0, turn: openingTurn }, { id: 1, turn: endTurn }] }} {template} />
