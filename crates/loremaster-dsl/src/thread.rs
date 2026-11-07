//! Long-range threads: promises to the reader.

use crate::pack::{register, replaceable, with_notes};
use loremaster_model::{Id, Thread, ThreadStatus};
use loremaster_project::Pack;

/// A thread of a pack, from [`BuildPack::thread`](crate::BuildPack::thread).
pub struct ThreadBuilder<'a> {
    pub(crate) pack: &'a mut Pack,
    pub(crate) i: usize,
}

impl<'a> ThreadBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, id: Id, name: String) -> Self {
        register(pack, format!("thread:{id}"));
        pack.threads.push(Thread {
            id,
            name,
            status: ThreadStatus::default(),
            summary: None,
            tracks: Vec::new(),
            notes: Vec::new(),
        });
        let i = pack.threads.len() - 1;
        ThreadBuilder { pack, i }
    }

    fn thread(&mut self) -> &mut Thread {
        &mut self.pack.threads[self.i]
    }

    /// `status:`.
    pub fn status(mut self, status: ThreadStatus) -> Self {
        self.thread().status = status;
        self
    }

    /// `summary:`.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.thread().summary = Some(summary.into());
        self
    }

    /// A variable or property that carries the thread.
    pub fn tracks(mut self, fact: impl Into<String>) -> Self {
        self.thread().tracks.push(fact.into());
        self
    }
}

replaceable!(ThreadBuilder, threads, "thread");
with_notes!(ThreadBuilder, threads);
