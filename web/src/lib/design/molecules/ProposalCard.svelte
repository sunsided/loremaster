<script lang="ts">
	import type { ProposalSummary } from '../types.ts';

	interface Props {
		proposal: ProposalSummary;
		/** Show the list of changes and problems. The margin card is short; the Atlas card is full. */
		full?: boolean;
		onaccept: () => void;
		onreject: () => void;
		/** A problem from the last action, for example "The file changed after the proposal was made". */
		problem?: string | null;
	}

	let { proposal, full = false, onaccept, onreject, problem = null }: Props = $props();
	const id = $props.id();
</script>

<!--
	An AI proposal: its summary, whether the story builds with it, and Accept and Reject.
	Accept is off while the proposal has problems. The card sits next to its ghost text in the margin,
	and in Atlas › Proposals.
-->
<section class="proposal" aria-labelledby="{id}-title">
	<p class="head" id="{id}-title"><span aria-hidden="true">✦ </span>Proposal {proposal.id} · {proposal.summary}</p>
	<p class="check" class:bad={!proposal.ok || proposal.stale}>
		{#if !proposal.ok}{proposal.problems.length} {proposal.problems.length === 1 ? 'problem' : 'problems'}{:else if proposal.stale}a file
			changed since{:else}checked, 0 problems{/if}
	</p>
	{#if full}
		{#if proposal.changes.length}
			<ul class="changes">
				{#each proposal.changes as c, i (i)}<li>{c}</li>{/each}
			</ul>
		{/if}
		{#if proposal.problems.length}
			<ul class="problems">
				{#each proposal.problems as p, i (i)}<li>{p}</li>{/each}
			</ul>
		{/if}
	{/if}
	{#if problem}<p class="problem" role="alert">{problem}</p>{/if}
	<div class="actions">
		<button type="button" class="accept" disabled={!proposal.ok} onclick={onaccept}>Accept</button>
		<button type="button" onclick={onreject}>Reject</button>
	</div>
</section>

<style>
	.proposal {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3);
		border: var(--size-border) solid var(--color-ghost);
		background: var(--color-surface-raised);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-body);
		color: var(--color-text);
		text-shadow: none;
	}
	p {
		margin: 0;
	}
	.head {
		color: var(--color-ghost);
		font-weight: var(--font-weight-strong);
	}
	.check {
		color: var(--color-text-muted);
	}
	.check.bad,
	.problem,
	.problems {
		color: var(--color-text-warning);
	}
	ul {
		margin: 0;
		padding-left: var(--space-5);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	button {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-3);
		border: var(--size-border) solid var(--color-border-control);
		background: none;
		color: var(--color-text);
		font: inherit;
	}
	.accept {
		border-color: var(--color-ghost);
		background: var(--color-ghost);
		color: var(--color-surface);
		font-weight: var(--font-weight-strong);
	}
	button:disabled {
		opacity: 0.5;
	}
	button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
</style>
