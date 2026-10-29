<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Heading from '../atoms/Heading.svelte';
	import HelpNote from './HelpNote.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/HelpNote',
		component: HelpNote,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `A help note explains a region in a short text. It is a (?) toggletip next to the heading of the region, so the text does not take space from the content.

1. Move the pointer onto the (?) button, or move the focus to it: the text shows under the button.
2. Click the button to keep the text open. A second click, Escape or a click outside closes it.
3. The pointer can move from the button onto the text, and the text stays open (WCAG 1.4.13).

The button has the label "Help: <topic>", \`aria-expanded\` and \`aria-describedby\`. Thus a screen reader reads the help text with the button, also when the text is closed. The button is 24px, the WCAG 2.2 AA minimum for a target.

Put the note in a flex row with the heading of the region.`
				}
			}
		}
	});
</script>

<Story name="Next to a heading" asChild>
	<div style="display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3) var(--space-4); max-width: 420px">
		<Heading level={2}>Assembly · room.shore</Heading>
		<HelpNote id="storybook-demo" topic="Assembly">
			The player reads the shown fragments in this order: layer by layer, then by position in the layer.
		</HelpNote>
	</div>
</Story>

<Story
	name="Open"
	asChild
	play={async ({ canvasElement }) => {
		canvasElement.querySelector('button')?.click();
	}}
>
	<div style="display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3) var(--space-4); max-width: 420px; min-height: 140px">
		<Heading level={2}>Assembly · room.shore</Heading>
		<HelpNote id="storybook-open" topic="Assembly">
			The player reads the shown fragments in this order: layer by layer, then by position in the layer.
		</HelpNote>
	</div>
</Story>
