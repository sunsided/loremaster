import type { SourceFile } from './types.ts';

// The example stories, bundled from the repo's examples folder. Each folder is one story.
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

export const lighthouse: SourceFile[] = examples.lighthouse;
