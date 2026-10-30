<script lang="ts">
	interface Action {
		id: string;
		label: string;
		run: () => void;
	}

	interface Props {
		/** The screen rectangle of the selection. The toolbar shows above it, or under it near the top. */
		rect: { left: number; top: number; bottom: number };
		actions: Action[];
		/** Escape: close the toolbar and go back to the text. */
		onclose: () => void;
		/** Set to a new number to move the focus into the toolbar (Ctrl+. in the editor). */
		focus?: number;
	}

	let { rect, actions, onclose, focus = 0 }: Props = $props();
	let bar: HTMLElement | undefined = $state();

	const top = $derived(rect.top > 56 ? rect.top - 44 : rect.bottom + 8);
	const left = $derived(Math.max(8, Math.min(rect.left, (typeof window === 'undefined' ? 1200 : window.innerWidth) - 360)));

	$effect(() => {
		if (focus) bar?.querySelector<HTMLButtonElement>('button')?.focus();
	});

	function keydown(event: KeyboardEvent) {
		const buttons = [...(bar?.querySelectorAll<HTMLButtonElement>('button') ?? [])];
		const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
		if (event.key === 'Escape') {
			event.preventDefault();
			onclose();
		} else if (event.key === 'ArrowRight' || event.key === 'ArrowLeft') {
			event.preventDefault();
			const next = (i + (event.key === 'ArrowRight' ? 1 : -1) + buttons.length) % buttons.length;
			buttons[next]?.focus();
		}
	}
</script>

<!--
	A small toolbar over selected text in the writer: "type first, tune later".
	The mouse uses it at once; with the keyboard, Ctrl+. moves the focus into it, the arrow keys move between
	the buttons, and Escape goes back to the text.
	mousedown does not take the focus from the editor, so the selection stays.
-->
<div
	class="tools"
	role="toolbar"
	aria-label="Tools for the selected text"
	tabindex="-1"
	bind:this={bar}
	style:top="{top}px"
	style:left="{left}px"
	onkeydown={keydown}
	onmousedown={(e) => e.preventDefault()}
>
	{#each actions as a, i (a.id)}
		<button type="button" tabindex={i === 0 ? 0 : -1} onclick={a.run}>{a.label}</button>
	{/each}
</div>

<style>
	.tools {
		position: fixed;
		z-index: 20;
		display: flex;
		flex-wrap: wrap;
		max-width: 360px;
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		box-shadow: var(--shadow-overlay);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		text-shadow: none;
	}
	button {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-4);
		border: 0;
		border-right: var(--size-border) solid var(--color-border-subtle);
		background: none;
		color: var(--color-text);
		font: inherit;
		white-space: nowrap;
	}
	button:last-child {
		border-right: 0;
	}
	button:hover {
		background: var(--color-accent-tint);
		color: var(--color-text-accent);
	}
	button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: -2px;
	}
</style>
