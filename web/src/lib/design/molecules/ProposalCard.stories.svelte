<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import ProposalCard from './ProposalCard.svelte';

	const proposal = {
		id: 'p1',
		summary: 'More life at dusk',
		ok: true,
		stale: false,
		problems: [],
		changes: ['room:shore · fragment 2 changed']
	};

	const { Story } = defineMeta({
		title: 'Molecules/ProposalCard',
		component: ProposalCard,
		tags: ['autodocs'],
		args: { proposal, onaccept: fn(), onreject: fn() },
		parameters: {
			docs: {
				description: {
					component: `An AI proposal from \`lantern serve\`: its id and summary, whether the story builds with it, and Accept and Reject.

- In write mode the card sits in the margin next to its ghost text. In Atlas › Proposals it is \`full\`: it also lists the changes and the problems.
- Accept is off while the proposal has problems. Accept applies the proposal to the working copy of the editor (only when the files did not change since the proposal), then tells the server.
- Reject asks for an optional note first ("Why?"). The server keeps the note, and the AI does not offer the idea again.
- A problem from the last action shows in the card, for example "The file rooms/shore.yaml changed after the proposal was made".`
				}
			}
		}
	});
</script>

<Story name="Checked" />
<Story name="Full" args={{ full: true }} />
<Story name="With problems" args={{ proposal: { ...proposal, ok: false, problems: ['unknown name `gull`'] }, full: true }} />
<Story name="Stale" args={{ proposal: { ...proposal, stale: true }, problem: 'The file rooms/shore.yaml changed after the proposal was made.' }} />
