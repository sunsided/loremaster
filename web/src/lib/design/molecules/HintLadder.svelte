<script lang="ts">
	import type { Hint } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';

	interface Props {
		/** The steps of the current room that the player has not done and can do now, with the levels shown so far. */
		hints: Hint[];
		/** Show the next level of the ladder of this step. */
		onhint: (step: string) => void;
	}

	let { hints, onhint }: Props = $props();

	function next(h: Hint): string {
		if (h.shown.length === 0) return 'Show a hint';
		if (h.shown.length + 1 < h.levels) return 'Next hint';
		return 'Show the command';
	}
</script>

<!--
	The hint ladder of the current room (UI-1, CK-6): one ladder for each puzzle that the player can work on here
	now. A ladder goes one level at a time, from a nudge to the command. The ladders do not name their steps, so a
	hint never gives away more than its own level.
-->
{#if hints.length}
	<ol class="ladders">
		{#each hints as h, i (h.step)}
			<li class="ladder">
				<p class="head">
					<span class="name">Puzzle {i + 1}</span>
					<span class="meta">{h.command ? h.levels : h.shown.length} of {h.levels} levels</span>
				</p>
				{#if h.shown.length}
					<ol class="levels" aria-label="Hints for puzzle {i + 1}">
						{#each h.shown as text, n (n)}
							<li>{text}</li>
						{/each}
						{#if h.command}
							<li class="command">Type <kbd>{h.command}</kbd></li>
						{/if}
					</ol>
				{/if}
				{#if !h.command}
					<Button size="compact" variant="quiet" onclick={() => onhint(h.step)} aria-label="{next(h)} for puzzle {i + 1}">
						{next(h)}
					</Button>
				{/if}
			</li>
		{/each}
	</ol>
{:else}
	<p class="note">No hint for this place now. Look around elsewhere, or finish what you started.</p>
{/if}

<style>
	.ladders,
	.levels {
		margin: 0;
		padding: 0;
	}
	.ladders {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		list-style: none;
	}
	.ladder {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-2);
	}
	.head {
		display: flex;
		gap: var(--space-3);
		align-items: baseline;
		margin: 0;
	}
	.name {
		font-weight: var(--font-weight-strong);
	}
	.meta,
	.note {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.levels {
		padding-left: var(--space-5);
		line-height: var(--line-height-body);
	}
	.command kbd {
		font-family: var(--font-mono, monospace);
		color: var(--color-text-accent);
	}
	.note {
		margin: var(--space-3) 0 0;
		line-height: var(--line-height-body);
	}
</style>
