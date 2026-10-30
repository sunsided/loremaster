// Where the story files are kept.
//
// - The working copy is in the Origin Private File System (OPFS) of the browser, one folder for each story.
// - On Chromium browsers, the author can link a folder on disk. Each change is then also written there.
// - A zip export and import moves stories in and out in all browsers.

import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import type { SourceFile } from '#lib/engine/types.ts';

const ROOT = 'lantern';
const isYaml = (name: string) => /\.ya?ml$/.test(name);

async function root(): Promise<FileSystemDirectoryHandle | null> {
	try {
		const opfs = await navigator.storage.getDirectory();
		return await opfs.getDirectoryHandle(ROOT, { create: true });
	} catch {
		return null;
	}
}

async function dirAt(base: FileSystemDirectoryHandle, parts: string[], create: boolean) {
	let dir = base;
	for (const part of parts) dir = await dir.getDirectoryHandle(part, { create });
	return dir;
}

/** All YAML files under a folder, with paths relative to it. */
export async function readFolder(dir: FileSystemDirectoryHandle, prefix = ''): Promise<SourceFile[]> {
	const out: SourceFile[] = [];
	for await (const [name, handle] of dir.entries()) {
		if (handle.kind === 'directory') out.push(...(await readFolder(handle as FileSystemDirectoryHandle, `${prefix}${name}/`)));
		else if (isYaml(name)) out.push({ path: `${prefix}${name}`, text: await (await (handle as FileSystemFileHandle).getFile()).text() });
	}
	return out.sort((a, b) => a.path.localeCompare(b.path));
}

/** Write one file under a folder. Missing folders are made. */
export async function writeFile(dir: FileSystemDirectoryHandle, path: string, text: string) {
	const parts = path.split('/');
	const name = parts.pop()!;
	const folder = await dirAt(dir, parts, true);
	const file = await folder.getFileHandle(name, { create: true });
	const writable = await file.createWritable();
	await writable.write(text);
	await writable.close();
}

/** Remove one file under a folder. */
export async function removeFile(dir: FileSystemDirectoryHandle, path: string) {
	const parts = path.split('/');
	const name = parts.pop()!;
	try {
		const folder = await dirAt(dir, parts, false);
		await folder.removeEntry(name);
	} catch {
		// The file is not there: nothing to remove.
	}
}

/** True when the browser has OPFS. */
export async function available(): Promise<boolean> {
	return (await root()) !== null;
}

/** The working copy of a story in OPFS. Null when there is none, or when OPFS is not available. */
export async function loadWorkingCopy(story: string): Promise<SourceFile[] | null> {
	const base = await root();
	if (!base) return null;
	try {
		const dir = await base.getDirectoryHandle(story);
		const files = await readFolder(dir);
		return files.length ? files : null;
	} catch {
		return null;
	}
}

/** Replace the working copy of a story with these files. */
export async function saveWorkingCopy(story: string, files: SourceFile[]): Promise<boolean> {
	const base = await root();
	if (!base) return false;
	try {
		await base.removeEntry(story, { recursive: true });
	} catch {
		// No working copy yet.
	}
	const dir = await base.getDirectoryHandle(story, { create: true });
	for (const f of files) await writeFile(dir, f.path, f.text);
	return true;
}

/** Write one file of the working copy. */
export async function saveWorkingFile(story: string, path: string, text: string): Promise<boolean> {
	const base = await root();
	if (!base) return false;
	await writeFile(await base.getDirectoryHandle(story, { create: true }), path, text);
	return true;
}

/** Remove one file of the working copy. */
export async function removeWorkingFile(story: string, path: string): Promise<boolean> {
	const base = await root();
	if (!base) return false;
	await removeFile(await base.getDirectoryHandle(story, { create: true }), path);
	return true;
}

/** Remove the working copy of a story, so that the bundled files show again. */
export async function dropWorkingCopy(story: string) {
	const base = await root();
	try {
		await base?.removeEntry(story, { recursive: true });
	} catch {
		// No working copy.
	}
}

/** Ask the browser to keep the working copy when storage runs low. */
export async function persist(): Promise<boolean> {
	try {
		return (await navigator.storage.persisted()) || (await navigator.storage.persist());
	} catch {
		return false;
	}
}

/** True when the browser can link a folder on disk (Chromium browsers). */
export const canLinkFolder = () => typeof window !== 'undefined' && 'showDirectoryPicker' in window;

/** Ask the author for a folder on disk, with write access. */
export async function pickFolder(): Promise<FileSystemDirectoryHandle> {
	const picker = (window as unknown as { showDirectoryPicker: (o: object) => Promise<FileSystemDirectoryHandle> })
		.showDirectoryPicker;
	return picker({ id: 'lantern-story', mode: 'readwrite' });
}

/** True when the page may write to the folder. Asks when `ask` is true; the browser needs a click for that. */
export async function folderAccess(dir: FileSystemDirectoryHandle, ask: boolean): Promise<boolean> {
	const h = dir as FileSystemDirectoryHandle & {
		queryPermission(o: object): Promise<PermissionState>;
		requestPermission(o: object): Promise<PermissionState>;
	};
	if ((await h.queryPermission({ mode: 'readwrite' })) === 'granted') return true;
	return ask && (await h.requestPermission({ mode: 'readwrite' })) === 'granted';
}

// The linked folder of each story is kept in IndexedDB: a folder handle can not go in local storage.
const DB = 'lantern';
const FOLDERS = 'folders';

function db(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const open = indexedDB.open(DB, 1);
		open.onupgradeneeded = () => open.result.createObjectStore(FOLDERS);
		open.onsuccess = () => resolve(open.result);
		open.onerror = () => reject(open.error);
	});
}

export async function rememberFolder(story: string, dir: FileSystemDirectoryHandle | null) {
	try {
		const d = await db();
		const tx = d.transaction(FOLDERS, 'readwrite');
		if (dir) tx.objectStore(FOLDERS).put(dir, story);
		else tx.objectStore(FOLDERS).delete(story);
	} catch {
		// Without IndexedDB the link lasts for this visit only.
	}
}

export async function linkedFolder(story: string): Promise<FileSystemDirectoryHandle | null> {
	try {
		const d = await db();
		return await new Promise((resolve) => {
			const get = d.transaction(FOLDERS).objectStore(FOLDERS).get(story);
			get.onsuccess = () => resolve((get.result as FileSystemDirectoryHandle) ?? null);
			get.onerror = () => resolve(null);
		});
	} catch {
		return null;
	}
}

/** The files as a zip archive. */
export function toZip(files: SourceFile[]): Uint8Array {
	return zipSync(Object.fromEntries(files.map((f) => [f.path, strToU8(f.text)])));
}

/** The YAML files in a zip archive. A single top folder (as made by "compress folder") is removed. */
export function fromZip(data: Uint8Array): SourceFile[] {
	const entries = Object.entries(unzipSync(data))
		.filter(([path]) => isYaml(path) && !path.startsWith('__MACOSX/'))
		.map(([path, bytes]) => ({ path, text: strFromU8(bytes) }));
	const tops = new Set(entries.map((e) => e.path.split('/')[0]));
	const single = tops.size === 1 && entries.every((e) => e.path.includes('/')) && !entries.some((e) => e.path.split('/')[0] === 'story.yaml');
	return entries
		.map((e) => (single ? { ...e, path: e.path.split('/').slice(1).join('/') } : e))
		.sort((a, b) => a.path.localeCompare(b.path));
}
