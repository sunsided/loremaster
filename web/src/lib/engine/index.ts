import init, { Engine as WasmEngine } from './pkg/lantern-wasm.js';
import type {
	AuthorCommand,
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
}
