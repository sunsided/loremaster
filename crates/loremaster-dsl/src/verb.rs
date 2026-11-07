//! Custom verbs.

use crate::convert::strings;
use crate::pack::{register, replaceable};
use loremaster_model::{Id, VerbDef, VerbSlots};
use loremaster_project::Pack;

/// A custom verb of a pack, from [`BuildPack::verb`](crate::BuildPack::verb).
pub struct VerbBuilder<'a> {
    pub(crate) pack: &'a mut Pack,
    pub(crate) i: usize,
}

impl<'a> VerbBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, id: Id, label: String) -> Self {
        register(pack, format!("verb:{id}"));
        pack.verbs.push(VerbDef {
            id,
            label,
            words: Vec::new(),
            slots: VerbSlots::default(),
            preposition: None,
            bar: true,
            default: None,
        });
        let i = pack.verbs.len() - 1;
        VerbBuilder { pack, i }
    }

    fn verb(&mut self) -> &mut VerbDef {
        &mut self.pack.verbs[self.i]
    }

    /// Words and phrases the parser accepts.
    pub fn words(mut self, words: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.verb().words.extend(strings(words));
        self
    }

    /// `slots:`, how many objects the verb takes.
    pub fn slots(mut self, slots: VerbSlots) -> Self {
        self.verb().slots = slots;
        self
    }

    /// `preposition:` between two objects, such as `with`.
    pub fn preposition(mut self, preposition: impl Into<String>) -> Self {
        self.verb().preposition = Some(preposition.into());
        self
    }

    /// `bar: false`: a parser-only verb, not on the verb bar.
    pub fn parser_only(mut self) -> Self {
        self.verb().bar = false;
        self
    }

    /// `default:`, the response when no rule handles the verb.
    pub fn default_response(mut self, text: impl Into<String>) -> Self {
        self.verb().default = Some(text.into());
        self
    }
}

replaceable!(VerbBuilder, verbs, "verb");
