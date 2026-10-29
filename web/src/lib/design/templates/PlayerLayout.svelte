<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** The id of the element that the skip link moves focus to. */
		skipTo?: string;
		/** Classes for the effect state, for example "fx-full fx-reduce". */
		fxClass?: string;
		/** The text-shadow for the glow effect, as a token reference. */
		glow?: string;
		header: Snippet;
		player: Snippet;
		/** The screen texture layers. */
		effects?: Snippet;
		panel?: Snippet;
	}

	let { skipTo = 'command', fxClass = 'fx-off', glow = 'none', header, player, effects, panel }: Props = $props();
</script>

<!--
	The page frame: a header, the player screen and the side panel.
	Wide screens show the player and the panel side by side.
	Below 900px (--bp-stack) the panel goes under the player and the page scrolls.
-->
<div class="layout {fxClass}" style:--glow={glow}>
	<a class="skip" href="#{skipTo}">Skip to the command input</a>
	{@render header()}
	<div class="body">
		<main class="screen fx-screen" aria-label="Player">
			{@render effects?.()}
			<div class="player">{@render player()}</div>
		</main>
		{#if panel}
			<div class="panel">{@render panel()}</div>
		{/if}
	</div>
</div>

<style>
	.layout {
		display: flex;
		flex-direction: column;
		height: 100dvh;
		background: var(--color-surface);
	}
	.skip {
		position: absolute;
		left: var(--space-4);
		top: -100px;
		z-index: var(--z-dialog);
		padding: var(--space-4) var(--space-5);
		background: var(--color-accent);
		color: var(--color-text-on-accent);
	}
	.skip:focus {
		top: var(--space-4);
	}
	.body {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
	}
	.screen {
		position: relative;
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
		background: var(--color-surface);
		box-shadow: var(--shadow-screen);
	}
	.player {
		position: relative;
		z-index: var(--z-content);
		display: flex;
		flex: 1 1 auto;
		flex-direction: column;
		gap: var(--space-5);
		box-sizing: border-box;
		min-width: 0;
		padding: var(--space-8) clamp(var(--space-6), 4vw, var(--space-10));
	}
	.panel {
		display: flex;
		flex: 0 1 var(--width-panel);
		flex-direction: column;
		min-width: 320px;
		min-height: 0;
		border-left: var(--size-border) solid var(--color-border-subtle);
	}
	.panel > :global(*) {
		flex: 1 1 auto;
	}

	@media (max-width: 900px) {
		.layout {
			height: auto;
			min-height: 100dvh;
		}
		.body {
			flex-direction: column;
		}
		.screen {
			min-height: 85dvh;
		}
		.panel {
			flex: none;
			min-width: 0;
			border-left: 0;
			border-top: var(--size-border) solid var(--color-border-subtle);
		}
	}
</style>
