<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** Words for screen readers after the text of the chip, for example "Show the AI jobs." */
		hint: string;
		/** What the chip shows on the status line. */
		chip: Snippet;
		/** The heading of the tray, for example "AI · loremaster serve". */
		title: string;
		/** A short fact at the right of the heading, for example "$0.31". */
		meta?: string;
		/** The content of the tray. */
		children: Snippet;
		open?: boolean;
	}

	let { hint, chip, title, meta, children, open = $bindable(false) }: Props = $props();
	let root: HTMLElement | undefined = $state();
	let button: HTMLButtonElement | undefined = $state();
	let panel: HTMLElement | undefined = $state();
	const id = $props.id();
	let place = $state({ top: 0, left: 0 });

	/** Put the tray under the chip, inside the screen. The tray is fixed, so the frame of the screen does not cut it. */
	function measure() {
		if (!button || !panel) return;
		const r = button.getBoundingClientRect();
		const margin = 16;
		const width = panel.offsetWidth;
		place = { top: r.bottom + 4, left: Math.max(margin, Math.min(r.left, window.innerWidth - width - margin)) };
	}

	function toggle() {
		open = !open;
		if (open)
			queueMicrotask(() => {
				measure();
				panel?.focus();
			});
	}

	function close(refocus = true) {
		open = false;
		if (refocus) button?.focus();
	}

	function keydown(event: KeyboardEvent) {
		if (open && event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			close();
		}
	}

	function outside(event: PointerEvent) {
		if (open && root && !root.contains(event.target as Node)) close(false);
	}
</script>

<svelte:window onpointerdown={outside} onresize={() => open && measure()} onscroll={() => open && measure()} />

<!--
	A chip on the status line that opens a small tray under it, for things of the whole session,
	for example the AI jobs. It is a disclosure: the chip has aria-expanded, and the tray is a labelled group.
	The tray gets the focus when it opens. Escape closes it and returns to the chip. A click outside closes it.
-->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<span class="status-tray" bind:this={root} onkeydown={keydown}>
	<button
		bind:this={button}
		type="button"
		class="chip"
		aria-expanded={open}
		aria-controls="{id}-tray"
		onclick={toggle}>{@render chip()}<span class="visually-hidden">. {hint}</span><span class="caret" aria-hidden="true">▾</span></button
	>
	<div class="tray" id="{id}-tray" role="group" aria-labelledby="{id}-title" tabindex="-1" bind:this={panel} hidden={!open} style:top="{place.top}px" style:left="{place.left}px">
		<p class="head"><span id="{id}-title" class="title">{title}</span>{#if meta}<span class="meta">{meta}</span>{/if}</p>
		<div class="body">{@render children()}</div>
	</div>
</span>

<style>
	.status-tray {
		position: relative;
		display: inline-flex;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		min-height: 1.6em;
		padding: 0 var(--space-3);
		border: 0;
		background: var(--color-status-text);
		color: var(--color-ghost);
		font: inherit;
	}
	.chip:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-status-text);
		outline-offset: 2px;
	}
	.caret {
		font-size: 0.8em;
		opacity: 0.8;
	}
	.tray {
		position: fixed;
		z-index: 40;
		width: min(26rem, calc(100vw - 32px));
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		box-shadow: var(--shadow-overlay);
		color: var(--color-text);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-regular);
		letter-spacing: normal;
		text-transform: none;
		text-shadow: none;
	}
	.tray[hidden] {
		display: none;
	}
	.tray:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.head {
		display: flex;
		justify-content: space-between;
		gap: var(--space-4);
		margin: 0;
		padding: var(--space-3) var(--space-4);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
	}
	.title {
		color: var(--color-text-strong);
		font-weight: var(--font-weight-strong);
	}
	.meta {
		color: var(--color-text-muted);
		text-transform: none;
		letter-spacing: normal;
	}
	.body {
		padding: var(--space-4);
	}
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
