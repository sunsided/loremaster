<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Image {
		/** Null when the player has no file for it: the caption shows instead. */
		src: string | null;
		/** The alt text. Empty for a decorative image. */
		alt: string;
	}

	/** A clickable part of the illustration. */
	interface Hotspot {
		/** The text of the button: what a click does. */
		label: string;
		/** Left, top, width and height, in percent of the image. */
		rect: [number, number, number, number];
		/** For the author tooltip and the verb hint, for example `thing:iron_gate` or `exit:north`. */
		inspect?: string;
	}

	interface Props {
		illustration?: Image | null;
		portrait?: Image | null;
		/** The hotspots on the illustration, in reading order. */
		hotspots?: Hotspot[];
		onhotspot?: (index: number) => void;
		/** Shown beside the images: the sound controls and captions. */
		children?: Snippet;
	}

	let { illustration = null, portrait = null, hotspots = [], onhotspot, children }: Props = $props();
</script>

{#snippet image(item: Image, kind: string)}
	<figure class={kind}>
		{#if item.src && kind === 'illustration' && hotspots.length}
			<div class="scene" role="group" aria-label="In the picture">
				<img src={item.src} alt={item.alt} />
				{#each hotspots as spot, i (i)}
					<button
						type="button"
						class="hotspot"
						style:left="{spot.rect[0]}%"
						style:top="{spot.rect[1]}%"
						style:width="{spot.rect[2]}%"
						style:height="{spot.rect[3]}%"
						data-inspect={spot.inspect}
						onclick={() => onhotspot?.(i)}><span class="name">{spot.label}</span></button
					>
				{/each}
			</div>
		{:else if item.src}
			<img src={item.src} alt={item.alt} />
		{:else if item.alt}
			<p class="text">{item.alt}</p>
		{/if}
	</figure>
{/snippet}

{#if illustration || portrait || children}
	<div class="media">
		{#if illustration || portrait}
			<div class="images">
				{#if illustration}{@render image(illustration, 'illustration')}{/if}
				{#if portrait}{@render image(portrait, 'portrait')}{/if}
			</div>
		{/if}
		{#if children}<div class="aside">{@render children()}</div>{/if}
	</div>
{/if}

<style>
	/* One short band, so the transcript under it keeps its room. */
	.media {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: var(--space-3);
	}
	.images {
		display: flex;
		gap: var(--space-3);
		min-width: 0;
	}
	.aside {
		flex: 1 1 240px;
		min-width: 0;
	}
	figure {
		margin: 0;
		min-width: 0;
	}
	.portrait {
		flex: none;
	}
	img {
		display: block;
		max-width: 100%;
		width: auto;
		height: min(14dvh, 96px);
		border: 1px solid var(--color-border-subtle);
	}
	/* A scene is larger, so its hotspots are large enough to hit. */
	.scene {
		position: relative;
	}
	.scene img {
		height: min(36dvh, 240px);
	}
	.hotspot {
		position: absolute;
		/* The thicker border on hover and focus stays inside the `rect`. */
		box-sizing: border-box;
		display: flex;
		align-items: flex-end;
		justify-content: center;
		padding: 0;
		border: 1px dashed var(--color-border-control);
		background: none;
		cursor: pointer;
	}
	.hotspot:hover,
	.hotspot:focus-visible {
		border: var(--size-focus-ring) solid var(--color-focus);
	}
	/* The name shows on hover and focus; it is the button's name for screen readers all the time. */
	.name {
		padding: 0 var(--space-1);
		background: var(--color-surface-raised);
		color: var(--color-text-strong);
		font-size: var(--font-size-xs);
		white-space: nowrap;
		opacity: 0;
	}
	.hotspot:hover .name,
	.hotspot:focus-visible .name {
		opacity: 1;
	}
	.text {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
