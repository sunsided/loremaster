use loremaster::prelude::*;

#[behavior(on = nope)]
fn unknown_kind(cx: &mut Cx<'_>) {}

#[behavior(when = turn)]
fn unknown_key(cx: &mut Cx<'_>) {}

#[behavior]
fn two_args(a: u32, cx: &mut Cx<'_>) {}

#[behavior(on = [])]
fn no_kinds(cx: &mut Cx<'_>) {}

#[behavior]
async fn not_plain(cx: &mut Cx<'_>) {}

#[behavior]
fn __(cx: &mut Cx<'_>) {}

fn main() {}
