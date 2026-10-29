<script lang="ts">
	import Heading from '../atoms/Heading.svelte';

	interface Props {
		title: string;
		/** The heading level in the document outline. */
		level?: 2 | 3 | 4;
	}

	let { title, level = 3 }: Props = $props();
</script>

<!-- A small section heading with a line that fills the rest of the row. -->
<div class="section-rule">
	<Heading {level}>{title}</Heading>
	<span class="line" aria-hidden="true"></span>
</div>

<style>
	.section-rule {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}
	.line {
		flex: 1 1 auto;
		height: var(--size-border);
		background: var(--color-border-subtle);
	}
</style>
