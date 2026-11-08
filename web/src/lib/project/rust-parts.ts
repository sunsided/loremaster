// The YAML files that the editor writes for parts made in Rust: an eject and an override (DS-12).

import { stringify } from 'yaml';
import type { RustPart, SourceFile } from '#lib/engine/types.ts';

const FOLDERS: Record<string, string> = {
	room: 'rooms',
	thing: 'things',
	thread: 'threads',
	dialogue: 'dialogues',
	ending: 'endings',
	timer: 'timers',
	verb: 'verbs',
	machine: 'machines',
	agenda: 'agendas',
	rule: 'rules'
};

/** Where a YAML file for this entity goes. Null for what YAML cannot take over: the story header, behaviors and text pools. */
export function yamlPath(key: string): string | null {
	const [kind, id] = key.split(':');
	const folder = FOLDERS[kind];
	return folder && id ? `${folder}/${id}.yaml` : null;
}

/** Eject works in the folders of `replace: true`, which rules do not have. */
export function canEject(key: string): boolean {
	return yamlPath(key) !== null && !key.startsWith('rule:');
}

const yaml = (value: unknown) => stringify(value, { lineWidth: 0 });

/** The YAML file that takes the Rust part over for good: `replace: true` and the value. */
export function ejectFile(key: string, part: RustPart): SourceFile {
	return { path: yamlPath(key)!, text: `replace: true\n${yaml(part.value)}` };
}

/** The YAML file that overrides the Rust part: the block `override:` with its base, and the value. */
export function overrideFile(key: string, part: RustPart): SourceFile {
	const { file, line, hash } = part.origin;
	const override = {
		entity: key,
		file,
		line,
		base_hash: hash,
		...(part.commit ? { commit: part.commit } : {}),
		base: part.value
	};
	return { path: yamlPath(key)!, text: yaml({ override }) + yaml(part.value) };
}
