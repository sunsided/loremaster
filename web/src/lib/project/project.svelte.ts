// The story files that the author edits, and where they are kept.

import type { SourceFile } from '#lib/engine/types.ts';
import { listOf, type FragmentList } from './source-map.ts';
import * as store from './store.ts';
import { readFragment, type FragmentData } from './yaml-edit.ts';

/** Where the changes go. */
export type Storage =
	/** In the working copy of the browser (OPFS). */
	| 'browser'
	/** In the working copy and in a linked folder on disk. */
	| 'folder'
	/** The linked folder needs permission again, after a reload of the page. */
	| 'folder-locked'
	/** Only in memory: the browser has no OPFS. Export a zip to keep the changes. */
	| 'memory';

export class Project {
	/** The story name, for example `lighthouse`. Each name has its own working copy. */
	readonly name: string;
	files = $state<SourceFile[]>([]);
	/** The file of each entity, from the last good build of the game. */
	origins = $state<Record<string, string>>({});
	storage = $state<Storage>('memory');
	/** The name of the linked folder. */
	folderName = $state<string | null>(null);
	/** True when the files differ from the bundled example. */
	edited = $state(false);
	#folder: FileSystemDirectoryHandle | null = null;
	/** The working copy is made at the first change, so that a story without changes follows the bundled files. */
	#hasCopy = false;
	#bundled: SourceFile[];

	private constructor(name: string, bundled: SourceFile[]) {
		this.name = name;
		this.#bundled = bundled;
	}

	/** Open a story: the working copy when there is one, else the bundled files. */
	static async open(name: string, bundled: SourceFile[]): Promise<Project> {
		const p = new Project(name, bundled);
		const copy = await store.loadWorkingCopy(name);
		p.files = copy ?? bundled;
		p.edited = copy !== null && !same(copy, bundled);
		p.#hasCopy = copy !== null;
		p.storage = (await store.available()) ? 'browser' : 'memory';
		const folder = await store.linkedFolder(name);
		if (folder) {
			p.#folder = folder;
			p.folderName = folder.name;
			p.storage = (await store.folderAccess(folder, false)) ? 'folder' : 'folder-locked';
			if (p.storage === 'folder') p.files = await store.readFolder(folder);
		}
		return p;
	}

	file(path: string): SourceFile | undefined {
		return this.files.find((f) => f.path === path);
	}

	/** The files with one file changed or added. It does not change the project. */
	withFile(path: string, text: string): SourceFile[] {
		const others = this.files.filter((f) => f.path !== path);
		return [...others, { path, text }].sort((a, b) => a.path.localeCompare(b.path));
	}

	/** Where the fragments of an owner are, for example `room:shore`. */
	listOf(owner: string): FragmentList | null {
		return listOf(owner, this.origins);
	}

	/** The fragment as written in the file. */
	fragment(owner: string, index: number): (FragmentData & FragmentList) | null {
		const at = this.listOf(owner);
		const file = at && this.file(at.file);
		if (!at || !file) return null;
		try {
			return { ...readFragment(file.text, at.list, index), ...at };
		} catch {
			return null;
		}
	}

	/** Keep a changed file: in memory, in the working copy and in the linked folder. */
	async write(path: string, text: string) {
		this.files = this.withFile(path, text);
		this.edited = !same(this.files, this.#bundled);
		if (this.storage !== 'memory') {
			if (this.#hasCopy) await store.saveWorkingFile(this.name, path, text);
			else this.#hasCopy = await store.saveWorkingCopy(this.name, this.files);
		}
		if (this.storage === 'folder' && this.#folder) await store.writeFile(this.#folder, path, text);
	}

	/**
	 * Link a folder on disk. A folder with a story in it becomes the source; an empty folder gets the current files.
	 * Returns the files to load.
	 */
	async linkFolder(): Promise<SourceFile[]> {
		const folder = await store.pickFolder();
		const there = await store.readFolder(folder);
		if (there.some((f) => f.path === 'story.yaml')) this.files = there;
		else for (const f of this.files) await store.writeFile(folder, f.path, f.text);
		this.#folder = folder;
		this.folderName = folder.name;
		this.storage = 'folder';
		await store.rememberFolder(this.name, folder);
		this.#hasCopy = await store.saveWorkingCopy(this.name, this.files);
		await store.persist();
		this.edited = !same(this.files, this.#bundled);
		return this.files;
	}

	/** Ask again for access to the linked folder. The browser needs a click for this. Returns the folder files. */
	async reconnect(): Promise<SourceFile[] | null> {
		if (!this.#folder || !(await store.folderAccess(this.#folder, true))) return null;
		this.storage = 'folder';
		this.files = await store.readFolder(this.#folder);
		this.#hasCopy = await store.saveWorkingCopy(this.name, this.files);
		return this.files;
	}

	async unlinkFolder() {
		this.#folder = null;
		this.folderName = null;
		await store.rememberFolder(this.name, null);
		this.storage = (await store.available()) ? 'browser' : 'memory';
	}

	/** Replace all files, for example from a zip. */
	async replaceAll(files: SourceFile[]) {
		this.files = files;
		this.edited = !same(files, this.#bundled);
		if (this.storage !== 'memory') this.#hasCopy = await store.saveWorkingCopy(this.name, files);
		if (this.storage === 'folder' && this.#folder) for (const f of files) await store.writeFile(this.#folder, f.path, f.text);
	}

	/** Go back to the bundled example. A linked folder is not changed. */
	async resetToBundled() {
		await store.dropWorkingCopy(this.name);
		this.#hasCopy = false;
		await this.unlinkFolder();
		this.files = this.#bundled;
		this.edited = false;
	}

	zip(): Uint8Array {
		return store.toZip(this.files);
	}
}

function same(a: SourceFile[], b: SourceFile[]): boolean {
	if (a.length !== b.length) return false;
	const byPath = new Map(b.map((f) => [f.path, f.text]));
	return a.every((f) => byPath.get(f.path) === f.text);
}
