//! The ending checks (CK, P4-04): backward reachability over the explored
//! states.
//!
//! - A **dead end** is a reachable state from which no ending can be reached,
//!   the softlock of early King's Quest. It is an error.
//! - A **point of no return** is a turn after which the best reachable ending
//!   (by rank, 1 is the best) gets worse. It is a warning, unless a rule,
//!   machine step or agenda step names the endings with `closes` (and a
//!   `reason`): then the check compares the list with the endings that really
//!   drop out. A closer is named by its rule id, or by
//!   `agenda:<id>/steps[<i>]` for an agenda step.
//!
//! A reached ending, even a bad one, resolves the game, so a turn into an
//! ended state is neither. When the budget stopped the walk, a state whose
//! future the walk did not finish proves nothing: the checks report only
//! what holds for the explored graph, and say that the rest is not proven.

use super::commands;
use crate::{Game, World};
use loremaster_model::{Diagnostic, Effect, Story};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// What the ending checks found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndingCheck {
    /// False when the story has no ending: then there is nothing to check.
    pub ran: bool,
    /// True when the walk finished every state, so the checks are proven.
    /// False when the budget stopped it: no finding does not mean "no dead
    /// end" then.
    pub proven: bool,
    /// States whose future the walk did not finish (they can reach a state
    /// the budget cut). The checks report nothing for them.
    pub unproven: usize,
    /// The ways into dead ends, one for each cause, shortest first.
    pub dead_ends: Vec<DeadEnd>,
    /// Points of no return, one for each cause and set of lost endings,
    /// shortest first. The accepted ones are here too.
    pub no_return: Vec<NoReturn>,
    /// `closes` lists that name an ending which never drops out. Only when proven.
    pub stale: Vec<StaleCloses>,
}

/// A turn from a state that can still reach an ending into a dead end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadEnd {
    /// The shortest command list from the start into the dead end. Empty
    /// when the start itself is one.
    pub path: Vec<String>,
    /// The room in which the last command was given.
    pub room: String,
    /// The rule that the turn blames: the first rule that fired and changes
    /// the state. None when only the command's own action ran.
    pub rule: Option<String>,
    /// All rules that fired on the turn.
    pub fired: Vec<String>,
    /// How many explored turns have this cause.
    pub turns: usize,
}

/// A turn after which the best reachable ending gets worse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoReturn {
    /// The shortest command list from the start, with the turn as its last command.
    pub path: Vec<String>,
    /// The room in which the last command was given.
    pub room: String,
    /// The rule that the turn blames: a rule with `closes` that fired or can
    /// still fire, or else the first rule that fired and changes the state.
    pub rule: Option<String>,
    /// All rules that fired on the turn.
    pub fired: Vec<String>,
    /// The best reachable ending before the turn, and its rank.
    pub best_before: Option<String>,
    pub rank_before: Option<u32>,
    /// The best reachable ending after the turn, and its rank. None when only
    /// endings without a rank are left.
    pub best_after: Option<String>,
    pub rank_after: Option<u32>,
    /// The endings that can be reached before the turn, but not after it.
    pub lost: Vec<String>,
    /// The ranked endings in `lost` that no `closes` names. Empty when the
    /// point of no return is accepted.
    pub unaccepted: Vec<String>,
    /// The rules whose `closes` accept the drop.
    pub accepted_by: Vec<String>,
    /// The `reason` of each rule in `accepted_by`, in the same order.
    pub reasons: Vec<String>,
    /// How many explored turns have this cause.
    pub turns: usize,
}

impl NoReturn {
    pub fn accepted(&self) -> bool {
        self.unaccepted.is_empty()
    }
}

/// " (rule `x`)", or " (agenda step `agenda:keeper/steps[0]`)".
fn blame(closer: &str) -> String {
    if closer.starts_with("agenda:") {
        format!(" (agenda step `{closer}`)")
    } else {
        format!(" (rule `{closer}`)")
    }
}

fn quoted(path: &[String]) -> String {
    if path.is_empty() {
        "the start".into()
    } else {
        format!("\"{}\"", path.join(", "))
    }
}

fn names(list: &[String]) -> String {
    list.iter()
        .map(|e| format!("`{e}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn ranked(ending: Option<&String>, rank: Option<u32>) -> String {
    match (ending, rank) {
        (Some(e), Some(r)) => format!("`{e}`, rank {r}"),
        _ => "endings without a rank".into(),
    }
}

impl EndingCheck {
    /// The findings as diagnostics: a dead end is an error, a point of no
    /// return that no `closes` accepts and a stale `closes` are warnings,
    /// and so is a check that the budget stopped. `file` is empty: `at`
    /// names the entity (`rule:<id>`, or `story:explore`), and
    /// `Project::locate` gives the file.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        if !self.ran {
            return out;
        }
        let at = |rule: &Option<String>, field: &str| match rule {
            // An agenda step: `agenda:keeper/steps[0].closes`.
            Some(r) if r.starts_with("agenda:") => format!("{r}{}", field.replace('/', ".")),
            Some(r) => format!("rule:{r}{field}"),
            None => "story:explore".into(),
        };
        for d in &self.dead_ends {
            let by = d.rule.as_ref().map(|r| blame(r)).unwrap_or_default();
            let more = if d.turns > 1 {
                format!("; {} explored turns lead into dead ends this way", d.turns)
            } else {
                String::new()
            };
            out.push(Diagnostic::error(
                "",
                at(&d.rule, ""),
                format!(
                    "dead end: after {} in `{}`{by}, no ending can be reached{more}",
                    quoted(&d.path),
                    d.room
                ),
            ));
        }
        for p in self.no_return.iter().filter(|p| !p.accepted()) {
            let by = p.rule.as_ref().map(|r| blame(r)).unwrap_or_default();
            let more = if p.turns > 1 {
                format!(" ({} explored turns)", p.turns)
            } else {
                String::new()
            };
            out.push(Diagnostic::warning(
                "",
                at(&p.rule, "/closes"),
                format!(
                    "point of no return: after {} in `{}`{by}, the best reachable ending gets worse, from {} to {}{more}; {} can no longer be reached. If this is on purpose, name them in `closes: [{}]` of the step, with a `reason`",
                    quoted(&p.path),
                    p.room,
                    ranked(p.best_before.as_ref(), p.rank_before),
                    ranked(p.best_after.as_ref(), p.rank_after),
                    names(&p.unaccepted),
                    p.unaccepted.join(", "),
                ),
            ));
        }
        for s in &self.stale {
            out.push(Diagnostic::warning(
                "",
                at(&Some(s.rule.clone()), "/closes"),
                format!(
                    "`closes` of `{}` names {}, but no explored turn of the rule loses it",
                    s.rule,
                    names(&s.endings)
                ),
            ));
        }
        if !self.proven {
            out.push(Diagnostic::warning(
                "",
                "story:explore",
                format!(
                    "the ending checks are not proven: the explore budget stopped the walk, and {} states have ways on that it did not finish; dead ends and points of no return after them are not reported",
                    self.unproven
                ),
            ));
        }
        out
    }
}

/// A `closes` list that names endings which never drop out at the rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaleCloses {
    pub rule: String,
    pub endings: Vec<String>,
}

/// The explored graph, as the walk leaves it.
pub(super) struct Graph<'a> {
    pub nodes: &'a [(Game, Vec<String>)],
    pub edges: &'a [Vec<(u32, u32)>],
    pub open: &'a [bool],
    pub closers: &'a HashMap<String, u32>,
    /// The turns on which a rule with `closes` fired: (state, command, rule).
    pub closed: &'a [(u32, u32, u32)],
}

pub(super) struct Outcome {
    pub report: EndingCheck,
    pub stuck: usize,
    pub stuck_path: Vec<String>,
}

/// The rules and agenda steps with `closes`, numbered.
pub(super) fn closers(story: &Story) -> HashMap<String, u32> {
    let mut out: HashMap<String, u32> = HashMap::new();
    // Numbered by the map's size, so that two rules with one id (a check
    // error, but `explore` may run without `check`) keep the numbers dense.
    let rules = story.rules.iter().filter(|r| !r.closes.is_empty());
    let steps = story.agendas.iter().flat_map(|a| {
        a.steps
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.closes.is_empty())
            .map(move |(i, _)| step_closer(&a.id, i))
    });
    for id in rules.map(|r| r.id.to_string()).chain(steps) {
        let next = out.len() as u32;
        out.entry(id).or_insert(next);
    }
    out
}

/// The closer id of an agenda step.
fn step_closer(agenda: &loremaster_model::Id, index: usize) -> String {
    format!("agenda:{agenda}/steps[{index}]")
}

/// The agenda step of a closer id.
fn step_of<'a>(story: &'a Story, id: &str) -> Option<&'a loremaster_model::AgendaStep> {
    story.agendas.iter().find_map(|a| {
        a.steps
            .iter()
            .enumerate()
            .find(|(i, _)| step_closer(&a.id, *i) == id)
            .map(|(_, s)| s)
    })
}

/// The `closes` and `reason` of a closer: a rule, or an agenda step.
fn closes_and_reason<'a>(
    story: &'a Story,
    id: &str,
) -> Option<(&'a [loremaster_model::Id], Option<&'a String>)> {
    if let Some(r) = story.rules.iter().find(|r| r.id.as_str() == id) {
        return Some((&r.closes, r.reason.as_ref()));
    }
    step_of(story, id).map(|s| (s.closes.as_slice(), s.reason.as_ref()))
}

/// The rules that fired on a turn, by id, and the agenda steps that ran,
/// as closer ids, in log order.
pub(super) fn fired(story: &Story, turn: &crate::Turn) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for entry in &turn.log {
        let id = match entry.kind.as_str() {
            "rule" => Some(entry.text.clone()),
            // `keeper#1 runs`: the agenda and the key of the step.
            "agenda" => entry
                .text
                .strip_suffix(" runs")
                .and_then(|name| name.split_once('#'))
                .and_then(|(agenda, key)| {
                    let a = story.agenda(agenda)?;
                    let i = a.keyed_steps().position(|(k, _)| k == key)?;
                    Some(step_closer(&a.id, i))
                }),
            _ => None,
        };
        if let Some(id) = id.filter(|id| !out.contains(id)) {
            out.push(id);
        }
    }
    out
}

/// A set of small numbers per state, in one flat vector.
struct Sets {
    words: usize,
    bits: Vec<u64>,
}

impl Sets {
    fn new(states: usize, size: usize) -> Self {
        let words = size.div_ceil(64).max(1);
        Self {
            words,
            bits: vec![0; states * words],
        }
    }
    fn of(&self, i: usize) -> &[u64] {
        &self.bits[i * self.words..(i + 1) * self.words]
    }
    fn insert(&mut self, i: usize, bit: usize) {
        self.bits[i * self.words + bit / 64] |= 1 << (bit % 64);
    }
    fn is_empty(&self, i: usize) -> bool {
        self.of(i).iter().all(|w| *w == 0)
    }
    /// Add the set of `from` to the set of `to`. True when it grew.
    fn absorb(&mut self, to: usize, from: usize) -> bool {
        let mut grew = false;
        for w in 0..self.words {
            let add = self.bits[from * self.words + w];
            let old = self.bits[to * self.words + w];
            if old | add != old {
                self.bits[to * self.words + w] = old | add;
                grew = true;
            }
        }
        grew
    }
    /// Backward closure: each state gets the sets of the states after it.
    fn close(&mut self, preds: &Preds) {
        let n = self.bits.len() / self.words;
        let mut queue: VecDeque<usize> = (0..n).filter(|&i| !self.is_empty(i)).collect();
        let mut queued: Vec<bool> = (0..n).map(|i| !self.is_empty(i)).collect();
        while let Some(v) = queue.pop_front() {
            queued[v] = false;
            for &u in preds.of(v) {
                if self.absorb(u as usize, v) && !queued[u as usize] {
                    queued[u as usize] = true;
                    queue.push_back(u as usize);
                }
            }
        }
    }
}

fn members(set: &[u64]) -> impl Iterator<Item = usize> + '_ {
    set.iter().enumerate().flat_map(|(w, &bits)| {
        (0..64)
            .filter(move |b| bits & (1 << b) != 0)
            .map(move |b| w * 64 + b)
    })
}

/// The states before each state, in one flat vector.
struct Preds {
    start: Vec<u32>,
    list: Vec<u32>,
}

impl Preds {
    fn new(edges: &[Vec<(u32, u32)>]) -> Self {
        let n = edges.len();
        let mut count = vec![0u32; n + 1];
        for out in edges {
            for &(j, _) in out {
                count[j as usize + 1] += 1;
            }
        }
        for i in 0..n {
            count[i + 1] += count[i];
        }
        let mut fill = count.clone();
        let mut list = vec![0u32; count[n] as usize];
        for (i, out) in edges.iter().enumerate() {
            for &(j, _) in out {
                list[fill[j as usize] as usize] = i as u32;
                fill[j as usize] += 1;
            }
        }
        Self { start: count, list }
    }
    fn of(&self, i: usize) -> &[u32] {
        &self.list[self.start[i] as usize..self.start[i + 1] as usize]
    }
}

/// Better is smaller: rank 1 is the best. An ending without a rank is worse
/// than every ranked one.
fn worse(before: Option<u32>, after: Option<u32>) -> bool {
    match (before, after) {
        (Some(a), Some(b)) => b > a,
        (Some(_), None) => true,
        _ => false,
    }
}

pub(super) fn check(world: &World, g: &Graph<'_>) -> Outcome {
    let story = &world.story;
    let n = g.nodes.len();
    let none = Outcome {
        report: EndingCheck::default(),
        stuck: 0,
        stuck_path: Vec::new(),
    };
    if story.endings.is_empty() || n == 0 {
        return none;
    }

    // The endings, numbered; an ended state with an unknown id gets its own number.
    let mut ids: Vec<String> = story.endings.iter().map(|e| e.id.to_string()).collect();
    let mut ranks: Vec<Option<u32>> = story.endings.iter().map(|e| e.rank).collect();
    let mut index: HashMap<String, usize> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();
    let ended: Vec<bool> = g.nodes.iter().map(|(s, _)| s.state().ended).collect();
    let ended_as: Vec<Option<usize>> = g
        .nodes
        .iter()
        .map(|(s, _)| {
            let state = s.state();
            state.ended.then(|| {
                let id = state.ending.as_ref().map_or("", |e| e.as_str()).to_string();
                *index.entry(id.clone()).or_insert_with(|| {
                    ids.push(id);
                    ranks.push(None);
                    ids.len() - 1
                })
            })
        })
        .collect();

    let preds = Preds::new(g.edges);
    let mut reach = Sets::new(n, ids.len());
    for (i, e) in ended_as.iter().enumerate() {
        if let Some(e) = e {
            reach.insert(i, *e);
        }
    }
    reach.close(&preds);

    // States that can reach a state the walk did not finish: not proven.
    let mut unproven: Vec<bool> = g.open.to_vec();
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| unproven[i]).collect();
    while let Some(v) = queue.pop_front() {
        for &u in preds.of(v) {
            if !unproven[u as usize] {
                unproven[u as usize] = true;
                queue.push_back(u as usize);
            }
        }
    }

    // The rules with `closes` that can still fire after each state.
    let rules: HashMap<&str, &loremaster_model::Rule> =
        story.rules.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut closer_ids = vec![String::new(); g.closers.len()];
    for (id, &r) in g.closers {
        closer_ids[r as usize] = id.clone();
    }
    let mut ahead = Sets::new(n, closer_ids.len());
    let mut on_turn: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for &(i, c, r) in g.closed {
        ahead.insert(i as usize, r as usize);
        on_turn.entry((i, c)).or_default().push(r);
    }
    if !g.closed.is_empty() {
        ahead.close(&preds);
    }
    let closes_of = |r: usize| -> Vec<usize> {
        closes_and_reason(story, &closer_ids[r])
            .map_or(&[][..], |(c, _)| c)
            .iter()
            .filter_map(|e| index.get(e.as_str()).copied())
            .collect()
    };

    let best = |i: usize| -> Option<usize> {
        members(reach.of(i))
            .filter(|&e| ranks[e].is_some())
            .min_by_key(|&e| (ranks[e], e))
    };
    let rank = |e: Option<usize>| e.and_then(|e| ranks[e]);
    let dead: Vec<bool> = (0..n)
        .map(|i| !ended[i] && reach.is_empty(i) && !unproven[i])
        .collect();

    // Replay one turn for the rules that fired on it.
    let replay = |i: usize, c: u32| -> Vec<String> {
        let mut game = g.nodes[i].0.clone();
        let Some((_, cmd)) = commands(&game).into_iter().nth(c as usize) else {
            return Vec::new();
        };
        fired(story, &game.act(cmd))
    };
    let changes = |id: &str| {
        let then = rules
            .get(id)
            .map(|r| &r.then)
            .or_else(|| step_of(story, id).map(|s| &s.then));
        then.is_some_and(|t| t.iter().any(|e| !matches!(e, Effect::Say(_))))
    };
    // The shortest path into a state, then the command of the turn.
    let path_of = |i: usize, c: u32| {
        let mut p = g.nodes[i].1.clone();
        p.push(label(&g.nodes[i].0, c));
        p
    };
    let room_of = |i: usize| g.nodes[i].0.state().room.to_string();

    // Dead ends: the turns into them, by cause.
    let mut dead_ends: Vec<DeadEnd> = Vec::new();
    let mut dead_at: BTreeMap<Option<String>, usize> = BTreeMap::new();
    if dead[0] {
        dead_ends.push(DeadEnd {
            path: Vec::new(),
            room: room_of(0),
            rule: None,
            fired: Vec::new(),
            turns: 1,
        });
    } else {
        for i in (0..n).filter(|&i| !dead[i]) {
            for &(j, c) in &g.edges[i] {
                let j = j as usize;
                if !dead[j] {
                    continue;
                }
                let fired = replay(i, c);
                let rule = fired.iter().find(|r| changes(r)).cloned();
                match dead_at.get(&rule) {
                    Some(&k) => dead_ends[k].turns += 1,
                    None => {
                        dead_at.insert(rule.clone(), dead_ends.len());
                        // The states come in walk order, so the first turn
                        // of a cause has the shortest path.
                        dead_ends.push(DeadEnd {
                            path: path_of(i, c),
                            room: room_of(i),
                            rule,
                            fired,
                            turns: 1,
                        });
                    }
                }
            }
        }
    }

    // Points of no return: turns between proven states into states that are
    // not ended and not dead, after which the best rank is worse. A state
    // that can reach a cut state has an incomplete set: it proves nothing.
    let mut no_return: Vec<NoReturn> = Vec::new();
    // By cause: the blamed rule, the lost endings, the unaccepted ones.
    type Cause = (Option<String>, Vec<usize>, Vec<usize>);
    let mut no_return_at: BTreeMap<Cause, usize> = BTreeMap::new();
    let mut lost_by: Vec<Vec<bool>> = vec![vec![false; ids.len()]; closer_ids.len()];
    for i in (0..n).filter(|&i| !ended[i] && !unproven[i]) {
        for &(j, c) in &g.edges[i] {
            let ju = j as usize;
            let pairs = || reach.of(i).iter().zip(reach.of(ju));
            if pairs().all(|(a, b)| a & !b == 0) {
                // Most turns lose nothing.
                continue;
            }
            let bits: Vec<u64> = pairs().map(|(a, b)| a & !b).collect();
            let lost: Vec<usize> = members(&bits).collect();
            if lost.is_empty() {
                continue;
            }
            // The rules with `closes` that this turn fires, or that can still fire after it.
            let mut candidates: Vec<usize> = on_turn
                .get(&(i as u32, c))
                .map(|v| v.iter().map(|&r| r as usize).collect())
                .unwrap_or_default();
            for r in members(ahead.of(ju)) {
                if !candidates.contains(&r) {
                    candidates.push(r);
                }
            }
            // Every turn counts for the stale `closes` check, also one into
            // an ending or a dead end.
            for &r in &candidates {
                for &e in &lost {
                    lost_by[r][e] = true;
                }
            }
            if ended[ju] || dead[ju] {
                continue;
            }
            let (before, after) = (best(i), best(ju));
            if !worse(rank(before), rank(after)) {
                continue;
            }
            let covering: Vec<usize> = candidates
                .iter()
                .copied()
                .filter(|&r| closes_of(r).iter().any(|e| lost.contains(e)))
                .collect();
            let named: Vec<usize> = covering.iter().flat_map(|&r| closes_of(r)).collect();
            let unaccepted: Vec<usize> = lost
                .iter()
                .copied()
                .filter(|e| ranks[*e].is_some() && !named.contains(e))
                .collect();
            let fired = replay(i, c);
            let rule = covering
                .first()
                .map(|&r| closer_ids[r].clone())
                .or_else(|| fired.iter().find(|r| changes(r)).cloned());
            let key = (rule.clone(), lost.clone(), unaccepted.clone());
            match no_return_at.get(&key) {
                Some(&k) => no_return[k].turns += 1,
                None => {
                    no_return_at.insert(key, no_return.len());
                    let accepted_by: Vec<String> = if unaccepted.is_empty() {
                        covering.iter().map(|&r| closer_ids[r].clone()).collect()
                    } else {
                        Vec::new()
                    };
                    let reasons = accepted_by
                        .iter()
                        .map(|r| {
                            closes_and_reason(story, r)
                                .and_then(|(_, reason)| reason.cloned())
                                .unwrap_or_default()
                        })
                        .collect();
                    no_return.push(NoReturn {
                        path: path_of(i, c),
                        room: room_of(i),
                        rule,
                        fired,
                        best_before: before.map(|e| ids[e].clone()),
                        rank_before: rank(before),
                        best_after: after.map(|e| ids[e].clone()),
                        rank_after: rank(after),
                        lost: lost.iter().map(|&e| ids[e].clone()).collect(),
                        unaccepted: unaccepted.iter().map(|&e| ids[e].clone()).collect(),
                        accepted_by,
                        reasons,
                        turns: 1,
                    });
                }
            }
        }
    }

    let proven = !g.open.iter().any(|o| *o);
    let stale = if proven {
        closer_ids
            .iter()
            .enumerate()
            .filter_map(|(r, id)| {
                let endings: Vec<String> = closes_and_reason(story, id)
                    .map_or(&[][..], |(c, _)| c)
                    .iter()
                    .filter(|e| index.get(e.as_str()).is_some_and(|&x| !lost_by[r][x]))
                    .map(ToString::to_string)
                    .collect();
                (!endings.is_empty()).then(|| StaleCloses {
                    rule: id.clone(),
                    endings,
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let stuck = dead.iter().filter(|d| **d).count();
    let stuck_path = (0..n)
        .find(|&i| dead[i])
        .map(|i| g.nodes[i].1.clone())
        .unwrap_or_default();
    Outcome {
        report: EndingCheck {
            ran: true,
            proven,
            unproven: unproven.iter().filter(|u| **u).count(),
            dead_ends,
            no_return,
            stale,
        },
        stuck,
        stuck_path,
    }
}

/// The label of command `c` in a state.
fn label(game: &Game, c: u32) -> String {
    commands(game)
        .into_iter()
        .nth(c as usize)
        .map(|(l, _)| l)
        .unwrap_or_default()
}
