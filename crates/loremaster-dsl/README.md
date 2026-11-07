# loremaster-dsl

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate is the builder API: you write a story in plain Rust. The builders make the same model values as a YAML project folder, so the checks, the runtime and the editor work without changes. rustfmt and rust-analyzer work on all of it.

```rust
use loremaster_dsl::prelude::*;

fn lighthouse() -> Pack {
    let mut p = Pack::new("lighthouse");
    p.story("The Lighthouse", "shore").var("beacon_lit", false);
    p.room("cottage", "Keeper's Cottage")
        .at(0.0, -1.0)
        .text("One room, smelling of tar and pipe smoke.")
        .text(frag("Through the salt-crusted window the sea has gone black.").ambient().when("night"))
        .exit(Direction::South, "shore")
        .door("cottage_door");
    p.thing("oil_lamp", "oil lamp").in_room("cottage").portable().prop("lit", false);
    p.rule("light_the_beacon", on::verb(["turn_on", "lens"]))
        .when("oil_lamp.carried and oil_lamp.lit")
        .instead()
        .set("beacon_lit", true)
        .end("beacon_lit");
    p
}

let world = Loremaster::new()
    .pack(lighthouse())
    .yaml("story/") // optional: more packs from YAML folders, in order
    .build()?;
```

- Each source makes a `Pack`. `Loremaster` joins the packs in their order. An id that two packs define is an error, unless the later entity calls `replace()`.
- Each entity gets an `Origin::Rust` with the file, line and column of the call that made it. A diagnostic about the entity names that place.
- The test `tests/lighthouse.rs` writes `examples/lighthouse` with the builder and compares the story with the YAML story, value by value and by `model_hash`.

The crate `loremaster` gives this API as `loremaster::dsl` and `loremaster::prelude` with the cargo feature `builder`.

## License

EUPL-1.2. Read [LICENSE](LICENSE).
