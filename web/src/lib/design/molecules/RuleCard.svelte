<script lang="ts">
	import type { RuleView } from '#lib/engine/types.ts';

	interface Props {
		rule: RuleView;
	}

	let { rule }: Props = $props();
</script>

<!--
	One rule as three short lines: When (the event), If (the condition), Then (the effects).
	The state is in text: "fired" or "not fired yet", and "true now" or "false now" for the condition.
-->
<!-- tabindex="-1" lets other parts of the app, such as the command palette, move focus to the card. -->
<article class="rule" class:fired={rule.fired} id="rule-card-{rule.id}" tabindex="-1" aria-labelledby="rule-{rule.id}">
	<header>
		<h3 id="rule-{rule.id}"><code>{rule.id}</code></h3>
		<ul class="badges">
			<li class="state">{rule.fired ? 'fired' : 'not fired yet'}</li>
			{#if rule.once}<li>once</li>{/if}
			{#if rule.instead}<li>instead of the default</li>{/if}
		</ul>
	</header>
	<dl>
		<dt>When</dt>
		<dd>{rule.when}</dd>
		<dt>If</dt>
		<dd>
			{#if rule.condition}
				<code>{rule.condition}</code>
				<span class="holds" class:false={!rule.condition_holds}>{rule.condition_holds ? 'true now' : 'false now'}</span>
			{:else}
				<span class="none">always</span>
			{/if}
		</dd>
		<dt>Then</dt>
		<dd>
			<ol>
				{#each rule.then as effect, i (i)}<li>{effect}</li>{/each}
			</ol>
		</dd>
	</dl>
</article>

<style>
	.rule {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-5) var(--space-6);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
	}
	.rule:focus {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.fired {
		border-left: var(--size-border-strong) solid var(--color-accent);
	}
	header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-4);
	}
	h3 {
		margin: 0;
		font-size: var(--font-size-md);
		color: var(--color-text-accent);
	}
	.badges {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin: 0;
		padding: 0;
		list-style: none;
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.badges li {
		padding: 0 var(--space-3);
		border: var(--size-border) solid var(--color-border-subtle);
	}
	.fired .state {
		border-color: var(--color-accent);
		color: var(--color-text-accent);
	}
	dl {
		display: grid;
		grid-template-columns: max-content minmax(0, 1fr);
		gap: var(--space-2) var(--space-5);
		margin: 0;
	}
	dt {
		color: var(--color-text-muted);
		font-weight: var(--font-weight-strong);
	}
	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
	ol {
		margin: 0;
		padding-left: var(--space-6);
	}
	.holds {
		margin-left: var(--space-3);
		font-size: var(--font-size-sm);
		color: var(--color-text-accent);
	}
	.holds.false {
		color: var(--color-text-warning);
	}
	.none {
		color: var(--color-text-muted);
	}
</style>
