<script lang="ts">
	import type { StatView, ThingInfo } from '#lib/engine/types.ts';
	import VisuallyHidden from '../atoms/VisuallyHidden.svelte';
	import { STAT_MARK_MS } from '../reveal.ts';

	interface Props {
		/** The stats of the story, with their current values. */
		stats: StatView[];
		/** What the player carries. */
		inventory?: ThingInfo[];
	}

	let { stats, inventory = [] }: Props = $props();
	const id = $props.id();

	const numbers = $derived(stats.filter((s) => typeof s.value === 'number'));
	const codewords = $derived(stats.filter((s) => s.value === true));
	const line = $derived(
		numbers
			.filter((s) => s.short)
			.map((s) => `${s.short} ${s.value}${s.max !== undefined && s.value !== s.max ? `/${s.max}` : ''}`)
			.join('  ')
	);
	const amount = (s: StatView) => `${s.value}${s.max !== undefined ? ` of ${s.max}` : ''}`;
	const spoken = (s: StatView) => `${s.label} ${amount(s)}`;

	let open = $state(false);
	/** The direction of each recent change, by the name of the variable. */
	let changed = $state<Record<string, 'up' | 'down'>>({});
	let announcement = $state('');

	// The values of the last check. They only compare and do not show.
	let previous: Map<string, StatView> | undefined;
	let timer: ReturnType<typeof setTimeout> | undefined;

	$effect(() => {
		const last = previous;
		previous = new Map(stats.map((s) => [s.var, s]));
		if (!last) return;
		const marks: Record<string, 'up' | 'down'> = {};
		const said: string[] = [];
		for (const s of stats) {
			const before = last.get(s.var);
			if (!before || (before.value === s.value && before.max === s.max)) continue;
			if (typeof s.value === 'number' && typeof before.value === 'number') {
				const same = s.value === before.value;
				const up = same ? (s.max ?? 0) > (before.max ?? 0) : s.value > before.value;
				// A maximum that is gone has no direction.
				if (!same || s.max !== undefined) marks[s.var] = up ? 'up' : 'down';
				said.push(
					same
						? s.max === undefined
							? `${s.label} has no maximum now.`
							: `${s.label} maximum ${up ? 'went up' : 'went down'} to ${s.max}.`
						: `${s.label} ${up ? 'up' : 'down'} to ${amount(s)}.`
				);
			} else if (typeof s.value === 'boolean') {
				if (s.value) marks[s.var] = 'up';
				said.push(s.value ? `Codeword ${s.label}.` : `Codeword ${s.label} removed.`);
			}
		}
		if (said.length === 0) return;
		changed = marks;
		announcement = said.join(' ');
		clearTimeout(timer);
		timer = setTimeout(() => {
			changed = {};
			announcement = '';
		}, STAT_MARK_MS);
	});
	$effect(() => () => clearTimeout(timer));
</script>

<!--
	The adventure sheet. From 900px (--bp-stack) it shows in full. Below that it is one line, and a tap opens
	the full sheet. A changed value has a short mark with a shape and a colour, and the live region says it.
-->
<section class="sheet" class:open aria-label="Adventure sheet">
	<button type="button" class="line" aria-expanded={open} aria-controls="{id}-full" onclick={() => (open = !open)}>
		<span aria-hidden="true">{line || 'Adventure sheet'}</span>
		<VisuallyHidden>Adventure sheet: {numbers.map(spoken).join(', ')}</VisuallyHidden>
	</button>
	<div class="full" id="{id}-full">
		{#if numbers.length}
			<dl class="numbers">
				{#each numbers as s (s.var)}
					<div class="stat" class:changed={changed[s.var]}>
						<dt>{s.label}</dt>
						<dd>
							{s.value}{#if s.max !== undefined}<span class="of">{` / ${s.max}`}</span>{/if}
							{#if changed[s.var]}<span aria-hidden="true">{changed[s.var] === 'up' ? '▲' : '▼'}</span>{/if}
						</dd>
					</div>
				{/each}
			</dl>
		{/if}
		<p class="list"><span class="head">Equipment</span> {inventory.map((t) => t.name).join(', ') || 'None'}</p>
		{#if codewords.length}
			<p class="list">
				<span class="head">Codewords</span>
				{#each codewords as c, i (c.var)}{i > 0 ? ', ' : ''}<span class:changed={changed[c.var]}
						>{c.label}{#if changed[c.var]}<span aria-hidden="true"> ▲</span>{/if}</span
					>{/each}
			</p>
		{/if}
	</div>
	<div role="status"><VisuallyHidden>{announcement}</VisuallyHidden></div>
</section>

<style>
	.sheet {
		position: relative;
		background: var(--color-surface);
		font-family: var(--font-ui);
		font-size: var(--font-size-md);
		color: var(--color-text);
		text-shadow: var(--glow, none);
	}
	.line {
		display: none;
		width: 100%;
		min-height: var(--size-target);
		padding: 0 var(--space-4);
		border: var(--size-border) solid var(--color-border-control);
		background: var(--color-surface-control);
		color: inherit;
		font: inherit;
		white-space: pre-wrap;
		text-align: left;
		cursor: pointer;
	}
	.line:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-focus);
	}
	.numbers {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2) var(--space-6);
		margin: 0;
	}
	.stat {
		display: flex;
		gap: var(--space-3);
	}
	dt {
		color: var(--color-text-muted);
	}
	dd {
		margin: 0;
		font-weight: var(--font-weight-strong);
	}
	.of {
		font-weight: var(--font-weight-regular);
		color: var(--color-text-muted);
	}
	.changed {
		background: var(--color-accent);
		color: var(--color-text-on-accent);
	}
	.changed dt,
	.changed .of {
		color: inherit;
	}
	.list {
		margin: var(--space-2) 0 0;
	}
	.head {
		color: var(--color-text-muted);
	}

	/* Narrow (--bp-stack): one line that stays at the top; a tap opens the full sheet in place. */
	@media (max-width: 900px) {
		.sheet {
			position: sticky;
			top: 0;
			z-index: var(--z-status);
		}
		/* Stays in view when it opens; a tall sheet scrolls by itself. */
		.sheet.open {
			max-height: 100dvh;
			overflow-y: auto;
		}
		.line {
			display: block;
		}
		.full {
			display: none;
			padding-top: var(--space-4);
		}
		.open .full {
			display: block;
		}
	}
</style>
