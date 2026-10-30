use crate::{Fragment, Id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Author text around the response to a verb. All parts are optional, and a
/// story without frames plays exactly as before.
///
/// Frames can be on the story, on a room and on a thing. For an action, the
/// most specific frame for the verb is used: the first object's thing, then
/// the current room, then the story.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// Shown before the response.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<Fragment>,
    /// Shown instead of the default response, when one of these fragments applies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instead: Vec<Fragment>,
    /// Shown after the response.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<Fragment>,
}

/// Frames by verb id, for example `look` or `examine`.
pub type Frames = BTreeMap<Id, Frame>;
