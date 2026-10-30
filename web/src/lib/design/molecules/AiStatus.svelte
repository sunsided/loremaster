<script lang="ts">
	import Button from '../atoms/Button.svelte';

	interface Job {
		id: string;
		task: string;
		/** A few words about the job, for example "shore, down". */
		about: string;
		status: 'running' | 'done' | 'failed' | 'cancelled';
		/** Seconds since the start, for a running job. */
		seconds: number;
		cost: number | null;
		proposals: number;
		/** True when the job can run again for another version, for example the first rooms of a new story. */
		again?: boolean;
	}

	interface Props {
		jobs: Job[];
		/** Files that the AI server reads and that differ from the files of the editor. Null: not checked. */
		differ?: string[] | null;
		oncancel: (id: string) => void;
		/** Link the story folder, so that the editor and the AI server use the same files. Leave it out where the browser cannot. */
		onlink?: () => void;
		/** Open the jobs and proposals in the Atlas. Leave it out in the Atlas itself. */
		onproposals?: () => void;
		/** Run a job again for another version. The old proposals stay. */
		onagain?: (id: string) => void;
	}

	let { jobs, differ = null, oncancel, onlink, onproposals, onagain }: Props = $props();
	const cost = $derived(jobs.reduce((sum, j) => sum + (j.cost ?? 0), 0));
	const names: Record<string, string> = {
		imagine_exit: 'Draft an exit',
		review_room: 'Review the room',
		seed_story: 'Draft the first rooms'
	};
</script>

<!--
	The AI jobs of the session, while `loremaster serve` runs: what Claude Code does now and did, with the cost,
	and a warning when the server reads other files than the editor. It shows in the AI tray of the status line
	and in Atlas › AI. The help for the current room is in the room desk (AiHere).
-->
<div class="ai-status">
	{#if differ && differ.length}
		<div class="warning" role="status">
			<p>
				The AI reads the story folder on disk, and {differ.length}
				{differ.length === 1 ? 'file differs' : 'files differ'} from the editor: {differ.slice(0, 3).join(', ')}{differ.length > 3
					? ', …'
					: ''}. Proposals for these files cannot be accepted.
			</p>
			{#if onlink}<Button size="compact" onclick={onlink}>Open the story folder</Button>{/if}
		</div>
	{/if}
	{#if jobs.length}
		<ul class="jobs">
			{#each jobs as j (j.id)}
				<li>
					<span class="name">{names[j.task] ?? j.task}</span>
					<span class="about">{j.about}</span>
					<span class="state" class:running={j.status === 'running'}
						>{j.status === 'running' ? `${j.seconds} s` : j.status}{j.proposals ? ` · ${j.proposals} ${j.proposals === 1 ? 'proposal' : 'proposals'}` : ''}{j.cost !== null
							? ` · $${j.cost.toFixed(2)}`
							: ''}</span
					>
					{#if j.status === 'running'}<Button size="compact" variant="quiet" onclick={() => oncancel(j.id)}>Cancel</Button>{/if}
					{#if j.again && onagain && j.status !== 'running'}
						<Button size="compact" variant="quiet" onclick={() => onagain(j.id)}
							>✦ Another take{j.cost !== null ? ` (last: $${j.cost.toFixed(2)})` : ''}</Button
						>
					{/if}
				</li>
			{/each}
		</ul>
	{:else}
		<p class="muted">No jobs yet. Start one from the room desk in write mode, or from an exit.</p>
	{/if}
	<div class="row">
		{#if cost > 0}<span class="muted">${cost.toFixed(2)} for {jobs.length} {jobs.length === 1 ? 'job' : 'jobs'}</span>{/if}
		{#if onproposals}<Button size="compact" variant="quiet" onclick={onproposals}>All jobs and proposals ↗</Button>{/if}
	</div>
</div>

<style>
	.ai-status {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.warning {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-2);
		padding: var(--space-3);
		border: var(--size-border) solid var(--color-warning);
		color: var(--color-text-warning);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-body);
	}
	p {
		margin: 0;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.muted {
		color: var(--color-text-muted);
	}
	.jobs {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.jobs li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0 var(--space-3);
	}
	.name {
		color: var(--color-text-strong);
	}
	.about,
	.state {
		color: var(--color-text-muted);
	}
	.state.running {
		color: var(--color-ghost);
	}
</style>
