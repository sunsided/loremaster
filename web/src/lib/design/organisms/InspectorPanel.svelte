<script lang="ts">
	import type { Snippet } from 'svelte';
	import TabList from '../molecules/TabList.svelte';
	import type { TabItem } from '../types.ts';

	interface Props {
		tabs: TabItem[];
		selected: string;
		/** The content of the selected tab. */
		children: Snippet;
	}

	let { tabs, selected = $bindable(), children }: Props = $props();
</script>

<!--
	The side panel next to the player. Its tabs select one inspector: Assembly, Map, Story or Rules.
	The player stays visible for all tabs. The tab panel scrolls on its own.
-->
<section class="inspector" aria-label="Inspector">
	<div class="tabs"><TabList label="Inspector" {tabs} bind:selected idBase="inspector" /></div>
	<!-- The tab panel scrolls, so keyboard users must be able to focus it. This is also the WAI-ARIA tabs pattern. -->
	<div
		class="panel"
		role="tabpanel"
		id="inspector-panel"
		aria-labelledby="inspector-tab-{selected}"
		tabindex="0"
	>
		{@render children()}
	</div>
</section>

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--color-surface-raised);
	}
	.tabs {
		padding: var(--space-4) var(--space-7) 0;
	}
	.panel {
		/* Contains absolutely positioned screen-reader text, so it scrolls with the panel and does not
		   make the page taller. */
		position: relative;
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		padding: var(--space-7);
	}
	.panel:focus-visible {
		outline-offset: -2px;
	}
</style>
