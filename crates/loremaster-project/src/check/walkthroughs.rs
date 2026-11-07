//! The checks of walkthrough plans (CK-4, CK-6, CK-7): step ids, `at`,
//! `needs` and `before`, at most three hints, hints that name a later step,
//! and long runs of steps in series only.

use super::{Checker, line_col};
use crate::{Diagnostic, Severity};
use loremaster_model::{Id, SERIAL_RUN, Story, Walkthrough, step_chart};
use std::collections::{BTreeMap, BTreeSet};

/// The most hints a step can have (CK-4). After them, the ladder shows the
/// command.
pub const MAX_HINTS: usize = 3;

impl Checker<'_> {
    pub(super) fn walkthroughs(&mut self) {
        let walks = self.project.walkthroughs.clone();
        for (key, walk) in &walks {
            self.walkthrough(key, walk);
        }
    }

    fn walkthrough(&mut self, key: &str, walk: &Walkthrough) {
        let mut position: BTreeMap<&Id, usize> = BTreeMap::new();
        for (i, step) in walk.steps.iter().enumerate() {
            if let Some(id) = step.id()
                && position.insert(id, i).is_some()
            {
                self.at_step(
                    key,
                    i,
                    Some(id.as_str()),
                    Severity::Error,
                    format!("two steps have the id `{id}`"),
                );
            }
        }
        // The first step of each id, as `needs` sees it.
        let mut first: BTreeMap<&Id, usize> = BTreeMap::new();
        for (i, step) in walk.steps.iter().enumerate() {
            if let Some(id) = step.id() {
                first.entry(id).or_insert(i);
            }
        }
        for (i, step) in walk.steps.iter().enumerate() {
            let Some(plan) = step.plan() else { continue };
            let anchor = plan.id.as_ref().map(Id::as_str);
            if let Some(room) = &plan.at
                && self.story().room(room.as_str()).is_none()
            {
                self.at_step(
                    key,
                    i,
                    anchor,
                    Severity::Error,
                    format!("unknown room `{room}`"),
                );
            }
            if (!plan.needs.is_empty() || !plan.before.is_empty()) && plan.id.is_none() {
                self.at_step(
                    key,
                    i,
                    anchor,
                    Severity::Warning,
                    "a step with `needs` or `before` needs an `id` to be in the chart",
                );
            }
            for n in &plan.needs {
                match first.get(n) {
                    None => self.at_step(
                        key,
                        i,
                        anchor,
                        Severity::Error,
                        format!("`needs` names `{n}`, which is no step of this walkthrough"),
                    ),
                    Some(&p) if p >= i => self.at_step(
                        key,
                        i,
                        anchor,
                        Severity::Error,
                        format!("`needs` names `{n}`, which comes later in the walkthrough"),
                    ),
                    Some(_) => {}
                }
            }
            for b in &plan.before {
                match first.get(b) {
                    None => self.at_step(
                        key,
                        i,
                        anchor,
                        Severity::Error,
                        format!("`before` names `{b}`, which is no step of this walkthrough"),
                    ),
                    Some(&p) if p <= i => self.at_step(
                        key,
                        i,
                        anchor,
                        Severity::Error,
                        format!("`before` names `{b}`, which comes earlier in the walkthrough"),
                    ),
                    Some(_) => {}
                }
            }
            if plan.hints.len() > MAX_HINTS {
                self.at_step(
                    key,
                    i,
                    anchor,
                    Severity::Error,
                    format!(
                        "a step has at most {MAX_HINTS} hints, this one has {}: the last level of the ladder is the command",
                        plan.hints.len()
                    ),
                );
            }
        }
        self.later_hints(key, walk);
        if let Some(chart) = step_chart(key, walk) {
            for (from, to) in chart.serial_runs(SERIAL_RUN) {
                let (a, b) = (&chart.steps[from], &chart.steps[to]);
                self.at_step(
                    key,
                    a.index,
                    Some(a.id.as_str()),
                    Severity::Warning,
                    format!(
                        "{} steps in a row, from `{}` to `{}`, can only be done one after the other (width 1): give the player a second puzzle to work on",
                        to - from + 1,
                        a.id,
                        b.id
                    ),
                );
            }
        }
    }

    /// CK-6: a hint must not name what only a later step brings up: the id
    /// or the command of a later step, or a thing that only later steps name.
    fn later_hints(&mut self, key: &str, walk: &Walkthrough) {
        let story = self.project.story.clone();
        let nouns = thing_nouns(&story);
        let named: Vec<BTreeSet<&Id>> = walk
            .steps
            .iter()
            .map(|s| things_in(&nouns, objects(s.command())))
            .collect();
        let mut seen: BTreeSet<&Id> = BTreeSet::new();
        let mut found = Vec::new();
        for (i, step) in walk.steps.iter().enumerate() {
            seen.extend(named[i].iter().copied());
            let Some(plan) = step.plan() else { continue };
            for (h, hint) in plan.hints.iter().enumerate() {
                if let Some(what) = later_name(walk, i, hint, &nouns, &named, &seen) {
                    found.push((i, plan.id.as_ref().map(Id::as_str), h, what));
                }
            }
        }
        for (i, anchor, h, what) in found {
            self.at_step(
                key,
                i,
                anchor,
                Severity::Warning,
                format!("hint {} names {what}: it gives away a later step", h + 1),
            );
        }
    }

    /// A diagnostic at step `i` of a walkthrough. In a YAML file the line is
    /// the one of the step's id (or command); a walkthrough from Rust has its
    /// place in its key.
    fn at_step(
        &mut self,
        key: &str,
        i: usize,
        anchor: Option<&str>,
        severity: Severity,
        message: impl Into<String>,
    ) {
        let at = format!("steps[{i}]");
        let mut d = match severity {
            Severity::Error => Diagnostic::error(key, at, message),
            _ => Diagnostic::warning(key, at, message),
        };
        if let Some(f) = self.project.file(key) {
            let needle = anchor
                .map(|id| format!("id: {id}"))
                .unwrap_or_else(|| step_command(self.project, key, i));
            if let Some(pos) = f.text.find(&needle) {
                let (line, column) = line_col(&f.text, pos);
                d.line = Some(line);
                d.column = Some(column);
            }
        } else if let Some((file, line, column)) = rust_place(key) {
            d.file = file;
            d.line = Some(line);
            d.column = Some(column);
        }
        self.out.push(d);
    }
}

fn step_command(project: &crate::Project, key: &str, i: usize) -> String {
    project
        .walkthroughs
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, w)| w.steps.get(i))
        .map(|s| s.command().to_string())
        .unwrap_or_default()
}

/// `file:line:column` (maybe with `#n`) of a walkthrough from Rust.
fn rust_place(key: &str) -> Option<(String, u32, u32)> {
    let key = key.split('#').next()?;
    let mut parts = key.rsplitn(3, ':');
    let column = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
    Some((parts.next()?.to_string(), line, column))
}

/// The nouns of each thing: its name and its aliases, in lower case.
fn thing_nouns(story: &Story) -> Vec<(&Id, Vec<String>)> {
    story
        .things
        .iter()
        .map(|t| {
            let mut nouns: Vec<String> = std::iter::once(&t.name)
                .chain(&t.aliases)
                .map(|n| n.to_lowercase())
                .filter(|n| !n.is_empty())
                .collect();
            nouns.dedup();
            (&t.id, nouns)
        })
        .collect()
}

/// The things whose name or an alias the text has, as whole words.
fn things_in<'a>(nouns: &[(&'a Id, Vec<String>)], text: &str) -> BTreeSet<&'a Id> {
    let text = text.to_lowercase();
    nouns
        .iter()
        .filter(|(_, ns)| ns.iter().any(|n| mentions(&text, n)))
        .map(|(id, _)| *id)
        .collect()
}

/// The command without its first word, the verb: `light lamp` names the
/// lamp, not a thing called "light".
fn objects(command: &str) -> &str {
    command.trim().split_once(' ').map_or("", |(_, rest)| rest)
}

/// What a hint of step `i` names of a later step, if anything.
fn later_name(
    walk: &Walkthrough,
    i: usize,
    hint: &str,
    nouns: &[(&Id, Vec<String>)],
    named: &[BTreeSet<&Id>],
    seen: &BTreeSet<&Id>,
) -> Option<String> {
    let text = hint.to_lowercase();
    let own = walk.steps[i].command().to_lowercase();
    for (j, later) in walk.steps.iter().enumerate().skip(i + 1) {
        if let Some(id) = later.id()
            && mentions(&text, &id.as_str().to_lowercase())
        {
            return Some(format!("the later step `{id}`"));
        }
        let command = later.command().to_lowercase();
        // A short command such as `n` or `up` reads as a plain word.
        if command.contains(' ') && command != own && mentions(&text, &command) {
            return Some(format!(
                "the command of the later step {} (`{command}`)",
                j + 1
            ));
        }
        for id in &named[j] {
            if seen.contains(id) {
                continue;
            }
            let (_, ns) = nouns.iter().find(|(t, _)| t == id)?;
            if let Some(n) = ns.iter().find(|n| mentions(&text, n)) {
                return Some(format!(
                    "\"{n}\", which only step {} (`{}`) brings up",
                    j + 1,
                    later.command()
                ));
            }
        }
    }
    None
}

/// `text` has `word` with no letter or digit right before or after it.
pub(crate) fn mentions(text: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let edge = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric() && c != '_');
    text.match_indices(word).any(|(at, _)| {
        edge(text[..at].chars().next_back()) && edge(text[at + word.len()..].chars().next())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourceFile, check_files};
    use assert2::check;

    fn files(test: &str) -> Vec<SourceFile> {
        [
            ("story.yaml", "title: T\nstart: cottage\n"),
            (
                "rooms/cottage.yaml",
                "name: Cottage\ndescription: [\"A room.\"]\nexits: [{ direction: south, to: shore }]\n",
            ),
            (
                "rooms/shore.yaml",
                "name: Shore\ndescription: [\"Sand.\"]\nexits: [{ direction: north, to: cottage }]\n",
            ),
            (
                "things/drawer.yaml",
                "name: drawer\nlocation: { room: cottage }\n",
            ),
            (
                "things/lens.yaml",
                "name: great lens\naliases: [lens]\nlocation: { room: shore }\n",
            ),
            ("tests/win.yaml", test),
        ]
        .into_iter()
        .map(|(path, text)| SourceFile {
            path: path.into(),
            text: text.into(),
        })
        .collect()
    }

    fn messages(test: &str) -> Vec<String> {
        check_files(files(test))
            .diagnostics
            .into_iter()
            .filter(|d| d.file == "tests/win.yaml")
            .map(|d| format!("{:?} {}: {}", d.severity, d.at, d.message))
            .collect()
    }

    #[test]
    fn a_good_plan_has_no_diagnostics() {
        let m = messages(
            "steps:\n\
             - { id: drawer, do: open drawer, at: cottage, hints: [\"Look around the cottage.\"] }\n\
             - s\n\
             - { id: lens, do: examine lens, needs: drawer }\n",
        );
        check!(m.is_empty(), "{m:?}");
    }

    #[test]
    fn needs_before_at_and_hints_are_checked() {
        let m = messages(
            "steps:\n\
             - { id: a, do: open drawer, at: attic, needs: b, hints: [x, y, z, w] }\n\
             - { id: b, do: wait, before: [a, c] }\n\
             - { id: b, do: look }\n",
        );
        let all = m.join("\n");
        check!(all.contains("unknown room `attic`"), "{all}");
        check!(
            all.contains("`needs` names `b`, which comes later"),
            "{all}"
        );
        check!(
            all.contains("`before` names `a`, which comes earlier"),
            "{all}"
        );
        check!(
            all.contains("`before` names `c`, which is no step"),
            "{all}"
        );
        check!(all.contains("at most 3 hints"), "{all}");
        check!(all.contains("two steps have the id `b`"), "{all}");
    }

    #[test]
    fn a_hint_that_names_a_later_step_is_a_warning() {
        let m = messages(
            "steps:\n\
             - { id: drawer, do: open drawer, hints: [\"Then look at the lens.\"] }\n\
             - s\n\
             - { id: seen, do: examine lens }\n",
        );
        check!(m.len() == 1, "{m:?}");
        check!(m[0].contains("hint 1 names \"lens\""), "{m:?}");
        let m = messages(
            "steps:\n\
             - { id: drawer, do: open drawer, hints: [\"Before the step lens_seen.\"] }\n\
             - { id: lens_seen, do: examine lens }\n",
        );
        check!(m[0].contains("the later step `lens_seen`"), "{m:?}");
    }

    #[test]
    fn a_long_run_in_series_is_a_warning() {
        let m = messages(
            "steps:\n\
             - { id: a, do: wait }\n\
             - { id: b, do: wait, needs: a }\n\
             - { id: c, do: wait, needs: b }\n\
             - { id: d, do: wait, needs: c }\n\
             - { id: e, do: wait, needs: d }\n",
        );
        check!(m.len() == 1, "{m:?}");
        check!(m[0].contains("5 steps in a row, from `a` to `e`"), "{m:?}");
    }

    #[test]
    fn mentions_whole_words_only() {
        check!(mentions("take the lens", "lens"));
        check!(!mentions("take the lenses", "lens"));
        check!(!mentions("a_lens", "lens"));
    }
}
