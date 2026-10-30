import {
	Game,
	type AuthorCommand,
	type FrameUse,
	type PhaseView,
	type VarView,
	type Command,
	type FragmentView,
	type MapView,
	type Preview,
	type RoomParts,
	type RuleView,
	type Scene,
	type Turn,
	type VerbInfo
} from '#lib/engine/index.ts';
import type { StoryStep } from '#lib/design/types.ts';

export interface Entry {
	id: number;
	turn: Turn;
	/** True when an edit of the story wrote the room text of this turn again. */
	rewritten?: boolean;
}

/** One play session: the transcript, the scene, and the verb bar state. */
export class Session {
	game = $state.raw<Game>() as Game;
	entries = $state<Entry[]>([]);
	scene = $state<Scene>() as Scene;
	assembly = $state<FragmentView[]>([]);
	map = $state<MapView>({ rooms: [], links: [] });
	parts = $state<RoomParts>({ objects: [], exits: [] });
	rules = $state<RuleView[]>([]);
	vars = $state<VarView[]>([]);
	/** The action frame used for the last action, or null. */
	frame = $state<FrameUse | null>(null);
	phases: PhaseView[] = [];
	/** The playthrough so far, one step per turn. */
	history = $state<StoryStep[]>([]);
	verbs: VerbInfo[];
	/**
	 * The command in the prompt. It is the one source for the prompt and the verb bar:
	 * typing changes it, and clicks on verbs and things write to it.
	 */
	draft = $state('');
	/** What the draft refers to so far. It changes with the draft and after each turn. */
	preview = $derived.by<Preview>(() => {
		this.entries.length;
		return this.game.preview(this.draft);
	});
	/** Fragment highlighted in both the transcript and the assembly panel. */
	focus = $state<number | null>(null);
	#next = 0;

	/** `first` is the first turn to show; by default the game starts from its beginning. */
	constructor(game: Game, first?: Turn) {
		this.game = game;
		this.verbs = game.verbs();
		this.phases = game.phases();
		this.#push(first ?? game.start());
	}

	/**
	 * A sandbox: this game state in another build of the story, for example the files of an AI proposal.
	 * The new session starts with the room text; this session does not change.
	 */
	sandbox(game: Game): Session {
		game.carryOver(this.game);
		return new Session(game, game.describe());
	}

	/** The verb bar: Walk to and Look around first, then the verbs that take things. */
	get barVerbs(): VerbInfo[] {
		const first = ['go', 'look'].flatMap((id) => this.verbs.filter((v) => v.id === id));
		return [...first, ...this.verbs.filter((v) => v.bar && !first.includes(v))];
	}

	#push(turn: Turn) {
		this.entries.push({ id: this.#next++, turn });
		this.#refresh();
		const command = turn.blocks.find((b) => b.kind === 'command' || b.kind === 'author');
		this.history.push({
			turn: this.scene.turns,
			time: this.scene.time,
			phase: this.scene.phase,
			room: this.scene.room_name,
			command: command
				? `${command.kind === 'author' ? '(author) ' : ''}${command.runs.map((r) => r.text).join('')}`
				: '',
			rules: turn.log.filter((l) => l.kind === 'rule').map((l) => l.text),
			changes: turn.log.filter((l) => l.kind !== 'rule').map((l) => l.text),
			ended: turn.ended
		});
	}

	/**
	 * Continue in a new build of the story, after an edit. The state carries over, and the room text of the
	 * last turn is written again with the new story. Older turns stay as they were played.
	 */
	carryOver(game: Game) {
		game.carryOver(this.game);
		this.game = game;
		this.verbs = game.verbs();
		this.phases = game.phases();
		this.#refresh();
		// The last turn that showed a room text, when that room is still the current room.
		const at = this.entries.findLastIndex((e) => e.turn.blocks.some((b) => b.kind === 'title'));
		const last = this.entries[at];
		if (!last) return;
		const blocks = last.turn.blocks;
		const start = blocks.findLastIndex((b) => b.kind === 'title');
		if (blocks[start].runs.map((r) => r.text).join('') !== this.scene.room_name) return;
		let end = start + 1;
		while (end < blocks.length && ['prose', 'objects', 'exits'].includes(blocks[end].kind)) end++;
		const fresh = game.describe().blocks;
		this.entries[at] = {
			...last,
			turn: { ...last.turn, blocks: [...blocks.slice(0, start), ...fresh, ...blocks.slice(end)] },
			rewritten: true
		};
	}

	#refresh() {
		this.scene = this.game.scene();
		this.assembly = this.game.assembly();
		this.map = this.game.map();
		this.parts = this.game.roomParts();
		this.rules = this.game.rules();
		this.vars = this.game.vars();
		this.frame = this.game.lastFrame();
	}

	input(text: string) {
		if (!text.trim()) return;
		this.draft = '';
		this.#push(this.game.input(text));
	}

	/** Run an author command, for example from the command palette. */
	author(command: AuthorCommand) {
		this.#push(this.game.author(command));
	}

	act(command: Command) {
		this.draft = '';
		this.#push(this.game.act(command));
	}

	/** The verb that the draft names, if any. */
	get verb(): VerbInfo | undefined {
		return this.verbs.find((v) => v.id === this.preview.verb);
	}

	#name(id: string): string {
		return [...this.scene.things, ...this.scene.inventory].find((t) => t.id === id)?.name ?? id;
	}

	/** The words that write a verb into the prompt, for example "look at" or "pick up". */
	#phrase(verb: VerbInfo): string {
		return verb.label.toLowerCase();
	}

	/** A verb on the verb bar was clicked: write it into the prompt, or clear the prompt if it is there already. */
	pickVerb(verb: VerbInfo) {
		// A verb without things, such as "Look around", runs at once.
		if (verb.slots === 'none') {
			this.input(this.#phrase(verb));
			return;
		}
		const onlyVerb = this.preview.verb === verb.id && this.preview.objects.length === 0;
		this.draft = onlyVerb ? '' : `${this.#phrase(verb)} `;
	}

	/**
	 * A thing was clicked in the prose or in a list. With no verb in the prompt, look at it.
	 * Otherwise add it to the command. Run the command when it has all its things.
	 */
	pickThing(id: string, name: string) {
		const verb = this.verb;
		if (!verb || verb.id === 'go' || verb.slots === 'none') {
			this.input(`look at ${name}`);
			return;
		}
		const names = [...this.preview.objects.map((o) => this.#name(o)), name];
		if (verb.slots === 'two' && names.length === 1) {
			this.draft = `${this.#phrase(verb)} ${names[0]} ${verb.preposition ?? 'with'} `;
			return;
		}
		const command =
			verb.slots === 'two'
				? `${this.#phrase(verb)} ${names[0]} ${verb.preposition ?? 'with'} ${names[1]}`
				: `${this.#phrase(verb)} ${names[0]}`;
		this.input(command);
	}

	/** Run a two-object verb with only its first thing, for example "use oil lamp". */
	runPending() {
		const verb = this.verb;
		const first = this.preview.objects[0];
		if (verb && first) this.input(`${this.#phrase(verb)} ${this.#name(first)}`);
	}

	/**
	 * Start again and play a path from the state explorer. A step is `go <direction>`
	 * or `<verb> <thing ids>`.
	 */
	replay(path: string[]) {
		this.restart();
		for (const step of path) {
			const [verb, ...objects] = step.split(' ');
			this.act(verb === 'go' ? { type: 'go', direction: objects[0] } : { type: 'verb', verb, objects });
		}
	}

	pickExit(direction: string) {
		this.act({ type: 'go', direction });
	}

	/**
	 * The verb that a click on a thing or an exit would use now: the verb in the prompt if it
	 * applies, else "Walk to" for an exit and "Look at" for a thing.
	 */
	clickVerb(kind: 'thing' | 'exit'): string {
		if (kind === 'exit') return 'go';
		const verb = this.verb;
		return verb && verb.id !== 'go' && verb.slots !== 'none' ? verb.id : 'examine';
	}

	/** Things the draft names or could still name, for highlights on the verb bar. */
	get named(): string[] {
		return [...this.preview.objects, ...this.preview.candidates];
	}

	/** True when a two-object verb has its first thing and could also run with only that one. */
	get canRunPartial(): boolean {
		return this.verb?.slots === 'two' && this.preview.objects.length === 1;
	}

	/** The command as the game reads it so far, for example "Use oil lamp on _". */
	get sentence(): string {
		const verb = this.verb;
		if (!verb) return '';
		const parts = [verb.label, ...this.preview.objects.map((o) => this.#name(o))];
		if (verb.id === 'go' && this.preview.direction) parts.push(this.preview.direction);
		if (verb.slots === 'two' && this.preview.objects.length === 1) parts.push(verb.preposition ?? 'with');
		if (!this.preview.complete) parts.push('_');
		return parts.join(' ');
	}

	restart() {
		this.game.reset();
		this.entries = [];
		this.history = [];
		this.draft = '';
		this.#push(this.game.start());
	}
}
