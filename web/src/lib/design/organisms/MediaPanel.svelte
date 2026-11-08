<script lang="ts">
	interface Image {
		/** Null when the player has no file for it: the caption shows instead. */
		src: string | null;
		/** The alt text. Empty for a decorative image. */
		alt: string;
	}

	interface Props {
		illustration?: Image | null;
		portrait?: Image | null;
	}

	let { illustration = null, portrait = null }: Props = $props();
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

{#if illustration || portrait}
	<div class="media">
		{#if illustration}{@render image(illustration, 'illustration')}{/if}
		{#if portrait}{@render image(portrait, 'portrait')}{/if}
	</div>
{/if}

<style>
	.media {
		flex: none;
		display: flex;
		gap: var(--space-3);
		justify-content: center;
		padding: var(--space-3) var(--space-4) 0;
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
		height: 14dvh;
		border: 1px solid var(--color-border-subtle);
	}
	.text {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
