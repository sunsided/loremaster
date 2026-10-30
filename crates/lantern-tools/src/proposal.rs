//! Proposals: changes that an assistant suggests and a person accepts.
//!
//! A proposal holds whole files: new or changed files, and files to delete.
//! The files are the source of truth. From them, [`Workspace::view_proposal`]
//! derives what the editor and the model need: the problems the change adds,
//! the changes per entity, field and fragment, and the text of each changed
//! room in all its states.
//!
//! The workspace checks a proposal against the project as it is now, so the
//! view of a proposal can change when the project changes.

use crate::{ToolError, Workspace, err};
use lantern_project::{Diagnostic, Project, Severity, SourceFile, check, load};
use lantern_runtime::{Game, World};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A suggested change to the project files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    /// One line on what the change does and why.
    pub summary: String,
    /// New or changed files, with their whole text.
    #[serde(default)]
    pub files: Vec<SourceFile>,
    /// Paths of files to delete.
    #[serde(default)]
    pub delete: Vec<String>,
    /// The SHA-256 of each touched file when the proposal was made; `None` for
    /// a file that did not exist. A file that changed since makes the proposal stale.
    #[serde(default)]
    pub base: BTreeMap<String, Option<String>>,
}

/// One touched file: its text before and after the proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileChange {
    pub path: String,
    /// The SHA-256 of the file when the proposal was made, or null for a new file.
    pub base_sha: Option<String>,
    /// The text on disk now, or null when the file does not exist.
    pub before: Option<String>,
    /// The text after the proposal, or null when the proposal deletes the file.
    pub after: Option<String>,
}

/// The SHA-256 of a text, as lowercase hex.
pub fn sha256(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A proposal as checked against the project now.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProposalView {
    pub id: String,
    pub summary: String,
    /// True when the project with the proposal has no errors.
    pub ok: bool,
    /// True when a touched file changed after the proposal was made.
    pub stale: bool,
    /// All problems of the project with the proposal.
    pub check: Vec<Diagnostic>,
    /// The problems that the proposal adds.
    pub new_problems: Vec<Diagnostic>,
    pub changes: Vec<Change>,
    /// The text of each new or changed room in every state, when the proposal has no errors.
    pub rooms: Vec<RoomPreview>,
    /// Each touched file with its whole text before and after.
    pub files: Vec<FileChange>,
}

/// One change, at the smallest level that has meaning for the editor.
///
/// `entity` is a key such as `room:shore`, `thing:lamp`, `rule:find_key` or
/// `story`. `before` and `after` are the YAML values as JSON.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    Added {
        entity: String,
        file: String,
        after: Value,
    },
    Removed {
        entity: String,
        file: String,
        before: Value,
    },
    /// A field other than a fragment list, for example `exits` or `name`.
    Field {
        entity: String,
        file: String,
        field: String,
        before: Value,
        after: Value,
    },
    /// One fragment of a fragment list such as `description`. An added
    /// fragment has no `before`, a removed one has no `after`.
    ///
    /// `owner` is the runtime's `Source.owner` of the list, for example
    /// `room:shore`, `thing:lamp/presence`, `room:shore/frames/look/before`,
    /// `story:intro` or `story/frames/look/before`. With the index it finds the
    /// runs of a transcript. `field` is the YAML path of the list in its entity.
    /// `before_fragment` and `after_fragment` are the fragments in one form,
    /// whether the YAML wrote them as a plain string or as a map.
    Fragment {
        entity: String,
        file: String,
        owner: String,
        field: Vec<String>,
        /// The fragment's `id`, when it has one.
        #[serde(skip_serializing_if = "Option::is_none")]
        fragment_id: Option<String>,
        before_index: Option<usize>,
        after_index: Option<usize>,
        before: Option<Value>,
        after: Option<Value>,
        before_fragment: Option<Value>,
        after_fragment: Option<Value>,
    },
}

/// The texts of one room, each with the number of combinations that give it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RoomPreview {
    pub room: String,
    pub texts: Vec<PreviewText>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreviewText {
    pub text: String,
    pub combinations: usize,
    /// One combination that gives the text, as `name=value` pairs.
    pub example: Vec<String>,
    pub warnings: Vec<String>,
}

/// The project files with a proposal applied.
pub(crate) fn overlay(base: &[SourceFile], p: &Proposal) -> Vec<SourceFile> {
    let mut files: Vec<SourceFile> = base
        .iter()
        .filter(|f| !p.delete.contains(&f.path) && !p.files.iter().any(|n| n.path == f.path))
        .cloned()
        .collect();
    files.extend(p.files.iter().cloned());
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

/// A proposal may only touch project files.
pub(crate) fn check_paths(p: &Proposal) -> Result<(), ToolError> {
    if p.files.is_empty() && p.delete.is_empty() {
        return Err(err(
            "The proposal changes nothing. Give `files` or `delete`.",
        ));
    }
    for path in p.files.iter().map(|f| &f.path).chain(&p.delete) {
        let in_folder = ["rooms/", "things/", "rules/"]
            .iter()
            .any(|d| path.strip_prefix(d).is_some_and(|rest| !rest.contains('/')));
        let ok = (path == "story.yaml" || in_folder)
            && path.ends_with(".yaml")
            && !path
                .split('/')
                .any(|c| c.is_empty() || c == "." || c == "..");
        if !ok {
            return Err(err(format!(
                "`{path}` is not a project file. Use story.yaml, or rooms/, things/ or rules/ with one `.yaml` file name."
            )));
        }
    }
    Ok(())
}

impl Workspace {
    /// Add a proposal and give it the next free id. Returns the id.
    pub fn add_proposal(
        &mut self,
        summary: String,
        files: Vec<SourceFile>,
        delete: Vec<String>,
    ) -> Result<String, ToolError> {
        let base = files
            .iter()
            .map(|f| &f.path)
            .chain(&delete)
            .map(|path| {
                let sha = self
                    .files
                    .iter()
                    .find(|f| &f.path == path)
                    .map(|f| sha256(&f.text));
                (path.clone(), sha)
            })
            .collect();
        let mut p = Proposal {
            id: String::new(),
            summary,
            files,
            delete,
            base,
        };
        check_paths(&p)?;
        self.next_proposal = self.next_proposal.max(1);
        while self
            .proposals
            .contains_key(&format!("p{}", self.next_proposal))
        {
            self.next_proposal += 1;
        }
        p.id = format!("p{}", self.next_proposal);
        self.next_proposal += 1;
        let id = p.id.clone();
        self.proposals.insert(id.clone(), p);
        Ok(id)
    }

    /// Replace all proposals with saved ones, for example after a restart or when
    /// another process changed them.
    pub fn set_proposals(&mut self, proposals: impl IntoIterator<Item = Proposal>) {
        self.proposals = proposals.into_iter().map(|p| (p.id.clone(), p)).collect();
    }

    pub fn proposals(&self) -> impl Iterator<Item = &Proposal> {
        self.proposals.values()
    }

    pub fn proposal(&self, id: &str) -> Option<&Proposal> {
        self.proposals.get(id)
    }

    /// Remove a proposal, for example when it was accepted or rejected.
    pub fn remove_proposal(&mut self, id: &str) -> Option<Proposal> {
        self.proposals.remove(id)
    }

    /// The project files with the proposal applied. The caller writes them.
    pub fn apply_proposal(&self, id: &str) -> Result<Vec<SourceFile>, ToolError> {
        let p = self.find_proposal(id)?;
        Ok(overlay(&self.files, p))
    }

    pub(crate) fn find_proposal(&self, id: &str) -> Result<&Proposal, ToolError> {
        self.proposals.get(id).ok_or_else(|| {
            err(format!(
                "There is no proposal `{id}`. Call `list_proposals`."
            ))
        })
    }

    /// Check a proposal against the project now.
    pub fn view_proposal(&self, id: &str) -> Result<ProposalView, ToolError> {
        let p = self.find_proposal(id)?;
        let files = overlay(&self.files, p);
        let (project, mut check) = match load(files) {
            Ok(project) => {
                let diags = check(&project);
                (Some(project), diags)
            }
            Err(diags) => (None, diags),
        };
        let mut ok = !check.iter().any(|d| d.severity == Severity::Error);
        let world = match (&project, ok) {
            (Some(project), true) => match World::compile(project.story.clone()) {
                Ok(w) => Some(Arc::new(w)),
                Err(e) => {
                    ok = false;
                    check.push(Diagnostic {
                        severity: Severity::Error,
                        file: String::new(),
                        at: e.at.clone(),
                        message: e.error.message,
                        line: None,
                        column: None,
                    });
                    None
                }
            },
            _ => None,
        };
        let new_problems = check
            .iter()
            .filter(|d| !self.diagnostics.iter().any(|b| same_problem(b, d)))
            .cloned()
            .collect();
        let changes = match (&self.project, &project) {
            (Some(before), Some(after)) => changes(before, after, p),
            _ => Vec::new(),
        };
        let rooms = match &world {
            Some(world) => previews(world, &changes),
            None => Vec::new(),
        };
        let current = |path: &str| {
            self.files
                .iter()
                .find(|f| f.path == path)
                .map(|f| f.text.clone())
        };
        let stale = p
            .base
            .iter()
            .any(|(path, sha)| current(path).map(|t| sha256(&t)) != *sha);
        let file_changes = p
            .files
            .iter()
            .map(|f| (f.path.clone(), Some(f.text.clone())))
            .chain(p.delete.iter().map(|d| (d.clone(), None)))
            .map(|(path, after)| FileChange {
                base_sha: p.base.get(&path).cloned().flatten(),
                before: current(&path),
                after,
                path,
            })
            .collect();
        Ok(ProposalView {
            id: p.id.clone(),
            summary: p.summary.clone(),
            ok,
            stale,
            check,
            new_problems,
            changes,
            rooms,
            files: file_changes,
        })
    }

    /// A world with the proposal applied, to play it before it is accepted.
    pub(crate) fn proposal_world(&self, id: &str) -> Result<Arc<World>, ToolError> {
        let p = self.find_proposal(id)?;
        let project = load(overlay(&self.files, p)).map_err(|_| {
            err(format!(
                "Proposal `{id}` does not load. Call `get_proposal` to see the problems."
            ))
        })?;
        if check(&project)
            .iter()
            .any(|d| d.severity == Severity::Error)
        {
            return Err(err(format!(
                "Proposal `{id}` has errors. Call `get_proposal` to see them."
            )));
        }
        World::compile(project.story).map(Arc::new).map_err(|e| {
            err(format!(
                "Proposal `{id}` does not compile: {}",
                e.error.message
            ))
        })
    }
}

/// Lines move when text is added above, so compare problems without them.
fn same_problem(a: &Diagnostic, b: &Diagnostic) -> bool {
    a.severity == b.severity && a.file == b.file && a.at == b.at && a.message == b.message
}

/// Each entity as a JSON object, keyed like `Project::origins`.
fn entities(project: &Project) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let story = &project.story;
    for r in &story.rooms {
        out.insert(format!("room:{}", r.id), to_value(r));
    }
    for t in &story.things {
        out.insert(format!("thing:{}", t.id), to_value(t));
    }
    for r in &story.rules {
        out.insert(format!("rule:{}", r.id), to_value(r));
    }
    let mut header = to_value(story);
    if let Value::Object(map) = &mut header {
        for key in ["rooms", "things", "rules"] {
            map.remove(key);
        }
    }
    out.insert("story".into(), header);
    out
}

fn to_value<T: Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

fn file_of<'a>(project: &'a Project, entity: &str) -> &'a str {
    if entity == "story" {
        return "story.yaml";
    }
    project
        .origins
        .get(entity)
        .map(String::as_str)
        .unwrap_or_default()
}

fn changes(before: &Project, after: &Project, p: &Proposal) -> Vec<Change> {
    let (old, new) = (entities(before), entities(after));
    let touched = |project: &Project, key: &str| {
        let file = file_of(project, key);
        p.delete.iter().any(|d| d == file) || p.files.iter().any(|f| f.path == file)
    };
    let mut out = Vec::new();
    for (key, b) in &old {
        if new.contains_key(key) || !touched(before, key) {
            continue;
        }
        out.push(Change::Removed {
            entity: key.clone(),
            file: file_of(before, key).into(),
            before: b.clone(),
        });
    }
    for (key, a) in &new {
        let file = file_of(after, key).to_string();
        let Some(b) = old.get(key) else {
            out.push(Change::Added {
                entity: key.clone(),
                file,
                after: a.clone(),
            });
            continue;
        };
        if a == b {
            continue;
        }
        let (lb, la) = (fragment_lists(key, b), fragment_lists(key, a));
        let mut seen = std::collections::BTreeSet::new();
        for (owner, path) in lb.keys().chain(la.keys()) {
            if !seen.insert((owner, path)) {
                continue;
            }
            let k = (owner.clone(), path.clone());
            let (fb, fa) = (lb.get(&k), la.get(&k));
            if fb != fa {
                let (fb, fa) = (
                    fb.map_or(&[][..], Vec::as_slice),
                    fa.map_or(&[][..], Vec::as_slice),
                );
                fragment_changes(key, &file, owner, path, fb, fa, &mut out);
            }
        }
        let (bm, am) = (plain_fields(key, b), plain_fields(key, a));
        let fields: std::collections::BTreeSet<&String> = bm.keys().chain(am.keys()).collect();
        for field in fields {
            let (fb, fa) = (bm.get(field), am.get(field));
            if fb != fa {
                out.push(Change::Field {
                    entity: key.clone(),
                    file: file.clone(),
                    field: field.clone(),
                    before: fb.cloned().unwrap_or(Value::Null),
                    after: fa.cloned().unwrap_or(Value::Null),
                });
            }
        }
    }
    out
}

/// The fields that hold fragment lists, per entity kind, with the runtime owner of each.
fn list_fields(entity: &str) -> &'static [&'static str] {
    match entity.split(':').next() {
        Some("room") => &["description"],
        Some("thing") => &["description", "presence"],
        Some("story") => &["intro"],
        _ => &[],
    }
}

/// The runtime owner of a fragment list, as `Source.owner` names it.
fn owner_of(entity: &str, path: &[&str]) -> String {
    match path {
        ["intro"] => "story:intro".into(),
        ["description"] => entity.into(),
        ["presence"] => format!("{entity}/presence"),
        ["frames", verb, part] => format!("{entity}/frames/{verb}/{part}"),
        _ => format!("{entity}/{}", path.join("/")),
    }
}

/// Every fragment list of an entity, keyed by runtime owner and YAML path.
fn fragment_lists(entity: &str, v: &Value) -> BTreeMap<(String, Vec<String>), Vec<Value>> {
    let mut out = BTreeMap::new();
    let list = |x: &Value| x.as_array().cloned().unwrap_or_default();
    for field in list_fields(entity) {
        if let Some(x) = v.get(field) {
            out.insert(
                (owner_of(entity, &[field]), vec![field.to_string()]),
                list(x),
            );
        }
    }
    let frames = v.get("frames").and_then(Value::as_object);
    for (verb, frame) in frames.into_iter().flatten() {
        for part in ["before", "instead", "after"] {
            if let Some(x) = frame.get(part) {
                let path = vec!["frames".to_string(), verb.clone(), part.to_string()];
                out.insert((owner_of(entity, &["frames", verb, part]), path), list(x));
            }
        }
    }
    out
}

/// The fields of an entity that are not fragment lists or frames.
fn plain_fields(entity: &str, v: &Value) -> Map<String, Value> {
    let mut map = v.as_object().cloned().unwrap_or_default();
    for field in list_fields(entity).iter().chain(&["frames"]) {
        map.remove(*field);
    }
    map
}

/// A fragment in one form, whether the YAML wrote it as a string or a map.
fn normalised(v: &Value) -> Value {
    match serde_json::from_value::<lantern_model::Fragment>(v.clone()) {
        Ok(f) => serde_json::json!({
            "id": f.id,
            "variants": f.variants,
            "when": f.when,
            "mode": f.mode,
            "layer": f.layer,
            "replaces": f.replaces,
        }),
        Err(_) => Value::Null,
    }
}

fn fragment_id(v: &Value) -> Option<String> {
    v.get("id").and_then(Value::as_str).map(str::to_string)
}

/// Fragment changes from a longest common subsequence of the two lists. A removed
/// fragment next to an added one at the same place reads as one changed fragment.
fn fragment_changes(
    entity: &str,
    file: &str,
    owner: &str,
    field: &[String],
    b: &[Value],
    a: &[Value],
    out: &mut Vec<Change>,
) {
    let (n, m) = (b.len(), a.len());
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if b[i] == a[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let change = |bi: Option<usize>, ai: Option<usize>| Change::Fragment {
        entity: entity.into(),
        file: file.into(),
        owner: owner.into(),
        field: field.to_vec(),
        fragment_id: ai
            .and_then(|i| fragment_id(&a[i]))
            .or_else(|| bi.and_then(|i| fragment_id(&b[i]))),
        before_index: bi,
        after_index: ai,
        before: bi.map(|i| b[i].clone()),
        after: ai.map(|i| a[i].clone()),
        before_fragment: bi.map(|i| normalised(&b[i])),
        after_fragment: ai.map(|i| normalised(&a[i])),
    };
    // Walk the diff. Between two common fragments lies one run of removals and
    // additions; pair them up, and each pair is one changed fragment.
    let mut flush = |removed: &mut Vec<usize>, added: &mut Vec<usize>| {
        for x in 0..removed.len().max(added.len()) {
            out.push(change(removed.get(x).copied(), added.get(x).copied()));
        }
        removed.clear();
        added.clear();
    };
    let (mut removed, mut added) = (Vec::new(), Vec::new());
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && b[i] == a[j] {
            flush(&mut removed, &mut added);
            i += 1;
            j += 1;
        } else if j < m && (i == n || lcs[i][j + 1] >= lcs[i + 1][j]) {
            added.push(j);
            j += 1;
        } else {
            removed.push(i);
            i += 1;
        }
    }
    flush(&mut removed, &mut added);
}

/// The text of each added or changed room in all its states.
fn previews(world: &Arc<World>, changes: &[Change]) -> Vec<RoomPreview> {
    let mut rooms: Vec<&str> = changes
        .iter()
        .filter_map(|c| match c {
            Change::Added { entity, .. }
            | Change::Field { entity, .. }
            | Change::Fragment { entity, .. } => entity.strip_prefix("room:"),
            Change::Removed { .. } => None,
        })
        .collect();
    rooms.sort_unstable();
    rooms.dedup();
    let game = Game::new(world.clone());
    rooms
        .into_iter()
        .filter_map(|room| {
            let m = lantern_runtime::room_matrix(&game, room)?;
            let mut texts: Vec<PreviewText> = Vec::new();
            for cell in m.cells {
                match texts.iter_mut().find(|t| t.text == cell.text) {
                    Some(t) => t.combinations += 1,
                    None => texts.push(PreviewText {
                        text: cell.text,
                        combinations: 1,
                        example: m
                            .dimensions
                            .iter()
                            .zip(&cell.values)
                            .map(|(d, v)| format!("{}={v}", d.name))
                            .collect(),
                        warnings: cell.warnings,
                    }),
                }
            }
            Some(RoomPreview {
                room: room.to_string(),
                texts,
            })
        })
        .collect()
}
