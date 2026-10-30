<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import GhostText from './GhostText.svelte';

	const { Story } = defineMeta({
		title: 'Atoms/GhostText',
		component: GhostText,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `Text that an AI proposal suggests and that nobody accepted yet.

It has three cues, so colour is not the only one: the ghost colour (\`--color-ghost\`, a cooler tone than the accent in each theme), italics with a dotted line, and a ✦ mark. Screen readers hear "proposal:" before the text.
\`look="block"\` is for a line of its own, for example a new fragment in the margin view.`
				}
			}
		}
	});
</script>

<Story name="Inline" asChild>
	<p style="font-family: var(--font-prose)">The sun is a red coal <GhostText>, and gulls cry over the slipway</GhostText>.</p>
</Story>
<Story name="Block" asChild>
	<GhostText look="block">The sun is a red coal sinking into the sea, and gulls cry over the slipway.</GhostText>
</Story>
