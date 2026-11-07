//! Dialogues with a thing.

use crate::convert::{IntoValue, id, strings};
use crate::pack::{register, with_notes};
use loremaster_model::{
    Dialogue, DialogueNode, DialogueOption, DialogueTarget, Effect, Id, Line, Outcomes,
};
use loremaster_project::Pack;

/// A dialogue of a pack, from [`BuildPack::dialogue`](crate::BuildPack::dialogue).
///
/// [`node`](Self::node) adds a node, and [`option`](Self::option) adds an
/// answer to the node added last. [`when`](Self::when) and the outcomes
/// ([`set`](Self::set), [`add`](Self::add), [`relation`](Self::relation),
/// [`remember`](Self::remember), [`then`](Self::then)) change the node or
/// option added last. Thus give the `when` and the outcomes of a node before
/// its first option, in the order of the YAML file: the node's fields, then
/// its `options`.
///
/// ```
/// use loremaster_dsl::prelude::*;
///
/// let mut p = Pack::new("cantina");
/// p.dialogue("blackbeard_talk", "blackbeard")
///     .start(["meet"])
///     .node("meet", "The figure looks up. \"Sit, if you like.\"")
///     .option("\"Who are you?\"", "names")
///     .option("\"Not tonight.\"", "end")
///     .node("names", "\"They call me Blackbeard,\" he says.")
///     .set("knows_blackbeard_name", true)
///     .relation("trust", 1);
/// assert_eq!(p.dialogues[0].nodes.len(), 2);
/// ```
pub struct DialogueBuilder<'a> {
    pub(crate) pack: &'a mut Pack,
    pub(crate) i: usize,
}

impl<'a> DialogueBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, id: Id, with: Id) -> Self {
        register(pack, format!("dialogue:{id}"));
        pack.dialogues
            .push(Dialogue::new(with.as_str()).with_id(id.as_str()));
        let i = pack.dialogues.len() - 1;
        DialogueBuilder { pack, i }
    }

    fn dialogue(&mut self) -> &mut Dialogue {
        &mut self.pack.dialogues[self.i]
    }

    #[track_caller]
    fn last_node(&mut self) -> &mut DialogueNode {
        match self.dialogue().nodes.last_mut() {
            Some(node) => node,
            None => panic!("the dialogue has no node yet: call `node` first"),
        }
    }

    /// The outcomes of the option added last, or of the node when it has no
    /// option yet.
    #[track_caller]
    fn outcomes(&mut self) -> &mut Outcomes {
        let node = self.last_node();
        match node.options.last_mut() {
            Some(option) => &mut option.outcomes,
            None => &mut node.outcomes,
        }
    }

    /// `start:`, the node keys that can start the talk, in order.
    pub fn start(mut self, keys: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.dialogue().start.extend(strings(keys));
        self
    }

    /// A node: the other character's line: a text, or a [`Line`] of
    /// segments, whose speech the dialogue's thing speaks.
    pub fn node(mut self, key: impl Into<String>, say: impl Into<Line>) -> Self {
        let node = DialogueNode::new(key, say.into());
        self.dialogue().nodes.push(node);
        self
    }

    /// An answer of the last node: the player's line and where it goes (a
    /// node key, `end` or `back`). The speech of the line is the player's.
    #[track_caller]
    pub fn option(mut self, say: impl Into<Line>, go: impl Into<String>) -> Self {
        let option = DialogueOption::new(say.into(), DialogueTarget::parse(&go.into()));
        self.last_node().options.push(option);
        self
    }

    /// `when:` of the option added last, or of the node when it has no option yet.
    #[track_caller]
    pub fn when(mut self, condition: impl Into<String>) -> Self {
        let condition = Some(condition.into());
        let node = self.last_node();
        match node.options.last_mut() {
            Some(option) => option.when = condition,
            None => node.when = condition,
        }
        self
    }

    /// `set:` a variable or property.
    #[track_caller]
    pub fn set(mut self, name: impl Into<String>, value: impl IntoValue) -> Self {
        let value = value.into_value().into();
        self.outcomes().set.insert(name.into(), value);
        self
    }

    /// `add:` to an integer variable.
    #[track_caller]
    pub fn add(mut self, name: impl Into<String>, amount: i64) -> Self {
        self.outcomes().add.insert(name.into(), amount);
        self
    }

    /// `relation:`: add to the player's relation to the dialogue's thing.
    #[track_caller]
    pub fn relation(mut self, axis: impl Into<String>, amount: i32) -> Self {
        self.outcomes().relation.insert(axis.into(), amount);
        self
    }

    /// `remember:`: a memory that the dialogue's thing keeps.
    #[track_caller]
    pub fn remember(mut self, memory: impl Into<String>) -> Self {
        self.outcomes().remember.push(memory.into());
        self
    }

    /// One world effect of `then:` (`move`, `goto`, `end`).
    #[track_caller]
    pub fn then(mut self, effect: Effect) -> Self {
        self.outcomes().then.push(effect);
        self
    }

    /// `end:` the game with an ending, as a world effect.
    #[track_caller]
    pub fn end(self, ending: impl AsRef<str>) -> Self {
        self.then(Effect::End(id(ending)))
    }
}

impl DialogueBuilder<'_> {
    /// This dialogue replaces the one with the same id from an earlier pack
    /// (`replace: true` in YAML). [`assemble`](loremaster_project::assemble)
    /// warns about it.
    pub fn replace(self) -> Self {
        let id = self.pack.dialogues[self.i].id.clone().unwrap_or_default();
        crate::pack::replace(self.pack, format!("dialogue:{id}"));
        self
    }
}
with_notes!(DialogueBuilder, dialogues);
