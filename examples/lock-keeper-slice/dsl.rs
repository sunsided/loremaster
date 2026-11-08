// The whole slice in world!, one pack per part (the files in dsl/). This file
// joins the packs in a fixed order, as the DSL doc proposes (Pack and
// assemble, DS-1, DSL step L1). A test can load today's pack plus only the
// parts that it needs, as crates/loremaster-runtime/tests/slice.rs does with
// the YAML folders.
//
// Behind the cargo feature `dsl`. Today, the endings and the tests compile
// with the world! core (DSL step L3, P3-03 #66). The other parts use syntax
// that world! does not have yet; they stay out of the build until it does.

#![cfg(feature = "dsl")]

// The test target loads this file with `#[path]`, so its child modules name
// their files explicitly (no mod.rs: this file sits next to dsl/).

use loremaster::prelude::*;

#[path = "dsl/endings.rs"]
pub mod endings;
#[path = "dsl/tests.rs"]
pub mod tests;
#[path = "dsl/today.rs"]
pub mod today;

// Pending parts. `cfg(any())` is never true, so the compiler does not read
// these files. Each one names what it waits for in its first lines, and the
// world! macro names it too: text pools and `one_of` (R9, P4-01 #78; the
// keeper dialogue says a pool, the rest of it compiles since DSL step L4,
// P4-08 #85), named slots of the command record and agendas (D6, P4-02 #79).
#[cfg(any())]
#[path = "dsl/agenda.rs"]
pub mod agenda;
#[cfg(any())]
#[path = "dsl/dialogue.rs"]
pub mod dialogue;
#[cfg(any())]
#[path = "dsl/keeper_rule.rs"]
pub mod keeper_rule;
#[cfg(any())]
#[path = "dsl/stove.rs"]
pub mod stove;
#[cfg(any())]
#[path = "dsl/texts.rs"]
pub mod texts;

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

    /// Each world! block makes `pack()` (DSL doc, "Joining the files"). None
    /// for a part whose world! file does not compile yet.
    pub fn pack(self) -> Option<Pack> {
        match self {
            Part::Endings => Some(endings::pack()),
            Part::Texts | Part::Dialogue | Part::Stove | Part::KeeperRule | Part::Agenda => None,
        }
    }
}

/// Today's pack plus the given parts, in the order of `Part::ALL`. The order
/// is explicit, so the result does not depend on the order of `parts`. No
/// pack uses `extend` or `replace`, so a duplicate id is an error.
///
/// Panics on a part that does not compile yet ([`Part::pack`]).
pub fn with(parts: &[Part]) -> Loremaster {
    let mut lm = Loremaster::new().pack(today::pack());
    for part in Part::ALL.iter().filter(|p| parts.contains(p)) {
        let pack = part
            .pack()
            .unwrap_or_else(|| panic!("the part {part:?} is not in world! yet"));
        lm = lm.pack(pack);
    }
    lm
}

/// The whole slice. `slice_equivalence` compares its model hash with the
/// hashes of ../yaml and ../mixed. Panics until all parts compile.
pub fn world() -> Loremaster {
    with(Part::ALL).pack(tests::pack())
}
