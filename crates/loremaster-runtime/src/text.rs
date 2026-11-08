pub(crate) mod pool;

use crate::eval::Eval;
use crate::world::CText;
use crate::{Run, Source, State};
use dramaturge_model::Pools;
use dramaturge_runtime::{Selection, select, select_named};
use loremaster_lang::{CompiledLine, Segment};
use loremaster_model::Id;
use pool::{LeafText, Lore, LoreJudge, LorePool, Rng, seen_by_player};
use serde::{Deserialize, Serialize};

/// Render compiled markup to text.
pub(crate) fn render(ev: &Eval, segments: &[Segment]) -> String {
    let mut out = String::new();
    render_into(ev, segments, &mut out);
    out
}

fn render_into(ev: &Eval, segments: &[Segment], out: &mut String) {
    for seg in segments {
        match seg {
            Segment::Text(t) => out.push_str(t),
            Segment::Insert { operand, .. } => out.push_str(&ev.operand_text(operand)),
            Segment::Choice {
                condition,
                then,
                otherwise,
                ..
            } => {
                let branch = if ev.holds(condition) { then } else { otherwise };
                render_into(ev, branch, out);
            }
        }
    }
}

/// Render a compiled line to runs, one for each segment that shows, with a
/// run of one space between them. Speech gets straight double quotes and its
/// speaker. Each segment's run is a copy of `base` with its text and speaker.
///
/// A line that shows one narration segment renders as before segments
/// existed: its text is not trimmed. Speech is always trimmed, so that the
/// quotes sit on the words, and when more than one segment shows, each is
/// trimmed, so that the join has exactly one space. Only the segments that
/// show count: a segment whose `when` does not hold, or whose markup renders
/// to nothing, is left out and does not change the text of the others.
pub(crate) fn line_runs(ev: &Eval, line: &CompiledLine, base: &Run) -> Vec<Run> {
    let shown: Vec<(&loremaster_lang::LinePart, String)> = line
        .iter()
        .filter(|part| ev.holds_opt(&part.when))
        .map(|part| (part, render(ev, &part.text)))
        .filter(|(_, rendered)| !rendered.trim().is_empty())
        .collect();
    let several = shown.len() > 1;
    let mut out: Vec<Run> = Vec::new();
    for (part, rendered) in &shown {
        let text = if several || part.speech {
            rendered.trim()
        } else {
            rendered.as_str()
        };
        if !out.is_empty() {
            out.push(Run::plain(" "));
        }
        out.push(Run {
            text: if part.speech {
                format!("\"{text}\"")
            } else {
                text.to_string()
            },
            speaker: part.speaker.clone(),
            ..base.clone()
        });
    }
    out
}

/// A compiled line as one string: the text of its runs.
pub(crate) fn line_text(ev: &Eval, line: &CompiledLine) -> String {
    line_runs(ev, line, &Run::plain(""))
        .iter()
        .map(|r| r.text.as_str())
        .collect()
}

/// Fragment source text split into pieces, each marked as active or not in
/// the current state. The editor uses it to strike through inline branches
/// that do not apply.
pub(crate) fn outline(ev: &Eval, segments: &[Segment], active: bool, out: &mut Vec<SourcePiece>) {
    for seg in segments {
        match seg {
            Segment::Text(t) => out.push(SourcePiece::new(SourceRole::Text, t.clone(), active)),
            Segment::Insert { operand, source } => {
                let mut piece = SourcePiece::new(SourceRole::Insert, source.clone(), active);
                piece.value = Some(ev.operand_text(operand));
                out.push(piece);
            }
            Segment::Choice {
                condition,
                then,
                otherwise,
                syntax,
                ..
            } => {
                let holds = ev.holds(condition);
                out.push(SourcePiece::new(
                    SourceRole::Syntax,
                    format!("{} ", syntax.open),
                    active,
                ));
                outline(ev, then, active && holds, out);
                if let Some(sep) = &syntax.separator {
                    out.push(SourcePiece::new(
                        SourceRole::Syntax,
                        format!(" {sep} "),
                        active,
                    ));
                    outline(ev, otherwise, active && !holds, out);
                }
                out.push(SourcePiece::new(
                    SourceRole::Syntax,
                    syntax.close.clone(),
                    active,
                ));
            }
        }
    }
}

/// What a piece of fragment source is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    /// Story text.
    Text,
    /// Markup: braces, conditions and separators.
    Syntax,
    /// A value insert such as `{time.phase}`.
    Insert,
}

/// One piece of fragment source, for the assembly panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePiece {
    pub role: SourceRole,
    pub text: String,
    /// False when the piece is in an inline branch whose condition does not hold now.
    pub active: bool,
    /// For an insert: the value it shows now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl SourcePiece {
    fn new(role: SourceRole, text: String, active: bool) -> Self {
        Self {
            role,
            text,
            active,
            value: None,
        }
    }
}

/// A text as the core chose it, in the order loremaster shows it.
pub(crate) struct Chosen<'w> {
    pub selection: Selection<'w, Lore>,
    /// Indexes into `selection.shown`, in the order they show: sorted by
    /// layer, without the leaves that a later fragment `replaces`.
    pub order: Vec<usize>,
}

/// Choose the text of a pool now, for the player, without a change to the
/// state. `rng` is a copy of the game's: a preview drops it, play keeps it.
pub(crate) fn choose<'w>(
    ev: &Eval,
    pool: &'w LorePool,
    pools: &'w Pools<Lore>,
    rng: &mut u64,
) -> Chosen<'w> {
    let caller = ev.state.room.as_str();
    let selection = select(
        pool,
        pools,
        caller,
        &seen_by_player(),
        &LoreJudge(ev),
        ev.state,
        &mut Rng(rng),
    );
    arrange(selection)
}

/// [`choose`] for the named pool `id` of `texts/`. The caller is the
/// current room, so its `first_time` texts show on the first visit.
pub(crate) fn choose_named<'w>(
    ev: &Eval,
    id: &str,
    pools: &'w Pools<Lore>,
    rng: &mut u64,
) -> Chosen<'w> {
    let caller = ev.state.room.as_str();
    let selection = select_named(
        id,
        pools,
        caller,
        &seen_by_player(),
        &LoreJudge(ev),
        ev.state,
        &mut Rng(rng),
    );
    arrange(selection)
}

/// Layers and `replaces`: a leaf of a fragment that replaces hides the
/// leaves of the other fragments before it in its layer; then the layers
/// sort in order.
fn arrange(selection: Selection<'_, Lore>) -> Chosen<'_> {
    let mut order: Vec<usize> = Vec::new();
    for (i, shown) in selection.shown.iter().enumerate() {
        let leaf = shown.leaf;
        if leaf.replaces {
            order.retain(|&j| {
                let other = selection.shown[j].leaf;
                other.layer != leaf.layer || other.fragment == leaf.fragment
            });
        }
        order.push(i);
    }
    order.sort_by_key(|&i| selection.shown[i].leaf.layer);
    Chosen { selection, order }
}

/// The fragments of `text` that show now, in assembly order, without a
/// change to the state.
pub(crate) fn visible(ev: &Eval, text: &CText) -> Vec<usize> {
    let mut rng = ev.state.rng;
    let chosen = choose(ev, &text.pool, &ev.world.pools, &mut rng);
    let mut out: Vec<usize> = Vec::new();
    for &i in &chosen.order {
        let f = chosen.selection.shown[i].leaf.fragment;
        if !out.contains(&f) {
            out.push(f);
        }
    }
    out
}

/// Record what showed: the progress of the candidates that show (not of the
/// ones that a later fragment replaced). Returns the story variables that
/// the player learned, to set to true.
pub(crate) fn commit(state: &mut State, chosen: &Chosen) -> Vec<String> {
    let mut selection = chosen.selection.clone();
    selection
        .steps
        .retain(|s| s.leaves.is_empty() || s.leaves.clone().any(|i| chosen.order.contains(&i)));
    let mut learned = Vec::new();
    dramaturge_runtime::commit(&selection, &seen_by_player(), state, &mut |var: &String,
                                                                           _: Option<
        &Id,
    >| {
        learned.push(var.clone())
    });
    learned
}

/// Render a chosen text to runs. A fragment's text remembers its source
/// (`owner`, the fragment, the layer); a line of a named pool renders like
/// `base`.
pub(crate) fn runs(ev: &Eval, chosen: &Chosen, owner: &str, base: &Run) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for &i in &chosen.order {
        let shown = &chosen.selection.shown[i];
        let leaf = shown.leaf;
        let runs = match &leaf.text {
            LeafText::Markup(segments) => {
                // Markup that renders to nothing can leave a space at the end; it would show as a gap.
                let text = render(ev, segments).trim_end().to_string();
                if text.trim().is_empty() {
                    continue;
                }
                vec![Run {
                    text,
                    source: Some(Source {
                        owner: owner.to_string(),
                        fragment: leaf.fragment,
                        layer: leaf.layer,
                        condition: leaf.when_src.clone(),
                        variant: chosen.selection.steps[shown.step].variant as usize,
                    }),
                    link: None,
                    speaker: None,
                }]
            }
            LeafText::Line(line) => line_runs(ev, line, base),
        };
        if runs.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(Run::plain(" "));
        }
        out.extend(runs);
    }
    out
}

pub(crate) fn article(name: &str) -> &'static str {
    match name.chars().next().map(|c| c.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

/// Join names: "a, b and c".
pub(crate) fn list(items: &[String], conj: &str) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} {conj} {last}", init.join(", ")),
    }
}
