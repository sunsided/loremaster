use crate::{Diagnostic, Severity};
use loremaster_model::{
    ClockConfig, Dialogue, Effect, Fragment, Id, Machine, Note, Room, Rule, Story, Thing, Thread,
    Timer, VarDef, VerbDef, Walkthrough,
};
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
    /// Walkthrough tests from `tests/`, with their file paths.
    pub walkthroughs: Vec<(String, Walkthrough)>,
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
    relations: Vec<Id>,
    #[serde(default)]
    frames: loremaster_model::Frames,
    #[serde(default)]
    seed: u64,
    #[serde(default)]
    notes: Vec<Note>,
    #[serde(default)]
    timers: Vec<Timer>,
    #[serde(default)]
    machines: Vec<Machine>,
    #[serde(default)]
    behaviors: Vec<Id>,
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
    let mut threads = Vec::new();
    let mut dialogues = Vec::new();
    let mut machines = Vec::new();
    let mut walkthroughs = Vec::new();
    let mut behaviors: Vec<Id> = Vec::new();

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
                Ok(mut r) => {
                    origins.insert(format!("room:{}", r.id), path.to_string());
                    for rule in nested_rules(&mut r.rules, &r.id) {
                        origins.insert(format!("rule:{}", rule.id), path.to_string());
                        rules.push(rule);
                    }
                    rooms.push(r);
                }
                Err(e) => errors.push(parse_err(e)),
            },
            "things" => match serde_saphyr::from_str::<Thing>(&with_default_id(&file.text, stem)) {
                Ok(mut t) => {
                    origins.insert(format!("thing:{}", t.id), path.to_string());
                    for rule in nested_rules(&mut t.rules, &t.id) {
                        origins.insert(format!("rule:{}", rule.id), path.to_string());
                        rules.push(rule);
                    }
                    things.push(t);
                }
                Err(e) => errors.push(parse_err(e)),
            },
            "threads" => {
                match serde_saphyr::from_str::<Thread>(&with_default_id(&file.text, stem)) {
                    Ok(t) => {
                        origins.insert(format!("thread:{}", t.id), path.to_string());
                        threads.push(t);
                    }
                    Err(e) => errors.push(parse_err(e)),
                }
            }
            "dialogues" => {
                match serde_saphyr::from_str::<Dialogue>(&with_default_id(&file.text, stem)) {
                    Ok(d) => {
                        origins.insert(format!("dialogue:{}", d.id), path.to_string());
                        dialogues.push(d);
                    }
                    Err(e) => errors.push(parse_err(e)),
                }
            }
            // Commands such as `n` and `y` are words here, not YAML 1.1 booleans.
            "tests" => match serde_saphyr::from_str_with_options::<Walkthrough>(
                &file.text,
                serde_saphyr::options! { strict_booleans: true },
            ) {
                Ok(w) => walkthroughs.push((path.to_string(), w)),
                Err(e) => errors.push(parse_err(e)),
            },
            // A file that names a behavior in Rust; the text holds author notes about it.
            "behaviors" => {
                origins.insert(format!("behavior:{stem}"), path.to_string());
                behaviors.push(Id::new(stem));
            }
            "machines" => {
                match serde_saphyr::from_str::<Machine>(&with_default_id(&file.text, stem)) {
                    Ok(m) => {
                        origins.insert(format!("machine:{}", m.id), path.to_string());
                        machines.push(m);
                    }
                    Err(e) => errors.push(parse_err(e)),
                }
            }
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
    for m in &header.machines {
        origins.insert(format!("machine:{}", m.id), "story.yaml".to_string());
    }
    machines.splice(0..0, header.machines);
    let mut vars = header.vars;
    for m in &machines {
        vars.push(VarDef {
            id: m.id.clone(),
            ty: loremaster_model::VarType::Enum(m.all_states()),
            initial: loremaster_model::Value::Text(m.start.clone()),
        });
        let file = origins
            .get(&format!("machine:{}", m.id))
            .cloned()
            .unwrap_or_default();
        for rule in machine_rules(m) {
            origins.insert(format!("rule:{}", rule.id), file.clone());
            rules.push(rule);
        }
    }
    let story = Story {
        title: header.title,
        author: header.author,
        start: header.start,
        intro: header.intro,
        clock: header.clock,
        vars,
        machines,
        verbs: header.verbs,
        rooms,
        things,
        rules,
        timers: header.timers,
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
        frames: header.frames,
        seed: header.seed,
        notes: header.notes,
        threads,
    };
    Ok(Project {
        story,
        files,
        origins,
        walkthroughs,
    })
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
            let mut when = format!("({})", from.join(" or "));
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
                instead: step.instead,
                within: None,
                priority: 0,
                machine: Some(m.id.clone()),
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
