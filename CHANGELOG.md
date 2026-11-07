# Changelog

All important changes to loremaster are in this file. All crates of the workspace have the same version.

The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

### Added

- Action phases: `check` rules run before a verb, and the new `stop` effect refuses the action.
- Rules in thing and room files, `within:` for a thing, room or region, and `priority`. More specific rules run first.
- Timers (`timers/`, or `timers:` in story.yaml), `timer.<id>` in conditions, and the effects `start` and `cancel`.
- `chance(n)` in conditions, with the same result for the same state.
- Custom events (`emit`, `on: { event }`) and dialogue events (`on: { talk: [dialogue, node] }`).
- Containers and supporters (`location: { in: box }`, `{ on: table }`), the `put` verb, `capacity`, and `in(thing, holder)` in conditions.
- Scenery in more rooms or regions with `also_in`.
- The score (`score:` in story.yaml) and the `score` command.
- Puzzle machines (`machines/`) with named states and steps, and Atlas › Machines in the editor.
- Walkthrough tests (`tests/*.yaml`) and `loremaster test`.
- `model_hash(story)`: the canonical hash of a model (SHA-256 of the story as JSON). The golden slice keeps the hash of its YAML form in `expected/model.hash`.
- Behavior manifests (XP-4): `Behavior::manifest` names the facts a behavior reads and writes and the event kinds (`EventKind`) it handles. The game sends it only those events, logs a note for a write the manifest does not name, and the rules view and the explorer use it. Without one, a behavior is opaque, as before.
- The `call` effect: `call: thief` runs a behavior at its place in a rule, with `Event::Call(owner)` (the rule's id, or `dialogue:<id>` / `timer:<id>`); `Flow::Stop` stops the rule. A `call` that names no behavior of the story is a check error; at run time, a behavior that the story does not name or that the build does not have is a note in the log.
- The Session API (ADR 0003): `Session::new`, `with_seed`, `load`, `from_save`, `start`, `submit(Input)` (text, command record, choice, tick), `choose`, `save`. Each call returns a `Step`: effects as data (`Text`, `Notice`, `Choices`, `Ending`) and a `Status` (ready or ended). A tick lets time pass without an action, also during a talk (`Game::tick`). Notices are only changes of state (`set`, `move`, `timer`, `talk`, `memory`, `relation`).
- **Breaking (library):** `loremaster_model::Effect` has the new variant `Await`; code that matches `Effect` exhaustively needs an arm for it.
- The `await` effect: a rule stops until the host answers a choice, a roll, a walk or an action, and goes on with the answer (`Session::resume`, `Status::Await`). A save game keeps the waiting rule, and the explorer tries each answer.
- Undo, snapshots and the death prompt (SE-8, SE-9): `Session::undo` (also the text `undo`), `set_undo_depth`, `snapshot`, `restore` and `restart`. An ending of kind `death` sends a death prompt await (`undo`, `restart`, `restore`), unless story.yaml says `death_prompt: false`. Walkthroughs run through the Session API.
- `extend: true` and `replace: true` in an entity file of a later pack: `replace` takes the place of the earlier entity (with a warning), `extend` adds lists and map entries to it; a value that both packs set is an error.
- The schema checks as one library function (XP-5): `loremaster_project::check_packs(packs)` joins packs and checks ids, references, facts, conditions, markup and reserved names, with no files and no I/O, so that the `world!` macro can run it at compile time. `check_files(files)` runs it for a YAML folder; the CLI, the web engine and the AI tools use it. Both return `Checked { project, diagnostics }`. Diagnostics about the story header name the file of its origin instead of always `story.yaml`.
- The builder API (DSL step L2, DS-3): the new crate `loremaster-dsl` writes a story in plain Rust as a `Pack` (story header, rooms, exits, things, fragments, rules with events and effects, machines, timers, endings, verbs, dialogues, threads, walkthroughs). Each entity gets an `Origin::Rust` with the file, line and column of the call. `Loremaster::new().pack(..).yaml(..).build()` joins packs from Rust and YAML in order, checks them and compiles the world. A test proves that the Lighthouse from the builder equals the Lighthouse from YAML, value by value and by `model_hash`. The crate `loremaster` gives the API as `loremaster::dsl` and `loremaster::prelude` with the new cargo feature `builder`.
- The Workbench API (SE-13, SE-14, ADR 0003): the new crate `loremaster-workbench`. A `Workbench` owns a `Session` (`session()`, `session_mut()`) and gives the authoring views: `atlas`, `map`, `rules`, `machines`, `symbols`, `try_markup`, `try_markup_in`, `try_condition`, `describe`, `preview_situation`, `assembly`, `room_parts`, `last_frame`, `talk_view`, `inspect_thing`, `inspect_exit`, `vars`, `phases`, `room_matrix`, `text_matrix` and `explore`. `author` runs an author command, and `start` and `play` give whole turns with the rule log for an editor transcript. `reload(world)` carries the running game over to a changed world: the room, the values and the things stay, and what the new world does not have is dropped. The undo history does not carry over. The web engine, the AI tools and the terminal use it.
- Read-only queries on `Session` (ADR 0003, point 8): `scene`, `verbs`, `preview`, `talk` and `choices`.
- `Pack` and `assemble` in `loremaster-project` (DSL step L1): each source makes a pack, and `assemble` joins packs in their order. An id that two packs define, or a second story header, is an error that names both origins. `load` is one YAML pack, assembled.
- Save games are an envelope with a format version, the story id, a hash of the story and the state. A restore migrates older saves (also the bare state of 0.1), refuses a newer version or another story with a message, and keeps the valid state of a changed story with a list of what it dropped. Read `docs/adr/0001-save-compatibility.md`.
- Endings as entities (`endings/`): a kind (`win`, `loss`, `death`, `neutral`), a rank, a title and a text. The save game keeps the ending, `expect: { ending: … }` checks it in a walkthrough, and `loremaster check` reports unknown and unused endings.
- Pending tests: `pending: { phase, needs }` in a walkthrough, `#[ignore = "pending: ..."]` in Rust, `task test:pending`, and a CI job that fails when a pending test passes.
- The golden slice `examples/lock-keeper-slice`: a part of the Lock-Keeper demo in YAML, with `slice_baseline` and eleven pending tests for the roadmap phases 1 to 4. Read its `PENDING.md`.
- The draft `world!` forms of the golden slice (`dsl/`, `mixed/`), behind the new cargo feature `dsl` of the crate `loremaster`. They do not compile until DSL step L2.
- Behaviors in Rust: the `Behavior` trait, `World::register`, and `behaviors:` in story.yaml or `behaviors/`.
- `verbs/` for custom verbs in their own files.
- Zork I: containers, scenery, the score and value of treasures from the importer; the lamp battery, the trophy case, the exorcism, the dam and the thief ported by hand. Read `docs/zork-fit-test.md`.

### Changed

- **Breaking:** a variable, machine, room or thing whose id is a built-in name of conditions (`here`, `turns`, `level`, `first_visit`, `time`, `action`, `timer`, `true`, `false`, `and`, `or`, `not`, the functions `in`, `carried`, `present`, `visited`, `visits`, `chance`, `remembers`, and the planned `seen_by`) is a check error at its declaration, and so is a variable or machine with the name of a clock phase. Before, it loaded, and its conditions silently read the built-in, or the variable instead of the phase. `loremaster_lang::RESERVED_NAMES` is the one list, and `loremaster_lang::reserved` tells a keyword, a derived value and a function apart for the message; the ZIL importer uses it too and adds `_zil` to such names and to the phase names of the default clock.
- **Breaking:** the loader rejects an unknown field in every entity file (rooms, things, rules, dialogues with their nodes and options, timers, verbs, machines, story.yaml and walkthrough steps and `expect`), an unknown folder and a YAML file other than story.yaml at the top, with the nearest known name: ``unknown field `descripton`: did you mean `description`?``. Before, only endings, threads and story.yaml rejected unknown fields, and an unknown folder loaded as nothing. A fragment with an unknown field names the field instead of "did not match any variant". The folders `agendas/` and `texts/` of the golden slice are skipped with a warning. `loremaster_project::FOLDERS` lists the folders, and `Pack::warnings` holds the warnings of a pack.
- **Breaking (library):** `Cx` is a narrow, stable view for behaviors (XP-1, XP-7). `Cx::state()` and `Cx::world()` are gone; use the new `turns`, `minutes`, `visited` and `room_has_tag`, or the read-only escape hatch `Cx::unstable()`. Random numbers come only from `Cx::roll`.
- loremaster depends on dramaturge 0.3.0-alpha.2 from crates.io, with an exact version, instead of the git branch `feature/0.3`. CI builds with `--locked`, packages the crates on every branch again, and a job that is not required tests against the newest dramaturge on `feature/0.3`.
- The turn log of a talk shows what a dialogue really changed, in the order of the changes, with the new values. Removed facts and forgotten memories get a line; a relation change of 0 and a fact outside the world scope do not.
- The room matrix and the text matrix try the player's relations to things (`blackbeard.trust`) as a dimension.
- One diagnostic form (XP-6): `Diagnostic` and `Severity` live in `loremaster-model` (`loremaster-project` re-exports them) and can be deserialized. A compile error becomes a `Diagnostic` with `Diagnostic::from(&error)`, and `Project::locate` gives it the file of its entity. The CLI, the web engine and the AI tools report compile errors with their file now.
- **Breaking (library):** `Project::origins` maps to the `Origin` enum (`Yaml { path }`, `Rust { file, line, column, pack }`, non-exhaustive) instead of a path string. `Origin::file()` gives the path.
- `Game::save` and `Game::load_save` read and write save games. The web engine's `save()` and `restore()` and the AI tools `save` and `session_start` use the envelope; `restore()` returns the dropped ids.
- **Breaking:** the `end` effect names an ending from `endings/` (`end: beacon_lit`) instead of a text. Move the text into `endings/<id>.yaml` with a `kind` and a `title`. The lighthouse and the tower examples show it.
- **Breaking (library):** the authoring views moved out of `Game` into `loremaster-workbench`. `Game::atlas`, `map`, `rules`, `machines`, `symbols`, `try_markup`, `try_condition`, `try_markup_situation` (now `Workbench::try_markup_in`), `preview_situation`, `describe_preview` (now `Workbench::describe`), `assembly`, `room_parts`, `last_frame`, `talk_view`, `inspect_thing`, `inspect_exit`, `vars`, `phases`, `author` and `carry_over` (now `Workbench::reload`) are gone from the public API, and so are the free functions `explore`, `room_matrix` and `text_matrix` and their types at the root of `loremaster-runtime`: use `loremaster_workbench`. `Game` is `#[doc(hidden)]`; hosts use `Session`. `Session::game()` is gone.
- The terminal player plays through `Session`. A death ending shows the death prompt: type `undo` or `restart`.
- **Breaking (JS API):** the wasm `Engine` class is gone. The web engine exports two classes that mirror the Rust split (ADR 0003): `Session` for players (`start`, `submit(input)`, `choose`, `resume`, `undo`, `undoSteps`, `setUndoDepth`, `setHostRolls`, `restart`, `snapshot`, `restore`, `save`, `load`, `status`, `scene`, `verbs`, `preview`, `talk`, `choices`; each play call returns a `Step` of effects and a status) and `Workbench` for the editor (play as whole turns with `start`, `play(input)`, `resume`, `undo`, `restart` and `author`, the session's queries, `carryOver`, and the authoring views). `input`, `act`, `leave`, `reset` and `tokens` are gone: use `play` with an `Input` (`{ type: "text" | "command" | "choice" | "tick", value }`), `restart`, and the free function `markupTokens`. `load(save)` (was `restore(save)`) returns `{ story_changed, from_version, dropped }`. The TS wrapper in `web/src/lib/engine/index.ts` calls only these two classes, and the player uses only the workbench.
- The web player shows what a paused rule or the death prompt waits for (a choice, a roll, an action, a walk) with its answers, and answers with `resume`. Undo is in the turn menu and the command palette.
- **Breaking (library):** `Game` and `Restored` are no longer public in `loremaster-runtime`; use `Session` (`Session::load` returns `(Session, Loaded)`). The runtime tests play through `Session` and the hidden `authoring` primitives.
- `Session::status()` tells what the session waits for now. `Workbench` has `resume`, `undo` and `restart` as whole turns with the rule log, and `dialogues`.
- `carried(x)` is also true for a thing in a carried container.
- The ZIL importer writes containers, surfaces and local-global scenery, and reads `machines/` and `timers/` for the flags that they use.

## [0.1.0] - 2026-10-31

### Added

- The first release on crates.io: `loremaster` (the command), `loremaster-model`, `loremaster-lang`, `loremaster-project`, `loremaster-runtime`, `loremaster-tools` and `loremaster-mcp`.
- `cargo install loremaster` gives the binary with the web editor in it.

### Notes

- The public API of `loremaster-lang` and `loremaster-runtime` uses types from dramaturge 0.2. A new major version of dramaturge thus can also be a breaking change of loremaster.

<!-- next-url -->
[Unreleased]: https://github.com/sunsided/loremaster/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sunsided/loremaster/releases/tag/v0.1.0
