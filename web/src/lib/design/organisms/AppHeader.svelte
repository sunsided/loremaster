<script lang="ts">
	import IconButton from '../atoms/IconButton.svelte';
	import MenuButton from '../molecules/MenuButton.svelte';
	import ModeSwitch from '../molecules/ModeSwitch.svelte';
	import type { MenuItem } from '../types.ts';

	interface Props {
		/** The story title. It is the label of the project menu. */
		title: string;
		/** Where the changes go, in a few words, for example "saved in browser". */
		saved?: string;
		/** `ok`: the files are safe. `attention`: the author must act, for example give folder access again. */
		savedState?: 'ok' | 'attention';
		/** The actions of the project menu: files, examples. */
		projectItems?: MenuItem[];
		/** Play or write. Set `onmode` to show the switch. */
		mode?: 'play' | 'write';
		onmode?: (mode: 'play' | 'write') => void;
		settingsOpen?: boolean;
		onsettings: () => void;
		/** Opens the command palette. When set, the header shows a button for it. */
		onpalette?: () => void;
	}

	let {
		title,
		saved = '',
		savedState = 'ok',
		projectItems = [],
		mode = 'play',
		onmode,
		settingsOpen = false,
		onsettings,
		onpalette
	}: Props = $props();
</script>

<!--
	The app header: the project menu with the save state, the Play/Write switch, and two icon buttons
	(the command palette and the look and motion settings). The state of the game is not here: it is in
	the status line of the screen.
-->
<header class="app-header">
	<!-- The page heading for screen readers. Sighted users see the title on the project menu. -->
	<h1 class="visually-hidden">{title}</h1>
	<div class="project">
		<MenuButton label="Project: {title}" items={projectItems}><span class="title">{title}</span></MenuButton>
		{#if saved}<span class="saved" class:attention={savedState === 'attention'} title={saved}><span class="saved-text">{saved}</span></span>{/if}
	</div>
	<div class="tools">
		{#if onmode}<ModeSwitch value={mode} onchange={onmode} />{/if}
		{#if onpalette}
			<IconButton label="Commands" shortcut="Ctrl K" aria-haspopup="dialog" onclick={onpalette}>⌘</IconButton>
		{/if}
		<IconButton label="Look and motion" aria-haspopup="dialog" aria-expanded={settingsOpen} onclick={onsettings}>
			<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"
				><circle cx="8" cy="8" r="2.4" /><path
					d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"
				/></svg
			>
		</IconButton>
	</div>
</header>

<style>
	.app-header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3) var(--space-6);
		padding: var(--space-3) var(--space-6);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
		background: var(--color-surface);
	}
	.project {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2) var(--space-4);
		min-width: 0;
	}
	.title {
		font-size: var(--font-size-md);
		color: var(--color-text-accent);
		font-weight: var(--font-weight-strong);
		letter-spacing: var(--letter-spacing-caps);
		text-transform: uppercase;
	}
	.saved {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.saved::before {
		content: '●';
		margin-right: var(--space-2);
		color: var(--color-text-accent);
		font-size: 0.8em;
	}
	.saved.attention,
	.saved.attention::before {
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
	/* On a phone the header stays one row: the save state shows as its dot only. */
	@media (max-width: 599px) {
		.app-header {
			flex-wrap: nowrap;
			padding: var(--space-2) var(--space-4);
		}
		.project {
			flex-wrap: nowrap;
		}
		.saved-text {
			position: absolute;
			width: 1px;
			height: 1px;
			overflow: hidden;
			clip: rect(0 0 0 0);
			white-space: nowrap;
		}
		.title {
			font-size: var(--font-size-sm);
		}
	}
	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
</style>
