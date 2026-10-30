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
		args: { jobs, differ: null, onreview: fn(), oncancel: fn(), onlink: fn(), onproposals: fn() },
		parameters: {
			docs: {
				description: {
					component: `The AI section of the Here panel. It shows only while \`lantern serve\` runs.

- **Review this room** starts the job \`review_room\` for the current room. The job plays from the state of the editor.
- **The jobs**: what Claude Code does now ("Draft an exit", "Review the room"), what for, the time, the proposals it made and the cost. A running job has Cancel.
- **A warning** when the server reads other files than the editor (the editor keeps a working copy in the browser; the server reads the folder on disk). Proposals for those files cannot be accepted. "Open the story folder" links the folder, so both use the same files.`
				}
			}
		}
	});
</script>

<Story name="Jobs" />
<Story name="Files differ" args={{ jobs: [], differ: ['rooms/shore.yaml'] }} />
