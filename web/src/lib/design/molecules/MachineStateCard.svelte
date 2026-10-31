<script lang="ts">
	import type { MachineView } from '#lib/engine/types.ts';
	import type { StateEdit } from '#lib/project/yaml-edit.ts';
	import Button from '../atoms/Button.svelte';
	import TextInput from '../atoms/TextInput.svelte';

	interface Props {
		machine: MachineView;
		/** The state to show. */
		stateName: string;
		/** Show the rule of a step in the rules view. */
		onrule?: (id: string) => void;
		/** Change the state in the story files. Resolves to a problem in words, or null. Without it, the card only shows. */
		onedit?: (edit: StateEdit) => Promise<string | null>;
	}

	let { machine, stateName, onrule, onedit }: Props = $props();
	const id = $props.id();

	// The fields follow the shown state; typing overrides them until the state changes.
	let draft = $derived(machine.descriptions[stateName] ?? '');
	let newName = $derived(stateName);
	/** The problem of the last change, for the state it was about. */
	let problem = $state<{ of: string; text: string } | null>(null);
	const inUse = $derived(machine.start === stateName || machine.steps.some((s) => s.from.includes(stateName) || s.to === stateName));

	async function run(edit: StateEdit) {
		const text = (await onedit?.(edit)) ?? null;
		problem = text ? { of: edit.name, text } : null;
	}

	const description = $derived(machine.descriptions[stateName] ?? null);
	const steps = $derived(machine.steps.map((step, i) => ({ ...step, n: i + 1 })));
	const into = $derived(steps.filter((s) => s.to === stateName));
	const out = $derived(steps.filter((s) => s.from.includes(stateName)));
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
	<h2 id="{id}-title">{stateName}</h2>
	<p class="where">{machine.id}{stateName === machine.state ? ' · now' : ''}</p>
	{#if onedit}
		<form
			class="edit"
			onsubmit={(e) => {
				e.preventDefault();
				run({ kind: 'describe', name: stateName, description: draft });
			}}
		>
			<label for="{id}-description">Description</label>
			<textarea id="{id}-description" rows="4" bind:value={draft} placeholder="What is true in this state, for the authors"></textarea>
			<div class="actions">
				<Button size="compact" type="submit" disabled={draft.trim() === (description ?? '')}>Save description</Button>
			</div>
		</form>
		<form
			class="edit"
			onsubmit={(e) => {
				e.preventDefault();
				run({ kind: 'rename', name: stateName, to: newName.trim() });
			}}
		>
			<TextInput id="{id}-name" label="Name" bind:value={newName} autocomplete="off" spellcheck="false" />
			<div class="actions">
				<Button size="compact" type="submit" disabled={!newName.trim() || newName.trim() === stateName}>Rename</Button>
				<Button
					size="compact"
					variant="quiet"
					disabled={inUse}
					title={inUse ? 'The start or a step uses this state' : undefined}
					onclick={() => run({ kind: 'delete', name: stateName })}>Delete state</Button
				>
			</div>
			{#if inUse}<p class="muted">The start or a step uses this state, so it can not go.</p>{/if}
		</form>
		<p class="muted">A rename changes the start and the steps of the machine. Conditions elsewhere, such as <code>{machine.id} == {stateName}</code>, keep the old name; the checks show them.</p>
		{#if problem?.of === stateName}<p class="problem" role="alert">{problem.text}</p>{/if}
	{:else if description}
		<p class="description">{description}</p>
	{:else}
		<p class="muted">No description. Write one in the list of states: <code>{'{ name: ' + stateName + ', description: … }'}</code>.</p>
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
	.edit {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	label {
		color: var(--color-text-muted);
	}
	textarea {
		box-sizing: border-box;
		width: 100%;
		padding: var(--space-3);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-control);
		color: var(--color-text);
		font: inherit;
		line-height: var(--line-height-body);
		resize: vertical;
	}
	textarea:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}
	.problem {
		color: var(--color-text-warning);
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
