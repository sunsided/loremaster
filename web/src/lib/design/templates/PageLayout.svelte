<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		header: Snippet;
		children: Snippet;
		/** Maximum width of the content. Use a token reference. */
		maxWidth?: string;
	}

	let { header, children, maxWidth = 'var(--width-player-max)' }: Props = $props();
</script>

<!--
	The frame of an editor view, such as Map, Story or Rules: the header and one main region.
	The main region scrolls. Its content stays in the center, not wider than maxWidth.
	On screens wider than --width-content-max the whole frame stays in the center.
-->
<div class="layout">
	<a class="skip" href="#view-main">Skip to the content</a>
	<div class="frame">
		{@render header()}
		<!-- The region scrolls, so keyboard users must be able to focus it. -->
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<main id="view-main" tabindex="0">
			<div class="content" style:max-width={maxWidth}>{@render children()}</div>
		</main>
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
	main {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		padding: var(--space-8) clamp(var(--space-6), 4vw, var(--space-10));
	}
	main:focus {
		outline: none;
	}
	.content {
		margin: 0 auto;
	}
	@media (max-width: 900px) {
		.layout {
			height: auto;
			min-height: 100dvh;
		}
	}
</style>
