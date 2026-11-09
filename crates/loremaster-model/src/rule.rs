use crate::{AwaitSpec, CombatSpec, CommandPattern, Id, Line, Note, StartTimers, TimeOfDay, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// When / if / then.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: Id,
    pub on: EventPattern,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    pub then: Vec<Effect>,
    /// Fire at most once per game: `once: true` for this rule, or
    /// `once: <key>` for all rules with that key together.
    #[serde(default, skip_serializing_if = "Once::is_off")]
    pub once: Once,
    /// For verb and check events: when the rule fires for the player's
    /// command, the command takes no time: the clock, the turn count, the
    /// timers and the agendas do not move. For a refusal that changes nothing.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub free: bool,
    /// For verb events: the rule's text replaces the default response.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub instead: bool,
    /// The thing, room or region that the rule belongs to. A rule in a thing or
    /// room file belongs to that thing or room. A thing rule fires for verbs on
    /// that thing, and for other events while the thing is present. A room or
    /// region rule fires while the player is there. More specific rules run first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within: Option<Id>,
    /// Order in one scope: a higher priority runs first. Equal rules run in file order.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub priority: i32,
    /// The machine that this rule is a step of. The loader sets it. A machine
    /// takes at most one step for one event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<Id>,
    /// The endings that this rule closes on purpose: after it fires, they can
    /// no longer be reached (a point of no return). The ending checks accept
    /// the rank drop when it matches this list. Needs a `reason`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closes: Vec<Id>,
    /// Why the rule closes the endings in `closes`, for the authors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Author notes. The player never sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// The `once` of a rule: `true`, `false`, or the key of a once-group. All
/// rules of a group fire at most once in total: the first that fires closes
/// the group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, expecting = "`true`, `false` or the key of a once-group")]
pub enum Once {
    Rule(bool),
    Group(Id),
}

impl Default for Once {
    fn default() -> Self {
        Once::Rule(false)
    }
}

impl Once {
    /// The rule fires at most once, alone or in a group.
    pub fn is_on(&self) -> bool {
        !self.is_off()
    }

    pub fn is_off(&self) -> bool {
        *self == Once::Rule(false)
    }

    /// The key of the once-group, if the rule is in one.
    pub fn group(&self) -> Option<&Id> {
        match self {
            Once::Group(key) => Some(key),
            Once::Rule(_) => None,
        }
    }
}

/// What a rule listens for.
///
/// In YAML, `on:` is `start`, `turn`, `next`, or a map with one event kind. Its serde
/// form is written by hand (in `command.rs`), so that `verb` and `check`
/// take the slots of the Command record next to the verb.
#[derive(Debug, Clone, PartialEq)]
pub enum EventPattern {
    /// The game begins.
    Start,
    /// Every turn, after the command is handled.
    Turn,
    /// Only in a machine step: the step runs by itself right after the
    /// machine enters its `from` state, in the same turn.
    Next,
    Enter(Id),
    Leave(Id),
    /// The clock reaches a time of day.
    Time(TimeOfDay),
    /// A verb was used. The list form `[verb, thing, other]` names the verb
    /// and its objects; the record form `{ verb: examine, direct: stove,
    /// actor: keeper }` names any slot of the command (D6). These rules run
    /// after the default action, or instead of it with `instead`.
    Verb(CommandPattern),
    /// Before a verb runs, with the same pattern as `verb`. A rule with `stop`
    /// refuses the action: the default action and the `verb` rules do not run.
    /// `[go]` and `[go, north]` check a move.
    Check(CommandPattern),
    /// A timer runs out.
    Timer(Id),
    /// An `emit` effect sends this event.
    Event(Id),
    /// A dialogue reaches a node: `[dialogue]` for any node, `[dialogue, node]` for one.
    Talk(Vec<Id>),
}

/// What a rule does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Set variables or properties: `{ bridge_collapsed: true, lamp.lit: false }`.
    Set(BTreeMap<String, Value>),
    /// Add to integer variables.
    Add(BTreeMap<String, i64>),
    /// Set integer variables or properties to a sum of whole numbers and
    /// integer variables or properties: `{ total: "r + skill - 1" }`. Each
    /// sum reads the values from before the effect.
    Calc(BTreeMap<String, String>),
    /// Show a line: a text in inline markup, or a list of narration and
    /// speech segments. `line:` is the same key.
    #[serde(alias = "line")]
    Say(Line),
    /// Move things: `{ lamp: cellar }`, `carried` or `nowhere` are special targets.
    Move(BTreeMap<Id, Id>),
    /// Move the player.
    Goto(Id),
    /// End the game with an ending from `endings/`, by its id.
    End(Id),
    /// Stop the action: no more rules in this phase, and no later phases.
    /// In a `check` rule, this refuses the action.
    Stop,
    /// Send a custom event. Rules `on: { event: name }` fire at once.
    Emit(Id),
    /// Run a behavior here, at this place in the effects. It gets the event
    /// `Event::Call` with the id of the owner: the rule's id, or
    /// `dialogue:<id>` / `timer:<id>` in a dialogue or a timer.
    Call(Id),
    /// Stop the rule, ask the host, and go on with the answer (SE-5, SE-6).
    Await(Box<AwaitSpec>),
    /// A fight by the built-in rules (P5-03): the rule waits for the player
    /// round by round, then goes on in the `win`, `loss` or `escape` room.
    Combat(Box<CombatSpec>),
    /// In a rule on a combat event: end the combat and move the player to
    /// the room. The effects after `combat` go on there.
    EndCombat(Id),
    /// Start timers, or start them again from the beginning.
    Start(StartTimers),
    /// Stop a timer. Its rules do not fire.
    Cancel(Id),
    /// Ask the host to play a sound asset once. It is not saved.
    PlaySound(Id),
}

/// A term of a `calc` sum: a whole number, or the name of an integer
/// variable or property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Int(i64),
    Name(String),
}

/// A term of a sum with its sign, `1` or `-1`.
pub type SumTerm = (i64, Term);

/// What is wrong with a sum, and the byte range of the bad part.
pub type SumError = (String, std::ops::Range<usize>);

/// Read a `calc` sum such as `r + skill - 2`: terms with `+` or `-` between
/// them. Each term has its sign, `1` or `-1`. An error has the message and
/// the byte range of the bad part.
pub fn parse_sum(expr: &str) -> Result<Vec<SumTerm>, SumError> {
    let mut out = Vec::new();
    let (mut sign, mut from) = (1, 0);
    if let Some(rest) = expr.strip_prefix('-') {
        (sign, from) = (-1, expr.len() - rest.len());
    }
    loop {
        let end = expr[from..]
            .find(['+', '-'])
            .map_or(expr.len(), |i| from + i);
        let raw = &expr[from..end];
        let term = raw.trim();
        let start = from + (raw.len() - raw.trim_start().len());
        let range = start..start + term.len();
        let is_name = term.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && term
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
        let term = if let Ok(n) = term.parse::<i64>() {
            Term::Int(n)
        } else if is_name {
            Term::Name(term.to_string())
        } else if term.is_empty() {
            return Err(("a number or a name expected".into(), from..end));
        } else {
            return Err((format!("`{term}` is not a number or a name"), range));
        };
        out.push((sign, term));
        if end == expr.len() {
            return Ok(out);
        }
        sign = if expr[end..].starts_with('-') { -1 } else { 1 };
        from = end + 1;
    }
}

fn is_zero(n: &i32) -> bool {
    *n == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once_reads_a_bool_or_a_group_key() {
        let once = |v| serde_json::from_value::<Once>(v).unwrap();
        assert_eq!(once(serde_json::json!(true)), Once::Rule(true));
        assert_eq!(once(serde_json::json!(false)), Once::Rule(false));
        assert_eq!(
            once(serde_json::json!("greet")),
            Once::Group("greet".into())
        );
        assert_eq!(
            serde_json::to_value(Once::Rule(true)).unwrap(),
            serde_json::json!(true)
        );
        assert_eq!(
            serde_json::to_value(Once::Group("greet".into())).unwrap(),
            serde_json::json!("greet")
        );
        assert!(serde_json::from_value::<Once>(serde_json::json!(3)).is_err());
    }

    #[test]
    fn a_sum_reads_numbers_and_names_with_signs() {
        assert_eq!(
            parse_sum("r + skill - 2"),
            Ok(vec![
                (1, Term::Name("r".into())),
                (1, Term::Name("skill".into())),
                (-1, Term::Int(2)),
            ])
        );
        assert_eq!(
            parse_sum("lamp.oil"),
            Ok(vec![(1, Term::Name("lamp.oil".into()))])
        );
        assert_eq!(parse_sum("-3"), Ok(vec![(-1, Term::Int(3))]));
        assert_eq!(parse_sum("r + 2x").unwrap_err().1, 4..6);
        assert!(parse_sum("r +").is_err());
        assert!(parse_sum("").is_err());
    }
}
