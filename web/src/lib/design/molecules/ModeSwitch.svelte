<script lang="ts">
	type Mode = 'play' | 'write';

	interface Props {
		/** `play`: the game as the player sees it. `write`: the same screen, open for editing. */
		value?: Mode;
		onchange?: (mode: Mode) => void;
	}

	let { value = $bindable('play'), onchange }: Props = $props();
	const modes: { value: Mode; label: string }[] = [
		{ value: 'play', label: 'Play' },
		{ value: 'write', label: 'Write' }
	];

	function pick(mode: Mode) {
		if (mode === value) return;
		value = mode;
		onchange?.(mode);
	}

	function keydown(event: KeyboardEvent) {
		if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) {
			event.preventDefault();
			pick(value === 'play' ? 'write' : 'play');
			// aria-checked updates on the next flush, so find the new button by its index.
			const buttons = (event.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>('[role="radio"]');
			buttons[modes.findIndex((m) => m.value === value)]?.focus();
		}
	}
</script>

<!--
	The two modes of the editor, as one switch: Play and Write. Ctrl+E (Cmd+E) switches from anywhere.
	It is a radio group: Tab moves to the checked mode, the arrow keys switch.
-->
<div class="mode-switch" role="radiogroup" aria-label="Mode" aria-keyshortcuts="Control+E Meta+E" tabindex="-1" onkeydown={keydown}>
	{#each modes as m (m.value)}
		<button
			type="button"
			role="radio"
			aria-checked={m.value === value}
			tabindex={m.value === value ? 0 : -1}
			onclick={() => pick(m.value)}>{m.label}</button
		>
	{/each}
</div>

<style>
	.mode-switch {
		display: inline-flex;
		border: var(--size-border) solid var(--color-border-control);
	}
	button {
		min-height: var(--size-target-compact);
		padding: 0 var(--space-4);
		border: 0;
		background: none;
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
	}
	button[aria-checked='true'] {
		background: var(--color-accent);
		color: var(--color-text-on-accent);
		font-weight: var(--font-weight-strong);
	}
	button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
		outline-offset: 2px;
	}
</style>
