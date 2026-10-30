<script lang="ts">
	import Button from '../atoms/Button.svelte';
	import Tag from '../atoms/Tag.svelte';
	import type { View } from '../types.ts';

	interface Props {
		title: string;
		/** Top-level views. Leave empty when the page has only one view. */
		views?: View[];
		current?: string;
		/** Facts about the game now, for example the time and the room. */
		now?: string[];
		settingsOpen?: boolean;
		onview?: (id: string) => void;
		onsettings: () => void;
		/** Opens the command palette. When set, the header shows a button for it. */
		onpalette?: () => void;
		/** True in write mode. */
		writing?: boolean;
		/** Turns write mode on or off. When set, the header shows a "Write" toggle. */
		onwrite?: () => void;
	}

	let { title, views = [], current = '', now = [], settingsOpen = false, onview, onsettings, onpalette, writing = false, onwrite }: Props = $props();
</script>

<header class="app-header">
	<h1 class="title">{title}</h1>
	{#if views.length}
	<nav aria-label="Views">
		<ul>
			{#each views as view (view.id)}
				<li>
					<Button
						size="compact"
						variant="quiet"
						disabled={view.disabled}
						aria-current={view.id === current ? 'page' : undefined}
						title={view.disabled ? 'Not available yet' : undefined}
						onclick={() => onview?.(view.id)}>{view.label}</Button
					>
				</li>
			{/each}
		</ul>
	</nav>
	{/if}
	<div class="now">
		{#if now.length}
			<span class="now-label">now</span>
			{#each now as fact, i (i)}<Tag>{fact}</Tag>{/each}
		{/if}
		{#if onwrite}
			<Button size="compact" variant={writing ? 'primary' : 'secondary'} pressed={writing} aria-keyshortcuts="Control+E Meta+E" onclick={onwrite}
				>Write <kbd>Ctrl E</kbd></Button
			>
		{/if}
		{#if onpalette}
			<Button size="compact" aria-haspopup="dialog" aria-keyshortcuts="Control+K Meta+K" onclick={onpalette}
				>Commands <kbd>Ctrl K</kbd></Button
			>
		{/if}
		<Button size="compact" aria-haspopup="dialog" aria-expanded={settingsOpen} onclick={onsettings}>Look and motion</Button>
	</div>
</header>

<style>
	.app-header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-4) var(--space-6);
		padding: var(--space-4) var(--space-7);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
		background: var(--color-surface);
	}
	.title {
		margin: 0;
		font-size: var(--font-size-md);
		color: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
	}
	ul {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	nav :global([aria-current='page']) {
		border-color: var(--color-accent);
	}
	.now {
		display: flex;
		flex: 1 1 320px;
		flex-wrap: wrap;
		justify-content: flex-end;
		align-items: center;
		gap: var(--space-3);
	}
	kbd {
		padding: 0 var(--space-2);
		border: var(--size-border) solid currentColor;
		font-family: var(--font-ui);
		font-size: var(--font-size-xs);
		color: inherit;
		opacity: 0.75;
	}
	.now-label {
		color: var(--color-text-warning);
	}
</style>
