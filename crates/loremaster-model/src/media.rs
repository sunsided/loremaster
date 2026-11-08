use crate::{Direction, Id};
use serde::{Deserialize, Serialize};

/// A media file of the project, from `assets/`. Content names assets by id;
/// the host decides how they look and sound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub id: Id,
    /// The path of the file, relative to the project folder.
    pub file: String,
    /// What the asset shows or says, as text: the alt text of an image, the
    /// caption of a sound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// The asset carries no information, so it needs no caption.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub decorative: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub licence: Option<Licence>,
}

/// Who made an asset and under what terms it is used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Licence {
    /// The licence, for example `CC0-1.0`.
    pub name: String,
    pub author: String,
    /// Where the asset comes from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// The media slots of a room, a character or a machine state.
///
/// Each slot is one asset id, or a list of variants: the first variant whose
/// `when` holds is the active one.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Media {
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "slot"
    )]
    pub illustration: Vec<MediaVariant>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "slot"
    )]
    pub portrait: Vec<MediaVariant>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "slot"
    )]
    pub music: Vec<MediaVariant>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "slot"
    )]
    pub ambience: Vec<MediaVariant>,
}

impl Media {
    pub fn is_empty(&self) -> bool {
        self == &Media::default()
    }

    /// The slots with their names.
    pub fn slots(&self) -> [(&'static str, &[MediaVariant]); 4] {
        [
            ("illustration", &self.illustration),
            ("portrait", &self.portrait),
            ("music", &self.music),
            ("ambience", &self.ambience),
        ]
    }
}

/// One asset of a slot: `gate`, or `{ asset: gate_broken, when: gate_broken }`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MediaVariant {
    pub asset: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// The hotspots of a room's illustration (ME-4).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<Region>,
}

/// A hotspot on an illustration (ME-4). It points to a thing or to an exit
/// of the room; a click on it selects that part. The rectangle is in percent
/// of the image, so the story holds no pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thing: Option<Id>,
    /// The direction of an exit of the room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit: Option<Direction>,
    /// Left, top, width and height, in percent of the image.
    pub rect: [u8; 4],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VariantMap {
    asset: Id,
    #[serde(default)]
    when: Option<String>,
    #[serde(default)]
    regions: Vec<Region>,
}

impl<'de> Deserialize<'de> for MediaVariant {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        crate::de::string_or_map(
            d,
            "an asset id, or a map with `asset` and `when`",
            |asset| MediaVariant {
                asset: Id::new(asset),
                when: None,
                regions: Vec::new(),
            },
            |v: VariantMap| MediaVariant {
                asset: v.asset,
                when: v.when,
                regions: v.regions,
            },
        )
    }
}

fn slot<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<MediaVariant>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(MediaVariant),
        Many(Vec<MediaVariant>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(v) => vec![v],
        OneOrMany::Many(v) => v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;
    use serde_json::json;

    #[test]
    fn a_slot_reads_one_id_or_a_list_of_variants() {
        let media: Media = serde_json::from_value(json!({
            "illustration": { "asset": "gate", "regions": [
                { "thing": "door", "rect": [10, 20, 30, 40] },
                { "exit": "north", "rect": [0, 0, 5, 5] },
            ] },
            "music": [{ "asset": "alarm", "when": "alarm" }, "calm"],
        }))
        .unwrap();
        check!(media.illustration[0].asset == Id::new("gate"));
        check!(media.illustration[0].regions[0].thing == Some(Id::new("door")));
        check!(media.illustration[0].regions[1].exit == Some(Direction::North));
        check!(media.music[0].when.as_deref() == Some("alarm"));
        check!(media.music[1].when == None);
        let back: Media = serde_json::from_value(serde_json::to_value(&media).unwrap()).unwrap();
        check!(back == media);
    }

    #[test]
    fn an_unknown_slot_is_an_error() {
        check!(serde_json::from_value::<Media>(json!({ "sound": "x" })).is_err());
    }
}
