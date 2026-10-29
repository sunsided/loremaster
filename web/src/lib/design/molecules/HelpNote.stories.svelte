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
					component: `A help note explains a region in a short text. Experienced users can collapse it.

1. The note shows in full at first. "Hide" collapses it into a round (?) button.
2. The (?) button shows the text again for this visit. "Always show" brings the full note back.
3. The collapsed state of each note stays across visits (browser storage, by \`id\`).

Put the note in a flex row with the heading of the region, with \`flex-wrap: wrap\`. The button then sits next to the heading, and the text takes a full line under it.
The (?) button has the label "Show help: <topic>" and \`aria-expanded\`. It is 24px, the WCAG 2.2 AA minimum for a target.`
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
