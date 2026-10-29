<script lang="ts">
	import { settings, type Effects, type Motion, type Theme } from '#lib/settings.svelte.ts';

	let { onclose }: { onclose: () => void } = $props();

	const themes: { id: Theme; label: string }[] = [
		{ id: 'phosphor', label: 'Phosphor' },
		{ id: 'paper', label: 'Paper' },
		{ id: 'night', label: 'Night' }
	];
	const effects: { id: Effects; label: string; note: string }[] = [
		{ id: 'full', label: 'Full', note: 'Moving grain, scanlines, glow, idle flicker, glitch on new text, line reveal.' },
		{ id: 'subtle', label: 'Subtle', note: 'Still grain, light scanlines, soft glow, a brief colour fringe on new text.' },
		{ id: 'off', label: 'Off', note: 'Flat colours and plain text. Fastest, best for screen readers and low-end devices.' }
	];
	const motions: { id: Motion; label: string }[] = [
		{ id: 'system', label: 'Follow my system setting' },
		{ id: 'reduce', label: 'Always reduce motion' }
	];

	function onkeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<div class="panel" role="dialog" aria-label="Look and motion" tabindex="-1" {onkeydown}>
	<div class="head">
		<h2>LOOK AND MOTION</h2>
		<button class="close" onclick={onclose}>close</button>
	</div>

	<fieldset>
		<legend>Theme</legend>
		<div class="row">
			{#each themes as t (t.id)}
				<label class="pill"><input type="radio" name="theme" value={t.id} bind:group={settings.theme} />{t.label}</label>
			{/each}
		</div>
	</fieldset>

	<fieldset>
		<legend>Visual effects</legend>
		<div class="cards">
			{#each effects as e (e.id)}
				<label class="card" class:on={settings.effects === e.id}>
					<span><input type="radio" name="fx" value={e.id} bind:group={settings.effects} /> <b>{e.label}</b></span>
					<span class="note">{e.note}</span>
				</label>
			{/each}
		</div>
	</fieldset>

	<fieldset>
		<legend>Motion</legend>
		{#each motions as m (m.id)}
			<label class="line">
				<input type="radio" name="motion" value={m.id} bind:group={settings.motion} />
				{m.label}
				{#if m.id === 'system'}
					<span class="note">(currently: {settings.systemReduce ? 'reduce motion' : 'motion allowed'})</span>
				{/if}
			</label>
		{/each}
		<p class="note">
			Reduced motion keeps every texture, because textures don't move. It stops all animation and
			shows new text at once.
		</p>
	</fieldset>

	<label class="line"><input type="checkbox" bind:checked={settings.sources} /> Show fragment numbers in the transcript</label>
</div>

<style>
	.panel {
		position: absolute;
		right: 16px;
		top: 52px;
		z-index: 20;
		width: min(560px, calc(100vw - 32px));
		box-sizing: border-box;
		padding: 20px 22px;
		background: var(--panel);
		border: 1px solid var(--chip-line);
		box-shadow: 0 16px 40px rgb(0 0 0 / 35%);
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
	.head {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	h2 {
		margin: 0;
		font-size: 14px;
		color: var(--accent);
		letter-spacing: 0.04em;
	}
	.close {
		border: 1px solid var(--chip-line);
		background: transparent;
		padding: 6px 10px;
	}
	fieldset {
		border: 0;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	legend {
		font-weight: 600;
		margin-bottom: 8px;
	}
	.row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}
	.pill,
	.line {
		display: flex;
		gap: 8px;
		align-items: center;
	}
	.pill {
		border: 1px solid var(--chip-line);
		padding: 8px 12px;
	}
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 10px;
	}
	.card {
		border: 1px solid var(--chip-line);
		padding: 12px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.card.on {
		border-color: var(--accent);
	}
	.note {
		color: var(--muted);
		line-height: 1.5;
		margin: 0;
	}
	input {
		accent-color: var(--accent);
	}
</style>
