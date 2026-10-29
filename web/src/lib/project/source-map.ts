// Where a fragment lives in the project files. The runtime names each fragment by its owner
// (`room:shore`, `thing:lamp/presence`, `room:shore/frames/look/before`, `story:intro`) and its
// index in that list. The loader knows the file of each entity.

/** A path into a YAML document: map keys and list indexes. */
export type YamlPath = (string | number)[];

/** The file and the YAML list that hold the fragments of one owner. */
export interface FragmentList {
	file: string;
	/** The path to the list, for example `['description']`. */
	list: YamlPath;
}

/** The file and the YAML list for a fragment owner. Null for an owner that is not a fragment list. */
export function listOf(owner: string, origins: Record<string, string>): FragmentList | null {
	const [entity, ...rest] = owner.split('/');
	if (entity === 'story:intro') return rest.length ? null : { file: storyFile(origins), list: ['intro'] };
	const file = entity.startsWith('story') ? storyFile(origins) : origins[entity];
	if (!file) return null;
	if (rest.length === 0) return { file, list: ['description'] };
	if (rest.length === 1 && rest[0] === 'presence') return { file, list: ['presence'] };
	if (rest.length === 3 && rest[0] === 'frames') return { file, list: ['frames', rest[1], rest[2]] };
	return null;
}

function storyFile(origins: Record<string, string>): string {
	return origins['story'] ?? 'story.yaml';
}
