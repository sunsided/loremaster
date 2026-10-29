<script lang="ts">
	import { onMount } from 'svelte';
	import { Game, ProjectError, type Diagnostic } from '#lib/engine/index.ts';
	import { lighthouse } from '#lib/engine/example.ts';
	import { settings } from '#lib/settings.svelte.ts';
	import { Session } from '#lib/player/session.svelte.ts';
	import Button from '#lib/design/atoms/Button.svelte';
	import Checkbox from '#lib/design/atoms/Checkbox.svelte';
	import CommandPrompt from '#lib/design/molecules/CommandPrompt.svelte';
	import StatusBar from '#lib/design/molecules/StatusBar.svelte';
	import AppHeader from '#lib/design/organisms/AppHeader.svelte';
	import AssemblyPanel from '#lib/design/organisms/AssemblyPanel.svelte';
	import ScreenEffects from '#lib/design/organisms/ScreenEffects.svelte';
	import SettingsDialog from '#lib/design/organisms/SettingsDialog.svelte';
	import Transcript from '#lib/design/organisms/Transcript.svelte';
	import VerbBar from '#lib/design/organisms/VerbBar.svelte';
	import PlayerLayout from '#lib/design/templates/PlayerLayout.svelte';
	import type { View } from '#lib/design/types.ts';

	let session: Session | undefined = $state();
	let problems: Diagnostic[] = $state([]);
	let failure = $state('');
	let showSettings = $state(false);
	let hidden = $state(false);

	const views: View[] = [
		{ id: 'play', label: 'Play and write' },
		{ id: 'map', label: 'Map', disabled: true },
		{ id: 'story', label: 'Story', disabled: true },
		{ id: 'rules', label: 'Rules', disabled: true }
	];

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
		[`fx-${settings.effects}`, settings.reduceMotion ? 'fx-reduce' : '', hidden ? 'fx-paused' : ''].join(' ')
	);
	const glow = $derived(
		settings.effects === 'full' ? 'var(--fx-glow-full)' : settings.effects === 'subtle' ? 'var(--fx-glow-soft)' : 'none'
	);
</script>

<svelte:head>
	<title>{session ? `${session.game.title} · lantern` : 'lantern'}</title>
</svelte:head>

{#if session}
	{@const s = session}
	<PlayerLayout {fxClass} {glow}>
		{#snippet header()}
			<AppHeader
				title={s.game.title}
				{views}
				current="play"
				now={[`day ${s.scene.day} · ${s.scene.time} · ${s.scene.phase}`, s.scene.room_name]}
				settingsOpen={showSettings}
				onsettings={() => (showSettings = true)}
			/>
		{/snippet}
		{#snippet effects()}
			<ScreenEffects theme={settings.theme} effects={settings.effects} />
		{/snippet}
		{#snippet player()}
			<StatusBar items={[`turn ${s.scene.turns}`, s.scene.time, s.scene.room]}>
				{#snippet actions()}
					<Checkbox bind:checked={settings.sources}>Show sources</Checkbox>
					<Button size="compact" variant="quiet" onclick={() => s.restart()}>Restart</Button>
				{/snippet}
			</StatusBar>
			<Transcript
				entries={s.entries}
				room={s.scene.room}
				showSources={settings.sources}
				focus={s.focus}
				effects={settings.effects}
				reduceMotion={settings.reduceMotion}
				onthing={(id, name) => s.pickThing(id, name)}
				onexit={(dir) => s.pickExit(dir)}
				onfocus={(i) => (s.focus = i)}
			/>
			{#if !s.scene.ended}
				<CommandPrompt oncommand={(text) => s.input(text)} />
				<VerbBar
					verbs={s.barVerbs}
					selected={s.pending?.verb.id ?? null}
					sentence={s.sentence}
					canRunPartial={s.pending?.verb.slots === 'two' && s.pending.objects.length === 1}
					things={s.scene.things}
					inventory={s.scene.inventory}
					exits={s.scene.exits}
					onverb={(v) => s.pickVerb(v)}
					onthing={(id, name) => s.pickThing(id, name)}
					onexit={(dir) => s.pickExit(dir)}
					onrun={() => s.runPending()}
					oncancel={() => (s.pending = null)}
				/>
			{:else}
				<p id="command" tabindex="-1">The story has ended. <Button size="compact" onclick={() => s.restart()}>Play again</Button></p>
			{/if}
		{/snippet}
		{#snippet panel()}
			<AssemblyPanel
				room={s.scene.room}
				fragments={s.assembly}
				focus={s.focus}
				time={s.scene.time}
				phase={s.scene.phase}
				onfocus={(i) => (s.focus = i)}
			/>
		{/snippet}
	</PlayerLayout>
	<SettingsDialog
		bind:open={showSettings}
		bind:theme={settings.theme}
		bind:effects={settings.effects}
		bind:motion={settings.motion}
		bind:sources={settings.sources}
		systemReduce={settings.systemReduce}
	/>
{:else}
	<main class="message">
		{#if problems.length}
			<h1>The story has problems</h1>
			<ul>
				{#each problems as p, i (i)}
					<li><code>{p.file}{p.line ? `:${p.line}:${p.column}` : ''}</code> {p.at ? `(${p.at})` : ''} {p.message}</li>
				{/each}
			</ul>
		{:else if failure}
			<h1>The engine did not start</h1>
			<pre>{failure}</pre>
		{:else}
			<p role="status">Loading the story…</p>
		{/if}
	</main>
{/if}

<style>
	.message {
		padding: var(--space-9);
	}
	h1 {
		font-size: var(--font-size-heading);
		color: var(--color-text-warning);
	}
</style>
