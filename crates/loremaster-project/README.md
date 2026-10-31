# loremaster-project

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate reads and writes the project folder. A project is a folder of YAML files, one file for each entity. The crate also has the checker and the compiler to a story bundle.

```text
story.yaml        title, start room, intro, clock, variables, custom verbs
rooms/*.yaml      one room for each file
things/*.yaml     one thing for each file
rules/*.yaml      one rule, or a list of rules, for each file
threads/*.yaml    one long-range thread for each file
```

## License

EUPL-1.2. Read [LICENSE](LICENSE).
