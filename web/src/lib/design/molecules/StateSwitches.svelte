<script lang="ts">
	import Button from '../atoms/Button.svelte';
	import StateSwitch from '../atoms/StateSwitch.svelte';
	import type { RoomSwitch } from '../types.ts';

	interface Props {
		/** The values that the room text depends on. */
		switches: RoomSwitch[];
		/** The values that the preview shows instead of the game, by name. Empty: no preview. */
		preview?: Record<string, string>;
		onpreview: (name: string, value: string) => void;
		/** Leave the preview: the room text is the game again. Leave it out for no buttons. */
		onreset?: () => void;
		/** Set the previewed values in the game for real. Leave it out for no buttons. */
		onapply?: () => void;
		/** Show the help line under the switches when there is no preview. */
		hint?: boolean;
	}

	let { switches, preview = {}, onpreview, onreset, onapply, hint = true }: Props = $props();

	const previewing = $derived(Object.keys(preview).length > 0);
	const announce = $derived(
		previewing ? `Preview: ${Object.entries(preview).map(([n, v]) => `${n} ${v}`).join(', ')}. The game did not move.` : ''
	);
</script>

<!--
	The values that the room text depends on, as switches. A change is a preview: the room text changes in place,
	and the game does not move. "Make it so" sets the values in the game. "Reset" ends the preview.
	The Here panel, the room desk and the pinned room over the fragment writer all use this block.
-->
<div class="state-switches">
	{#if switches.length}
		<div class="switches">
			{#each switches as sw (sw.name)}
				<StateSwitch
					name={sw.name}
					value={preview[sw.name] ?? sw.value}
					options={sw.options.includes(sw.value) ? sw.options : [sw.value, ...sw.options]}
					changed={sw.name in preview}
					onchange={(v) => onpreview(sw.name, v)}
				/>
			{/each}
		</div>
		{#if previewing && onapply && onreset}
			<div class="actions">
				<Button size="compact" variant="primary" onclick={onapply}>Make it so</Button>
				<Button size="compact" onclick={onreset}>Reset</Button>
			</div>
		{:else if hint && !previewing}
			<p class="note">Change a value to preview the room text in place. The game does not move.</p>
		{/if}
	{:else}
		<p class="note">The text of this room does not depend on a value.</p>
	{/if}
	<p class="visually-hidden" aria-live="polite">{announce}</p>
</div>

<style>
	.switches {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.actions {
		display: flex;
		gap: var(--space-3);
		margin-top: var(--space-3);
	}
	.note {
		margin: var(--space-3) 0 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-body);
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
