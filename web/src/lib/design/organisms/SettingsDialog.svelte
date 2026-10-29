<script lang="ts">
	import Button from '../atoms/Button.svelte';
	import Checkbox from '../atoms/Checkbox.svelte';
	import Heading from '../atoms/Heading.svelte';
	import RadioGroup from '../molecules/RadioGroup.svelte';
	import type { EffectsLevel, MotionPreference, RadioOption, Theme } from '../types.ts';

	interface Props {
		open?: boolean;
		theme?: Theme;
		effects?: EffectsLevel;
		motion?: MotionPreference;
		sources?: boolean;
		/** True when the operating system asks for reduced motion. */
		systemReduce?: boolean;
	}

	let {
		open = $bindable(false),
		theme = $bindable('phosphor'),
		effects = $bindable('full'),
		motion = $bindable('system'),
		sources = $bindable(true),
		systemReduce = false
	}: Props = $props();

	let dialog: HTMLDialogElement | undefined = $state();

	// A modal dialog keeps focus inside, closes on Escape and gives focus back to the button that opened it.
	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) dialog.showModal();
		if (!open && dialog.open) dialog.close();
	});

	const themes: RadioOption<Theme>[] = [
		{ value: 'phosphor', label: 'Phosphor' },
		{ value: 'paper', label: 'Paper' },
		{ value: 'night', label: 'Night' }
	];
	const levels: RadioOption<EffectsLevel>[] = [
		{ value: 'full', label: 'Full', description: 'Moving grain, scanlines, glow, idle flicker, glitch on new text, line reveal.' },
		{ value: 'subtle', label: 'Subtle', description: 'Still grain, light scanlines, soft glow, a short colour fringe on new text.' },
		{ value: 'off', label: 'Off', description: 'Flat colours and plain text. This is the fastest. It is best for screen readers.' }
	];
	const motions = $derived<RadioOption<MotionPreference>[]>([
		{ value: 'system', label: `Use my system setting (now: ${systemReduce ? 'reduce motion' : 'motion is permitted'})` },
		{ value: 'reduce', label: 'Always reduce motion' }
	]);
</script>

<dialog bind:this={dialog} class="settings" aria-labelledby="settings-title" onclose={() => (open = false)}>
	<div class="head">
		<Heading level={2} look="title" id="settings-title">Look and motion</Heading>
		<Button size="compact" variant="quiet" onclick={() => (open = false)}>Close</Button>
	</div>

	<RadioGroup legend="Theme" name="theme" look="pills" options={themes} bind:value={theme} />
	<RadioGroup legend="Visual effects" name="effects" look="cards" options={levels} bind:value={effects} />
	<div class="motion">
		<RadioGroup legend="Motion" name="motion" options={motions} bind:value={motion} />
		<p class="note">
			Reduced motion keeps all textures, because textures do not move. It stops all animation and shows new
			text immediately.
		</p>
	</div>
	<Checkbox bind:checked={sources}>Show fragment numbers in the transcript</Checkbox>
</dialog>

<style>
	.settings {
		box-sizing: border-box;
		width: min(var(--width-dialog), calc(100vw - var(--space-9)));
		max-height: calc(100vh - var(--space-9));
		padding: var(--space-7) var(--space-8);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-raised);
		color: var(--color-text);
		box-shadow: var(--shadow-overlay);
	}
	.settings[open] {
		display: flex;
		flex-direction: column;
		gap: var(--space-7);
	}
	.settings::backdrop {
		background: rgb(0 0 0 / 40%);
	}
	.head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-5);
	}
	.motion {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.note {
		margin: 0;
		color: var(--color-text-muted);
	}
</style>
