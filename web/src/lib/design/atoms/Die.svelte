<script lang="ts">
	interface Props {
		/** The result on the die. A value over 6 shows as a number. */
		face: number;
		/** The face that the ASCII die shows during its roll animation. */
		spin?: number;
	}

	let { face, spin }: Props = $props();

	const PIPS: Record<number, string[]> = {
		1: ['     ', '  o  ', '     '],
		2: ['o    ', '     ', '    o'],
		3: ['o    ', '  o  ', '    o'],
		4: ['o   o', '     ', 'o   o'],
		5: ['o   o', '  o  ', 'o   o'],
		6: ['o   o', 'o   o', 'o   o']
	};
	const DOTS: Record<number, [number, number][]> = {
		1: [[50, 50]],
		2: [[28, 28], [72, 72]],
		3: [[28, 28], [50, 50], [72, 72]],
		4: [[28, 28], [72, 28], [28, 72], [72, 72]],
		5: [[28, 28], [72, 28], [50, 50], [28, 72], [72, 72]],
		6: [[28, 28], [72, 28], [28, 50], [72, 50], [28, 72], [72, 72]]
	};

	const shown = $derived(spin ?? face);
	const art = $derived(
		[
			'.-----.',
			...(PIPS[shown] ?? ['     ', String(shown).padStart(3).padEnd(5), '     ']).map((line) => `|${line}|`),
			"'-----'"
		].join('\n')
	);
</script>

<!-- One die, as decoration: the text of the dice display has the result. The theme selects the look. -->
<pre class="ascii" aria-hidden="true">{art}</pre>
<svg class="drawn" viewBox="0 0 100 100" aria-hidden="true">
	<rect x="6" y="6" width="88" height="88" rx="16" />
	{#if DOTS[face]}
		{#each DOTS[face] as [cx, cy], i (i)}<circle {cx} {cy} r="9" />{/each}
	{:else}
		<text x="50" y="66" text-anchor="middle">{face}</text>
	{/if}
</svg>

<style>
	.ascii {
		margin: 0;
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-tight);
		color: var(--color-text-accent);
		text-shadow: var(--glow, none);
	}
	.drawn {
		display: none;
		width: var(--size-target);
		height: var(--size-target);
		fill: var(--color-surface-control);
		stroke: var(--color-text);
		stroke-width: var(--size-border-strong);
	}
	.drawn circle,
	.drawn text {
		fill: var(--color-text);
		stroke: none;
	}
	.drawn text {
		font-family: var(--font-ui);
		font-size: 42px;
	}
	:global([data-theme='paper']) .ascii,
	:global([data-theme='night']) .ascii {
		display: none;
	}
	:global([data-theme='paper']) .drawn,
	:global([data-theme='night']) .drawn {
		display: block;
	}
</style>
