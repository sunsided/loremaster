<script lang="ts">
	import type { RollDetail } from '#lib/engine/types.ts';
	import Die from '../atoms/Die.svelte';
	import { DICE_FRAME_MS, DICE_FRAMES } from '../reveal.ts';

	interface Props {
		/** The dice of the newest turn. */
		rolls: RollDetail[];
		/** No animation: the final result shows at once. */
		reduceMotion?: boolean;
	}

	let { rolls, reduceMotion = true }: Props = $props();

	/** The frame of the roll animation, or null when the dice rest. */
	let frame = $state<number | null>(null);

	$effect(() => {
		if (reduceMotion || rolls.length === 0) {
			frame = null;
			return;
		}
		frame = 0;
		const timer = setInterval(() => {
			const next = (frame ?? 0) + 1;
			frame = next >= DICE_FRAMES ? null : next;
			if (frame === null) clearInterval(timer);
		}, DICE_FRAME_MS);
		return () => clearInterval(timer);
	});

	function sentence(r: RollDetail): string {
		if (r.target !== undefined) return `${r.total} against ${r.who ?? 'the target'} ${r.target}${r.result ? `: ${r.result}` : ''}`;
		return `${r.who ? `${r.who}: ` : ''}${r.dice} = ${r.total}`;
	}
</script>

<!--
	The dice of the newest turn. The text always shows the result, so the dice are not the only source.
	The element stays in the page without dice, so that a screen reader hears the first roll too.
-->
<div class="dice" role="status" aria-label="Dice">
	{#each rolls as roll, r (r)}
		<div class="roll">
			{#if roll.faces?.length}
				<div class="faces">
					{#each roll.faces as face, i (i)}
						<Die {face} spin={frame === null ? undefined : ((face + frame * (i + 2)) % 6) + 1} />
					{/each}
				</div>
			{/if}
			<p class="text">{sentence(roll)}</p>
		</div>
	{/each}
</div>

<style>
	.dice {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-5);
		font-family: var(--font-ui);
	}
	/* Without dice the live region stays in the page, but out of the flow: it adds no gap to the player column. */
	.dice:not(:has(.roll)) {
		position: absolute;
	}
	.roll {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}
	.faces {
		display: flex;
		gap: var(--space-2);
	}
	.text {
		margin: 0;
		font-size: var(--font-size-md);
		color: var(--color-text);
	}
</style>
