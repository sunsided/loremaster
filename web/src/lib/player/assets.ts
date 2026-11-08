// The media files of the bundled example stories.

const urls = import.meta.glob('../../../../examples/*/assets/*.{svg,png,jpg,jpeg,webp,wav,ogg,mp3}', {
	query: '?url&no-inline',
	import: 'default',
	eager: true
}) as Record<string, string>;

/**
 * The URL of an asset file, or null when the player has no such file.
 * shortcut: only bundled examples have media bytes, a linked folder or a zip shows the captions; upgrade when SourceFile can hold binary files.
 */
export function assetUrl(story: string, file: string): string | null {
	return urls[`../../../../examples/${story}/${file}`] ?? null;
}
