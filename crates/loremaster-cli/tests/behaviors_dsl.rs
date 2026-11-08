//! Rust behaviors in world! (DSL step L5, P5-05 #92): the example
//! examples/parrot-cove plays with all three forms.
#![cfg(feature = "dsl")]

#[path = "../../../examples/parrot-cove/world.rs"]
mod cove;

use cove::Parrot;
use loremaster::dsl::RustBlock;
use loremaster::prelude::*;
use loremaster_runtime::{Input, Session, run_walkthrough};
use loremaster_workbench::Workbench;
use std::sync::Arc;

fn world() -> Arc<loremaster_runtime::World> {
    Arc::new(Loremaster::new().pack(cove::pack()).build().unwrap())
}

#[test]
fn one_line_names_and_registers_a_behavior() {
    let w = world();
    assert_eq!(
        w.story
            .behaviors
            .iter()
            .map(|b| b.as_str())
            .collect::<Vec<_>>(),
        ["parrot", "bell", "weigh_gold#rust", "sluice#1#rust"]
    );
    assert!(w.missing_behaviors().is_empty());
}

#[test]
fn the_attribute_makes_a_behavior() {
    assert_eq!(Parrot.id(), "parrot");
    assert_eq!(
        Parrot.description(),
        "The parrot squawks on some turns. The roll is in the save state."
    );
}

#[test]
fn the_walkthrough_passes() {
    let project = Loremaster::new().pack(cove::pack()).assemble().unwrap();
    assert!(!project.walkthroughs.is_empty());
    for (id, walk) in &project.walkthroughs {
        let result = run_walkthrough(world(), walk, id);
        assert!(
            result.passed(),
            "{:#?}\n{}",
            result.failures,
            result.transcript
        );
    }
}

#[test]
fn a_rust_block_runs_at_its_place_in_the_effects() {
    let mut s = Session::new(world());
    s.start();
    s.submit(Input::Text("take gold".into()));
    let text = s.submit(Input::Text("weigh".into())).plain_text();
    let (tips, reads, settles) = (
        text.find("The scale tips.").unwrap(),
        text.find("It reads 3.").unwrap(),
        text.find("It settles.").unwrap(),
    );
    assert!(tips < reads && reads < settles, "{text}");
}

/// Inline Rust keeps the determinism rules: the same commands give the same
/// text, also with rolls.
#[test]
fn replay_is_the_same() {
    let play = || {
        let mut s = Session::new(world());
        let mut text = s.start().plain_text();
        for c in ["take gold", "weigh", "pull", "wait", "wait", "wait", "wait"] {
            text.push_str(&s.submit(Input::Text(c.into())).plain_text());
        }
        text
    };
    let first = play();
    assert_eq!(first, play());
    assert!(first.contains("Pieces of eight"), "{first}");
    assert!(first.contains("A bell rings"), "{first}");
}

#[test]
fn the_rules_view_marks_rust() {
    let mut bench = Workbench::new(world());
    bench.start();
    let rules = bench.rules();
    let rule = rules.iter().find(|r| r.id == "weigh_gold").unwrap();
    assert!(
        rule.then.iter().any(|t| t == "run Rust (weigh_gold#rust)"),
        "{:?}",
        rule.then
    );
}

/// A `rust` block has no origin, so a later pack can give the same id (a
/// replaced machine), and its code runs.
#[test]
fn the_later_pack_gives_the_instance() {
    let mut first = Pack::new("first");
    first.story("Two", "hall");
    first.room("hall", "Hall").text("A hall.");
    first
        .rule("a", on::start())
        .effect(Effect::Call(loremaster::dsl::model::Id::new("a#rust")));
    first.register_block(RustBlock::new("a", |cx: &mut Cx<'_>| {
        cx.say("one");
        Flow::Continue
    }));
    let mut second = Pack::new("second");
    second.register_block(RustBlock::new("a", |cx: &mut Cx<'_>| {
        cx.say("two");
        Flow::Continue
    }));
    let world = Loremaster::new().pack(first).pack(second).build().unwrap();
    let text = Session::new(Arc::new(world)).start().plain_text();
    assert!(text.contains("two") && !text.contains("one"), "{text}");
}

#[behavior(on = turn)]
fn ping(cx: &mut Cx<'_>) {
    cx.say("ping");
}

/// A `call` runs a behavior whatever its event filter says.
#[test]
fn a_call_ignores_the_filter() {
    let mut p = Pack::new("p");
    p.story("Ping", "hall");
    p.room("hall", "Hall").text("A hall.");
    p.rule("a", on::start())
        .effect(Effect::Call(loremaster::dsl::model::Id::new("ping")));
    p.register(Ping);
    let world = Loremaster::new().pack(p).build().unwrap();
    let text = Session::new(Arc::new(world)).start().plain_text();
    assert!(text.contains("ping"), "{text}");
}
