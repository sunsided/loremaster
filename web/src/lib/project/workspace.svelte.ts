// The story that the author plays and edits: the project files and the play session on them.

import { Game, ProjectError, type Diagnostic, type SourceFile } from '#lib/engine/index.ts';
import { Session } from '#lib/player/session.svelte.ts';
import { Project } from './project.svelte.ts';
import { fromZip } from './store.ts';

export class Workspace {
	readonly project: Project;
	session = $state.raw<Session | null>(null);
	/** Problems of the files that the game could not build. The game keeps the last good build. */
	problems = $state<Diagnostic[]>([]);

	private constructor(project: Project) {
		this.project = project;
	}

	/** Open a story by name: its working copy, else the bundled files. */
	static async open(name: string, bundled: SourceFile[]): Promise<Workspace> {
		const ws = new Workspace(await Project.open(name, bundled));
		await ws.#start(ws.project.files);
		return ws;
	}

	/** Build the files and start a new game. Keeps the old game when the files have errors. */
	async #start(files: SourceFile[]): Promise<boolean> {
		const game = await this.#build(files);
		if (!game) return false;
		this.project.origins = game.origins();
		this.session = new Session(game);
		return true;
	}

	async #build(files: SourceFile[]): Promise<Game | null> {
		try {
			const game = await Game.open(files);
			this.problems = [];
			return game;
		} catch (e) {
			if (e instanceof ProjectError) {
				this.problems = e.diagnostics;
				return null;
			}
			throw e;
		}
	}

	/**
	 * Change one file. The story is built again and the game continues in it, with its state.
	 * Returns the problems when the new text does not build; then nothing is saved.
	 */
	async change(path: string, text: string): Promise<Diagnostic[] | null> {
		return this.changeMany([{ path, text }]);
	}

	/** Change or add several files at once, for example a new room and the exit to it. */
	async changeMany(changes: SourceFile[]): Promise<Diagnostic[] | null> {
		let files = this.project.files;
		for (const c of changes) files = [...files.filter((f) => f.path !== c.path), c];
		const game = await this.#build(files);
		if (!game) return this.problems;
		for (const c of changes) await this.project.write(c.path, c.text);
		this.project.origins = game.origins();
		this.session?.carryOver(game);
		return null;
	}

	/** Link a folder on disk and play the story in it. */
	async linkFolder() {
		await this.#start(await this.project.linkFolder());
	}

	/** Ask again for the folder access, after a reload of the page. */
	async reconnect() {
		const files = await this.project.reconnect();
		if (files) await this.#start(files);
	}

	async unlinkFolder() {
		await this.project.unlinkFolder();
	}

	/** Load the story from a zip archive. */
	async importZip(file: File) {
		const files = fromZip(new Uint8Array(await file.arrayBuffer()));
		const game = await this.#build(files);
		if (!game) return;
		await this.project.replaceAll(files);
		this.project.origins = game.origins();
		this.session = new Session(game);
	}

	/** Go back to the bundled example. */
	async reset() {
		await this.project.resetToBundled();
		await this.#start(this.project.files);
	}
}
