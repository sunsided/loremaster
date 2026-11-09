//! Protocol mode (US-65): a host in any language plays a story over JSON-RPC
//! 2.0 on stdio, one message per line, like the MCP stdio transport. The
//! methods are the Session API. One process plays one session.
//!
//! `initialize` gives the title and the interface hash. `session/start`
//! takes `{ seed?, interface? }` and refuses a host built for another
//! interface (IF-5). The others need a started session: `session/submit`
//! `{ input }`, `session/resume` `{ await_id, value }`, `session/event`
//! `{ name, args? }`, `session/snapshot`, `session/restore` `{ snapshot }`,
//! `session/status`, and the queries `session/scene`, `session/fact`
//! `{ name }`, `session/verbs`,
//! `session/journal`, `session/stats`, `session/choices` and `session/combat`.

use crate::{error_reply, read_sorted};
use loremaster_project::check_files;
use loremaster_runtime::{Input, Interface, Session, Snapshot, World};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::Arc;

const INVALID_REQUEST: i64 = -32600;
const INVALID_PARAMS: i64 = -32602;
const METHOD_NOT_FOUND: i64 = -32601;
const NOT_STARTED: i64 = -32002;
const REFUSED: i64 = -32001;
const INTERFACE_MISMATCH: i64 = -32010;

/// An error reply: the code, the message and optional data.
type Failure = (i64, String, Option<Value>);

fn failure(code: i64, message: impl Into<String>) -> Failure {
    (code, message.into(), None)
}

/// The server of one story, with its session once `session/start` ran.
pub struct Protocol {
    world: Arc<World>,
    session: Option<Session>,
}

/// Read `root`, and answer the requests on stdin until it closes.
pub fn serve_stdio(root: &Path) -> io::Result<()> {
    let mut server = Protocol::open(root)?;
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = server.handle_line(&line) {
            writeln!(out, "{reply}")?;
            out.flush()?;
        }
    }
    Ok(())
}

impl Protocol {
    /// Load and check the project. A project with errors does not start.
    pub fn open(root: &Path) -> io::Result<Self> {
        let checked = check_files(read_sorted(root)?);
        let errors: Vec<String> = (checked.diagnostics.iter())
            .filter(|d| d.severity == loremaster_project::Severity::Error)
            .map(ToString::to_string)
            .collect();
        let Some(project) = checked.project.filter(|_| errors.is_empty()) else {
            return Err(io::Error::other(format!(
                "the story has errors:\n{}",
                errors.join("\n")
            )));
        };
        let world = World::compile(project.story).map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Self {
            world: Arc::new(world),
            session: None,
        })
    }

    /// One JSON-RPC message in, its reply out. A notification gets none.
    pub fn handle_line(&mut self, line: &str) -> Option<Value> {
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return Some(error_reply(Value::Null, -32700, &e.to_string())),
        };
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
        let method = match msg.get("method").and_then(Value::as_str) {
            Some(m) if msg["jsonrpc"] == "2.0" && params.is_object() => m,
            _ => {
                let why = "a request is an object with `jsonrpc: \"2.0\"`, a `method` and object `params`";
                return Some(error_reply(id.unwrap_or(Value::Null), INVALID_REQUEST, why));
            }
        };
        let answer = self.call(method, params);
        // A notification runs, but gets no reply.
        let id = id?;
        Some(match answer {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message, data)) => {
                let mut reply = error_reply(id, code, &message);
                if let Some(data) = data {
                    reply["error"]["data"] = data;
                }
                reply
            }
        })
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, Failure> {
        match method {
            "initialize" => Ok(json!({
                "name": "loremaster",
                "title": self.world.story.title,
                "interface_hash": self.interface().hash(),
            })),
            "session/start" => self.start(params),
            "session/submit" => {
                let input: Input = field(&params, "input")?;
                to_result(self.session()?.submit(input))
            }
            "session/resume" => {
                let await_id: String = field(&params, "await_id")?;
                let value = field(&params, "value")?;
                let step = (self.session()?.resume(&await_id, value))
                    .map_err(|why| failure(REFUSED, why))?;
                to_result(step)
            }
            "session/event" => {
                let name: String = field(&params, "name")?;
                let args: BTreeMap<String, loremaster_runtime::Value> = optional(&params, "args")?;
                let step = (self.session()?.host_event(&name, args))
                    .map_err(|why| failure(REFUSED, why))?;
                to_result(step)
            }
            "session/snapshot" => to_result(self.session()?.snapshot()),
            "session/restore" => {
                let snapshot: Snapshot = field(&params, "snapshot")?;
                let step = (self.session()?.restore(&snapshot))
                    .map_err(|e| failure(REFUSED, e.to_string()))?;
                to_result(step)
            }
            "session/status" => to_result(self.session()?.status()),
            "session/scene" => to_result(self.session()?.scene()),
            "session/fact" => {
                let name: String = field(&params, "name")?;
                to_result(self.session()?.fact(&name))
            }
            "session/verbs" => to_result(self.session()?.verbs()),
            "session/journal" => to_result(self.session()?.journal()),
            "session/stats" => to_result(self.session()?.stats()),
            "session/choices" => to_result(self.session()?.choices()),
            "session/combat" => to_result(self.session()?.combat()),
            _ => Err(failure(
                METHOD_NOT_FOUND,
                format!("unknown method `{method}`"),
            )),
        }
    }

    fn interface(&self) -> Interface {
        self.world.story.interface.clone().unwrap_or_default()
    }

    fn session(&mut self) -> Result<&mut Session, Failure> {
        self.session
            .as_mut()
            .ok_or_else(|| failure(NOT_STARTED, "call `session/start` first"))
    }

    /// A new session, also when one is running: the host starts over.
    fn start(&mut self, params: Value) -> Result<Value, Failure> {
        if let Some(built) = params.get("interface") {
            let built: Interface = serde_json::from_value(built.clone())
                .map_err(|e| failure(INVALID_PARAMS, format!("`interface`: {e}")))?;
            if let Err(m) = self.interface().compare(&built) {
                let data = serde_json::to_value(&m).ok();
                return Err((INTERFACE_MISMATCH, m.to_string(), data));
            }
        }
        let mut session = match params.get("seed") {
            None | Some(Value::Null) => Session::new(self.world.clone()),
            Some(seed) => {
                let seed = (seed.as_u64())
                    .ok_or_else(|| failure(INVALID_PARAMS, "`seed` is an unsigned integer"))?;
                Session::with_seed(self.world.clone(), seed)
            }
        };
        let step = session.start();
        self.session = Some(session);
        to_result(step)
    }
}

fn field<T: DeserializeOwned>(params: &Value, name: &str) -> Result<T, Failure> {
    let value = (params.get(name).cloned())
        .ok_or_else(|| failure(INVALID_PARAMS, format!("`{name}` is missing")))?;
    serde_json::from_value(value).map_err(|e| failure(INVALID_PARAMS, format!("`{name}`: {e}")))
}

fn optional<T: DeserializeOwned + Default>(params: &Value, name: &str) -> Result<T, Failure> {
    match params.get(name) {
        Some(_) => field(params, name),
        None => Ok(T::default()),
    }
}

fn to_result<T: Serialize>(value: T) -> Result<Value, Failure> {
    serde_json::to_value(value).map_err(|e| failure(-32603, e.to_string()))
}
