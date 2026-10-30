<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import { fn } from 'storybook/test';
	import { authorTurn, talkTurns, doorTurn, drawerTurn, endTurn, entries, findings, exits, openingTurn, things } from '../fixtures.ts';
	import Transcript from './Transcript.svelte';
	import FragmentWriter from './FragmentWriter.svelte';
	import { shoreFragment, writerTools } from '../fixtures.ts';

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

In a **talk**, the transcript shows only what is said: the answer that the player chose (with a line on the left, and "You say:" for screen readers), then the line of the other character. The answers to choose from are in the verb bar. The log region announces the new line. For a layout without the verb bar, set \`choicesInText\`: then the numbered answers show under the newest line.

When \`showSources\` is on (in the app: in write mode), text from an action frame has a small "frame" mark before it. The mark has a title that names the frame, for example "story frame for look, before".

Two kinds of text are not story prose, and they look different:
- A **parser note**, such as "(first opening the cottage door)", tells the player about a step that the game did by itself. It is small, italic and muted, in the interface font.
- An **author action**, such as "go to Lantern Room" from the command palette, is not part of the game. It has an "author" badge and the warning colour.

New text can show with a line reveal and a glitch. The glitch shows not more than one time in 2 s. With \`reduceMotion\` or with effects off, new text shows immediately.

The transcript scrolls to the newest text. Put it in a container with a fixed height.

**Write mode.** When \`writable\` is the id of an entry, each fragment of its room text is a button that opens the writer (\`onwrite\` with the source of the fragment). Links in that text are plain text: a click edits, it does not play. "+ Add room text" (\`onadd\`) adds a fragment. While the author edits a fragment (\`writing\`), its text is struck through and the \`writer\` snippet shows after its paragraph. A turn whose room text an edit wrote again has the note "Written again after an edit of the story."

**The margin.** With \`margin\` (in the app: in write mode), each fragment of the editable room text is on its own line, with a note in the margin: its layer as a tag, then its mode and condition. Presence texts and frame texts have a tag too; generated lines ("There is …", the exits) have a dashed tag. The note is the description of the fragment button (\`aria-describedby\`), so a screen reader reads it with the text. "+ N hidden here" shows the fragments that do not show now, struck through, with the reason in the margin; a click opens one for editing. Below 760px of width the notes go under their lines.

**Preview.** With \`preview\`, the newest room text shows the preview blocks in its place, with a line in the warning colour on the left. Screen readers hear "Preview of the room text in other values. The game did not move."`
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
<Story name="Write mode" args={{ entries: [{ id: 0, turn: openingTurn }], writable: 0, onwrite: fn(), onadd: fn() }} {template} />
<Story name="Write mode, editing" args={{ entries: [{ id: 0, turn: openingTurn, rewritten: true }], writable: 0, writing: { owner: 'room:shore', fragment: 0 } }}>
	{#snippet template(args)}
		<div style="height: 620px; display: flex; flex-direction: column">
			<Transcript {...args}>
				{#snippet writer()}
					<FragmentWriter fragment={shoreFragment} title="Room text of The Shore, fragment 1" tools={writerTools} onsave={async () => null} oncancel={() => {}} />
				{/snippet}
			</Transcript>
		</div>
	{/snippet}
</Story>
<Story
	name="Write mode with the margin"
	args={{
		entries: [{ id: 0, turn: openingTurn }],
		writable: 0,
		onwrite: fn(),
		onadd: fn(),
		margin: {
			owner: 'room:shore',
			things: 2,
			exits: 2,
			fragments: [
				{ index: 0, layer: 'base', detail: 'always', shown: true, why: null, text: '' },
				{ index: 1, layer: 'ambient', detail: 'always · when time.phase == dusk', shown: true, why: null, text: '' },
				{ index: 2, layer: 'ambient', detail: 'shuffle · when time.phase == night', shown: false, why: 'time.phase == night is false', text: 'Waves hiss on the stones.' }
			]
		}
	}}
	{template}
/>
<Story
	name="Write mode with review findings"
	args={{
		entries: [{ id: 0, turn: openingTurn }],
		writable: 0,
		onwrite: fn(),
		onadd: fn(),
		findings,
		ondismiss: fn(),
		margin: {
			owner: 'room:shore',
			things: 2,
			exits: 2,
			fragments: [
				{ index: 0, layer: 'base', detail: 'always', shown: true, why: null, text: '' },
				{ index: 1, layer: 'ambient', detail: 'always · when time.phase == dusk', shown: true, why: null, text: '' },
				{ index: 2, layer: 'ambient', detail: 'shuffle · when time.phase == night', shown: false, why: 'time.phase == night is false', text: 'Waves hiss on the stones.' }
			]
		}
	}}
	{template}
/>
<Story name="In a talk" args={{ entries: talkTurns.map((turn, id) => ({ id, turn })), room: 'cantina' }} {template} />
<Story name="In a talk, answers in the text" args={{ entries: talkTurns.map((turn, id) => ({ id, turn })), room: 'cantina', choicesInText: true }} {template} />
<Story name="Ending" args={{ entries: [{ id: 0, turn: openingTurn }, { id: 1, turn: endTurn }] }} {template} />
