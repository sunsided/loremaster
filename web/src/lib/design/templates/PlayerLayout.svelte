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
		/** The status line, edge to edge on top of the player screen. */
		status?: Snippet;
		/** The screen texture layers. */
		effects?: Snippet;
		/** The side panel on the right. */
		panel?: Snippet;
		/** The map. It shows on the left on ultrawide screens only. */
		map?: Snippet;
	}

	let { skipTo = 'command', fxClass = 'fx-off', glow = 'none', header, player, status, effects, panel, map }: Props = $props();
</script>

<!--
	The page frame: a header, the player screen and the side panels.
	- Below 900px (--bp-stack): one column. The panel goes under the player. The page scrolls.
	- From 900px: the player and the panel are side by side.
	- From 2200px with 21:9 or wider (--bp-ultrawide): the map is on the left.
	- Wider than --width-content-max: the content stays at that width, in the center.
-->
<div class="layout {fxClass}" style:--glow={glow}>
	<a class="skip" href="#{skipTo}">Skip to the command input</a>
	<div class="frame">
		{@render header()}
		<div class="body" class:has-map={!!map}>
			{#if map}
				<div class="map">{@render map()}</div>
			{/if}
			<main class="screen fx-screen" aria-label="Player">
				{@render effects?.()}
				{#if status}<div class="status">{@render status()}</div>{/if}
				<div class="player">{@render player()}</div>
			</main>
			{#if panel}
				<div class="panel">{@render panel()}</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.layout {
		height: 100dvh;
		background: var(--color-surface-outer);
	}
	.frame {
		display: flex;
		flex-direction: column;
		height: 100%;
		max-width: var(--width-content-max);
		margin: 0 auto;
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
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(320px, var(--width-panel));
		/* One row, as high as the space under the header. Without this the row grows with the
		   tallest panel and the whole page scrolls. */
		grid-template-rows: minmax(0, 1fr);
		flex: 1 1 auto;
		min-height: 0;
	}
	.map {
		display: none;
	}
	.screen {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
		background: var(--color-surface);
		box-shadow: var(--shadow-screen);
	}
	.status {
		position: relative;
		z-index: var(--z-status);
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
		min-height: 0;
		padding: var(--space-7) clamp(var(--space-6), 4vw, var(--space-10)) var(--space-6);
	}
	.panel,
	.map {
		flex-direction: column;
		min-width: 0;
		min-height: 0;
	}
	.panel {
		display: flex;
		border-left: var(--size-border) solid var(--color-border-subtle);
	}
	.panel > :global(*),
	.map > :global(*) {
		flex: 1 1 auto;
	}

	/* Ultrawide (--bp-ultrawide): map | player | panel. */
	@media (min-width: 2200px) and (min-aspect-ratio: 21/9) {
		.body.has-map {
			grid-template-columns:
				minmax(360px, var(--width-map))
				minmax(0, var(--width-player-max))
				minmax(360px, var(--width-panel));
			justify-content: center;
		}
		.map {
			display: flex;
			border-right: var(--size-border) solid var(--color-border-subtle);
		}
	}

	/* Narrow (--bp-stack): one column. */
	@media (max-width: 900px) {
		.layout {
			height: auto;
			min-height: 100dvh;
		}
		.body {
			display: flex;
			flex-direction: column;
		}
		.screen {
			min-height: 85dvh;
		}
		/* On a phone the panel is a bottom sheet (fixed), so it takes no place in the page; the page keeps
		   room at the bottom for the line of the sheet. */
		.panel {
			border-left: 0;
		}
		.frame {
			padding-bottom: calc(var(--size-target) + var(--space-5));
		}
	}
</style>
