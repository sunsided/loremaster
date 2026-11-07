//! The `world!` forms of the golden slice (roadmap phase 0, P0-02 and P0-03).
//! They are draft syntax: the macro does not exist yet, so they compile only
//! with the cargo feature `dsl`, and that build fails until DSL step L2.
//! `slice_equivalence` in loremaster-runtime compares the forms.
#![cfg(feature = "dsl")]

#[path = "../../../examples/lock-keeper-slice/dsl.rs"]
mod dsl;

#[path = "../../../examples/lock-keeper-slice/mixed.rs"]
mod mixed;
