import type { SourceFile } from './types.ts';

// The lighthouse example, bundled from the repo's examples folder.
const raw = import.meta.glob('../../../../examples/lighthouse/**/*.yaml', {
	query: '?raw',
	import: 'default',
	eager: true
}) as Record<string, string>;

export const lighthouse: SourceFile[] = Object.entries(raw).map(([path, text]) => ({
	path: path.replace(/^.*examples\/lighthouse\//, ''),
	text
}));
