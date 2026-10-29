<script lang="ts">
	import type { FragmentView, Layer, RoomParts } from '#lib/engine/types.ts';
	import Heading from '../atoms/Heading.svelte';
	import FragmentItem from '../molecules/FragmentItem.svelte';
	import PartItem from '../molecules/PartItem.svelte';
	import SectionRule from '../molecules/SectionRule.svelte';

	interface Props {
		/** The current room id. */
		room: string;
		fragments: FragmentView[];
		/** The things and exits of the generated layers. */
		parts?: RoomParts;
		/** The highlighted fragment index, or null. */
		focus?: number | null;
		/** The time of day and the phase, for the footer. */
		time: string;
		phase: string;
		onfocus?: (index: number | null) => void;
	}

	let { room, fragments, parts, focus = null, time, phase, onfocus }: Props = $props();

	const layers: Layer[] = ['base', 'ambient', 'event', 'objects', 'characters', 'exits'];
	const generated: Partial<Record<Layer, string>> = {
		objects: 'The runtime makes this layer from the things in the room.',
		exits: 'The runtime makes this layer from the visible exits.'
	};
	const partsOf = (layer: Layer) =>
		layer === 'objects' ? (parts?.objects ?? []) : layer === 'exits' ? (parts?.exits ?? []) : [];

	const groups = $derived(layers.map((layer) => ({ layer, items: fragments.filter((f) => f.layer === layer) })));

	function meta(f: FragmentView): string {
		const parts: string[] = [f.mode.replace('_', ' ')];
		if (f.condition) parts.push(`when ${f.condition}`);
		if (f.replaces) parts.push('replaces');
		return parts.join(' · ');
	}
</script>

<!--
	Shows how the runtime assembles the room text now: each layer in order,
	each fragment with its state. Hidden fragments show why they are hidden.
-->
<!-- The panel scrolls, so it must be focusable: keyboard users scroll it with the arrow keys. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<aside class="assembly" aria-labelledby="assembly-title" tabindex="0">
	<header>
		<Heading level={2} id="assembly-title">Assembly · room.{room}</Heading>
	</header>
	<p class="intro">
		The player reads the shown fragments in this order: layer by layer, then by position in the layer. The
		numbers are the same as the numbers in the transcript.
	</p>

	{#each groups as g (g.layer)}
		<section class="layer">
			<SectionRule title={g.layer} />
			{#if g.items.length}
				<ul>
					{#each g.items as f (f.index)}
						<FragmentItem
							number={f.index + 1}
							text={f.text}
							variants={f.variants}
							meta={meta(f)}
							shown={f.shown}
							why={f.why}
							active={focus === f.index}
							onactivate={(on) => onfocus?.(on ? f.index : null)}
						/>
					{/each}
				</ul>
			{:else if generated[g.layer]}
				<p class="empty">{generated[g.layer]}</p>
				{#if partsOf(g.layer).length}
					<ul>
						{#each partsOf(g.layer) as part (part.id)}
							<PartItem label={part.label} text={part.text} detail={part.detail} shown={part.shown} why={part.why} />
						{/each}
					</ul>
				{:else}
					<p class="empty">{g.layer === 'objects' ? 'No things are in this room.' : 'This room has no exits.'}</p>
				{/if}
			{:else}
				<p class="empty">This layer has no fragments.</p>
			{/if}
		</section>
	{/each}

	<footer>
		<span>time {time} · {phase}</span>
	</footer>
</aside>

<style>
	.assembly {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		box-sizing: border-box;
		min-height: 0;
		padding: var(--space-7) var(--space-7);
		overflow-y: auto;
		background: var(--color-surface-raised);
	}
	.intro,
	.empty {
		margin: 0;
		color: var(--color-text-muted);
	}
	.empty {
		font-style: italic;
	}
	.layer {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	ul {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	footer {
		margin-top: auto;
		padding-top: var(--space-5);
		border-top: var(--size-border) solid var(--color-border-subtle);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
