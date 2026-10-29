<script lang="ts">
	import type { Session } from './session.svelte.ts';

	let { session }: { session: Session } = $props();
</script>

<div class="bar">
	<div class="verbs" role="group" aria-label="Verbs">
		{#each session.barVerbs as verb (verb.id)}
			<button
				class="verb"
				aria-pressed={session.pending?.verb.id === verb.id}
				onclick={() => session.pickVerb(verb)}>{verb.label}</button
			>
		{/each}
	</div>

	<div class="reach">
		{#if session.pending?.verb.id === 'go'}
			<span class="label">to</span>
			{#each session.scene.exits as exit (exit.direction)}
				<button class="chip" onclick={() => session.pickExit(exit.direction)}
					>{exit.to_name} <span class="dim">({exit.direction})</span></button
				>
			{:else}
				<span class="dim">nowhere</span>
			{/each}
		{:else}
			<span class="label">here</span>
			{#each session.scene.things as t (t.id)}
				<button class="chip" onclick={() => session.pickThing(t.id, t.name)}>{t.name}</button>
			{:else}
				<span class="dim">nothing in reach</span>
			{/each}
			<span class="label">carrying</span>
			{#each session.scene.inventory as t (t.id)}
				<button class="chip" onclick={() => session.pickThing(t.id, t.name)}>{t.name}</button>
			{:else}
				<span class="dim">nothing</span>
			{/each}
		{/if}
	</div>

	<div class="sentence" aria-live="polite">
		{#if session.pending}
			<span>{session.sentence}</span>
			{#if session.pending.verb.slots === 'two' && session.pending.objects.length === 1}
				<button class="chip" onclick={() => session.runPending()}
					>just {session.pending.verb.label.toLowerCase()} it</button
				>
			{/if}
			<button class="chip" onclick={() => (session.pending = null)}>cancel</button>
		{:else}
			<span class="dim">Pick a verb, then click something. Clicking a thing on its own looks at it.</span>
		{/if}
	</div>
</div>

<style>
	.bar {
		border-top: 1px solid var(--line);
		padding-top: 14px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.verbs {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
		gap: 6px;
		max-width: 820px;
	}
	.verb {
		border: 1px solid var(--chip-line);
		background: var(--chip);
		color: var(--text);
		padding: 10px 8px;
		min-height: 44px;
		text-align: left;
	}
	.verb[aria-pressed='true'] {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--on-accent);
	}
	.reach,
	.sentence {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		align-items: center;
	}
	.sentence {
		color: var(--warn-text);
		min-height: 32px;
	}
	.label {
		color: var(--muted);
		margin-right: 2px;
	}
	.label:not(:first-child) {
		margin-left: 10px;
	}
	.chip {
		border: 1px solid var(--chip-line);
		background: transparent;
		color: var(--text);
		padding: 6px 10px;
	}
	.chip:hover {
		border-color: var(--accent);
	}
	.dim {
		color: var(--muted);
	}
</style>
