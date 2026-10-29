<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import FragmentMarker from './FragmentMarker.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/FragmentMarker',
		component: FragmentMarker,
		tags: ['autodocs'],
		args: { number: 2, active: false, onactivate: fn() },
		parameters: {
			docs: {
				description: {
					component: `A fragment marker shows the number of a fragment before its text in the transcript.

Hover or focus highlights the same fragment in the assembly panel. A click keeps the highlight on. A second click removes it.
Screen readers announce "Fragment 2" and the pressed state.

The marker shows only when "Show sources" is on.`
				}
			}
		}
	});
</script>

<Story name="Default" />
<Story name="Active" args={{ active: true }} />
<Story name="In prose" asChild>
	<p style="font-family: var(--font-prose); font-size: var(--font-size-prose); max-width: var(--measure-prose)">
		<FragmentMarker number={1} />Shingle crunches underfoot. <FragmentMarker number={2} active />The sun is a red coal sinking
		into the sea.
	</p>
</Story>
