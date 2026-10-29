<script lang="ts">
	import type { SourcePiece } from '#lib/engine/types.ts';

	interface Props {
		/** The pieces of one variant, from the runtime. */
		pieces: SourcePiece[];
	}

	let { pieces }: Props = $props();
</script>

<!--
	The source text of a fragment, with its inline markup.
	- Markup (braces, conditions, separators) is muted.
	- A branch whose condition does not hold now is struck through.
	- An insert such as {time.phase} shows its current value after it.
	Screen readers hear "not now:" before a struck-through piece, because they do not announce the line.
	No whitespace between tags: a gap would show in the text.
-->
<span class="source">{#each pieces as piece, i (i)}{#if !piece.active}<s class="piece {piece.role}"
				><span class="visually-hidden">not now: </span>{piece.text}</s
			>{:else if piece.role === 'insert'}<span class="piece insert">{piece.text}</span><span class="value"
				><span class="visually-hidden">, now </span>{piece.value}</span
			>{:else}<span class="piece {piece.role}">{piece.text}</span>{/if}{/each}</span>

<style>
	.source {
		font-family: var(--font-prose);
		overflow-wrap: anywhere;
	}
	.syntax,
	.insert {
		font-family: var(--font-ui);
		font-size: 0.9em;
		color: var(--color-text-muted);
	}
	.value {
		margin-left: var(--space-1);
		padding: 0 var(--space-2);
		border: var(--size-border) solid var(--color-border-subtle);
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	s {
		color: var(--color-text-muted);
		text-decoration: line-through;
		text-decoration-color: var(--color-text-warning);
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
