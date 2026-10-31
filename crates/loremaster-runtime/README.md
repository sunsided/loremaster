# loremaster-runtime

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate is the runtime. It is pure and deterministic. It takes a story and a save state, and a command (typed, or from the verb bar). It gives the output and the new state. It has no input or output of its own, no clock and no global random numbers. Thus the editor, the web player and the terminal player show the same game.

The public API uses types from [dramaturge](https://crates.io/crates/dramaturge-model) 0.2. A new major version of dramaturge thus can also be a breaking change of this crate.

## License

EUPL-1.2. Read [LICENSE](LICENSE).
