//! The quality report (CK-8, US-42): design checks from the 1984 checklist
//! that find what players trip over. They are not errors: a story can have a
//! one-way exit on purpose. The report says which checks it does not run yet,
//! and why.
//!
//! All checks here read the model and the walkthroughs; none plays the
//! story. The checks of endings over the explorer graph (dead ends, points of
//! no return; CK-1 to CK-3) are a separate report.

use crate::{Diagnostic, Project};
use loremaster_model::{Id, SERIAL_RUN, Story, VerbDef, step_chart};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A check of the quality report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QualityCheck {
    /// An exit from A to B, and no exit from B back to A.
    OneWayExit,
    /// Two rooms with the same description.
    SameRoomText,
    /// A portable thing that no rule, key, door, text and walkthrough uses.
    UnusedThing,
    /// Many steps in a row that the player can only do one after the other.
    PuzzleWidth,
    /// A verb of the story that no walkthrough uses.
    VerbCoverage,
    /// A death without a warning before it.
    NoWarningBeforeDeath,
    /// A death that the winning paths do not pass near.
    DeathOffWinningPath,
    /// A change in a far room without a signal to the player.
    FarChangeWithoutSignal,
    /// Gifts to characters that the story has no answer for.
    GiftCoverage,
}

impl QualityCheck {
    /// The name of the check, as in JSON: `one_way_exit`.
    pub fn name(self) -> &'static str {
        match self {
            QualityCheck::OneWayExit => "one_way_exit",
            QualityCheck::SameRoomText => "same_room_text",
            QualityCheck::UnusedThing => "unused_thing",
            QualityCheck::PuzzleWidth => "puzzle_width",
            QualityCheck::VerbCoverage => "verb_coverage",
            QualityCheck::NoWarningBeforeDeath => "no_warning_before_death",
            QualityCheck::DeathOffWinningPath => "death_off_winning_path",
            QualityCheck::FarChangeWithoutSignal => "far_change_without_signal",
            QualityCheck::GiftCoverage => "gift_coverage",
        }
    }
}

/// One finding: the check, and where and what.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub check: QualityCheck,
    pub diagnostic: Diagnostic,
}

/// The puzzle width of one walkthrough with step ids (US-43).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PuzzleWidth {
    pub walkthrough: String,
    pub name: String,
    /// How many steps have an id.
    pub steps: usize,
    pub max: usize,
    pub mean: f32,
}

/// Which verbs the walkthroughs use.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbCoverage {
    /// The verbs of the story (`verbs:` of `story.yaml`) that a walkthrough uses.
    pub story_used: Vec<Id>,
    /// The verbs of the story that no walkthrough uses.
    pub story_unused: Vec<Id>,
    /// The standard verbs that a walkthrough uses.
    pub standard_used: Vec<Id>,
}

/// A check that the report does not run yet, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deferred {
    pub check: QualityCheck,
    pub reason: String,
}

/// The quality report of a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityReport {
    pub findings: Vec<Finding>,
    pub widths: Vec<PuzzleWidth>,
    pub verbs: VerbCoverage,
    pub deferred: Vec<Deferred>,
}

/// The quality report of a loaded project.
pub fn quality(project: &Project) -> QualityReport {
    let mut findings = Vec::new();
    one_way_exits(project, &mut findings);
    same_room_text(project, &mut findings);
    unused_things(project, &mut findings);
    let widths = puzzle_widths(project, &mut findings);
    let verbs = verb_coverage(project, &mut findings);
    QualityReport {
        findings,
        widths,
        verbs,
        deferred: deferred(),
    }
}

fn finding(check: QualityCheck, diagnostic: Diagnostic) -> Finding {
    Finding { check, diagnostic }
}

fn file_of(project: &Project, key: &str) -> String {
    project
        .origins
        .get(key)
        .map(|o| o.file().to_string())
        .unwrap_or_else(|| "story.yaml".into())
}

fn one_way_exits(project: &Project, out: &mut Vec<Finding>) {
    let story = &project.story;
    for room in &story.rooms {
        for (i, exit) in room.exits.iter().enumerate() {
            let Some(to) = &exit.to else { continue };
            if to == &room.id {
                continue;
            }
            let back = story
                .room(to.as_str())
                .is_some_and(|r| r.exits.iter().any(|e| e.to.as_ref() == Some(&room.id)));
            if !back && story.room(to.as_str()).is_some() {
                out.push(finding(
                    QualityCheck::OneWayExit,
                    Diagnostic::warning(
                        file_of(project, &format!("room:{}", room.id)),
                        format!("exits[{i}]"),
                        format!(
                            "the exit {} leads to `{to}`, and no exit of `{to}` leads back: the player cannot return this way",
                            exit.direction.name()
                        ),
                    ),
                ));
            }
        }
    }
}

/// The text of a room as the author wrote it: all its fragments.
fn room_text(story: &Story, id: &Id) -> String {
    story
        .room(id.as_str())
        .map(|r| {
            r.description
                .iter()
                .flat_map(|f| f.variants.iter())
                .map(|v| v.trim())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn same_room_text(project: &Project, out: &mut Vec<Finding>) {
    let story = &project.story;
    let mut by_text: BTreeMap<String, Vec<&Id>> = BTreeMap::new();
    for room in &story.rooms {
        let text = room_text(story, &room.id);
        if !text.is_empty() {
            by_text.entry(text).or_default().push(&room.id);
        }
    }
    for rooms in by_text.values().filter(|r| r.len() > 1) {
        let names: Vec<String> = rooms.iter().map(|r| format!("`{r}`")).collect();
        for room in rooms {
            out.push(finding(
                QualityCheck::SameRoomText,
                Diagnostic::warning(
                    file_of(project, &format!("room:{room}")),
                    "description",
                    format!(
                        "the rooms {} have the same text: the player cannot tell them apart",
                        names.join(", ")
                    ),
                ),
            ));
        }
    }
}

/// The verbs that only move a thing around or look at it: a walkthrough
/// step with one of them does not use the thing in a puzzle.
const PLAIN_VERBS: &[&str] = &[
    "take",
    "drop",
    "examine",
    "look",
    "inventory",
    "put",
    "wait",
    "go",
];

fn unused_things(project: &Project, out: &mut Vec<Finding>) {
    let story = &project.story;
    let verbs = all_verbs(story);
    for thing in story.things.iter().filter(|t| t.portable && !t.red_herring) {
        // Everything in the story but the thing itself: rules, keys, doors,
        // texts, machines, dialogues and the other things.
        let mut rest = story.clone();
        rest.things.retain(|t| t.id != thing.id);
        let json = serde_json::to_string(&rest).unwrap_or_default();
        if crate::check::mentions(&json, thing.id.as_str()) {
            continue;
        }
        let nouns: Vec<String> = std::iter::once(&thing.name)
            .chain(&thing.aliases)
            .map(|n| n.to_lowercase())
            .collect();
        let used = project.walkthroughs.iter().any(|(_, w)| {
            w.steps.iter().any(|s| {
                let command = s.command().to_lowercase();
                let plain =
                    verb_of(&verbs, &command).is_some_and(|v| PLAIN_VERBS.contains(&v.id.as_str()));
                !plain && nouns.iter().any(|n| crate::check::mentions(&command, n))
            })
        });
        if used {
            continue;
        }
        out.push(finding(
            QualityCheck::UnusedThing,
            Diagnostic::warning(
                file_of(project, &format!("thing:{}", thing.id)),
                "id",
                format!(
                    "`{}` can be carried, but no rule, key, text or walkthrough uses it: give it a use, or mark it `red_herring: true`",
                    thing.id
                ),
            ),
        ));
    }
}

fn puzzle_widths(project: &Project, out: &mut Vec<Finding>) -> Vec<PuzzleWidth> {
    let mut widths = Vec::new();
    for (key, walk) in &project.walkthroughs {
        let Some(chart) = step_chart(key, walk) else {
            continue;
        };
        for (from, to) in chart.serial_runs(SERIAL_RUN) {
            out.push(finding(
                QualityCheck::PuzzleWidth,
                Diagnostic::warning(
                    key.clone(),
                    format!("steps[{}]", chart.steps[from].index),
                    format!(
                        "{} steps in a row, from `{}` to `{}`, have width 1: the game is in series only there",
                        to - from + 1,
                        chart.steps[from].id,
                        chart.steps[to].id
                    ),
                ),
            ));
        }
        widths.push(PuzzleWidth {
            walkthrough: key.clone(),
            name: chart.name.clone(),
            steps: chart.steps.len(),
            max: chart.max_width,
            mean: chart.mean_width(),
        });
    }
    widths
}

fn all_verbs(story: &Story) -> Vec<VerbDef> {
    let mut verbs = story.verbs.clone();
    verbs.extend(loremaster_model::standard_verbs());
    verbs
}

/// The verb of a command: the one with the longest word that starts it.
fn verb_of<'a>(verbs: &'a [VerbDef], command: &str) -> Option<&'a VerbDef> {
    let command = command.trim().to_lowercase();
    verbs
        .iter()
        .flat_map(|v| v.words.iter().map(move |w| (v, w.to_lowercase())))
        .filter(|(_, w)| {
            command == *w
                || command
                    .strip_prefix(w.as_str())
                    .is_some_and(|rest| rest.starts_with(' '))
        })
        .max_by_key(|(_, w)| w.len())
        .map(|(v, _)| v)
}

fn verb_coverage(project: &Project, out: &mut Vec<Finding>) -> VerbCoverage {
    let story = &project.story;
    let verbs = all_verbs(story);
    let used: BTreeSet<&Id> = project
        .walkthroughs
        .iter()
        .flat_map(|(_, w)| w.steps.iter())
        .filter_map(|s| verb_of(&verbs, s.command()))
        .map(|v| &v.id)
        .collect();
    let own: BTreeSet<&Id> = story.verbs.iter().map(|v| &v.id).collect();
    let mut coverage = VerbCoverage::default();
    for v in &story.verbs {
        if used.contains(&v.id) {
            coverage.story_used.push(v.id.clone());
        } else {
            coverage.story_unused.push(v.id.clone());
            if !project.walkthroughs.is_empty() {
                out.push(finding(
                    QualityCheck::VerbCoverage,
                    Diagnostic::warning(
                        file_of(project, "story"),
                        format!("verbs.{}", v.id),
                        format!(
                            "no walkthrough uses the verb `{}`: a test does not show that it works",
                            v.id
                        ),
                    ),
                ));
            }
        }
    }
    coverage.standard_used = used
        .into_iter()
        .filter(|v| !own.contains(v))
        .cloned()
        .collect();
    coverage
}

fn deferred() -> Vec<Deferred> {
    let d = |check, reason: &str| Deferred {
        check,
        reason: reason.into(),
    };
    vec![
        d(
            QualityCheck::NoWarningBeforeDeath,
            "the model has no mark for a warning yet, so a check cannot tell a warned death from a sudden one",
        ),
        d(
            QualityCheck::DeathOffWinningPath,
            "needs the winnable states of the explorer graph (CK-1, P4-04 #81)",
        ),
        d(
            QualityCheck::FarChangeWithoutSignal,
            "needs the signal of a far change (US-06), which the model does not have yet",
        ),
        d(
            QualityCheck::GiftCoverage,
            "needs the actors of D6 (`give` to characters, P4-02 #79)",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourceFile, load};
    use assert2::check;

    fn project(extra: &[(&str, &str)]) -> Project {
        let mut files = vec![
            (
                "story.yaml",
                "title: T\nstart: hall\nverbs:\n  - { id: pray, label: Pray, words: [pray] }\n",
            ),
            (
                "rooms/hall.yaml",
                "name: Hall\ndescription: [\"A hall.\"]\nexits: [{ direction: north, to: maze_a }]\n",
            ),
            (
                "rooms/maze_a.yaml",
                "name: Maze\ndescription: [\"Twisty little passages.\"]\nexits: [{ direction: north, to: maze_b }]\n",
            ),
            (
                "rooms/maze_b.yaml",
                "name: Maze\ndescription: [\"Twisty little passages.\"]\nexits: [{ direction: south, to: maze_a }]\n",
            ),
            (
                "things/coin.yaml",
                "name: coin\nlocation: { room: hall }\nportable: true\n",
            ),
            (
                "things/feather.yaml",
                "name: feather\nlocation: { room: hall }\nportable: true\nred_herring: true\n",
            ),
        ];
        files.extend_from_slice(extra);
        load(
            files
                .into_iter()
                .map(|(p, t)| SourceFile {
                    path: p.into(),
                    text: t.into(),
                })
                .collect(),
        )
        .unwrap()
    }

    fn checks(r: &QualityReport, check: QualityCheck) -> Vec<String> {
        r.findings
            .iter()
            .filter(|f| f.check == check)
            .map(|f| {
                format!(
                    "{} {}: {}",
                    f.diagnostic.file, f.diagnostic.at, f.diagnostic.message
                )
            })
            .collect()
    }

    #[test]
    fn one_way_exits_and_same_texts_are_found() {
        let r = quality(&project(&[]));
        let one_way = checks(&r, QualityCheck::OneWayExit);
        check!(one_way.len() == 1, "{one_way:?}");
        check!(
            one_way[0].starts_with("rooms/hall.yaml exits[0]"),
            "{one_way:?}"
        );
        let same = checks(&r, QualityCheck::SameRoomText);
        check!(same.len() == 2, "{same:?}");
        check!(same[0].contains("`maze_a`, `maze_b`"), "{same:?}");
    }

    #[test]
    fn a_thing_nobody_uses_is_found_unless_it_is_a_red_herring() {
        let r = quality(&project(&[]));
        let unused = checks(&r, QualityCheck::UnusedThing);
        check!(unused.len() == 1, "{unused:?}");
        check!(unused[0].contains("`coin` can be carried"), "{unused:?}");
        // A walkthrough that does more than take it uses it.
        let r = quality(&project(&[(
            "tests/t.yaml",
            "steps: [take coin, pray, flip coin]\n",
        )]));
        check!(checks(&r, QualityCheck::UnusedThing).is_empty());
    }

    #[test]
    fn verb_coverage_counts_the_verbs_of_the_walkthroughs() {
        let r = quality(&project(&[("tests/t.yaml", "steps: [take coin, n]\n")]));
        check!(r.verbs.story_unused == [Id::new("pray")]);
        check!(r.verbs.standard_used.contains(&Id::new("take")));
        check!(checks(&r, QualityCheck::VerbCoverage).len() == 1);
        let r = quality(&project(&[("tests/t.yaml", "steps: [pray]\n")]));
        check!(r.verbs.story_used == [Id::new("pray")]);
        check!(checks(&r, QualityCheck::VerbCoverage).is_empty());
    }

    #[test]
    fn the_names_are_the_json_names() {
        for d in quality(&project(&[])).deferred {
            check!(serde_json::to_value(d.check).unwrap() == d.check.name());
        }
        let c = QualityCheck::OneWayExit;
        check!(serde_json::to_value(c).unwrap() == c.name());
    }

    #[test]
    fn the_report_names_the_checks_it_does_not_run_yet() {
        let r = quality(&project(&[]));
        let deferred: Vec<QualityCheck> = r.deferred.iter().map(|d| d.check).collect();
        check!(deferred.contains(&QualityCheck::GiftCoverage));
        check!(deferred.contains(&QualityCheck::NoWarningBeforeDeath));
    }
}
