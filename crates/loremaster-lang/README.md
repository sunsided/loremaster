# loremaster-lang

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate has the small languages in a story:

- **Conditions**, for example `time.phase == night and not lamp.lit`. The crate parses them, examines them against the story and compiles them to dramaturge conditions.
- **Inline markup** in fragment text, for example `The {night: moonlit | sunlit} courtyard lies quiet.`
- **Dialogues**. The crate compiles them to a dramaturge dialogue graph.

The public API uses types from [dramaturge](https://crates.io/crates/dramaturge-model) 0.2. A new major version of dramaturge thus can also be a breaking change of this crate.

## License

EUPL-1.2. Read [LICENSE](LICENSE).
