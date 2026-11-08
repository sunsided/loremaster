// The two classes of the engine mirror ADR 0003: `Session` plays a story and returns steps of effects;
// `Workbench` owns a session and adds the authoring views of the editor. Nothing here calls the engine
// in another way.
import init, {
	Session as WasmSession,
	Workbench as WasmWorkbench,
	markupTokens as wasmMarkupTokens,
	newStory as wasmNewStory,
	projectNotes,
	projectThreads,
	qualityReport
} from './pkg/loremaster-wasm.js';
import type {
	AuthorCommand,
	Choice,
	CombatView,
	StatView,
	MediaView,
	Input,
	JournalEntry,
	Loaded,
	Snapshot,
	Status,
	Step,
	Talk,
	Value,
	DialogueOutline,
	TalkView,
	AtlasRoom,
	Exploration,
	RoomMatrix,
	FrameUse,
	MarkupCheck,
	Inspect,
	NoteRef,
	ThreadView,
	PhaseView,
	VarView,
	Diagnostic,
	FragmentView,
	Hint,
	QualityReport,
	StepChart,
	MapView,
	Preview,
	RoomParts,
	RuleView,
	MachineView,
	Scene,
	SourceFile,
	Turn,
	VerbInfo
} from './types.ts';

export type * from './types.ts';

let ready: Promise<unknown> | undefined;
/** True once the WASM module is loaded. */
let loaded = false;

/** Load the WASM module once. */
export function loadEngine(): Promise<unknown> {
	ready ??= init().then((module) => {
		loaded = true;
		return module;
	});
	return ready;
}

/** The files of a new story with this title: `story.yaml` and one start room. */
export async function newStory(title: string): Promise<SourceFile[]> {
	await loadEngine();
	return wasmNewStory(title) as SourceFile[];
}

/** All author notes of the files, with their place. Empty when the files do not load. */
export async function notesOf(files: SourceFile[]): Promise<NoteRef[]> {
	await loadEngine();
	return projectNotes(files) as NoteRef[];
}

/** All threads of the files, with their beats. Empty when the files do not load. */
export async function threadsOf(files: SourceFile[]): Promise<ThreadView[]> {
	await loadEngine();
	return projectThreads(files) as ThreadView[];
}

/** The quality report of the files (CK-8). Null when the files do not load. */
export async function qualityOf(files: SourceFile[]): Promise<QualityReport | null> {
	await loadEngine();
	return qualityReport(files) as QualityReport | null;
}

/** Thrown when a project has errors; carries the diagnostics. */
export class ProjectError extends Error {
	constructor(public diagnostics: Diagnostic[]) {
		super(diagnostics.map((d) => `${d.file}: ${d.message}`).join('\n'));
	}
}

/**
 * Highlighted spans of fragment text, or of a condition. Offsets are UTF-16, as in JavaScript strings.
 * Call it after `loadEngine()` has resolved, for example once a `Workbench` is open.
 */
export function markupTokens(text: string, condition: boolean): { from: number; to: number; kind: string }[] {
	// Synchronous for the editor's highlighter, so it cannot wait for the module itself.
	if (!loaded) throw new Error('markupTokens: the engine is not loaded yet; await loadEngine() first.');
	return wasmMarkupTokens(text, condition);
}

/** Build an engine object from project files; turn the thrown diagnostics into a `ProjectError`. */
async function build<T>(make: () => T): Promise<T> {
	await loadEngine();
	try {
		return make();
	} catch (e) {
		if (Array.isArray(e)) throw new ProjectError(e as Diagnostic[]);
		throw e;
	}
}

/** A running game for a player (the Session API). Each call that plays returns a step: effects and a status. */
export class Session {
	#session: WasmSession;

	private constructor(session: WasmSession) {
		this.#session = session;
	}

	/** A new game of the files. Throws a `ProjectError` when the files have errors. */
	static async open(files: SourceFile[]): Promise<Session> {
		return new Session(await build(() => new WasmSession(files)));
	}

	/** Give the memory of the engine back. Do not use the session after this. */
	free(): void {
		this.#session.free();
	}

	get title(): string {
		return this.#session.title;
	}

	/** The opening. A second call starts nothing again. */
	start(): Step {
		return this.#session.start();
	}

	submit(input: Input): Step {
		return this.#session.submit(input);
	}

	/** Choose an answer of the talk in progress, by its index in `choices`. */
	choose(index: number): Step {
		return this.#session.choose(index);
	}

	/** Answer the request of an `await` status, also the death prompt. Throws when the answer does not fit. */
	resume(awaitId: string, value: Value): Step {
		return this.#session.resume(awaitId, value);
	}

	/** Go back to the state before the last input. */
	undo(): Step {
		return this.#session.undo();
	}

	/** How many inputs `undo` can go back now. */
	undoSteps(): number {
		return this.#session.undoSteps();
	}

	/** How many inputs `undo` can go back. 0 turns undo off. */
	setUndoDepth(depth: number): void {
		this.#session.setUndoDepth(depth);
	}

	/** Answer rolls with `resume` instead of letting the session roll. */
	setHostRolls(hostRolls: boolean): void {
		this.#session.setHostRolls(hostRolls);
	}

	/** A stage host: a command on a thing in the room or a move first awaits `walk_to`. */
	setHostWalks(hostWalks: boolean): void {
		this.#session.setHostWalks(hostWalks);
	}

	/** Start the story again. `undo` can go back to the state before it. */
	restart(): Step {
		return this.#session.restart();
	}

	/** The state now, to put back later with `restore`. */
	snapshot(): Snapshot {
		return this.#session.snapshot();
	}

	/** Put a snapshot back. Throws for a snapshot of another story. */
	restore(snapshot: Snapshot): Step {
		return this.#session.restore(snapshot);
	}

	/** The save state, as JSON-compatible data. */
	save(): unknown {
		return this.#session.save();
	}

	/** Continue from a save, also one from an earlier version of the story. Throws when the save cannot be read. */
	load(save: unknown): Loaded {
		return this.#session.load(save);
	}

	/** What the session waits for now. */
	status(): Status {
		return this.#session.status();
	}

	scene(): Scene {
		return this.#session.scene();
	}

	/** The quest journal: open quests first, then finished ones. */
	journal(): JournalEntry[] {
		return this.#session.journal();
	}

	/** The combat in progress, or null. */
	combat(): CombatView | null {
		return this.#session.combat();
	}

	/** The adventure sheet. Empty when the story has no `stats:`. */
	stats(): StatView[] {
		return this.#session.stats();
	}

	/** The active illustration, portrait, music and ambience. */
	media(): MediaView {
		return this.#session.media();
	}

	verbs(): VerbInfo[] {
		return this.#session.verbs();
	}

	/** What a command that is still being typed refers to. */
	preview(text: string): Preview {
		return this.#session.preview(text);
	}

	/** The talk in progress, or null. */
	talk(): Talk | null {
		return this.#session.talk();
	}

	/** The answers the player can choose now. Empty outside a talk. */
	choices(): Choice[] {
		return this.#session.choices();
	}

	/** The hint ladder of the current room: the steps not done yet whose needs are done. */
	hints(): Hint[] {
		return this.#session.hints();
	}

	/** Show the next hint level, of `step` or of the first step with a level left. It is no turn. */
	hint(step?: string): Step {
		return this.#session.hint(step);
	}
}

/**
 * A running game with the authoring views of the editor (the Workbench API). It owns a session; its play
 * methods return whole turns with the rule log, for the transcript.
 */
export class Workbench {
	#bench: WasmWorkbench;

	private constructor(bench: WasmWorkbench) {
		this.#bench = bench;
	}

	/** A new game of the files. Throws a `ProjectError` when the files have errors. */
	static async open(files: SourceFile[]): Promise<Workbench> {
		return new Workbench(await build(() => new WasmWorkbench(files)));
	}

	/** Give the memory of the engine back. Do not use the workbench after this. */
	free(): void {
		this.#bench.free();
	}

	get title(): string {
		return this.#bench.title;
	}

	/** The file of each entity, for example `room:shore` to `rooms/shore.yaml`. */
	origins(): Record<string, string> {
		return this.#bench.origins();
	}

	// ---- play, as whole turns ----

	/** The opening. A second call starts nothing again. */
	start(): Turn {
		return this.#bench.start();
	}

	play(input: Input): Turn {
		return this.#bench.play(input);
	}

	/** Answer the request of an `await` status, also the death prompt. Throws when the answer does not fit. */
	resume(awaitId: string, value: Value): Turn {
		return this.#bench.resume(awaitId, value);
	}

	/** Go back to the state before the last input. */
	undo(): Turn {
		return this.#bench.undo();
	}

	/** How many inputs `undo` can go back now. */
	undoSteps(): number {
		return this.#bench.undoSteps();
	}

	/** Start the story again. `undo` can go back to the state before it. */
	restart(): Turn {
		return this.#bench.restart();
	}

	author(command: AuthorCommand): Turn {
		return this.#bench.author(command);
	}

	// ---- the queries of the session ----

	/** What the session waits for now. */
	status(): Status {
		return this.#bench.status();
	}

	scene(): Scene {
		return this.#bench.scene();
	}

	/** The quest journal of the session. */
	journal(): JournalEntry[] {
		return this.#bench.journal();
	}

	/** The combat in progress, or null. */
	combat(): CombatView | null {
		return this.#bench.combat();
	}

	/** The adventure sheet. Empty when the story has no `stats:`. */
	stats(): StatView[] {
		return this.#bench.stats();
	}

	/** The active illustration, portrait, music and ambience. */
	media(): MediaView {
		return this.#bench.media();
	}

	verbs(): VerbInfo[] {
		return this.#bench.verbs();
	}

	preview(text: string): Preview {
		return this.#bench.preview(text);
	}

	/** The answers the player can choose now. Empty outside a talk. */
	choices(): Choice[] {
		return this.#bench.choices();
	}

	/** The hint ladder of the current room: the steps not done yet whose needs are done. */
	hints(): Hint[] {
		return this.#bench.hints();
	}

	/** The story has hints, also when none is open in this room now. */
	hasHints(): boolean {
		return this.#bench.hasHints();
	}

	/** Show the next hint level, of `step` or of the first step with a level left, as a turn with a note. It is no turn of the game. */
	hint(step?: string): Turn {
		return this.#bench.hint(step);
	}

	/** The save state, as JSON-compatible data. */
	save(): unknown {
		return this.#bench.save();
	}

	/** Continue from a save, also one from an earlier version of the story. Throws when the save cannot be read. */
	load(save: unknown): Loaded {
		return this.#bench.load(save);
	}

	/** Continue the game of an earlier version of the story in this one. Values still at their start value take the new start value. */
	carryOver(from: Workbench): void {
		this.#bench.carryOver(from.#bench);
	}

	// ---- views ----

	/** The talk in progress for the inspector, with the hidden options, or null. */
	talkView(): TalkView | null {
		return this.#bench.talkView();
	}

	/** Every dialogue as an outline of nodes, options and their targets. */
	dialogues(): DialogueOutline[] {
		return this.#bench.dialogues();
	}

	assembly(): FragmentView[] {
		return this.#bench.assembly();
	}

	roomParts(): RoomParts {
		return this.#bench.roomParts();
	}

	inspect(kind: 'thing' | 'exit', id: string): Inspect | null {
		return this.#bench.inspect(kind, id);
	}

	vars(): VarView[] {
		return this.#bench.vars();
	}

	phases(): PhaseView[] {
		return this.#bench.phases();
	}

	lastFrame(): FrameUse | null {
		return this.#bench.lastFrame();
	}

	/** Walk all reachable states from the start of the story. This can take a moment. */
	explore(maxStates = 2000, maxDepth = 40): Exploration {
		return this.#bench.explore(maxStates, maxDepth);
	}

	/** The text of a room in every combination of the values that the text depends on. */
	roomMatrix(room: string): RoomMatrix | null {
		return this.#bench.roomMatrix(room);
	}

	/** All rooms with what is in them now, for the Atlas. */
	atlas(): AtlasRoom[] {
		return this.#bench.atlas();
	}

	/** The dependency chart of each walkthrough with step ids, from `needs` (CK-7). */
	charts(): StepChart[] {
		return this.#bench.charts();
	}

	rules(): RuleView[] {
		return this.#bench.rules();
	}

	machines(): MachineView[] {
		return this.#bench.machines();
	}

	map(): MapView {
		return this.#bench.map();
	}

	/** Check fragment text and render it in the current state. */
	tryMarkup(text: string): MarkupCheck {
		return this.#bench.tryMarkup(text);
	}

	/** Check fragment text and render it as it would read in other values. It does not change the game. */
	tryMarkupIn(room: string, values: [string, string][], text: string): MarkupCheck {
		const check = this.#bench.tryMarkupIn(room, values, text) as MarkupCheck | null;
		return check ?? { error: { start: 0, end: 0, message: `The room ${room} does not exist.` }, text: '', pieces: [], holds: null };
	}

	/** Check a condition and tell whether it holds now. */
	tryCondition(text: string): MarkupCheck {
		return this.#bench.tryCondition(text);
	}

	/** Every name that conditions and markup can use, with its value now. */
	symbols(): { name: string; kind: string; detail: string; value: string | null; values: string[] }[] {
		return this.#bench.symbols();
	}

	/** One fragment text in all combinations of the values that it depends on. Null when it has a problem. */
	textMatrix(text: string, when: string | null): RoomMatrix | null {
		return this.#bench.textMatrix(text, when ?? undefined);
	}

	/** The room description in other values. It does not change the game. Null for an unknown room. */
	previewSituation(room: string, values: [string, string][]): Turn | null {
		return this.#bench.previewSituation(room, values);
	}

	/** The room description as it would show now. It does not change the game. */
	describe(): Turn {
		return this.#bench.describe();
	}
}
