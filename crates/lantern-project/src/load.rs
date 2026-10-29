use crate::{Diagnostic, Severity};
use lantern_model::{ClockConfig, Fragment, Id, Room, Rule, Story, Thing, VarDef, VerbDef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// Entity key such as `room:cellar` to the file that defines it.
    pub origins: BTreeMap<String, String>,
}

impl Project {
    pub fn file(&self, path: &str) -> Option<&SourceFile> {
        self.files.iter().find(|f| f.path == path)
    }
}

/// `story.yaml`: everything that is not a room, thing or rule.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoryFile {
    title: String,
    #[serde(default)]
    author: Option<String>,
    start: Id,
    #[serde(default)]
    intro: Vec<Fragment>,
    #[serde(default)]
    clock: ClockConfig,
    #[serde(default)]
    vars: Vec<VarDef>,
    #[serde(default)]
    verbs: Vec<VerbDef>,
    #[serde(default)]
    frames: lantern_model::Frames,
    #[serde(default)]
    seed: u64,
}

/// Load a project from its files. Paths use `/` and are relative to the
/// project folder.
pub fn load(files: Vec<SourceFile>) -> Result<Project, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    let mut origins = BTreeMap::new();
    let mut header: Option<StoryFile> = None;
    let mut rooms = Vec::new();
    let mut things = Vec::new();
    let mut rules = Vec::new();

    let mut sorted: Vec<&SourceFile> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));

    for file in sorted {
        let path = file.path.as_str();
        let Some(stem) = yaml_stem(path) else {
            continue;
        };
        let dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let parse_err = |e: serde_saphyr::Error| parse_diagnostic(path, &e.to_string());
        match dir {
            "" if stem == "story" => match serde_saphyr::from_str::<StoryFile>(&file.text) {
                Ok(s) => header = Some(s),
                Err(e) => errors.push(parse_err(e)),
            },
            "rooms" => match serde_saphyr::from_str::<Room>(&with_default_id(&file.text, stem)) {
                Ok(r) => {
                    origins.insert(format!("room:{}", r.id), path.to_string());
                    rooms.push(r);
                }
                Err(e) => errors.push(parse_err(e)),
            },
            "things" => match serde_saphyr::from_str::<Thing>(&with_default_id(&file.text, stem)) {
                Ok(t) => {
                    origins.insert(format!("thing:{}", t.id), path.to_string());
                    things.push(t);
                }
                Err(e) => errors.push(parse_err(e)),
            },
            "rules" => {
                let parsed = if file.text.trim_start().starts_with('-') {
                    serde_saphyr::from_str::<Vec<Rule>>(&file.text)
                } else {
                    serde_saphyr::from_str::<Rule>(&with_default_id(&file.text, stem))
                        .map(|r| vec![r])
                };
                match parsed {
                    Ok(list) => {
                        for r in list {
                            origins.insert(format!("rule:{}", r.id), path.to_string());
                            rules.push(r);
                        }
                    }
                    Err(e) => errors.push(parse_err(e)),
                }
            }
            _ => {}
        }
    }

    let Some(header) = header else {
        errors.push(Diagnostic {
            severity: Severity::Error,
            file: "story.yaml".into(),
            at: String::new(),
            message: "the project has no story.yaml".into(),
            line: None,
            column: None,
        });
        return Err(errors);
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    let story = Story {
        title: header.title,
        author: header.author,
        start: header.start,
        intro: header.intro,
        clock: header.clock,
        vars: header.vars,
        verbs: header.verbs,
        rooms,
        things,
        rules,
        frames: header.frames,
        seed: header.seed,
    };
    Ok(Project {
        story,
        files,
        origins,
    })
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
    Diagnostic {
        severity: Severity::Error,
        file: path.to_string(),
        at: String::new(),
        message: message.lines().next().unwrap_or(message).to_string(),
        line: number_after("line"),
        column: number_after("column"),
    }
}

/// Read every YAML file under a project folder.
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
            } else if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
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
