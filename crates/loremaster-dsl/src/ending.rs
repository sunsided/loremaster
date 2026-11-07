//! Endings: the ways a story can end.

use crate::pack::{register, replaceable, with_notes};
use loremaster_model::{Ending, EndingKind, Id};
use loremaster_project::Pack;

/// An ending of a pack, from [`BuildPack::ending`](crate::BuildPack::ending).
pub struct EndingBuilder<'a> {
    pub(crate) pack: &'a mut Pack,
    pub(crate) i: usize,
}

impl<'a> EndingBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, id: Id, kind: EndingKind, title: String) -> Self {
        register(pack, format!("ending:{id}"));
        pack.endings.push(Ending {
            id,
            kind,
            rank: None,
            title,
            text: String::new(),
            notes: Vec::new(),
        });
        let i = pack.endings.len() - 1;
        EndingBuilder { pack, i }
    }

    fn ending(&mut self) -> &mut Ending {
        &mut self.pack.endings[self.i]
    }

    /// `rank:`: 1 is the best ending.
    pub fn rank(mut self, rank: u32) -> Self {
        self.ending().rank = Some(rank);
        self
    }

    /// `text:`, with inline markup.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.ending().text = text.into();
        self
    }
}

replaceable!(EndingBuilder, endings, "ending");
with_notes!(EndingBuilder, endings);
