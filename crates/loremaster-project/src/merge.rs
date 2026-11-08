//! `extend` and `replace`: how a later pack changes an entity that an earlier
//! pack defines (DS-1).

use crate::Origin;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extension {
    /// The entity, for example `room:cellar`.
    pub key: String,
    /// The fields to add, as in the entity's YAML file.
    pub fields: Value,
    pub origin: Origin,
}

/// The `override:` block of a YAML file (DS-12): the Rust part that the file
/// overrides and the state of that part when the override was made. `base`
/// is the value itself: the hash alone shows that Rust changed, but not
/// what changed, and the three-way diff of `assemble` needs it. A commit is
/// not enough, because a project does not always have its git history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideBase {
    /// The entity, for example `room:cellar`.
    pub entity: String,
    pub file: String,
    pub line: u32,
    pub base_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    pub base: Value,
}

/// A YAML file that overrides a Rust part.
#[derive(Debug, Clone, PartialEq)]
pub struct Override {
    pub base: OverrideBase,
    /// The YAML file.
    pub path: String,
}

/// Take the top-level `override:` block out of a YAML file. Like
/// [`merge_mode`], blanks its lines, so that the line numbers stay right.
pub(crate) fn take_override(text: &str) -> Result<(Option<OverrideBase>, String), String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Block {
        r#override: OverrideBase,
    }
    let mut lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim_end() == "override:") else {
        return Ok((None, text.to_string()));
    };
    let inside = lines[start + 1..]
        .iter()
        .take_while(|l| l.is_empty() || l.starts_with([' ', '\t']))
        .count();
    let end = start + 1 + inside;
    let block = lines[start..end].join("\n") + "\n";
    let block: Block = serde_saphyr::from_str(&block).map_err(|e| e.to_string())?;
    lines[start..end].fill("");
    Ok((Some(block.r#override), lines.join("\n") + "\n"))
}

/// The three-way diff of a drifted override (DS-13), one block for each
/// top-level field that is not the same on all three sides.
pub(crate) fn three_way(base: &Value, now: &Value, over: &Value) -> String {
    let fields = [base, now, over]
        .iter()
        .filter_map(|v| v.as_object())
        .flat_map(|m| m.keys())
        .collect::<std::collections::BTreeSet<_>>();
    let show = |v: &Value, f: &str| v.get(f).map_or("(not set)".to_string(), Value::to_string);
    let mut out = String::new();
    for f in fields {
        let (b, n, o) = (show(base, f), show(now, f), show(over, f));
        if b != n || b != o {
            out += &format!("\n{f}:\n  base:     {b}\n  Rust now: {n}\n  override: {o}");
        }
    }
    out
}

/// Read `extend: true` or `replace: true` from the top level of a YAML file:
/// a whole line at column 0, so a nested value never counts. Returns the mode
/// and the text with that line blanked, so that the line numbers of parse
/// errors stay right. A file with both is an error.
pub(crate) fn merge_mode(text: &str) -> Result<(Option<MergeMode>, String), String> {
    let mut mode = None;
    let mut conflict = false;
    let kept: Vec<&str> = text
        .lines()
        .map(|line| {
            let found = match line.trim_end() {
                "extend: true" => Some(MergeMode::Extend),
                "replace: true" => Some(MergeMode::Replace),
                _ => None,
            };
            if found.is_some() {
                conflict |= mode.is_some_and(|m| Some(m) != found);
                mode = found;
                ""
            } else {
                line
            }
        })
        .collect();
    match mode {
        Some(_) if conflict => {
            Err("a file can say `extend: true` or `replace: true`, not both".into())
        }
        Some(m) => Ok((Some(m), kept.join("\n") + "\n")),
        None => Ok((None, text.to_string())),
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
        let (mode, text) = merge_mode("extend: true\nexits: []\n").unwrap();
        assert_eq!(mode, Some(MergeMode::Extend));
        // Blanked, not removed: the line numbers stay.
        assert_eq!(text, "\nexits: []\n");
        let (mode, text) = merge_mode("name: Cellar\n").unwrap();
        assert_eq!(mode, None);
        assert_eq!(text, "name: Cellar\n");
        // Only a whole top-level line counts, not a nested value.
        let nested = "name: Cellar\nnotes:\n  - extend: true\n  - { replace: true }\n";
        let (mode, text) = merge_mode(nested).unwrap();
        assert_eq!(mode, None);
        assert_eq!(text, nested);
        // Both modes in one file are an error.
        assert!(merge_mode("extend: true\nreplace: true\nname: Cellar\n").is_err());
    }

    #[test]
    fn the_override_block_is_taken_out() {
        let text = "override:\n  entity: room:a\n  file: w.rs\n  line: 3\n  base_hash: x\n  base: { id: a }\nname: A\n";
        let (base, rest) = take_override(text).unwrap();
        let base = base.unwrap();
        assert_eq!(base.entity, "room:a");
        assert_eq!(base.base["id"], "a");
        assert_eq!(base.commit, None);
        assert_eq!(rest, "\n\n\n\n\n\nname: A\n");
        assert_eq!(take_override("name: A\n").unwrap().0, None);
    }

    #[test]
    fn the_diff_shows_only_changed_fields() {
        let d = three_way(
            &json!({ "id": "a", "name": "A" }),
            &json!({ "id": "a", "name": "B" }),
            &json!({ "id": "a", "name": "C" }),
        );
        assert_eq!(
            d,
            "\nname:\n  base:     \"A\"\n  Rust now: \"B\"\n  override: \"C\""
        );
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
