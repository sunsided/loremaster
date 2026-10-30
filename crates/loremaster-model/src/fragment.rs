use crate::{Id, Note};
use serde::{Deserialize, Serialize};

/// Where a fragment sits when a description is assembled. Layers are
/// assembled in declaration order of this enum.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    #[default]
    Base,
    Ambient,
    Event,
    Objects,
    Characters,
    Exits,
}

/// How often a fragment shows, and which of its variants.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentMode {
    /// Every time its condition holds.
    #[default]
    Always,
    /// Only on the first visit to the owning room (or first examine).
    FirstVisit,
    /// Only the first time it would show, ever.
    Once,
    /// Variants in order, one per showing, wrapping around.
    Cycle,
    /// A random variant each showing, from the seeded RNG.
    Shuffle,
    /// A random variant, picked once and then kept.
    Sticky,
}

/// One conditional piece of text. A description is an ordered list of these.
///
/// In project files a fragment is either a plain string (always shown, base
/// layer) or a map with the fields below.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "FragmentRepr", into = "FragmentRepr")]
pub struct Fragment {
    /// Optional stable name, used by the debugger and for links from the editor.
    pub id: Option<Id>,
    /// The text, in inline markup. For cycle, shuffle and sticky modes these
    /// are the variants; otherwise exactly one entry.
    pub variants: Vec<String>,
    /// Condition source, parsed by `loremaster-lang`.
    pub when: Option<String>,
    pub mode: FragmentMode,
    pub layer: Layer,
    /// When visible, hide the fragments before it in the same layer.
    pub replaces: bool,
    /// Author notes on this one fragment, for example the setup of a thread.
    pub notes: Vec<Note>,
}

impl Fragment {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            id: None,
            variants: vec![text.into()],
            when: None,
            mode: FragmentMode::Always,
            layer: Layer::Base,
            replaces: false,
            notes: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum FragmentRepr {
    Plain(String),
    Full(FragmentFull),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FragmentFull {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    variants: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    mode: FragmentMode,
    #[serde(default, skip_serializing_if = "is_default")]
    layer: Layer,
    #[serde(default, skip_serializing_if = "is_default")]
    replaces: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    notes: Vec<Note>,
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

impl From<FragmentRepr> for Fragment {
    fn from(repr: FragmentRepr) -> Self {
        match repr {
            FragmentRepr::Plain(text) => Fragment::text(text),
            FragmentRepr::Full(f) => {
                let mut variants = f.variants;
                if let Some(text) = f.text {
                    variants.insert(0, text);
                }
                Fragment {
                    id: f.id,
                    variants,
                    when: f.when,
                    mode: f.mode,
                    layer: f.layer,
                    replaces: f.replaces,
                    notes: f.notes,
                }
            }
        }
    }
}

impl From<Fragment> for FragmentRepr {
    fn from(f: Fragment) -> Self {
        let plain = f.id.is_none()
            && f.when.is_none()
            && f.mode == FragmentMode::Always
            && f.layer == Layer::Base
            && !f.replaces
            && f.notes.is_empty()
            && f.variants.len() == 1;
        if plain {
            return FragmentRepr::Plain(f.variants.into_iter().next().unwrap_or_default());
        }
        let (text, variants) = if f.variants.len() == 1 {
            (f.variants.into_iter().next(), Vec::new())
        } else {
            (None, f.variants)
        };
        FragmentRepr::Full(FragmentFull {
            id: f.id,
            text,
            variants,
            when: f.when,
            mode: f.mode,
            layer: f.layer,
            replaces: f.replaces,
            notes: f.notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn plain_string_is_an_always_base_fragment() {
        let f: Fragment = serde_json::from_str(r#""A bare room.""#).unwrap();
        check!(f == Fragment::text("A bare room."));
        check!(serde_json::to_string(&f).unwrap() == r#""A bare room.""#);
    }

    #[test]
    fn full_fragment_round_trips() {
        let json = r#"{"variants":["a","b"],"when":"lamp.lit","mode":"cycle","layer":"ambient"}"#;
        let f: Fragment = serde_json::from_str(json).unwrap();
        check!(f.variants == vec!["a".to_string(), "b".to_string()]);
        check!(f.mode == FragmentMode::Cycle);
        check!(serde_json::to_string(&f).unwrap() == json);
    }
}
