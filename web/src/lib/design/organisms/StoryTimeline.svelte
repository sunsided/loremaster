<script lang="ts">
	import Heading from '../atoms/Heading.svelte';
	import type { StoryStep } from '../types.ts';

	interface Props {
		steps: StoryStep[];
	}

	let { steps }: Props = $props();
</script>

<!--
	The playthrough so far, one step per turn: when it happened, where, what the player did,
	which rules fired and what state changed. It is an ordered list, oldest first.
-->
<section class="timeline" aria-labelledby="timeline-title">
	<Heading level={2} look="title" id="timeline-title">Story so far</Heading>
	<p class="summary">
		This is the playthrough of this session. Each step is one turn. Rule names and state changes show why the
		story went this way.
	</p>
	<ol>
		{#each steps as step, i (i)}
			<li class="step" class:ended={step.ended}>
				<div class="when">
					<span class="time">{step.time}</span>
					<span class="phase">{step.phase}</span>
				</div>
				<div class="what">
					<p class="command">
						{#if step.command}<span aria-hidden="true">&gt;&nbsp;</span><span class="visually-hidden">Command: </span>{step.command}
						{:else}The game starts{/if}
						<span class="room">in {step.room}</span>
					</p>
					{#if step.rules.length}
						<p class="facts"><span class="label">rules</span>{#each step.rules as r (r)}<code>{r}</code>{/each}</p>
					{/if}
					{#if step.changes.length}
						<p class="facts"><span class="label">changes</span>{#each step.changes as c, j (j)}<code>{c}</code>{/each}</p>
					{/if}
					{#if step.ended}<p class="end">The game ended here.</p>{/if}
				</div>
			</li>
		{:else}
			<li class="empty">No turns yet.</li>
		{/each}
	</ol>
</section>

<style>
	.timeline {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	.summary {
		margin: 0;
		color: var(--color-text-muted);
	}
	ol {
		margin: 0;
		padding: 0;
		list-style: none;
		border-left: var(--size-border) solid var(--color-border-control);
	}
	.step {
		display: grid;
		grid-template-columns: 7ch minmax(0, 1fr);
		gap: var(--space-6);
		padding: var(--space-4) 0 var(--space-4) var(--space-6);
		position: relative;
	}
	.step::before {
		content: '';
		position: absolute;
		left: -5px;
		top: var(--space-5);
		width: 9px;
		height: 9px;
		background: var(--color-surface);
		border: var(--size-border) solid var(--color-border-control);
	}
	.step:has(.facts)::before {
		background: var(--color-accent);
		border-color: var(--color-accent);
	}
	.when {
		display: flex;
		flex-direction: column;
		color: var(--color-text-muted);
	}
	.time {
		color: var(--color-text);
		font-weight: var(--font-weight-strong);
	}
	.phase {
		font-size: var(--font-size-sm);
	}
	.what p {
		margin: 0 0 var(--space-2);
	}
	.command {
		color: var(--color-text-strong);
	}
	.room {
		color: var(--color-text-muted);
		margin-left: var(--space-3);
	}
	.facts {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
		font-size: var(--font-size-sm);
	}
	.label {
		color: var(--color-text-muted);
		min-width: 7ch;
	}
	code {
		padding: 0 var(--space-3);
		border: var(--size-border) solid var(--color-border-subtle);
		overflow-wrap: anywhere;
	}
	.end {
		color: var(--color-text-accent);
	}
	.empty {
		padding: var(--space-4) var(--space-6);
		color: var(--color-text-muted);
	}
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
