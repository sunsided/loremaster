import type { SourceFile } from './types.ts';

// The example stories, bundled from the repo's examples folder. Each folder with a
// story.yaml is one story. Other folders, such as the golden slice
// (examples/lock-keeper-slice), are not stories that the editor can open.
const raw = import.meta.glob('../../../../examples/*/**/*.yaml', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

/** Example stories by folder name, for example `lighthouse` and `cantina`. */
export const examples: Record<string, SourceFile[]> = {};
for (const [path, text] of Object.entries(raw)) {
	const match = path.match(/examples\/([^/]+)\/(.+)$/);
	if (!match) continue;
	(examples[match[1]] ??= []).push({ path: match[2], text });
}
for (const [name, files] of Object.entries(examples)) {
	if (!files.some((f) => f.path === 'story.yaml')) delete examples[name];
}

export const lighthouse: SourceFile[] = examples.lighthouse;
