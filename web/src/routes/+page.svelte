<script lang="ts">
	import { onMount } from 'svelte';
	import { Workspace } from '#lib/project/workspace.svelte.ts';
	import { AiLink, type ProposalView } from '#lib/project/proposals.svelte.ts';
	import ProposalCard from '#lib/design/molecules/ProposalCard.svelte';
	import AiStatus from '#lib/design/molecules/AiStatus.svelte';
	import { canLinkFolder } from '#lib/project/store.ts';
	import { emptyFragment, insertFragment, removeFragment, writeFragment } from '#lib/project/yaml-edit.ts';
	import FragmentWriter from '#lib/design/organisms/FragmentWriter.svelte';
	import ProseEditor from '#lib/design/molecules/ProseEditor.svelte';
	import { FragmentDraft } from '#lib/design/fragment-draft.svelte.ts';
	import { inMarkup } from '#lib/project/transform.ts';
	import { tick, untrack } from 'svelte';
	import CreatePanel from '#lib/design/organisms/CreatePanel.svelte';
	import { addExit, addResponseRule, directionOf, directions, opposite, roomFile, slug, step, thingFile, type Direction } from '#lib/project/create.ts';
	import type { CreateRequest } from '#lib/design/types.ts';
	import type { Source } from '#lib/engine/types.ts';
	import type { WriterFragment, WriterTools, WritingAt } from '#lib/design/types.ts';
	import { examples } from '#lib/engine/example.ts';
	import { newStory, notesOf, type NoteRef } from '#lib/engine/index.ts';
	import NotesList from '#lib/design/organisms/NotesList.svelte';
	import RadioGroup from '#lib/design/molecules/RadioGroup.svelte';
	import type { NoteItem } from '#lib/design/types.ts';
	import { addNote, pathOf, removeNote, setNoteDone } from '#lib/project/yaml-edit.ts';
	import { addStory, myStories, type StoryEntry } from '#lib/project/stories.ts';
	import NewStoryDialog from '#lib/design/organisms/NewStoryDialog.svelte';
	import { settings } from '#lib/settings.svelte.ts';
	import Button from '#lib/design/atoms/Button.svelte';
	import CommandPrompt from '#lib/design/molecules/CommandPrompt.svelte';
	import StatusLine from '#lib/design/organisms/StatusLine.svelte';
	import AppHeader from '#lib/design/organisms/AppHeader.svelte';
	import MapPanel from '#lib/design/organisms/MapPanel.svelte';
	import ScreenEffects from '#lib/design/organisms/ScreenEffects.svelte';
	import SettingsDialog from '#lib/design/organisms/SettingsDialog.svelte';
	import Transcript from '#lib/design/organisms/Transcript.svelte';
	import VerbBar from '#lib/design/organisms/VerbBar.svelte';
	import PlayerLayout from '#lib/design/templates/PlayerLayout.svelte';
	import HerePanel from '#lib/design/organisms/HerePanel.svelte';
	import BottomSheet from '#lib/design/organisms/BottomSheet.svelte';
	import StateSwitch from '#lib/design/atoms/StateSwitch.svelte';
	import CommandPalette from '#lib/design/organisms/CommandPalette.svelte';
	import AuthorTooltip from '#lib/design/organisms/AuthorTooltip.svelte';
	import RulesView from '#lib/design/organisms/RulesView.svelte';
	import ExplorerView from '#lib/design/organisms/ExplorerView.svelte';
	import AtlasMap from '#lib/design/organisms/AtlasMap.svelte';
	import RoomCard from '#lib/design/molecules/RoomCard.svelte';
	import AtlasLayout from '#lib/design/templates/AtlasLayout.svelte';
	import type { Exploration } from '#lib/engine/types.ts';
	import StoryTimeline from '#lib/design/organisms/StoryTimeline.svelte';
	import type { GhostLine, MenuItem, PaletteItem, ProposalSummary } from '#lib/design/types.ts';

	let workspace: Workspace | undefined = $state();
	const session = $derived(workspace?.session ?? undefined);
	const STORY_KEY = 'lantern.story';
	let mine = $state<StoryEntry[]>(myStories());
	let storyName = $state(readStory());
	let showNewStory = $state(false);
	/** True for a story that the author made, false for a bundled example. */
	const isMine = $derived(!(storyName in examples));

	function readStory(): string {
		try {
			const name = localStorage.getItem(STORY_KEY);
			return name && (name in examples || mine.some((s) => s.name === name)) ? name : 'lighthouse';
		} catch {
			return 'lighthouse';
		}
	}

	/** The bundled files of a story: the example, or the new-story files of the author's story. */
	async function bundledFiles(name: string) {
		if (name in examples) return examples[name];
		return newStory(mine.find((s) => s.name === name)?.title ?? name);
	}

	/** Make a new story from the dialog and open it. */
	async function createStory({ title, idea }: { title: string; idea: string }) {
		const entry = addStory(title, Object.keys(examples));
		mine = myStories();
		await openStory(entry.name);
		// The start room promises a blinking cursor: keep the promise, the command prompt gets the focus.
		await tick();
		prompt?.focus();
		if (!idea) return;
		// The server writes the same new story, then drafts the first rooms. The proposal shows when it is ready.
		const problem = await ai.bootstrap(title, idea);
		if (problem) failure = problem;
	}

	/** Load a story and start a new session with it. */
	async function openStory(name: string) {
		storyName = name;
		try {
			localStorage.setItem(STORY_KEY, name);
		} catch {
			// Storage is not available: the choice lasts for this visit only.
		}
		failure = '';
		exploration = null;
		workspace = undefined;
		try {
			workspace = await Workspace.open(name, await bundledFiles(name));
		} catch (e) {
			failure = String(e);
		}
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
				return p.edited ? 'edited, saved in browser' : isMine ? 'new story' : 'bundled example';
			default:
				return p.edited ? 'edited, not saved' : isMine ? 'new story' : 'bundled example';
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

	// AI proposals from `lantern serve`, when it runs.
	const ai = new AiLink();
	// Why the new story dialog cannot draft with AI now; null when it can.
	const newStoryAi = $derived(
		ai.reachable ? null : 'To draft with AI, start `lantern serve --bootstrap` and keep this page open.'
	);

	/** The words that Tab completes in the command prompt: the verbs; the things in reach and carried, and the exits. */
	const promptVerbs = $derived(session?.verbs.flatMap((v) => v.words) ?? []);
	const promptNouns = $derived.by(() => {
		const s = session;
		if (!s) return [];
		return [...s.scene.things.map((t) => t.name), ...s.scene.inventory.map((t) => t.name), ...s.scene.exits.map((e) => e.direction), 'back'];
	});

	// Author notes and TODOs, read from the files after each change.
	let allNotes = $state<NoteRef[]>([]);
	$effect(() => {
		const files = workspace?.project.files;
		if (!files) return;
		let current = true;
		notesOf(files).then((n) => {
			if (current) allNotes = n;
		});
		return () => {
			current = false;
		};
	});
	const allTodos = $derived(allNotes.filter((n) => n.note.todo));
	const openTodos = $derived(allTodos.filter((n) => !n.note.done));
	const doneTodos = $derived(allTodos.filter((n) => n.note.done));
	let todoFilter = $state<'open' | 'done' | 'all'>('open');
	const noteKey = (n: NoteRef) => `${n.file}#${n.at}`;

	/** Where a note is, in words: the room or thing name, or the fragment. */
	function noteWhere(n: NoteRef, here: boolean): string | undefined {
		const s = session;
		const [kind, id] = n.entity.split(':');
		const fragment = n.at.includes('.notes') ? n.at.match(/^(.*?)\[(\d+)\]/) : null;
		const part = fragment ? `${fragment[1] === 'description' ? 'fragment' : fragment[1]} ${Number(fragment[2]) + 1}` : '';
		if (here && kind === 'room') return part || undefined;
		const name =
			kind === 'room'
				? (s?.map.rooms.find((r) => r.id === id)?.name ?? id)
				: kind === 'thing'
					? (s?.scene.things.find((t) => t.id === id)?.name ?? id)
					: kind === 'thread'
						? `thread ${id}`
						: kind === 'rule'
							? `rule ${id}`
							: 'the story';
		return part ? `${name} · ${part}` : name;
	}
	const noteItem = (n: NoteRef, here: boolean): NoteItem => ({
		key: noteKey(n),
		text: n.note.text,
		todo: n.note.todo,
		done: n.note.done,
		where: noteWhere(n, here),
		thread: n.note.thread,
		role: n.note.role
	});

	/** The notes of the current room and of the things in it. */
	const hereNotes = $derived.by(() => {
		const s = session;
		if (!s) return [];
		const here = new Set([`room:${s.scene.room}`, ...s.scene.things.map((t) => `thing:${t.id}`)]);
		return allNotes.filter((n) => here.has(n.entity)).map((n) => noteItem(n, true));
	});
	const hereNotesMeta = $derived.by(() => {
		const todos = hereNotes.filter((n) => n.todo && !n.done).length;
		return hereNotes.length ? `${hereNotes.length}${todos ? ` · ${todos} TODO` : ''}` : 'none';
	});

	/** Change the file of a note. A problem shows as the failure line. */
	async function editNote(file: string, edit: (text: string) => string) {
		const ws = workspace;
		const text = ws?.project.file(file)?.text;
		if (!ws || text === undefined) return;
		try {
			const problems = await ws.change(file, edit(text));
			if (problems?.length) failure = problems.map((p) => p.message).join(' ');
		} catch (e) {
			failure = String(e);
		}
	}
	function addRoomNote(note: { text: string; todo: boolean }) {
		const s = session;
		const file = s && workspace?.project.origins[`room:${s.scene.room}`];
		if (file) editNote(file, (t) => addNote(t, ['notes'], note));
	}
	/** A note for the whole story, in story.yaml. */
	function addStoryNote(note: { text: string; todo: boolean }) {
		editNote(workspace?.project.origins['story'] ?? 'story.yaml', (t) => addNote(t, ['notes'], note));
	}
	function toggleNote(key: string, done: boolean) {
		const n = allNotes.find((x) => noteKey(x) === key);
		if (n) editNote(n.file, (t) => setNoteDone(t, pathOf(n.at), done));
	}
	function deleteNote(key: string) {
		const n = allNotes.find((x) => noteKey(x) === key);
		if (n) editNote(n.file, (t) => removeNote(t, pathOf(n.at)));
	}
	/** Go to the room of a TODO. */
	function goToNote(key: string) {
		const n = allNotes.find((x) => noteKey(x) === key);
		const s = session;
		if (!n || !s) return;
		const [kind, id] = n.entity.split(':');
		const room = kind === 'room' ? id : kind === 'thing' ? s.game.atlas().find((r) => r.things.some((t) => t.id === id))?.id : undefined;
		if (room) {
			if (room !== s.scene.room) s.author({ type: 'goto', room });
			closeAtlas();
		}
	}

	// The Atlas: the whole world at full width, in place of the player screen.
	let atlasOpen = $state(false);
	let atlasTab = $state<'map' | 'rules' | 'state' | 'story' | 'checks' | 'todos' | 'proposals'>('map');
	let atlasRoom = $state<string | null>(null);
	const atlasTabs = $derived([
		{ id: 'map', label: 'Map' },
		{ id: 'rules', label: 'Rules' },
		{ id: 'state', label: 'State' },
		{ id: 'story', label: 'Story' },
		{ id: 'checks', label: 'Checks' },
		{ id: 'todos', label: `TODOs ${openTodos.length}` },
		...(ai.reachable ? [{ id: 'proposals', label: `Proposals ${ai.proposals.length}` }] : [])
	]);
	// The Proposals tab goes away when the server stops; show the map in its place.
	$effect(() => {
		if (atlasTab === 'proposals' && !ai.reachable) atlasTab = 'map';
	});

	let proposalProblems = $state<Record<string, string>>({});

	function summarize(v: ProposalView): ProposalSummary {
		return {
			id: v.id,
			summary: v.summary,
			ok: v.ok,
			stale: v.stale,
			problems: v.new_problems.map((p) => p.message),
			changes: v.changes.map((c) =>
				c.kind === 'fragment'
					? `${c.owner} · fragment ${(c.after_index ?? c.before_index ?? 0) + 1} ${c.before_index == null ? 'added' : c.after_index == null ? 'removed' : 'changed'}`
					: c.kind === 'field'
						? `${c.entity} · ${c.field} changed`
						: `${c.entity} ${c.kind}`
			)
		};
	}
	const proposalSummaries = $derived(ai.proposals.map(summarize));

	// Compare the files of the AI server with the files of the editor, when the server comes up and after changes.
	$effect(() => {
		const ws = workspace;
		const reachable = ai.reachable;
		const files = ws?.project.files;
		if (!ws || !reachable || !files) return;
		const syncTimer = setTimeout(() => void ai.compare(files, ws.project.editedPaths()), 800);
		return () => clearTimeout(syncTimer);
	});
	const aiDiffers = $derived(ai.reachable && ai.sync !== null && !ai.sync.same);

	/** Start an AI job from the current game. The job plays from the state of the editor. */
	async function aiJob(task: string, args: Record<string, unknown>) {
		const s = session;
		if (!s) return;
		const problem = await ai.startJob(task, args, s.game.save());
		if (problem) failure = problem;
	}
	let now = $state(Date.now());
	$effect(() => {
		if (!ai.jobs.some((j) => j.status === 'running')) return;
		const t = setInterval(() => (now = Date.now()), 1000);
		return () => clearInterval(t);
	});
	const aiJobs = $derived(
		ai.jobs.map((j) => ({
			id: j.id,
			task: j.task,
			about: Object.values(j.args).filter((v) => typeof v === 'string').join(', '),
			status: j.status,
			seconds: Math.max(0, Math.round((now - (j.started ?? now)) / 1000)),
			cost: j.cost_usd,
			proposals: j.proposals.length,
			// A new story's draft can run again while its proposal is open, before the story changes.
			again: j.task === 'seed_story' && j.proposals.some((p) => ai.proposals.some((v) => v.id === p))
		}))
	);
	/** Another version of the first rooms: the same idea, one more job. The open proposal stays. */
	async function seedAgain(id: string) {
		const job = ai.jobs.find((j) => j.id === id);
		const idea = job?.args.idea;
		if (!session || typeof idea !== 'string') return;
		const problem = await ai.bootstrap(session.game.title, idea);
		if (problem) failure = problem;
	}
	const runningJob = $derived(aiJobs.find((j) => j.status === 'running'));

	/** The ghost lines of the proposals that change the text of the current room. */
	const ghostsHere = $derived.by<GhostLine[]>(() => {
		const s = session;
		if (!s || !writing) return [];
		const owner = `room:${s.scene.room}`;
		const out: GhostLine[] = [];
		for (const v of ai.proposals)
			for (const c of v.changes) {
				if (c.kind !== 'fragment' || c.owner !== owner) continue;
				const raw = c.after_fragment?.variants[0] ?? '';
				const shown = raw ? (renderMarkup(raw) ?? raw) : '';
				out.push({
					proposal: v.id,
					kind: c.before_index == null ? 'added' : c.after_index == null ? 'removed' : 'changed',
					before: c.before_index ?? null,
					text: shown
				});
			}
		return out;
	});

	// The proposals with an accept or a reject in progress. A second click on the same proposal does nothing.
	const proposalBusy = new Set<string>();
	async function onProposal(id: string, action: 'accept' | 'reject', note?: string) {
		const view = ai.proposals.find((p) => p.id === id);
		const ws = workspace;
		if (!view || !ws || proposalBusy.has(id)) return;
		proposalBusy.add(id);
		try {
			const seed = action === 'accept' && ai.isSeed(id);
			const problem = action === 'accept' ? await ai.accept(view, ws) : await ai.reject(view, note);
			// The first rooms of a new story: play it from the start, with its new intro.
			if (seed && !problem) {
				await ws.restart();
				closeAtlas();
			}
			proposalProblems = { ...proposalProblems, [id]: problem ?? '' };
			if (!problem) delete proposalProblems[id];
		} finally {
			proposalBusy.delete(id);
		}
	}
	const atlasRooms = $derived.by(() => {
		const s = session;
		if (!s || !atlasOpen) return [];
		s.entries.length;
		return s.game.atlas();
	});

	function openAtlas(tab: typeof atlasTab = atlasTab) {
		atlasTab = tab;
		atlasRoom = atlasRoom ?? session?.scene.room ?? null;
		atlasOpen = true;
	}

	function closeAtlas() {
		atlasOpen = false;
		queueMicrotask(() => prompt?.focus());
	}

	/** A rule for the Rules tab to show and focus. */
	let revealRule = $state<string | null>(null);

	function showRule(id: string) {
		openAtlas('rules');
		revealRule = id;
	}

	function backToPrompt() {
		prompt?.focus();
	}

	/** The project menu in the header: files, and the other examples. The command palette has them too. */
	const projectItems = $derived.by<MenuItem[]>(() => {
		const ws = workspace;
		if (!ws) return [];
		const p = ws.project;
		const items: MenuItem[] = [];
		if (canLinkFolder()) {
			if (p.storage === 'folder-locked')
				items.push({ id: 'files-reconnect', group: 'Files', label: `Give access to the folder “${p.folderName}” again`, run: () => fileAction(() => ws.reconnect()) });
			if (p.storage === 'folder')
				items.push({ id: 'files-unlink', group: 'Files', label: `Stop saving to the folder “${p.folderName}”`, run: () => fileAction(() => ws.unlinkFolder()) });
			else items.push({ id: 'files-link', group: 'Files', label: 'Open a story folder on disk', run: () => fileAction(() => ws.linkFolder()) });
		}
		items.push({ id: 'files-export', group: 'Files', label: 'Export the story as a zip file', run: () => exportZip(ws) });
		items.push({ id: 'files-import', group: 'Files', label: 'Import a story from a zip file', run: () => zipInput?.click() });
		if (p.edited || p.storage === 'folder')
			items.push({
				id: 'files-reset',
				group: 'Files',
				label: isMine ? 'Start this story again from one empty room' : `Go back to the bundled example “${p.name}”`,
				run: () => fileAction(() => ws.reset())
			});
		items.push({ id: 'story-new', group: 'Stories', label: 'New story…', run: () => (showNewStory = true) });
		for (const s of mine.filter((s) => s.name !== storyName))
			items.push({ id: `story-${s.name}`, group: 'Stories', label: `Open my story “${s.title}”`, run: () => openStory(s.name) });
		for (const name of Object.keys(examples).filter((n) => n !== storyName))
			items.push({ id: `story-${name}`, group: 'Examples', label: `Open the example “${name}”`, run: () => openStory(name) });
		return items;
	});

	/** Everything the command palette can do, built from the game state now. */
	const paletteItems = $derived.by<PaletteItem[]>(() => {
		const s = session;
		if (!s) return [];
		const items: PaletteItem[] = [];
		items.push({ id: 'todo-add', group: 'Notes', label: 'Add a TODO, or find the done ones', keywords: 'note todo task struck', run: () => openAtlas('todos') });
		items.push({ id: 'atlas', group: 'Atlas', label: atlasOpen ? 'Close the Atlas' : 'Open the Atlas', keywords: 'world map ctrl m', run: () => (atlasOpen ? closeAtlas() : openAtlas()) });
		for (const tab of atlasTabs) {
			items.push({ id: `atlas-${tab.id}`, group: 'Atlas', label: `Show ${tab.label} in the Atlas`, keywords: tab.id === 'checks' ? 'explore' : '', run: () => openAtlas(tab.id as typeof atlasTab) });
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
			id: 'tooltips',
			group: 'Look',
			label: settings.tooltips ? 'Hide author tooltips' : 'Show author tooltips',
			run: () => (settings.tooltips = !settings.tooltips)
		});
		items.push({ id: 'settings', group: 'Look', label: 'Open look and motion settings', run: () => (showSettings = true) });
		items.push({ id: 'restart', group: 'Game', label: 'Restart the game', run: () => s.restart() });
		items.push({ id: 'write-toggle', group: 'Write', label: writing ? 'Stop writing' : 'Write in the game', keywords: 'edit mode', run: toggleWriting });
		items.push({ id: 'create-exit', group: 'Write', label: `Create an exit from ${s.scene.room_name}`, keywords: 'new room door', run: () => openPanel({ creating: { kind: 'exit' } }) });
		items.push({ id: 'create-thing', group: 'Write', label: `Create a thing in ${s.scene.room_name}`, keywords: 'new object item', run: () => openPanel({ creating: { kind: 'thing' } }) });
		items.push({ id: 'create-character', group: 'Write', label: `Create a character in ${s.scene.room_name}`, keywords: 'new person npc', run: () => openPanel({ creating: { kind: 'character' } }) });
		items.push(...projectItems.map((i) => ({ ...i, group: i.group === 'Examples' ? 'Game' : 'Files' })));
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
		// Ctrl+E (Cmd+E on a Mac) toggles write mode, also while the author types.
		// Not Ctrl+W: browsers close the tab on it, and a page can not stop that.
		if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === 'm' && !showSettings) {
			event.preventDefault();
			showPalette = false;
			if (atlasOpen) closeAtlas();
			else openAtlas();
			return;
		}
		if (event.key === 'Escape' && atlasOpen && !showPalette && !showSettings) {
			const t = event.target as HTMLElement | null;
			if (!(t && (t.tagName === 'INPUT' && (t as HTMLInputElement).value))) {
				event.preventDefault();
				closeAtlas();
				return;
			}
		}
		if (event.key === '/' && atlasOpen && atlasTab === 'map' && !(event.target instanceof HTMLInputElement)) {
			event.preventDefault();
			document.getElementById('atlas-search')?.focus();
			return;
		}
		// In the fragment editor the key is left alone, so an open draft is not closed by accident.
		const inEditor = event.target instanceof Element && !!event.target.closest('.cm-editor');
		if ((event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'e' && !showSettings && !inEditor) {
			event.preventDefault();
			showPalette = false;
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
		creating = null;
		responding = null;
		respondable = null;
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
				const values = Object.entries(previewValues);
				const c = values.length ? game.tryMarkupIn(s.scene.room, values, text) : game.tryMarkup(text);
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

	/**
	 * The draft of the fragment under edit. The prose view in the game text and the writer edit it together.
	 * It is made when the author opens a fragment, and it stays while the files change (for example a new thing).
	 */
	let draft = $state.raw<FragmentDraft | null>(null);
	let draftKey = $state('');
	$effect(() => {
		const at = writingAt;
		const key = at ? `${at.owner}#${at.fragment}` : '';
		if (key === untrack(() => draftKey)) return;
		draftKey = key;
		const f = untrack(() => writerFragment);
		draft = at && at.fragment >= -1 && f ? new FragmentDraft(f, untrack(() => writingVariant)) : null;
	});
	let fragmentWriter: FragmentWriter | undefined = $state();
	/** On wide screens the writer sits next to the game text, in place of the Here panel. */
	let sideBySide = $state(false);

	/** How a piece of markup reads now, for the chips in the prose view. */
	function renderMarkup(markup: string): string | null {
		const s = session;
		const values = Object.entries(previewValues);
		const c = s && values.length ? s.game.tryMarkupIn(s.scene.room, values, markup) : s?.game.tryMarkup(markup);
		return !c || c.error ? null : c.text;
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
		if (at.fragment === -2) return '';
		const what = part === 'presence' ? `Presence text of ${id}` : kind === 'room' ? `Room text of ${session?.scene.room_name ?? id}` : `Text of ${entity}`;
		return at.fragment < 0 ? `${what}: new fragment` : `${what}, fragment ${at.fragment + 1}`;
	}

	/** A panel at the end of the newest turn: a form for something new, or a new response. */
	let creating = $state<{ kind: 'exit' | 'thing' | 'character'; direction?: string; name?: string } | null>(null);
	let responding = $state<{ verb: string; thing: string | null; label: string } | null>(null);
	/** The last command that got the default response, in write mode: the author can write a response for it. */
	let respondable = $state<{ verb: string; thing: string | null; label: string } | null>(null);

	function openPanel(panel: { creating?: typeof creating; responding?: typeof responding }) {
		writing = true;
		creating = panel.creating ?? null;
		responding = panel.responding ?? null;
		writingAt = { owner: 'panel', fragment: -2 };
	}

	function closeWriter() {
		writingAt = null;
		creating = null;
		responding = null;
		backToPrompt();
	}

	const ids = (kind: string) => new Set(Object.keys(workspace?.project.origins ?? {}).filter((k) => k.startsWith(`${kind}:`)).map((k) => k.slice(kind.length + 1)));

	/** Play a typed command. In write mode, a direction without an exit offers a new exit. */
	function play(text: string) {
		const s = session;
		if (!s) return;
		const dir = directionOf(text);
		if (writing && dir && !s.scene.exits.some((e) => e.direction === dir)) {
			s.draft = '';
			openPanel({ creating: { kind: 'exit', direction: dir } });
			return;
		}
		const pv = s.preview;
		s.input(text);
		respondable = null;
		const last = s.entries.at(-1)?.turn;
		const quiet = ['go', 'look', 'examine', 'inventory', 'wait', 'take', 'drop'];
		if (writing && last && pv.verb && pv.complete && !quiet.includes(pv.verb) && !last.log.some((l) => l.kind === 'rule')) {
			respondable = { verb: pv.verb, thing: pv.objects[0] ?? null, label: text.trim() };
		}
	}

	async function create(request: CreateRequest): Promise<string | null> {
		const ws = workspace;
		const s = session;
		if (!ws || !s) return 'No story is open.';
		const here = s.scene.room;
		const hereFile = ws.project.listOf(`room:${here}`)?.file;
		const hereText = hereFile && ws.project.file(hereFile)?.text;
		if (!hereFile || !hereText) return 'The editor does not know the file of this room.';
		if (request.kind === 'exit') {
			const dir = request.direction as Direction;
			const changes: { path: string; text: string }[] = [];
			let to: string;
			if ('name' in request.to) {
				to = slug(request.to.name, ids('room'));
				const me = s.map.rooms.find((r) => r.current);
				const [dx, dy] = step(dir);
				const position: [number, number] = [Math.round((me?.x ?? 0) + dx), Math.round((me?.y ?? 0) + dy)];
				changes.push({
					path: `rooms/${to}.yaml`,
					text: roomFile({ name: request.to.name, position, exits: request.back ? [{ direction: opposite(dir), to: here }] : [] })
				});
			} else {
				to = request.to.id;
				if (request.back) {
					const file = ws.project.listOf(`room:${to}`)?.file;
					const text = file && ws.project.file(file)?.text;
					if (file && text) changes.push({ path: file, text: addExit(text, opposite(dir), here) });
				}
			}
			changes.push({ path: hereFile, text: addExit(hereText, dir, to) });
			const problems = await ws.changeMany(changes);
			if (problems?.length) return problems.map((p) => p.message).join(' ');
			s.input(dir);
			// A new room has no text yet: write it at once.
			writingAt = 'name' in request.to ? { owner: `room:${to}`, fragment: -1 } : null;
			creating = null;
			return null;
		}
		const id = slug(request.name, new Set([...ids('thing'), ...ids('room')]));
		const problems = await ws.changeMany([
			{
				path: `things/${id}.yaml`,
				text: thingFile({ name: request.name, room: here, portable: request.portable, proper: request.proper })
			}
		]);
		if (problems?.length) return problems.map((p) => p.message).join(' ');
		creating = null;
		// Write how the new thing shows in the room.
		writingAt = { owner: `thing:${id}/presence`, fragment: -1 };
		return null;
	}

	/** "Make it a thing": the selected words name a new thing in the room. The text says it, so it is not listed. */
	async function makeThing(name: string) {
		const ws = workspace;
		const s = session;
		if (!ws || !s) return;
		const id = slug(name, new Set([...ids('thing'), ...ids('room')]));
		await ws.changeMany([{ path: `things/${id}.yaml`, text: thingFile({ name: name.toLowerCase(), room: s.scene.room, listed: false }) }]);
	}

	/** Create a thing for an unknown name in the text, and keep the writer open. */
	async function createNamed(id: string, property: string | null) {
		const ws = workspace;
		const s = session;
		if (!ws || !s) return;
		const free = ids('thing').has(id) ? slug(id, ids('thing')) : id;
		await ws.changeMany([
			{
				path: `things/${free}.yaml`,
				text: thingFile({ name: free.replace(/_/g, ' '), room: s.scene.room, props: property ? { [property]: false } : undefined })
			}
		]);
	}

	async function saveResponse(r: { verb: string; thing: string | null }, f: WriterFragment): Promise<string | null> {
		const ws = workspace;
		const s = session;
		if (!ws || !s) return 'No story is open.';
		const path = `rules/${s.scene.room}.yaml`;
		const id = slug(`${r.verb}_${r.thing ?? 'nothing'}`, ids('rule'));
		const text = addResponseRule(ws.project.file(path)?.text ?? null, { id, verb: r.verb, thing: r.thing, when: f.when, say: f.variants[0] });
		const problems = await ws.change(path, text);
		if (problems?.length) return problems.map((p) => p.message).join(' ');
		closeWriter();
		respondable = null;
		return null;
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
		closeWriter();
		return null;
	}
	let hidden = $state(false);

	// On ultrawide screens the map has its own column on the left.
	let ultrawide = $state(false);

	let exploration = $state<Exploration | null>(null);
	let exploring = $state(false);
	let exploreMode = $state<'room' | 'game'>('room');
	/** The room matrix of the current room. It follows the state, so it shows only while the tab is open. */
	const matrix = $derived(
		session && atlasOpen && atlasTab === 'checks' && exploreMode === 'room'
			? (session.scene, session.game.roomMatrix(session.scene.room))
			: null
	);

	/** The notes of the margin in write mode: each room fragment with its layer, condition and state. */
	const marginInfo = $derived.by(() => {
		const s = session;
		if (!s) return null;
		const owner = `room:${s.scene.room}`;
		const detail = (f: (typeof s.assembly)[number]) => {
			const parts: string[] = [f.mode.replace('_', ' ')];
			if (f.condition) parts.push(`when ${f.condition}`);
			if (f.replaces) parts.push('replaces');
			// Author notes on the fragment: a count, and the threads that it is a beat of.
			const notes = allNotes.filter((n) => n.entity === owner && n.at.startsWith(`description[${f.index}].notes`));
			if (notes.length) parts.push(`✎ ${notes.length} ${notes.length === 1 ? 'note' : 'notes'}`);
			for (const n of notes) if (n.note.thread) parts.push(`${n.note.role ?? 'beat'} of ${n.note.thread}`);
			return parts.join(' · ');
		};
		return {
			owner: `room:${s.scene.room}`,
			fragments: s.assembly.map((f) => ({ index: f.index, layer: f.layer, detail: detail(f), shown: f.shown, why: f.why, text: f.text })),
			things: s.parts.objects.length,
			exits: s.parts.exits.length
		};
	});

	// The Here panel.
	/** The rooms next to the current room on its level, for the small map. */
	const hereMap = $derived.by(() => {
		const s = session;
		if (!s) return { rooms: [], links: [] };
		const me = s.map.rooms.find((r) => r.current);
		if (!me) return s.map;
		const near = new Set([me.id]);
		for (const l of s.map.links) {
			if (l.from === me.id) near.add(l.to);
			if (l.to === me.id) near.add(l.from);
		}
		const rooms = s.map.rooms.filter((r) => near.has(r.id) && r.level === me.level);
		const ids = new Set(rooms.map((r) => r.id));
		return { rooms, links: s.map.links.filter((l) => ids.has(l.from) && ids.has(l.to)) };
	});
	/** The values that the room text depends on, with the values now. */
	const hereStates = $derived.by(() => {
		const s = session;
		if (!s || atlasOpen) return { switches: [], texts: 0 };
		s.entries.length;
		const m = s.game.roomMatrix(s.scene.room);
		if (!m) return { switches: [], texts: 0 };
		const symbols = new Map(s.game.symbols().map((sym) => [sym.name, sym.value ?? '']));
		return {
			switches: m.dimensions.map((d) => ({ name: d.name, options: d.values, value: symbols.get(d.name) ?? d.values[0] })),
			texts: new Set(m.cells.map((c) => c.text)).size
		};
	});
	/** The line of the bottom sheet on phones: exits, and the next rule that matters. */
	const sheetSummary = $derived.by(() => {
		const s = session;
		if (!s) return '';
		const parts = [`${s.scene.exits.length} ${s.scene.exits.length === 1 ? 'exit' : 'exits'}`];
		const rule = s.rules.find((r) => r.relevant);
		if (rule) parts.push(`${rule.id} ${rule.relevant}`);
		if (Object.keys(previewValues).length) parts.unshift('PREVIEW');
		return parts.join(' · ');
	});

	/** The values that the room text shows instead of the game: a preview. It never moves the game. */
	let previewValues = $state<Record<string, string>>({});
	$effect(() => {
		// A turn or another room ends the preview.
		session?.entries.length;
		session?.scene.room;
		previewValues = {};
	});
	function setPreview(name: string, value: string) {
		const current = hereStates.switches.find((w) => w.name === name)?.value;
		const next = { ...previewValues };
		if (value === current) delete next[name];
		else next[name] = value;
		previewValues = next;
	}

	/** The preview of the room text. While the author writes, the fragment in the game text shows the preview instead. */
	const previewTurn = $derived.by(() => {
		const s = session;
		const entries = Object.entries(previewValues);
		return s && entries.length && !writing ? s.game.previewSituation(s.scene.room, entries) : null;
	});

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

	onMount(() => {
		openStory(storyName);
		ai.start();
		// Same query as --bp-ultrawide in PlayerLayout.
		const wide = window.matchMedia('(min-width: 2200px) and (min-aspect-ratio: 21/9)');
		ultrawide = wide.matches;
		const onWide = () => (ultrawide = wide.matches);
		wide.addEventListener('change', onWide);
		// Same width as --bp-stack in PlayerLayout: from here the side panel is next to the game text.
		const side = window.matchMedia('(min-width: 900px)');
		sideBySide = side.matches;
		const onSide = () => (sideBySide = side.matches);
		side.addEventListener('change', onSide);
		const onVisibility = () => (hidden = document.hidden);
		document.addEventListener('visibilitychange', onVisibility);
		return () => {
			document.removeEventListener('visibilitychange', onVisibility);
			wide.removeEventListener('change', onWide);
			side.removeEventListener('change', onSide);
			ai.stop();
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

{#snippet notesSection()}
	<NotesList notes={hereNotes} empty="No notes for this room yet." onadd={addRoomNote} ontoggle={toggleNote} onremove={deleteNote} />
{/snippet}

{#snippet aiSection()}
	{#if session}
		{@const s = session}
		<AiStatus
			jobs={aiJobs}
			differ={aiDiffers ? (ai.sync?.differ ?? []) : null}
			onreview={() => aiJob('review_room', { room: s.scene.room })}
			oncancel={(id) => ai.cancelJob(id)}
			onlink={canLinkFolder() ? () => fileAction(() => workspace!.linkFolder()) : undefined}
			onproposals={() => openAtlas('proposals')}
			onagain={seedAgain}
		/>
	{/if}
{/snippet}

{#snippet fragmentWriterView()}
	{#if writingAt && writingAt.fragment >= -1 && writerFragment && writerTools && draft && session}
		{@const at = writingAt}
		{#if hereStates.switches.length}
			<!-- The room stays pinned while the author writes: a value change shows in the fragment in the game text. -->
			<section class="pinned" aria-label="{session.scene.room_name}, pinned">
				<p class="pinned-head"><span>{session.scene.room_name}</span><span class="muted">pinned</span></p>
				<div class="pinned-switches">
					{#each hereStates.switches as sw (sw.name)}
						<StateSwitch
							name={sw.name}
							value={previewValues[sw.name] ?? sw.value}
							options={sw.options.includes(sw.value) ? sw.options : [sw.value, ...sw.options]}
							changed={sw.name in previewValues}
							onchange={(v) => setPreview(sw.name, v)}
						/>
					{/each}
				</div>
			</section>
		{/if}
		{#key draftKey}
			<FragmentWriter
				bind:this={fragmentWriter}
				{draft}
				fragment={writerFragment}
				variant={writingVariant}
				title={writerTitle(at)}
				tools={writerTools}
				fresh={at.fragment < 0}
				onsave={(f) =>
					saveFragment(at, (text, list) =>
						at.fragment < 0 ? insertFragment(text, list, Number.MAX_SAFE_INTEGER, f) : writeFragment(text, list, at.fragment, f)
					)}
				oncancel={closeWriter}
				onremove={() => saveFragment(at, (text, list) => removeFragment(text, list, at.fragment))}
				oncreate={createNamed}
				onthing={makeThing}
				prose
			/>
		{/key}
	{/if}
{/snippet}

{#if session && atlasOpen}
	{@const s = session}
	{@const card = atlasRooms.find((r) => r.id === atlasRoom)}
	<AtlasLayout tabs={atlasTabs} selected={atlasTab} onselect={(id) => (atlasTab = id as typeof atlasTab)}>
		{#snippet header()}
			<AppHeader
				title={s.game.title}
				saved={storageLabel}
				savedState={workspace?.project.storage === 'folder-locked' ? 'attention' : 'ok'}
				{projectItems}
				back={{ label: `Back to ${s.scene.room_name}`, run: closeAtlas }}
				settingsOpen={showSettings}
				onsettings={() => (showSettings = true)}
				onpalette={() => (showPalette = true)}
			/>
		{/snippet}
				{#if atlasTab === 'map'}
					<AtlasMap map={s.map} rooms={atlasRooms} bind:selected={atlasRoom} onselect={(id) => (atlasRoom = id)} />
				{:else if atlasTab === 'story'}
					<StoryTimeline steps={s.history} />
				{:else if atlasTab === 'checks'}
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
				{:else if atlasTab === 'rules'}
					<RulesView rules={s.rules} reveal={revealRule} onrevealed={() => (revealRule = null)} />
				{:else if atlasTab === 'todos'}
					<section class="todos-view" aria-label="TODOs">
						<RadioGroup
							legend="Show"
							name="todo-filter"
							look="pills"
							options={[
								{ value: 'open', label: `Open ${openTodos.length}` },
								{ value: 'done', label: `Done ${doneTodos.length}` },
								{ value: 'all', label: 'All' }
							]}
							bind:value={todoFilter}
						/>
						<NotesList
							notes={(todoFilter === 'open' ? openTodos : todoFilter === 'done' ? doneTodos : allTodos).map((n) => noteItem(n, false))}
							empty={todoFilter === 'done'
								? 'No done TODOs. A done TODO stays in its file, struck through, until you remove it with ×.'
								: 'No open TODOs.'}
							todoOnly
							placeholder="A TODO for the whole story"
							onadd={addStoryNote}
							ontoggle={toggleNote}
							onremove={deleteNote}
							onselect={goToNote}
						/>
						<p class="muted-line">
							A TODO added here goes into story.yaml. Add a TODO for a room in the Notes section of the Here panel. A done TODO stays,
							struck through, until you remove it with ×. A removed TODO is gone from the file.
						</p>
					</section>
				{:else if atlasTab === 'proposals'}
					<section class="proposals-view" aria-label="Proposals">
						{#each proposalSummaries as p (p.id)}
							<ProposalCard
								proposal={p}
								full
								problem={proposalProblems[p.id] ?? null}
								onaccept={() => onProposal(p.id, 'accept')}
								onreject={(note) => onProposal(p.id, 'reject', note)}
							/>
						{:else}
							<p class="muted-line">No open proposals. An assistant makes them through lantern serve.</p>
						{/each}
					</section>
				{:else if atlasTab === 'state'}
					<section class="state-view" aria-label="State">
						<table>
							<caption class="visually-hidden">Variables and properties, with their values now</caption>
							<thead><tr><th scope="col">Name</th><th scope="col">Value</th></tr></thead>
							<tbody>
								{#each s.vars as v (v.name)}
									<tr>
										<th scope="row"><code>{v.name}</code></th>
										<td>
											{#if v.options.length || typeof v.value === 'boolean'}
												<StateSwitch
													name={v.name}
													value={String(v.value)}
													options={v.options.length ? v.options : ['false', 'true']}
													onchange={(val) =>
														s.author({ type: 'set', name: v.name, value: typeof v.value === 'boolean' ? val === 'true' : val })}
												/>
											{:else}
												{String(v.value)}
											{/if}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</section>
				{/if}

		{#snippet aside()}
			{#if atlasTab === 'map' && card}
				<RoomCard
					room={card}
					rules={s.rules.filter((r) => r.when.includes(card.id) || (r.condition ?? '').includes(card.id))}
					ongo={() => {
						s.author({ type: 'goto', room: card.id });
						closeAtlas();
					}}
					onwrite={() => {
						if (!card.current) s.author({ type: 'goto', room: card.id });
						closeAtlas();
						if (!writing) toggleWriting();
					}}
					onexplore={() => {
						if (!card.current) s.author({ type: 'goto', room: card.id });
						atlasTab = 'checks';
						exploreMode = 'room';
					}}
				/>
			{/if}
		{/snippet}
	</AtlasLayout>
{/if}
{#if session && !atlasOpen}
	{@const s = session}
	<PlayerLayout {fxClass} {glow}>
		{#snippet header()}
			<AppHeader
				title={s.game.title}
				saved={aiDiffers ? `${storageLabel} · AI sees other files` : storageLabel}
				savedState={workspace?.project.storage === 'folder-locked' || aiDiffers ? 'attention' : 'ok'}
				{projectItems}
				mode={writing ? 'write' : 'play'}
				onmode={(m) => {
					if ((m === 'write') !== writing) toggleWriting();
				}}
				settingsOpen={showSettings}
				onsettings={() => (showSettings = true)}
				onpalette={() => (showPalette = true)}
			/>
		{/snippet}
		{#snippet status()}
			<StatusLine
				room={s.scene.room_name}
				level={new Set(s.map.rooms.map((r) => r.level)).size > 1 ? (s.map.rooms.find((r) => r.current)?.level ?? null) : null}
				day={s.scene.day}
				time={s.scene.time}
				phase={s.scene.phase}
				turn={s.scene.turns}
				turnItems={[{ id: 'restart', label: 'Restart the game', run: () => s.restart() }]}
				onroom={() => openAtlas('map')}
			>
				{#snippet marks()}
					{#if openTodos.length}<button type="button" class="job-chip" onclick={() => openAtlas('todos')}
							>{openTodos.length} TODO</button
						>{/if}
					{#if runningJob}<button type="button" class="job-chip" onclick={() => openAtlas('proposals')}
							>✦ {runningJob.task === 'review_room' ? 'reviewing' : 'drafting'}… {runningJob.seconds} s</button
						>{:else if ai.reachable && ai.proposals.length}<button type="button" class="job-chip" onclick={() => openAtlas('proposals')}
							>✦ {ai.proposals.length} {ai.proposals.length === 1 ? 'proposal' : 'proposals'}</button
						>{/if}
					{#if Object.keys(previewValues).length}<span class="preview-mark"
							>PREVIEW · {Object.values(previewValues).join(', ')}</span
						>{/if}
				{/snippet}
			</StatusLine>
		{/snippet}
		{#snippet effects()}
			<ScreenEffects theme={settings.theme} effects={settings.effects} />
		{/snippet}
		{#snippet player()}
			<Transcript
				entries={s.entries}
				room={s.scene.room}
				showSources={writing}
				focus={s.focus}
				effects={settings.effects}
				reduceMotion={settings.reduceMotion}
				reveal={settings.reveal}
				onthing={(id, name) => s.pickThing(id, name)}
				onexit={(dir) => s.pickExit(dir)}
				onfocus={(i) => (s.focus = i)}
				brief={{ room: s.scene.room_name, things: s.scene.things, exits: s.scene.exits }}
				writable={writableEntry}
				writing={writingAt}
				onwrite={startWriting}
				onadd={() => (writingAt = { owner: `room:${s.scene.room}`, fragment: -1 })}
				preview={previewTurn?.blocks ?? null}
				ghosts={ghostsHere}
				proposals={proposalSummaries.filter((p) => ghostsHere.some((g) => g.proposal === p.id))}
				onproposal={onProposal}
				{proposalProblems}
				margin={writing ? marginInfo : null}
				respond={respondable?.label ?? null}
				onrespond={() => respondable && openPanel({ responding: respondable })}
			>
				{#snippet writer()}
					{#if writingAt?.fragment === -2 && creating}
						<CreatePanel
							kind={creating.kind}
							here={s.scene.room_name}
							direction={creating.direction}
							directions={directions.filter((d) => !s.scene.exits.some((e) => e.direction === d))}
							rooms={s.map.rooms.filter((r) => !r.current).map((r) => ({ id: r.id, name: r.name }))}
							name={creating.name}
							onsubmit={create}
							oncancel={closeWriter}
							ondraft={ai.reachable && creating.kind === 'exit'
								? (dir) => {
										void aiJob('imagine_exit', { room: s.scene.room, direction: dir });
										closeWriter();
									}
								: undefined}
						/>
					{:else if writingAt?.fragment === -2 && responding && writerTools}
						{@const r = responding}
						<FragmentWriter
							kind="response"
							fragment={{ variants: [''], when: null, mode: 'always', layer: 'base', replaces: false }}
							title="Response to “{r.label}” in {s.scene.room_name}"
							tools={writerTools}
							fresh
							onsave={(f) => saveResponse(r, f)}
							oncancel={closeWriter}
						/>
					{:else if !sideBySide}
						{@render fragmentWriterView()}
					{/if}
				{/snippet}
				{#snippet editor()}
					{#if draft && writerTools}
						{@const d = draft}
						{#key draftKey}
							<ProseEditor
								bind:value={d.variants[d.current]}
								label="{writingAt ? writerTitle(writingAt) : 'Text'}, as the player reads it"
								render={renderMarkup}
								stamp={`${JSON.stringify(previewValues)}#${s.entries.length}`}
								placeholder="Write what the player reads…"
								onsubmit={() => fragmentWriter?.save()}
								oncancel={closeWriter}
								onselection={(from, to, rect) =>
									(d.selection = rect && !inMarkup(d.text, from, to) ? { view: 'prose', from, to, rect } : null)}
								ontools={() => d.toolsFocus++}
								onmarkup={() => (d.showMarkup = !d.showMarkup)}
								target={d.target?.view === 'prose' ? d.target : null}
								autofocus
							/>
						{/key}
					{/if}
				{/snippet}
			</Transcript>
			{#if !s.scene.ended}
				<CommandPrompt bind:value={s.draft} bind:input={prompt} oncommand={play} verbs={promptVerbs} nouns={promptNouns} />
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
					hint={!s.entries.some((e) => e.turn.blocks.some((b) => b.kind === 'command'))}
				/>
			{:else}
				<p id="command" tabindex="-1">The story has ended. <Button size="compact" onclick={() => s.restart()}>Play again</Button></p>
			{/if}
		{/snippet}
		{#snippet map()}
			<MapPanel map={s.map} framed />
		{/snippet}
		{#snippet panel()}
			{#if sideBySide && draft && writingAt && writingAt.fragment >= -1}
				<div class="side-writer">{@render fragmentWriterView()}</div>
			{:else if sideBySide}
				<HerePanel
					map={hereMap}
					switches={hereStates.switches}
					texts={hereStates.texts}
					preview={previewValues}
					rules={s.rules.filter((r) => r.relevant)}
					ruleCount={s.rules.length}
					last={s.history.at(-1) ?? null}
					things={s.scene.things}
					exits={s.scene.exits}
					onpreview={setPreview}
					onreset={() => (previewValues = {})}
					onapply={() => {
						s.author({ type: 'situation', room: s.scene.room, values: Object.entries(previewValues) });
						previewValues = {};
					}}
					onrule={showRule}
					onopen={(view) => openAtlas(view === 'explore' ? 'checks' : view)}
					onthing={(id, name) => s.pickThing(id, name)}
					onexit={(dir) => s.pickExit(dir)}
					ai={ai.reachable ? aiSection : undefined}
					notes={notesSection}
					notesMeta={hereNotesMeta}
				/>
			{:else}
				<BottomSheet title="Here" summary={sheetSummary}>
					<HerePanel
						map={hereMap}
						switches={hereStates.switches}
						texts={hereStates.texts}
						preview={previewValues}
						rules={s.rules.filter((r) => r.relevant)}
						ruleCount={s.rules.length}
						last={s.history.at(-1) ?? null}
						things={s.scene.things}
						exits={s.scene.exits}
						onpreview={setPreview}
						onreset={() => (previewValues = {})}
						onapply={() => {
							s.author({ type: 'situation', room: s.scene.room, values: Object.entries(previewValues) });
							previewValues = {};
						}}
						onrule={showRule}
						onopen={(view) => openAtlas(view === 'explore' ? 'checks' : view)}
						onthing={(id, name) => s.pickThing(id, name)}
						onexit={(dir) => s.pickExit(dir)}
						ai={ai.reachable ? aiSection : undefined}
					notes={notesSection}
					notesMeta={hereNotesMeta}
					/>
				</BottomSheet>
			{/if}
		{/snippet}
	</PlayerLayout>
{/if}
{#if session}
	{@const s = session}
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
	<NewStoryDialog bind:open={showNewStory} aiUnavailable={newStoryAi} oncreate={createStory} />
	<SettingsDialog
		bind:open={showSettings}
		bind:theme={settings.theme}
		bind:effects={settings.effects}
		bind:motion={settings.motion}
		bind:tooltips={settings.tooltips}
		bind:reveal={settings.reveal}
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
					<Button variant="primary" onclick={() => fileAction(() => ws.reset())}
						>{isMine ? 'Start this story again from one empty room' : 'Go back to the bundled example'}</Button
					>
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
	/* The writer next to the game text, in the place of the Here panel. */
	.pinned {
		position: sticky;
		top: 0;
		z-index: 2;
		margin-bottom: var(--space-4);
		padding: var(--space-3) var(--space-4);
		border: var(--size-border) solid var(--color-border-subtle);
		background: var(--color-surface-control);
		font-family: var(--font-ui);
	}
	.pinned-head {
		display: flex;
		justify-content: space-between;
		margin: 0 0 var(--space-2);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
		color: var(--color-text-strong);
	}
	.pinned-head .muted {
		color: var(--color-text-muted);
	}
	.pinned-switches {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.job-chip {
		min-height: 1.6em;
		padding: 0 var(--space-3);
		border: 0;
		background: var(--color-status-text);
		color: var(--color-ghost);
		font: inherit;
	}
	.job-chip:focus-visible {
		outline: var(--size-focus-ring) solid var(--color-status-text);
		outline-offset: 2px;
	}
	.todos-view {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		max-width: 48rem;
		padding: var(--space-5) var(--space-6);
	}
	.proposals-view {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 22rem), 1fr));
		gap: var(--space-4);
		padding: var(--space-5) var(--space-6);
	}
	.muted-line {
		color: var(--color-text-muted);
		font-family: var(--font-ui);
	}
	.state-view {
		padding: var(--space-5) var(--space-6);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.state-view table {
		border-collapse: collapse;
	}
	.state-view th,
	.state-view td {
		padding: var(--space-2) var(--space-6) var(--space-2) 0;
		border-bottom: var(--size-border) solid var(--color-border-subtle);
		text-align: left;
	}
	.state-view thead th {
		color: var(--color-text-muted);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-caps);
		font-size: var(--font-size-xs);
	}
	.preview-mark {
		padding: 0 var(--space-3);
		background: var(--color-status-text);
		color: var(--color-status-bg);
	}
	.side-writer {
		position: relative;
		box-sizing: border-box;
		height: 100%;
		min-height: 0;
		overflow-y: auto;
		padding: var(--space-5);
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
