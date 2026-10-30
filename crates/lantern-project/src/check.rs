use crate::{Diagnostic, Project, Severity};
use lantern_lang::{LangError, Scope, compile_condition, compile_markup};
use lantern_model::{
    Effect, EventPattern, Fragment, Location, Story, ThreadStatus, standard_verbs,
};
use std::collections::BTreeSet;

/// Check a loaded project: every reference resolves, every condition and
/// every piece of markup parses and type-checks.
pub fn check(project: &Project) -> Vec<Diagnostic> {
    let mut c = Checker {
        project,
        scope: Scope::from_story(&project.story),
        out: Vec::new(),
    };
    c.run();
    c.out
}

struct Checker<'a> {
    project: &'a Project,
    scope: Scope,
    out: Vec<Diagnostic>,
}

impl Checker<'_> {
    fn story(&self) -> &Story {
        &self.project.story
    }

    fn file_of(&self, key: &str) -> String {
        self.project
            .origins
            .get(key)
            .cloned()
            .unwrap_or_else(|| "story.yaml".into())
    }

    fn error(&mut self, file: &str, at: &str, message: impl Into<String>) {
        self.out.push(Diagnostic {
            severity: Severity::Error,
            file: file.to_string(),
            at: at.to_string(),
            message: message.into(),
            line: None,
            column: None,
        });
    }

    fn warning(&mut self, file: &str, at: &str, message: impl Into<String>) {
        self.out.push(Diagnostic {
            severity: Severity::Warning,
            file: file.to_string(),
            at: at.to_string(),
            message: message.into(),
            line: None,
            column: None,
        });
    }

    /// Report a language error, locating the source text inside the file.
    fn lang_error(&mut self, file: &str, at: &str, source: &str, err: LangError) {
        let (line, column) = self
            .project
            .file(file)
            .and_then(|f| {
                f.text
                    .find(source)
                    .map(|pos| line_col(&f.text, pos + err.span.start))
            })
            .map_or((None, None), |(l, c)| (Some(l), Some(c)));
        self.out.push(Diagnostic {
            severity: Severity::Error,
            file: file.to_string(),
            at: at.to_string(),
            message: err.message,
            line,
            column,
        });
    }

    fn condition(&mut self, file: &str, at: &str, source: &str) {
        if let Err(e) = compile_condition(source, &self.scope) {
            self.lang_error(file, at, source, e);
        }
    }

    fn markup(&mut self, file: &str, at: &str, source: &str) {
        if let Err(e) = compile_markup(source, &self.scope) {
            self.lang_error(file, at, source, e);
        }
    }

    fn fragments(&mut self, file: &str, at: &str, fragments: &[Fragment]) {
        for (i, f) in fragments.iter().enumerate() {
            if let Some(when) = &f.when {
                self.condition(file, &format!("{at}[{i}].when"), when);
            }
            for v in &f.variants {
                self.markup(file, &format!("{at}[{i}]"), v);
            }
            if f.variants.is_empty() {
                self.error(file, &format!("{at}[{i}]"), "fragment has no text");
            }
        }
    }

    fn frames(&mut self, file: &str, at: &str, frames: &lantern_model::Frames) {
        let verbs: BTreeSet<String> = standard_verbs()
            .iter()
            .chain(self.story().verbs.iter())
            .map(|v| v.id.to_string())
            .collect();
        for (verb, frame) in frames {
            let at = format!("{at}.{verb}");
            if !verbs.contains(verb.as_str()) {
                self.error(file, &at, format!("unknown verb `{verb}`"));
            }
            self.fragments(file, &format!("{at}.before"), &frame.before);
            self.fragments(file, &format!("{at}.instead"), &frame.instead);
            self.fragments(file, &format!("{at}.after"), &frame.after);
        }
    }

    fn room_ref(&mut self, file: &str, at: &str, id: &str) {
        if self.story().room(id).is_none() {
            self.error(file, at, format!("unknown room `{id}`"));
        }
    }

    fn thing_ref(&mut self, file: &str, at: &str, id: &str) {
        if self.story().thing(id).is_none() {
            self.error(file, at, format!("unknown thing `{id}`"));
        }
    }

    fn run(&mut self) {
        let story = self.story().clone();
        self.room_ref("story.yaml", "start", story.start.as_str());
        self.fragments("story.yaml", "intro", &story.intro);
        self.frames("story.yaml", "frames", &story.frames);
        for v in &story.vars {
            let ok = match (&v.ty, &v.initial) {
                (lantern_model::VarType::Bool, lantern_model::Value::Bool(_)) => true,
                (lantern_model::VarType::Int, lantern_model::Value::Int(_)) => true,
                (lantern_model::VarType::Enum(vals), lantern_model::Value::Text(t)) => {
                    vals.contains(t)
                }
                _ => false,
            };
            if !ok {
                self.error(
                    "story.yaml",
                    &format!("vars.{}", v.id),
                    "initial value does not match the type",
                );
            }
        }

        let mut seen = BTreeSet::new();
        for room in &story.rooms {
            let file = self.file_of(&format!("room:{}", room.id));
            if !seen.insert(format!("room:{}", room.id)) {
                self.error(&file, "id", format!("room `{}` is defined twice", room.id));
            }
            self.fragments(&file, "description", &room.description);
            self.frames(&file, "frames", &room.frames);
            for (i, exit) in room.exits.iter().enumerate() {
                let at = format!("exits[{i}]");
                match &exit.to {
                    Some(to) => self.room_ref(&file, &format!("{at}.to"), to.as_str()),
                    None if exit.blocked.is_none() => self.error(
                        &file,
                        &at,
                        "the exit needs `to` (the room it leads to) or `blocked` (the text of a message exit)",
                    ),
                    None => {
                        for (field, set) in [
                            ("door", exit.door.is_some()),
                            ("passable", exit.passable.is_some()),
                            ("traversal", exit.traversal.is_some()),
                        ] {
                            if set {
                                self.warning(
                                    &file,
                                    &format!("{at}.{field}"),
                                    format!("a message exit (no `to`) does not use `{field}`"),
                                );
                            }
                        }
                    }
                }
                if let Some(door) = &exit.door {
                    self.thing_ref(&file, &format!("{at}.door"), door.as_str());
                }
                for (field, src) in [("visible", &exit.visible), ("passable", &exit.passable)] {
                    if let Some(src) = src {
                        self.condition(&file, &format!("{at}.{field}"), src);
                    }
                }
                for (field, src) in [("blocked", &exit.blocked), ("traversal", &exit.traversal)] {
                    if let Some(src) = src {
                        self.markup(&file, &format!("{at}.{field}"), src);
                    }
                }
            }
        }

        for thing in &story.things {
            let file = self.file_of(&format!("thing:{}", thing.id));
            if !seen.insert(format!("thing:{}", thing.id)) {
                self.error(
                    &file,
                    "id",
                    format!("thing `{}` is defined twice", thing.id),
                );
            }
            if let Location::Room(r) = &thing.location {
                self.room_ref(&file, "location", r.as_str());
            }
            if let Some(key) = &thing.key {
                self.thing_ref(&file, "key", key.as_str());
            }
            self.fragments(&file, "description", &thing.description);
            self.fragments(&file, "presence", &thing.presence);
            self.frames(&file, "frames", &thing.frames);
            for (i, stage) in thing.names.iter().enumerate() {
                if let Some(when) = &stage.when {
                    self.condition(&file, &format!("names[{i}].when"), when);
                }
            }
        }

        self.threads();

        let verbs: BTreeSet<String> = standard_verbs()
            .iter()
            .chain(story.verbs.iter())
            .map(|v| v.id.to_string())
            .collect();
        for rule in &story.rules {
            let file = self.file_of(&format!("rule:{}", rule.id));
            match &rule.on {
                EventPattern::Enter(r) | EventPattern::Leave(r) => {
                    self.room_ref(&file, "on", r.as_str())
                }
                EventPattern::Verb(parts) => match parts.split_first() {
                    Some((verb, objects)) => {
                        if !verbs.contains(verb.as_str()) {
                            self.error(&file, "on.verb", format!("unknown verb `{verb}`"));
                        }
                        for o in objects {
                            if self.story().thing(o.as_str()).is_none()
                                && self.story().room(o.as_str()).is_none()
                            {
                                self.error(&file, "on.verb", format!("unknown thing `{o}`"));
                            }
                        }
                    }
                    None => self.error(&file, "on.verb", "name the verb"),
                },
                EventPattern::Start | EventPattern::Turn | EventPattern::Time(_) => {}
            }
            if let Some(when) = &rule.when {
                self.condition(&file, "when", when);
            }
            for (i, effect) in rule.then.iter().enumerate() {
                let at = format!("then[{i}]");
                match effect {
                    Effect::Say(text) | Effect::End(text) => self.markup(&file, &at, text),
                    Effect::Goto(room) => self.room_ref(&file, &at, room.as_str()),
                    Effect::Move(moves) => {
                        for (thing, to) in moves {
                            self.thing_ref(&file, &at, thing.as_str());
                            if to.as_str() != "carried" && to.as_str() != "nowhere" {
                                self.room_ref(&file, &at, to.as_str());
                            }
                        }
                    }
                    Effect::Set(values) => {
                        for (name, value) in values {
                            if !self.settable(name, value) {
                                self.error(&file, &at, format!("cannot set `{name}` to `{value}`"));
                            }
                        }
                    }
                    Effect::Add(values) => {
                        for name in values.keys() {
                            if !matches!(self.ty_of(name), Some(lantern_lang::Ty::Int)) {
                                self.error(
                                    &file,
                                    &at,
                                    format!("`{name}` is not a number variable"),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Notes name known threads; an open thread that is set up gets paid off.
    fn threads(&mut self) {
        let mut seen = BTreeSet::new();
        let ids: Vec<String> = self
            .story()
            .threads
            .iter()
            .map(|t| t.id.to_string())
            .collect();
        for id in ids {
            if !seen.insert(id.clone()) {
                let file = self.file_of(&format!("thread:{id}"));
                self.error(&file, "id", format!("thread `{id}` is defined twice"));
            }
        }
        for n in crate::notes(self.project) {
            if let Some(thread) = &n.note.thread
                && self.story().thread(thread.as_str()).is_none()
            {
                self.error(
                    &n.file,
                    &format!("{}.thread", n.at),
                    format!("unknown thread `{thread}`"),
                );
            }
        }
        for t in crate::threads(self.project) {
            let at = format!("thread {}", t.id);
            match t.status {
                ThreadStatus::Open if t.setups > 0 && t.payoffs == 0 => self.warning(
                    &t.file,
                    &at,
                    format!(
                        "`{}` is open and has {} setup(s) and no payoff",
                        t.id, t.setups
                    ),
                ),
                ThreadStatus::Open | ThreadStatus::Resolved if t.payoffs > 0 && t.setups == 0 => {
                    self.warning(
                        &t.file,
                        &at,
                        format!("`{}` has a payoff and no setup", t.id),
                    )
                }
                ThreadStatus::Resolved if t.open_todos > 0 => self.warning(
                    &t.file,
                    &at,
                    format!(
                        "`{}` is resolved and has {} open TODO(s)",
                        t.id, t.open_todos
                    ),
                ),
                _ => {}
            }
        }
    }

    fn ty_of(&self, name: &str) -> Option<lantern_lang::Ty> {
        if let Some(ty) = self.scope.vars.get(name) {
            return Some(ty.clone());
        }
        let (thing, prop) = name.split_once('.')?;
        self.scope.things.get(thing)?.get(prop).cloned()
    }

    fn settable(&self, name: &str, value: &lantern_model::Value) -> bool {
        use lantern_lang::Ty;
        use lantern_model::Value;
        match (self.ty_of(name), value) {
            (Some(Ty::Bool), Value::Bool(_))
            | (Some(Ty::Int), Value::Int(_))
            | (Some(Ty::Text), Value::Text(_)) => true,
            (Some(Ty::Enum(vals)), Value::Text(t)) => vals.contains(t),
            _ => false,
        }
    }
}

fn line_col(text: &str, pos: usize) -> (u32, u32) {
    let before = &text[..pos.min(text.len())];
    let line = before.matches('\n').count() as u32 + 1;
    let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) as u32 + 1;
    (line, col)
}
