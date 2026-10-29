<script lang="ts">
	import Heading from '../atoms/Heading.svelte';

	interface Props {
		title: string;
		/** A short label after the title, for example "generated". */
		tag?: string;
		/** The heading level in the document outline. */
		level?: 2 | 3 | 4;
	}

	let { title, tag, level = 3 }: Props = $props();
</script>

<!-- A small section heading with a line that fills the rest of the row. -->
<div class="section-rule">
	<Heading {level}>{title}</Heading>
	{#if tag}<span class="tag">{tag}</span>{/if}
	<span class="line" aria-hidden="true"></span>
</div>

<style>
	.section-rule {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}
	.tag {
		padding: 0 var(--space-3);
		border: var(--size-border) dashed var(--color-border-control);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.line {
		flex: 1 1 auto;
		height: var(--size-border);
		background: var(--color-border-subtle);
	}
</style>
