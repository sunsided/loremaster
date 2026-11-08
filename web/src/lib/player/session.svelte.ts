import {
	Workbench,
	type AuthorCommand,
	type CombatView,
	type RollDetail,
	type StatView,
	type Awaiting,
	type Input,
	type MediaCue,
	type MediaView,
	type Status,
	type Value,
	type Choice,
	type DialogueOutline,
	type TalkInfo,
	type TalkView,
	type FrameUse,
	type PhaseView,
	type VarView,
	type Command,
	type FragmentView,
	type Hint,
	type MapView,
	type StepChart,
	type Preview,
	type RoomParts,
	type RuleView,
	type MachineView,
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

/** One play session: the transcript, the scene, and the verb bar state. It plays through the engine's workbench. */
export class Session {
	bench = $state.raw<Workbench>() as Workbench;
	/** What the game waits for: the next command, an answer to a paused rule or the death prompt, or nothing (ended). */
	status = $state<Status>({ type: 'ready' });
	/** How many turns `undo` can go back. */
	undoSteps = $state(0);
	entries = $state<Entry[]>([]);
	/** Counts the turns of the game, also author commands and undo. A hint is no turn and does not count. */
	turns = $state(0);
	scene = $state<Scene>() as Scene;
	assembly = $state<FragmentView[]>([]);
	map = $state<MapView>({ rooms: [], links: [] });
	parts = $state<RoomParts>({ objects: [], exits: [] });
	rules = $state<RuleView[]>([]);
	machines = $state<MachineView[]>([]);
	vars = $state<VarView[]>([]);
	/** The adventure sheet. Empty when the story has no `stats:`. */
	stats = $state<StatView[]>([]);
	/** The combat in progress, or null. */
	combat = $state<CombatView | null>(null);
	/** The dice of the newest turn. */
	rolls = $derived<RollDetail[]>(this.entries.at(-1)?.turn.rolls ?? []);
	/** The active illustration, portrait, music and ambience. */
	media = $state.raw<MediaView>({});
	/** The newest sound captions, oldest first. */
	captions = $state<{ id: number; text: string }[]>([]);
	#onsound: ((sound: MediaCue) => void) | null = null;
	/** Sounds played before the host set `onsound`, such as those of the start turn. */
	#unheard: MediaCue[] = [];
	/** The host plays a sound effect here. Sounds from before it is set play when it is set. */
	get onsound() {
		return this.#onsound;
	}
	set onsound(play) {
		this.#onsound = play;
		if (play) for (const sound of this.#unheard.splice(0)) play(sound);
	}
	/** The action frame used for the last action, or null. */
	frame = $state<FrameUse | null>(null);
	/** The talk in progress, or null. While it is open, only answers and Leave run. */
	talk = $state<TalkInfo | null>(null);
	/** The answers the player can choose now. */
	choices = $state<Choice[]>([]);
	/** The talk in progress for the inspector, with the hidden options. */
	talkView = $state<TalkView | null>(null);
	/** Every dialogue of the story, as an outline. */
	dialogues = $state<DialogueOutline[]>([]);
	/** The hint ladder of the current room (CK-6). */
	hints = $state<Hint[]>([]);
	/** The dependency charts of the walkthroughs (CK-7). They change only with the story. */
	charts = $state.raw<StepChart[]>([]);
	/** The story has hints, also when none is open in this room now. */
	hasHints = $state(false);
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
		return this.bench.preview(this.draft);
	});
	/** Fragment highlighted in both the transcript and the assembly panel. */
	focus = $state<number | null>(null);
	#next = 0;

	/** `first` is the first turn to show; by default the game starts from its beginning. */
	constructor(bench: Workbench, first?: Turn) {
		this.bench = bench;
		this.verbs = bench.verbs();
		this.phases = bench.phases();
		this.charts = bench.charts();
		this.hasHints = bench.hasHints();
		this.#push(first ?? bench.start());
	}

	/**
	 * A sandbox: this game state in another build of the story, for example the files of an AI proposal.
	 * The new session starts with the room text; this session does not change.
	 */
	sandbox(bench: Workbench): Session {
		bench.carryOver(this.bench);
		return new Session(bench, bench.describe());
	}

	/** The verb bar: Walk to and Look around first, then the verbs that take things. */
	get barVerbs(): VerbInfo[] {
		const first = ['go', 'look'].flatMap((id) => this.verbs.filter((v) => v.id === id));
		return [...first, ...this.verbs.filter((v) => v.bar && !first.includes(v))];
	}

	#push(turn: Turn) {
		this.turns++;
		this.entries.push({ id: this.#next++, turn });
		const before = this.media;
		this.#refresh();
		this.#sounds(turn, before);
		// A chosen answer in a talk has no typed command: the answer is the command.
		const command = turn.blocks.find((b) => b.kind === 'command' || b.kind === 'author' || b.kind === 'said');
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

	/** Captions for the sounds of a turn and for music or ambience that changed; the sounds go to the host. */
	#sounds(turn: Turn, before: MediaView) {
		const changed = (['music', 'ambience'] as const).flatMap((slot) => {
			const cue = this.media[slot];
			return cue && cue.id !== before[slot]?.id ? [cue] : [];
		});
		const sounds = turn.sounds ?? [];
		if (this.#onsound) for (const sound of sounds) this.#onsound(sound);
		else this.#unheard.push(...sounds);
		const texts = [...changed, ...sounds].flatMap((c) => (c.caption ? [c.caption] : []));
		this.captions = [...this.captions, ...texts.map((text) => ({ id: this.#next++, text }))].slice(-3);
	}

	/**
	 * Continue in a new build of the story, after an edit. The state carries over, and the room text of the
	 * last turn is written again with the new story. Older turns stay as they were played.
	 */
	carryOver(bench: Workbench) {
		bench.carryOver(this.bench);
		this.bench = bench;
		this.verbs = bench.verbs();
		this.phases = bench.phases();
		this.charts = bench.charts();
		this.hasHints = bench.hasHints();
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
		const fresh = bench.describe().blocks;
		this.entries[at] = {
			...last,
			turn: { ...last.turn, blocks: [...blocks.slice(0, start), ...fresh, ...blocks.slice(end)] },
			rewritten: true
		};
	}

	#refresh() {
		this.scene = this.bench.scene();
		this.status = this.bench.status();
		this.undoSteps = this.bench.undoSteps();
		this.assembly = this.bench.assembly();
		this.map = this.bench.map();
		this.parts = this.bench.roomParts();
		this.rules = this.bench.rules();
		this.machines = this.bench.machines();
		this.vars = this.bench.vars();
		this.stats = this.bench.stats();
		this.media = this.bench.media();
		this.combat = this.bench.combat();
		this.frame = this.bench.lastFrame();
		this.choices = this.bench.choices();
		this.talkView = this.bench.talkView();
		this.talk = this.talkView
			? { dialogue: this.talkView.dialogue, with: this.talkView.with, name: this.talkView.name, node: this.talkView.node }
			: null;
		this.dialogues = this.bench.dialogues();
		this.hints = this.bench.hints();
	}

	/** Play one input. */
	#play(input: Input) {
		this.draft = '';
		this.#push(this.bench.play(input));
	}

	input(text: string) {
		if (!text.trim()) return;
		this.#play({ type: 'text', value: text });
	}

	/** Run an author command, for example from the command palette. */
	author(command: AuthorCommand) {
		this.#push(this.bench.author(command));
	}

	act(command: Command) {
		this.#play({ type: 'command', value: command });
	}

	/** Choose an answer in the talk in progress. */
	choose(index: number) {
		this.#play({ type: 'choice', value: index });
	}

	/** Leave the talk in progress. */
	leave() {
		this.#play({ type: 'command', value: { type: 'leave' } });
	}

	/** The paused rule or the death prompt that waits for an answer, or null. */
	get awaiting(): Awaiting | null {
		return this.status.type === 'await' ? this.status.awaiting : null;
	}

	/** Answer what the game waits for. An answer that does not fit shows as a note. */
	resume(value: Value) {
		const awaiting = this.awaiting;
		if (!awaiting) return;
		this.draft = '';
		try {
			this.#push(this.bench.resume(awaiting.id, value));
		} catch (e) {
			this.#push({ blocks: [{ kind: 'note', runs: [{ text: String(e) }] }], ended: false, log: [] });
		}
	}

	/**
	 * Show the next level of the hint ladder (UI-1), of `step` or of the first step with a level left. The hint
	 * shows in the transcript as a note. It is no turn, so the playthrough does not list it.
	 */
	hint(step?: string) {
		this.entries.push({ id: this.#next++, turn: this.bench.hint(step) });
		this.#refresh();
	}

	/** Go back to the state before the last turn. The transcript keeps the turns; the undo shows as a new turn. */
	undo() {
		this.draft = '';
		this.#push(this.bench.undo());
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

	/** Start the story again with an empty transcript. `undo` can go back to the state before it. */
	restart() {
		this.entries = [];
		this.history = [];
		this.draft = '';
		this.#push(this.bench.restart());
	}
}
