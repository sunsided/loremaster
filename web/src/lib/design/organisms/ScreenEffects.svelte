<script lang="ts">
	import type { EffectsLevel, Theme } from '../types.ts';

	interface Props {
		theme: Theme;
		effects: EffectsLevel;
	}

	let { theme, effects }: Props = $props();
	const on = $derived(effects !== 'off');
	const scanAlpha = $derived(effects === 'full' ? 'var(--fx-scan-full)' : 'var(--fx-scan-soft)');
</script>

<!--
	Texture layers over the player screen. They are decoration only: they are
	hidden from screen readers and do not get pointer events.
	Place this component as the first child of an element with position: relative.
-->
{#if on}
	<svg class="fx-overlay fx-grain-layer grain" aria-hidden="true">
		<filter id="fx-noise">
			<feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" stitchTiles="stitch" />
			<feColorMatrix type="saturate" values="0" />
		</filter>
		<rect width="100%" height="100%" filter="url(#fx-noise)" />
	</svg>
	{#if theme === 'paper'}
		<svg class="fx-overlay paper" aria-hidden="true">
			<filter id="fx-paper">
				<feTurbulence type="fractalNoise" baseFrequency="0.02 0.3" numOctaves="3" result="fibre" />
				<feDiffuseLighting in="fibre" lighting-color="#ffffff" surfaceScale="1.6">
					<feDistantLight azimuth="45" elevation="58" />
				</feDiffuseLighting>
			</filter>
			<rect width="100%" height="100%" filter="url(#fx-paper)" />
		</svg>
	{/if}
	{#if theme === 'phosphor'}
		<div class="fx-overlay scan" aria-hidden="true" style:--alpha={scanAlpha}></div>
		<div class="fx-overlay vignette" aria-hidden="true"></div>
	{/if}
{/if}

<style>
	.fx-overlay {
		position: absolute;
		pointer-events: none;
	}
	.grain {
		left: -10%;
		top: -10%;
		width: 120%;
		height: 120%;
		z-index: var(--z-texture);
		opacity: var(--fx-grain-opacity);
		mix-blend-mode: var(--fx-grain-blend);
	}
	.paper {
		inset: 0;
		width: 100%;
		height: 100%;
		z-index: var(--z-texture);
		opacity: var(--fx-paper-opacity, 0.1);
		mix-blend-mode: multiply;
	}
	.scan {
		inset: 0;
		z-index: var(--z-scanlines);
		background-image: repeating-linear-gradient(
			to bottom,
			rgb(0 0 0 / var(--alpha)) 0 1px,
			transparent 1px 3px
		);
	}
	.vignette {
		inset: 0;
		z-index: var(--z-vignette);
		background: radial-gradient(ellipse at center, transparent 58%, rgb(0 0 0 / 42%) 100%);
	}
</style>
