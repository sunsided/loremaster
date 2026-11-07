use crate::{Diagnostic, Origin, Pack, Project, Severity, SourceFile, assemble};
use loremaster_lang::{LangError, Scope, compile_condition, compile_markup};
use loremaster_model::{
    Effect, EventPattern, Fragment, Line, LineSegment, Location, PLAYER_SPEAKER, Story,
    ThreadStatus, standard_verbs,
};
use std::collections::BTreeSet;

/// The result of the schema checks: the project, when the packs join, and
/// every error and warning, also the ones of joining the packs.
#[derive(Debug, Clone)]
pub struct Checked {
    /// None when a pack does not read or the packs do not join.
    pub project: Option<Project>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Checked {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

/// The schema checks as one library function (XP-5): join the packs in their
/// order, then check ids, references, facts, conditions and markup of the
/// model, and the names that conditions keep for themselves.
///
/// Every source of a story runs the same checks through this function: the
/// YAML loader at load time ([`check_files`]), and the `world!` macro at
/// compile time on its packs. It needs no files and no I/O. Diagnostics name
/// the file of each entity's origin, YAML or Rust, and the place in the
/// entity (`at`). The line comes from the text of a YAML file, or from the
/// Rust origin of the entity.
pub fn check_packs(packs: Vec<Pack>) -> Checked {
    match assemble(packs) {
        Ok(project) => Checked {
            diagnostics: check(&project),
            project: Some(project),
        },
        Err(diagnostics) => Checked {
            project: None,
            diagnostics,
        },
    }
}

/// [`check_packs`] for the files of one YAML folder: a file that does not
/// read (an unknown field, an unknown folder) is an error too.
pub fn check_files(files: Vec<SourceFile>) -> Checked {
    match Pack::from_files(files) {
        Ok(pack) => check_packs(vec![pack]),
        Err(diagnostics) => Checked {
            project: None,
            diagnostics,
        },
    }
}

/// Check a loaded project: every reference resolves, every condition and
/// every piece of markup parses and type-checks. [`check_packs`] joins packs
/// and runs this.
pub fn check(project: &Project) -> Vec<Diagnostic> {
    let mut c = Checker {
        project,
        scope: Scope::from_story(&project.story),
        out: project.assembly.clone(),
        place: None,
    };
    c.run();
    c.out
}

struct Checker<'a> {
    project: &'a Project,
    scope: Scope,
    out: Vec<Diagnostic>,
    /// The Rust place (file, line, column) of the entity being checked, when
    /// it comes from Rust. Its diagnostics get the line and column, which a
    /// YAML entity gets from the text of its file.
    place: Option<(String, u32, u32)>,
}

impl Checker<'_> {
    fn story(&self) -> &Story {
        &self.project.story
    }

    /// The file of an entity. The checks of the entity follow, so this also
    /// sets the Rust place for their diagnostics.
    fn file_of(&mut self, key: &str) -> String {
        let origin = self.project.origins.get(key);
        self.place = match origin {
            Some(Origin::Rust {
                file, line, column, ..
            }) => Some((file.clone(), *line, *column)),
            _ => None,
        };
        origin
            .map(|o| o.file().to_string())
            .unwrap_or_else(|| "story.yaml".into())
    }

    fn push(&mut self, mut diagnostic: Diagnostic) {
        if diagnostic.line.is_none()
            && let Some((file, line, column)) = &self.place
            && *file == diagnostic.file
        {
            diagnostic.line = Some(*line);
            diagnostic.column = Some(*column);
        }
        self.out.push(diagnostic);
    }

    fn error(&mut self, file: &str, at: &str, message: impl Into<String>) {
        self.push(Diagnostic::error(file, at, message));
    }

    fn warning(&mut self, file: &str, at: &str, message: impl Into<String>) {
        self.push(Diagnostic::warning(file, at, message));
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
        self.push(Diagnostic {
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

    /// A line: the markup and the condition of each segment, the speaker
    /// of each `says`, and a warning for a bare string right after speech.
    /// `at` is the place of the effect, node or option; a segment is at
    /// `<at>.say[i]`.
    fn line(&mut self, file: &str, at: &str, line: &Line) {
        // YAML cannot write an empty line; a builder or `world!` can.
        if line.segments.is_empty() {
            self.error(
                file,
                at,
                "the line is empty: give it a text or at least one segment",
            );
            return;
        }
        if let Some(text) = line.as_plain() {
            self.markup(file, at, text);
            return;
        }
        for (i, s) in line.segments.iter().enumerate() {
            let at = format!("{at}.say[{i}]");
            self.markup(file, &at, &s.text);
            if let Some(when) = &s.when {
                self.condition(file, &format!("{at}.when"), when);
            }
            if let Some(by) = s.speaker()
                && let Err(e) = loremaster_lang::check_speaker(by.as_str(), &self.scope)
            {
                self.error(file, &format!("{at}.by"), e.message);
            }
        }
        for i in line.bare_after_speech() {
            self.warning(
                file,
                &format!("{at}.say[{i}]"),
                "a bare string after `says` reads like more speech; write `{ narrate: \"…\" }` for narration",
            );
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
        // The header comes from story.yaml, or from a pack in Rust.
        let header = self.file_of("story");
        self.room_ref(&header, "start", story.start.as_str());
        self.fragments(&header, "intro", &story.intro);
        self.frames(&header, "frames", &story.frames);
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
                    &header,
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

        self.reserved_names();
        self.threads();
        self.dialogues();
        self.player_thing();
        self.timers();
        self.endings();
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

    /// Variables, machines, rooms and things must not take a built-in name
    /// of conditions, such as `here` or `turns`: conditions would read the
    /// built-in instead (P2-12). A variable or machine must not take the name
    /// of a phase of the story's clock either: it would hide the phase.
    fn reserved_names(&mut self) {
        let story = self.story().clone();
        let phases = story.clock.phase_names();
        for v in &story.vars {
            let phase = phases.contains(&v.id.as_str());
            if !phase && !loremaster_lang::is_reserved(v.id.as_str()) {
                continue;
            }
            let (file, at, kind) = if story.machines.iter().any(|m| m.id == v.id) {
                (
                    self.file_of(&format!("machine:{}", v.id)),
                    format!("machines.{}", v.id),
                    "machine",
                )
            } else if story.score.as_ref().is_some_and(|s| s.var == v.id) {
                (self.file_of("story"), "score.var".into(), "score variable")
            } else {
                (self.file_of("story"), format!("vars.{}", v.id), "variable")
            };
            if phase {
                // A bare phase name tests the time of day only while no
                // variable has that name (`lower_bool` in loremaster-lang).
                self.error(
                    &file,
                    &at,
                    format!(
                        "the {kind} `{}` has the name of a phase of the clock, so conditions would read it instead of the phase; choose another id",
                        v.id
                    ),
                );
            } else {
                self.reserved(&file, &at, kind, v.id.as_str());
            }
        }
        for r in &story.rooms {
            if loremaster_lang::is_reserved(r.id.as_str()) {
                let file = self.file_of(&format!("room:{}", r.id));
                self.reserved(&file, "id", "room", r.id.as_str());
            }
        }
        for t in &story.things {
            if loremaster_lang::is_reserved(t.id.as_str()) {
                let file = self.file_of(&format!("thing:{}", t.id));
                self.reserved(&file, "id", "thing", t.id.as_str());
            }
        }
    }

    fn reserved(&mut self, file: &str, at: &str, kind: &str, id: &str) {
        use loremaster_lang::Reserved;
        let why = match loremaster_lang::reserved(id) {
            Some(Reserved::Keyword) => "is a keyword of conditions, so conditions cannot name it",
            Some(Reserved::Function) => {
                "has the name of a function of conditions, which is reserved for it"
            }
            Some(Reserved::Path) => {
                "is the start of built-in paths of conditions (such as `time.hour`), so the name is reserved for them"
            }
            _ => "has a name that conditions use for a built-in, so they would not read it",
        };
        self.error(
            file,
            at,
            format!("the {kind} `{id}` {why}; choose another id"),
        );
    }

    /// Dialogues: references, conditions and markup, then the structure of
    /// the graph through dramaturge's validator.
    fn dialogues(&mut self) {
        use dramaturge_authoring::{
            GraphValidator, ValidationIssueKind, ValidationOptions, ValidationSeverity,
        };
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
                    self.no_await(&file, &format!("{at}.then"), &o.then);
                    for (name, value) in &o.set {
                        if !self.settable(name, value) {
                            self.error(&file, &at, format!("cannot set `{name}` to `{value}`"));
                        }
                    }
                }
            }
            for n in &d.nodes {
                let at = format!("nodes.{}", n.key);
                self.line(&file, &at, &n.say);
                for (i, o) in n.options.iter().enumerate() {
                    self.line(&file, &format!("{at}.options[{i}]"), &o.say);
                }
            }
            let compiled = match loremaster_lang::compile_dialogue(d, &self.scope, &mut symbols) {
                Ok(c) => c,
                Err(e) => {
                    // The lines are checked above; report their problems once.
                    let known = self
                        .out
                        .iter()
                        .any(|x| x.file == file && x.at == e.at && x.message == e.error.message);
                    if !known {
                        self.lang_error(&file, &e.at, &e.source, e.error);
                    }
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
            // Talks loop back on purpose, so cycles are no issue here.
            let validator = GraphValidator::with_options(
                ValidationOptions::new().skip(ValidationIssueKind::Cycle),
            );
            for issue in validator.validate_with_symbols(&compiled.graph, &symbols) {
                // By design here: start nodes other than the first are entered
                // by their `when`, and facts are set by rules too (Info).
                let start = issue
                    .node_id
                    .is_some_and(|id| compiled.starts.contains(&id));
                if issue.severity == ValidationSeverity::Info
                    || (start && issue.kind == ValidationIssueKind::Unreachable)
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
                Effect::Say(line) => self.line(file, &at, line),
                Effect::Await(spec) => self.await_spec(file, &at, spec),
                Effect::End(ending) => {
                    if self.story().ending(ending.as_str()).is_none() {
                        self.error(
                            file,
                            &at,
                            format!("unknown ending `{ending}`: add it as endings/{ending}.yaml"),
                        );
                    }
                }
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
                Effect::Call(b) => {
                    if !self.story().behaviors.contains(b) {
                        self.error(
                            file,
                            &at,
                            format!("`call: {b}` names no behavior of the story (`behaviors:` in story.yaml or `behaviors/{b}.yaml`)"),
                        );
                    }
                }
                Effect::Stop | Effect::Emit(_) => {}
            }
        }
    }

    /// `by: player` and the speech of an option name the player. A thing
    /// with the id `player` could never speak, and front ends could not tell
    /// its speech from the player's, so it gets a warning once the story has
    /// speech.
    fn player_thing(&mut self) {
        if self.story().thing(PLAYER_SPEAKER).is_none() || !uses_speech(self.story()) {
            return;
        }
        let file = self.file_of(&format!("thing:{PLAYER_SPEAKER}"));
        self.warning(
            &file,
            "id",
            "`player` names the player as a speaker (`by: player`, the speech of an option), so this thing cannot speak in a line; give it another id",
        );
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
        let file = &self.file_of("story");
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

    /// Endings: unique ids, a rank of 1 or more, valid markup, and an `end` that names each one.
    fn endings(&mut self) {
        let endings = self.story().endings.clone();
        let named = ended_endings(self.story());
        let mut seen = BTreeSet::new();
        for e in &endings {
            let file = self.file_of(&format!("ending:{}", e.id));
            let at = format!("endings.{}", e.id);
            if !seen.insert(e.id.clone()) {
                self.error(&file, &at, format!("ending `{}` is defined twice", e.id));
            }
            if e.rank == Some(0) {
                self.error(&file, &format!("{at}.rank"), "the best rank is 1");
            }
            self.markup(&file, &format!("{at}.title"), &e.title);
            self.markup(&file, &format!("{at}.text"), &e.text);
            if !named.contains(e.id.as_str()) {
                self.warning(
                    &file,
                    &at,
                    format!("no `end` effect names ending `{}`", e.id),
                );
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
                self.no_await(&file, &format!("{at}.at.{n}"), effects);
            }
        }
    }

    /// An `await` asks exactly one thing, and its answer fits `into`.
    fn await_spec(&mut self, file: &str, at: &str, spec: &loremaster_model::AwaitSpec) {
        use loremaster_lang::Ty;
        if spec.kinds() != 1 {
            self.error(
                file,
                at,
                "an `await` needs exactly one of `choice`, `action`, `roll` and `walk_to`",
            );
            return;
        }
        if let Some(room) = &spec.walk_to {
            self.room_ref(file, at, room.as_str());
        }
        if let Some(dice) = &spec.roll
            && loremaster_model::Dice::parse(dice).is_none()
        {
            self.error(
                file,
                at,
                format!("`{dice}` is not dice such as `2d6` or `1d20+3`"),
            );
        }
        if spec.action.is_none() && (!spec.args.is_empty() || !spec.results.is_empty()) {
            self.error(file, at, "`args` and `results` belong to an `action`");
        }
        let Some(into) = &spec.into else {
            return;
        };
        let answers: Vec<String> = if !spec.choice.is_empty() {
            spec.choice.iter().map(|o| o.id().to_string()).collect()
        } else if spec.walk_to.is_some() {
            vec!["arrived".into(), "refused".into()]
        } else {
            spec.results.iter().map(|r| r.to_string()).collect()
        };
        match (self.ty_of(into), spec.roll.is_some()) {
            (None, _) => self.error(file, at, format!("`into: {into}` names no variable")),
            (Some(Ty::Int), true) | (Some(Ty::Text), false) => {}
            // An action without results: the host answers with any value.
            (Some(_), false) if spec.action.is_some() && spec.results.is_empty() => {}
            (Some(Ty::Enum(values)), false) => {
                for a in answers.iter().filter(|a| !values.contains(a)) {
                    self.error(file, at, format!("`{into}` cannot hold the answer `{a}`"));
                }
            }
            (Some(_), true) => self.error(
                file,
                at,
                format!("a roll needs a number variable, `{into}` is not one"),
            ),
            (Some(_), false) => self.error(
                file,
                at,
                format!("`{into}` cannot hold the answer of this `await`"),
            ),
        }
    }

    /// Only a rule can wait for the host: a timer or a dialogue has no place to go on from.
    fn no_await(&mut self, file: &str, at: &str, effects: &[Effect]) {
        for (i, e) in effects.iter().enumerate() {
            if matches!(e, Effect::Await(_)) {
                self.error(file, &format!("{at}[{i}]"), "`await` works only in rules");
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

/// The ids of the endings that some `end` effect names.
fn ended_endings(story: &Story) -> BTreeSet<String> {
    all_effects(story)
        .filter_map(|e| match e {
            Effect::End(id) => Some(id.to_string()),
            _ => None,
        })
        .collect()
}

/// The names of the custom events that some `emit` sends.
fn emitted_events(story: &Story) -> BTreeSet<String> {
    all_effects(story)
        .filter_map(|e| match e {
            Effect::Emit(name) => Some(name.to_string()),
            _ => None,
        })
        .collect()
}

/// Does a line of the story have speech? Machine steps are rules by now.
fn uses_speech(story: &Story) -> bool {
    let effect_lines = all_effects(story).filter_map(|e| match e {
        Effect::Say(line) => Some(line),
        _ => None,
    });
    let dialogue_lines = story.dialogues.iter().flat_map(|d| {
        d.nodes
            .iter()
            .flat_map(|n| std::iter::once(&n.say).chain(n.options.iter().map(|o| &o.say)))
    });
    effect_lines
        .chain(dialogue_lines)
        .any(|l| l.segments.iter().any(LineSegment::is_speech))
}

/// The effects of all rules (machine steps included), timers and dialogues.
fn all_effects(story: &Story) -> impl Iterator<Item = &Effect> {
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
    rule_effects.chain(timer_effects).chain(dialogue_effects)
}

fn line_col(text: &str, pos: usize) -> (u32, u32) {
    let before = &text[..pos.min(text.len())];
    let line = before.matches('\n').count() as u32 + 1;
    let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) as u32 + 1;
    (line, col)
}
