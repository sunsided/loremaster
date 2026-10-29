<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import ScreenEffects from './ScreenEffects.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/ScreenEffects',
		component: ScreenEffects,
		tags: ['autodocs'],
		args: { theme: 'phosphor', effects: 'full' },
		argTypes: {
			theme: { control: 'inline-radio', options: ['phosphor', 'night', 'paper'] },
			effects: { control: 'inline-radio', options: ['full', 'subtle', 'off'] }
		},
		parameters: {
			docs: {
				description: {
					component: `Screen effects are the texture layers over the player screen: grain, paper fibre, scanlines and vignette.

They are decoration only. Screen readers ignore them and they do not get pointer events.
The phosphor theme has grain, scanlines and a vignette. The paper theme has grain and paper fibre. The night theme has grain only.

Put the component as the first child of an element with \`position: relative\` and \`overflow: hidden\`.
Set the theme in the Storybook toolbar to the same value as the \`theme\` control.`
				}
			}
		}
	});
</script>

{#snippet template(args: ComponentProps<typeof ScreenEffects>)}
	<div
		class="fx-{args.effects}"
		style="position: relative; overflow: hidden; height: 240px; padding: var(--space-8); background: var(--color-surface); font-family: var(--font-prose); font-size: var(--font-size-prose)"
	>
		<ScreenEffects {...args} />
		<p style="position: relative; z-index: var(--z-content); margin: 0">The sun is a red coal sinking into the sea.</p>
	</div>
{/snippet}

<Story name="Full" {template} />
<Story name="Subtle" args={{ effects: 'subtle' }} {template} />
<Story name="Off" args={{ effects: 'off' }} {template} />
