import {
	Game,
	type Command,
	type FragmentView,
	type MapView,
	type Scene,
	type Turn,
	type VerbInfo
} from '#lib/engine/index.ts';

export interface Entry {
	id: number;
	turn: Turn;
}

/** A verb picked on the verb bar, waiting for its objects. */
export interface Pending {
	verb: VerbInfo;
	objects: { id: string; name: string }[];
}

/** One play session: the transcript, the scene, and the verb bar state. */
export class Session {
	game: Game;
	entries = $state<Entry[]>([]);
	scene = $state<Scene>() as Scene;
	assembly = $state<FragmentView[]>([]);
	map = $state<MapView>({ rooms: [], links: [] });
	verbs: VerbInfo[];
	pending = $state<Pending | null>(null);
	/** Fragment highlighted in both the transcript and the assembly panel. */
	focus = $state<number | null>(null);
	#next = 0;

	constructor(game: Game) {
		this.game = game;
		this.verbs = game.verbs();
		this.#push(game.start());
	}

	get barVerbs(): VerbInfo[] {
		const walk = this.verbs.find((v) => v.id === 'go');
		return [...(walk ? [walk] : []), ...this.verbs.filter((v) => v.bar)];
	}

	#push(turn: Turn) {
		this.entries.push({ id: this.#next++, turn });
		this.scene = this.game.scene();
		this.assembly = this.game.assembly();
		this.map = this.game.map();
	}

	input(text: string) {
		if (!text.trim()) return;
		this.pending = null;
		this.#push(this.game.input(text));
	}

	act(command: Command) {
		this.pending = null;
		this.#push(this.game.act(command));
	}

	pickVerb(verb: VerbInfo) {
		this.pending = this.pending?.verb.id === verb.id ? null : { verb, objects: [] };
	}

	/** A thing was clicked in the prose or in a list. */
	pickThing(id: string, name: string) {
		const p = this.pending;
		if (!p || p.verb.id === 'go') {
			this.act({ type: 'verb', verb: 'examine', objects: [id] });
			return;
		}
		const objects = [...p.objects, { id, name }];
		const want = p.verb.slots === 'two' ? 2 : 1;
		if (objects.length >= want) {
			this.act({ type: 'verb', verb: p.verb.id, objects: objects.map((o) => o.id) });
		} else {
			this.pending = { verb: p.verb, objects };
		}
	}

	/** Run a two-object verb with only its first object, e.g. "Use lamp". */
	runPending() {
		const p = this.pending;
		if (p && p.objects.length > 0) {
			this.act({ type: 'verb', verb: p.verb.id, objects: p.objects.map((o) => o.id) });
		}
	}

	pickExit(direction: string) {
		this.act({ type: 'go', direction });
	}

	restart() {
		this.game.reset();
		this.entries = [];
		this.pending = null;
		this.#push(this.game.start());
	}

	/** "Use oil lamp on _" */
	get sentence(): string {
		const p = this.pending;
		if (!p) return '';
		const parts = [p.verb.label, ...p.objects.map((o) => o.name)];
		if (p.verb.slots === 'two' && p.objects.length === 1) parts.push(p.verb.preposition ?? 'with');
		parts.push('_');
		return parts.join(' ');
	}
}
