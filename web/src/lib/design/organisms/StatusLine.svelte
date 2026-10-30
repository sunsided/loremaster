<script lang="ts">
	import type { Snippet } from 'svelte';
	import MenuButton from '../molecules/MenuButton.svelte';
	import type { MenuItem } from '../types.ts';

	interface Props {
		/** The name of the current room. */
		room: string;
		/** The level of the room. Leave it out for a story on one level. */
		level?: number | null;
		day: number;
		/** The time, for example "20:30". */
		time: string;
		/** The phase of the day, for example "dusk". */
		phase: string;
		turn: number;
		/** The actions of the turn menu, for example "Restart the game". */
		turnItems: MenuItem[];
		/** Short marks after the room, for example a running AI job or a preview. */
		marks?: Snippet;
		/** A click on the room: open the world view (the Atlas). */
		onroom?: () => void;
	}

	let { room, level = null, day, time, phase, turn, turnItems, marks, onroom }: Props = $props();
</script>

<!--
	The status line on top of the screen, in inverse video, like the Infocom games:
	the room (and its level) on the left, the day, the time and the turn on the right.
	It is the one place for the state of the game. "Turn N ▾" opens the turn menu.
-->
<div class="status-line" role="group" aria-label="Game status">
	{#if onroom}<button type="button" class="room room-button" aria-keyshortcuts="Control+M" title="Open the Atlas (Ctrl+M)" onclick={onroom}
			>{room}</button
		>{:else}<span class="room">{room}</span>{/if}
	{#if level !== null}<span class="level">level {level}</span>{/if}
	{#if marks}<span class="marks">{@render marks()}</span>{/if}
	<span class="right">
		<span><span class="wide">Day {day} · </span>{time} · {phase}</span>
		<MenuButton label="Turn {turn}: game actions" items={turnItems} align="end" look="inverse"
			><span class="wide">Turn </span><span class="narrow" aria-hidden="true">T</span>{turn}</MenuButton
		>
	</span>
</div>

<style>
	.status-line {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1) var(--space-6);
		padding: 0 var(--space-5);
		background: var(--color-status-bg);
		color: var(--color-status-text);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-strong);
		text-shadow: none;
	}
	.room {
		min-height: var(--size-target-compact);
		display: inline-flex;
		align-items: center;
	}
	.room-button {
		padding: 0 var(--space-2);
		margin-left: calc(-1 * var(--space-2));
		border: var(--size-border) solid transparent;
		background: none;
		color: inherit;
		font: inherit;
	}
	.room-button:hover {
		border-color: var(--color-status-text);
	}
	.room-button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-status-text);
		outline-offset: 1px;
	}
	.level {
		font-weight: var(--font-weight-regular);
	}
	.marks {
		display: inline-flex;
		gap: var(--space-3);
	}
	.narrow {
		display: none;
	}
	/* On a phone the line stays one row: the day and the word "Turn" go. */
	@media (max-width: 599px) {
		.status-line {
			flex-wrap: nowrap;
			gap: var(--space-4);
			padding: 0 var(--space-4);
		}
		.room {
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
			display: block;
			line-height: var(--size-target-compact);
			min-width: 0;
		}
		.wide,
		.room-button {
		padding: 0 var(--space-2);
		margin-left: calc(-1 * var(--space-2));
		border: var(--size-border) solid transparent;
		background: none;
		color: inherit;
		font: inherit;
	}
	.room-button:hover {
		border-color: var(--color-status-text);
	}
	.room-button:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-status-text);
		outline-offset: 1px;
	}
	.level {
			display: none;
		}
		.narrow {
			display: inline;
		}
		.right {
			gap: var(--space-3);
			flex: none;
		}
	}
	.right {
		display: inline-flex;
		align-items: center;
		gap: var(--space-5);
		margin-left: auto;
	}
</style>
