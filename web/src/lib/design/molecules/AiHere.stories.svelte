<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import AiHere from './AiHere.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/AiHere',
		component: AiHere,
		tags: ['autodocs'],
		args: { onreview: fn(), onproposals: fn(), findings: 0, proposals: 0, reviewing: null },
		parameters: {
			docs: {
				description: {
					component: `The AI help for the current room, in the room desk. It shows only while \`loremaster serve\` runs.

- **Review this room** starts the job \`review_room\` for the current room. While the review runs, the button shows the time and you cannot click it.
- **Findings**: the number of findings of the last review of this room. The findings are notes in the margin, next to their lines. This block does not show them again.
- **Proposals**: the number of open proposals that change this room. They show as ghost text on the page and as ghosts on the map. "Open ↗" opens Atlas › AI.

The jobs of the whole session are in the AI tray of the status line (StatusTray with AiStatus).`
				}
			}
		}
	});
</script>

<Story name="Nothing yet" />
<Story name="After a review" args={{ findings: 3, proposals: 1 }} />
<Story name="Review runs" args={{ reviewing: 14 }} />
