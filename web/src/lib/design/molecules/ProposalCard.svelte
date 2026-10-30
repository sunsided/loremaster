<script lang="ts">
	import type { ProposalSummary } from '../types.ts';

	interface Props {
		proposal: ProposalSummary;
		/** Show the list of changes and problems. The margin card is short; the Atlas card is full. */
		full?: boolean;
		onaccept: () => void;
		/** Reject, with an optional note why. The AI does not offer a rejected idea again. */
		onreject: (note?: string) => void;
		/** A problem from the last action, for example "The file changed after the proposal was made". */
		problem?: string | null;
		/** Play the story with the proposal in a sandbox. When set, the card offers Walk in. */
		onwalk?: () => void;
	}

	let { proposal, full = false, onaccept, onreject, problem = null, onwalk }: Props = $props();
	const id = $props.id();
	let rejecting = $state(false);
	let note = $state('');

	/** Keys while the focus is in the card, not in the note: A accepts, R rejects, W walks in. */
	function keys(e: KeyboardEvent) {
		if (e.ctrlKey || e.metaKey || e.altKey || e.target instanceof HTMLInputElement) return;
		const key = e.key.toLowerCase();
		if (key === 'a' && proposal.ok) onaccept();
		else if (key === 'r') rejecting = true;
		else if (key === 'w' && onwalk) onwalk();
		else return;
		e.preventDefault();
	}
</script>

<!--
	An AI proposal: its summary, whether the story builds with it, and Accept and Reject.
	Accept is off while the proposal has problems. The card sits next to its ghost text in the margin,
	and in Atlas › Proposals. With the focus in the card, A accepts, R rejects and W walks in.
	Alt+P moves the focus to the next proposal card on the page.
-->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section class="proposal" aria-labelledby="{id}-title" data-proposal={proposal.id} tabindex="-1" onkeydown={keys}>
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
	{#if rejecting}
		<form
			class="note"
			onsubmit={(e) => {
				e.preventDefault();
				onreject(note.trim() || undefined);
				rejecting = false;
			}}
		>
			<label for="{id}-note">Why? (optional, the AI reads it)</label>
			<!-- svelte-ignore a11y_autofocus -->
			<input id="{id}-note" bind:value={note} autocomplete="off" autofocus onkeydown={(e) => e.key === 'Escape' && (rejecting = false)} />
			<div class="actions">
				<button type="submit">Reject</button>
				<button type="button" onclick={() => (rejecting = false)}>Back</button>
			</div>
		</form>
	{:else}
		<div class="actions">
			<button type="button" class="accept" disabled={!proposal.ok} onclick={onaccept} aria-keyshortcuts="A" title="Accept (A)">Accept</button>
			{#if onwalk}<button type="button" onclick={onwalk} aria-keyshortcuts="W" title="Play it in a sandbox first (W)">Walk in</button>{/if}
			<button type="button" onclick={() => (rejecting = true)} aria-keyshortcuts="R" title="Reject (R)">Reject…</button>
		</div>
	{/if}
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
	.note {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.note label {
		color: var(--color-text-muted);
	}
	.note input {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-2);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface);
		color: var(--color-text);
		font: inherit;
	}
	.note input:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 1px;
	}
	button:disabled {
		opacity: 0.5;
	}
	.proposal:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
</style>
