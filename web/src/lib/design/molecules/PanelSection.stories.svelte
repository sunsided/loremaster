<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import PanelSection from './PanelSection.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/PanelSection',
		component: PanelSection,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `One section of a side panel: a heading with a button that folds the section, a short fact on the right (\`meta\`), optional links (\`actions\`) and the content.

- The heading is a real \`h2\`, so screen readers can move from section to section. The button in it has \`aria-expanded\`.
- The folded state of each \`id\` stays across visits (browser storage).
- Use sections instead of tabs when the user wants to see more than one thing at a time.`
				}
			}
		}
	});
</script>

<Story name="Open" asChild>
	<div style="max-width: 360px">
		<PanelSection id="story-rules" title="Rules here" meta="1 of 6">
			{#snippet actions()}<a href="#all">All ↗</a>{/snippet}
			<p style="margin: 0">dusk_falls · fires in 30 minutes</p>
		</PanelSection>
	</div>
</Story>
