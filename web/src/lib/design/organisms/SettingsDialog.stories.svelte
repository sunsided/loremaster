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
					component: `The settings dialog sets the theme, the visual effects, the motion and the author tooltips.

It is a modal \`dialog\`. When it opens, focus goes into the dialog and stays there. Escape closes it. When it closes, focus goes back to the button that opened it.

The motion setting has two choices:
- "Use my system setting" follows \`prefers-reduced-motion\` of the operating system.
- "Always reduce motion" stops all animation in all cases.

"New text appears" sets how the game text shows with effects on: \"Line by line\" types each line after the other, like a terminal; \"A paragraph at once\" wipes each paragraph in one move. With effects off or reduced motion, the text shows at once.

Reduced motion keeps the textures, because they do not move.
The app keeps these settings in the browser storage of the user.

On a hosted page (\`hostedOrigin\` is set), the dialog also offers the link to \`loremaster serve\` on the computer of the user. It is off by default, because the browser asks for permission to connect to the local network.`
				},
				story: { inline: false, iframeHeight: 640 }
			}
		}
	});
</script>

<script lang="ts">
	let open = $state(false);
	let theme = $state<'phosphor' | 'night' | 'paper'>('phosphor');
	let effects = $state<'full' | 'subtle' | 'off'>('full');
	let motion = $state<'system' | 'reduce'>('system');
	let hostedOpen = $state(false);
	let serveLink = $state(false);
</script>

<Story name="Open from a button" asChild>
	<Button aria-haspopup="dialog" aria-expanded={open} onclick={() => (open = true)}>Look and motion</Button>
	<p>Theme: {theme} · effects: {effects} · motion: {motion}</p>
	<SettingsDialog bind:open bind:theme bind:effects bind:motion />
</Story>

<Story name="On a hosted page" asChild>
	<Button aria-haspopup="dialog" aria-expanded={hostedOpen} onclick={() => (hostedOpen = true)}>Look and motion</Button>
	<p>Link to loremaster serve: {serveLink ? 'on' : 'off'}</p>
	<SettingsDialog bind:open={hostedOpen} hostedOrigin="https://sunsided.github.io" bind:serveLink />
</Story>
