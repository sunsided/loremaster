<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import { fn } from 'storybook/test';
	import { fragments } from '../fixtures.ts';
	import AssemblyPanel from './AssemblyPanel.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/AssemblyPanel',
		component: AssemblyPanel,
		tags: ['autodocs'],
		args: { room: 'shore', fragments, focus: null, time: '20:30', phase: 'dusk', onfocus: fn() },
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The assembly panel shows how the runtime makes the text of the current room. It answers two questions: "Why does this text show?" and "Why does that text not show?"

The panel has one section for each layer, in assembly order: base, ambient, event, objects, characters, exits.
Each fragment shows its source text, its mode and its condition. A hidden fragment also shows the reason.
The runtime makes the objects and exits layers from the things and exits of the room, so these layers have no fragments.

Hover on a fragment highlights the same text in the transcript.
The panel does not change the game state.`
				}
			}
		}
	});
</script>

{#snippet template(args: ComponentProps<typeof AssemblyPanel>)}
	<div style="height: 640px; max-width: 480px; display: flex">
		<AssemblyPanel {...args} />
	</div>
{/snippet}

<Story name="Shore at dusk" {template} />
<Story name="Fragment highlighted" args={{ focus: 1 }} {template} />
<Story name="Empty room" args={{ fragments: [] }} {template} />
