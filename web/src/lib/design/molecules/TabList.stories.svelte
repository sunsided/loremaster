<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import type { ComponentProps } from 'svelte';
	import TabList from './TabList.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/TabList',
		component: TabList,
		tags: ['autodocs'],
		args: {
			label: 'Inspector',
			idBase: 'demo',
			selected: 'assembly',
			tabs: [
				{ id: 'assembly', label: 'Assembly' },
				{ id: 'map', label: 'Map' },
				{ id: 'story', label: 'Story' },
				{ id: 'rules', label: 'Rules' }
			]
		},
		parameters: {
			docs: {
				description: {
					component: `A tab list selects one of a small set of panels. It follows the WAI-ARIA tabs pattern.

Keyboard:
1. Tab moves focus to the selected tab only.
2. The Left and Right arrow keys move to the previous or next tab and select it.
3. Home selects the first tab. End selects the last tab.

Each tab controls the element with the id \`{idBase}-panel\`. The panel must have \`role="tabpanel"\` and \`aria-labelledby\` set to the id of the selected tab. InspectorPanel does this.`
				}
			}
		}
	});
</script>

{#snippet template(args: ComponentProps<typeof TabList>)}
	<TabList {...args} />
	<div role="tabpanel" id="{args.idBase}-panel" aria-labelledby="{args.idBase}-tab-{args.selected}" tabindex="0" style="padding: var(--space-6)">
		Content of the {args.selected} tab.
	</div>
{/snippet}

<Story name="Assembly selected" {template} />
<Story name="Rules selected" args={{ selected: 'rules' }} {template} />
