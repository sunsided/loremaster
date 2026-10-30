//! The state explorer: a breadth-first walk over the states a player can
//! reach, with a budget. It answers "can this ever show?" and "can the player
//! get stuck?" for the whole story, not only for the states the author tried.

use crate::{BlockKind, Command, Game, Turn};
use loremaster_model::{EventPattern, Id, Location, Value, VerbSlots};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

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
    /// States from which no explored path reaches an ending. Empty when the story has no ending.
    pub stuck: usize,
    /// The shortest path into a stuck state, if there is one.
    pub stuck_path: Vec<String>,
    /// For each open or resolved thread with a payoff: whether a payoff showed, and in which order.
    #[serde(default)]
    pub threads: Vec<crate::ThreadReach>,
}

/// A state as the explorer compares it: the parts that change what can happen.
/// The clock counts only by phase. Counters count only up to 3, and only when
/// the story reads them.
fn fingerprint(game: &Game, counters: bool) -> String {
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
    serde_json::to_string(&(
        &s.room,
        &s.facts,
        &s.locations,
        &s.fired,
        game.scene().phase,
        s.ended,
        visited,
        cap(&s.talked),
        cap(&s.interactions),
        &s.talk,
        &s.relations,
        &s.memories,
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
/// story without time rules) are left out, so the walk stays small.
fn commands(game: &Game) -> Vec<(String, Command)> {
    let world = game.world();
    let story = &world.story;
    let scope = game.in_scope();
    let mut out: Vec<(String, Command)> = Vec::new();
    // In a talk, only the answers and leaving run.
    if game.talk().is_some() {
        out.extend(game.choices().into_iter().map(|c| {
            (
                format!("say {}", c.text),
                Command::Choose { index: c.index },
            )
        }));
        out.push(("leave".into(), Command::Leave));
        return out;
    }
    let verb = |v: &str, objects: Vec<Id>| Command::Verb {
        verb: Id::new(v),
        objects,
    };
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
    for rule in &story.rules {
        if let EventPattern::Verb(parts) = &rule.on
            && let Some((v, objects)) = parts.split_first()
            && objects.iter().all(|o| scope.contains(o))
        {
            out.push((
                label(v.as_str(), objects),
                verb(v.as_str(), objects.to_vec()),
            ));
        }
    }
    // The standard verbs that change state.
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
        if t.portable && !carried {
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
        if let Some(key) = &t.key
            && scope.contains(key)
        {
            let objects = vec![id.clone(), key.clone()];
            out.push((label("unlock", &objects), verb("unlock", objects)));
        }
    }
    // Waiting, when the story reacts to time.
    if story
        .rules
        .iter()
        .any(|r| matches!(r.on, EventPattern::Time(_) | EventPattern::Turn))
    {
        out.push(("wait".into(), verb("wait", Vec::new())));
    }
    // Custom verbs without things.
    for v in story.verbs.iter().filter(|v| v.slots == VerbSlots::None) {
        out.push((v.id.to_string(), verb(v.id.as_str(), Vec::new())));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.dedup_by(|a, b| a.0 == b.0);
    out
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
    let opening = game.start();
    let counters = reads_counters(&game);

    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut nodes: Vec<(Game, Vec<String>)> = Vec::new();
    let mut edges: Vec<Vec<usize>> = Vec::new();
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

    seen.insert(fingerprint(&game, counters), 0);
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
    queue.push_back(0);

    while let Some(i) = queue.pop_front() {
        let (game, path) = nodes[i].clone();
        if game.state().ended || path.len() >= budget.max_depth {
            continue;
        }
        for (label, cmd) in commands(&game) {
            let mut next = game.clone();
            let turn = next.act(cmd);
            sources(&turn, &mut shown);
            for entry in turn.log.iter().filter(|l| l.kind == "rule") {
                fired.insert(entry.text.clone());
            }
            let key = fingerprint(&next, counters);
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
                    queue.push_back(j);
                    j
                }
            };
            if !edges[i].contains(&j) {
                edges[i].push(j);
            }
        }
    }

    // Stuck states: no path to an ending, when the story has endings at all.
    let has_endings = start.world().story.rules.iter().any(|r| {
        r.then
            .iter()
            .any(|e| matches!(e, loremaster_model::Effect::End(_)))
    });
    let (stuck, stuck_path) = if has_endings && !truncated {
        let mut good: Vec<bool> = nodes.iter().map(|(g, _)| g.state().ended).collect();
        loop {
            let mut changed = false;
            for i in 0..nodes.len() {
                if !good[i] && edges[i].iter().any(|&j| good[j]) {
                    good[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let bad: Vec<usize> = (0..nodes.len()).filter(|&i| !good[i]).collect();
        let path = bad
            .iter()
            .map(|&i| &nodes[i].1)
            .min_by_key(|p| p.len())
            .cloned()
            .unwrap_or_default();
        (bad.len(), path)
    } else {
        (0, Vec::new())
    };

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
        threads: thread_ids
            .iter()
            .enumerate()
            .filter(|(t, _)| beats.iter().any(|b| b.thread == *t && b.payoff))
            .map(|(t, id)| crate::ThreadReach {
                thread: id.clone(),
                paid_off: paid_off[t],
                payoff_before_setup: too_early[t].clone().unwrap_or_default(),
                payoffs: crate::threads::payoff_labels(&beats, t),
            })
            .collect(),
    }
}
