<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn, userEvent, within } from 'storybook/test';
	import CreatePanel from './CreatePanel.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/CreatePanel',
		component: CreatePanel,
		tags: ['autodocs'],
		args: {
			kind: 'exit',
			here: 'The Shore',
			direction: 'south',
			directions: ['south', 'west', 'up', 'down', 'in', 'out'],
			rooms: [
				{ id: 'cottage', name: "Keeper's Cottage" },
				{ id: 'stairwell', name: 'Lighthouse Stairwell' }
			],
			onsubmit: fn(async () => null),
			oncancel: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `The create panel is a small form in the story text, for something new. It opens in write mode, at the end of the newest turn.

- **Exit**: the direction, and where the exit leads: a new room (with its name), an existing room, or nowhere ("nowhere, only a message": the text that a move in that direction prints, for example "Cold surf breaks over the rocks."). "with the way back" also adds the opposite exit. The panel opens when the player walks where there is no exit, with that direction. After "Create" the player walks through the new exit, and the writer opens for the text of a new room.
- **Thing**: the name, and "the player can take it". After "Create" the writer opens for the presence text of the thing.
- **Character**: the name, and "proper name, without "the"".

With \`ondraft\` (while the AI server runs), a new exit can also be **drafted with AI**: the job \`imagine_exit\` makes some proposals for that direction, and they show as ghost text and in Atlas › Proposals.

The ids come from the names, for example "The Beach" becomes \`the_beach\`. Enter creates, Escape cancels.
\`onsubmit\` returns a problem text when the story does not build with the new files; the panel then stays open and shows it.`
				}
			}
		}
	});
</script>

<Story name="New exit" />
<Story name="New thing" args={{ kind: 'thing', name: 'brass key' }} />
<Story name="New character" args={{ kind: 'character' }} />
<Story
	name="New message exit"
	play={async ({ canvasElement }) => {
		await userEvent.selectOptions(within(canvasElement).getByLabelText('Leads to'), 'message');
	}}
/>
<Story name="New exit, with AI" args={{ ondraft: fn() }} />
<Story name="Refused" args={{ onsubmit: fn(async () => 'There is already a room `the_beach`.') }} />
