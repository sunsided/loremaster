<script lang="ts">
	import type { ReviewFinding } from '../types.ts';

	interface Props {
		finding: ReviewFinding;
		/** Hide the finding, for example when it is fixed or not a problem. */
		ondismiss?: () => void;
	}

	let { finding, ondismiss }: Props = $props();
</script>

<!--
	One finding of an AI review, in the margin next to its fragment: how serious it is, the problem and the fix.
	The severity is a word, not only a colour. Dismiss hides it.
-->
<div class="finding {finding.severity}" role="note" aria-label="AI review, {finding.severity}">
	<p>
		<span class="sev"><span aria-hidden="true">✦ </span>{finding.severity}</span>
		{finding.problem}
	</p>
	<p class="fix"><span class="label">Fix:</span> {finding.fix}</p>
	{#if ondismiss}<button type="button" class="dismiss" onclick={ondismiss} aria-label="Dismiss: {finding.problem}">×</button>{/if}
</div>

<style>
	.finding {
		position: relative;
		margin: var(--space-2) 0 0;
		padding: var(--space-1) var(--space-7) var(--space-1) var(--space-4);
		border-left: 2px dotted var(--color-ghost);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-body);
		color: var(--color-text);
		text-shadow: none;
	}
	.finding.high {
		border-left-style: solid;
		border-left-color: var(--color-warning);
	}
	p {
		margin: 0;
	}
	.sev {
		display: inline-block;
		margin-right: var(--space-2);
		padding: 0 var(--space-2);
		border: var(--size-border) solid var(--color-ghost);
		color: var(--color-ghost);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
	}
	.high .sev {
		border-color: var(--color-warning);
		color: var(--color-text-warning);
	}
	.low .sev {
		border-style: dashed;
	}
	.fix {
		color: var(--color-text-muted);
		font-style: italic;
	}
	.label {
		font-style: normal;
	}
	.dismiss {
		position: absolute;
		top: 0;
		right: 0;
		min-width: var(--size-target-compact);
		min-height: var(--size-target-compact);
		border: 0;
		background: none;
		color: var(--color-text-muted);
		font: inherit;
	}
	.dismiss:hover {
		color: var(--color-text-strong);
	}
	.dismiss:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: -2px;
	}
</style>
