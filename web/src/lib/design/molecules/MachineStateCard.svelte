<script lang="ts">
	import type { MachineView } from '#lib/engine/types.ts';

	interface Props {
		machine: MachineView;
		state: string;
		/** Show the rule of a step in the rules view. */
		onrule?: (id: string) => void;
	}

	let { machine, state, onrule }: Props = $props();
	const id = $props.id();

	const description = $derived(machine.descriptions[state] ?? null);
	const steps = $derived(machine.steps.map((step, i) => ({ ...step, n: i + 1 })));
	const into = $derived(steps.filter((s) => s.to === state));
	const out = $derived(steps.filter((s) => s.from.includes(state)));
</script>

{#snippet stepList(label: string, list: typeof steps, word: string, other: (s: (typeof steps)[number]) => string)}
	<h3>{label}</h3>
	{#if list.length}
		<ul>
			{#each list as s (s.n)}
				<li>
					<span class="n">{s.n}</span>
					<span class="text">
						{word} <strong>{other(s)}</strong> when {s.when}
						{#if onrule}<button type="button" class="link" onclick={() => onrule(s.rule)}>rule</button>{/if}
					</span>
				</li>
			{/each}
		</ul>
	{:else}
		<p class="muted">none</p>
	{/if}
{/snippet}

<!-- One state of a puzzle machine in the Atlas: its description, and the steps into it and out of it. -->
<section class="state-card" aria-labelledby="{id}-title">
	<h2 id="{id}-title">{state}</h2>
	<p class="where">{machine.id}{state === machine.state ? ' · now' : ''}</p>
	{#if description}
		<p class="description">{description}</p>
	{:else}
		<p class="muted">No description. Write one in the list of states: <code>{'{ name: ' + state + ', description: … }'}</code>.</p>
	{/if}
	{@render stepList('steps in', into, 'from', (s) => s.from.join(' or '))}
	{@render stepList('steps out', out, 'to', (s) => s.to)}
</section>

<style>
	.state-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-5);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	h2 {
		margin: 0;
		font-size: var(--font-size-md);
		color: var(--color-text-accent);
	}
	h3 {
		margin: var(--space-2) 0 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		font-weight: var(--font-weight-strong);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
	}
	p {
		margin: 0;
		line-height: var(--line-height-body);
	}
	.where,
	.muted {
		color: var(--color-text-muted);
	}
	.description {
		color: var(--color-text-strong);
	}
	ul {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li {
		display: grid;
		grid-template-columns: 1.5em minmax(0, 1fr);
		gap: var(--space-3);
	}
	.n {
		color: var(--color-text-muted);
	}
	.text {
		overflow-wrap: anywhere;
	}
	strong {
		color: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
	}
	.link {
		margin-left: var(--space-2);
		padding: 0;
		border: 0;
		background: none;
		color: var(--color-text-accent);
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	.link:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
</style>
