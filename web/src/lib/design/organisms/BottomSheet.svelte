<script lang="ts">
	import type { Snippet } from 'svelte';

	type Height = 'peek' | 'half' | 'full';

	interface Props {
		/** The name of the sheet, for its heading and for screen readers, for example "Here". */
		title: string;
		/** One line that shows when the sheet only peeks, for example "2 exits · 1 rule in 30 min". */
		summary: string;
		/** How far the sheet is open. */
		height?: Height;
		children: Snippet;
	}

	let { title, summary, height = $bindable('peek'), children }: Props = $props();
	const id = $props.id();
	const next: Record<Height, Height> = { peek: 'half', half: 'full', full: 'peek' };

	// Drag the grab bar up or down to open or close the sheet. A tap is a click: see `click`.
	let start: { y: number; h: Height } | null = null;
	let dragged = false;
	function down(e: PointerEvent) {
		start = { y: e.clientY, h: height };
		dragged = false;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}
	function up(e: PointerEvent) {
		if (!start) return;
		const dy = e.clientY - start.y;
		if (Math.abs(dy) >= 8) {
			dragged = true;
			height = dy < 0 ? (start.h === 'peek' ? 'half' : 'full') : start.h === 'full' ? 'half' : 'peek';
		}
		start = null;
	}
	// A tap, Enter, Space and the activation of a screen reader all give a click.
	function click() {
		if (dragged) {
			dragged = false;
			return;
		}
		height = next[height];
	}
</script>

<!--
	A panel at the bottom of a phone screen. It peeks with one summary line, and opens to half or full height.
	The grab bar is a button: a tap moves to the next height (peek, half, full), and a drag opens or closes.
	Keyboard and switch users use the same button. The content scrolls inside the sheet.
-->
<section class="sheet {height}" aria-labelledby="{id}-title">
	<button
		type="button"
		class="grab"
		aria-expanded={height !== 'peek'}
		aria-controls="{id}-body"
		onpointerdown={down}
		onpointerup={up}
		onclick={click}
	>
		<span class="bar" aria-hidden="true"></span>
		<span class="line">
			<span id="{id}-title" class="title">{title}</span>
			<span class="summary">{summary}</span>
			<span class="arrow" aria-hidden="true">{height === 'full' ? '▾' : '▴'}</span>
			<span class="visually-hidden">{height === 'peek' ? ', open' : height === 'half' ? ', open more' : ', close'}</span>
		</span>
	</button>
	<div class="body" id="{id}-body" hidden={height === 'peek'}>{@render children()}</div>
</section>

<style>
	.sheet {
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: var(--z-sheet);
		display: flex;
		flex-direction: column;
		border-top: var(--size-border) solid var(--color-accent);
		background: var(--color-surface-raised);
		box-shadow: var(--shadow-overlay);
		padding-bottom: env(safe-area-inset-bottom, 0px);
	}
	.half {
		height: 50dvh;
	}
	.full {
		height: 88dvh;
	}
	.grab {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-2);
		width: 100%;
		min-height: var(--size-target);
		padding: var(--space-2) var(--space-5) var(--space-3);
		border: 0;
		background: none;
		color: var(--color-text);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		text-align: left;
		touch-action: none;
		cursor: grab;
	}
	.grab:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: -2px;
	}
	.bar {
		align-self: center;
		width: 40px;
		height: 4px;
		border-radius: 2px;
		background: var(--color-border-control);
	}
	.line {
		display: flex;
		align-items: baseline;
		gap: var(--space-3);
		min-width: 0;
	}
	.title {
		color: var(--color-text-strong);
		font-weight: var(--font-weight-strong);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
		font-size: var(--font-size-xs);
	}
	.summary {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-muted);
	}
	.arrow {
		color: var(--color-text-accent);
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		position: relative;
	}
	.body[hidden] {
		display: none;
	}
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
	@media (prefers-reduced-motion: no-preference) {
		.sheet {
			transition: height var(--motion-sheet) ease-out;
		}
	}
</style>
