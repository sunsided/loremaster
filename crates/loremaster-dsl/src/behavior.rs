//! Rust behaviors in a pack: an instance that the pack keeps (DSL step L5),
//! and the inline `rust |cx| { .. }` block of a `world!` rule.

use loremaster_runtime::{Behavior, Cx, Event, EventKind, Flow, Manifest};

/// The result of the Rust code in a `#[behavior]` function or a `rust`
/// block: a [`Flow`], or `()` for [`Flow::Continue`].
pub trait IntoFlow {
    fn into_flow(self) -> Flow;
}

impl IntoFlow for Flow {
    fn into_flow(self) -> Flow {
        self
    }
}

impl IntoFlow for () {
    fn into_flow(self) -> Flow {
        Flow::Continue
    }
}

/// The behavior of a `rust |cx| { .. }` block in a rule. Its id is
/// `<rule>#rust`, and the rule calls it with `call: <rule>#rust` at the
/// place of the block. It handles only that call.
pub struct RustBlock<F> {
    id: String,
    owner: String,
    f: F,
}

impl<F> RustBlock<F>
where
    F: Fn(&mut Cx<'_>) -> Flow + Send + Sync + 'static,
{
    /// The block of the rule `owner`.
    pub fn new(owner: &str, f: F) -> Self {
        RustBlock {
            id: format!("{owner}#rust"),
            owner: owner.to_string(),
            f,
        }
    }
}

impl<F> std::fmt::Debug for RustBlock<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RustBlock").field("id", &self.id).finish()
    }
}

impl<F> Behavior for RustBlock<F>
where
    F: Fn(&mut Cx<'_>) -> Flow + Send + Sync,
{
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        "inline Rust"
    }

    fn manifest(&self) -> Manifest {
        Manifest::opaque().events([EventKind::Call])
    }

    fn handle(&self, event: &Event, cx: &mut Cx<'_>) -> Flow {
        match event {
            Event::Call(owner) if owner.as_str() == self.owner => (self.f)(cx),
            _ => Flow::Continue,
        }
    }
}
