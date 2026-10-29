<script lang="ts">
	import { onMount } from 'svelte';
	import { Workspace } from '#lib/project/workspace.svelte.ts';
	import { canLinkFolder } from '#lib/project/store.ts';
	import { emptyFragment, insertFragment, removeFragment, writeFragment } from '#lib/project/yaml-edit.ts';
	import FragmentWriter from '#lib/design/organisms/FragmentWriter.svelte';
	import type { Source } from '#lib/engine/types.ts';
	import type { WriterFragment, WriterTools, WritingAt } from '#lib/design/types.ts';
	import { examples } from '#lib/engine/example.ts';
	import { settings } from '#lib/settings.svelte.ts';
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
	import AuthorTooltip from '#lib/design/organisms/AuthorTooltip.svelte';
	import RulesView from '#lib/design/organisms/RulesView.svelte';
	import ExplorerView from '#lib/design/organisms/ExplorerView.svelte';
	import type { Exploration } from '#lib/engine/types.ts';
	import StoryTimeline from '#lib/design/organisms/StoryTimeline.svelte';
	import type { PaletteItem, TabItem } from '#lib/design/types.ts';

	let workspace: Workspace | undefined = $state();
	const session = $derived(workspace?.session ?? undefined);
	const STORY_KEY = 'lantern.story';
	let storyName = $state(readStory());

	function readStory(): string {
		try {
			const name = localStorage.getItem(STORY_KEY);
			return name && name in examples ? name : 'lighthouse';
		} catch {
			return 'lighthouse';
		}
	}

	/** Load an example story and start a new session with it. */
	function openStory(name: string) {
		storyName = name;
		try {
			localStorage.setItem(STORY_KEY, name);
		} catch {
			// Storage is not available: the choice lasts for this visit only.
		}
		failure = '';
		exploration = null;
		workspace = undefined;
		Workspace.open(name, examples[name])
			.then((ws) => (workspace = ws))
			.catch((e) => (failure = String(e)));
	}
	const problems = $derived(workspace?.problems ?? []);
	// In development, the workspace is on `window.lantern` for experiments in the browser console.
	$effect(() => {
		if (import.meta.env.DEV) (window as unknown as { lantern?: Workspace }).lantern = workspace;
	});
	let failure = $state('');
	let zipInput: HTMLInputElement | undefined = $state();

	/** Run a file action. A cancelled folder picker is not an error. */
	function fileAction(action: () => Promise<unknown>) {
		action().catch((e) => {
			if (e instanceof DOMException && e.name === 'AbortError') return;
			failure = String(e);
		});
	}

	function exportZip(ws: Workspace) {
		const blob = new Blob([ws.project.zip() as BlobPart], { type: 'application/zip' });
		const a = document.createElement('a');
		a.href = URL.createObjectURL(blob);
		a.download = `${ws.project.name}.zip`;
		a.click();
		setTimeout(() => URL.revokeObjectURL(a.href), 1000);
	}

	/** Where the changes go, in a few words for the header. */
	const storageLabel = $derived.by(() => {
		const p = workspace?.project;
		if (!p) return '';
		switch (p.storage) {
			case 'folder':
				return `saved to folder “${p.folderName}”`;
			case 'folder-locked':
				return 'folder needs access';
			case 'browser':
				return p.edited ? 'edited, saved in browser' : 'bundled example';
			default:
				return p.edited ? 'edited, not saved' : 'bundled example';
		}
	});
	let showSettings = $state(false);
	let prompt: HTMLInputElement | undefined = $state();
	let showPalette = $state(false);
	/** The kind of thing or exit under the pointer or with focus, for the verb hint. */
	let hovered = $state<'thing' | 'exit' | null>(null);

	function inspectKind(target: EventTarget | null): 'thing' | 'exit' | null {
		const el = target instanceof Element ? target.closest<HTMLElement>('[data-inspect]') : null;
		const kind = el?.dataset.inspect?.split(':')[0];
		return kind === 'thing' || kind === 'exit' ? kind : null;
	}

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
		items.push({
			id: 'tooltips',
			group: 'Look',
			label: settings.tooltips ? 'Hide author tooltips' : 'Show author tooltips',
			run: () => (settings.tooltips = !settings.tooltips)
		});
		items.push({ id: 'settings', group: 'Look', label: 'Open look and motion settings', run: () => (showSettings = true) });
		items.push({ id: 'restart', group: 'Game', label: 'Restart the game', run: () => s.restart() });
		const ws = workspace!;
		const p = ws.project;
		if (canLinkFolder()) {
			if (p.storage === 'folder-locked')
				items.push({ id: 'files-reconnect', group: 'Files', label: `Give access to the folder “${p.folderName}” again`, run: () => fileAction(() => ws.reconnect()) });
			if (p.storage === 'folder')
				items.push({ id: 'files-unlink', group: 'Files', label: `Stop saving to the folder “${p.folderName}”`, run: () => fileAction(() => ws.unlinkFolder()) });
			else
				items.push({ id: 'files-link', group: 'Files', label: 'Open a story folder on disk', keywords: 'link save directory', run: () => fileAction(() => ws.linkFolder()) });
		}
		items.push({ id: 'files-export', group: 'Files', label: 'Export the story as a zip file', keywords: 'download save', run: () => exportZip(ws) });
		items.push({ id: 'files-import', group: 'Files', label: 'Import a story from a zip file', keywords: 'upload load', run: () => zipInput?.click() });
		if (p.edited || p.storage === 'folder')
			items.push({ id: 'files-reset', group: 'Files', label: `Go back to the bundled example “${p.name}”`, keywords: 'discard revert', run: () => fileAction(() => ws.reset()) });
		for (const name of Object.keys(examples).filter((n) => n !== storyName)) {
			items.push({
				id: `story-${name}`,
				group: 'Game',
				label: `Open the example “${name}”`,
				keywords: 'story switch load',
				run: () => openStory(name)
			});
		}
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
		// W toggles write mode, but not while the author types somewhere.
		const t = event.target as HTMLElement | null;
		const typing = !!t && (t.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName));
		if (event.key.toLowerCase() === 'w' && !event.ctrlKey && !event.metaKey && !event.altKey && !typing && !showPalette && !showSettings) {
			event.preventDefault();
			toggleWriting();
		}
	}

	// Write mode: the room text of the newest turn that shows a room is editable in place.
	let writing = $state(false);
	let writingAt = $state<WritingAt | null>(null);
	let writingVariant = $state(0);

	function toggleWriting() {
		writing = !writing;
		writingAt = null;
	}

	/** The entry with the room text that the author can edit: the newest turn that showed a room. */
	const writableEntry = $derived.by(() => {
		if (!writing || !session) return null;
		return session.entries.findLast((e) => e.turn.blocks.some((b) => b.kind === 'title'))?.id ?? null;
	});

	const writerTools = $derived.by<WriterTools | null>(() => {
		const s = session;
		if (!s) return null;
		const game = s.game;
		const problem = (e: { start: number; end: number; message: string } | null) =>
			e ? { from: e.start, to: e.end, message: e.message } : null;
		s.entries.length; // The values change with each turn.
		return {
			symbols: game.symbols(),
			textMatrix: (text, when) => game.textMatrix(text, when),
			tokenize: (text, condition) => game.tokens(text, condition) as ReturnType<WriterTools['tokenize']>,
			tryMarkup: (text) => {
				const c = game.tryMarkup(text);
				return { problem: problem(c.error), text: c.text, pieces: c.pieces };
			},
			tryCondition: (text) => {
				const c = game.tryCondition(text);
				return { problem: problem(c.error), holds: c.holds };
			}
		};
	});

	function startWriting(source: Source) {
		writingAt = { owner: source.owner, fragment: source.fragment };
		writingVariant = source.variant;
	}

	/** The fragment under edit, from the file; a new empty one for index -1. */
	const writerFragment = $derived.by<WriterFragment | null>(() => {
		const at = writingAt;
		const p = workspace?.project;
		if (!at || !p) return null;
		if (at.fragment < 0) return emptyFragment();
		return p.fragment(at.owner, at.fragment);
	});

	function writerTitle(at: WritingAt): string {
		const [entity, part] = at.owner.split('/');
		const [kind, id] = entity.split(':');
		const what = part === 'presence' ? `Presence text of ${id}` : kind === 'room' ? `Room text of ${session?.scene.room_name ?? id}` : `Text of ${entity}`;
		return at.fragment < 0 ? `${what}: new fragment` : `${what}, fragment ${at.fragment + 1}`;
	}

	/** Write a change to the file of the fragment. Returns a problem text when the story does not build with it. */
	async function saveFragment(at: WritingAt, change: (text: string, list: (string | number)[]) => string): Promise<string | null> {
		const ws = workspace;
		const where = ws?.project.listOf(at.owner);
		const file = where && ws?.project.file(where.file);
		if (!ws || !where || !file) return 'The editor does not know the file of this text.';
		let text: string;
		try {
			text = change(file.text, where.list);
		} catch (e) {
			return String(e instanceof Error ? e.message : e);
		}
		const problems = await ws.change(where.file, text);
		if (problems?.length) return problems.map((p) => p.message).join(' ');
		writingAt = null;
		return null;
	}
	let hidden = $state(false);

	// On ultrawide screens the map has its own column, so the inspector does not repeat it.
	let ultrawide = $state(false);
	let inspector = $state('assembly');
	const inspectorTabs = $derived<TabItem[]>([
		{ id: 'assembly', label: 'Assembly' },
		...(ultrawide ? [] : [{ id: 'map', label: 'Map' }]),
		{ id: 'story', label: 'Story' },
		{ id: 'rules', label: 'Rules' },
		{ id: 'explore', label: 'Explore' }
	]);

	let exploration = $state<Exploration | null>(null);
	let exploring = $state(false);
	let exploreMode = $state<'room' | 'game'>('room');
	/** The room matrix of the current room. It follows the state, so it shows only while the tab is open. */
	const matrix = $derived(
		session && inspector === 'explore' && exploreMode === 'room'
			? (session.scene, session.game.roomMatrix(session.scene.room))
			: null
	);

	function runExplorer() {
		const s = session;
		if (!s) return;
		exploring = true;
		// Let the button show "Exploring…" before the walk blocks the page.
		setTimeout(() => {
			exploration = s.game.explore();
			exploring = false;
		}, 30);
	}
	$effect(() => {
		if (ultrawide && inspector === 'map') inspector = 'assembly';
	});

	onMount(() => {
		openStory(storyName);
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
<svelte:document
	onpointerover={(e) => (hovered = inspectKind(e.target))}
	onfocusin={(e) => (hovered = inspectKind(e.target))}
	onfocusout={() => (hovered = null)}
/>

<svelte:head>
	<title>{session ? `${session.game.title} · lantern` : 'lantern'}</title>
</svelte:head>

{#if session}
	{@const s = session}
	<PlayerLayout {fxClass} {glow}>
		{#snippet header()}
			<AppHeader
				title={s.game.title}
				now={[`day ${s.scene.day} · ${s.scene.time} · ${s.scene.phase}`, s.scene.room_name, storageLabel]}
				settingsOpen={showSettings}
				onsettings={() => (showSettings = true)}
				onpalette={() => (showPalette = true)}
				{writing}
				onwrite={toggleWriting}
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
				brief={{ room: s.scene.room_name, things: s.scene.things, exits: s.scene.exits }}
				writable={writableEntry}
				writing={writingAt}
				onwrite={startWriting}
				onadd={() => (writingAt = { owner: `room:${s.scene.room}`, fragment: -1 })}
			>
				{#snippet writer()}
					{#if writingAt && writerFragment && writerTools}
						{@const at = writingAt}
						{#key `${at.owner}#${at.fragment}`}
							<FragmentWriter
								fragment={writerFragment}
								variant={writingVariant}
								title={writerTitle(at)}
								tools={writerTools}
								fresh={at.fragment < 0}
								onsave={(f) =>
									saveFragment(at, (text, list) =>
										at.fragment < 0 ? insertFragment(text, list, Number.MAX_SAFE_INTEGER, f) : writeFragment(text, list, at.fragment, f)
									)}
								oncancel={() => (writingAt = null)}
								onremove={() => saveFragment(at, (text, list) => removeFragment(text, list, at.fragment))}
							/>
						{/key}
					{/if}
				{/snippet}
			</Transcript>
			{#if !s.scene.ended}
				<CommandPrompt bind:value={s.draft} bind:input={prompt} oncommand={(text) => s.input(text)} />
				<VerbBar
					verbs={s.barVerbs}
					selected={s.preview.verb}
					sentence={s.sentence}
					canRunPartial={s.canRunPartial}
					marked={s.named}
					markedExit={s.preview.direction}
					hintedVerb={hovered ? s.clickVerb(hovered) : null}
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
				{:else if inspector === 'explore'}
					<ExplorerView
						bind:mode={exploreMode}
						{matrix}
						roomName={s.scene.room_name}
						ontry={(values) => {
							s.author({ type: 'situation', room: s.scene.room, values });
							prompt?.focus();
						}}
						result={exploration}
						running={exploring}
						onrun={runExplorer}
						onreplay={(path) => {
							s.replay(path);
							prompt?.focus();
						}}
					/>
				{:else if inspector === 'rules'}
					<RulesView rules={s.rules} reveal={revealRule} onrevealed={() => (revealRule = null)} />
				{:else}
					<AssemblyPanel
						room={s.scene.room}
						fragments={s.assembly}
						parts={s.parts}
						frame={s.frame}
						focus={s.focus}
						time={s.scene.time}
						phase={s.scene.phase}
						onfocus={(i) => (s.focus = i)}
						onedit={writing
							? (index) => {
									writingAt = { owner: `room:${s.scene.room}`, fragment: index };
									writingVariant = 0;
								}
							: undefined}
					/>
				{/if}
			</InspectorPanel>
		{/snippet}
	</PlayerLayout>
	<AuthorTooltip inspect={(kind, id) => s.game.inspect(kind, id)} enabled={settings.tooltips} />
	<CommandPalette bind:open={showPalette} items={paletteItems} fallback={playFallback} />
	<input
		bind:this={zipInput}
		type="file"
		accept=".zip,application/zip"
		hidden
		onchange={(e) => {
			const file = e.currentTarget.files?.[0];
			e.currentTarget.value = '';
			if (file && workspace) fileAction(() => workspace!.importZip(file));
		}}
	/>
	<SettingsDialog
		bind:open={showSettings}
		bind:theme={settings.theme}
		bind:effects={settings.effects}
		bind:motion={settings.motion}
		bind:sources={settings.sources}
		bind:tooltips={settings.tooltips}
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
			{#if workspace?.project.edited}
				{@const ws = workspace}
				<p>The changed files do not build.</p>
				<div class="actions">
					<Button onclick={() => exportZip(ws)}>Export the changed files as a zip</Button>
					<Button variant="primary" onclick={() => fileAction(() => ws.reset())}>Go back to the bundled example</Button>
				</div>
			{/if}
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
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
	}
	h1 {
		font-size: var(--font-size-heading);
		color: var(--color-text-warning);
	}
</style>
