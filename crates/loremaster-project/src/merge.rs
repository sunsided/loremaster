//! `extend` and `replace`: how a later pack changes an entity that an earlier
//! pack defines (DS-1).

use crate::Origin;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// How a pack changes an entity of an earlier pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMode {
    /// The later entity takes the place of the earlier one.
    Replace,
    /// The later fields are added to the earlier entity.
    Extend,
}

/// Fields that a pack adds to an entity of an earlier pack.
#[derive(Debug, Clone, PartialEq)]
pub struct Extension {
    /// The entity, for example `room:cellar`.
    pub key: String,
    /// The fields to add, as in the entity's YAML file.
    pub fields: Value,
    pub origin: Origin,
}

/// Read `extend: true` or `replace: true` from the top level of a YAML file.
/// Returns the mode and the text without that line.
pub(crate) fn merge_mode(text: &str) -> (Option<MergeMode>, String) {
    let mut mode = None;
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| {
            let found = match line.trim_end() {
                "extend: true" => Some(MergeMode::Extend),
                "replace: true" => Some(MergeMode::Replace),
                _ => None,
            };
            if found.is_some() {
                mode = found;
            }
            found.is_none()
        })
        .collect();
    match mode {
        Some(m) => (Some(m), kept.join("\n") + "\n"),
        None => (None, text.to_string()),
    }
}

/// Add `fields` to `entity`. Lists are appended, maps are merged key by key,
/// and a value that both sides set is an error. Returns the paths of the
/// conflicts, or an error when the result is not a valid entity.
pub(crate) fn extend_entity<T: Serialize + DeserializeOwned>(
    entity: &mut T,
    fields: &Value,
) -> Result<(), String> {
    let mut base = serde_json::to_value(&*entity).map_err(|e| e.to_string())?;
    let mut conflicts = Vec::new();
    merge(&mut base, fields, "", &mut conflicts);
    if !conflicts.is_empty() {
        let list = conflicts
            .iter()
            .map(|c| format!("`{c}`"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("both packs set {list}"));
    }
    *entity = serde_json::from_value(base).map_err(|e| e.to_string())?;
    Ok(())
}

fn merge(base: &mut Value, ext: &Value, path: &str, conflicts: &mut Vec<String>) {
    let (Value::Object(base), Value::Object(ext)) = (base, ext) else {
        return;
    };
    for (key, value) in ext {
        if key == "id" {
            continue;
        }
        let at = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        match (base.get_mut(key), value) {
            (None | Some(Value::Null), _) => {
                base.insert(key.clone(), value.clone());
            }
            (Some(Value::Array(items)), Value::Array(more)) => items.extend(more.iter().cloned()),
            (Some(b @ Value::Object(_)), Value::Object(_)) => merge(b, value, &at, conflicts),
            (Some(_), _) => conflicts.push(at),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_mode_line_is_taken_out() {
        let (mode, text) = merge_mode("extend: true\nexits: []\n");
        assert_eq!(mode, Some(MergeMode::Extend));
        assert_eq!(text, "exits: []\n");
        let (mode, text) = merge_mode("name: Cellar\n");
        assert_eq!(mode, None);
        assert_eq!(text, "name: Cellar\n");
    }

    #[test]
    fn lists_append_maps_merge_and_scalars_conflict() {
        let mut base = json!({ "name": "Cellar", "description": ["Dark."], "props": { "a": 1 } });
        let mut conflicts = Vec::new();
        merge(
            &mut base,
            &json!({ "description": ["Damp."], "props": { "b": 2 }, "level": 1 }),
            "",
            &mut conflicts,
        );
        assert!(conflicts.is_empty());
        assert_eq!(
            base,
            json!({ "name": "Cellar", "description": ["Dark.", "Damp."], "props": { "a": 1, "b": 2 }, "level": 1 })
        );
        merge(
            &mut base,
            &json!({ "name": "Vault", "props": { "a": 3 } }),
            "",
            &mut conflicts,
        );
        assert_eq!(conflicts, ["name", "props.a"]);
    }
}
