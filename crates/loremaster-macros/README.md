# loremaster-macros

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate is the `world!` macro: a story pack in Rust, in a grammar close to the YAML files of a project folder. Use it through the crate `loremaster` with its cargo feature `dsl`.

```rust
use loremaster::prelude::*;

world! {
    extern {
        ending drowned; // an id of another pack
    }

    story "The Lock" { start: chamber, seed: 11 }
    var lock_open: bool = false;

    room chamber "Lock Chamber" {
        at [1, 0];
        "Stone walls drop into the lock.";
        ambient cycle when night => ["Water knocks.", "A rope creaks."];
    }

    rule jump on verb(jump) instead when in(chamber) and not lock_open {
        line "The water closes over you.";
        end drowned;
    }

    verb jump "Jump" { words [jump, "jump in"]; slots none; no_bar; }
}

let world = Loremaster::new().pack(pack()).yaml("endings/").build()?;
```

- A block makes the function `pack()`. It calls the builder API of `loremaster-dsl`, so the pack has the same model values as the builder and the YAML loader make.
- Each entity gets an `Origin::Rust` with the file, line and column of its id in the block.
- At compile time, the macro runs the schema checks (`check_packs`) on the pack: unknown ids, reserved names, conditions and markup. An error is a compile error at the tokens it names. Ids of other packs go into `extern { .. }`.
- The grammar has the story header, vars, rooms, things, rules, machines, timers, endings, verbs and walkthrough tests. A `line` takes speech segments: `line "He shrugs." keeper says "Prove it," narrate "he says.";` and `line when in(chamber) then "Water drips.";`. Dialogues, text pools, `one_of` and agendas come later; the macro names the step or issue that brings each one.
- The tests in `tests/ui/` (trybuild) hold the compile errors, and `tests/grammar.rs` the model values and origins.

## License

EUPL-1.2. Read [LICENSE](LICENSE).
