<script lang="ts">
	import type { Awaiting, Value } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';

	interface Props {
		/** What the story waits for: the `awaiting` of an `await` status. */
		awaiting: Awaiting;
		/** Answer the request. The value is one of the answers that the request names. */
		onanswer: (value: Value) => void;
		/** The death prompt can offer to restore a save. Without a place for saves, that answer is hidden. */
		canRestore?: boolean;
	}

	let { awaiting, onanswer, canRestore = false }: Props = $props();

	const request = $derived(awaiting.request);
	const deathLabels: Record<string, string> = { undo: 'Undo the last turn', restart: 'Start again', restore: 'Restore a save' };

	/**
	 * The typed roll, for a roll that the host answers. `bind:value` on a number input gives a number, or
	 * undefined while the field is empty or invalid.
	 */
	let roll = $state<number | undefined>();
	const rollOk = $derived(request.kind === 'roll' && typeof roll === 'number' && Number.isInteger(roll) && roll >= request.min && roll <= request.max);

	/** "Your roll" for the player's own combat roll, "Roll for GATE WARDEN" for another, or just "Roll". */
	const rollHead = $derived(request.kind !== 'roll' || !request.who ? 'Roll' : request.who === 'You' ? 'Your roll' : `Roll for ${request.who}`);

	function submitRoll(event: SubmitEvent) {
		event.preventDefault();
		if (rollOk && roll !== undefined) onanswer(roll);
	}

	function randomRoll() {
		if (request.kind !== 'roll') return;
		onanswer(request.min + Math.floor(Math.random() * (request.max - request.min + 1)));
	}
</script>

<!--
	The story waits for an answer: a rule paused at an `await` effect, or the death prompt. The prompt names what
	the story asks and offers the answers as buttons. The game goes on when the player answers.
-->
<section class="await" aria-label="The story waits for an answer">
	{#if request.kind === 'choice'}
		<p class="head">Choose one:</p>
		<ol class="answers">
			{#each request.options as option (option.id)}
				<li><Button size="compact" onclick={() => onanswer(option.id)}>{option.text}</Button></li>
			{/each}
		</ol>
	{:else if request.kind === 'action'}
		<p class="head">The story waits for the action <code>{request.name}</code>.</p>
		<ol class="answers">
			{#each request.results?.length ? request.results : [] as result (result)}
				<li><Button size="compact" onclick={() => onanswer(result)}>{result}</Button></li>
			{:else}
				<li><Button size="compact" onclick={() => onanswer(true)}>Done</Button></li>
			{/each}
		</ol>
	{:else if request.kind === 'roll'}
		<form class="roll" onsubmit={submitRoll}>
			<label>
				<span class="head">{rollHead} {request.dice} ({request.min} to {request.max}):</span>
				<input type="number" min={request.min} max={request.max} step="1" bind:value={roll} />
			</label>
			<Button size="compact" type="submit" disabled={!rollOk}>Answer</Button>
			<Button size="compact" variant="quiet" onclick={randomRoll}>Roll for me</Button>
		</form>
	{:else if request.kind === 'walk_to'}
		<p class="head">The story waits for the player to walk to <code>{request.room}</code>.</p>
		<ol class="answers">
			<li><Button size="compact" onclick={() => onanswer('arrived')}>Arrived</Button></li>
			<li><Button size="compact" variant="quiet" onclick={() => onanswer('refused')}>Refused</Button></li>
		</ol>
	{:else if request.kind === 'death_prompt'}
		<p class="head">You have died.</p>
		<ol class="answers">
			{#each request.options.filter((o) => canRestore || o !== 'restore') as option (option)}
				<li><Button size="compact" onclick={() => onanswer(option)}>{deathLabels[option] ?? option}</Button></li>
			{/each}
		</ol>
	{/if}
</section>

<style>
	.await {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-3);
		padding-top: var(--space-5);
		border-top: var(--size-border) solid var(--color-border-subtle);
		font-family: var(--font-ui);
	}
	.head {
		margin: 0;
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.answers {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.roll {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
	}
	.roll label {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.roll input {
		width: 5rem;
		min-height: var(--size-target-compact);
		font: inherit;
	}
</style>
