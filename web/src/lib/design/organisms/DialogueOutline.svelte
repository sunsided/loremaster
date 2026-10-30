<script lang="ts">
	import type { DialogueOutline } from '#lib/engine/types.ts';

	interface Props {
		/** The dialogue, or null when the thing has none. */
		outline: DialogueOutline | null;
		/** The current name of the thing, for example "Blackbeard". */
		name: string;
		/** The file of the dialogue, for example "dialogues/blackbeard.yaml". */
		file?: string | null;
		/** The node of the talk in progress. It is marked. */
		current?: string | null;
	}

	let { outline, name, file = null, current = null }: Props = $props();

	const uid = $props.id();
	const nodeId = (key: string) => `${uid}-node-${key}`;
	const problems = $derived(
		outline
			? outline.nodes.reduce((n, node) => n + (node.unreachable ? 1 : 0) + node.options.filter((o) => o.missing).length, 0)
			: 0
	);

	/** Move the keyboard focus to a node, for the links from the options. */
	function goTo(key: string) {
		document.getElementById(nodeId(key))?.focus();
	}

	/** `*word*` is emphasis. */
	const pieces = (text: string) => text.split('*').map((t, i) => ({ text: t, em: i % 2 === 1 }));
</script>

<!--
	The outline of a dialogue: its nodes in file order, each with the other character's line and the player's answers.
	Each answer shows where it goes. A link to a node moves the keyboard focus there.
	Problems show as warnings: an answer that goes to a node that does not exist, and a node that no start node leads to.
-->
<div class="outline" role="group" aria-label="Dialogue of {name}">
	{#if !outline}
		<p class="empty">
			{name} has no dialogue. Add <code>dialogues/&lt;id&gt;.yaml</code> with <code>with:</code> and the thing id. Until then,
			“Talk to” gives the default response.
		</p>
	{:else}
		<p class="head">
			<span class="file">{file ?? `dialogues/${outline.id}.yaml`}</span>
			<span class="meta"
				>{outline.nodes.length} nodes · starts at {outline.start.join(', then ')}{#if problems}<span class="warn"
						>{` · ▲ ${problems} ${problems === 1 ? 'problem' : 'problems'}`}</span
					>{/if}</span
			>
		</p>
		<ol class="nodes">
			{#each outline.nodes as node (node.key)}
				<li class="node" class:current={node.key === current} class:unreachable={node.unreachable}>
					<h3 class="node-head" id={nodeId(node.key)} tabindex="-1">
						<span class="key">{node.key}</span>
						{#if node.start !== undefined}<span class="tag">start {node.start + 1}</span>{/if}
						{#if node.key === current}<span class="tag now">now</span>{/if}
					</h3>
					{#if node.when}<p class="when"><span class="label">when</span> <code>{node.when}</code></p>{/if}
					<p class="say">{#each pieces(node.say) as p, j (j)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}</p>
					{#if node.outcomes.length}
						<ul class="outcomes" aria-label="Changes on entering {node.key}">
							{#each node.outcomes as o, i (i)}<li>{o}</li>{/each}
						</ul>
					{/if}
					{#if node.unreachable}<p class="problem" role="note">▲ No start node leads here.</p>{/if}
					{#if node.options.length}
						<ol class="options" aria-label="Answers in {node.key}">
							{#each node.options as o, i (i)}
								<li class="option">
									<p class="answer">
										<span class="n" aria-hidden="true">{i + 1}</span><span>{#each pieces(o.say) as p, j (j)}{#if p.em}<em>{p.text}</em>{:else}{p.text}{/if}{/each}</span>
									</p>
									{#if o.when}<p class="when"><span class="label">when</span> <code>{o.when}</code></p>{/if}
									<p class="go">
										{#if o.missing}
											<span class="problem">▲ goes to <code>{o.go}</code>, but there is no node “{o.go}”</span>
										{:else if o.go === 'end'}
											<span class="muted">→ ends the talk</span>
										{:else if o.go === 'back'}
											<span class="muted">→ back to the node before</span>
										{:else}
											<button type="button" class="link" onclick={() => goTo(o.go)}>→ {o.go}</button>
										{/if}
										{#each o.outcomes as out, k (k)}<span class="chip">{out}</span>{/each}
									</p>
								</li>
							{/each}
						</ol>
					{:else}
						<p class="muted">No answers: the talk ends after this line.</p>
					{/if}
				</li>
			{/each}
		</ol>
	{/if}
</div>

<style>
	.outline {
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-body);
		min-width: 0;
	}
	.empty,
	.head {
		margin: 0 0 var(--space-3);
		color: var(--color-text-muted);
	}
	.head {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2) var(--space-4);
	}
	.file {
		color: var(--color-text);
		overflow-wrap: anywhere;
	}
	.warn,
	.problem {
		color: var(--color-text-warning);
	}
	.nodes,
	.options,
	.outcomes {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.node {
		margin: 0 0 var(--space-4);
		padding: var(--space-3) 0 var(--space-3) var(--space-4);
		border-left: var(--size-border) solid var(--color-border-subtle);
	}
	.node.current {
		border-left: var(--size-border-strong) solid var(--color-accent);
	}
	.node.unreachable {
		border-left-style: dashed;
		border-left-color: var(--color-warning);
	}
	.node-head {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-3);
		margin: 0;
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-strong);
		color: var(--color-text-strong);
	}
	.node-head:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.tag,
	.chip {
		padding: 0 var(--space-2);
		border: var(--size-border) solid var(--color-border-control);
		font-size: var(--font-size-xs);
		font-weight: var(--font-weight-regular);
		color: var(--color-text-muted);
		white-space: nowrap;
	}
	.tag.now {
		border-color: var(--color-accent);
		color: var(--color-text-accent);
	}
	.say {
		margin: var(--space-2) 0;
		font-family: var(--font-prose);
		color: var(--color-text);
		overflow-wrap: break-word;
	}
	.when {
		margin: var(--space-1) 0;
		color: var(--color-text-muted);
		overflow-wrap: anywhere;
	}
	.label {
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
		font-size: var(--font-size-xs);
	}
	.outcomes {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-bottom: var(--space-2);
	}
	.outcomes li {
		padding: 0 var(--space-2);
		border: var(--size-border) dashed var(--color-border-control);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.option {
		margin-top: var(--space-3);
	}
	.answer {
		display: flex;
		gap: var(--space-3);
		margin: 0;
		color: var(--color-text-strong);
		overflow-wrap: break-word;
	}
	.n {
		flex: none;
		min-width: 1.5em;
		color: var(--color-text-accent);
	}
	.option .when,
	.go {
		margin-left: calc(1.5em + var(--space-3));
	}
	.go {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		margin-top: var(--space-1);
		margin-bottom: 0;
	}
	.muted {
		margin: var(--space-2) 0 0;
		color: var(--color-text-muted);
	}
	.link {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-2);
		margin-left: calc(-1 * var(--space-2));
		border: 0;
		background: none;
		color: var(--color-text-accent);
		font: inherit;
		text-decoration: underline;
		text-underline-offset: 3px;
	}
	.link:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 1px;
	}
	code {
		font-family: var(--font-ui);
		color: var(--color-text);
	}
</style>
