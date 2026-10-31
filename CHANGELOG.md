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
- Behaviors in Rust: the `Behavior` trait, `World::register`, and `behaviors:` in story.yaml or `behaviors/`.
- `verbs/` for custom verbs in their own files.
- Zork I: containers, scenery, the score and value of treasures from the importer; the lamp battery, the trophy case, the exorcism, the dam and the thief ported by hand. Read `docs/zork-fit-test.md`.

### Changed

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
