//! The rules view and the puzzle machines.

use crate::room_here;
use loremaster_model::{EventPattern, Id, TimeOfDay, Value};
use loremaster_runtime::authoring::Inspector;
use serde::{Deserialize, Serialize};

/// A rule as When / If / Then, with its state now, for the rules view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleView {
    pub id: String,
    /// The event, in words, for example "the player enters lantern_room".
    pub when: String,
    /// The condition as written.
    pub condition: Option<String>,
    /// True when the condition holds now (or there is none).
    pub condition_holds: bool,
    /// Each effect, in words.
    pub then: Vec<String>,
    pub once: bool,
    pub instead: bool,
    /// True when the rule fired at least one time in this game.
    pub fired: bool,
    /// The kind of event: start, turn, enter, leave, time or verb.
    pub trigger: String,
    /// Why the rule matters here and now, or None. For example "fires when the
    /// player enters this room" or "fired in the last turn".
    pub relevant: Option<String>,
}

/// A puzzle machine for the Atlas: its states, where it is now and its steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineView {
    pub id: String,
    pub state: String,
    /// The state at the start of the game.
    pub start: String,
    pub states: Vec<String>,
    /// Author text about states, by state name. States without a description are not in it.
    pub descriptions: std::collections::BTreeMap<String, String>,
    pub steps: Vec<MachineStepView>,
}

/// One step of a machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineStepView {
    /// The id of the rule that the step became.
    pub rule: String,
    pub from: Vec<String>,
    pub to: String,
    /// The event, in words.
    pub when: String,
    /// The step's own condition, as written.
    pub condition: Option<String>,
    /// The step ran at least once in this game.
    pub fired: bool,
    /// The machine is in a `from` state and the condition holds: the event takes the step now.
    pub ready: bool,
}

/// All rules with their state now. Does not change any state.
pub(crate) fn rules(ins: Inspector<'_>) -> Vec<RuleView> {
    let story = &ins.world().story;
    let thing_or_room = |id: &Id| {
        story
            .thing(id.as_str())
            .map(|t| t.name.clone())
            .or_else(|| story.room(id.as_str()).map(|r| r.name.clone()))
            .unwrap_or_else(|| id.to_string())
    };
    let verb_text = |p: &loremaster_model::CommandPattern| {
        let verb = ins
            .world()
            .verb(p.verb.as_str())
            .map_or_else(|| p.verb.to_string(), |v| v.label.to_lowercase());
        let objects: Vec<String> = p.objects().map(thing_or_room).collect();
        let prep = p.preposition.as_deref().unwrap_or("with");
        let text = if objects.is_empty() {
            verb
        } else {
            format!("{verb} {}", objects.join(&format!(" {prep} ")))
        };
        match &p.topic {
            Some(topic) => format!("{text} about {topic}"),
            None => text,
        }
    };
    // The player, or another actor of a Command record (D6).
    let who = |p: &loremaster_model::CommandPattern| match &p.actor {
        Some(a) if a.as_str() != loremaster_model::PLAYER_SPEAKER => thing_or_room(a),
        _ => "the player".to_string(),
    };
    story
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let when = match &r.on {
                EventPattern::Start => "the game starts".to_string(),
                EventPattern::Turn => "a turn passes".to_string(),
                EventPattern::Enter(room) => {
                    format!("the player enters {}", thing_or_room(room))
                }
                EventPattern::Leave(room) => {
                    format!("the player leaves {}", thing_or_room(room))
                }
                EventPattern::Time(t) => format!("the clock reaches {t}"),
                EventPattern::Verb(p) => {
                    format!("{} uses \"{}\"", who(p), verb_text(p))
                }
                EventPattern::Check(p) => {
                    format!("{} tries \"{}\"", who(p), verb_text(p))
                }
                EventPattern::Timer(t) => format!("the timer {t} runs out"),
                EventPattern::Event(e) => format!("the event {e} happens"),
                EventPattern::Talk(parts) => match parts.as_slice() {
                    [d, node, ..] => format!("the dialogue {d} reaches {node}"),
                    [d] => format!("the dialogue {d} goes on"),
                    [] => "a dialogue goes on".to_string(),
                },
            };
            let then = r
                .then
                .iter()
                .map(|e| match e {
                    loremaster_model::Effect::Set(m) => format!(
                        "set {}",
                        m.iter()
                            .map(|(k, v)| format!("{k} = {v}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    loremaster_model::Effect::Add(m) => format!(
                        "add {}",
                        m.iter()
                            .map(|(k, v)| format!("{v} to {k}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    loremaster_model::Effect::Say(t) => match t.as_plain() {
                        Some(text) => format!("say \"{text}\""),
                        // The readable form quotes the speech already.
                        None => format!("say {}", t.source()),
                    },
                    loremaster_model::Effect::Move(m) => format!(
                        "move {}",
                        m.iter()
                            .map(|(t, to)| format!("{} to {to}", thing_or_room(t)))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    loremaster_model::Effect::Goto(room) => {
                        format!("move the player to {}", thing_or_room(room))
                    }
                    loremaster_model::Effect::End(id) => {
                        match ins.world().story.ending(id.as_str()) {
                            Some(e) => {
                                format!("end the game: {} ({})", e.title, e.kind.as_str())
                            }
                            None => format!("end the game: {id}"),
                        }
                    }
                    loremaster_model::Effect::Stop => "stop the action".to_string(),
                    loremaster_model::Effect::Emit(e) => format!("send the event {e}"),
                    loremaster_model::Effect::Start(t) => format!(
                        "start {}",
                        t.entries()
                            .iter()
                            .map(|(id, n)| match n {
                                Some(n) => format!("{id} at {n}"),
                                None => id.to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    loremaster_model::Effect::Cancel(t) => format!("cancel {t}"),
                    loremaster_model::Effect::Call(b) if b.as_str().ends_with("#rust") => {
                        format!("run Rust ({b})")
                    }
                    loremaster_model::Effect::Call(b) => format!("run the behavior {b}"),
                    loremaster_model::Effect::Await(spec) => {
                        match loremaster_runtime::authoring::describe_await(spec) {
                            Some(r) => format!("wait for the host: {r}"),
                            None => "wait for the host".to_string(),
                        }
                    }
                })
                .collect();
            let trigger = match &r.on {
                EventPattern::Start => "start",
                EventPattern::Turn => "turn",
                EventPattern::Enter(_) => "enter",
                EventPattern::Leave(_) => "leave",
                EventPattern::Time(_) => "time",
                EventPattern::Verb(_) => "verb",
                EventPattern::Check(_) => "check",
                EventPattern::Timer(_) => "timer",
                EventPattern::Event(_) => "event",
                EventPattern::Talk(_) => "talk",
            };
            let relevant = relevance(ins, r);
            let when = match &r.within {
                Some(w) => format!("{when}, in {}", thing_or_room(w)),
                None => when,
            };
            RuleView {
                id: r.id.to_string(),
                trigger: trigger.to_string(),
                relevant,
                when,
                condition: r.when.clone(),
                condition_holds: ins.rule_holds(i),
                then,
                once: r.once,
                instead: r.instead,
                fired: ins.state().fired.contains(&r.id),
            }
        })
        .chain(ins.world().story.behaviors.iter().map(|id| {
            let found = ins.world().behavior(id.as_str());
            let mut when = match found.map(|b| b.description()) {
                Some("") | None if found.is_some() => "code in this game".to_string(),
                Some(d) => format!("code in this game: {d}"),
                None => "code in the game, which this build does not have".to_string(),
            };
            if let Some(m) = ins.world().manifest(id.as_str()) {
                when.push_str(&format!(" ({})", m.describe()));
            }
            RuleView {
                id: format!("behavior:{id}"),
                trigger: "event".into(),
                relevant: None,
                when,
                condition: None,
                condition_holds: found.is_some(),
                then: Vec::new(),
                once: false,
                instead: false,
                fired: false,
            }
        }))
        .collect()
}

/// The puzzle machines with their states and steps.
pub(crate) fn machines(ins: Inspector<'_>) -> Vec<MachineView> {
    let rules = rules(ins);
    ins.world()
        .story
        .machines
        .iter()
        .map(|m| MachineView {
            id: m.id.to_string(),
            state: ins
                .state()
                .facts
                .get(m.id.as_str())
                .map(Value::to_string)
                .unwrap_or_else(|| m.start.clone()),
            start: m.start.clone(),
            states: m.all_states(),
            descriptions: m
                .all_states()
                .iter()
                .filter_map(|s| Some((s.clone(), m.description(s)?.to_string())))
                .collect(),
            steps: m
                .steps
                .iter()
                .enumerate()
                .map(|(i, step)| {
                    let id = m.step_rule(i);
                    let view = rules.iter().find(|r| r.id == id.as_str());
                    MachineStepView {
                        rule: id.to_string(),
                        from: step.from.list().iter().map(|s| s.to_string()).collect(),
                        to: step.to.clone(),
                        when: view.map(|r| r.when.clone()).unwrap_or_default(),
                        condition: step.when.clone(),
                        fired: view.is_some_and(|r| r.fired),
                        ready: view.is_some_and(|r| r.condition_holds),
                    }
                })
                .collect(),
        })
        .collect()
}

/// Why a rule matters at the player's position and time now, if it does.
fn relevance(ins: Inspector<'_>, rule: &loremaster_model::Rule) -> Option<String> {
    if ins.last_fired().contains(&rule.id) {
        return Some("fired in the last turn".into());
    }
    if rule.once && ins.state().fired.contains(&rule.id) {
        return None;
    }
    let here = &ins.state().room;
    match &rule.on {
        EventPattern::Enter(r) | EventPattern::Leave(r) if r == here => {
            Some("about this room".into())
        }
        EventPattern::Enter(r) => {
            let next_door = room_here(ins)
                .is_some_and(|room| room.exits.iter().any(|e| e.to.as_ref() == Some(r)));
            next_door.then(|| "fires in a room next to this one".into())
        }
        EventPattern::Verb(p) | EventPattern::Check(p) => {
            let scope = ins.in_scope();
            let things: Vec<&Id> = p.objects().collect();
            if things.is_empty() {
                Some("works with any thing".into())
            } else if things.iter().all(|t| scope.contains(t)) {
                Some("uses things in reach".into())
            } else {
                None
            }
        }
        EventPattern::Time(t) => {
            let now = ins.state().minutes % TimeOfDay::MINUTES_PER_DAY;
            let at = u32::from(t.0);
            let ahead = (at + TimeOfDay::MINUTES_PER_DAY - now) % TimeOfDay::MINUTES_PER_DAY;
            (ahead <= 60).then(|| format!("fires in {ahead} minutes"))
        }
        EventPattern::Turn => Some("checks every turn".into()),
        EventPattern::Timer(t) => ins
            .state()
            .timers
            .get(t)
            .filter(|left| **left <= 5)
            .map(|left| format!("the timer runs out in {left} turns")),
        EventPattern::Talk(parts) => ins
            .state()
            .talk
            .as_ref()
            .filter(|t| parts.first() == Some(&t.dialogue))
            .map(|_| "about the talk now".into()),
        EventPattern::Leave(_) | EventPattern::Start | EventPattern::Event(_) => None,
    }
}
