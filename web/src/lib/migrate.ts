// The project was called lantern before. Copy the browser storage of that name once, so that settings
// and stories stay. The old entries stay too, so an older build still finds them.
//
// - localStorage: each `lantern.*` key becomes `loremaster.*`, unless the new key exists.
// - OPFS: the folder `lantern` is copied to `loremaster` (see `store.ts`), unless the new folder has files.

const OLD = 'lantern.';
const NEW = 'loremaster.';

try {
	for (let i = 0; i < localStorage.length; i++) {
		const key = localStorage.key(i);
		if (!key?.startsWith(OLD)) continue;
		const target = NEW + key.slice(OLD.length);
		if (localStorage.getItem(target) === null) localStorage.setItem(target, localStorage.getItem(key) ?? '');
	}
} catch {
	// Private mode or blocked storage: nothing to copy.
}

/** Copy all files and folders of `from` into `to`. */
async function copyDir(from: FileSystemDirectoryHandle, to: FileSystemDirectoryHandle) {
	for await (const [name, handle] of from.entries()) {
		if (handle.kind === 'directory') {
			await copyDir(handle as FileSystemDirectoryHandle, await to.getDirectoryHandle(name, { create: true }));
		} else {
			const file = await (handle as FileSystemFileHandle).getFile();
			const writable = await (await to.getFileHandle(name, { create: true })).createWritable();
			await writable.write(await file.arrayBuffer());
			await writable.close();
		}
	}
}

/** Copy the OPFS folder of the old name into `root`, when `root` is empty and the old folder exists. */
export async function migrateOpfs(opfs: FileSystemDirectoryHandle, root: FileSystemDirectoryHandle) {
	for await (const _ of root.keys()) return;
	let old: FileSystemDirectoryHandle;
	try {
		old = await opfs.getDirectoryHandle('lantern');
	} catch {
		return;
	}
	await copyDir(old, root);
}
