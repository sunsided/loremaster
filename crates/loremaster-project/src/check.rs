use crate::{Diagnostic, Origin, Pack, Project, Severity, SourceFile, assemble};
use dramaturge_model::{Candidate, Condition, Entry, FragmentMode, Pick, Pool, Pools, TextTypes};
use loremaster_lang::{
    LangError, SEEN_BY_ELSEWHERE, Scope, compile_condition, compile_markup, has_seen_by,
};
use loremaster_model::{
    DialogueId, Effect, EventPattern, Fragment, Line, LineSegment, Location, PLAYER_SPEAKER, Story,
    ThreadStatus, standard_verbs,
};
use std::collections::BTreeSet;

mod walkthroughs;

pub use walkthroughs::MAX_HINTS;
pub(crate) use walkthroughs::mentions;

/// The host types of a pool in the checks: only what dramaturge's checks
/// read (conditions for the score, the tuple keys, the named pools).
struct CheckTypes;

impl TextTypes for CheckTypes {
    type Leaf = ();
    type Cond = Condition;
    type Learn = ();
    type Actor = String;
    type Topic = String;
}

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
        named: None,
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
    /// The named pools of `texts/` for dramaturge's checks, built once.
    named: Option<Pools<CheckTypes>>,
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

    /// A condition outside a text: `seen_by` has no perceiver there.
    fn condition(&mut self, file: &str, at: &str, source: &str) {
        match compile_condition(source, &self.scope) {
            Ok(c) if has_seen_by(&c) => self.error(file, at, SEEN_BY_ELSEWHERE),
            Ok(_) => {}
            Err(e) => self.lang_error(file, at, source, e),
        }
    }

    /// The `when` of a fragment or a candidate: `seen_by` asks for the
    /// perceiver of the text.
    fn text_condition(&mut self, file: &str, at: &str, source: &str) {
        if let Err(e) = compile_condition(source, &self.scope) {
            self.lang_error(file, at, source, e);
        }
    }

    /// A line that cannot show a text pool: an option's `say` and a
    /// journal line render without the core.
    fn line_without_pool(&mut self, file: &str, at: &str, line: &Line) {
        if line.pool.is_some() {
            self.error(
                file,
                at,
                "`{ text: <pool> }` works in a rule's `say` and in a dialogue node's lines, not here",
            );
            return;
        }
        self.line(file, at, line);
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
        if let Some(pool) = &line.pool {
            if self.story().text_pool(pool.as_str()).is_none() {
                self.error(
                    file,
                    at,
                    format!("unknown text pool `{pool}`; add texts/{pool}.yaml"),
                );
            }
            return;
        }
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
        // A fragment without an id is keyed by its index.
        let mut keys = BTreeSet::new();
        for (i, f) in fragments.iter().enumerate() {
            let key = f.id.as_ref().map_or(i.to_string(), |id| id.to_string());
            if !keys.insert(key.clone()) {
                let place = if f.id.is_some() { ".id" } else { "" };
                self.error(
                    file,
                    &format!("{at}[{i}]{place}"),
                    format!("two fragments of this text have the key `{key}` (an `id`, or the index of a fragment without one); they would share their progress"),
                );
            }
            if let Some(when) = &f.when {
                self.text_condition(file, &format!("{at}[{i}].when"), when);
            }
            for v in &f.variants {
                self.markup(file, &format!("{at}[{i}]"), v);
            }
            self.learn(file, &format!("{at}[{i}].learn"), &f.learn);
            if !f.one_of.is_empty() {
                let at = format!("{at}[{i}]");
                if !f.variants.is_empty() {
                    self.error(
                        file,
                        &at,
                        "a group has `one_of` in place of `text` or `variants`",
                    );
                }
                self.candidates(file, &at, &f.one_of, &mut |c, at, text| {
                    c.markup(file, at, text)
                });
                let index = i.to_string();
                let id = f.id.as_ref().map_or(index.as_str(), |id| id.as_str());
                let pick = match f.pick {
                    loremaster_model::Pick::All => Pick::All,
                    loremaster_model::Pick::First => Pick::First,
                    loremaster_model::Pick::Best => Pick::Best,
                };
                let pool = self.pool(id, pick, &f.one_of, &|_: &String| None);
                self.pool_issues(file, &at, &pool);
            } else if f.variants.is_empty() {
                self.error(file, &format!("{at}[{i}]"), "fragment has no text");
            }
        }
    }

    /// `learn` names story variables of type bool: they become true.
    fn learn(&mut self, file: &str, at: &str, learn: &[impl AsRef<str>]) {
        for var in learn {
            let var = var.as_ref();
            if self.ty_of(var) != Some(loremaster_lang::Ty::Bool) {
                self.error(
                    file,
                    at,
                    format!("`learn` needs a story variable of type bool; `{var}` is not one"),
                );
            }
        }
    }

    /// The candidates of a `one_of` list: conditions, texts (`text` checks
    /// one), `learn`, and the speaker, a thing or `player`.
    fn candidates<T>(
        &mut self,
        file: &str,
        at: &str,
        one_of: &[loremaster_model::Candidate<T>],
        text: &mut dyn FnMut(&mut Self, &str, &T),
    ) {
        use dramaturge_syntax::Variant;
        // A candidate without a key is keyed by its index.
        let mut keys = BTreeSet::new();
        for (i, c) in one_of.iter().enumerate() {
            let at = format!("{at}.one_of[{i}]");
            let key = c.key.clone().unwrap_or_else(|| i.to_string());
            if !keys.insert(key.clone()) {
                let place = if c.key.is_some() { ".key" } else { "" };
                self.error(
                    file,
                    &format!("{at}{place}"),
                    format!("two candidates of this `one_of` have the key `{key}` (a `key`, or the index of a candidate without one); they would share their progress"),
                );
            }
            if let Some(when) = &c.when {
                self.text_condition(file, &format!("{at}.when"), when);
            }
            if let Some(by) = &c.speaker
                && let Err(e) = loremaster_lang::check_speaker(by, &self.scope)
            {
                self.error(file, &format!("{at}.speaker"), e.message);
            }
            self.learn(file, &format!("{at}.learn"), &c.learn);
            for v in &c.variants {
                match v {
                    Variant::Text(t) => text(self, &at, t),
                    Variant::Pool(p) => self.candidates(file, &at, &p.one_of, text),
                    _ => {}
                }
            }
        }
    }

    /// A pool for dramaturge's checks: the conditions, keys, modes, speaker
    /// and topic of the candidates, and the named pools that its texts name.
    fn pool<T>(
        &self,
        id: &str,
        pick: Pick,
        one_of: &[loremaster_model::Candidate<T>],
        named: &dyn Fn(&T) -> Option<String>,
    ) -> Pool<CheckTypes> {
        use dramaturge_syntax::{FragmentMode as SMode, Pick as SPick, Variant};
        let mut pool = Pool::new(id, pick);
        for (i, c) in one_of.iter().enumerate() {
            let variants = c
                .variants
                .iter()
                .filter_map(|v| match v {
                    Variant::Text(t) => Some(named(t).map_or(Entry::Leaf(()), Entry::named)),
                    Variant::Pool(p) => {
                        let index = i.to_string();
                        let id = p.id.as_deref().or(c.key.as_deref()).unwrap_or(&index);
                        let pick = match p.pick.unwrap_or_default() {
                            SPick::All => Pick::All,
                            SPick::Best => Pick::Best,
                            _ => Pick::First,
                        };
                        Some(Entry::Pool(self.pool(id, pick, &p.one_of, named)))
                    }
                    _ => None,
                })
                .collect();
            let mut candidate = Candidate::new(variants).mode(match c.mode {
                SMode::FirstTime => FragmentMode::FirstTime,
                SMode::Once => FragmentMode::Once,
                SMode::Cycle => FragmentMode::Cycle,
                SMode::Shuffle => FragmentMode::Shuffle,
                SMode::Sticky => FragmentMode::Sticky,
                _ => FragmentMode::Always,
            });
            candidate.key = c.key.as_deref().map(Into::into);
            candidate.speaker = c.speaker.clone();
            candidate.topic = c.topic.clone();
            candidate.when = c
                .when
                .as_deref()
                .and_then(|w| compile_condition(w, &self.scope).ok());
            pool.candidates.push(candidate);
        }
        pool
    }

    /// dramaturge's checks of a pool (ties of `pick: best`, progress
    /// without a key, unknown pools) as diagnostics at `at`.
    fn pool_issues(&mut self, file: &str, at: &str, pool: &Pool<CheckTypes>) {
        if self.named.is_none() {
            self.named = Some(self.named_pools());
        }
        let pools = self.named.as_ref().expect("built above");
        let issues = dramaturge_authoring::GraphValidator::new().validate_pool(pool, pools);
        self.report_pool_issues(file, at, issues);
    }

    fn report_pool_issues(
        &mut self,
        file: &str,
        at: &str,
        issues: Vec<dramaturge_authoring::ValidationIssue>,
    ) {
        use dramaturge_authoring::{ValidationIssueKind as K, ValidationSeverity};
        for issue in issues {
            // The pool checks; an error of another kind fails closed.
            let pool_kind = matches!(issue.kind, K::PickTie | K::UnkeyedProgress | K::UnknownPool);
            if !pool_kind && issue.severity != ValidationSeverity::Error {
                continue;
            }
            let at = match issue.path.as_deref() {
                Some(path) if at.is_empty() => path.to_string(),
                Some(path) => format!("{at} ({path})"),
                None => at.to_string(),
            };
            match issue.severity {
                ValidationSeverity::Error => self.error(file, &at, issue.message),
                _ => self.warning(file, &at, issue.message),
            }
        }
    }

    fn named_pools(&self) -> Pools<CheckTypes> {
        let mut pools = Pools::new();
        for p in &self.story().texts {
            let id = p.id.clone().unwrap_or_default();
            let pick = match p.pick.unwrap_or_default() {
                dramaturge_syntax::Pick::All => Pick::All,
                dramaturge_syntax::Pick::Best => Pick::Best,
                _ => Pick::First,
            };
            let named = |l: &Line| l.pool.as_ref().map(|p| p.to_string());
            pools.insert(self.pool(&id, pick, &p.one_of, &named));
        }
        pools
    }

    /// The named text pools of `texts/`: their candidates, and dramaturge's
    /// checks of all of them.
    fn texts(&mut self) {
        let story = self.story().clone();
        let (mut seen, mut reported) = (BTreeSet::new(), BTreeSet::new());
        for p in &story.texts {
            let id = p.id.clone().unwrap_or_default();
            let file = self.file_of(&format!("text:{id}"));
            // One error per id: its origin is the last file of the id.
            if !seen.insert(id.clone()) && reported.insert(id.clone()) {
                self.error(&file, "id", format!("text pool `{id}` is defined twice"));
            }
            self.candidates(&file, "", &p.one_of, &mut |c, at, line| {
                c.line(&file, at.trim_start_matches('.'), line)
            });
        }
        let pools = self.named_pools();
        let issues = dramaturge_authoring::GraphValidator::new().validate_pools(&pools);
        for issue in issues {
            // The path starts with the pool id: the file of that pool.
            let id = issue
                .path
                .as_deref()
                .and_then(|p| p.split('/').next())
                .unwrap_or("");
            let file = self.file_of(&format!("text:{id}"));
            self.report_pool_issues(&file, "", vec![issue]);
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
        let mut verbs: BTreeSet<String> = standard_verbs()
            .iter()
            .chain(story.verbs.iter())
            .map(|v| v.id.to_string())
            .collect();
        // The world adds the `score` verb for a story with a score.
        if story.score.is_some() {
            verbs.insert("score".into());
        }
        for verb in story.clock.costs.keys() {
            if !verbs.contains(verb.as_str()) {
                self.error(
                    &header,
                    &format!("clock.costs.{verb}"),
                    format!("unknown verb `{verb}`"),
                );
            }
        }
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
            self.limits(&header, v);
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
        self.texts();
        self.player_thing();
        self.timers();
        self.agendas();
        self.endings();
        self.machines();
        self.score();
        self.stats();
        self.episodes();

        let verbs: BTreeSet<String> = standard_verbs()
            .iter()
            .chain(story.verbs.iter())
            .map(|v| v.id.to_string())
            .collect();
        let emitted = emitted_events(&story);
        let (mut seen, mut reported) = (BTreeSet::new(), BTreeSet::new());
        for rule in &story.rules {
            let file = self.file_of(&format!("rule:{}", rule.id));
            // One error per id: its origin is the last file of the id.
            if !seen.insert(rule.id.as_str()) && reported.insert(rule.id.as_str()) {
                self.error(&file, "id", format!("rule `{}` is defined twice", rule.id));
            }
            self.closes(
                &file,
                "",
                &format!("`{}`", rule.id),
                &rule.closes,
                &rule.reason,
            );
            match &rule.on {
                EventPattern::Enter(r) | EventPattern::Leave(r) => {
                    self.room_ref(&file, "on", r.as_str())
                }
                EventPattern::Verb(pattern) | EventPattern::Check(pattern) => {
                    let at = if matches!(rule.on, EventPattern::Check(_)) {
                        "on.check"
                    } else {
                        "on.verb"
                    };
                    self.command(&file, at, pattern, &verbs);
                }
                EventPattern::Timer(t) => self.timer_ref(&file, "on.timer", t.as_str()),
                EventPattern::Talk(parts) => match parts.split_first() {
                    Some((d, node)) => match self.story().dialogue(d.as_str()) {
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
                EventPattern::Next if rule.machine.is_none() => {
                    self.error(&file, "on", "`on: next` works only in a machine step")
                }
                EventPattern::Start
                | EventPattern::Turn
                | EventPattern::Time(_)
                | EventPattern::Next => {}
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
        self.walkthroughs();
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

    /// Dialogues: the structure (dramaturge-syntax's check), references,
    /// conditions and markup, then the graph through dramaturge's validator.
    fn dialogues(&mut self) {
        use dramaturge_authoring::{
            GraphValidator, ValidationIssueKind, ValidationOptions, ValidationSeverity,
        };
        use dramaturge_syntax::IssueKind;
        let story = self.story().clone();
        let mut symbols = self.scope.symbols.clone();
        let mut with = BTreeSet::new();
        for d in &story.dialogues {
            let file = self.file_of(&format!("dialogue:{}", d.id_str()));
            if self.story().thing(d.with.as_str()).is_some() && !with.insert(d.with.clone()) {
                self.warning(
                    &file,
                    "with",
                    format!("`{}` has two dialogues; the talk uses the first", d.with),
                );
            }
            // Keys, start nodes, targets and labels. A node without options
            // ends the talk in loremaster by design, so `NoWayOn` is no issue.
            let mut issues = dramaturge_syntax::check(d);
            issues.retain(|i| i.kind != IssueKind::NoWayOn);
            let broken = issues
                .iter()
                .any(|i| i.severity == dramaturge_syntax::Severity::Error);
            for issue in &issues {
                match issue.severity {
                    dramaturge_syntax::Severity::Error => {
                        self.error(&file, &issue.path, issue.message.clone())
                    }
                    _ => self.warning(&file, &issue.path, issue.message.clone()),
                }
            }
            self.not_played_yet(&file, d);
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
                        let value = loremaster_model::Value::from(value);
                        if !self.settable(name, &value) {
                            self.error(&file, &at, format!("cannot set `{name}` to `{value}`"));
                        }
                    }
                }
            }
            for n in &d.nodes {
                let at = format!("nodes.{}", n.key);
                self.line(&file, &at, &n.say);
                for (i, l) in n.lines.iter().enumerate() {
                    self.line(&file, &format!("{at}.lines[{i}]"), &l.say);
                }
                for (i, o) in n.options.iter().enumerate() {
                    if let Some(say) = &o.say {
                        self.line_without_pool(&file, &format!("{at}.options[{i}]"), say);
                    }
                }
            }
            if broken {
                continue;
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
            let prefix = format!("{}/", d.id_str());
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
            for issue in validator.validate_with_symbols(&compiled, &symbols) {
                // By design here: facts are set by rules too (Info).
                if issue.severity == ValidationSeverity::Info {
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

    /// Fields of the dialogue shape that the talks of loremaster do not
    /// play yet: the option fields `hint`, `tags` and `intentions`.
    fn not_played_yet(&mut self, file: &str, d: &loremaster_model::Dialogue) {
        for n in &d.nodes {
            let at = format!("nodes.{}", n.key);
            for (i, o) in n.options.iter().enumerate() {
                let at = format!("{at}.options[{i}]");
                for (field, used) in [
                    ("hint", o.hint.is_some()),
                    ("tags", !o.tags.is_empty()),
                    ("intentions", !o.intentions.is_empty()),
                ] {
                    if used {
                        self.error(file, &format!("{at}.{field}"), not_yet(field));
                    }
                }
            }
        }
    }

    /// `closes` and `reason` of a rule, machine step or agenda step: each id
    /// is an ending, and a list needs a reason. The ending checks of the
    /// explorer compare the list with the endings that really drop out.
    /// `prefix` is the place of the owner in its file (`steps[0].` for an
    /// agenda step, empty for a rule), `name` how the messages name it.
    fn closes(
        &mut self,
        file: &str,
        prefix: &str,
        name: &str,
        closes: &[loremaster_model::Id],
        reason: &Option<String>,
    ) {
        for (i, ending) in closes.iter().enumerate() {
            if self.story().ending(ending.as_str()).is_none() {
                self.error(
                    file,
                    &format!("{prefix}closes[{i}]"),
                    format!("unknown ending `{ending}` in `closes`"),
                );
            }
        }
        let reason = reason.as_deref().is_some_and(|r| !r.trim().is_empty());
        if !closes.is_empty() && !reason {
            self.error(
                file,
                &format!("{prefix}closes"),
                format!("`closes` of {name} needs a `reason`: why the endings close on purpose"),
            );
        }
        if closes.is_empty() && reason {
            self.warning(
                file,
                &format!("{prefix}reason"),
                format!("`reason` of {name} without `closes`"),
            );
        }
    }

    /// The effects of a rule or of a dialogue's `then`.
    fn effects(&mut self, file: &str, at: &str, effects: &[Effect]) {
        for (i, effect) in effects.iter().enumerate() {
            let at = format!("{at}[{i}]");
            match effect {
                Effect::Say(line) => self.line(file, &at, line),
                Effect::Await(spec) => self.await_spec(file, &at, spec),
                Effect::Combat(spec) => self.combat_spec(file, &at, spec),
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
                Effect::Calc(sums) => {
                    for (name, sum) in sums {
                        let at = format!("{at}.calc.{name}");
                        if !matches!(self.ty_of(name), Some(loremaster_lang::Ty::Int)) {
                            self.error(file, &at, format!("`{name}` is not a number variable"));
                        }
                        match loremaster_model::parse_sum(sum) {
                            Err((why, _)) => self.error(file, &at, format!("`{sum}`: {why}")),
                            Ok(terms) => {
                                for (_, term) in terms {
                                    if let loremaster_model::Term::Name(n) = term
                                        && !matches!(self.ty_of(&n), Some(loremaster_lang::Ty::Int))
                                    {
                                        self.error(
                                            file,
                                            &at,
                                            format!("`{sum}`: `{n}` is not a number variable or property"),
                                        );
                                    }
                                }
                            }
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

    /// A stat names a number or a flag. Only a number has a `max`, and a
    /// `max` that is a variable is a number variable. A variable shows once.
    fn stats(&mut self) {
        use loremaster_lang::Ty;
        use loremaster_model::Limit;
        let stats = self.story().stats.clone();
        let file = &self.file_of("story");
        let mut seen = BTreeSet::new();
        for (i, stat) in stats.iter().enumerate() {
            let at = format!("stats[{i}]");
            if !seen.insert(stat.var.as_str()) {
                let var = &stat.var;
                self.error(file, &at, format!("`{var}` is in `stats` twice"));
            }
            let ty = self.ty_of(&stat.var);
            if !matches!(ty, Some(Ty::Int | Ty::Bool)) {
                let var = &stat.var;
                self.error(
                    file,
                    &at,
                    format!("`{var}` is not a number or a true/false value"),
                );
            }
            match &stat.max {
                Some(_) if ty == Some(Ty::Bool) => {
                    self.error(file, &at, "a true/false stat can not have `max`");
                }
                Some(Limit::Var(max)) if self.ty_of(max.as_str()) != Some(Ty::Int) => {
                    self.error(file, &at, format!("`{max}` is not a number variable"));
                }
                _ => {}
            }
        }
    }

    /// `export:` and `import:` (TX-12): each fact is a variable or a thing
    /// property, each relation a thing and a declared axis, and each memory
    /// a thing and a key. An exported memory needs a dialogue that gives it,
    /// or an import; an imported one declares its key.
    fn episodes(&mut self) {
        let story = self.story().clone();
        let file = &self.file_of("story");
        for (list, items) in [("export", &story.export), ("import", &story.import)] {
            for (i, name) in items.facts.iter().enumerate() {
                use loremaster_lang::Ty;
                if !matches!(
                    self.ty_of(name),
                    Some(Ty::Bool | Ty::Int | Ty::Text | Ty::Enum(_))
                ) {
                    self.error(
                        file,
                        &format!("{list}.facts[{i}]"),
                        format!("`{name}` is not a story variable or a thing property"),
                    );
                }
            }
            for (i, name) in items.relations.iter().enumerate() {
                let at = format!("{list}.relations[{i}]");
                match name.split_once('.') {
                    Some((thing, axis)) if story.thing(thing).is_some() => {
                        if !self.scope.relations.contains(axis) {
                            self.error(file, &at, format!("unknown relation axis `{axis}`"));
                        }
                    }
                    Some((thing, _)) => self.error(file, &at, format!("unknown thing `{thing}`")),
                    None => self.error(file, &at, format!("`{name}` is not `thing.axis`")),
                }
            }
            for (i, name) in items.memories.iter().enumerate() {
                let at = format!("{list}.memories[{i}]");
                match name.split_once('.') {
                    Some((thing, key)) if story.thing(thing).is_some() => {
                        if !self.scope.memories.contains(key) {
                            self.error(file, &at, format!("no dialogue remembers `{key}`"));
                        }
                    }
                    Some((thing, _)) => self.error(file, &at, format!("unknown thing `{thing}`")),
                    None => self.error(file, &at, format!("`{name}` is not `thing.key`")),
                }
            }
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
            // Quests (D8): journal text per state, only on a quest.
            let journals = m.states.iter().filter(|s| s.journal().is_some()).count();
            if m.quest && journals == 0 {
                self.warning(
                    &file,
                    &at,
                    "the quest has no state with `journal:` text, so it never shows in the journal",
                );
            }
            for s in &m.states {
                let place = format!("{at}.states.{}", s.name());
                if !m.quest && (s.journal().is_some() || s.done()) {
                    self.warning(
                        &file,
                        &place,
                        "`journal` and `done` work only on a machine with `quest: true`",
                    );
                }
                if let Some(line) = s.journal() {
                    self.line_without_pool(&file, &format!("{place}.journal"), line);
                } else if m.quest && s.done() {
                    self.warning(
                        &file,
                        &place,
                        "`done: true` without `journal:` text has no effect: the quest leaves the journal in this state",
                    );
                }
            }
            if !m.quest && m.title.is_some() {
                self.warning(
                    &file,
                    &at,
                    "`title` works only on a machine with `quest: true`",
                );
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
            if let Some(state) = endless_loop(m) {
                self.error(
                    &file,
                    &at,
                    format!("the `on: next` steps from the state `{state}` loop forever: give a step `when:`, or an `await`, `end` or `stop`"),
                );
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

    /// The slots of a Command record: a known verb, things (or rooms) as its
    /// objects, and a known actor. For `go`, the direct object is a
    /// direction.
    fn command(
        &mut self,
        file: &str,
        at: &str,
        pattern: &loremaster_model::CommandPattern,
        verbs: &BTreeSet<String>,
    ) {
        let verb = pattern.verb.as_str();
        if verb.is_empty() {
            self.error(file, at, "name the verb");
        } else if !verbs.contains(verb) {
            self.error(file, at, format!("unknown verb `{verb}`"));
        }
        if pattern.indirect.is_some() && pattern.direct.is_none() {
            // A command has its objects in order; a rule could never match it.
            self.error(
                file,
                &format!("{at}.indirect"),
                "`indirect` needs a `direct` object: the indirect one is the second",
            );
        }
        for (i, o) in pattern.objects().enumerate() {
            let direction = verb == "go" && i == 0;
            if !direction
                && self.story().thing(o.as_str()).is_none()
                && self.story().room(o.as_str()).is_none()
            {
                self.error(file, at, format!("unknown thing `{o}`"));
            }
        }
        if let Some(actor) = &pattern.actor
            && actor.as_str() != PLAYER_SPEAKER
            && self.story().agenda_of(actor.as_str()).is_none()
        {
            let message = if self.story().thing(actor.as_str()).is_some() {
                format!("`{actor}` is a thing, but not an actor: give it an agenda in agendas/")
            } else {
                format!("unknown actor `{actor}`: an actor is `player` or a thing with an agenda")
            };
            self.error(file, &format!("{at}.actor"), message);
        }
    }

    /// Agendas (D6): the actor is a thing with one agenda, each step's
    /// place, condition, command and effects check, and `closes` names
    /// endings, with a `reason`.
    fn agendas(&mut self) {
        let agendas = self.story().agendas.clone();
        let verbs: BTreeSet<String> = standard_verbs()
            .iter()
            .chain(self.story().verbs.iter())
            .map(|v| v.id.to_string())
            .collect();
        let mut seen = BTreeSet::new();
        let mut actors = BTreeSet::new();
        for a in &agendas {
            let file = self.file_of(&format!("agenda:{}", a.id));
            if !seen.insert(a.id.clone()) {
                self.error(&file, "id", format!("agenda `{}` is defined twice", a.id));
            }
            if a.actor.as_str() == PLAYER_SPEAKER {
                self.error(
                    &file,
                    "actor",
                    "the player acts by commands, not by an agenda: name a thing",
                );
            } else if self.story().thing(a.actor.as_str()).is_none() {
                self.error(
                    &file,
                    "actor",
                    format!("unknown thing `{}`: an actor is a thing", a.actor),
                );
            }
            if !actors.insert(a.actor.clone()) {
                self.error(
                    &file,
                    "actor",
                    format!("`{}` has two agendas; an actor has one", a.actor),
                );
            }
            let mut keys = BTreeSet::new();
            for (i, step) in a.steps.iter().enumerate() {
                let at = format!("steps[{i}]");
                let key = step.key(i);
                if !keys.insert(key.clone()) {
                    self.error(&file, &at, format!("two steps have the key `{key}`"));
                }
                if let Some(place) = &step.place {
                    self.room_ref(&file, &format!("{at}.place"), place.as_str());
                }
                if let Some(when) = &step.when {
                    self.condition(&file, &format!("{at}.when"), when);
                }
                if let Some(cmd) = &step.action {
                    let at = format!("{at}.do");
                    if cmd.actor.is_some() {
                        self.error(
                            &file,
                            &format!("{at}.actor"),
                            "the agenda's actor runs the command; leave `actor` out",
                        );
                    }
                    // The actor is reported above; check the other slots.
                    let mut cmd = cmd.clone();
                    cmd.actor = None;
                    self.command(&file, &at, &cmd, &verbs);
                }
                self.effects(&file, &format!("{at}.then"), &step.then);
                self.no_await(&file, &format!("{at}.then"), &step.then);
                let name = format!("step `{key}` of agenda `{}`", a.id);
                self.closes(&file, &format!("{at}."), &name, &step.closes, &step.reason);
                if step.action.is_none() && step.then.is_empty() && step.place.is_none() {
                    self.warning(
                        &file,
                        &at,
                        "the step does nothing: give it `place`, `do` or `then`",
                    );
                }
            }
        }
    }

    /// A combat names enemies with int `skill` and `stamina`, and rooms. The
    /// player has `skill` and `stamina`; `luck` is optional, and a number.
    fn combat_spec(&mut self, file: &str, at: &str, spec: &loremaster_model::CombatSpec) {
        use loremaster_lang::Ty;
        if spec.enemies.is_empty() {
            self.error(file, at, "a `combat` needs at least one enemy");
        }
        for (i, e) in spec.enemies.iter().enumerate() {
            if spec.enemies[..i].contains(e) {
                self.error(file, at, format!("the enemy `{e}` is named twice"));
            }
            self.thing_ref(file, at, e.as_str());
            for stat in ["skill", "stamina"] {
                if !matches!(self.ty_of(&format!("{e}.{stat}")), Some(Ty::Int)) {
                    self.error(file, at, format!("the enemy `{e}` needs a number `{stat}`"));
                }
            }
        }
        for room in [Some(&spec.win), Some(&spec.loss), spec.escape.as_ref()]
            .into_iter()
            .flatten()
        {
            self.room_ref(file, at, room.as_str());
        }
        for stat in ["skill", "stamina"] {
            if !matches!(self.ty_of(stat), Some(Ty::Int)) {
                self.error(
                    file,
                    at,
                    format!("`combat` needs a number variable `{stat}`"),
                );
            }
        }
        // With a higher floor, `stamina` never reaches 0 and the combat has no loss.
        let floor = self.story().var("stamina").and_then(|v| v.min.clone());
        if !matches!(floor, None | Some(loremaster_model::Limit::Value(..=0))) {
            self.error(file, at, "`combat` needs a `min` of `stamina` of 0 or less");
        }
        if self.ty_of("luck").is_some_and(|t| t != Ty::Int) {
            self.error(file, at, "`luck` is not a number variable");
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
        if let Some(var) = &spec.against {
            if spec.roll.is_none() {
                self.error(file, at, "`against` belongs to a `roll`");
            }
            if self.ty_of(var) != Some(Ty::Int) {
                self.error(
                    file,
                    at,
                    format!("`against: {var}` is not a number variable"),
                );
            }
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
            if matches!(e, Effect::Await(_) | Effect::Combat(_)) {
                self.error(
                    file,
                    &format!("{at}[{i}]"),
                    "`await` and `combat` work only in rules",
                );
            }
        }
    }

    /// `min` and `max` of a variable: only on an `int`, a number or another
    /// `int` variable, and a constant `min` not above a constant `max`.
    fn limits(&mut self, file: &str, v: &loremaster_model::VarDef) {
        use loremaster_model::{Limit, VarType};
        for (key, limit) in [("min", &v.min), ("max", &v.max)] {
            let Some(limit) = limit else { continue };
            let at = format!("vars.{}.{key}", v.id);
            if v.ty != VarType::Int {
                self.error(file, &at, format!("`{key}` needs an `int` variable"));
            } else if let Limit::Var(other) = limit
                && self
                    .story()
                    .var(other.as_str())
                    .is_none_or(|o| o.ty != VarType::Int)
            {
                self.error(
                    file,
                    &at,
                    format!("`{key}: {other}` names no `int` variable"),
                );
            }
        }
        if let (Some(Limit::Value(min)), Some(Limit::Value(max))) = (&v.min, &v.max)
            && min > max
        {
            self.error(
                file,
                &format!("vars.{}.min", v.id),
                format!("`min` {min} is above `max` {max}"),
            );
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

fn not_yet(field: &str) -> String {
    format!("`{field}` is in the dialogue shape, but loremaster's talks do not play it yet")
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
        d.nodes.iter().flat_map(|n| {
            std::iter::once(&n.say).chain(n.options.iter().filter_map(|o| o.say.as_ref()))
        })
    });
    effect_lines
        .chain(dialogue_lines)
        .any(|l| l.segments.iter().any(LineSegment::is_speech))
}

/// The effects of all rules (machine steps included), timers, agendas and dialogues.
fn all_effects(story: &Story) -> impl Iterator<Item = &Effect> {
    let rule_effects = story.rules.iter().flat_map(|r| r.then.iter());
    let timer_effects = story.timers.iter().flat_map(|t| t.at.values().flatten());
    let agenda_effects = story
        .agendas
        .iter()
        .flat_map(|a| a.steps.iter().flat_map(|s| s.then.iter()));
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
        .chain(agenda_effects)
        .chain(dialogue_effects)
}

fn line_col(text: &str, pos: usize) -> (u32, u32) {
    let before = &text[..pos.min(text.len())];
    let line = before.matches('\n').count() as u32 + 1;
    let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) as u32 + 1;
    (line, col)
}

/// A state on a loop of `on: next` steps that never ends: in each state of
/// it, the first `on: next` step has no `when`, no `await` and no `end`, so
/// it always runs.
fn endless_loop(m: &loremaster_model::Machine) -> Option<String> {
    let always = |state: &str| -> Option<&str> {
        let step = m
            .steps
            .iter()
            .find(|s| s.on == EventPattern::Next && s.from.list().contains(&state))?;
        let stops = step.when.is_some()
            || step.then.iter().any(|e| {
                matches!(
                    e,
                    Effect::Await(_) | Effect::Combat(_) | Effect::End(_) | Effect::Stop
                )
            });
        (!stops).then_some(step.to.as_str())
    };
    // A state that no step reaches can not loop; the checks warn about it.
    let mut reached = vec![m.start.clone()];
    let mut i = 0;
    while i < reached.len() {
        for step in &m.steps {
            if step.from.list().contains(&reached[i].as_str()) && !reached.contains(&step.to) {
                reached.push(step.to.clone());
            }
        }
        i += 1;
    }
    for start in &reached {
        let mut seen = vec![start.as_str()];
        let mut at = start.as_str();
        while let Some(to) = always(at) {
            if seen.contains(&to) {
                return Some(to.to_string());
            }
            seen.push(to);
            at = to;
        }
    }
    None
}
