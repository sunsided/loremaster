use crate::hash::value_hash;
use crate::merge::{
    Extension, MergeMode, Override, extend_entity, merge_mode, take_override, three_way,
};
use crate::{Diagnostic, Origin, Severity};
use loremaster_model::{
    Agenda, Asset, ClockConfig, Dialogue, DialogueId, Effect, Ending, Fragment, Id, Machine, Note,
    Room, Rule, Story, TextPool, Thing, Thread, Timer, VarDef, VerbDef, Walkthrough,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A file of a project, with its path relative to the project folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: String,
    pub text: String,
}

/// A loaded project: the story plus where each entity came from.
#[derive(Debug, Clone)]
pub struct Project {
    pub story: Story,
    pub files: Vec<SourceFile>,
    /// Entity key such as `room:cellar` to where it is defined.
    pub origins: BTreeMap<String, Origin>,
    /// Walkthrough tests from `tests/`, with their file paths.
    pub walkthroughs: Vec<(String, Walkthrough)>,
    /// Warnings from joining the packs, for example a replaced entity.
    /// `check` reports them first.
    pub assembly: Vec<Diagnostic>,
    /// The entities that Rust makes, as Rust makes them: before any YAML
    /// `extend`, `replace` or override. The editor shows, ejects and
    /// overrides these.
    pub rust_parts: BTreeMap<String, RustPart>,
}

/// An entity from Rust, for the editor (DS-10, DS-12).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RustPart {
    /// Always an [`Origin::Rust`], with the content hash.
    pub origin: Origin,
    /// The canonical model value.
    pub value: serde_json::Value,
    /// The commit that the pack was exported from, when it is known.
    pub commit: Option<String>,
    /// The YAML file that overrides this part, and how it stands.
    pub overridden_by: Option<(String, OverrideState)>,
}

/// How an override stands against its Rust part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverrideState {
    /// Rust did not change since the override was made.
    Active,
    /// Rust now equals the override, so the file can go.
    NoLongerNeeded,
}

impl Project {
    pub fn file(&self, path: &str) -> Option<&SourceFile> {
        self.files.iter().find(|f| f.path == path)
    }

    /// Give a diagnostic without a file the file of its entity. `at` starts
    /// with the entity key, as in `room:shore/exits[0]` or `story:intro`,
    /// which is how a compile error names its place.
    pub fn locate(&self, mut diagnostic: Diagnostic) -> Diagnostic {
        if diagnostic.file.is_empty() {
            // A whole rule key first: the rule of a machine step is
            // `rule:lock#1`. Other keys keep `#` for a fragment: `room:vault#2`.
            let whole = diagnostic.at.split('/').next().unwrap_or_default();
            let key = if whole.starts_with("rule:") && self.origins.contains_key(whole) {
                whole
            } else {
                diagnostic.at.split(['/', '#']).next().unwrap_or_default()
            };
            let key = if key.starts_with("story:") {
                "story"
            } else {
                key
            };
            diagnostic.file = self
                .origins
                .get(key)
                .map(|o| o.file().to_string())
                .unwrap_or_else(|| "story.yaml".into());
        }
        diagnostic
    }
}

fn yes() -> bool {
    true
}

/// The story header (`story.yaml` in a YAML folder): everything that is not a
/// room, thing or rule. Exactly one pack of a project gives it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryHeader {
    pub title: String,
    #[serde(default)]
    pub author: Option<String>,
    pub start: Id,
    #[serde(default)]
    pub intro: Vec<Fragment>,
    #[serde(default)]
    pub clock: ClockConfig,
    #[serde(default)]
    pub vars: Vec<VarDef>,
    #[serde(default)]
    pub verbs: Vec<VerbDef>,
    #[serde(default)]
    pub relations: Vec<Id>,
    #[serde(default)]
    pub frames: loremaster_model::Frames,
    #[serde(default)]
    pub seed: u64,
    /// `same` (strict) or `fresh` (forgiving): the next roll after a load.
    #[serde(
        default,
        skip_serializing_if = "loremaster_model::ReloadRolls::is_same"
    )]
    pub reload_rolls: loremaster_model::ReloadRolls,
    /// `false` turns off the death prompt (SE-9).
    #[serde(default = "yes")]
    pub death_prompt: bool,
    /// How many things the player can hold (WO-3).
    #[serde(default)]
    pub carry_limit: Option<u32>,
    #[serde(default)]
    pub playable: Vec<Id>,
    /// The most `on: next` machine steps in one turn (P5-02).
    #[serde(default)]
    pub step_limit: Option<u32>,
    #[serde(default)]
    pub notes: Vec<Note>,
    #[serde(default)]
    pub timers: Vec<Timer>,
    #[serde(default)]
    pub machines: Vec<Machine>,
    #[serde(default)]
    pub behaviors: Vec<Id>,
    #[serde(default)]
    pub score: Option<loremaster_model::Score>,
    /// The adventure sheet of the player.
    #[serde(default)]
    pub stats: Vec<loremaster_model::Stat>,
    /// The contract with the host (IF-1).
    #[serde(default)]
    pub interface: Option<loremaster_model::Interface>,
    /// What the episode of this story carries to the next story (TX-12).
    #[serde(default)]
    pub export: loremaster_model::EpisodeItems,
    /// What this story takes from the episode of the story before (TX-12).
    #[serde(default)]
    pub import: loremaster_model::EpisodeItems,
}
/// Load a project from its files: one YAML pack, assembled. Paths use `/` and
/// are relative to the project folder.
pub fn load(files: Vec<SourceFile>) -> Result<Project, Vec<Diagnostic>> {
    assemble(packs_of(files)?)
}

/// The packs of a folder: each exported Rust pack (`packs/*.yaml`) in path
/// order, then the YAML files.
pub(crate) fn packs_of(mut files: Vec<SourceFile>) -> Result<Vec<Pack>, Vec<Diagnostic>> {
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut packs = Vec::new();
    let mut errors = Vec::new();
    // A file in `packs/` that is not read would drop Rust parts without a word.
    for f in files.iter().filter(|f| f.path.starts_with("packs/")) {
        let message = if is_exported_pack(&f.path) {
            continue;
        } else if is_old_json_pack(&f.path) {
            "an old JSON pack: packs are YAML now; run `loremaster export-pack` again, which removes this file"
        } else {
            "not read: an exported pack is `packs/<name>.yaml`, at the top of `packs/`; move or delete this file"
        };
        errors.push(Diagnostic::error(&f.path, "", message.to_string()));
    }
    for f in files.iter().filter(|f| is_exported_pack(&f.path)) {
        // Every export has a name, so an empty file is not one.
        match serde_saphyr::from_str::<Pack>(&f.text) {
            Ok(pack) if !pack.name.is_empty() => packs.push(pack),
            found => errors.push(Diagnostic::error(
                &f.path,
                "",
                format!(
                    "not an exported pack: {}",
                    found
                        .err()
                        .map_or("no `name`".to_string(), |e| e.to_string())
                ),
            )),
        }
    }
    match Pack::from_files(files) {
        Ok(pack) if errors.is_empty() => packs.push(pack),
        Ok(_) => return Err(errors),
        Err(mut found) => {
            errors.append(&mut found);
            return Err(errors);
        }
    }
    Ok(packs)
}

/// Only the top level of `packs/`, which `export_packs` writes and cleans.
fn is_exported_pack(path: &str) -> bool {
    path.strip_prefix("packs/")
        .is_some_and(|name| !name.contains('/') && yaml_stem(name).is_some())
}

fn is_old_json_pack(path: &str) -> bool {
    path.strip_prefix("packs/")
        .is_some_and(|name| !name.contains('/') && name.ends_with(".json"))
}

/// The parts of a project that one source gives (DS-1): a YAML folder, and
/// later a `world!` block or a builder function. [`assemble`] joins packs.
///
/// As YAML it is the file that `loremaster export-pack` writes. The parts
/// that only a running program can have (`files`, `warnings`, `overrides`,
/// `instances`) are not in it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Pack {
    /// The name in diagnostics, for example `yaml` or a Rust module.
    pub name: String,
    /// The story header. Exactly one pack of a project gives it.
    pub header: Option<StoryHeader>,
    pub rooms: Vec<Room>,
    pub things: Vec<Thing>,
    /// Rules, also the ones of rooms and things (with their scope set).
    pub rules: Vec<Rule>,
    pub threads: Vec<Thread>,
    pub dialogues: Vec<Dialogue>,
    pub machines: Vec<Machine>,
    pub walkthroughs: Vec<(String, Walkthrough)>,
    pub behaviors: Vec<Id>,
    pub verbs: Vec<VerbDef>,
    pub timers: Vec<Timer>,
    pub endings: Vec<Ending>,
    /// Media files with their licence and caption, from `assets/`.
    pub assets: Vec<Asset>,
    /// Named text pools, from `texts/`.
    pub texts: Vec<TextPool>,
    /// What actors do on their own (D6).
    pub agendas: Vec<Agenda>,
    /// The source files, for diagnostics with lines.
    #[serde(skip)]
    pub files: Vec<SourceFile>,
    /// Entity key such as `room:cellar` to where it is defined. The key
    /// `story` is the origin of the header.
    pub origins: BTreeMap<String, Origin>,
    /// Entity keys that this pack replaces on purpose (`replace: true`).
    pub replaces: BTreeSet<String>,
    /// Fields that this pack adds to entities of earlier packs (`extend: true`).
    pub extensions: Vec<Extension>,
    /// The YAML files of this pack that override a Rust part (`override:`).
    #[serde(skip)]
    pub overrides: Vec<Override>,
    /// The commit of an exported pack, when it is known.
    pub commit: Option<String>,
    /// Warnings from reading the pack, for example a folder that the loader
    /// does not read yet. `assemble` passes them on.
    #[serde(skip)]
    pub warnings: Vec<Diagnostic>,
    /// Rust values that the pack hands to the runtime: loremaster-dsl keeps
    /// the instance of each Rust behavior here: an
    /// `Arc<dyn loremaster_runtime::Behavior>`, behind the type-erasing
    /// `Arc<dyn Any + Send + Sync>` of this field. Add them with its
    /// `BuildPack::register` or `register_block`; `Loremaster::build`
    /// ignores any other value.
    /// The project does not read them.
    #[serde(skip)]
    pub instances: Vec<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
}

impl Pack {
    pub fn new(name: impl Into<String>) -> Self {
        Pack {
            name: name.into(),
            ..Pack::default()
        }
    }

    /// The value of the entity `key` (`room:cellar`) in canonical form.
    pub fn entity_value(&self, key: &str) -> Option<serde_json::Value> {
        fn find<T: Serialize>(
            list: &[T],
            id: &str,
            id_of: fn(&T) -> &str,
        ) -> Option<serde_json::Value> {
            let e = list.iter().find(|e| id_of(e) == id)?;
            serde_json::to_value(e).ok()
        }
        if key == "story" {
            return serde_json::to_value(self.header.as_ref()?).ok();
        }
        let (kind, id) = key.split_once(':')?;
        match kind {
            "room" => find(&self.rooms, id, |e| e.id.as_str()),
            "thing" => find(&self.things, id, |e| e.id.as_str()),
            "rule" => find(&self.rules, id, |e| e.id.as_str()),
            "thread" => find(&self.threads, id, |e| e.id.as_str()),
            "dialogue" => find(&self.dialogues, id, |e| e.id_str()),
            "ending" => find(&self.endings, id, |e| e.id.as_str()),
            "timer" => find(&self.timers, id, |e| e.id.as_str()),
            "verb" => find(&self.verbs, id, |e| e.id.as_str()),
            "machine" => find(&self.machines, id, |e| e.id.as_str()),
            "agenda" => find(&self.agendas, id, |e| e.id.as_str()),
            "text" => find(&self.texts, id, |e| e.id.as_deref().unwrap_or_default()),
            "behavior" => self
                .behaviors
                .iter()
                .any(|b| b.as_str() == id)
                .then(|| serde_json::Value::String(id.to_string())),
            _ => None,
        }
    }

    /// Fill in the content hash of each entity from Rust (DS-11).
    pub fn stamp_hashes(&mut self) {
        let stamped: Vec<(String, String)> = self
            .origins
            .iter()
            .filter(|(_, o)| matches!(o, Origin::Rust { .. }))
            .filter_map(|(k, _)| Some((k.clone(), value_hash(&self.entity_value(k)?))))
            .collect();
        for (key, new) in stamped {
            if let Some(Origin::Rust { hash, .. }) = self.origins.get_mut(&key) {
                *hash = new;
            }
        }
    }

    /// The pack as the YAML of `loremaster export-pack`, with its hashes.
    pub fn to_yaml(&mut self) -> String {
        self.stamp_hashes();
        // A pack is plain data with string keys, as in `model_hash`.
        serde_saphyr::to_string(self).expect("a pack always serializes to YAML")
    }

    /// A pack from the files of a YAML folder. Paths use `/` and are relative
    /// to the folder. On an error, the list also holds the warnings so far.
    pub fn from_files(files: Vec<SourceFile>) -> Result<Pack, Vec<Diagnostic>> {
        let mut errors = Vec::new();
        let mut origins = BTreeMap::new();
        let mut header: Option<StoryHeader> = None;
        // `story.yaml` or `story.yml`: the origin names the file that gave the header.
        let mut header_path = String::new();
        let mut rooms = Vec::new();
        let mut things = Vec::new();
        let mut rules = Vec::new();
        let mut threads = Vec::new();
        let mut dialogues = Vec::new();
        let mut machines = Vec::new();
        let mut walkthroughs = Vec::new();
        let mut behaviors: Vec<Id> = Vec::new();
        let mut verbs: Vec<VerbDef> = Vec::new();
        let mut timers: Vec<Timer> = Vec::new();
        let mut endings: Vec<Ending> = Vec::new();
        let mut assets: Vec<Asset> = Vec::new();
        let mut texts: Vec<TextPool> = Vec::new();
        let mut agendas: Vec<Agenda> = Vec::new();
        let mut replaces = BTreeSet::new();
        let mut extensions = Vec::new();
        let mut overrides = Vec::new();

        let mut sorted: Vec<&SourceFile> = files.iter().collect();
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        let mut unknown_folders = BTreeSet::new();
        let mut warnings = Vec::new();

        for file in sorted {
            let path = file.path.as_str();
            let Some(stem) = yaml_stem(path) else {
                continue;
            };
            let dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            // Exported Rust packs, read by `packs_of`.
            if dir == "packs" || dir.starts_with("packs/") {
                continue;
            }
            if dir.is_empty() && stem != "story" {
                errors.push(unknown_file(path));
                continue;
            }
            if !dir.is_empty() && !FOLDERS.contains(&dir) {
                // One message for each folder, at its first file.
                if unknown_folders.insert(dir) {
                    errors.push(unknown_folder(path, dir));
                }
                continue;
            }
            let parse_err = |e: serde_saphyr::Error| parse_diagnostic(path, &e.to_string());
            let (mode, file_text) = match merge_mode(&file.text) {
                Ok(found) => found,
                Err(message) => {
                    errors.push(parse_diagnostic(path, &message));
                    continue;
                }
            };
            let (over, file_text) = match take_override(&file_text) {
                Ok(found) => found,
                Err(message) => {
                    errors.push(parse_diagnostic(path, &message));
                    continue;
                }
            };
            if let Some(base) = over {
                let kind = if dir == "rules" {
                    Some("rule")
                } else {
                    entity_kind(dir)
                };
                let matches_folder = kind.is_some_and(|k| {
                    base.entity
                        .split_once(':')
                        .is_some_and(|(have, id)| have == k && id == stem)
                });
                if mode.is_some() || !matches_folder {
                    errors.push(parse_diagnostic(
                        path,
                        &format!(
                            "an `override:` of `{}` needs its own file in the folder of its kind, named after its id, without `extend` or `replace`",
                            base.entity
                        ),
                    ));
                    continue;
                }
                replaces.insert(base.entity.clone());
                overrides.push(Override {
                    base,
                    path: path.to_string(),
                });
            }
            if let Some(mode) = mode {
                let Some(kind) = entity_kind(dir) else {
                    errors.push(parse_diagnostic(
                        path,
                        "`extend` and `replace` work only in rooms/, things/, threads/, dialogues/, endings/, timers/, verbs/, machines/ and agendas/",
                    ));
                    continue;
                };
                // The id from the parsed file (`id:` in any YAML style), else
                // the file name.
                let fields = match serde_saphyr::from_str::<serde_json::Value>(&file_text) {
                    Ok(fields) => fields,
                    Err(e) => {
                        errors.push(parse_err(e));
                        continue;
                    }
                };
                let id = fields
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(stem);
                let key = format!("{kind}:{id}");
                match mode {
                    MergeMode::Replace => {
                        replaces.insert(key);
                    }
                    MergeMode::Extend => {
                        extensions.push(Extension {
                            key,
                            fields,
                            origin: Origin::yaml(path),
                        });
                        continue;
                    }
                }
            }
            match dir {
                "" if stem == "story" => match serde_saphyr::from_str::<StoryHeader>(&file_text) {
                    Ok(s) => {
                        header = Some(s);
                        header_path = path.to_string();
                    }
                    Err(e) => errors.push(parse_err(e)),
                },
                "rooms" => match serde_saphyr::from_str::<Room>(&with_default_id(&file_text, stem))
                {
                    Ok(mut r) => {
                        origins.insert(format!("room:{}", r.id), Origin::yaml(path));
                        for rule in nested_rules(&mut r.rules, &r.id) {
                            origins.insert(format!("rule:{}", rule.id), Origin::yaml(path));
                            rules.push(rule);
                        }
                        rooms.push(r);
                    }
                    Err(e) => errors.push(parse_err(e)),
                },
                "things" => {
                    match serde_saphyr::from_str::<Thing>(&with_default_id(&file_text, stem)) {
                        Ok(mut t) => {
                            origins.insert(format!("thing:{}", t.id), Origin::yaml(path));
                            for rule in nested_rules(&mut t.rules, &t.id) {
                                origins.insert(format!("rule:{}", rule.id), Origin::yaml(path));
                                rules.push(rule);
                            }
                            things.push(t);
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                "threads" => {
                    match serde_saphyr::from_str::<Thread>(&with_default_id(&file_text, stem)) {
                        Ok(t) => {
                            origins.insert(format!("thread:{}", t.id), Origin::yaml(path));
                            threads.push(t);
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                "dialogues" => {
                    // An `id` in the file wins over the file stem.
                    match serde_saphyr::from_str::<Dialogue>(&file_text) {
                        Ok(d) => {
                            let d = match d.id {
                                Some(_) => d,
                                None => d.with_id(stem),
                            };
                            origins.insert(format!("dialogue:{}", d.id_str()), Origin::yaml(path));
                            dialogues.push(d);
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                // Commands such as `n` and `y` are words here, not YAML 1.1 booleans.
                "tests" => match serde_saphyr::from_str_with_options::<Walkthrough>(
                    &file_text,
                    serde_saphyr::options! { strict_booleans: true },
                ) {
                    Ok(w) => walkthroughs.push((path.to_string(), w)),
                    Err(e) => errors.push(parse_err(e)),
                },
                // A file that names a behavior in Rust; the text holds author notes about it.
                "behaviors" => {
                    origins.insert(format!("behavior:{stem}"), Origin::yaml(path));
                    behaviors.push(Id::new(stem));
                }
                "assets" => {
                    match serde_saphyr::from_str::<Asset>(&with_default_id(&file_text, stem)) {
                        Ok(a) => {
                            origins.insert(format!("asset:{}", a.id), Origin::yaml(path));
                            assets.push(a);
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                "endings" => {
                    match serde_saphyr::from_str::<Ending>(&with_default_id(&file_text, stem)) {
                        Ok(e) => {
                            origins.insert(format!("ending:{}", e.id), Origin::yaml(path));
                            endings.push(e);
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                // Rules and dialogue lines name a pool by its id: the file
                // name, or an `id` in the file, as in the other folders.
                "texts" => match serde_saphyr::from_str::<TextPool>(&file_text) {
                    Ok(mut t) => {
                        let id = t.id.get_or_insert_with(|| stem.to_string());
                        origins.insert(format!("text:{id}"), Origin::yaml(path));
                        texts.push(t);
                    }
                    Err(e) => errors.push(parse_err(e)),
                },
                "timers" => {
                    match serde_saphyr::from_str::<Timer>(&with_default_id(&file_text, stem)) {
                        Ok(t) => {
                            origins.insert(format!("timer:{}", t.id), Origin::yaml(path));
                            timers.push(t);
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                "verbs" => {
                    match serde_saphyr::from_str::<VerbDef>(&with_default_id(&file_text, stem)) {
                        Ok(v) => {
                            origins.insert(format!("verb:{}", v.id), Origin::yaml(path));
                            verbs.push(v);
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                "agendas" => {
                    match serde_saphyr::from_str::<Agenda>(&with_default_id(&file_text, stem)) {
                        Ok(a) => {
                            origins.insert(format!("agenda:{}", a.id), Origin::yaml(path));
                            agendas.push(a);
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                "machines" => {
                    match serde_saphyr::from_str::<Machine>(&with_default_id(&file_text, stem)) {
                        Ok(m) => {
                            origins.insert(format!("machine:{}", m.id), Origin::yaml(path));
                            machines.push(m);
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                "rules" => {
                    let list = file_text
                        .lines()
                        .map(str::trim_start)
                        .find(|l| !l.is_empty() && !l.starts_with('#'))
                        .is_some_and(|l| l.starts_with('-'));
                    let parsed = if list {
                        serde_saphyr::from_str::<Vec<Rule>>(&file_text)
                    } else {
                        serde_saphyr::from_str::<Rule>(&with_default_id(&file_text, stem))
                            .map(|r| vec![r])
                    };
                    match parsed {
                        Ok(list) => {
                            for r in list {
                                origins.insert(format!("rule:{}", r.id), Origin::yaml(path));
                                rules.push(r);
                            }
                        }
                        Err(e) => errors.push(parse_err(e)),
                    }
                }
                _ => {}
            }
            // An `id:` in the body that is not the overridden one would leave
            // the Rust part in place. Checked now, before a later file with
            // the same entity takes its origin.
            if let Some(o) = overrides.last().filter(|o| o.path == path) {
                let defined = origins.get(&o.base.entity).and_then(Origin::yaml_path);
                if defined != Some(path) && !errors.iter().any(|d| d.file == path) {
                    errors.push(parse_diagnostic(
                        path,
                        &format!(
                            "this file overrides `{}`, but does not define it",
                            o.base.entity
                        ),
                    ));
                }
            }
        }

        if !errors.is_empty() {
            if header.is_none() {
                errors.push(no_story());
            }
            errors.append(&mut warnings);
            return Err(errors);
        }
        if header.is_some() {
            origins.insert("story".into(), Origin::yaml(header_path));
        }
        Ok(Pack {
            name: "yaml".into(),
            header,
            rooms,
            things,
            rules,
            threads,
            dialogues,
            machines,
            walkthroughs,
            behaviors,
            verbs,
            timers,
            endings,
            assets,
            texts,
            agendas,
            files,
            origins,
            replaces,
            extensions,
            overrides,
            commit: None,
            warnings,
            instances: Vec::new(),
        })
    }
}

/// The folders of a YAML project. A YAML file in any other folder is an
/// error, so that a misspelled folder such as `room/` does not load as
/// nothing. `behaviors/` holds notes about behaviors in Rust, `tests/` the
/// walkthroughs.
pub const FOLDERS: &[&str] = &[
    "rooms",
    "things",
    "rules",
    "threads",
    "dialogues",
    "endings",
    "timers",
    "verbs",
    "machines",
    "texts",
    "agendas",
    "behaviors",
    "tests",
    "assets",
];

fn unknown_folder(path: &str, dir: &str) -> Diagnostic {
    let hint = match loremaster_lang::closest(dir, FOLDERS.iter().copied()) {
        Some(near) => format!("did you mean `{near}/`?"),
        None => format!(
            "a project has story.yaml and the folders {}",
            FOLDERS
                .iter()
                .map(|f| format!("{f}/"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    Diagnostic::error(path, "", format!("unknown folder `{dir}/`: {hint}"))
}

fn unknown_file(path: &str) -> Diagnostic {
    let hint = match yaml_stem(path).and_then(|s| loremaster_lang::closest(s, ["story"])) {
        Some(_) => "did you mean `story.yaml`?".to_string(),
        None => "the top folder holds only story.yaml; put entities in their folders".to_string(),
    };
    Diagnostic::error(path, "", format!("unknown file `{path}`: {hint}"))
}

/// Add a "did you mean" to serde's message for an unknown field: `unknown
/// field `descripton`, expected one of …` (serde-saphyr writes the names
/// without backticks).
pub(crate) fn suggest_field(message: &str) -> String {
    let Some((head, rest)) = message.split_once("unknown field `") else {
        return message.to_string();
    };
    let Some((field, expected)) = rest.split_once("`, expected ") else {
        return message.to_string();
    };
    let known = expected
        .trim_start_matches("one of ")
        .split(", ")
        .flat_map(|s| s.split(" or "))
        .map(|s| s.trim_matches('`'));
    match loremaster_lang::closest(field, known) {
        Some(near) => format!("{head}unknown field `{field}`: did you mean `{near}`?"),
        None => message.to_string(),
    }
}

/// The entity kind of a folder, for `extend` and `replace`.
fn entity_kind(dir: &str) -> Option<&'static str> {
    Some(match dir {
        "rooms" => "room",
        "things" => "thing",
        "threads" => "thread",
        "dialogues" => "dialogue",
        "endings" => "ending",
        "timers" => "timer",
        "verbs" => "verb",
        "machines" => "machine",
        "agendas" => "agenda",
        _ => return None,
    })
}

fn no_story() -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        file: "story.yaml".into(),
        at: String::new(),
        message: "the project has no story.yaml".into(),
        line: None,
        column: None,
    }
}

/// Join packs in their order into one project (DS-1). An id that two packs
/// define is an error that names both origins, and exactly one pack gives the
/// story header. Machine rules and machine variables are made here, after the
/// merge, so they are the same for each source. On an error, the list also
/// holds the warnings so far, such as a replaced entity.
pub fn assemble(packs: Vec<Pack>) -> Result<Project, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    let mut assembly = Vec::new();
    let mut origins: BTreeMap<String, Origin> = BTreeMap::new();
    let mut owner: BTreeMap<String, usize> = BTreeMap::new();
    let mut header: Option<StoryHeader> = None;
    let mut files = Vec::new();
    let mut rooms = Vec::new();
    let mut things = Vec::new();
    let mut rules = Vec::new();
    let mut threads = Vec::new();
    let mut dialogues = Vec::new();
    let mut machines = Vec::new();
    let mut walkthroughs = Vec::new();
    let mut behaviors: Vec<Id> = Vec::new();
    let mut verbs: Vec<VerbDef> = Vec::new();
    let mut timers: Vec<Timer> = Vec::new();
    let mut assets: Vec<Asset> = Vec::new();
    let mut endings: Vec<Ending> = Vec::new();
    let mut texts: Vec<TextPool> = Vec::new();
    let mut agendas: Vec<Agenda> = Vec::new();
    let any_rust = packs
        .iter()
        .any(|p| p.origins.values().any(|o| matches!(o, Origin::Rust { .. })));
    let mut rust_parts: BTreeMap<String, RustPart> = BTreeMap::new();
    for (i, mut pack) in packs.into_iter().enumerate() {
        assembly.append(&mut pack.warnings);
        pack.stamp_hashes();
        // First the overrides of earlier packs' Rust parts: this pack's own
        // Rust parts cannot be their base.
        let mut overridden = BTreeSet::new();
        for over in std::mem::take(&mut pack.overrides) {
            let key = over.base.entity.clone();
            let value = pack.entity_value(&key);
            let Some(part) = rust_parts.get_mut(&key) else {
                if any_rust {
                    errors.push(Diagnostic::error(
                        &over.path,
                        "",
                        format!("`{key}` is overridden, but no Rust part has this id any more; delete the file or give the Rust part back"),
                    ));
                } else {
                    // A YAML-only load: the entity stands alone.
                    assembly.push(Diagnostic::warning(
                        &over.path,
                        "",
                        format!("`{key}` is an override, but its Rust pack is not loaded, so it is not compared"),
                    ));
                }
                pack.replaces.remove(&key);
                continue;
            };
            let Origin::Rust { hash, .. } = &part.origin else {
                continue;
            };
            overridden.insert(key.clone());
            if value.as_ref() == Some(&part.value) {
                assembly.push(Diagnostic::info(
                    &over.path,
                    "",
                    format!(
                        "override no longer needed: `{key}` in Rust now equals this file; delete it"
                    ),
                ));
                part.overridden_by = Some((over.path, OverrideState::NoLongerNeeded));
            } else if *hash == over.base.base_hash {
                assembly.push(Diagnostic::warning(
                    &over.path,
                    "",
                    format!("override active, port it to the DSL: `{key}` (`loremaster codegen` prints it)"),
                ));
                part.overridden_by = Some((over.path, OverrideState::Active));
            } else {
                let diff = three_way(
                    &over.base.base,
                    &part.value,
                    value.as_ref().unwrap_or(&serde_json::Value::Null),
                );
                errors.push(Diagnostic::error(
                    &over.path,
                    "",
                    format!("both sides changed: `{key}` changed in Rust and in this override. Merge them, then set `base_hash: {hash}` and `base` to the Rust value.{diff}"),
                ));
            }
        }
        for (key, origin) in &pack.origins {
            if matches!(origin, Origin::Rust { .. })
                && let Some(value) = pack.entity_value(key)
            {
                rust_parts.insert(
                    key.clone(),
                    RustPart {
                        origin: origin.clone(),
                        value,
                        commit: pack.commit.clone(),
                        overridden_by: None,
                    },
                );
            }
        }
        // A replaced room or thing takes its nested rules with it: they come
        // from the replaced file and run `within` the entity.
        for key in &pack.replaces {
            let Some(first) = origins.get(key).cloned() else {
                continue;
            };
            let Some(id) = key
                .strip_prefix("room:")
                .or_else(|| key.strip_prefix("thing:"))
            else {
                continue;
            };
            let nested: Vec<String> = rules
                .iter()
                .filter(|r: &&Rule| r.within.as_ref().is_some_and(|w| w.as_str() == id))
                .map(|r| format!("rule:{}", r.id))
                .filter(|k| origins.get(k) == Some(&first))
                .collect();
            rules.retain(|r| !nested.contains(&format!("rule:{}", r.id)));
            for k in &nested {
                origins.remove(k);
                owner.remove(k);
            }
        }
        for (key, origin) in pack.origins {
            // Within one pack, the checks report a duplicate, as before.
            if let Some(&j) = owner.get(&key)
                && j != i
                && pack.replaces.contains(&key)
            {
                let first = &origins[&key];
                if !overridden.contains(&key) {
                    assembly.push(Diagnostic::warning(
                        origin.file(),
                        "",
                        format!("`{key}` replaces the one in {first}"),
                    ));
                }
                remove_entity(
                    &key,
                    &mut rules,
                    &mut rooms,
                    &mut things,
                    &mut threads,
                    &mut dialogues,
                    &mut endings,
                    &mut timers,
                    &mut verbs,
                    &mut machines,
                    &mut agendas,
                );
                owner.insert(key.clone(), i);
                origins.insert(key, origin);
                continue;
            }
            if pack.replaces.contains(&key) {
                errors.push(Diagnostic::error(
                    origin.file(),
                    "",
                    format!("`{key}` replaces an entity that no earlier pack defines"),
                ));
            }
            if let Some(&j) = owner.get(&key)
                && j != i
            {
                let first = &origins[&key];
                let message = if key == "story" {
                    format!("two packs give the story header: {first} and {origin}")
                } else {
                    format!("`{key}` is defined twice: in {first} and in {origin}")
                };
                errors.push(Diagnostic {
                    severity: Severity::Error,
                    file: origin.file().to_string(),
                    at: String::new(),
                    message,
                    line: None,
                    column: None,
                });
                continue;
            }
            owner.insert(key.clone(), i);
            origins.insert(key, origin);
        }
        if header.is_none() {
            header = pack.header;
        }
        rooms.extend(pack.rooms);
        things.extend(pack.things);
        rules.extend(pack.rules);
        threads.extend(pack.threads);
        dialogues.extend(pack.dialogues);
        machines.extend(pack.machines);
        walkthroughs.extend(pack.walkthroughs);
        behaviors.extend(pack.behaviors);
        verbs.extend(pack.verbs);
        timers.extend(pack.timers);
        endings.extend(pack.endings);
        assets.extend(pack.assets);
        texts.extend(pack.texts);
        agendas.extend(pack.agendas);
        files.extend(pack.files);
        for ext in pack.extensions {
            let found = extend_with(
                &ext,
                &mut rooms,
                &mut things,
                &mut threads,
                &mut dialogues,
                &mut endings,
                &mut timers,
                &mut verbs,
                &mut machines,
                &mut agendas,
            );
            let message = match found {
                Ok(true) => continue,
                Ok(false) => format!(
                    "`{}` extends an entity that no earlier pack defines",
                    ext.key
                ),
                Err(why) => format!("`{}` cannot be extended: {}", ext.key, suggest_field(&why)),
            };
            errors.push(Diagnostic::error(ext.origin.file(), "", message));
        }
    }
    let header_origin = origins
        .get("story")
        .cloned()
        .unwrap_or_else(|| Origin::yaml("story.yaml"));
    let Some(header) = header else {
        errors.push(no_story());
        errors.append(&mut assembly);
        return Err(errors);
    };
    if !errors.is_empty() {
        errors.append(&mut assembly);
        return Err(errors);
    }
    for m in &header.machines {
        origins.insert(format!("machine:{}", m.id), header_origin.clone());
    }
    machines.splice(0..0, header.machines);
    let mut vars = header.vars;
    if let Some(score) = &header.score
        && !vars.iter().any(|v| v.id == score.var)
    {
        vars.push(VarDef {
            id: score.var.clone(),
            ty: loremaster_model::VarType::Int,
            initial: loremaster_model::Value::Int(0),
            min: None,
            max: None,
        });
    }
    for m in &machines {
        vars.push(VarDef {
            id: m.id.clone(),
            ty: loremaster_model::VarType::Enum(m.all_states()),
            initial: loremaster_model::Value::Text(m.start.clone()),
            min: None,
            max: None,
        });
        let file = origins
            .get(&format!("machine:{}", m.id))
            .cloned()
            .unwrap_or_else(|| header_origin.clone());
        for rule in machine_rules(m) {
            origins.insert(format!("rule:{}", rule.id), file.clone());
            rules.push(rule);
        }
    }
    let mut story = Story {
        title: header.title,
        author: header.author,
        start: header.start,
        intro: header.intro,
        clock: header.clock,
        vars,
        machines,
        verbs: header.verbs.into_iter().chain(verbs).collect(),
        score: header.score,
        rooms,
        things,
        rules,
        endings,
        assets,
        timers: header.timers.into_iter().chain(timers).collect(),
        agendas,
        behaviors: header
            .behaviors
            .into_iter()
            .chain(behaviors)
            .fold(Vec::new(), |mut all, b| {
                if !all.contains(&b) {
                    all.push(b);
                }
                all
            }),
        relations: header.relations,
        dialogues,
        texts,
        frames: header.frames,
        seed: header.seed,
        reload_rolls: header.reload_rolls,
        death_prompt: header.death_prompt,
        carry_limit: header.carry_limit,
        playable: header.playable,
        step_limit: header.step_limit,
        stats: header.stats,
        notes: header.notes,
        threads,
        interface: header.interface,
        export: header.export,
        import: header.import,
    };
    inherit_results(&mut story);
    Ok(Project {
        story,
        files,
        origins,
        walkthroughs,
        assembly,
        rust_parts,
    })
}

/// An `await` of a declared action can leave out `results`: it takes the
/// declared ones (IF-1).
fn inherit_results(story: &mut Story) {
    let Some(interface) = story.interface.clone() else {
        return;
    };
    for effect in story.rules.iter_mut().flat_map(|r| &mut r.then) {
        if let Effect::Await(spec) = effect
            && spec.results.is_empty()
            && let Some(action) = spec
                .action
                .as_ref()
                .and_then(|a| interface.action(a.as_str()))
        {
            spec.results = action.results.clone();
        }
    }
}

/// Take the entity of `key` (`room:cellar`) out of its list.
#[allow(clippy::too_many_arguments)]
fn remove_entity(
    key: &str,
    rules: &mut Vec<Rule>,
    rooms: &mut Vec<Room>,
    things: &mut Vec<Thing>,
    threads: &mut Vec<Thread>,
    dialogues: &mut Vec<Dialogue>,
    endings: &mut Vec<Ending>,
    timers: &mut Vec<Timer>,
    verbs: &mut Vec<VerbDef>,
    machines: &mut Vec<Machine>,
    agendas: &mut Vec<Agenda>,
) {
    let Some((kind, id)) = key.split_once(':') else {
        return;
    };
    match kind {
        "rule" => rules.retain(|e| e.id.as_str() != id),
        "room" => rooms.retain(|e| e.id.as_str() != id),
        "thing" => things.retain(|e| e.id.as_str() != id),
        "thread" => threads.retain(|e| e.id.as_str() != id),
        "dialogue" => dialogues.retain(|e| e.id_str() != id),
        "ending" => endings.retain(|e| e.id.as_str() != id),
        "timer" => timers.retain(|e| e.id.as_str() != id),
        "verb" => verbs.retain(|e| e.id.as_str() != id),
        "machine" => machines.retain(|e| e.id.as_str() != id),
        "agenda" => agendas.retain(|e| e.id.as_str() != id),
        _ => {}
    }
}

/// Add the fields of `ext` to the entity it names. `Ok(false)` when no
/// earlier pack defines the entity.
#[allow(clippy::too_many_arguments)]
fn extend_with(
    ext: &Extension,
    rooms: &mut [Room],
    things: &mut [Thing],
    threads: &mut [Thread],
    dialogues: &mut [Dialogue],
    endings: &mut [Ending],
    timers: &mut [Timer],
    verbs: &mut [VerbDef],
    machines: &mut [Machine],
    agendas: &mut [Agenda],
) -> Result<bool, String> {
    fn one<T: serde::Serialize + serde::de::DeserializeOwned>(
        list: &mut [T],
        id: &str,
        id_of: fn(&T) -> &str,
        fields: &serde_json::Value,
    ) -> Result<bool, String> {
        match list.iter_mut().find(|e| id_of(e) == id) {
            Some(e) => extend_entity(e, fields).map(|()| true),
            None => Ok(false),
        }
    }
    let Some((kind, id)) = ext.key.split_once(':') else {
        return Ok(false);
    };
    if ext.fields.get("rules").is_some() {
        return Err("put new rules in rules/, with `within`".into());
    }
    let f = &ext.fields;
    match kind {
        "room" => one(rooms, id, |e| e.id.as_str(), f),
        "thing" => one(things, id, |e| e.id.as_str(), f),
        "thread" => one(threads, id, |e| e.id.as_str(), f),
        "dialogue" => one(dialogues, id, |e| e.id_str(), f),
        "ending" => one(endings, id, |e| e.id.as_str(), f),
        "timer" => one(timers, id, |e| e.id.as_str(), f),
        "verb" => one(verbs, id, |e| e.id.as_str(), f),
        "machine" => one(machines, id, |e| e.id.as_str(), f),
        "agenda" => one(agendas, id, |e| e.id.as_str(), f),
        _ => Ok(false),
    }
}

/// The rules of a machine: one for each step. A step holds in one of its
/// `from` states and when its own `when` holds; it sets the new state first.
fn machine_rules(m: &Machine) -> Vec<Rule> {
    m.steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let from: Vec<String> = step
                .from
                .list()
                .iter()
                .map(|s| format!("{} == {s}", m.id))
                .collect();
            // A step without `from` never fires; the checker reports it.
            let from = if from.is_empty() {
                "false".to_string()
            } else {
                from.join(" or ")
            };
            let mut when = format!("({from})");
            if let Some(extra) = &step.when {
                when = format!("{when} and ({extra})");
            }
            let mut then = vec![Effect::Set(
                [(
                    m.id.to_string(),
                    loremaster_model::Value::Text(step.to.clone()),
                )]
                .into(),
            )];
            then.extend(step.say.iter().map(|s| Effect::Say(s.clone())));
            then.extend(step.then.iter().cloned());
            Rule {
                id: m.step_rule(i),
                on: step.on.clone(),
                when: Some(when),
                then,
                once: false,
                free: false,
                instead: step.instead,
                within: None,
                priority: 0,
                machine: Some(m.id.clone()),
                closes: step.closes.clone(),
                reason: step.reason.clone(),
                notes: Vec::new(),
            }
        })
        .collect()
}

/// Take the rules out of a room or thing file. They belong to it.
fn nested_rules(rules: &mut Vec<Rule>, owner: &Id) -> Vec<Rule> {
    std::mem::take(rules)
        .into_iter()
        .map(|mut r| {
            r.within.get_or_insert_with(|| owner.clone());
            r
        })
        .collect()
}

fn yaml_stem(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next()?;
    name.strip_suffix(".yaml")
        .or_else(|| name.strip_suffix(".yml"))
}

/// Entities take their id from the file name unless they set one.
fn with_default_id(text: &str, stem: &str) -> String {
    if text.lines().any(|l| l.starts_with("id:")) {
        text.to_string()
    } else {
        format!("id: {stem}\n{text}")
    }
}

/// serde-saphyr reports `... at line L column C`; pull those out.
fn parse_diagnostic(path: &str, message: &str) -> Diagnostic {
    let number_after = |word: &str| {
        message
            .split(word)
            .nth(1)
            .and_then(|rest| {
                rest.trim_start()
                    .split(|c: char| !c.is_ascii_digit())
                    .next()
            })
            .and_then(|n| n.parse::<u32>().ok())
    };
    let message = suggest_field(message.lines().next().unwrap_or(message));
    Diagnostic {
        severity: Severity::Error,
        file: path.to_string(),
        at: String::new(),
        message,
        line: number_after("line"),
        column: number_after("column"),
    }
}

/// The asset files that are not in the project folder (ME-3). This needs
/// the disk, so it is apart from [`check`].
#[cfg(not(target_family = "wasm"))]
pub fn missing_asset_files(project: &Project, root: &std::path::Path) -> Vec<Diagnostic> {
    (project.story.assets.iter())
        .filter(|a| safe_asset_path(&a.file) && !root.join(&a.file).is_file())
        .map(|a| {
            let key = format!("asset:{}", a.id);
            let file = project.origins.get(&key).map(|o| o.file().to_string());
            Diagnostic::error(
                file.unwrap_or_default(),
                key,
                format!("the file `{}` does not exist", a.file),
            )
        })
        .collect()
}

/// The path stays inside the project: relative, without `..`, and no URL.
/// Windows drops trailing dots and spaces of a name, so `.. ` is `..` there:
/// a name of only dots and spaces is refused, except `.`.
pub(crate) fn safe_asset_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with(['/', '\\'])
        && !path.contains(':')
        && !(path.split(['/', '\\'])).any(|part| {
            part != "." && !part.is_empty() && part.trim_end_matches(['.', ' ']).is_empty()
        })
}

/// Read every YAML file under a project folder, and old JSON packs under
/// `packs/`, so that loading can say to export them again.
#[cfg(not(target_family = "wasm"))]
pub fn read_dir(root: &std::path::Path) -> std::io::Result<Vec<SourceFile>> {
    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        out: &mut Vec<SourceFile>,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else if path.extension().is_some_and(|e| {
                e == "yaml"
                    || e == "yml"
                    || (e == "json"
                        && path
                            .strip_prefix(root)
                            .is_ok_and(|rel| rel.starts_with("packs")))
            }) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push(SourceFile {
                    path: rel,
                    text: std::fs::read_to_string(&path)?,
                });
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    Ok(out)
}
