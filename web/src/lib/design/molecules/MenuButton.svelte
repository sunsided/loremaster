<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { MenuItem } from '../types.ts';

	interface Props {
		/** The accessible name of the button, when its content is not enough, for example "Project: The Lighthouse". */
		label: string;
		/** What the button shows. */
		children: Snippet;
		items: MenuItem[];
		/** `start`: the menu opens under the left edge. `end`: under the right edge. */
		align?: 'start' | 'end';
		/** The look of the button: `plain` for a header, `inverse` on the status line. */
		look?: 'plain' | 'inverse';
	}

	let { label, children, items, align = 'start', look = 'plain' }: Props = $props();
	let open = $state(false);
	let root: HTMLElement | undefined = $state();
	let button: HTMLButtonElement | undefined = $state();
	const id = $props.id();

	function buttons(): HTMLButtonElement[] {
		return [...(root?.querySelectorAll<HTMLButtonElement>('.menu button:not(:disabled)') ?? [])];
	}

	function toggle() {
		open = !open;
		if (open) queueMicrotask(() => buttons()[0]?.focus());
	}

	function close(refocus = true) {
		open = false;
		if (refocus) button?.focus();
	}

	function keydown(event: KeyboardEvent) {
		if (!open) return;
		const list = buttons();
		const i = list.indexOf(document.activeElement as HTMLButtonElement);
		if (event.key === 'Escape') {
			event.preventDefault();
			close();
		} else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			const next = (i + (event.key === 'ArrowDown' ? 1 : -1) + list.length) % list.length;
			list[next]?.focus();
		} else if (event.key === 'Home' || event.key === 'End') {
			event.preventDefault();
			(event.key === 'Home' ? list[0] : list.at(-1))?.focus();
		}
	}

	function outside(event: PointerEvent) {
		if (open && root && !root.contains(event.target as Node)) close(false);
	}
</script>

<svelte:window onpointerdown={outside} />

<!--
	A button that opens a short list of actions under it.
	It is a disclosure: the button has aria-expanded, and the actions are normal buttons in a list.
	The arrow keys move between the actions, Escape closes the list and returns to the button,
	and a click outside closes it.
-->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="menu-button {look}" bind:this={root} onkeydown={keydown}>
	<button
		bind:this={button}
		type="button"
		class="trigger"
		aria-label={label}
		aria-expanded={open}
		aria-controls="{id}-menu"
		onclick={toggle}>{@render children()}<span class="caret" aria-hidden="true">▾</span></button
	>
	<div class="menu" class:end={align === 'end'} id="{id}-menu" hidden={!open}>
		<ul>
			{#each items as item, i (item.id)}
				{#if item.group && item.group !== items[i - 1]?.group}
					<li class="group" aria-hidden="true">{item.group}</li>
				{/if}
				<li>
					<button
						type="button"
						disabled={item.disabled}
						onclick={() => {
							close();
							item.run();
						}}>{item.label}</button
					>
				</li>
			{/each}
		</ul>
	</div>
</div>

<style>
	.menu-button {
		position: relative;
		display: inline-flex;
	}
	.trigger {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		min-height: var(--size-target-compact);
		padding: 0 var(--space-2);
		border: var(--size-border) solid transparent;
		background: none;
		color: inherit;
		font: inherit;
	}
	.plain .trigger:hover,
	.plain .trigger[aria-expanded='true'] {
		border-color: var(--color-border-control);
	}
	.inverse .trigger:hover,
	.inverse .trigger[aria-expanded='true'] {
		border-color: var(--color-status-text);
	}
	.trigger:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
	.inverse .trigger:focus-visible {
		outline-color: var(--color-status-text);
	}
	.caret {
		font-size: 0.8em;
		opacity: 0.8;
	}
	.menu {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		z-index: 40;
		min-width: 220px;
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
	.menu.end {
		left: auto;
		right: 0;
	}
	.menu[hidden] {
		display: none;
	}
	ul {
		margin: 0;
		padding: var(--space-2) 0;
		list-style: none;
	}
	.group {
		padding: var(--space-3) var(--space-4) var(--space-1);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
	}
	.menu button {
		display: block;
		width: 100%;
		min-height: var(--size-target-compact);
		padding: 0 var(--space-4);
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
	}
	.menu button:hover,
	.menu button:focus-visible {
		background: var(--color-accent-tint);
		color: var(--color-text-accent);
		outline: none;
	}
	.menu button:disabled {
		color: var(--color-text-muted);
	}
</style>
