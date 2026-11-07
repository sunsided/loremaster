# loremaster-workbench

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate is the Workbench API: the authoring views over a running game. A `Workbench` owns a `Session` of [loremaster-runtime](https://crates.io/crates/loremaster-runtime). Play goes through the session; the views read the full world and state:

- the Atlas and the map,
- the rules and the puzzle machines,
- the names that conditions and markup can use, and checks of text that the author is typing,
- a preview of a room in other values, the assembly of a room, and tooltips for things and exits,
- the room matrix and the state explorer, with the reach of story threads.

The views never change the world. A changed world comes from files or packs, and `reload` carries the running game over to it: the room, the values and the things stay; what the new world does not have is dropped.

```rust
use loremaster_runtime::World;
use loremaster_workbench::Workbench;
use std::sync::Arc;

let project = loremaster_project::load(loremaster_project::scaffold("The Shore")).unwrap();
let mut bench = Workbench::new(Arc::new(World::compile(project.story).unwrap()));
bench.session_mut().start();
for room in bench.atlas() {
    println!("{}: {} things", room.name, room.things.len());
}
```

## License

EUPL-1.2. Read [LICENSE](LICENSE).
