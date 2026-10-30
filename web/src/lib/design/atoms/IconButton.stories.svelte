<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import IconButton from './IconButton.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/IconButton',
		component: IconButton,
		tags: ['autodocs'],
		args: { label: 'Commands', shortcut: 'Ctrl K', onclick: fn() },
		parameters: {
			docs: {
				description: {
					component: `A small square button with an icon, for actions that the author uses often and knows: the command palette, the settings.

- \`label\` is the accessible name. It also shows as a tooltip on hover and on keyboard focus.
- \`shortcut\` shows in the tooltip and goes to \`aria-keyshortcuts\`.
- The icon is hidden from screen readers. Use a small SVG with \`currentColor\`, or one or two characters.

The button is 32px high. Use it in toolbars, not as the main action of a form.`
				}
			}
		}
	});
</script>

<Story name="Text icon" asChild>
	<div style="padding: 8px 120px 40px 8px; display: flex; justify-content: flex-end">
		<IconButton label="Commands" shortcut="Ctrl K" onclick={() => {}}>⌘</IconButton>
	</div>
</Story>
