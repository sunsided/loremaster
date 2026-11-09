//! Checks of text that the author is typing: markup, conditions, and the
//! names they can use.

use loremaster_runtime::SourcePiece;
use loremaster_runtime::authoring::Inspector;
use serde::{Deserialize, Serialize};

/// The result of a check of text that the author is typing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkupCheck {
    /// The first error, with its byte range in the text.
    pub error: Option<TextError>,
    /// The rendered text in the current state. Empty for a condition.
    pub text: String,
    /// The source in pieces, with branches that do not hold now marked as not active.
    pub pieces: Vec<SourcePiece>,
    /// For a condition: whether it holds now.
    pub holds: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextError {
    pub start: usize,
    pub end: usize,
    pub message: String,
}

/// A name that conditions and markup can use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    /// For example `oil_lamp.lit`, `time.phase`, `oil_lamp.the`.
    pub name: String,
    /// `variable`, `property`, `argument` (of a host event), `name`,
    /// `derived`, `phase` or `markup`.
    pub kind: String,
    /// What it is, in a few words.
    pub detail: String,
    /// The value now, as text, when it has one.
    pub value: Option<String>,
    /// The values to compare with, for example the values of an enum.
    pub values: Vec<String>,
}

impl MarkupCheck {
    fn failed(e: loremaster_lang::LangError) -> Self {
        Self {
            error: Some(TextError {
                start: e.span.start,
                end: e.span.end,
                message: e.message,
            }),
            ..Self::default()
        }
    }
}

/// Check fragment text that the author is typing, and render it in the current state.
pub(crate) fn try_markup(ins: Inspector<'_>, src: &str) -> MarkupCheck {
    match loremaster_lang::compile_markup(src, ins.scope()) {
        Ok(segments) => MarkupCheck {
            error: None,
            text: ins.render(&segments),
            pieces: ins.outline(&segments),
            holds: None,
        },
        Err(e) => MarkupCheck::failed(e),
    }
}

/// Check a condition that the author is typing, and tell whether it holds now.
pub(crate) fn try_condition(ins: Inspector<'_>, src: &str) -> MarkupCheck {
    if src.trim().is_empty() {
        return MarkupCheck::default();
    }
    match loremaster_lang::compile_condition(src, ins.scope()) {
        Ok(c) => MarkupCheck {
            holds: Some(ins.holds(&c)),
            ..MarkupCheck::default()
        },
        Err(e) => MarkupCheck::failed(e),
    }
}

/// Every name that conditions and markup can use, for completion and hover in the editor.
pub(crate) fn symbols(ins: Inspector<'_>) -> Vec<Symbol> {
    let scope = ins.scope();
    let story = &ins.world().story;
    let mut out = Vec::new();
    let mut add = |name: String, kind: &str, detail: String, values: Vec<String>| {
        out.push(Symbol {
            name,
            kind: kind.to_string(),
            detail,
            value: None,
            values,
        });
    };
    let rooms: Vec<String> = scope.rooms.iter().cloned().collect();
    let values = |ty: &loremaster_lang::Ty| match ty {
        loremaster_lang::Ty::Enum(v) => v.clone(),
        loremaster_lang::Ty::Bool => vec!["true".into(), "false".into()],
        _ => Vec::new(),
    };
    for (name, ty) in &scope.vars {
        add(
            name.clone(),
            "variable",
            format!("variable, {}", ty.describe()),
            values(ty),
        );
    }
    for (thing, props) in &scope.things {
        for (prop, ty) in props {
            add(
                format!("{thing}.{prop}"),
                "property",
                format!("property of {thing}, {}", ty.describe()),
                Vec::new(),
            );
        }
        for (form, what) in [
            ("name", "the current name"),
            ("the", "the name with \"the\""),
            ("The", "the name with \"The\""),
            ("a", "the name with \"a\" or \"an\""),
            ("A", "the name with \"A\" or \"An\""),
        ] {
            add(
                format!("{thing}.{form}"),
                "name",
                format!("{what} of {thing}, for text"),
                Vec::new(),
            );
        }
        for (derived, what) in [
            ("carried", "true when the player carries it"),
            ("present", "true when it is carried or in the current room"),
            ("interactions", "how many commands named it"),
            ("talked", "how many times the player talked to it"),
        ] {
            add(
                format!("{thing}.{derived}"),
                "derived",
                what.to_string(),
                Vec::new(),
            );
        }
        let mut places = rooms.clone();
        places.extend(["carried".to_string(), "nowhere".to_string()]);
        add(
            format!("{thing}.location"),
            "derived",
            "a room id, carried or nowhere".into(),
            places,
        );
    }
    for (event, args) in &scope.events {
        for (arg, ty) in args {
            add(
                format!("{event}.{arg}"),
                "argument",
                format!("last argument of the host event {event}, {}", ty.describe()),
                values(ty),
            );
        }
    }
    for room in &rooms {
        add(
            format!("{room}.visits"),
            "derived",
            "how many times the player entered it".into(),
            Vec::new(),
        );
        add(
            format!("{room}.visited"),
            "derived",
            "true after the first visit".into(),
            Vec::new(),
        );
    }
    add(
        "time.phase".into(),
        "derived",
        "the phase of the day".into(),
        scope.phases.clone(),
    );
    add(
        "time.hour".into(),
        "derived",
        "the hour, 0 to 23".into(),
        Vec::new(),
    );
    add(
        "time.day".into(),
        "derived",
        "the day, from 1".into(),
        Vec::new(),
    );
    add(
        "turns".into(),
        "derived",
        "the number of turns".into(),
        Vec::new(),
    );
    add(
        "here".into(),
        "derived",
        "the id of the current room".into(),
        rooms.clone(),
    );
    add(
        "first_visit".into(),
        "derived",
        "true on the first visit to the current room".into(),
        Vec::new(),
    );
    for (name, what) in [
        (
            "action.repeat",
            "times in a row the player gave this command",
        ),
        (
            "action.count",
            "times the player used this verb in this room",
        ),
        (
            "action.total",
            "times the player used this verb in the game",
        ),
        ("action.verb", "the id of the current verb"),
    ] {
        add(name.into(), "derived", what.into(), Vec::new());
    }
    for phase in &scope.phases {
        add(
            phase.clone(),
            "phase",
            format!("true when time.phase is {phase}"),
            Vec::new(),
        );
    }
    add(
        "first".into(),
        "markup",
        "in markup: {first: …} shows only on the first visit".into(),
        Vec::new(),
    );
    let _ = story;
    // The value now: render the name as an insert, or test it as a condition.
    for sym in &mut out {
        let insert = try_markup(ins, &format!("{{{}}}", sym.name));
        sym.value = if insert.error.is_none() {
            Some(insert.text)
        } else {
            try_condition(ins, &sym.name).holds.map(|b| b.to_string())
        };
    }
    out
}
