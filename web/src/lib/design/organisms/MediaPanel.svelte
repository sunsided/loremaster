<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Image {
		/** Null when the player has no file for it: the caption shows instead. */
		src: string | null;
		/** The alt text. Empty for a decorative image. */
		alt: string;
	}

	interface Props {
		illustration?: Image | null;
		portrait?: Image | null;
		/** Shown beside the images: the sound controls and captions. */
		children?: Snippet;
	}

	let { illustration = null, portrait = null, children }: Props = $props();
</script>

{#snippet image(item: Image, kind: string)}
	<figure class={kind}>
		{#if item.src}
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
	.text {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
