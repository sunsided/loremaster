<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import AppHeader from './AppHeader.svelte';

	const views = [
		{ id: 'play', label: 'Play and write' },
		{ id: 'map', label: 'Map', disabled: true },
		{ id: 'story', label: 'Story', disabled: true },
		{ id: 'rules', label: 'Rules', disabled: true }
	];

	const { Story } = defineMeta({
		title: 'Organisms/AppHeader',
		component: AppHeader,
		tags: ['autodocs'],
		args: {
			title: 'The Lighthouse',
			views,
			current: 'play',
			now: ['day 1 · 20:30 · dusk', 'The Shore'],
			settingsOpen: false,
			onview: fn(),
			onsettings: fn()
		},
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The app header shows the story title, the views and the state of the game now.

The page has one view now, so it passes no views. The views are for later top-level views, such as a project list. They are a \`nav\` list. The current view has \`aria-current="page"\`. A view that is not available yet is disabled.
The "now" tags show facts, not controls.
The "Look and motion" button opens the settings dialog. It has \`aria-haspopup="dialog"\` and \`aria-expanded\`.

On narrow screens the header wraps to more lines.`
				}
			}
		}
	});
</script>

<Story name="Default" args={{ views: [], current: '' }} />
<Story name="With views" />
<Story name="Before the game starts" args={{ now: [] }} />
