//! The Command record (decision D6): the slots of a command, and the event
//! patterns of rules that match on them.
//!
//! A command has an actor, a verb, a direct and an indirect object, the
//! preposition between them and a topic. The player is the actor of typed
//! commands; an agenda runs commands for its actor. A rule matches a verb
//! with the list form `verb: [take, lamp]` (verb, direct, indirect), or with
//! the record form `{ verb: examine, direct: stove, actor: keeper }`, which
//! can name any slot.

use crate::{EventPattern, Id, TimeOfDay};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The slots of a command that a rule matches, or that an agenda step runs
/// (`do:`). A slot that is `None` matches anything, except `actor`: a
/// pattern without an actor matches the player's commands only, so the rules
/// that a story has for the player do not fire for other actors.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandPattern {
    pub verb: Id,
    /// Who acts: `player`, or the id of an actor thing.
    pub actor: Option<Id>,
    /// The first object: `take lamp`, `unlock door with key`. For `go`, the
    /// direction.
    pub direct: Option<Id>,
    /// The second object: the key in `unlock door with key`.
    pub indirect: Option<Id>,
    /// The word between the objects, such as `with`, `on` or `in`.
    pub preposition: Option<String>,
    /// What a command is about, for `ask keeper about stove`. Free text; the
    /// parser does not fill it yet.
    pub topic: Option<String>,
}

/// The verb of a rule pattern that matches any verb: `on: { verb: ["*", darkness] }`.
/// An exact-verb rule that fires in the same scope and phase holds it back.
pub const ANY_VERB: &str = "*";

/// The names of the slots, in the order of the record.
const SLOTS: &[&str] = &[
    "verb",
    "actor",
    "direct",
    "indirect",
    "preposition",
    "topic",
];

impl CommandPattern {
    /// Does the pattern match any verb (`*`)?
    pub fn any_verb(&self) -> bool {
        self.verb.as_str() == ANY_VERB
    }

    /// A pattern that names only a verb.
    pub fn new(verb: impl Into<Id>) -> Self {
        Self {
            verb: verb.into(),
            actor: None,
            direct: None,
            indirect: None,
            preposition: None,
            topic: None,
        }
    }

    /// The list form `[verb, direct, indirect]`. An empty list gives an empty
    /// verb, which the checks report. A command has no slot for a fourth
    /// part, so parts after the third are dropped; YAML rejects them when it
    /// reads the list, and a builder that takes more should report it.
    pub fn from_parts(parts: impl IntoIterator<Item = Id>) -> Self {
        let mut parts = parts.into_iter();
        let mut p = Self::new(parts.next().unwrap_or_else(|| Id::new("")));
        p.direct = parts.next();
        p.indirect = parts.next();
        p
    }

    pub fn with_actor(mut self, actor: impl Into<Id>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    pub fn with_direct(mut self, direct: impl Into<Id>) -> Self {
        self.direct = Some(direct.into());
        self
    }

    pub fn with_indirect(mut self, indirect: impl Into<Id>) -> Self {
        self.indirect = Some(indirect.into());
        self
    }

    /// The list form `[verb, direct, indirect]`, when the pattern has one:
    /// no actor, preposition or topic, and no indirect object without a
    /// direct one.
    pub fn parts(&self) -> Option<Vec<Id>> {
        if self.actor.is_some()
            || self.preposition.is_some()
            || self.topic.is_some()
            || (self.indirect.is_some() && self.direct.is_none())
            // `[]` is the empty verb; with an object, the list would re-read
            // the object as the verb.
            || (self.verb.as_str().is_empty() && self.direct.is_some())
        {
            return None;
        }
        Some(
            std::iter::once(self.verb.clone())
                .filter(|v| !v.as_str().is_empty())
                .chain(self.direct.clone())
                .chain(self.indirect.clone())
                .collect(),
        )
    }

    /// The objects that the pattern names: the direct one, then the indirect one.
    pub fn objects(&self) -> impl Iterator<Item = &Id> {
        self.direct.iter().chain(self.indirect.iter())
    }

    /// Write the slots that are set into a map, after the entries before.
    fn write_slots<M: SerializeMap>(&self, map: &mut M, verb_key: &str) -> Result<(), M::Error> {
        map.serialize_entry(verb_key, &self.verb)?;
        if let Some(a) = &self.actor {
            map.serialize_entry("actor", a)?;
        }
        if let Some(d) = &self.direct {
            map.serialize_entry("direct", d)?;
        }
        if let Some(i) = &self.indirect {
            map.serialize_entry("indirect", i)?;
        }
        if let Some(p) = &self.preposition {
            map.serialize_entry("preposition", p)?;
        }
        if let Some(t) = &self.topic {
            map.serialize_entry("topic", t)?;
        }
        Ok(())
    }
}

impl From<&str> for CommandPattern {
    fn from(verb: &str) -> Self {
        Self::new(verb)
    }
}

/// The slots of a record form that `verb` (or `check`) does not give.
#[derive(Default)]
struct Slots {
    actor: Option<Id>,
    direct: Option<Id>,
    indirect: Option<Id>,
    preposition: Option<String>,
    topic: Option<String>,
}

impl Slots {
    fn any(&self) -> Option<&'static str> {
        [
            ("actor", self.actor.is_some()),
            ("direct", self.direct.is_some()),
            ("indirect", self.indirect.is_some()),
            ("preposition", self.preposition.is_some()),
            ("topic", self.topic.is_some()),
        ]
        .into_iter()
        .find(|(_, set)| *set)
        .map(|(k, _)| k)
    }

    /// Read one slot. False when the key is not a slot.
    fn read<'de, A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<bool, A::Error> {
        fn once<E: de::Error, T>(slot: &mut Option<T>, key: &str, value: T) -> Result<(), E> {
            if slot.replace(value).is_some() {
                return Err(E::custom(format!("`{key}` is given twice")));
            }
            Ok(())
        }
        match key {
            "actor" => once(&mut self.actor, key, map.next_value()?)?,
            "direct" => once(&mut self.direct, key, map.next_value()?)?,
            "indirect" => once(&mut self.indirect, key, map.next_value()?)?,
            "preposition" => once(&mut self.preposition, key, map.next_value()?)?,
            "topic" => once(&mut self.topic, key, map.next_value()?)?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// The pattern of a verb (one id, or the list form) with these slots.
    fn pattern<E: de::Error>(self, verb: OneOrList, key: &str) -> Result<CommandPattern, E> {
        let mut p = match verb {
            OneOrList::One(v) => CommandPattern::new(v),
            OneOrList::List(list) => {
                // `[]` names no verb: it loads, and the checks report it.
                if list.len() > 3 {
                    return Err(E::custom(format!(
                        "`{key}` names at most the verb and two things: [verb, direct, indirect]; for more slots, write `{{ {key}: <verb>, direct: …, indirect: … }}`"
                    )));
                }
                CommandPattern::from_parts(list)
            }
        };
        for (name, slot, value) in [
            ("direct", &mut p.direct, self.direct),
            ("indirect", &mut p.indirect, self.indirect),
        ] {
            if let Some(v) = value {
                if slot.is_some() {
                    return Err(E::custom(format!(
                        "`{name}` is given twice: in the list of `{key}` and as a field"
                    )));
                }
                *slot = Some(v);
            }
        }
        p.actor = self.actor;
        p.preposition = self.preposition;
        p.topic = self.topic;
        Ok(p)
    }
}

/// A verb as one id (`verb: examine`) or as the list form (`verb: [take, lamp]`).
enum OneOrList {
    One(Id),
    List(Vec<Id>),
}

impl<'de> Deserialize<'de> for OneOrList {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = OneOrList;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a verb id, or a list [verb, direct, indirect]")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<OneOrList, E> {
                Ok(OneOrList::One(Id::new(v)))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<OneOrList, A::Error> {
                let mut list = Vec::new();
                while let Some(id) = seq.next_element::<Id>()? {
                    list.push(id);
                }
                Ok(OneOrList::List(list))
            }
        }
        deserializer.deserialize_any(V)
    }
}

/// `do:` of an agenda step: `{ verb: open, direct: stove }`, or the list form
/// `[open, stove]`.
impl Serialize for CommandPattern {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        self.write_slots(&mut map, "verb")?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for CommandPattern {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = CommandPattern;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "a command: { verb: …, direct: …, indirect: … } or [verb, direct, indirect]",
                )
            }
            fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<CommandPattern, A::Error> {
                let list = OneOrList::deserialize(de::value::SeqAccessDeserializer::new(seq))?;
                Slots::default().pattern(list, "do")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<CommandPattern, A::Error> {
                let mut verb = None;
                let mut slots = Slots::default();
                while let Some(key) = map.next_key::<String>()? {
                    if key == "verb" {
                        if verb.replace(map.next_value::<Id>()?).is_some() {
                            return Err(de::Error::duplicate_field("verb"));
                        }
                    } else if !slots.read(&key, &mut map)? {
                        return Err(de::Error::unknown_field(&key, SLOTS));
                    }
                }
                let verb = verb.ok_or_else(|| de::Error::missing_field("verb"))?;
                slots.pattern(OneOrList::One(verb), "verb")
            }
        }
        deserializer.deserialize_any(V)
    }
}

/// The keys of `on:`: the event kinds, then the slots of the record form.
const EVENT_KEYS: &[&str] = &[
    "start",
    "turn",
    "next",
    "enter",
    "leave",
    "time",
    "verb",
    "check",
    "timer",
    "event",
    "talk",
    "actor",
    "direct",
    "indirect",
    "preposition",
    "topic",
];

/// `on:` is a word (`start`, `turn`, `next`) or a map with one event kind, as before
/// D6. A `verb` or `check` takes the list form, or a verb id with more slots
/// of the Command record next to it: `{ verb: examine, direct: stove }`.
impl Serialize for EventPattern {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let one = |key: &str, serializer: S| -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(Some(1))?;
            match self {
                EventPattern::Enter(id)
                | EventPattern::Leave(id)
                | EventPattern::Timer(id)
                | EventPattern::Event(id) => map.serialize_entry(key, id)?,
                EventPattern::Time(t) => map.serialize_entry(key, t)?,
                EventPattern::Talk(parts) => map.serialize_entry(key, parts)?,
                _ => unreachable!("one-key events only"),
            }
            map.end()
        };
        match self {
            EventPattern::Start => serializer.serialize_str("start"),
            EventPattern::Turn => serializer.serialize_str("turn"),
            EventPattern::Next => serializer.serialize_str("next"),
            EventPattern::Enter(_) => one("enter", serializer),
            EventPattern::Leave(_) => one("leave", serializer),
            EventPattern::Time(_) => one("time", serializer),
            EventPattern::Timer(_) => one("timer", serializer),
            EventPattern::Event(_) => one("event", serializer),
            EventPattern::Talk(_) => one("talk", serializer),
            EventPattern::Verb(p) | EventPattern::Check(p) => {
                let key = if matches!(self, EventPattern::Verb(_)) {
                    "verb"
                } else {
                    "check"
                };
                // The list form, when it says it all: the model (and its
                // hash) stays as it was before the record form.
                if let Some(parts) = p.parts() {
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry(key, &parts)?;
                    return map.end();
                }
                let mut map = serializer.serialize_map(None)?;
                p.write_slots(&mut map, key)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for EventPattern {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = EventPattern;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "an event: `start`, `turn`, `next`, or a map such as `{ verb: [take, lamp] }`",
                )
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<EventPattern, E> {
                match v {
                    "start" => Ok(EventPattern::Start),
                    "turn" => Ok(EventPattern::Turn),
                    "next" => Ok(EventPattern::Next),
                    other => Err(E::unknown_variant(other, &["start", "turn", "next"])),
                }
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<EventPattern, A::Error> {
                // The event kind, by key; `verb` and `check` keep their verb
                // until the slots are read.
                enum Head {
                    Done(EventPattern),
                    Verb(OneOrList),
                    Check(OneOrList),
                }
                let mut head: Option<(String, Head)> = None;
                let mut slots = Slots::default();
                while let Some(key) = map.next_key::<String>()? {
                    if slots.read(&key, &mut map)? {
                        continue;
                    }
                    let next = match key.as_str() {
                        "start" | "turn" | "next" => {
                            map.next_value::<de::IgnoredAny>()?;
                            Head::Done(match key.as_str() {
                                "start" => EventPattern::Start,
                                "turn" => EventPattern::Turn,
                                _ => EventPattern::Next,
                            })
                        }
                        "enter" => Head::Done(EventPattern::Enter(map.next_value()?)),
                        "leave" => Head::Done(EventPattern::Leave(map.next_value()?)),
                        "time" => Head::Done(EventPattern::Time(map.next_value::<TimeOfDay>()?)),
                        "timer" => Head::Done(EventPattern::Timer(map.next_value()?)),
                        "event" => Head::Done(EventPattern::Event(map.next_value()?)),
                        "talk" => Head::Done(EventPattern::Talk(map.next_value()?)),
                        "verb" => Head::Verb(map.next_value()?),
                        "check" => Head::Check(map.next_value()?),
                        other => return Err(de::Error::unknown_field(other, EVENT_KEYS)),
                    };
                    if let Some((first, _)) = &head {
                        return Err(de::Error::custom(format!(
                            "an event has one kind, but `{first}` and `{key}` are both given"
                        )));
                    }
                    head = Some((key, next));
                }
                let Some((key, head)) = head else {
                    return Err(de::Error::custom(
                        "the event names no kind: give one of `start`, `turn`, `next`, `enter`, `leave`, `time`, `verb`, `check`, `timer`, `event` or `talk`",
                    ));
                };
                match head {
                    Head::Verb(v) => Ok(EventPattern::Verb(slots.pattern(v, "verb")?)),
                    Head::Check(v) => Ok(EventPattern::Check(slots.pattern(v, "check")?)),
                    Head::Done(event) => match slots.any() {
                        Some(slot) => Err(de::Error::custom(format!(
                            "`{slot}` is a slot of a command: it works with `verb` and `check`, not with `{key}`"
                        ))),
                        None => Ok(event),
                    },
                }
            }
        }
        deserializer.deserialize_any(V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    fn read(text: &str) -> Result<EventPattern, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    #[test]
    fn the_list_form_stays() {
        let p = read(r#"{ "verb": ["take", "lamp"] }"#).unwrap();
        check!(p == EventPattern::Verb(CommandPattern::new("take").with_direct("lamp")));
        check!(serde_json::to_string(&p).unwrap() == r#"{"verb":["take","lamp"]}"#);
        check!(serde_json::to_string(&EventPattern::Start).unwrap() == r#""start""#);
        check!(read(r#""turn""#).unwrap() == EventPattern::Turn);
        check!(read(r#""next""#).unwrap() == EventPattern::Next);
        check!(serde_json::to_string(&EventPattern::Next).unwrap() == r#""next""#);
    }

    #[test]
    fn the_record_form_names_slots() {
        let p = read(r#"{ "verb": "examine", "direct": "stove" }"#).unwrap();
        // Without other slots, the record is the list form.
        check!(p == EventPattern::Verb(CommandPattern::new("examine").with_direct("stove")));
        check!(serde_json::to_string(&p).unwrap() == r#"{"verb":["examine","stove"]}"#);

        let p = read(r#"{ "check": "open", "direct": "stove", "actor": "keeper" }"#).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        check!(json == r#"{"check":"open","actor":"keeper","direct":"stove"}"#);
        check!(serde_json::from_str::<EventPattern>(&json).unwrap() == p);
    }

    #[test]
    fn mistakes_have_messages() {
        let e = read(r#"{ "verb": "examine", "direkt": "stove" }"#).unwrap_err();
        check!(e.contains("unknown field `direkt`"), "{e}");
        let e = read(r#"{ "enter": "kitchen", "direct": "stove" }"#).unwrap_err();
        check!(e.contains("slot of a command"), "{e}");
        let e = read(r#"{ "verb": ["take", "lamp"], "direct": "lamp" }"#).unwrap_err();
        check!(e.contains("given twice"), "{e}");
        let e = read(r#"{ "verb": "take", "enter": "kitchen" }"#).unwrap_err();
        check!(e.contains("one kind"), "{e}");
        let e = read(r#"{ "actor": "keeper" }"#).unwrap_err();
        check!(e.contains("names no kind"), "{e}");
    }

    #[test]
    fn an_empty_verb_with_an_object_keeps_the_record_form() {
        let p = EventPattern::Verb(CommandPattern::new("").with_direct("stove"));
        let json = serde_json::to_string(&p).unwrap();
        check!(json == r#"{"verb":"","direct":"stove"}"#);
        check!(serde_json::from_str::<EventPattern>(&json).unwrap() == p);
    }

    #[test]
    fn a_command_reads_both_forms() {
        let c: CommandPattern =
            serde_json::from_str(r#"{ "verb": "open", "direct": "stove" }"#).unwrap();
        check!(c == CommandPattern::new("open").with_direct("stove"));
        let c: CommandPattern = serde_json::from_str(r#"["open", "stove"]"#).unwrap();
        check!(c == CommandPattern::new("open").with_direct("stove"));
    }
}
