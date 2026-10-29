<script lang="ts">
	import { onMount } from 'svelte';
	import { Game, ProjectError, type Diagnostic } from '#lib/engine/index.ts';
	import { lighthouse } from '#lib/engine/example.ts';
	import { settings } from '#lib/settings.svelte.ts';
	import { Session } from '#lib/player/session.svelte.ts';
	import Transcript from '#lib/player/Transcript.svelte';
	import VerbBar from '#lib/player/VerbBar.svelte';
	import AssemblyPanel from '#lib/player/AssemblyPanel.svelte';
	import ScreenEffects from '#lib/player/ScreenEffects.svelte';
	import SettingsPanel from '#lib/player/SettingsPanel.svelte';

	let session: Session | undefined = $state();
	let problems: Diagnostic[] = $state([]);
	let failure = $state('');
	let command = $state('');
	let showSettings = $state(false);
	let hidden = $state(false);
	let input: HTMLInputElement | undefined = $state();

	onMount(() => {
		Game.open(lighthouse)
			.then((game) => (session = new Session(game)))
			.catch((e) => {
				if (e instanceof ProjectError) problems = e.diagnostics;
				else failure = String(e);
			});
		const onVisibility = () => (hidden = document.hidden);
		document.addEventListener('visibilitychange', onVisibility);
		return () => document.removeEventListener('visibilitychange', onVisibility);
	});

	const fxClass = $derived(
		[
			`fx-${settings.effects}`,
			settings.reduceMotion ? 'fx-reduce' : '',
			hidden ? 'fx-paused' : ''
		].join(' ')
	);
	const glow = $derived(
		settings.effects === 'full' ? 'var(--glow-full)' : settings.effects === 'subtle' ? 'var(--glow-soft)' : 'none'
	);

	function submit(e: SubmitEvent) {
		e.preventDefault();
		session?.input(command);
		command = '';
	}
</script>

<div class="app {fxClass}" style:--glow={glow}>
	<header class="top">
		<span class="title">{session?.game.title.toUpperCase() ?? 'LANTERN'}</span>
		<nav aria-label="Views">
			<button class="tab active" aria-current="page">Play and write</button>
			<button class="tab" disabled title="Coming later">Map</button>
			<button class="tab" disabled title="Coming later">Story</button>
			<button class="tab" disabled title="Coming later">Rules</button>
		</nav>
		<div class="lens">
			{#if session}
				<span class="lens-label">now</span>
				<span class="chip">day {session.scene.day} · {session.scene.time} · {session.scene.phase}</span>
				<span class="chip">{session.scene.room_name}</span>
			{/if}
			<button class="chip" aria-expanded={showSettings} onclick={() => (showSettings = !showSettings)}
				>look and motion</button
			>
		</div>
		{#if showSettings}
			<SettingsPanel onclose={() => (showSettings = false)} />
		{/if}
	</header>

	{#if session}
		<div class="body">
			<main aria-label="Player" class="fx-screen">
				<ScreenEffects />
				<div class="player">
					<div class="status">
						<span>turn {session.scene.turns}</span>
						<span>{session.scene.time}</span>
						<span>{session.scene.room}</span>
						<label class="toggle"
							><input type="checkbox" bind:checked={settings.sources} /> sources shown</label
						>
						<button class="link-btn" onclick={() => session?.restart()}>restart</button>
					</div>
					<Transcript {session} />
					{#if !session.scene.ended}
						<form class="prompt" onsubmit={submit}>
							<label for="command" class="visually-hidden">Command</label>
							<span aria-hidden="true">&gt;</span>
							<input
								id="command"
								bind:this={input}
								bind:value={command}
								autocomplete="off"
								spellcheck="false"
								placeholder="type a command, or use the verbs below"
							/>
						</form>
						<VerbBar {session} />
					{:else}
						<p class="ended">The story has ended. <button class="link-btn" onclick={() => session?.restart()}>Play again</button></p>
					{/if}
				</div>
			</main>
			<AssemblyPanel {session} />
		</div>
	{:else if problems.length}
		<main class="problems">
			<h1>The story has problems</h1>
			<ul>
				{#each problems as p, i (i)}
					<li>
						<code>{p.file}{p.line ? `:${p.line}:${p.column}` : ''}</code>
						{p.at ? `(${p.at})` : ''}
						{p.message}
					</li>
				{/each}
			</ul>
		</main>
	{:else if failure}
		<main class="problems"><h1>Could not start the engine</h1><pre>{failure}</pre></main>
	{:else}
		<main class="problems"><p>Loading…</p></main>
	{/if}
</div>

<style>
	.app {
		min-height: 100vh;
		height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--bg);
	}
	.top {
		position: relative;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 14px;
		padding: 10px 20px;
		border-bottom: 1px solid var(--line);
	}
	.title {
		color: var(--accent);
		font-weight: 600;
		letter-spacing: 0.06em;
	}
	nav {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}
	.tab {
		border: 1px solid transparent;
		background: transparent;
		padding: 6px 10px;
		color: var(--muted);
	}
	.tab.active {
		border-color: var(--accent);
		color: var(--text);
	}
	.tab:disabled {
		cursor: default;
		opacity: 0.6;
	}
	.lens {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		flex: 1 1 360px;
		justify-content: flex-end;
		align-items: center;
	}
	.lens-label {
		color: var(--warn);
	}
	.chip {
		border: 1px solid var(--chip-line);
		background: var(--chip);
		padding: 6px 10px;
		color: var(--text);
	}
	.body {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
	}
	main.fx-screen {
		position: relative;
		overflow: hidden;
		flex: 999 1 560px;
		min-width: 0;
		min-height: 0;
		display: flex;
		background: var(--bg);
		box-shadow: var(--screen-inset);
	}
	.player {
		position: relative;
		z-index: 1;
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 24px clamp(16px, 4vw, 48px);
		box-sizing: border-box;
		max-height: 100%;
	}
	.status {
		display: flex;
		flex-wrap: wrap;
		gap: 18px;
		color: var(--muted);
		align-items: center;
	}
	.toggle {
		margin-left: auto;
		display: flex;
		gap: 6px;
		align-items: center;
	}
	.toggle input {
		accent-color: var(--accent);
	}
	.link-btn {
		background: none;
		border: 0;
		padding: 0;
		color: var(--accent);
		text-decoration: underline;
	}
	.prompt {
		display: flex;
		gap: 8px;
		align-items: center;
		font-size: 15px;
		color: var(--muted);
		max-width: 70ch;
	}
	.prompt input {
		flex: 1 1 auto;
		background: transparent;
		border: 0;
		border-bottom: 1px solid var(--line);
		padding: 6px 2px;
		color: var(--strong);
		caret-color: var(--accent);
		font-family: var(--ui);
		font-size: 15px;
	}
	.prompt input:focus-visible {
		outline: none;
		border-bottom-color: var(--accent);
	}
	.ended {
		color: var(--muted);
	}
	.problems {
		padding: 32px;
	}
	.problems h1 {
		font-size: 18px;
		color: var(--warn);
	}
	/* Narrow screens stack the player above the assembly panel and scroll the page. */
	@media (max-width: 900px) {
		.app {
			height: auto;
		}
		.body {
			flex-direction: column;
		}
		main.fx-screen {
			flex: 1 1 auto;
			min-height: 80vh;
		}
	}
</style>
