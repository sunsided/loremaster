<script lang="ts">
	import type { RuleView } from '#lib/engine/types.ts';
	import Heading from '../atoms/Heading.svelte';
	import RuleCard from '../molecules/RuleCard.svelte';

	interface Props {
		rules: RuleView[];
	}

	let { rules }: Props = $props();
	const fired = $derived(rules.filter((r) => r.fired).length);
</script>

<!-- All rules of the story as When / If / Then cards, with their state in the current game. -->
<section class="rules-view" aria-labelledby="rules-title">
	<Heading level={2} id="rules-title">Rules</Heading>
	<p class="summary">{rules.length} rules. {fired} fired in this game.</p>
	<ul>
		{#each rules as rule (rule.id)}<li><RuleCard {rule} /></li>{/each}
	</ul>
</section>

<style>
	.rules-view {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	.summary {
		margin: 0;
		color: var(--color-text-muted);
	}
	ul {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 360px), 1fr));
		gap: var(--space-6);
		margin: 0;
		padding: 0;
		list-style: none;
	}
</style>
