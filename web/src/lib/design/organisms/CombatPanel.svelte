<script lang="ts">
	import type { CombatView } from '#lib/engine/types.ts';
	import Heading from '../atoms/Heading.svelte';

	interface Props {
		/** The combat in progress. */
		combat: CombatView;
	}

	let { combat }: Props = $props();
	const id = $props.id();
	const strength = (n?: number) => n ?? 'not rolled yet';
	const newest = $derived([...combat.log].reverse());
</script>

<!--
	The combat in progress. The transcript already reads the rounds to a screen reader, so the log here is no live
	region. The round choices are in the await prompt.
-->
<section class="combat" aria-labelledby="{id}-title">
	<Heading level={2} id="{id}-title">Combat{combat.round > 0 ? `, round ${combat.round}` : ''}</Heading>
	<ul class="enemies">
		{#each combat.enemies as enemy (enemy.id)}
			<li class:beaten={enemy.stamina <= 0}>
				<span class="name">{enemy.name}{enemy.stamina <= 0 ? ' (beaten)' : ''}</span>
				<span>SKILL {enemy.skill}</span>
				<span>STAMINA {enemy.stamina}</span>
				<span>Attack strength {strength(enemy.strength)}</span>
			</li>
		{/each}
	</ul>
	<p class="you">Your attack strength: {strength(combat.player_strength)}</p>
	{#if newest.length}
		<!-- The log scrolls, so it must be focusable: keyboard users scroll it with the arrow keys. -->
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<ol class="log" aria-label="Combat log, newest first" tabindex="0">
			{#each newest as entry, i (i)}<li>{entry}</li>{/each}
		</ol>
	{/if}
</section>

<style>
	.combat {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding-top: var(--space-4);
		border-top: var(--size-border) solid var(--color-border-subtle);
		font-family: var(--font-ui);
		font-size: var(--font-size-md);
		color: var(--color-text);
	}
	.enemies,
	.log {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.enemies li {
		display: flex;
		flex-wrap: wrap;
		gap: 0 var(--space-5);
	}
	.name {
		font-weight: var(--font-weight-strong);
		color: var(--color-text-strong);
	}
	.beaten .name {
		text-decoration: line-through;
	}
	.you {
		margin: 0;
	}
	.log {
		max-height: calc(var(--size-target) * 3);
		overflow-y: auto;
		color: var(--color-text-muted);
	}
	.log:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
	}
</style>
