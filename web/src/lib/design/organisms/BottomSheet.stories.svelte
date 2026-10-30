<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import BottomSheet from './BottomSheet.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/BottomSheet',
		component: BottomSheet,
		tags: ['autodocs'],
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `On a phone (below 900px) the side panel is a bottom sheet: "Here" in play mode, "Desk" in write mode.

- It **peeks** with one line: the title and a summary, for example "2 exits · dusk_falls fires in 30 minutes".
- A tap on the grab bar moves to the next height: peek, half, full, then peek again. A drag up or down opens or closes it.
- The grab bar is a button with \`aria-expanded\`, so keyboard and switch users can open and close the sheet. The section has a heading.
- The content scrolls inside the sheet. The page keeps room at the bottom for the line of the sheet.
- With reduced motion, the sheet changes height without a slide.`
				}
			},
			story: { inline: false, iframeHeight: 520 }
		}
	});
</script>

<Story name="Peek" asChild>
	<div style="padding: 16px; font-family: var(--font-prose)">The game text is here.</div>
	<BottomSheet title="Here" summary="2 exits · dusk_falls fires in 30 minutes">
		<p style="margin: 0; padding: 16px">The sections of the Here panel.</p>
	</BottomSheet>
</Story>
<Story name="Half open" asChild>
	<div style="padding: 16px; font-family: var(--font-prose)">The game text is here.</div>
	<BottomSheet title="Here" summary="2 exits · dusk_falls fires in 30 minutes" height="half">
		<p style="margin: 0; padding: 16px">The sections of the Here panel.</p>
	</BottomSheet>
</Story>
