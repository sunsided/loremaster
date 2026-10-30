<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import ProseEditor from './ProseEditor.svelte';

	/** A small renderer for the stories. The app asks the runtime how markup reads now. */
	function render(markup: string): string | null {
		if (markup.startsWith('{night:')) return markup.slice(7, -1).split('|')[1]?.trim().replace('{time.phase}', 'dusk') ?? '';
		if (markup.startsWith('{first:')) return markup.slice(7, -1).trim();
		if (markup === '{time.phase}') return 'dusk';
		if (markup.startsWith('{beacon_lit ?')) return '';
		return null;
	}

	const { Story } = defineMeta({
		title: 'Molecules/ProseEditor',
		component: ProseEditor,
		tags: ['autodocs'],
		args: {
			value: 'Shingle crunches underfoot. To the north a whitewashed cottage huddles against the wind{first: , its windows dark}. East, the lighthouse rises {night: black against the stars | pale against the {time.phase} sky}. The *Marigold* is late.',
			label: 'Room text of The Shore, fragment 1, as the player reads it',
			render,
			onsubmit: fn(),
			oncancel: fn(),
			onselection: fn(),
			ontools: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `The prose view of a fragment. The author types in the game text, and the text reads as the player reads it.

- Markup shows as a **chip** with the text that it gives now, with a dotted line under it. A chip whose text is empty now shows as "◦". The title of a chip shows its source.
- A click on a chip, or the cursor in it, shows the source of the markup in place. When the cursor leaves, the chip comes back. The arrow keys move over a chip as one character.
- \`*Emphasis*\` reads in italics; the stars show only when the cursor is next to them.
- Enter calls \`onsubmit\`, Shift+Enter adds a line break, Escape calls \`oncancel\`, Ctrl+. calls \`ontools\` (the selection toolbar).
- \`onselection\` reports the selection and its place on the screen, for the selection toolbar.

The view has the font and the line height of its container, so it looks like the text around it. A line in the accent colour on the left and a faint tint show that the text is open for editing.`
				}
			}
		}
	});
</script>

<Story name="Room text" asChild>
	<div style="max-width: 62ch; font-family: var(--font-prose); font-size: var(--font-size-prose); line-height: var(--line-height-prose)">
		<p style="margin: 0 0 var(--space-3)">The ferry left you on the shingle an hour ago.</p>
		<ProseEditor
			value="Shingle crunches underfoot. To the north a whitewashed cottage huddles against the wind{'{'}first: , its windows dark{'}'}. East, the lighthouse rises {'{'}night: black against the stars | pale against the {'{'}time.phase{'}'} sky{'}'}. The *Marigold* is late."
			label="Room text of The Shore, fragment 1, as the player reads it"
			{render}
		/>
	</div>
</Story>
<Story name="Empty" asChild>
	<div style="max-width: 62ch; font-family: var(--font-prose); font-size: var(--font-size-prose)">
		<ProseEditor value="" label="New room text" {render} placeholder="Write what the player reads…" />
	</div>
</Story>
