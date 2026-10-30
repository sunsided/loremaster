<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import SelectionTools from './SelectionTools.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/SelectionTools',
		component: SelectionTools,
		tags: ['autodocs'],
		args: {
			rect: { left: 40, top: 80, bottom: 100 },
			actions: [
				{ id: 'thing', label: 'Make it a thing', run: fn() },
				{ id: 'first', label: 'Only the first time', run: fn() },
				{ id: 'when', label: 'Only when…', run: fn() },
				{ id: 'variants', label: 'Make variants', run: fn() }
			],
			onclose: fn(),
			ondismiss: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `A small toolbar over selected text in the writer, for "type first, tune later". It floats above the selection (under it near the top of the window).

The writer gives these actions, when they apply:
- **Use the name**: the selection is the name of a thing as it reads now, for example "the oil lamp". It becomes \`{oil_lamp.the}\`, so the name follows the thing.
- **Make it a thing**: the selected words become a new thing in the room. The text stays as it is; the thing is not listed again.
- **Only the first time**: \`{first: …}\` around the selection.
- **Only when…**: \`{ ? …}\` around the selection, with the cursor where the condition goes.
- **Make variants**: alternatives in the selection, split at " / " or at line breaks, become the variants of the fragment.

A mousedown on the toolbar does not take the focus, so the selection stays. With the keyboard, Ctrl+. in the editor moves the focus to the first button (\`focus\` changes), the arrow keys move between the buttons, and Escape goes back to the text. A click outside the toolbar closes it (\`ondismiss\`).`
				}
			}
		}
	});
</script>

<Story name="Over a selection" asChild>
	<div style="height: 160px; position: relative">
		<p style="margin: 100px 0 0 40px; font-family: var(--font-prose)">To the north a whitewashed <mark>cottage</mark> huddles.</p>
		<SelectionTools
			rect={{ left: 40, top: 80, bottom: 100 }}
			actions={[
				{ id: 'thing', label: 'Make it a thing', run: () => {} },
				{ id: 'first', label: 'Only the first time', run: () => {} },
				{ id: 'when', label: 'Only when…', run: () => {} }
			]}
			onclose={() => {}}
		/>
	</div>
</Story>
