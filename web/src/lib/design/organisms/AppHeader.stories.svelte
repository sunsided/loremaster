<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import AppHeader from './AppHeader.svelte';

	const projectItems = [
		{ id: 'folder', label: 'Open a story folder on disk', run: fn(), group: 'Files' },
		{ id: 'export', label: 'Export the story as a zip file', run: fn(), group: 'Files' },
		{ id: 'import', label: 'Import a story from a zip file', run: fn(), group: 'Files' },
		{ id: 'cantina', label: 'Open the example “cantina”', run: fn(), group: 'Examples' }
	];

	const { Story } = defineMeta({
		title: 'Organisms/AppHeader',
		component: AppHeader,
		tags: ['autodocs'],
		args: {
			title: 'The Lighthouse',
			saved: 'saved in browser',
			projectItems,
			mode: 'play',
			onmode: fn(),
			settingsOpen: false,
			onsettings: fn(),
			onpalette: fn()
		},
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The app header has four things:

1. The **project menu**: the story title with a ▾. It opens the file actions (story folder, zip export and import, back to the bundled example) and the other examples. Next to it, a dot and a few words tell where the changes go, for example "saved in browser". The dot is in the warning colour when the author must act, for example to give folder access again.
2. The **Play / Write switch** (ModeSwitch). Ctrl+E (Cmd+E) switches from anywhere.
3. The **Commands** icon button (⌘). It opens the command palette, also with Ctrl+K.
4. The **Look and motion** icon button (gear). It opens the settings dialog.

The state of the game (room, time, turn) is not in the header. It is in the status line of the screen (StatusLine).
On a phone the header fits in one row.`
				}
			}
		}
	});
</script>

<Story name="Play" />
<Story name="Write" args={{ mode: 'write', saved: 'edited, saved in browser' }} />
<Story name="Folder needs access" args={{ saved: 'folder needs access', savedState: 'attention' }} />
