import init, { Engine as WasmEngine } from './pkg/lantern-wasm.js';
import type {
	AuthorCommand,
	Exploration,
	RoomMatrix,
	FrameUse,
	MarkupCheck,
	Inspect,
	PhaseView,
	VarView,
	Command,
	Diagnostic,
	FragmentView,
	MapView,
	Preview,
	RoomParts,
	RuleView,
	Scene,
	SourceFile,
	Turn,
	VerbInfo
} from './types.ts';

export type * from './types.ts';

let ready: Promise<unknown> | undefined;

/** Load the WASM module once. */
export function loadEngine(): Promise<unknown> {
	ready ??= init();
	return ready;
}

/** Thrown when a project has errors; carries the diagnostics. */
export class ProjectError extends Error {
	constructor(public diagnostics: Diagnostic[]) {
		super(diagnostics.map((d) => `${d.file}: ${d.message}`).join('\n'));
	}
}

/** A typed wrapper around the WASM engine. */
export class Game {
	#engine: WasmEngine;

	private constructor(engine: WasmEngine) {
		this.#engine = engine;
	}

	static async open(files: SourceFile[]): Promise<Game> {
		await loadEngine();
		try {
			return new Game(new WasmEngine(files));
		} catch (e) {
			if (Array.isArray(e)) throw new ProjectError(e as Diagnostic[]);
			throw e;
		}
	}

	get title(): string {
		return this.#engine.title();
	}

	start(): Turn {
		return this.#engine.start();
	}

	author(command: AuthorCommand): Turn {
		return this.#engine.author(command);
	}

	inspect(kind: 'thing' | 'exit', id: string): Inspect | null {
		return this.#engine.inspect(kind, id);
	}

	lastFrame(): FrameUse | null {
		return this.#engine.lastFrame();
	}

	/** Walk all reachable states from the start of the story. This can take a moment. */
	explore(maxStates = 2000, maxDepth = 40): Exploration {
		return this.#engine.explore(maxStates, maxDepth);
	}

	/** The text of a room in every combination of the values that the text depends on. */
	roomMatrix(room: string): RoomMatrix | null {
		return this.#engine.roomMatrix(room);
	}

	vars(): VarView[] {
		return this.#engine.vars();
	}

	phases(): PhaseView[] {
		return this.#engine.phases();
	}

	preview(text: string): Preview {
		return this.#engine.preview(text);
	}

	input(text: string): Turn {
		return this.#engine.input(text);
	}

	act(command: Command): Turn {
		return this.#engine.act(command);
	}

	scene(): Scene {
		return this.#engine.scene();
	}

	assembly(): FragmentView[] {
		return this.#engine.assembly();
	}

	roomParts(): RoomParts {
		return this.#engine.roomParts();
	}

	rules(): RuleView[] {
		return this.#engine.rules();
	}

	map(): MapView {
		return this.#engine.map();
	}

	verbs(): VerbInfo[] {
		return this.#engine.verbs();
	}

	reset(): void {
		this.#engine.reset();
	}

	/** The file of each entity, for example `room:shore` to `rooms/shore.yaml`. */
	origins(): Record<string, string> {
		return this.#engine.origins();
	}

	/** The room description as it would show now. It does not change the game. */
	describe(): Turn {
		return this.#engine.describe();
	}

	/** Continue the game of an earlier version of the story in this one. Values still at their start value take the new start value. */
	carryOver(from: Game): void {
		this.#engine.carryOver(from.#engine);
	}

	/** Highlighted spans of fragment text, or of a condition. Offsets are UTF-16, as in JavaScript strings. */
	tokens(text: string, condition: boolean): { from: number; to: number; kind: string }[] {
		return this.#engine.tokens(text, condition);
	}

	/** Check fragment text and render it in the current state. */
	tryMarkup(text: string): MarkupCheck {
		return this.#engine.tryMarkup(text);
	}

	/** Check a condition and tell whether it holds now. */
	tryCondition(text: string): MarkupCheck {
		return this.#engine.tryCondition(text);
	}

	/** Every name that conditions and markup can use, with its value now. */
	symbols(): { name: string; kind: string; detail: string; value: string | null; values: string[] }[] {
		return this.#engine.symbols();
	}

	/** One fragment text in all combinations of the values that it depends on. Null when it has a problem. */
	textMatrix(text: string, when: string | null): RoomMatrix | null {
		return this.#engine.textMatrix(text, when ?? undefined);
	}

	/** The room description in other values. It does not change the game. Null for an unknown room. */
	previewSituation(room: string, values: [string, string][]): Turn | null {
		return this.#engine.previewSituation(room, values);
	}

	/** The save state, as JSON-compatible data. */
	save(): unknown {
		return this.#engine.save();
	}

	/** Continue from a save state, also one from an earlier version of the story. */
	restore(state: unknown): void {
		this.#engine.restore(state);
	}
}
