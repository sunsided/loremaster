<script lang="ts">
	import { onMount } from 'svelte';
	import type { Inspect } from '#lib/engine/types.ts';

	interface Props {
		/** Gets the facts for a target. A target is written as `thing:<id>` or `exit:<direction>`. */
		inspect: (kind: 'thing' | 'exit', id: string) => Inspect | null;
		/** Turn the tooltips on or off. */
		enabled?: boolean;
		/** Milliseconds of hover before the tooltip shows. Focus shows it at once. */
		delay?: number;
	}

	let { inspect, enabled = true, delay = 350 }: Props = $props();

	const TOOLTIP_ID = 'author-tooltip';

	let info = $state<Inspect | null>(null);
	let pos = $state({ x: 0, y: 0, below: true });
	let target: HTMLElement | null = null;
	let showTimer: ReturnType<typeof setTimeout> | undefined;
	let hideTimer: ReturnType<typeof setTimeout> | undefined;

	function targetOf(el: EventTarget | null): HTMLElement | null {
		return el instanceof Element ? el.closest<HTMLElement>('[data-inspect]') : null;
	}

	function show(el: HTMLElement) {
		const [kind, id] = (el.dataset.inspect ?? '').split(':') as ['thing' | 'exit', string];
		const found = inspect(kind, id);
		if (!found) return;
		hideNow();
		target = el;
		info = found;
		const r = el.getBoundingClientRect();
		const below = r.bottom + 220 < window.innerHeight;
		pos = { x: Math.min(r.left, window.innerWidth - 340), y: below ? r.bottom + 6 : r.top - 6, below };
		// Screen readers read the tooltip as the description of the element.
		el.setAttribute('aria-describedby', TOOLTIP_ID);
	}

	function hideNow() {
		clearTimeout(showTimer);
		clearTimeout(hideTimer);
		target?.removeAttribute('aria-describedby');
		target = null;
		info = null;
	}

	/** Wait a moment before hiding, so the pointer can move onto the tooltip (WCAG 1.4.13). */
	function hideSoon() {
		clearTimeout(showTimer);
		clearTimeout(hideTimer);
		hideTimer = setTimeout(hideNow, 200);
	}

	onMount(() => {
		const over = (e: PointerEvent) => {
			if (!enabled) return;
			const el = targetOf(e.target);
			if (!el || el === target) return;
			clearTimeout(showTimer);
			showTimer = setTimeout(() => show(el), delay);
		};
		const out = (e: PointerEvent) => {
			if (targetOf(e.target) && !targetOf(e.relatedTarget)) hideSoon();
		};
		const focusIn = (e: FocusEvent) => {
			const el = enabled ? targetOf(e.target) : null;
			if (el) show(el);
		};
		const focusOut = () => hideSoon();
		// Escape closes the tooltip without moving focus.
		const key = (e: KeyboardEvent) => {
			if (e.key === 'Escape' && info) hideNow();
		};
		document.addEventListener('pointerover', over);
		document.addEventListener('pointerout', out);
		document.addEventListener('focusin', focusIn);
		document.addEventListener('focusout', focusOut);
		document.addEventListener('keydown', key);
		return () => {
			hideNow();
			document.removeEventListener('pointerover', over);
			document.removeEventListener('pointerout', out);
			document.removeEventListener('focusin', focusIn);
			document.removeEventListener('focusout', focusOut);
			document.removeEventListener('keydown', key);
		};
	});

	$effect(() => {
		if (!enabled) hideNow();
	});
</script>

<!--
	An author tooltip: facts about the thing or exit under the pointer or with focus.
	It does not take clicks away from the element: a click still plays the game.
	It shows on hover (after a short delay) and on keyboard focus, stays while the pointer is on it,
	and closes with Escape. It is the description of the element for screen readers.
-->
{#if info}
	<!-- aria-hidden: screen readers get the text through aria-describedby on the element, not by browsing to it. -->
	<div
		id={TOOLTIP_ID}
		role="tooltip"
		aria-hidden="true"
		class="tooltip"
		class:above={!pos.below}
		style:left="{pos.x}px"
		style:top="{pos.y}px"
		onpointerenter={() => clearTimeout(hideTimer)}
		onpointerleave={hideSoon}
	>
		<p class="title"><span class="kind">{info.kind}</span> {info.title}<span class="sep">. </span></p>
		<dl>
			{#each info.facts as [label, value] (label)}
				<dt>{label}<span class="sep">: </span></dt>
				<dd>{value}<span class="sep">. </span></dd>
			{/each}
			{#if info.rules.length}
				<dt>rules<span class="sep">: </span></dt>
				<dd>{info.rules.join(', ')}<span class="sep">. </span></dd>
			{/if}
		</dl>
	</div>
{/if}

<style>
	.tooltip {
		position: fixed;
		z-index: var(--z-dialog);
		box-sizing: border-box;
		width: max-content;
		max-width: min(330px, calc(100vw - var(--space-6)));
		padding: var(--space-4) var(--space-5);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		color: var(--color-text);
		box-shadow: var(--shadow-overlay);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-tight);
		text-shadow: none;
	}
	.above {
		transform: translateY(-100%);
	}
	.title {
		margin: 0 0 var(--space-3);
		font-weight: var(--font-weight-strong);
		color: var(--color-text-strong);
	}
	.kind {
		margin-right: var(--space-2);
		padding: 0 var(--space-2);
		border: var(--size-border) dashed var(--color-warning);
		font-size: var(--font-size-xs);
		font-weight: var(--font-weight-regular);
		color: var(--color-text-warning);
		text-transform: uppercase;
	}
	dl {
		display: grid;
		grid-template-columns: max-content minmax(0, 1fr);
		gap: var(--space-1) var(--space-4);
		margin: 0;
	}
	dt {
		color: var(--color-text-muted);
	}
	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
	/* Punctuation for the spoken description only. */
	.sep {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
