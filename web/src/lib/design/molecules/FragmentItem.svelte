<script lang="ts">
	interface Props {
		/** The fragment number, starting at 1. */
		number: number;
		/** The source text of the fragment, with its markup. */
		text: string;
		/** Mode, condition and other facts, for example "always · when night". */
		meta: string;
		/** True when the player sees this fragment now. */
		shown: boolean;
		/** Why the fragment is hidden. Only used when `shown` is false. */
		why?: string | null;
		/** True when the fragment is highlighted. */
		active?: boolean;
		onactivate?: (on: boolean) => void;
	}

	let { number, text, meta, shown, why, active = false, onactivate }: Props = $props();
</script>

<!--
	One fragment in the assembly panel. A shown fragment has its number.
	A hidden fragment has a dash, struck-through text and the reason.
	The state is also in text for screen readers, not in colour only.
-->
<li
	class="fragment"
	class:shown
	class:active
	onmouseenter={() => onactivate?.(true)}
	onmouseleave={() => onactivate?.(false)}
>
	<span class="number" aria-hidden="true">{shown ? number : '–'}</span>
	<div>
		<span class="visually-hidden">Fragment {number}, {shown ? 'shown' : 'hidden'}:</span>
		<span class="text">{text}</span>
		<span class="meta"
			>{meta}{#if !shown && why}{' · '}<span class="why">{why}</span>{/if}</span
		>
	</div>
</li>

<style>
	.fragment {
		display: grid;
		grid-template-columns: 22px minmax(0, 1fr);
		gap: var(--space-4);
		padding: var(--space-3) var(--space-4);
		border: var(--size-border) solid transparent;
		color: var(--color-text-muted);
	}
	.shown {
		color: var(--color-text);
	}
	.active {
		border-color: var(--color-warning);
	}
	.number {
		color: var(--color-text-warning);
	}
	.text {
		display: block;
		font-family: var(--font-prose);
		font-size: var(--font-size-md);
		line-height: var(--line-height-body);
		overflow-wrap: anywhere;
	}
	.fragment:not(.shown) .text {
		text-decoration: line-through;
		text-decoration-color: var(--color-border-control);
	}
	.meta {
		display: block;
		margin-top: var(--space-1);
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.why {
		color: var(--color-text-warning);
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
