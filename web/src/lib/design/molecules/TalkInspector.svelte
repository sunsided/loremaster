<script lang="ts">
	import type { TalkView } from '#lib/engine/types.ts';

	interface Props {
		talk: TalkView;
	}

	let { talk }: Props = $props();

	const hidden = $derived(talk.options.filter((o) => o.choice === null).length);
	const pieces = (text: string) => text.split('*').map((t, i) => ({ text: t, em: i % 2 === 1 }));
</script>

<!--
	The talk in progress, for the playtest inspector: the node, what entering it changed, and all its answers.
	An answer that does not show now is struck out, with the condition that is false.
	The relations and memories of the character show at the end: the room matrix and the explorer do not show them.
-->
<div class="talk">
	<p class="where">
		<span class="node">{talk.node}</span>
		<span class="muted">in dialogues/{talk.dialogue}.yaml</span>
	</p>
	{#if talk.when}<p class="line"><span class="label">when</span> <code>{talk.when}</code></p>{/if}
	{#if talk.outcomes.length}
		<p class="line"><span class="label">on enter</span> {talk.outcomes.join(' · ')}</p>
	{/if}
	<ol class="options" aria-label="Answers{hidden ? `, ${hidden} hidden` : ''}">
		{#each talk.options as o, i (i)}
			<li class:hidden={o.choice === null}>
				<span class="n">{o.choice === null ? '–' : o.choice + 1}</span>
				<span class="body">
					<span class="say"
						>{#if o.choice === null}<span class="visually-hidden">Hidden: </span>{/if}{#each pieces(o.say) as p, j (j)}{#if p.em}<em
									>{p.text}</em
								>{:else}{p.text}{/if}{/each}</span
					>
					<span class="detail">
						{#if o.choice === null && o.when}<span class="why">false now: <code>{o.when}</code></span>{:else if o.when}<code>{o.when}</code>{/if}
						<span>→ {o.go === 'end' ? 'ends the talk' : o.go === 'back' ? 'back' : o.go}</span>
						{#if o.outcomes.length}<span>· {o.outcomes.join(' · ')}</span>{/if}
					</span>
				</span>
			</li>
		{/each}
	</ol>
	<dl class="state">
		{#each talk.relations as [axis, value] (axis)}
			<div><dt>{axis}</dt><dd>{value}</dd></div>
		{/each}
		<div>
			<dt>remembers</dt>
			<dd>{talk.memories.length ? talk.memories.join(', ') : 'nothing'}</dd>
		</div>
	</dl>
</div>

<style>
	.talk {
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-body);
	}
	.where,
	.line {
		margin: 0 0 var(--space-2);
		overflow-wrap: anywhere;
	}
	.node {
		color: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
	}
	.muted,
	.label {
		color: var(--color-text-muted);
	}
	.label {
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
	}
	.options {
		margin: var(--space-3) 0;
		padding: 0;
		list-style: none;
	}
	.options li {
		display: flex;
		gap: var(--space-3);
		margin-bottom: var(--space-3);
	}
	.n {
		flex: none;
		min-width: 1.5em;
		color: var(--color-text-accent);
	}
	.body {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.say {
		color: var(--color-text-strong);
		overflow-wrap: break-word;
	}
	.hidden .say {
		color: var(--color-text-muted);
		text-decoration: line-through;
	}
	.hidden .n {
		color: var(--color-text-muted);
	}
	.detail {
		display: flex;
		flex-wrap: wrap;
		gap: 0 var(--space-3);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		overflow-wrap: anywhere;
	}
	.why {
		color: var(--color-text-warning);
	}
	code {
		font-family: var(--font-ui);
	}
	.state {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2) var(--space-5);
		margin: 0;
		padding-top: var(--space-3);
		border-top: var(--size-border) dashed var(--color-border-subtle);
	}
	.state div {
		display: flex;
		gap: var(--space-2);
	}
	dt {
		color: var(--color-text-muted);
	}
	dd {
		margin: 0;
		color: var(--color-text-strong);
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
