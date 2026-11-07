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
- The Session API (ADR 0003): `Session::new`, `with_seed`, `load`, `start`, `submit(Input)` (text, command record, choice, tick), `choose`, `save`. Each call returns a `Step`: effects as data (`Text`, `Notice`, `Choices`, `Ending`) and a `Status` (ready or ended).
- The `await` effect: a rule stops until the host answers a choice, a roll, a walk or an action, and goes on with the answer (`Session::resume`, `Status::Await`). A save game keeps the waiting rule, and the explorer tries each answer.
- Undo, snapshots and the death prompt (SE-8, SE-9): `Session::undo` (also the text `undo`), `set_undo_depth`, `snapshot`, `restore` and `restart`. An ending of kind `death` sends a death prompt await (`undo`, `restart`, `restore`), unless story.yaml says `death_prompt: false`. Walkthroughs run through the Session API.
- `extend: true` and `replace: true` in an entity file of a later pack: `replace` takes the place of the earlier entity (with a warning), `extend` adds lists and map entries to it; a value that both packs set is an error.
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

- loremaster depends on dramaturge 0.3.0-alpha.2 from crates.io, with an exact version, instead of the git branch `feature/0.3`. CI builds with `--locked`, packages the crates on every branch again, and a job that is not required tests against the newest dramaturge on `feature/0.3`.
- The turn log of a talk shows what a dialogue really changed, in the order of the changes, with the new values. Removed facts and forgotten memories get a line; a relation change of 0 and a fact outside the world scope do not.
- The room matrix and the text matrix try the player's relations to things (`blackbeard.trust`) as a dimension.
- One diagnostic form (XP-6): `Diagnostic` and `Severity` live in `loremaster-model` (`loremaster-project` re-exports them) and can be deserialized. A compile error becomes a `Diagnostic` with `Diagnostic::from(&error)`, and `Project::locate` gives it the file of its entity. The CLI, the web engine and the AI tools report compile errors with their file now.
- **Breaking (library):** `Project::origins` maps to the `Origin` enum (`Yaml { path }`, `Rust { file, line, column, pack }`, non-exhaustive) instead of a path string. `Origin::file()` gives the path.
- `Game::save` and `Game::load_save` read and write save games. The web engine's `save()` and `restore()` and the AI tools `save` and `session_start` use the envelope; `restore()` returns the dropped ids.
- **Breaking:** the `end` effect names an ending from `endings/` (`end: beacon_lit`) instead of a text. Move the text into `endings/<id>.yaml` with a `kind` and a `title`. The lighthouse and the tower examples show it.
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
