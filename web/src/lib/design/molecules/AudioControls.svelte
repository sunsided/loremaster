<script lang="ts">
	import Button from '../atoms/Button.svelte';
	import Checkbox from '../atoms/Checkbox.svelte';

	interface Props {
		/** The sound is on. It is off by default. */
		sound?: boolean;
		/** Show captions also while the sound is on. */
		captions?: boolean;
		/** The volumes, from 0 to 100. */
		music?: number;
		ambience?: number;
		effects?: number;
	}

	let {
		sound = $bindable(false),
		captions = $bindable(false),
		music = $bindable(70),
		ambience = $bindable(70),
		effects = $bindable(70)
	}: Props = $props();
</script>

<section class="audio" aria-label="Sound">
	<Button size="compact" pressed={sound} aria-keyshortcuts="Alt+M" onclick={() => (sound = !sound)}>
		Sound {sound ? 'on' : 'off'}
	</Button>
	<span class="key">Alt+M</span>
	<Checkbox bind:checked={captions}>Captions</Checkbox>
	<details>
		<summary>Volumes</summary>
		<div class="volumes">
			<label>Music <input type="range" min="0" max="100" step="5" bind:value={music} /></label>
			<label>Ambience <input type="range" min="0" max="100" step="5" bind:value={ambience} /></label>
			<label>Effects <input type="range" min="0" max="100" step="5" bind:value={effects} /></label>
		</div>
	</details>
</section>

<style>
	.audio {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
		font-size: var(--font-size-xs);
		color: var(--color-text);
	}
	.key {
		color: var(--color-text-muted);
	}
	summary {
		cursor: pointer;
	}
	.volumes {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}
	label {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
	}
	input[type='range'] {
		width: 80px;
		accent-color: var(--color-accent);
	}
	input[type='range']:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}
</style>
