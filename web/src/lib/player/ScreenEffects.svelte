<script lang="ts">
	import { settings } from '#lib/settings.svelte.ts';

	const on = $derived(settings.effects !== 'off');
	const scanAlpha = $derived(settings.effects === 'full' ? 'var(--scan-full)' : 'var(--scan-soft)');
</script>

<!-- Texture layers over the player. They never take pointer events. -->
{#if on}
	<svg class="fx-overlay fx-grain-layer grain" aria-hidden="true">
		<filter id="fx-noise">
			<feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" stitchTiles="stitch" />
			<feColorMatrix type="saturate" values="0" />
		</filter>
		<rect width="100%" height="100%" filter="url(#fx-noise)" />
	</svg>
	{#if settings.theme === 'paper'}
		<svg class="fx-overlay paper" aria-hidden="true">
			<filter id="fx-paper">
				<feTurbulence type="fractalNoise" baseFrequency="0.035 0.55" numOctaves="4" result="fibre" />
				<feDiffuseLighting in="fibre" lighting-color="#ffffff" surfaceScale="1.6">
					<feDistantLight azimuth="45" elevation="58" />
				</feDiffuseLighting>
			</filter>
			<rect width="100%" height="100%" filter="url(#fx-paper)" />
		</svg>
	{/if}
	{#if settings.theme === 'phosphor'}
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
		z-index: 2;
		opacity: var(--grain-opacity);
		mix-blend-mode: var(--grain-blend);
	}
	.paper {
		inset: 0;
		width: 100%;
		height: 100%;
		z-index: 2;
		opacity: 0.38;
		mix-blend-mode: multiply;
	}
	.scan {
		inset: 0;
		z-index: 3;
		background-image: repeating-linear-gradient(
			to bottom,
			rgb(0 0 0 / calc(var(--alpha) * 100%)) 0 1px,
			transparent 1px 3px
		);
	}
	.vignette {
		inset: 0;
		z-index: 4;
		background: radial-gradient(ellipse at center, transparent 58%, rgba(0, 0, 0, 0.42) 100%);
	}
</style>
