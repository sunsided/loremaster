use crate::{Id, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The `await` effect: a rule stops, the session asks the host, and the rule
/// goes on with the answer (SE-5, SE-6). Exactly one kind of request is set.
///
/// ```yaml
/// - await: { choice: [stay, flee], into: decision }
/// - await: { action: pick_lock, args: { difficulty: 3 }, results: [open, broken], into: lock_result }
/// - await: { roll: 2d6, into: total }
/// - await: { walk_to: harbour }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AwaitSpec {
    /// A choice: the ids of the options, or `{ id, text }`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choice: Vec<AwaitOption>,
    /// A custom action that the host does, for example a mini-game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<Id>,
    /// The arguments of the action.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, Value>,
    /// The results that the action can give.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<Id>,
    /// A roll of dice, for example `2d6` or `1d20+3`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roll: Option<String>,
    /// The host walks the player to a room (for stage hosts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk_to: Option<Id>,
    /// The variable that takes the answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub into: Option<String>,
}

/// An option of a choice: an id, or an id with a text to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AwaitOption {
    Id(Id),
    Labeled { id: Id, text: String },
}

impl AwaitOption {
    pub fn id(&self) -> &Id {
        match self {
            AwaitOption::Id(id) | AwaitOption::Labeled { id, .. } => id,
        }
    }

    /// The text to show: the label, or the id.
    pub fn text(&self) -> &str {
        match self {
            AwaitOption::Id(id) => id.as_str(),
            AwaitOption::Labeled { text, .. } => text,
        }
    }
}

impl AwaitSpec {
    /// How many kinds of request are set. A valid await sets exactly one.
    pub fn kinds(&self) -> usize {
        usize::from(!self.choice.is_empty())
            + usize::from(self.action.is_some())
            + usize::from(self.roll.is_some())
            + usize::from(self.walk_to.is_some())
    }
}

/// Dice as `NdM+K`: `count` dice with `sides` sides, plus `bonus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dice {
    pub count: u32,
    pub sides: u32,
    pub bonus: i64,
}

impl Dice {
    /// Read `2d6`, `d20` or `1d20+3` / `3d6-1`.
    pub fn parse(s: &str) -> Option<Dice> {
        let s = s.trim().to_ascii_lowercase();
        let (count, rest) = s.split_once('d')?;
        let count = if count.is_empty() {
            1
        } else {
            count.parse().ok()?
        };
        let (sides, bonus) = match rest.find(['+', '-']) {
            Some(i) => (&rest[..i], rest[i..].parse().ok()?),
            None => (rest, 0),
        };
        let sides = sides.parse().ok()?;
        (count >= 1 && sides >= 1).then_some(Dice {
            count,
            sides,
            bonus,
        })
    }

    pub fn min(&self) -> i64 {
        i64::from(self.count) + self.bonus
    }

    pub fn max(&self) -> i64 {
        i64::from(self.count) * i64::from(self.sides) + self.bonus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dice_parse() {
        assert_eq!(
            Dice::parse("2d6"),
            Some(Dice {
                count: 2,
                sides: 6,
                bonus: 0
            })
        );
        assert_eq!(
            Dice::parse("d20"),
            Some(Dice {
                count: 1,
                sides: 20,
                bonus: 0
            })
        );
        assert_eq!(
            Dice::parse("1d20+3"),
            Some(Dice {
                count: 1,
                sides: 20,
                bonus: 3
            })
        );
        assert_eq!(
            Dice::parse("3D6-1").map(|d| (d.min(), d.max())),
            Some((2, 17))
        );
        assert_eq!(Dice::parse("6"), None);
        assert_eq!(Dice::parse("0d6"), None);
    }
}
