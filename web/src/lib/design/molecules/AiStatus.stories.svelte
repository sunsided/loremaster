<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import AiStatus from './AiStatus.svelte';

	const jobs = [
		{ id: 'j2', task: 'imagine_exit', about: 'shore, down', status: 'running' as const, seconds: 12, cost: null, proposals: 1 },
		{ id: 'j1', task: 'review_room', about: 'cottage', status: 'done' as const, seconds: 0, cost: 0.04, proposals: 1 }
	];

	const { Story } = defineMeta({
		title: 'Molecules/AiStatus',
		component: AiStatus,
		tags: ['autodocs'],
		args: { jobs, differ: null, oncancel: fn(), onlink: fn(), onproposals: fn(), onagain: fn() },
		parameters: {
			docs: {
				description: {
					component: `The AI jobs of the session. It shows only while \`loremaster serve\` runs. It is the content of the AI tray of the status line (StatusTray), and it is at the top of Atlas › AI.

- **The jobs**: what Claude Code does now ("Draft an exit", "Review the room", "Draft the first rooms"), what for, the time, the proposals it made and the cost. A running job has Cancel.
- **The cost**: the sum of the cost of all jobs of the session.
- **Another take**: the draft of a new story can run again while its proposal is open. The button shows the cost of the last job, so that the author decides.
- **A warning** when the server reads other files than the editor (the editor keeps a working copy in the browser; the server reads the folder on disk). Proposals for those files cannot be accepted. "Open the story folder" links the folder, so both use the same files.
- **All jobs and proposals ↗** opens Atlas › AI. Leave out \`onproposals\` in the Atlas itself.

The help for the current room ("Review this room", findings, proposals here) is in the room desk (AiHere).`
				}
			}
		}
	});
</script>

<Story name="Jobs" />
<Story
	name="New story draft"
	args={{
		jobs: [
			{
				id: 'j1',
				task: 'seed_story',
				about: 'A lighthouse keeper writes letters to a ship that sank.',
				status: 'done' as const,
				seconds: 0,
				cost: 0.26,
				proposals: 1,
				again: true
			}
		]
	}}
/>
<Story name="No jobs yet" args={{ jobs: [] }} />
<Story name="Files differ" args={{ jobs: [], differ: ['rooms/shore.yaml'] }} />
