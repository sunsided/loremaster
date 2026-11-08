<script lang="ts">
	interface Props {
		/** The newest captions, oldest first. */
		captions: { id: number; text: string }[];
	}

	let { captions }: Props = $props();
</script>

<div class="captions" role="log" aria-live="polite" aria-label="Sound captions">
	{#each captions as caption (caption.id)}
		<p>{caption.text}</p>
	{/each}
</div>

<style>
	.captions {
		flex: none;
		padding: 0 var(--space-4);
	}
	p {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
