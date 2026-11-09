//! The state explorer: a breadth-first walk over the states a player can
//! reach, with a budget. It answers "can this ever show?" and "can the player
//! get stuck?" for the whole story, not only for the states the author tried.

use crate::game::SWITCH;
use crate::game::{ATTACK_TEXT, LUCK_TEXT};
use crate::{BlockKind, Command, Game, Turn};
use loremaster_model::{EventPattern, Id, Location, Value, VerbSlots};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

mod endings;

const WIN_ROUND: &str = " (win the round)";
const LOSE_ROUND: &str = " (lose the round)";

pub use endings::{DeadEnd, EndingCheck, NoReturn, StaleCloses};

/// Limits for one exploration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Budget {
    /// Stop after this many distinct states.
    pub max_states: usize,
    /// Do not go deeper than this many commands from the start.
    pub max_depth: usize,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_states: 2000,
            max_depth: 40,
        }
    }
}

/// One way the text of a room can read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextVariant {
    pub text: String,
    /// How many explored states show this text.
    pub states: usize,
    /// The shortest command list from the start that shows it.
    pub path: Vec<String>,
    /// Variables and properties in the first state that showed it.
    pub facts: Vec<String>,
}

/// What the explorer found for one room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCoverage {
    pub id: String,
    pub name: String,
    pub reached: bool,
    pub variants: Vec<TextVariant>,
    /// Every full room text the explorer saw, with the things in the room. The room matrix uses it to mark reachable combinations.
    pub seen: Vec<String>,
}

/// An ending the explorer reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndingFound {
    pub text: String,
    pub path: Vec<String>,
}

/// The result of an exploration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exploration {
    pub states: usize,
    /// True when the budget stopped the walk; the results are then a lower bound.
    pub truncated: bool,
    pub rooms: Vec<RoomCoverage>,
    /// Fragments that never showed, as `owner#index: start of the text`.
    pub never_shown: Vec<String>,
    /// Rules that never fired.
    pub never_fired: Vec<String>,
    pub endings: Vec<EndingFound>,
    /// Dead ends: states from which no ending can be reached. 0 when the story
    /// has no ending. When the budget stopped the walk, only the proven ones
    /// count: states whose future the walk did not finish are left out.
    pub stuck: usize,
    /// The shortest path into a stuck state, if there is one.
    pub stuck_path: Vec<String>,
    /// The ending checks: dead ends and points of no return (CK, P4-04).
    #[serde(default)]
    pub ending_checks: EndingCheck,
    /// For each open or resolved thread with a payoff: whether a payoff showed, and in which order.
    #[serde(default)]
    pub threads: Vec<crate::threads::ThreadReach>,
}

/// A state as the explorer compares it: the parts that change what can happen.
/// The clock counts only by phase, and by the minutes until each agenda's
/// next step is due. Timers count by the turns they have left. Counters
/// count only up to 3, and only when the story reads them. Of the rules
/// that fired, only the `once` rules count: the game reads `fired` only to
/// keep them from firing again.
fn fingerprint(game: &Game, counters: bool, once_ids: &BTreeSet<&Id>) -> String {
    let s = game.state();
    let cap = |m: &BTreeMap<Id, u32>| -> BTreeMap<String, u32> {
        if !counters {
            return BTreeMap::new();
        }
        m.iter()
            .map(|(k, v)| (k.to_string(), (*v).min(3)))
            .collect()
    };
    let visited: BTreeSet<&Id> = s.visits.keys().collect();
    let fired_once: Vec<&Id> = s.fired.iter().filter(|id| once_ids.contains(id)).collect();
    serde_json::to_string(&(
        &s.room,
        &s.facts,
        &s.locations,
        fired_once,
        game.eval().phase(),
        // Two endings from one state are two states.
        (s.ended, &s.ending),
        visited,
        cap(&s.talked),
        cap(&s.interactions),
        &s.talk,
        &s.relations,
        &s.memories,
        // Not the await's id: it counts the awaits, and equal states stay equal.
        s.awaiting
            .as_ref()
            .map(|a| (&a.rule, a.next, &a.request, &a.into)),
        // Only the hit that waits for its Luck test; round and log are history.
        s.combat.as_ref().map(|c| (true, &c.pending)),
        // In one tuple: serde serializes tuples of 16 at most.
        // The turns left of each timer: a timer that runs out can end the game.
        (
            &s.actors,
            game.agenda_waits(),
            &s.active,
            &s.orders,
            &s.timers,
        ),
    ))
    .unwrap_or_default()
}

/// True when any condition or markup in the story reads the talk or interaction counters.
fn reads_counters(game: &Game) -> bool {
    let text = serde_json::to_string(&game.world().story).unwrap_or_default();
    text.contains(".talked") || text.contains(".interactions")
}

/// The commands worth trying in a state: the actions that can move the story
/// forward. Commands that only undo or repeat (drop, close, turn off, lock) and
/// commands that can not change the state (examine with no rule, wait in a
/// story without time rules) are left out, so the walk stays small. With a
/// carry limit and full hands, `drop` frees a hand, so it is tried then.
fn commands(game: &Game) -> Vec<(String, Command)> {
    let world = game.world();
    let story = &world.story;
    let scope = game.in_scope();
    let mut out: Vec<(String, Command)> = Vec::new();
    // A rule waits: only its answers run.
    if game.awaiting().is_some() {
        let combat = game.state().combat.is_some();
        let mut out = Vec::new();
        for c in game.choices() {
            let cmd = Command::Choose { index: c.index };
            // shortcut: the walk skips Luck tests. They only shift damage and
            // multiply the states; win, loss and escape stay reachable.
            if combat && c.text == LUCK_TEXT {
                continue;
            }
            if combat && c.text.starts_with(ATTACK_TEXT) {
                out.push((format!("answer {}{WIN_ROUND}", c.text), cmd.clone()));
                out.push((format!("answer {}{LOSE_ROUND}", c.text), cmd));
            } else {
                out.push((format!("answer {}", c.text), cmd));
            }
        }
        return unique(out);
    }
    // In a talk, only the answers and leaving run.
    if game.talk().is_some() {
        out.extend(game.choices().into_iter().map(|c| {
            (
                format!("say {}", c.text),
                Command::Choose { index: c.index },
            )
        }));
        out.push(("leave".into(), Command::Leave));
        return unique(out);
    }
    let verb = |v: &str, objects: Vec<Id>| Command::verb(v, objects);
    let label = |v: &str, objects: &[Id]| {
        std::iter::once(v.to_string())
            .chain(objects.iter().map(Id::to_string))
            .collect::<Vec<_>>()
            .join(" ")
    };

    for (_, exit) in game.visible_exits() {
        let dir = exit.direction.name().to_string();
        out.push((format!("go {dir}"), Command::Go { direction: dir }));
    }
    // Verbs that rules listen to, with exactly the things the rules name.
    // A rule for another actor's commands (`actor: keeper`) is not a command
    // of the player, and an indirect object needs a direct one.
    for rule in &story.rules {
        if let EventPattern::Verb(p) = &rule.on
            && for_player(p)
            && (p.direct.is_some() || p.indirect.is_none())
        {
            let objects: Vec<Id> = p.objects().cloned().collect();
            if !objects.iter().all(|o| scope.contains(o)) {
                continue;
            }
            // The label is the dedup key: keep the slots that tell rules apart.
            let mut text = label(p.verb.as_str(), &objects);
            if let Some(prep) = &p.preposition {
                text.push_str(&format!(" ({prep})"));
            }
            if let Some(topic) = &p.topic {
                text.push_str(&format!(" about {topic}"));
            }
            out.push((
                text,
                Command::Verb {
                    verb: p.verb.clone(),
                    objects,
                    actor: None,
                    preposition: p.preposition.clone(),
                    topic: p.topic.clone(),
                },
            ));
        }
    }
    // The standard verbs that change state.
    let take_rules = story.rules.iter().any(|r| {
        matches!(&r.on, EventPattern::Verb(p) | EventPattern::Check(p)
            if p.verb.as_str() == "take"
                && for_player(p))
    });
    for t in scope.iter().filter_map(|id| story.thing(id.as_str())) {
        let id = t.id.clone();
        if story.dialogue_with(id.as_str()).is_some() {
            out.push((
                label("talk", std::slice::from_ref(&id)),
                verb("talk", vec![id.clone()]),
            ));
        }
        let carried = matches!(
            game.state().location_of(id.as_str()),
            Some(Location::Carried)
        );
        // With a carry limit, full hands refuse the default take, and a drop
        // frees a hand: it is no longer only an undo. A rule on `take` can
        // still handle a take with full hands.
        let full = story
            .carry_limit
            .is_some_and(|limit| game.state().held_count() >= limit);
        if full && carried {
            out.push((
                label("drop", std::slice::from_ref(&id)),
                verb("drop", vec![id.clone()]),
            ));
        }
        if t.portable && !carried && (!full || take_rules) {
            out.push((
                label("take", std::slice::from_ref(&id)),
                verb("take", vec![id.clone()]),
            ));
        }
        for (prop, on) in [("open", "open"), ("lit", "turn_on")] {
            if t.props.contains_key(prop) {
                out.push((
                    label(on, std::slice::from_ref(&id)),
                    verb(on, vec![id.clone()]),
                ));
            }
        }
        // A search reveals what is concealed in or on the thing (WO-2).
        if !game.searchable(id.as_str()).is_empty() {
            out.push((
                label("search", std::slice::from_ref(&id)),
                verb("search", vec![id.clone()]),
            ));
        }
        if let Some(key) = &t.key
            && scope.contains(key)
        {
            let objects = vec![id.clone(), key.clone()];
            out.push((label("unlock", &objects), verb("unlock", objects)));
        }
    }
    // Waiting, when the story reacts to time.
    // Waiting matters when the story reacts to time: rules on time and turns, and timers.
    // An agenda step is time-driven too: it is due at a time of day.
    if !story.timers.is_empty()
        || !story.agendas.is_empty()
        || world.behaviors_watch_time()
        || story
            .rules
            .iter()
            .any(|r| matches!(r.on, EventPattern::Time(_) | EventPattern::Turn))
    {
        out.push(("wait".into(), verb("wait", Vec::new())));
    }
    // shortcut: orders are not tried, because a switch reaches the same
    // states. Upgrade when an order can do what a switch cannot (timing).
    for a in story
        .playable
        .iter()
        .filter(|a| Some(*a) != game.state().active.as_ref())
    {
        out.push((format!("{SWITCH} {a}"), verb(SWITCH, vec![a.clone()])));
    }
    // Custom verbs without things.
    for v in story.verbs.iter().filter(|v| v.slots == VerbSlots::None) {
        out.push((v.id.to_string(), verb(v.id.as_str(), Vec::new())));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

/// Labels that a path can replay: of two answers with one text, the second
/// gets ` (2)`, the third ` (3)`, and so on.
fn unique(mut out: Vec<(String, Command)>) -> Vec<(String, Command)> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (label, _) in &mut out {
        let n = seen.entry(label.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            label.push_str(&format!(" ({n})"));
        }
    }
    out
}

/// Force the rolls of a combat round for its label in [`commands`], so that
/// the walk sees both outcomes. The extreme rolls keep impossible outcomes
/// out: with a big gap in SKILL, a forced "win" still loses.
fn force_round(game: &mut Game, label: &str) {
    // Other labels hold text of the author, which can end in the same words.
    if game.state().combat.is_none() {
        return;
    }
    let (enemy, player) = if label.ends_with(WIN_ROUND) {
        (2, 12)
    } else if label.ends_with(LOSE_ROUND) {
        (12, 2)
    } else {
        return;
    };
    let mut totals = vec![enemy; game.combat_fighters()];
    totals.push(player);
    game.forced.totals = totals.into();
}

/// The command of one step of an explorer path in this state, with the rolls
/// of a combat round forced as the walk saw them. `None` when the state has
/// no such step, also when the story has ended.
pub(crate) fn step_command(game: &mut Game, step: &str) -> Option<Command> {
    if game.state().ended {
        return None;
    }
    let (label, cmd) = commands(game).into_iter().find(|(l, _)| l == step)?;
    force_round(game, &label);
    Some(cmd)
}

/// A pattern that matches the player's commands: no `actor`, or the player.
fn for_player(p: &loremaster_model::CommandPattern) -> bool {
    p.actor.as_ref().is_none_or(|a| a.as_str() == crate::PLAYER)
}

fn facts_of(game: &Game) -> Vec<String> {
    game.state()
        .facts
        .iter()
        .filter(|(_, v)| !matches!(v, Value::Bool(false)))
        .map(|(k, v)| format!("{k} = {v}"))
        .collect()
}

fn description(turn: &Turn) -> String {
    turn.blocks
        .iter()
        // The written text only. The objects layer lists things, and every way of
        // carrying them around would count as its own variant.
        .filter(|b| b.kind == BlockKind::Prose)
        .map(|b| b.plain_text())
        .collect::<Vec<_>>()
        .join("\n")
}

fn sources(turn: &Turn, into: &mut BTreeSet<(String, usize)>) {
    for block in &turn.blocks {
        for run in &block.runs {
            if let Some(src) = &run.source {
                into.insert((src.owner.clone(), src.fragment));
            }
        }
    }
}

/// Walk the story from its start.
pub fn explore(start: &Game, budget: Budget) -> Exploration {
    let mut game = start.clone();
    // The explorer rolls itself: a roll for the host would leave it without
    // a move. An action without results stays a dead end; its host decides.
    game.host_rolls = false;
    game.host_walks = false;
    let opening = game.start();
    let counters = reads_counters(&game);
    let once_ids: BTreeSet<&Id> = start
        .world()
        .story
        .rules
        .iter()
        .filter(|r| crate::game::reads_fired(r))
        .map(|r| &r.id)
        .collect();

    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut nodes: Vec<(Game, Vec<String>)> = Vec::new();
    // The graph for the ending checks: for each state, one edge per command
    // (the next state, and the index of the command in `commands` of the
    // state), so that two commands into one state stay two turns.
    let mut edges: Vec<Vec<(u32, u32)>> = Vec::new();
    // States whose future the walk did not finish: the depth cut them, or the
    // budget dropped a state after them.
    let mut open: Vec<bool> = Vec::new();
    // The rules with `closes`, and the turns on which they fired: (state, command, rule).
    let closers = endings::closers(&start.world().story);
    let mut closed: Vec<(u32, u32, u32)> = Vec::new();
    let mut shown: BTreeSet<(String, usize)> = BTreeSet::new();
    let mut fired: BTreeSet<String> = BTreeSet::new();
    let mut variants: BTreeMap<String, BTreeMap<String, TextVariant>> = BTreeMap::new();
    let mut full_texts: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut endings: BTreeMap<String, EndingFound> = BTreeMap::new();
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut truncated = false;

    // Thread beats: the beats that each state's path showed, to see a payoff before its setup.
    let (thread_ids, beats) = crate::threads::beats(&start.world().story);
    let mut histories: Vec<BTreeSet<usize>> = Vec::new();
    let mut paid_off = vec![false; thread_ids.len()];
    let mut too_early: Vec<Option<Vec<String>>> = vec![None; thread_ids.len()];
    let beats_of = |turn: &Turn,
                    before: &BTreeSet<usize>,
                    path: &[String],
                    paid_off: &mut Vec<bool>,
                    too_early: &mut Vec<Option<Vec<String>>>| {
        let mut after = before.clone();
        for (b, beat) in beats.iter().enumerate() {
            if after.contains(&b) || !beat.hit(turn) {
                continue;
            }
            if beat.payoff {
                paid_off[beat.thread] = true;
                let set_up = beats
                    .iter()
                    .enumerate()
                    .any(|(s, x)| x.thread == beat.thread && !x.payoff && after.contains(&s));
                let has_setups = beats.iter().any(|x| x.thread == beat.thread && !x.payoff);
                let shorter = too_early[beat.thread]
                    .as_ref()
                    .is_none_or(|p| p.len() > path.len());
                if has_setups && !set_up && shorter {
                    too_early[beat.thread] = Some(path.to_vec());
                }
            }
            after.insert(b);
        }
        after
    };

    sources(&opening, &mut shown);
    let mut record = |game: &Game,
                      path: &[String],
                      variants: &mut BTreeMap<String, BTreeMap<String, TextVariant>>,
                      shown: &mut BTreeSet<(String, usize)>| {
        let preview = game.describe_preview();
        sources(&preview, shown);
        let text = description(&preview);
        full_texts
            .entry(game.state().room.to_string())
            .or_default()
            .insert(crate::matrix::room_text(&preview).0);
        let entry = variants
            .entry(game.state().room.to_string())
            .or_default()
            .entry(text.clone())
            .or_insert_with(|| TextVariant {
                text,
                states: 0,
                path: path.to_vec(),
                facts: facts_of(game),
            });
        entry.states += 1;
    };

    seen.insert(fingerprint(&game, counters, &once_ids), 0);
    record(&game, &[], &mut variants, &mut shown);
    histories.push(beats_of(
        &opening,
        &BTreeSet::new(),
        &[],
        &mut paid_off,
        &mut too_early,
    ));
    nodes.push((game, Vec::new()));
    edges.push(Vec::new());
    open.push(false);
    queue.push_back(0);

    while let Some(i) = queue.pop_front() {
        let (game, path) = nodes[i].clone();
        if game.state().ended {
            continue;
        }
        if path.len() >= budget.max_depth {
            // The depth stops the walk here: what comes after is not known,
            // unless nothing can come after it.
            if commands(&game).is_empty() {
                continue;
            }
            open[i] = true;
            truncated = true;
            continue;
        }
        for (c, (label, cmd)) in commands(&game).into_iter().enumerate() {
            let mut next = game.clone();
            force_round(&mut next, &label);
            let turn = next.act(cmd);
            sources(&turn, &mut shown);
            let closing: Vec<u32> = endings::fired(&start.world().story, &turn)
                .iter()
                .filter_map(|id| closers.get(id.as_str()).copied())
                .collect();
            for entry in turn.log.iter().filter(|l| l.kind == "rule") {
                fired.insert(entry.text.clone());
            }
            let key = fingerprint(&next, counters, &once_ids);
            let mut next_path = path.clone();
            next_path.push(label);
            let history = beats_of(
                &turn,
                &histories[i],
                &next_path,
                &mut paid_off,
                &mut too_early,
            );
            let j = match seen.get(&key) {
                Some(&j) => j,
                None => {
                    if nodes.len() >= budget.max_states {
                        truncated = true;
                        open[i] = true;
                        continue;
                    }
                    if next.state().ended {
                        let text = turn
                            .blocks
                            .iter()
                            .filter(|b| b.kind == BlockKind::End)
                            .map(|b| b.plain_text())
                            .collect::<Vec<_>>()
                            .join(" ");
                        endings.entry(text.clone()).or_insert(EndingFound {
                            text,
                            path: next_path.clone(),
                        });
                    }
                    record(&next, &next_path, &mut variants, &mut shown);
                    histories.push(history);
                    let j = nodes.len();
                    seen.insert(key, j);
                    nodes.push((next, next_path));
                    edges.push(Vec::new());
                    open.push(false);
                    queue.push_back(j);
                    j
                }
            };
            let (from, c) = (i as u32, c as u32);
            edges[i].push((j as u32, c));
            closed.extend(closing.into_iter().map(|r| (from, c, r)));
        }
    }

    let ending_checks = endings::check(
        start.world(),
        &endings::Graph {
            nodes: &nodes,
            edges: &edges,
            open: &open,
            closers: &closers,
            closed: &closed,
        },
    );
    let (stuck, stuck_path) = (ending_checks.stuck, ending_checks.stuck_path.clone());

    let world = start.world();
    let rooms = world
        .story
        .rooms
        .iter()
        .map(|r| {
            let found = variants.remove(r.id.as_str()).unwrap_or_default();
            let mut list: Vec<TextVariant> = found.into_values().collect();
            list.sort_by_key(|v| v.path.len());
            RoomCoverage {
                id: r.id.to_string(),
                name: r.name.clone(),
                reached: !list.is_empty(),
                variants: list,
                seen: full_texts
                    .remove(r.id.as_str())
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
            }
        })
        .collect();

    let mut never_shown = Vec::new();
    let mut check = |owner: String, fragments: &[loremaster_model::Fragment]| {
        for (i, f) in fragments.iter().enumerate() {
            if !shown.contains(&(owner.clone(), i)) {
                let text: String = f
                    .variants
                    .first()
                    .map_or("", String::as_str)
                    .chars()
                    .take(60)
                    .collect();
                never_shown.push(format!("{owner}#{}: {text}", i + 1));
            }
        }
    };
    for r in &world.story.rooms {
        check(format!("room:{}", r.id), &r.description);
    }
    for t in &world.story.things {
        check(format!("thing:{}/presence", t.id), &t.presence);
    }

    let never_fired = world
        .story
        .rules
        .iter()
        .map(|r| r.id.to_string())
        .filter(|id| !fired.contains(id))
        .collect();

    Exploration {
        states: nodes.len(),
        truncated,
        rooms,
        never_shown,
        never_fired,
        endings: endings.into_values().collect(),
        stuck,
        stuck_path,
        ending_checks: ending_checks.report,
        threads: thread_ids
            .iter()
            .enumerate()
            .filter(|(t, _)| beats.iter().any(|b| b.thread == *t && b.payoff))
            .map(|(t, id)| crate::threads::ThreadReach {
                thread: id.clone(),
                paid_off: paid_off[t],
                payoff_before_setup: too_early[t].clone().unwrap_or_default(),
                payoffs: crate::threads::payoff_labels(&beats, t),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Every read of `State.fired` that changes play goes through
    /// `reads_fired`, and the explorer builds its set from it: the
    /// fingerprint keeps only those rules, so another read would merge
    /// states that play differently.
    #[test]
    fn the_game_reads_fired_only_for_once_rules() {
        fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    out.push((path.display().to_string(), text));
                }
            }
        }
        let mut files = Vec::new();
        walk(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        let mut reads = Vec::new();
        for (path, text) in &files {
            let fingerprint = path.ends_with("explore.rs");
            // Not this test.
            let code = text.split("#[cfg(test)]").next().unwrap();
            for (at, _) in code.match_indices(".fired") {
                let line_start = code[..at].rfind('\n').map_or(0, |n| n + 1);
                if code[line_start..at].trim_start().starts_with("//") {
                    continue;
                }
                let after = &code[at + ".fired".len()..];
                if after.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
                    continue; // `.fired_x`, not the field
                }
                let next = after.trim_start();
                let mut from = at.saturating_sub(60);
                while !code.is_char_boundary(from) {
                    from -= 1;
                }
                let before = &code[from..at];
                let allowed = next.starts_with(".insert(")
                    || next.starts_with(".retain(")
                    || before.contains("reads_fired(rule) &&")
                    || (fingerprint && next.starts_with(".iter().filter(|id| once_ids.contains"));
                if !allowed {
                    let line = code[..at].lines().count();
                    reads.push(format!("{path}:{line}"));
                }
            }
        }
        assert!(reads.is_empty(), "reads of `fired`: {reads:#?}");
        let explore = &files
            .iter()
            .find(|(p, _)| p.ends_with("src/explore.rs"))
            .unwrap()
            .1;
        assert!(explore.contains(".filter(|r| crate::game::reads_fired(r))"));
    }

    #[test]
    fn two_answers_with_one_text_get_two_labels() {
        let choose = |index| ("say Yes.".to_string(), Command::Choose { index });
        let labels: Vec<String> = unique(vec![choose(0), choose(1), choose(2)])
            .into_iter()
            .map(|(l, _)| l)
            .collect();
        assert_eq!(labels, ["say Yes.", "say Yes. (2)", "say Yes. (3)"]);
    }
}
