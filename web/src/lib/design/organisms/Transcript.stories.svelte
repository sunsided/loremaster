<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import { fn } from 'storybook/test';
	import { authorTurn, doorTurn, drawerTurn, endTurn, entries, exits, openingTurn, things } from '../fixtures.ts';
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

The **room brief** is one line under the newest turn: the room, the things here and the exits, all clickable. It shows only when the newest turn did not print the room, for example after "pick up iron key". Thus the text never ends without something to act on. The brief is not part of the log, so screen readers do not read it again after each turn, and it does not stay in the history.

Screen-reader labels, such as "You typed:", are not part of copied text.

Two kinds of text are not story prose, and they look different:
- A **parser note**, such as "(first opening the cottage door)", tells the player about a step that the game did by itself. It is small, italic and muted, in the interface font.
- An **author action**, such as "go to Lantern Room" from the command palette, is not part of the game. It has an "author" badge and the warning colour.

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
<Story name="Parser note and author action" args={{ entries: [{ id: 0, turn: openingTurn }, { id: 1, turn: doorTurn }, { id: 2, turn: authorTurn }] }} {template} />
<Story
	name="Room brief after a turn without the room"
	args={{ entries: [{ id: 0, turn: openingTurn }, { id: 1, turn: drawerTurn }], room: 'cottage', brief: { room: "Keeper's Cottage", things, exits: [exits[0]] } }}
	{template}
/>
<Story name="Ending" args={{ entries: [{ id: 0, turn: openingTurn }, { id: 1, turn: endTurn }] }} {template} />
