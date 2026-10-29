<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** A stable id, unique on the page. */
		id: string;
		/** The name of the region, for the button label, for example "Assembly". */
		topic: string;
		children: Snippet;
	}

	let { id, topic, children }: Props = $props();

	/** Open from a click. It stays open until a second click, Escape or a click outside. */
	let pinned = $state(false);
	/** Open from hover or focus. */
	let hovered = $state(false);
	const open = $derived(pinned || hovered);
	const textId = $derived(`help-${id}`);
	let root: HTMLElement | undefined = $state();
	let bubble: HTMLElement | undefined = $state();
	/** The bubble is fixed to the viewport, so a scroll container does not cut it. It stays in the viewport. */
	let place = $state('');

	$effect(() => {
		if (!open || !root || !bubble) return;
		const button = root.getBoundingClientRect();
		const width = bubble.offsetWidth;
		const margin = 16;
		const left = Math.max(margin, Math.min(button.left, window.innerWidth - width - margin));
		place = `top: ${button.bottom + 6}px; left: ${left}px`;
		// A scroll moves the button away from the fixed bubble: close it.
		const close = () => {
			pinned = false;
			hovered = false;
		};
		document.addEventListener('scroll', close, { capture: true, once: true });
		return () => document.removeEventListener('scroll', close, { capture: true });
	});

	function outside(event: PointerEvent) {
		if (pinned && root && !root.contains(event.target as Node)) pinned = false;
	}
	function escape(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			pinned = false;
			hovered = false;
		}
	}
</script>

<svelte:window onpointerdown={outside} onkeydown={escape} />

<!--
	A help text in a (?) toggletip next to the heading of a region.
	Hover or focus shows the text. A click keeps it open. Escape, a second click or a click outside closes it.
	The text is always the description of the button, so screen readers read it with the button.
-->
<span
	class="help"
	bind:this={root}
	role="presentation"
	onpointerenter={() => (hovered = true)}
	onpointerleave={() => (hovered = false)}
>
	<button
		class="toggle"
		aria-expanded={open}
		aria-describedby={textId}
		aria-label="Help: {topic}"
		onfocus={() => (hovered = true)}
		onblur={() => (hovered = false)}
		onclick={() => (pinned = !pinned)}>?</button
	>
	<span class="bubble" id={textId} role="tooltip" class:open bind:this={bubble} style={place}>{@render children()}</span>
</span>

<style>
	.help {
		display: inline-flex;
	}
	.toggle {
		width: 24px;
		height: 24px;
		padding: 0;
		border: var(--size-border) solid var(--color-border-control);
		border-radius: 50%;
		background: transparent;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		line-height: 1;
	}
	.toggle:hover,
	.toggle[aria-expanded='true'] {
		border-color: var(--color-accent);
		color: var(--color-text-accent);
	}
	.bubble {
		position: fixed;
		z-index: 10;
		display: none;
		width: max-content;
		max-width: min(36ch, calc(100vw - 32px));
		padding: var(--space-3) var(--space-4);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		color: var(--color-text);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-body);
		box-shadow: var(--shadow-overlay);
	}
	/* A bridge over the gap, so the pointer can move from the button to the text. */
	.bubble::before {
		content: '';
		position: absolute;
		inset: -8px 0 100% 0;
	}
	.bubble.open {
		display: block;
	}
</style>
