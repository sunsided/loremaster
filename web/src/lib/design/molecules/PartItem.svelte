<script lang="ts">
	interface Props {
		/** The thing name, or the direction and the room it leads to. */
		label: string;
		/** The presence text of a thing, or the generated text. */
		text: string;
		/** More facts, for example "door: cottage door, closed". */
		detail?: string | null;
		/** True when the line is part of the room text now. */
		shown: boolean;
		/** Why the line is not in the room text. Only used when `shown` is false. */
		why?: string | null;
		/** What the line is part of, for screen readers, for example "room text" or "response". */
		within?: string;
	}

	let { label, text, detail, shown, why, within = 'room text' }: Props = $props();
</script>

<!--
	One thing or one exit in a generated layer of the assembly panel.
	A shown line has a check mark. A line that is not in the room text has a dash, struck-through text and the reason.
	The state is also in text for screen readers.
-->
<li class="part" class:shown>
	<span class="mark" aria-hidden="true">{shown ? '✓' : '–'}</span>
	<div>
		<span class="visually-hidden">{shown ? `In the ${within}` : `Not in the ${within}`}:</span>
		<span class="label">{label}</span>
		{#if text !== label}<span class="text">{text}</span>{/if}
		<span class="meta"
			>{#if detail}{detail}{/if}{#if detail && !shown && why}{' · '}{/if}{#if !shown && why}<span class="why"
					>{why}</span
				>{/if}</span
		>
	</div>
</li>

<style>
	.part {
		display: grid;
		grid-template-columns: 22px minmax(0, 1fr);
		gap: var(--space-4);
		padding: var(--space-3) var(--space-4);
		color: var(--color-text-muted);
	}
	.shown {
		color: var(--color-text);
	}
	.mark {
		color: var(--color-text-warning);
	}
	.label {
		display: block;
		font-weight: var(--font-weight-strong);
	}
	.text {
		display: block;
		font-family: var(--font-prose);
		line-height: var(--line-height-body);
		overflow-wrap: anywhere;
	}
	.part:not(.shown) .text {
		text-decoration: line-through;
		text-decoration-color: var(--color-border-control);
	}
	.meta {
		display: block;
		margin-top: var(--space-1);
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.meta:empty {
		display: none;
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
