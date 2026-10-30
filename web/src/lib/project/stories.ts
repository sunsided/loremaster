// The stories that the author made, by name. Each name has its own working copy, as the examples have.

export interface StoryEntry {
	/** The working copy name, made from the title, for example `the_drowned_bell`. */
	name: string;
	title: string;
}

const KEY = 'lantern.myStories';

/** The stories that the author made in this browser, newest first. */
export function myStories(): StoryEntry[] {
	try {
		const list = JSON.parse(localStorage.getItem(KEY) ?? '[]');
		return Array.isArray(list) ? list.filter((s) => typeof s?.name === 'string' && typeof s?.title === 'string') : [];
	} catch {
		return [];
	}
}

/** Add a story. The name comes from the title and is not one of `taken`. */
export function addStory(title: string, taken: string[]): StoryEntry {
	const base =
		title
			.toLowerCase()
			.normalize('NFKD')
			.replace(/[^a-z0-9]+/g, '_')
			.replace(/^_+|_+$/g, '') || 'story';
	const used = new Set([...taken, ...myStories().map((s) => s.name)]);
	let name = base;
	for (let i = 2; used.has(name); i++) name = `${base}_${i}`;
	const entry = { name, title };
	try {
		localStorage.setItem(KEY, JSON.stringify([entry, ...myStories()]));
	} catch {
		// Storage is not available: the story lasts for this visit only.
	}
	return entry;
}
