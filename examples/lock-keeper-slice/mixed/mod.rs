// The mixed form: the YAML folder plus two world! packs (the lock machine and
// the keeper's agenda). The packs join in a fixed order, as the DSL doc
// proposes (Pack and assemble, DS-1, DSL step L1). No pack uses `extend` or
// `replace`, so a duplicate id is an error. The ids are the same as in
// ../yaml and ../dsl, so all three forms give one model (rule 1).
//
// Behind the cargo feature `dsl`. It does not compile until DSL step L2.

#![cfg(feature = "dsl")]

use loremaster::prelude::*;

#[path = "agendas/keeper.rs"]
pub mod agenda;
#[path = "machines/lock.rs"]
pub mod lock;

/// The YAML files of this folder. The YAML loader reads only `*.yaml`, so
/// the `.rs` files next to them do not count.
const YAML: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/lock-keeper-slice/mixed/"
);

/// The YAML part plus the lock machine, without the agenda. The YAML part
/// still holds all new-syntax files; a test filters them by path, as it does
/// for ../yaml.
pub fn today() -> Loremaster {
    Loremaster::new().yaml(YAML).pack(lock::pack())
}

/// The whole slice. `slice_equivalence` compares its model hash with the
/// hashes of ../yaml and ../dsl.
pub fn world() -> Loremaster {
    today().pack(agenda::pack())
}
