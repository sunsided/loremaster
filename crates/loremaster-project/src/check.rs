use crate::{Diagnostic, Project, Severity};
use loremaster_lang::{LangError, Scope, compile_condition, compile_markup};
use loremaster_model::{
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

    fn frames(&mut self, file: &str, at: &str, frames: &loremaster_model::Frames) {
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
                (loremaster_model::VarType::Bool, loremaster_model::Value::Bool(_)) => true,
                (loremaster_model::VarType::Int, loremaster_model::Value::Int(_)) => true,
                (loremaster_model::VarType::Enum(vals), loremaster_model::Value::Text(t)) => {
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
            match &thing.location {
                Location::Room(r) => self.room_ref(&file, "location", r.as_str()),
                Location::In(h) | Location::On(h) => {
                    self.holder_ref(&file, "location", &thing.location, h.as_str())
                }
                Location::Carried | Location::Nowhere => {}
            }
            for (i, w) in thing.also_in.iter().enumerate() {
                let region = story.rooms.iter().any(|r| r.region.as_ref() == Some(w));
                if story.room(w.as_str()).is_none() && !region {
                    self.error(
                        &file,
                        &format!("also_in[{i}]"),
                        format!("`{w}` is not a room or a region"),
                    );
                }
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
        self.dialogues();
        self.timers();
        self.machines();
        self.score();

        let verbs: BTreeSet<String> = standard_verbs()
            .iter()
            .chain(story.verbs.iter())
            .map(|v| v.id.to_string())
            .collect();
        let emitted = emitted_events(&story);
        for rule in &story.rules {
            let file = self.file_of(&format!("rule:{}", rule.id));
            match &rule.on {
                EventPattern::Enter(r) | EventPattern::Leave(r) => {
                    self.room_ref(&file, "on", r.as_str())
                }
                EventPattern::Verb(parts) | EventPattern::Check(parts) => {
                    let at = if matches!(rule.on, EventPattern::Check(_)) {
                        "on.check"
                    } else {
                        "on.verb"
                    };
                    match parts.split_first() {
                        Some((verb, objects)) => {
                            if !verbs.contains(verb.as_str()) {
                                self.error(&file, at, format!("unknown verb `{verb}`"));
                            }
                            for o in objects {
                                let direction = verb.as_str() == "go";
                                if !direction
                                    && self.story().thing(o.as_str()).is_none()
                                    && self.story().room(o.as_str()).is_none()
                                {
                                    self.error(&file, at, format!("unknown thing `{o}`"));
                                }
                            }
                        }
                        None => self.error(&file, at, "name the verb"),
                    }
                }
                EventPattern::Timer(t) => self.timer_ref(&file, "on.timer", t.as_str()),
                EventPattern::Talk(parts) => match parts.split_first() {
                    Some((d, node)) => match self.story().dialogues.iter().find(|x| x.id == *d) {
                        None => self.error(&file, "on.talk", format!("unknown dialogue `{d}`")),
                        Some(dialogue) => {
                            if let Some(n) = node.first()
                                && dialogue.node(n.as_str()).is_none()
                            {
                                self.error(
                                    &file,
                                    "on.talk",
                                    format!("dialogue `{d}` has no node `{n}`"),
                                );
                            }
                        }
                    },
                    None => self.error(&file, "on.talk", "name the dialogue"),
                },
                EventPattern::Event(name) => {
                    if !emitted.contains(name.as_str()) {
                        self.warning(
                            &file,
                            "on.event",
                            format!("no `emit` sends the event `{name}`"),
                        );
                    }
                }
                EventPattern::Start | EventPattern::Turn | EventPattern::Time(_) => {}
            }
            if let Some(w) = &rule.within
                && self.story().thing(w.as_str()).is_none()
                && self.story().room(w.as_str()).is_none()
                && !self
                    .story()
                    .rooms
                    .iter()
                    .any(|r| r.region.as_ref() == Some(w))
            {
                self.error(
                    &file,
                    "within",
                    format!("`{w}` is not a thing, a room or a region"),
                );
            }
            if let Some(when) = &rule.when {
                self.condition(&file, "when", when);
            }
            self.effects(&file, "then", &rule.then);
        }
    }

    /// Dialogues: references, conditions and markup, then the structure of
    /// the graph through dramaturge's validator.
    fn dialogues(&mut self) {
        use dramaturge_authoring::{GraphValidator, ValidationSeverity};
        let story = self.story().clone();
        let mut symbols = self.scope.symbols.clone();
        let mut with = BTreeSet::new();
        for d in &story.dialogues {
            let file = self.file_of(&format!("dialogue:{}", d.id));
            if self.story().thing(d.with.as_str()).is_some() && !with.insert(d.with.clone()) {
                self.warning(
                    &file,
                    "with",
                    format!("`{}` has two dialogues; the talk uses the first", d.with),
                );
            }
            for n in &d.nodes {
                for (at, o) in std::iter::once((format!("nodes.{}", n.key), &n.outcomes)).chain(
                    n.options
                        .iter()
                        .enumerate()
                        .map(|(i, o)| (format!("nodes.{}.options[{i}]", n.key), &o.outcomes)),
                ) {
                    self.effects(&file, &format!("{at}.then"), &o.then);
                    for (name, value) in &o.set {
                        if !self.settable(name, value) {
                            self.error(&file, &at, format!("cannot set `{name}` to `{value}`"));
                        }
                    }
                }
            }
            let compiled = match loremaster_lang::compile_dialogue(d, &self.scope, &mut symbols) {
                Ok(c) => c,
                Err(e) => {
                    self.lang_error(&file, &e.at, &e.source, e.error);
                    continue;
                }
            };
            let prefix = format!("{}/", d.id);
            let node_at = |id| {
                symbols
                    .node_key(id)
                    .and_then(|k: &str| k.strip_prefix(&prefix))
                    .map(|k| format!("nodes.{k}"))
                    .unwrap_or_default()
            };
            for issue in GraphValidator::new().validate_with_symbols(&compiled.graph, &symbols) {
                // dramaturge reports issues as text only; these are by design here:
                // talks loop back, start nodes other than the first are entered
                // by their `when`, and facts are set by rules too.
                let start = issue
                    .node_id
                    .is_some_and(|id| compiled.starts.contains(&id));
                if issue.severity == ValidationSeverity::Info
                    || issue.message.contains("is part of a cycle")
                    || (start && issue.message.contains("is unreachable"))
                {
                    continue;
                }
                let at = issue.node_id.map(node_at).unwrap_or_default();
                let message = issue.message.replace(&prefix, "");
                match issue.severity {
                    ValidationSeverity::Error => self.error(&file, &at, message),
                    _ => self.warning(&file, &at, message),
                }
            }
        }
    }

    /// The effects of a rule or of a dialogue's `then`.
    fn effects(&mut self, file: &str, at: &str, effects: &[Effect]) {
        for (i, effect) in effects.iter().enumerate() {
            let at = format!("{at}[{i}]");
            match effect {
                Effect::Say(text) | Effect::End(text) => self.markup(file, &at, text),
                Effect::Goto(room) => self.room_ref(file, &at, room.as_str()),
                Effect::Move(moves) => {
                    for (thing, to) in moves {
                        self.thing_ref(file, &at, thing.as_str());
                        match Location::from_word(to.as_str()) {
                            Location::Room(r) => self.room_ref(file, &at, r.as_str()),
                            loc @ (Location::In(_) | Location::On(_)) => {
                                let h = loc.holder().cloned().unwrap_or_else(|| to.clone());
                                self.holder_ref(file, &at, &loc, h.as_str())
                            }
                            Location::Carried | Location::Nowhere => {}
                        }
                    }
                }
                Effect::Set(values) => {
                    for (name, value) in values {
                        if !self.settable(name, value) {
                            self.error(file, &at, format!("cannot set `{name}` to `{value}`"));
                        }
                    }
                }
                Effect::Add(values) => {
                    for name in values.keys() {
                        if !matches!(self.ty_of(name), Some(loremaster_lang::Ty::Int)) {
                            self.error(file, &at, format!("`{name}` is not a number variable"));
                        }
                    }
                }
                Effect::Start(timers) => {
                    for (t, _) in timers.entries() {
                        self.timer_ref(file, &at, t.as_str());
                    }
                }
                Effect::Cancel(t) => self.timer_ref(file, &at, t.as_str()),
                Effect::Stop | Effect::Emit(_) => {}
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

    /// `in: x` needs a container `x`, `on: x` a supporter.
    fn holder_ref(&mut self, file: &str, at: &str, loc: &Location, id: &str) {
        let Some(holder) = self.story().thing(id) else {
            self.error(file, at, format!("unknown thing `{id}`"));
            return;
        };
        match loc {
            Location::In(_) if !holder.container => {
                self.error(file, at, format!("`{id}` is not a container"))
            }
            Location::On(_) if !holder.supporter => {
                self.error(file, at, format!("`{id}` is not a supporter"))
            }
            _ => {}
        }
    }

    fn timer_ref(&mut self, file: &str, at: &str, id: &str) {
        if self.story().timer(id).is_none() {
            self.error(file, at, format!("unknown timer `{id}`"));
        }
    }

    /// The score: a number variable, a positive maximum, and ranks in reach.
    fn score(&mut self) {
        let Some(score) = self.story().score.clone() else {
            return;
        };
        let file = "story.yaml";
        if score.max <= 0 {
            self.error(file, "score.max", "the highest score must be more than 0");
        }
        if !matches!(
            self.ty_of(score.var.as_str()),
            Some(loremaster_lang::Ty::Int)
        ) {
            self.error(
                file,
                "score.var",
                format!("`{}` is not a number variable", score.var),
            );
        }
        let mut seen = BTreeSet::new();
        for (i, rank) in score.ranks.iter().enumerate() {
            let at = format!("score.ranks[{i}]");
            if !seen.insert(rank.from) {
                self.error(file, &at, format!("two ranks start at {}", rank.from));
            }
            if rank.from > score.max {
                self.warning(
                    file,
                    &at,
                    format!(
                        "the rank {} starts above the highest score {}",
                        rank.name, score.max
                    ),
                );
            }
        }
    }

    /// Machines: known states, and states that some step reaches.
    fn machines(&mut self) {
        let machines = self.story().machines.clone();
        let mut seen = BTreeSet::new();
        for m in &machines {
            let file = self.file_of(&format!("machine:{}", m.id));
            let at = format!("machines.{}", m.id);
            if !seen.insert(m.id.clone()) {
                self.error(&file, &at, format!("machine `{}` is defined twice", m.id));
            }
            let clash = self.story().vars.iter().filter(|v| v.id == m.id).count() > 1
                || self.story().thing(m.id.as_str()).is_some();
            if clash {
                self.error(
                    &file,
                    &at,
                    format!("`{}` is a machine and also a variable or thing", m.id),
                );
            }
            let states = m.all_states();
            let mut names = BTreeSet::new();
            for s in &states {
                if !names.insert(s.as_str()) {
                    self.error(
                        &file,
                        &format!("{at}.states"),
                        format!("the state `{s}` is there twice"),
                    );
                }
                if !is_state_name(s) {
                    self.error(
                        &file,
                        &format!("{at}.states"),
                        format!("`{s}` is not a state name: use one word of letters, digits and _, not a keyword"),
                    );
                }
            }
            let known = |s: &str| states.iter().any(|x| x == s);
            if !known(&m.start) {
                self.error(
                    &file,
                    &format!("{at}.start"),
                    format!("unknown state `{}`", m.start),
                );
            }
            for (i, step) in m.steps.iter().enumerate() {
                if step.from.list().is_empty() {
                    self.error(
                        &file,
                        &format!("{at}.steps[{i}].from"),
                        "the step has no `from` state, so it never fires",
                    );
                }
                for s in step.from.list().into_iter().chain([step.to.as_str()]) {
                    if !known(s) {
                        self.error(
                            &file,
                            &format!("{at}.steps[{i}]"),
                            format!("unknown state `{s}`"),
                        );
                    }
                }
            }
            for s in &states {
                if *s != m.start && !m.steps.iter().any(|step| step.to == *s) {
                    self.warning(&file, &at, format!("no step goes to the state `{s}`"));
                }
            }
        }
    }

    /// Timers: their conditions and effects.
    fn timers(&mut self) {
        let timers = self.story().timers.clone();
        let mut seen = BTreeSet::new();
        for t in &timers {
            let file = self.file_of(&format!("timer:{}", t.id));
            let at = format!("timers.{}", t.id);
            if !seen.insert(t.id.clone()) {
                self.error(&file, &at, format!("timer `{}` is defined twice", t.id));
            }
            if t.turns == 0 {
                self.error(&file, &at, "a timer needs at least one turn");
            }
            if let Some(cond) = &t.runs_while {
                self.condition(&file, &format!("{at}.runs_while"), cond);
            }
            for (n, effects) in &t.at {
                // The timer counts down before it looks at `at`, so it never shows its start value.
                if *n >= t.turns {
                    self.warning(
                        &file,
                        &format!("{at}.at.{n}"),
                        format!(
                            "the timer starts at {} and counts down first, so it never reaches {n}",
                            t.turns
                        ),
                    );
                }
                self.effects(&file, &format!("{at}.at.{n}"), effects);
            }
        }
    }

    fn ty_of(&self, name: &str) -> Option<loremaster_lang::Ty> {
        if let Some(ty) = self.scope.vars.get(name) {
            return Some(ty.clone());
        }
        let (thing, prop) = name.split_once('.')?;
        self.scope.things.get(thing)?.get(prop).cloned()
    }

    fn settable(&self, name: &str, value: &loremaster_model::Value) -> bool {
        use loremaster_lang::Ty;
        use loremaster_model::Value;
        match (self.ty_of(name), value) {
            (Some(Ty::Bool), Value::Bool(_))
            | (Some(Ty::Int), Value::Int(_))
            | (Some(Ty::Text), Value::Text(_)) => true,
            (Some(Ty::Enum(vals)), Value::Text(t)) => vals.contains(t),
            _ => false,
        }
    }
}

/// A state name is one word that conditions can name: `exorcism == lit`.
fn is_state_name(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !matches!(s, "and" | "or" | "not" | "true" | "false")
}

/// The names of the custom events that some `emit` sends.
fn emitted_events(story: &Story) -> BTreeSet<String> {
    let rule_effects = story.rules.iter().flat_map(|r| r.then.iter());
    let timer_effects = story.timers.iter().flat_map(|t| t.at.values().flatten());
    let dialogue_effects = story.dialogues.iter().flat_map(|d| {
        d.nodes.iter().flat_map(|n| {
            n.outcomes
                .then
                .iter()
                .chain(n.options.iter().flat_map(|o| o.outcomes.then.iter()))
        })
    });
    rule_effects
        .chain(timer_effects)
        .chain(dialogue_effects)
        .filter_map(|e| match e {
            Effect::Emit(name) => Some(name.to_string()),
            _ => None,
        })
        .collect()
}

fn line_col(text: &str, pos: usize) -> (u32, u32) {
    let before = &text[..pos.min(text.len())];
    let line = before.matches('\n').count() as u32 + 1;
    let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) as u32 + 1;
    (line, col)
}
