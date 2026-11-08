use loremaster::prelude::*;

#[behavior(on = nope)]
fn unknown_kind(cx: &mut Cx<'_>) {}

#[behavior(when = turn)]
fn unknown_key(cx: &mut Cx<'_>) {}

#[behavior]
fn two_args(a: u32, cx: &mut Cx<'_>) {}

fn main() {}
