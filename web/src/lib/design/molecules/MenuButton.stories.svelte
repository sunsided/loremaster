<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import MenuButton from './MenuButton.svelte';

	const items = [
		{ id: 'folder', label: 'Open a story folder on disk', run: fn(), group: 'Files' },
		{ id: 'export', label: 'Export the story as a zip file', run: fn(), group: 'Files' },
		{ id: 'cantina', label: 'Open the example “cantina”', run: fn(), group: 'Examples' },
		{ id: 'tower', label: 'Open the example “tower”', run: fn(), group: 'Examples', disabled: true }
	];

	const { Story } = defineMeta({
		title: 'Molecules/MenuButton',
		component: MenuButton,
		tags: ['autodocs'],
		args: { label: 'Project: The Lighthouse', items },
		parameters: {
			docs: {
				description: {
					component: `A button that opens a short list of actions under it, for example the project menu in the header and the turn menu in the status line.

- It is a disclosure: the button has \`aria-expanded\` and \`aria-controls\`. The actions are normal buttons in a list.
- When it opens, the focus goes to the first action. The arrow keys, Home and End move between the actions. Escape closes the list and returns the focus to the button. A click outside closes it.
- An item with \`group\` starts a new section with that heading. A disabled item stays in the list.
- \`look="inverse"\` is for the status line. \`align="end"\` opens the list under the right edge.`
				}
			},
			story: { iframeHeight: 260 }
		}
	});
</script>

<Story name="Project menu" asChild>
	<div style="min-height: 240px">
		<MenuButton label="Project: The Lighthouse" {items}><strong>THE LIGHTHOUSE</strong></MenuButton>
	</div>
</Story>
