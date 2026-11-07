//! The compile errors of `world!`, with trybuild: each file in ui/fail must
//! fail with the message in its `.stderr` file, at the right tokens. The
//! files in ui/pass must compile and run.
//!
//! After a change of a message, run `TRYBUILD=overwrite cargo test -p
//! loremaster-macros --test ui` and read the diff of the `.stderr` files.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/fail/*.rs");
}
