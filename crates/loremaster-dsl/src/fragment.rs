//! Fragments: conditional pieces of text. A plain `&str` is a fragment that
//! always shows in the base layer; [`frag`] and [`variants`] start one with
//! more fields.

use crate::convert::strings;
use loremaster_model::{Fragment, FragmentMode, Id, Layer, Note};

/// A fragment with one text: `frag("The sea has gone black.").ambient().when("night")`.
pub fn frag(text: impl Into<String>) -> Fragment {
    Fragment::text(text)
}

/// A fragment with variants, for the modes `cycle`, `shuffle` and `sticky`:
/// `variants(["Waves hiss.", "A bell clangs."]).shuffle()`.
pub fn variants(texts: impl IntoIterator<Item = impl AsRef<str>>) -> Fragment {
    Fragment {
        variants: strings(texts),
        ..Fragment::text("")
    }
}

/// Builder methods on a [`Fragment`]. Each sets one field, as in the YAML map
/// form of a fragment.
pub trait FragmentExt: Sized {
    /// `when:`, a condition.
    fn when(self, condition: impl Into<String>) -> Self;
    /// `layer:`.
    fn layer(self, layer: Layer) -> Self;
    /// `mode:`.
    fn mode(self, mode: FragmentMode) -> Self;
    /// `id:`, a stable name for the debugger and the editor.
    fn id(self, id: impl AsRef<str>) -> Self;
    /// `replaces: true`: hide the fragments before it in the same layer.
    fn replaces(self) -> Self;
    /// One more variant.
    fn variant(self, text: impl Into<String>) -> Self;
    /// An author note on this fragment.
    fn note(self, text: impl Into<String>) -> Self;

    /// `layer: ambient`.
    fn ambient(self) -> Self {
        self.layer(Layer::Ambient)
    }
    /// `layer: event`.
    fn event(self) -> Self {
        self.layer(Layer::Event)
    }
    /// `mode: first_visit`.
    fn first_visit(self) -> Self {
        self.mode(FragmentMode::FirstVisit)
    }
    /// `mode: once`.
    fn once(self) -> Self {
        self.mode(FragmentMode::Once)
    }
    /// `mode: cycle`.
    fn cycle(self) -> Self {
        self.mode(FragmentMode::Cycle)
    }
    /// `mode: shuffle`.
    fn shuffle(self) -> Self {
        self.mode(FragmentMode::Shuffle)
    }
    /// `mode: sticky`.
    fn sticky(self) -> Self {
        self.mode(FragmentMode::Sticky)
    }
}

impl FragmentExt for Fragment {
    fn when(mut self, condition: impl Into<String>) -> Self {
        self.when = Some(condition.into());
        self
    }
    fn layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }
    fn mode(mut self, mode: FragmentMode) -> Self {
        self.mode = mode;
        self
    }
    fn id(mut self, id: impl AsRef<str>) -> Self {
        self.id = Some(Id::new(id.as_ref()));
        self
    }
    fn replaces(mut self) -> Self {
        self.replaces = true;
        self
    }
    fn variant(mut self, text: impl Into<String>) -> Self {
        self.variants.push(text.into());
        self
    }
    fn note(mut self, text: impl Into<String>) -> Self {
        self.notes.push(Note::text(text));
        self
    }
}

/// A fragment argument: `&str`, `String` or a [`Fragment`].
pub trait IntoFragment {
    fn into_fragment(self) -> Fragment;
}

impl IntoFragment for Fragment {
    fn into_fragment(self) -> Fragment {
        self
    }
}

impl IntoFragment for &str {
    fn into_fragment(self) -> Fragment {
        Fragment::text(self)
    }
}

impl IntoFragment for String {
    fn into_fragment(self) -> Fragment {
        Fragment::text(self)
    }
}
