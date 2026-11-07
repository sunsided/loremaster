// The whole slice in world!, one pack per part. This file joins the packs in
// a fixed order, as the DSL doc proposes (Pack and assemble, DS-1, DSL step
// L1; the builder API, DS-3, step L2). A test can load today's pack plus only
// the parts that it needs, as crates/loremaster-runtime/tests/slice.rs does
// with the YAML folders.
//
// Behind the cargo feature `dsl`. It does not compile until DSL step L2
// (phase 1 and 2), and the world! grammar of the parts until steps L3 and L4.

#![cfg(feature = "dsl")]

use loremaster::prelude::*;

pub mod agenda;
pub mod dialogue;
pub mod endings;
pub mod keeper_rule;
pub mod stove;
pub mod tests;
pub mod texts;
pub mod today;

/// The parts of the slice with new syntax. The names match the parts table
/// in ../PENDING.md.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Endings,
    Texts,
    Dialogue,
    Stove,
    KeeperRule,
    Agenda,
}

impl Part {
    pub const ALL: &'static [Part] = &[
        Part::Endings,
        Part::Texts,
        Part::Dialogue,
        Part::Stove,
        Part::KeeperRule,
        Part::Agenda,
    ];

    /// Each world! block makes `pack()` (DSL doc, "Joining the files").
    fn pack(self) -> Pack {
        match self {
            Part::Endings => endings::pack(),
            Part::Texts => texts::pack(),
            Part::Dialogue => dialogue::pack(),
            Part::Stove => stove::pack(),
            Part::KeeperRule => keeper_rule::pack(),
            Part::Agenda => agenda::pack(),
        }
    }
}

/// Today's pack plus the given parts, in the order of `Part::ALL`. The order
/// is explicit, so the result does not depend on the order of `parts`. No
/// pack uses `extend` or `replace`, so a duplicate id is an error.
pub fn with(parts: &[Part]) -> Loremaster {
    let mut lm = Loremaster::new().pack(today::pack());
    for part in Part::ALL.iter().filter(|p| parts.contains(p)) {
        lm = lm.pack(part.pack());
    }
    lm
}

/// The whole slice. `slice_equivalence` compares its model hash with the
/// hashes of ../yaml and ../mixed.
pub fn world() -> Loremaster {
    with(Part::ALL).pack(tests::pack())
}
