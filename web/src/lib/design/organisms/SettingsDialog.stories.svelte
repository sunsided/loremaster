<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Button from '../atoms/Button.svelte';
	import SettingsDialog from './SettingsDialog.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/SettingsDialog',
		component: SettingsDialog,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `The settings dialog sets the theme, the visual effects, the motion and the fragment numbers.

It is a modal \`dialog\`. When it opens, focus goes into the dialog and stays there. Escape closes it. When it closes, focus goes back to the button that opened it.

The motion setting has two choices:
- "Use my system setting" follows \`prefers-reduced-motion\` of the operating system.
- "Always reduce motion" stops all animation in all cases.

Reduced motion keeps the textures, because they do not move.
The app keeps these settings in the browser storage of the user.`
				},
				story: { inline: false, iframeHeight: 640 }
			}
		}
	});
</script>

<script lang="ts">
	let open = $state(false);
	let theme = $state<'phosphor' | 'paper' | 'night'>('phosphor');
	let effects = $state<'full' | 'subtle' | 'off'>('full');
	let motion = $state<'system' | 'reduce'>('system');
	let sources = $state(true);
</script>

<Story name="Open from a button" asChild>
	<Button aria-haspopup="dialog" aria-expanded={open} onclick={() => (open = true)}>Look and motion</Button>
	<p>Theme: {theme} · effects: {effects} · motion: {motion} · numbers: {sources ? 'on' : 'off'}</p>
	<SettingsDialog bind:open bind:theme bind:effects bind:motion bind:sources />
</Story>
