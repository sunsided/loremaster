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
        self.shared_ids(&walks);
    }

    /// The hint ladder has one step per id across the walkthroughs that are
    /// not pending, the first in file order. A later step with the same id
    /// and another command, room, hints or `needs` would never get its own.
    /// The needs are the chart's, as the ladder reads them: with the
    /// `before` of earlier steps, in any order. A step without `at` gets its
    /// room from the replay, which the checks do not run, so a room is
    /// compared only when both steps give one.
    fn shared_ids(&mut self, walks: &[(String, Walkthrough)]) {
        type First<'a> = (
            &'a str,
            &'a str,
            &'a loremaster_model::StepPlan,
            BTreeSet<Id>,
        );
        let mut first: BTreeMap<&Id, First> = BTreeMap::new();
        let mut found = Vec::new();
        for (key, walk) in walks.iter().filter(|(_, w)| w.pending.is_none()) {
            let chart = step_chart(key, walk);
            for (i, step) in walk.steps.iter().enumerate() {
                let (Some(id), Some(plan)) = (step.id(), step.plan()) else {
                    continue;
                };
                let needs: BTreeSet<Id> = chart
                    .as_ref()
                    .and_then(|c| c.steps.iter().find(|s| s.index == i))
                    .map_or_else(
                        || plan.needs.iter().cloned().collect(),
                        |s| s.needs.iter().cloned().collect(),
                    );
                let Some((other, command, was, was_needs)) = first.get(id) else {
                    first.insert(id, (key.as_str(), step.command(), plan, needs));
                    continue;
                };
                let (other, command, was) = (*other, *command, *was);
                if other == key {
                    continue;
                }
                let differs = if command != step.command() {
                    format!("with `{command}`")
                } else if was.at.is_some() && plan.at.is_some() && was.at != plan.at {
                    "in another room".to_string()
                } else if was.hints != plan.hints {
                    "with other hints".to_string()
                } else if *was_needs != needs {
                    "with other `needs`".to_string()
                } else {
                    continue;
                };
                found.push((key.clone(), i, format!(
                    "the step id `{id}` is also in {other} {differs}: the hint ladder keeps only the first, so give this step another id"
                )));
            }
        }
        for (key, i, message) in found {
            self.at_step(&key, i, Severity::Warning, message);
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
            if let Some(room) = &plan.at
                && self.story().room(room.as_str()).is_none()
            {
                self.at_step(key, i, Severity::Error, format!("unknown room `{room}`"));
            }
            if (!plan.needs.is_empty() || !plan.before.is_empty()) && plan.id.is_none() {
                self.at_step(
                    key,
                    i,
                    Severity::Warning,
                    "a step with `needs` or `before` needs an `id` to be in the chart",
                );
            }
            for n in &plan.needs {
                match first.get(n) {
                    None => self.at_step(
                        key,
                        i,
                        Severity::Error,
                        format!("`needs` names `{n}`, which is no step of this walkthrough"),
                    ),
                    Some(&p) if p >= i => self.at_step(
                        key,
                        i,
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
                        Severity::Error,
                        format!("`before` names `{b}`, which is no step of this walkthrough"),
                    ),
                    Some(&p) if p <= i => self.at_step(
                        key,
                        i,
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
        // What the player has or sees from the start is no spoiler: the
        // things carried and the things in the start room.
        let mut seen: BTreeSet<&Id> = story
            .things
            .iter()
            .filter(|t| match &t.location {
                loremaster_model::Location::Carried => true,
                loremaster_model::Location::Room(r) => *r == story.start,
                _ => false,
            })
            .map(|t| &t.id)
            .collect();
        let mut found = Vec::new();
        for (i, step) in walk.steps.iter().enumerate() {
            seen.extend(named[i].iter().copied());
            let Some(plan) = step.plan() else { continue };
            for (h, hint) in plan.hints.iter().enumerate() {
                if let Some(what) = later_name(walk, i, hint, &nouns, &named, &seen) {
                    found.push((i, h, what));
                }
            }
        }
        for (i, h, what) in found {
            self.at_step(
                key,
                i,
                Severity::Warning,
                format!("hint {} names {what}: it gives away a later step", h + 1),
            );
        }
    }

    /// A diagnostic at step `i` of a walkthrough. In a YAML file the line is
    /// the one of the step (see [`step_position`]); a walkthrough from Rust
    /// has its place in its key.
    fn at_step(&mut self, key: &str, i: usize, severity: Severity, message: impl Into<String>) {
        let at = format!("steps[{i}]");
        let mut d = match severity {
            Severity::Error => Diagnostic::error(key, at, message),
            _ => Diagnostic::warning(key, at, message),
        };
        if let Some(f) = self.project.file(key) {
            if let Some(pos) = step_position(self.project, key, &f.text, i) {
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

/// Where step `i` starts in the YAML text: the `i`-th item of the block
/// list after `steps:`. A flow list (`steps: [..]`) has no lines to count:
/// there each step is looked for after the one before it, by `id: <id>` or
/// else by its command as a word.
fn step_position(project: &crate::Project, key: &str, text: &str, i: usize) -> Option<usize> {
    let (_, walk) = project.walkthroughs.iter().find(|(k, _)| k == key)?;
    let mut from = text.find("steps:")?;
    let items = list_items(text, from);
    if items.len() == walk.steps.len() {
        return items.get(i).copied();
    }
    let mut at = None;
    for step in walk.steps.iter().take(i + 1) {
        let found = match step.id() {
            Some(id) => text[from..].find(&format!("id: {id}")).map(|p| p + from),
            None => find_word(text, step.command(), from),
        };
        // A step that is not found leaves the search where it was.
        if let Some(p) = found {
            from = p + 1;
            at = Some(p);
        } else {
            at = None;
        }
    }
    at
}

/// The items of the block list on the lines after `from`: the places after
/// the `- ` of each line at the indentation of the first item. Comments,
/// blank lines and the deeper lines of an item are skipped.
fn list_items(text: &str, from: usize) -> Vec<usize> {
    let mut items = Vec::new();
    let Some(eol) = text[from..].find('\n') else {
        return items;
    };
    let mut offset = from + eol + 1;
    let mut indent = None;
    for line in text[offset..].split_inclusive('\n') {
        let here = offset;
        offset += line.len();
        let trimmed = line.trim_start_matches(' ');
        let body = trimmed.trim_end();
        if body.is_empty() || body.starts_with('#') {
            continue;
        }
        let n = line.len() - trimmed.len();
        // YAML separates the `-` from its item with a space or a tab.
        let item = body == "-"
            || body
                .strip_prefix('-')
                .is_some_and(|r| r.starts_with([' ', '\t']));
        match indent {
            None if item => indent = Some(n),
            None => break,
            Some(i) if n < i || (n == i && !item) => break,
            Some(i) if n > i => continue,
            _ => {}
        }
        items.push(here + n + body.len().min(2));
    }
    items
}

/// The first place at or after `from` where `text` has `word` as a whole word.
fn find_word(text: &str, word: &str, from: usize) -> Option<usize> {
    let edge = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric() && c != '_');
    text[from..]
        .match_indices(word)
        .map(|(p, _)| p + from)
        .find(|&at| {
            edge(text[..at].chars().next_back()) && edge(text[at + word.len()..].chars().next())
        })
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
    fn the_line_is_the_one_of_the_step_also_for_a_repeated_command() {
        let d: Vec<_> = check_files(files("steps:\n  - wait\n  - { do: wait, at: attic }\n"))
            .diagnostics
            .into_iter()
            .filter(|d| d.file == "tests/win.yaml")
            .collect();
        check!(d.len() == 1, "{d:?}");
        check!(d[0].at == "steps[1]");
        check!(d[0].line == Some(3), "{d:?}");
    }

    #[test]
    fn the_line_is_the_one_of_the_step_also_after_a_hint_with_its_command() {
        let d: Vec<_> = check_files(files(
            "steps:\n  - { id: a, do: open drawer, hints: [\"Then wait.\"] }\n  - { do: wait, at: attic }\n",
        ))
        .diagnostics
        .into_iter()
        .filter(|d| d.file == "tests/win.yaml")
        .collect();
        check!(d.len() == 1, "{d:?}");
        check!(d[0].line == Some(3), "{d:?}");
    }

    #[test]
    fn a_tab_after_the_dash_starts_an_item_too() {
        let text = "steps:\n  -\t{ do: wait }\n  - look\n";
        check!(list_items(text, 0) == vec![11, 28]);
    }

    #[test]
    fn a_thing_of_the_start_room_is_no_later_step() {
        // The drawer is in the start room, so a hint may name it.
        let m = messages(
            "steps:\n\
             - { id: look, do: look, hints: [\"Look at the drawer.\"] }\n\
             - { id: open, do: open drawer }\n",
        );
        check!(m.is_empty(), "{m:?}");
    }

    #[test]
    fn a_step_id_in_two_walkthroughs_with_other_commands_is_a_warning() {
        let mut f = files("steps:\n  - { id: d, do: open drawer }\n");
        f.push(SourceFile {
            path: "tests/zz.yaml".into(),
            text: "steps:\n  - { id: d, do: look }\n".into(),
        });
        let m: Vec<String> = check_files(f)
            .diagnostics
            .into_iter()
            .filter(|d| d.file == "tests/zz.yaml")
            .map(|d| d.message)
            .collect();
        check!(m.len() == 1, "{m:?}");
        check!(m[0].contains("is also in tests/win.yaml"), "{m:?}");
    }

    /// The diagnostics of tests/zz.yaml, next to tests/win.yaml.
    fn shared(win: &str, zz: &str) -> Vec<String> {
        let mut f = files(win);
        f.push(SourceFile {
            path: "tests/zz.yaml".into(),
            text: zz.into(),
        });
        check_files(f)
            .diagnostics
            .into_iter()
            .filter(|d| d.file == "tests/zz.yaml")
            .map(|d| d.message)
            .collect()
    }

    #[test]
    fn a_shared_step_id_with_other_hints_or_needs_is_a_warning() {
        let m = shared(
            "steps:\n  - { id: d, do: open drawer }\n",
            "steps:\n  - { id: d, do: open drawer, hints: [\"Look.\"] }\n",
        );
        check!(m.len() == 1, "{m:?}");
        check!(m[0].contains("other hints"), "{m:?}");
        let m = shared(
            "steps:\n  - { id: s, do: s }\n  - { id: d, do: open drawer }\n",
            "steps:\n  - { id: s, do: s }\n  - { id: d, do: open drawer, needs: s }\n",
        );
        check!(m.len() == 1, "{m:?}");
        check!(m[0].contains("other `needs`"), "{m:?}");
        // The ladder reads the chart: `before` of an earlier step is a need.
        let m = shared(
            "steps:\n  - { id: s, do: s }\n  - { id: d, do: open drawer }\n",
            "steps:\n  - { id: s, do: s, before: [d] }\n  - { id: d, do: open drawer }\n",
        );
        check!(m.len() == 1, "{m:?}");
        check!(m[0].contains("other `needs`"), "{m:?}");
        // The order of `needs` does not matter.
        let m = shared(
            "steps:\n  - { id: s, do: s }\n  - { id: n, do: n }\n  - { id: d, do: open drawer, needs: [s, n] }\n",
            "steps:\n  - { id: s, do: s }\n  - { id: n, do: n }\n  - { id: d, do: open drawer, needs: [n, s] }\n",
        );
        check!(m.is_empty(), "{m:?}");
    }

    #[test]
    fn a_shared_step_id_with_one_room_given_is_no_warning() {
        // The replay finds the room of the step without `at`.
        let m = shared(
            "steps:\n  - { id: d, do: open drawer }\n",
            "steps:\n  - { id: d, do: open drawer, at: cottage }\n",
        );
        check!(m.is_empty(), "{m:?}");
    }

    #[test]
    fn mentions_whole_words_only() {
        check!(mentions("take the lens", "lens"));
        check!(!mentions("take the lenses", "lens"));
        check!(!mentions("a_lens", "lens"));
    }
}
