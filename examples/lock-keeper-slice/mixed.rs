// The mixed form: the YAML folder plus two world! packs (the lock machine and
// the keeper's agenda). The packs join in a fixed order, as the DSL doc
// proposes (Pack and assemble, DS-1, DSL step L1). No pack uses `extend` or
// `replace`, so a duplicate id is an error. The ids are the same as in
// ../yaml and ../dsl, so all three forms give one model (rule 1).
//
// Behind the cargo feature `dsl`. The lock machine compiles with the world!
// core (DSL step L3, P3-03 #66); the agenda waits for agendas in world!
// (the YAML agenda loads since D6, P4-02 #79).

#![cfg(feature = "dsl")]

use loremaster::dsl::project::{SourceFile, read_dir};
use loremaster::prelude::*;

#[path = "mixed/machines/lock.rs"]
pub mod lock;

// Pending: `cfg(any())` is never true, so the compiler does not read the file.
// world! has no agendas yet; the YAML form loads since P4-02 (#79).
#[cfg(any())]
#[path = "mixed/agendas/keeper.rs"]
pub mod agenda;

/// The YAML files of this folder. The YAML loader reads only `*.yaml`, so
/// the `.rs` files next to them do not count.
pub fn yaml_files() -> Vec<SourceFile> {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/lock-keeper-slice/mixed/"
    );
    read_dir(std::path::Path::new(dir)).expect("the mixed form reads")
}

/// The given YAML files plus the lock machine, without the agenda. The YAML
/// part still holds all new-syntax files; a test filters them by path, as it
/// does for ../yaml, and passes the rest.
pub fn today(files: Vec<SourceFile>) -> Loremaster {
    Loremaster::new().yaml_files(files).pack(lock::pack())
}

/// The whole slice. `slice_equivalence` compares its model hash with the
/// hashes of ../yaml and ../dsl. Panics until world! has agendas.
pub fn world() -> Loremaster {
    panic!("the agenda (mixed/agendas/keeper.rs) is not in world! yet; YAML has it since P4-02 #79")
}
