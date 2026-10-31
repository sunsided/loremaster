# loremaster-model

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate has the world types: rooms, exits, things, fragments, rules, the clock and the verbs. A project folder deserializes into these types. The story bundle contains them.

Conditions and inline markup stay as source text in the model. `loremaster-lang` parses them.

## License

EUPL-1.2. Read [LICENSE](LICENSE).
