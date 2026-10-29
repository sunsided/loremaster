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
	import MapPanel from '#lib/design/organisms/MapPanel.svelte';
	import ScreenEffects from '#lib/design/organisms/ScreenEffects.svelte';
	import SettingsDialog from '#lib/design/organisms/SettingsDialog.svelte';
	import Transcript from '#lib/design/organisms/Transcript.svelte';
	import VerbBar from '#lib/design/organisms/VerbBar.svelte';
	import PlayerLayout from '#lib/design/templates/PlayerLayout.svelte';
	import InspectorPanel from '#lib/design/organisms/InspectorPanel.svelte';
	import CommandPalette from '#lib/design/organisms/CommandPalette.svelte';
	import RulesView from '#lib/design/organisms/RulesView.svelte';
	import StoryTimeline from '#lib/design/organisms/StoryTimeline.svelte';
	import type { PaletteItem, TabItem } from '#lib/design/types.ts';

	let session: Session | undefined = $state();
	let problems: Diagnostic[] = $state([]);
	let failure = $state('');
	let showSettings = $state(false);
	let prompt: HTMLInputElement | undefined = $state();
	let showPalette = $state(false);

	/** A rule for the Rules tab to show and focus. */
	let revealRule = $state<string | null>(null);

	function showRule(id: string) {
		inspector = 'rules';
		revealRule = id;
	}

	function backToPrompt() {
		prompt?.focus();
	}

	/** Everything the command palette can do, built from the game state now. */
	const paletteItems = $derived.by<PaletteItem[]>(() => {
		const s = session;
		if (!s) return [];
		const items: PaletteItem[] = [];
		for (const tab of inspectorTabs) {
			items.push({ id: `tab-${tab.id}`, group: 'Inspector', label: `Show ${tab.label}`, run: () => (inspector = tab.id) });
		}
		for (const room of s.map.rooms) {
			items.push({
				id: `room-${room.id}`,
				group: 'Rooms',
				label: `Go to ${room.name}`,
				hint: room.current ? 'you are here' : room.visited ? 'visited' : 'not visited',
				keywords: room.id,
				run: () => {
					s.author({ type: 'goto', room: room.id });
					backToPrompt();
				}
			});
		}
		for (const rule of s.rules) {
			items.push({
				id: `rule-${rule.id}`,
				group: 'Rules',
				label: `Show rule ${rule.id}`,
				hint: rule.fired ? 'fired' : rule.when,
				keywords: `${rule.when} ${rule.condition ?? ''} ${rule.then.join(' ')}`,
				run: () => showRule(rule.id)
			});
		}
		for (const v of s.vars) {
			if (typeof v.value === 'boolean') {
				items.push({
					id: `var-${v.name}`,
					group: 'States',
					label: `Set ${v.name} to ${!v.value}`,
					hint: `now ${v.value}`,
					keywords: 'toggle flag',
					run: () => s.author({ type: 'set', name: v.name, value: !v.value })
				});
			} else if (v.options.length) {
				for (const option of v.options.filter((o) => o !== v.value)) {
					items.push({
						id: `var-${v.name}-${option}`,
						group: 'States',
						label: `Set ${v.name} to ${option}`,
						hint: `now ${v.value}`,
						run: () => s.author({ type: 'set', name: v.name, value: option })
					});
				}
			}
		}
		for (const phase of s.phases) {
			items.push({
				id: `time-${phase.name}-${phase.from}`,
				group: 'Time',
				label: `Set the time to ${phase.name} (${phase.from})`,
				hint: s.scene.phase === phase.name ? 'now' : undefined,
				keywords: 'clock phase',
				run: () => s.author({ type: 'time', time: phase.from })
			});
		}
		const themes = ['phosphor', 'night', 'paper'] as const;
		for (const theme of themes.filter((t) => t !== settings.theme)) {
			items.push({ id: `theme-${theme}`, group: 'Look', label: `Use the ${theme} theme`, run: () => (settings.theme = theme) });
		}
		for (const level of (['full', 'subtle', 'off'] as const).filter((l) => l !== settings.effects)) {
			items.push({ id: `fx-${level}`, group: 'Look', label: `Effects: ${level}`, run: () => (settings.effects = level) });
		}
		items.push({
			id: 'sources',
			group: 'Look',
			label: settings.sources ? 'Hide fragment numbers' : 'Show fragment numbers',
			run: () => (settings.sources = !settings.sources)
		});
		items.push({ id: 'settings', group: 'Look', label: 'Open look and motion settings', run: () => (showSettings = true) });
		items.push({ id: 'restart', group: 'Game', label: 'Restart the game', run: () => s.restart() });
		return items;
	});

	function playFallback(query: string): PaletteItem | null {
		const s = session;
		if (!s) return null;
		return {
			id: 'play-query',
			group: 'Play',
			label: `Do “${query}”`,
			hint: 'as a typed command',
			run: () => {
				s.input(query);
				backToPrompt();
			}
		};
	}

	function onWindowKeydown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
			event.preventDefault();
			if (!showSettings) showPalette = !showPalette;
		}
	}
	let hidden = $state(false);

	// On ultrawide screens the map has its own column, so the inspector does not repeat it.
	let ultrawide = $state(false);
	let inspector = $state('assembly');
	const inspectorTabs = $derived<TabItem[]>([
		{ id: 'assembly', label: 'Assembly' },
		...(ultrawide ? [] : [{ id: 'map', label: 'Map' }]),
		{ id: 'story', label: 'Story' },
		{ id: 'rules', label: 'Rules' }
	]);
	$effect(() => {
		if (ultrawide && inspector === 'map') inspector = 'assembly';
	});

	onMount(() => {
		Game.open(lighthouse)
			.then((game) => (session = new Session(game)))
			.catch((e) => {
				if (e instanceof ProjectError) problems = e.diagnostics;
				else failure = String(e);
			});
		// Same query as --bp-ultrawide in PlayerLayout.
		const wide = window.matchMedia('(min-width: 2200px) and (min-aspect-ratio: 21/9)');
		ultrawide = wide.matches;
		const onWide = () => (ultrawide = wide.matches);
		wide.addEventListener('change', onWide);
		const onVisibility = () => (hidden = document.hidden);
		document.addEventListener('visibilitychange', onVisibility);
		return () => {
			document.removeEventListener('visibilitychange', onVisibility);
			wide.removeEventListener('change', onWide);
		};
	});

	const fxClass = $derived(
		[`fx-${settings.effects}`, settings.reduceMotion ? 'fx-reduce' : '', hidden ? 'fx-paused' : ''].join(' ')
	);
	const glow = $derived(
		settings.effects === 'full' ? 'var(--fx-glow-full)' : settings.effects === 'subtle' ? 'var(--fx-glow-soft)' : 'none'
	);
</script>

<svelte:window onkeydown={onWindowKeydown} />

<svelte:head>
	<title>{session ? `${session.game.title} · lantern` : 'lantern'}</title>
</svelte:head>

{#if session}
	{@const s = session}
	<PlayerLayout {fxClass} {glow}>
		{#snippet header()}
			<AppHeader
				title={s.game.title}
				now={[`day ${s.scene.day} · ${s.scene.time} · ${s.scene.phase}`, s.scene.room_name]}
				settingsOpen={showSettings}
				onsettings={() => (showSettings = true)}
				onpalette={() => (showPalette = true)}
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
				<CommandPrompt bind:value={s.draft} bind:input={prompt} oncommand={(text) => s.input(text)} />
				<VerbBar
					verbs={s.barVerbs}
					selected={s.preview.verb}
					sentence={s.sentence}
					canRunPartial={s.canRunPartial}
					marked={s.named}
					markedExit={s.preview.direction}
					things={s.scene.things}
					inventory={s.scene.inventory}
					exits={s.scene.exits}
					onverb={(v) => {
						s.pickVerb(v);
						prompt?.focus();
					}}
					onthing={(id, name) => s.pickThing(id, name)}
					onexit={(dir) => s.pickExit(dir)}
					onrun={() => s.runPending()}
					oncancel={() => (s.draft = '')}
				/>
			{:else}
				<p id="command" tabindex="-1">The story has ended. <Button size="compact" onclick={() => s.restart()}>Play again</Button></p>
			{/if}
		{/snippet}
		{#snippet map()}
			<MapPanel map={s.map} framed />
		{/snippet}
		{#snippet panel()}
			<InspectorPanel tabs={inspectorTabs} bind:selected={inspector}>
				{#if inspector === 'map'}
					<MapPanel map={s.map} />
				{:else if inspector === 'story'}
					<StoryTimeline steps={s.history} />
				{:else if inspector === 'rules'}
					<RulesView rules={s.rules} reveal={revealRule} onrevealed={() => (revealRule = null)} />
				{:else}
					<AssemblyPanel
						room={s.scene.room}
						fragments={s.assembly}
						parts={s.parts}
						focus={s.focus}
						time={s.scene.time}
						phase={s.scene.phase}
						onfocus={(i) => (s.focus = i)}
					/>
				{/if}
			</InspectorPanel>
		{/snippet}
	</PlayerLayout>
	<CommandPalette bind:open={showPalette} items={paletteItems} fallback={playFallback} />
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
