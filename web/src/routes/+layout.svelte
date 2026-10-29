<script lang="ts">
	import '../app.css';
	import { settings } from '#lib/settings.svelte.ts';

	let { children } = $props();

	$effect(() => {
		document.documentElement.dataset.theme = settings.theme;
	});
</script>

<svelte:head>
	<title>lantern</title>
</svelte:head>

{@render children()}
