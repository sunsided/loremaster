use crate::eval::Eval;
use crate::world::CFragment;
use crate::{Run, Source, rng};
use loremaster_lang::Segment;
use loremaster_model::FragmentMode;
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

/// A fragment chosen for display, and which of its variants.
pub(crate) struct Pick {
    pub index: usize,
    pub variant: usize,
}

/// Decide which fragments are visible, in assembly order, without touching
/// progress. `first_visit` applies to rooms; elsewhere it means "first showing".
pub(crate) fn visible(ev: &Eval, fragments: &[CFragment], first_visit: bool) -> Vec<usize> {
    let mut shown: Vec<usize> = Vec::new();
    for (i, f) in fragments.iter().enumerate() {
        let seen = ev.state.progress.get(&f.key).map_or(0, |p| p.shown);
        let eligible = match f.mode {
            FragmentMode::FirstVisit => first_visit && (f.owner.starts_with("room:") || seen == 0),
            FragmentMode::Once => seen == 0,
            _ => true,
        };
        if !eligible || f.variants.is_empty() || !ev.holds_opt(&f.when) {
            continue;
        }
        if f.replaces {
            shown.retain(|&j| fragments[j].layer != f.layer);
        }
        shown.push(i);
    }
    shown.sort_by_key(|&i| fragments[i].layer);
    shown
}

/// Pick a variant for each visible fragment and record that it was shown.
pub(crate) fn pick(
    state: &mut crate::State,
    fragments: &[CFragment],
    visible: &[usize],
) -> Vec<Pick> {
    visible
        .iter()
        .map(|&i| {
            let f = &fragments[i];
            let n = f.variants.len();
            let mut rng_state = state.rng;
            let progress = state.progress.entry(f.key.clone()).or_default();
            let variant = match f.mode {
                FragmentMode::Cycle => progress.shown as usize % n,
                FragmentMode::Shuffle => rng::below(&mut rng_state, n),
                FragmentMode::Sticky => *progress
                    .sticky
                    .get_or_insert_with(|| rng::below(&mut rng_state, n) as u32)
                    as usize,
                _ => 0,
            };
            progress.shown += 1;
            state.rng = rng_state;
            Pick { index: i, variant }
        })
        .collect()
}

/// Render picked fragments to runs that remember their source.
pub(crate) fn runs(ev: &Eval, fragments: &[CFragment], picks: &[Pick]) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for p in picks {
        let f = &fragments[p.index];
        // Markup that renders to nothing can leave a space at the end; it would show as a gap.
        let text = render(ev, &f.variants[p.variant]).trim_end().to_string();
        if text.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(Run::plain(" "));
        }
        out.push(Run {
            text,
            source: Some(Source {
                owner: f.owner.clone(),
                fragment: f.index,
                layer: f.layer,
                condition: f.when_src.clone(),
                variant: p.variant,
            }),
            link: None,
        });
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
