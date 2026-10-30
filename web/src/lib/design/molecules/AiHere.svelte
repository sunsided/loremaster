<script lang="ts">
	import Button from '../atoms/Button.svelte';

	interface Props {
		/** Ask the AI to review the current room. */
		onreview: () => void;
		/** Seconds since the start of a review of this room that runs now. Null: no review runs. */
		reviewing?: number | null;
		/** The number of findings of the last review of this room. They show in the margin. */
		findings?: number;
		/** The number of open proposals that change this room. They show as ghost text. */
		proposals?: number;
		/** Open the proposals in the Atlas. */
		onproposals: () => void;
	}

	let { onreview, reviewing = null, findings = 0, proposals = 0, onproposals }: Props = $props();
</script>

<!--
	The AI help for the current room, in the room desk. It counts only what is about this room,
	and it tells where that shows: findings in the margin, proposals as ghost text.
	The jobs of the whole session are in the AI tray of the status line.
-->
<div class="ai-here">
	<div>
		<Button size="compact" onclick={onreview} disabled={reviewing !== null}
			>{reviewing !== null ? `✦ Reviewing… ${reviewing} s` : '✦ Review this room'}</Button
		>
	</div>
	<dl class="facts">
		<dt>{findings} {findings === 1 ? 'finding' : 'findings'}</dt>
		<dd>{findings ? 'in the margin, next to their lines' : 'none from the last review'}</dd>
		<dt>{proposals} {proposals === 1 ? 'proposal' : 'proposals'}</dt>
		<dd>
			{#if proposals}ghost text on the page ·
				<button type="button" class="link" onclick={onproposals}>Open ↗</button>{:else}none for this room{/if}
		</dd>
	</dl>
</div>

<style>
	.ai-here {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.facts {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: var(--space-1) var(--space-4);
		margin: 0;
		align-items: baseline;
	}
	dt {
		color: var(--color-text-strong);
	}
	dd {
		margin: 0;
		color: var(--color-text-muted);
	}
	.link {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-1);
		border: 0;
		background: none;
		color: var(--color-text-accent);
		font: inherit;
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	.link:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
</style>
