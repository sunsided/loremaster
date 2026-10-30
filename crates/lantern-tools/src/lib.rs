//! The operations an AI assistant may do on a lantern project.
//!
//! A [`Workspace`] holds the project files, the compiled world and any number
//! of named play sessions. [`tools`] lists the operations with a JSON Schema
//! for their arguments; [`Workspace::call`] runs one. Arguments and results
//! are JSON values, so the same layer serves an MCP server, an in-process tool
//! loop for a chat API, and (later) the browser.
//!
//! This crate has no IO. The caller reads the files and gives them to
//! [`Workspace::new`] or [`Workspace::reload`].
//!
//! The tools read, play and verify. They do not change project files.

mod tools;

pub use tools::{ToolDef, tools};

use lantern_project::{Diagnostic, Project, Severity, SourceFile, check, load};
use lantern_runtime::{Game, World};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// The session that tools use when the call names none.
pub const DEFAULT_SESSION: &str = "main";

/// A tool call that could not run. The message is for the model: it says what
/// was wrong and, where possible, what to do instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError(pub String);

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ToolError {}

pub type ToolResult = Result<Value, ToolError>;

fn err(message: impl Into<String>) -> ToolError {
    ToolError(message.into())
}

/// A project as the tools see it: files, diagnostics, the compiled world and
/// the play sessions.
pub struct Workspace {
    files: Vec<SourceFile>,
    project: Option<Project>,
    diagnostics: Vec<Diagnostic>,
    world: Option<Arc<World>>,
    sessions: BTreeMap<String, Game>,
}

impl Workspace {
    /// Load, check and compile a project. A project with errors still gives a
    /// workspace: the read and check tools work, the play tools say why not.
    pub fn new(files: Vec<SourceFile>) -> Self {
        let mut ws = Self {
            files: Vec::new(),
            project: None,
            diagnostics: Vec::new(),
            world: None,
            sessions: BTreeMap::new(),
        };
        ws.reload(files);
        ws
    }

    /// Replace the project files. Sessions keep their state where the new
    /// world still has the rooms, things and variables they refer to.
    pub fn reload(&mut self, files: Vec<SourceFile>) {
        self.files = files;
        self.project = None;
        self.world = None;
        self.diagnostics = match load(self.files.clone()) {
            Ok(project) => {
                let diags = check(&project);
                if !diags.iter().any(|d| d.severity == Severity::Error) {
                    match World::compile(project.story.clone()) {
                        Ok(world) => self.world = Some(Arc::new(world)),
                        Err(e) => {
                            return self.compile_failed(project, e.at.clone(), e.error.message);
                        }
                    }
                }
                self.project = Some(project);
                diags
            }
            Err(diags) => diags,
        };
        self.carry_sessions();
    }

    fn compile_failed(&mut self, project: Project, at: String, message: String) {
        self.diagnostics = vec![Diagnostic {
            severity: Severity::Error,
            file: String::new(),
            at,
            message,
            line: None,
            column: None,
        }];
        self.project = Some(project);
        self.sessions.clear();
    }

    fn carry_sessions(&mut self) {
        match &self.world {
            Some(world) => {
                for game in self.sessions.values_mut() {
                    *game = game.carry_over(world.clone());
                }
            }
            None => self.sessions.clear(),
        }
    }

    /// The files as last given.
    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// True when the files differ from the ones the workspace holds.
    pub fn is_stale(&self, files: &[SourceFile]) -> bool {
        self.files != files
    }

    /// Run one tool. `args` is a JSON object as described by the tool's schema.
    pub fn call(&mut self, name: &str, args: Value) -> ToolResult {
        let args = Args(match args {
            Value::Null => serde_json::Map::new(),
            Value::Object(map) => map,
            _ => return Err(err("The arguments must be a JSON object.")),
        });
        tools::dispatch(self, name, &args)
    }

    fn project(&self) -> Result<&Project, ToolError> {
        self.project
            .as_ref()
            .ok_or_else(|| err("The project does not load. Call `check` to see the problems."))
    }

    fn world(&self) -> Result<&Arc<World>, ToolError> {
        self.world.as_ref().ok_or_else(|| {
            err("The project has errors, so it cannot be played. Call `check` to see them.")
        })
    }

    /// The named session. The default session starts on first use.
    fn session(&mut self, name: &str) -> Result<&mut Game, ToolError> {
        let world = self.world()?.clone();
        if !self.sessions.contains_key(name) {
            if name != DEFAULT_SESSION {
                return Err(err(format!(
                    "There is no session `{name}`. Call `session_start` to make one."
                )));
            }
            let mut game = Game::new(world);
            game.start();
            self.sessions.insert(name.to_string(), game);
        }
        Ok(self.sessions.get_mut(name).expect("inserted above"))
    }
}

/// The arguments of one call.
struct Args(serde_json::Map<String, Value>);

impl Args {
    fn str(&self, key: &str) -> Result<&str, ToolError> {
        self.opt_str(key)
            .ok_or_else(|| err(format!("The argument `{key}` (text) is missing.")))
    }

    fn opt_str(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str)
    }

    fn usize_or(&self, key: &str, default: usize) -> usize {
        self.0
            .get(key)
            .and_then(Value::as_u64)
            .map_or(default, |n| n as usize)
    }

    fn session(&self) -> &str {
        self.opt_str("session").unwrap_or(DEFAULT_SESSION)
    }

    fn value<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<T, ToolError> {
        let v = self
            .0
            .get(key)
            .ok_or_else(|| err(format!("The argument `{key}` is missing.")))?;
        serde_json::from_value(v.clone()).map_err(|e| err(format!("`{key}`: {e}")))
    }

    fn opt_value<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>, ToolError> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(_) => self.value(key).map(Some),
        }
    }
}

fn json<T: Serialize>(value: &T) -> ToolResult {
    serde_json::to_value(value).map_err(|e| err(e.to_string()))
}
