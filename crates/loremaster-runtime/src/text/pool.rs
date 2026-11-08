//! loremaster's side of dramaturge's text selection core (D2): the host types
//! of a pool, the judge, the progress store and the random numbers.
//!
//! dramaturge chooses which candidates show and which variant (`pick`,
//! modes, `seen_by`, speaker and topic). loremaster keeps the markup, the
//! layers, `replaces` and the joining of the text.

use crate::eval::Eval;
use crate::{State, rng};
use dramaturge_model::{CharacterId, Condition, Pool, TextTypes};
use dramaturge_runtime::{Count, FragmentProgress, Judge, ProgressStore, Query, TextRng};
use loremaster_lang::{CompiledLine, Segment};
use loremaster_model::{Id, Layer, PLAYER_SPEAKER};

/// The host types of loremaster's text pools.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lore;

impl TextTypes for Lore {
    type Leaf = Leaf;
    type Cond = Condition;
    /// A story variable that becomes true.
    type Learn = String;
    /// A thing id, or `player`.
    type Actor = Id;
    /// A thing or a topic id.
    type Topic = Id;
}

/// A text pool of loremaster.
pub(crate) type LorePool = Pool<Lore>;

/// What shows: compiled markup or a line, with the place it has in a
/// description. dramaturge does not look at `layer` and `replaces`.
#[derive(Debug, Clone)]
pub(crate) struct Leaf {
    pub text: LeafText,
    pub layer: Layer,
    pub replaces: bool,
    /// The fragment of the description (or the candidate of a named pool)
    /// that holds the leaf.
    pub fragment: usize,
    /// The condition of the candidate, as written.
    pub when_src: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum LeafText {
    /// A variant of a description fragment.
    Markup(Vec<Segment>),
    /// A line of a named text pool, with speech segments.
    Line(CompiledLine),
}

/// Conditions of a pool: `seen_by(x)` from the query's perceiver, the rest
/// as everywhere else.
pub(crate) struct LoreJudge<'e, 'a>(pub &'e Eval<'a>);

impl Judge<Lore> for LoreJudge<'_, '_> {
    fn holds(&self, cond: &Condition, query: &Query<Lore>) -> bool {
        match cond {
            Condition::All(conds) => conds.iter().all(|c| self.holds(c, query)),
            Condition::Any(conds) => conds.iter().any(|c| self.holds(c, query)),
            Condition::Not(inner) => !self.holds(inner, query),
            Condition::SeenBy(who) => query
                .perceiver
                .as_ref()
                .and_then(|p| self.entity(p))
                .is_some_and(|e| CharacterId::new(e) == *who),
            other => self.0.holds(other),
        }
    }
}

impl LoreJudge<'_, '_> {
    fn entity(&self, actor: &Id) -> Option<dramaturge_model::EntityId> {
        let scope = &self.0.world.scope;
        if actor.as_str() == PLAYER_SPEAKER {
            Some(scope.player())
        } else {
            scope.entity(actor.as_str())
        }
    }
}

/// The query of a text that the player sees: the player is the perceiver
/// until actors come (D6).
pub(crate) fn seen_by_player() -> Query<Lore> {
    Query::new().perceiver(Id::new(PLAYER_SPEAKER))
}

/// The progress of texts lives in the save state, keyed as dramaturge keys
/// it: `{pool id}/{candidate key or index}`.
///
/// "The first time" is the first visit of a room: of the room of a room
/// description, else of the current room (the caller of a named pool, or
/// the room where a thing is). The game counts the visits, so a resolution
/// counts nothing more here.
impl ProgressStore for State {
    fn fragment_progress(&self, key: &str) -> Option<FragmentProgress> {
        self.progress.get(key).map(|p| FragmentProgress {
            shown: p.shown,
            pick: p.pick,
        })
    }

    fn set_fragment_progress(&mut self, key: &str, progress: FragmentProgress) {
        let p = self.progress.entry(key.to_string()).or_default();
        p.shown = progress.shown;
        p.pick = progress.pick;
    }

    fn first_time(&self, count: Count<'_>) -> bool {
        let room = match count {
            Count::Inline { owner } => owner.strip_prefix("room:").unwrap_or(self.room.as_str()),
            Count::Named { caller, .. } => caller,
            // A kind of count that a later dramaturge adds: the current room.
            _ => self.room.as_str(),
        };
        self.visits_to(room) <= 1
    }

    fn count_resolution(&mut self, _: Count<'_>) {}
}

/// The story's random numbers, as the rest of the game draws them, so that
/// a seed gives the same text as before the core.
pub(crate) struct Rng<'a>(pub &'a mut u64);

impl TextRng for Rng<'_> {
    fn next_u64(&mut self) -> u64 {
        rng::next(self.0)
    }

    fn below(&mut self, n: u32) -> u32 {
        rng::below(self.0, n as usize) as u32
    }
}
