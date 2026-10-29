<script lang="ts">
	import { tick, untrack } from 'svelte';
	import type { RuleView } from '#lib/engine/types.ts';
	import Heading from '../atoms/Heading.svelte';
	import TextInput from '../atoms/TextInput.svelte';
	import RadioGroup from '../molecules/RadioGroup.svelte';
	import RuleCard from '../molecules/RuleCard.svelte';
	import type { RadioOption } from '../types.ts';

	type Scope = 'here' | 'fired' | 'all';

	interface Props {
		rules: RuleView[];
		/** A rule id to show: the view selects "All", opens the rule and moves focus to it. */
		reveal?: string | null;
		/** Called after the view showed the `reveal` rule. */
		onrevealed?: () => void;
	}

	let { rules, reveal = null, onrevealed }: Props = $props();

	let scope = $state<Scope>('here');
	let query = $state('');
	let open = $state<string[]>([]);

	const triggers: { id: RuleView['trigger'][]; label: string }[] = [
		{ id: ['enter', 'leave'], label: 'Rooms' },
		{ id: ['verb'], label: 'Actions' },
		{ id: ['time'], label: 'Time' },
		{ id: ['turn'], label: 'Every turn' },
		{ id: ['start'], label: 'Start' }
	];

	const here = $derived(rules.filter((r) => r.relevant));
	const fired = $derived(rules.filter((r) => r.fired));
	const scopes = $derived<RadioOption<Scope>[]>([
		{ value: 'here', label: `Here and now (${here.length})` },
		{ value: 'fired', label: `Fired (${fired.length})` },
		{ value: 'all', label: `All (${rules.length})` }
	]);

	const shown = $derived.by(() => {
		const base = scope === 'here' ? here : scope === 'fired' ? fired : rules;
		const words = query.toLowerCase().split(/\s+/).filter(Boolean);
		if (!words.length) return base;
		return base.filter((r) => {
			const text = `${r.id} ${r.when} ${r.condition ?? ''} ${r.then.join(' ')}`.toLowerCase();
			return words.every((w) => text.includes(w));
		});
	});

	const groups = $derived(
		triggers
			.map((t) => ({ label: t.label, rules: shown.filter((r) => t.id.includes(r.trigger)) }))
			.filter((g) => g.rules.length)
	);

	function toggle(id: string, isOpen: boolean) {
		open = isOpen ? [...new Set([...open, id])] : open.filter((x) => x !== id);
	}

	$effect(() => {
		const id = reveal;
		if (!id) return;
		// Only `reveal` starts this effect. The state it changes must not start it again.
		untrack(() => {
			scope = 'all';
			query = '';
			toggle(id, true);
		});
		tick().then(() => {
			const row = document.getElementById(`rule-row-${id}`);
			row?.scrollIntoView({ block: 'center' });
			row?.focus();
			onrevealed?.();
		});
	});

	function status(rule: RuleView): string {
		if (scope === 'here' && rule.relevant) return rule.relevant;
		return rule.fired ? 'fired' : 'not fired yet';
	}
</script>

<!--
	The rules of the story, made for big games: by default it shows only the rules that matter
	at the player's position and time. Each rule is one line; open it to see the full card.
-->
<section class="rules-view" aria-labelledby="rules-title">
	<Heading level={2} id="rules-title">Rules</Heading>

	<div class="controls">
		<RadioGroup legend="Show" name="rule-scope" look="pills" options={scopes} bind:value={scope} />
		<div class="filter">
			<TextInput id="rule-filter" label="Filter" bind:value={query} placeholder="words in id, event, condition or effects" />
		</div>
	</div>

	{#each groups as g (g.label)}
		<details class="group" open>
			<summary><span class="group-name">{g.label}</span> <span class="count">{g.rules.length}</span></summary>
			<ul>
				{#each g.rules as rule (rule.id)}
					<li>
						<details
							class="rule"
							open={open.includes(rule.id)}
							ontoggle={(e) => toggle(rule.id, (e.currentTarget as HTMLDetailsElement).open)}
						>
							<summary id="rule-row-{rule.id}">
								<code class="id">{rule.id}</code>
								<span class="when">{rule.when}</span>
								<span class="status" class:fired={rule.fired}>{status(rule)}</span>
							</summary>
							<div class="card"><RuleCard {rule} /></div>
						</details>
					</li>
				{/each}
			</ul>
		</details>
	{:else}
		<p class="empty">
			{#if query}No rule matches “{query}”.
			{:else if scope === 'here'}No rule is about this place or this time. Select “All” to see every rule.
			{:else if scope === 'fired'}No rule fired yet.
			{:else}This story has no rules.{/if}
		</p>
	{/each}
</section>

<style>
	.rules-view {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	.controls {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.filter {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-4);
	}
	.group > summary {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		min-height: var(--size-target-compact);
		cursor: pointer;
		font-weight: var(--font-weight-strong);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
	}
	/* Display flex and grid hide the native marker, so draw one. It turns when the item opens. */
	summary {
		list-style: none;
	}
	summary::-webkit-details-marker {
		display: none;
	}
	.group > summary::before,
	.rule > summary::before {
		content: '▸';
		color: var(--color-text-muted);
		transition: transform var(--duration-fast);
	}
	.group[open] > summary::before,
	.rule[open] > summary::before {
		transform: rotate(90deg);
	}
	.count {
		padding: 0 var(--space-3);
		border: var(--size-border) solid var(--color-border-subtle);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	ul {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: var(--space-3) 0 0;
		padding: 0;
		list-style: none;
	}
	.rule > summary {
		display: grid;
		grid-template-columns: auto auto minmax(0, 1fr);
		gap: var(--space-1) var(--space-4);
		align-items: baseline;
		padding: var(--space-3) var(--space-4);
		border: var(--size-border) solid var(--color-border-subtle);
		cursor: pointer;
	}
	.rule[open] > summary {
		border-color: var(--color-border-control);
	}
	.rule > summary:hover {
		border-color: var(--color-accent);
	}
	.id {
		color: var(--color-text-accent);
	}
	.when {
		color: var(--color-text);
		overflow-wrap: anywhere;
	}
	.status {
		grid-column: 3;
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.status.fired {
		color: var(--color-text-accent);
	}
	.card {
		margin-top: var(--space-2);
	}
	.empty {
		margin: 0;
		color: var(--color-text-muted);
	}
</style>
