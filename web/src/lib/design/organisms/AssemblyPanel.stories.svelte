<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import { fn } from 'storybook/test';
	import { fragments, lookFrame, parts } from '../fixtures.ts';
	import AssemblyPanel from './AssemblyPanel.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/AssemblyPanel',
		component: AssemblyPanel,
		tags: ['autodocs'],
		args: { room: 'shore', fragments, parts, focus: null, time: '20:30', phase: 'dusk', onfocus: fn() },
		parameters: {
			layout: 'fullscreen',
			docs: {
				description: {
					component: `The assembly panel shows how the runtime makes the text of the current room. It answers two questions: "Why does this text show?" and "Why does that text not show?"

The panel has one section for each layer, in assembly order: base, ambient, event, objects, characters, exits.
Each fragment shows its source text, its mode and its condition. A hidden fragment also shows the reason.
The runtime makes the objects and exits layers from the things and exits of the room. These layers are in a dashed box with the tag \"generated\", so they look different from the written fragments. They show a hint, then each thing or exit with its state (PartItem).

Hover on a fragment highlights the same text in the transcript.
When the last action used an action frame, the panel starts with a \"last action\" box. It names the verb, the frame (on the story, a room or a thing) and the counters \`action.repeat\`, \`action.count\` and \`action.total\`. Then it lists each fragment of the frame: a check mark when it showed, or a dash and the reason when it did not.

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
<Story name="After a framed action" args={{ frame: lookFrame }} {template} />
<Story name="Empty room" args={{ fragments: [], parts: { objects: [], exits: [] } }} {template} />
