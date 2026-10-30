//! An MCP server for one lantern project, over stdio or local HTTP.
//!
//! The server speaks JSON-RPC 2.0. Over stdio it reads one message per line,
//! as the MCP stdio transport asks; [`http`] serves the same messages, plus
//! the proposal endpoints of the editor. It offers the tools of
//! [`lantern_tools`] and the story writing guide as a resource. Before each
//! tool call it reads the project folder again, so edits made in the editor
//! or by hand show at once.
//!
//! Proposals are saved in `.lantern/proposals/<id>.json` in the project
//! folder, so that every server on the folder sees the same ones. A new
//! `.lantern/.gitignore` keeps them out of git; delete that line to track them.

pub mod http;
pub mod jobs;

use lantern_project::read_dir;
use lantern_tools::{Proposal, Workspace, prompts_json, render_prompt, tools};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// The protocol versions this server knows, newest first.
const PROTOCOL_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const GUIDE_URI: &str = "lantern://guide/writing-stories";
const GUIDE: &str = include_str!("../../../docs/writing-stories.md");

const INSTRUCTIONS: &str = "\
lantern is a world editor for text adventures. Every text in the game comes from the world state: \
rooms, things and rules are YAML files with text fragments, conditions and inline markup.

These tools read, play and verify the project. They do not change files: to suggest a change, \
call `propose` with whole files. A person sees the proposal in the editor and accepts it or not.
- Call `overview` first. Read the guide resource lantern://guide/writing-stories before you write YAML.
- Use `get_entity` and `neighbors` to see a room in context; `room_matrix` shows its text in all states.
- Play with `input`. For your own tests, use a session name other than \"main\" (`session_start`), \
so that you do not move a session someone else uses.
- Check a draft with `try_markup`, `try_condition` and `text_matrix` before you suggest it.
- `check` and `explore` are the ground truth: a suggestion must keep the project free of errors and the ending reachable.
- `propose` checks your change. Fix the problems it reports with a new proposal and withdraw the old one. \
`session_start` with `proposal` plays your draft.";

/// Serve the project in `root` on stdin and stdout until stdin closes.
pub fn serve_stdio(root: &Path) -> io::Result<()> {
    let mut server = Server::new(root.to_path_buf())?;
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = server.handle_line(&line) {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

/// The server state: the project folder and its workspace.
pub struct Server {
    root: PathBuf,
    ws: Workspace,
}

impl Server {
    pub fn new(root: PathBuf) -> io::Result<Self> {
        let ws = Workspace::new(read_sorted(&root)?);
        let mut server = Self { root, ws };
        server.refresh();
        Ok(server)
    }

    pub fn workspace(&self) -> &Workspace {
        &self.ws
    }

    /// Run one tool as an MCP call would: refresh first, save the proposals after.
    pub fn call_tool(&mut self, name: &str, args: Value) -> Result<Value, String> {
        self.refresh();
        let result = self.ws.call(name, args);
        self.save_proposals();
        result.map_err(|e| e.0)
    }

    /// End a play session, for example the session of a job that did not start.
    pub fn end_session(&mut self, name: &str) -> bool {
        self.ws.end_session(name)
    }

    /// The summaries and notes of rejected proposals, newest last, at most `limit`.
    pub fn rejected(&self, limit: usize) -> Vec<String> {
        let log = self.root.join(".lantern").join("rejected.jsonl");
        let text = std::fs::read_to_string(log).unwrap_or_default();
        let lines: Vec<String> = text
            .lines()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .map(|v| {
                // One line for each proposal, also when the note has line breaks.
                let one_line = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
                let summary =
                    one_line(v.get("summary").and_then(Value::as_str).unwrap_or_default());
                match v.get("note").and_then(Value::as_str).map(one_line) {
                    Some(note) => format!("- {summary} (rejected: {note})"),
                    None => format!("- {summary}"),
                }
            })
            .collect();
        lines[lines.len().saturating_sub(limit)..].to_vec()
    }

    /// Handle one JSON-RPC message. Returns the reply, or nothing for a notification.
    pub fn handle_line(&mut self, line: &str) -> Option<Value> {
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                return Some(error_reply(
                    Value::Null,
                    -32700,
                    &format!("Parse error: {e}"),
                ));
            }
        };
        if msg.is_array() {
            // MCP 2025-06-18 and later have no batches.
            return Some(error_reply(
                Value::Null,
                -32600,
                "Batch requests are not supported",
            ));
        }
        let id = msg.get("id").cloned()?; // No id: a notification. Nothing to answer.
        let method = msg
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        Some(match self.handle(method, params) {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => error_reply(id, code, &message),
        })
    }

    fn handle(&mut self, method: &str, params: Value) -> Result<Value, (i64, String)> {
        match method {
            "initialize" => {
                let asked = params.get("protocolVersion").and_then(Value::as_str);
                let version = asked
                    .filter(|v| PROTOCOL_VERSIONS.contains(v))
                    .unwrap_or(PROTOCOL_VERSIONS[0]);
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
                    "serverInfo": { "name": "lantern", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": INSTRUCTIONS,
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({
                "tools": tools().iter().map(|t| json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                    "annotations": {
                        "readOnlyHint": t.read_only,
                        "destructiveHint": false,
                        "openWorldHint": false,
                    },
                })).collect::<Vec<_>>(),
            })),
            "tools/call" => {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);
                self.refresh();
                let result = self.ws.call(name, args);
                self.save_proposals();
                let (text, is_error) = match result {
                    Ok(v) => (v.to_string(), false),
                    Err(e) => (e.0, true),
                };
                Ok(json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }))
            }
            "prompts/list" => Ok(json!({ "prompts": prompts_json() })),
            "prompts/get" => {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);
                let text = render_prompt(name, &args).map_err(|e| (-32602, e.0))?;
                Ok(json!({
                    "messages": [{ "role": "user", "content": { "type": "text", "text": text } }],
                }))
            }
            "resources/list" => Ok(json!({
                "resources": [{
                    "uri": GUIDE_URI,
                    "name": "writing-stories",
                    "title": "How to write a lantern story",
                    "description": "The project format: files, fragments, inline markup, conditions, action frames and name stages.",
                    "mimeType": "text/markdown",
                }],
            })),
            "resources/read" => match params.get("uri").and_then(Value::as_str) {
                Some(GUIDE_URI) => Ok(json!({
                    "contents": [{ "uri": GUIDE_URI, "mimeType": "text/markdown", "text": GUIDE }],
                })),
                _ => Err((-32002, "Resource not found".into())),
            },
            _ => Err((-32601, format!("Method not found: {method}"))),
        }
    }

    /// Read the project folder and the saved proposals again, and reload when a file changed.
    pub fn refresh(&mut self) {
        // stderr is free for logs: stdout carries the protocol.
        match read_sorted(&self.root) {
            Ok(files) if self.ws.is_stale(&files) => self.ws.reload(files),
            Ok(_) => {}
            Err(e) => eprintln!(
                "lantern: cannot read {}: {e}; the tools use the files read before",
                self.root.display()
            ),
        }
        let saved = self.load_proposals();
        if saved.iter().ne(self.ws.proposals()) {
            self.ws.set_proposals(saved);
        }
    }

    fn proposals_dir(&self) -> PathBuf {
        self.root.join(".lantern").join("proposals")
    }

    /// The proposals in `.lantern/proposals/`, in id order. A file that does not parse is left out.
    fn load_proposals(&self) -> Vec<Proposal> {
        let Ok(entries) = std::fs::read_dir(self.proposals_dir()) else {
            return Vec::new();
        };
        let mut out: Vec<Proposal> = entries
            .filter_map(|e| {
                let path = e.ok()?.path();
                if path.extension()? != "json" {
                    return None;
                }
                let text = std::fs::read_to_string(&path).ok()?;
                match serde_json::from_str(&text) {
                    Ok(p) => Some(p),
                    Err(err) => {
                        eprintln!("lantern: skip {}: {err}", path.display());
                        None
                    }
                }
            })
            .collect();
        out.sort_by(|a: &Proposal, b| a.id.cmp(&b.id));
        out
    }

    /// Make `.lantern/proposals/` hold the proposals of the workspace.
    fn save_proposals(&self) {
        let saved = self.load_proposals();
        if saved.iter().eq(self.ws.proposals()) {
            return;
        }
        if let Err(e) = self.write_proposals(&saved) {
            eprintln!("lantern: cannot save proposals: {e}");
        }
    }

    fn write_proposals(&self, saved: &[Proposal]) -> io::Result<()> {
        let dir = self.proposals_dir();
        std::fs::create_dir_all(&dir)?;
        let ignore = self.root.join(".lantern").join(".gitignore");
        if !ignore.exists() {
            std::fs::write(
                &ignore,
                "# Proposals are suggestions. Delete this line to track them in git.\nproposals/\n",
            )?;
        }
        for p in self.ws.proposals() {
            if !saved.contains(p) {
                let text = serde_json::to_string_pretty(p).map_err(io::Error::other)?;
                std::fs::write(dir.join(format!("{}.json", p.id)), text)?;
            }
        }
        for p in saved {
            if self.ws.proposal(&p.id).is_none() {
                std::fs::remove_file(dir.join(format!("{}.json", p.id)))?;
            }
        }
        Ok(())
    }

    /// Accept a proposal: write its files into the project and remove it.
    /// With `applied`, the caller (the editor) already wrote the change itself,
    /// so only the proposal goes. A stale proposal, whose files changed since it
    /// was made, is refused unless `force`. Returns the paths written and deleted.
    pub fn accept(&mut self, id: &str, applied: bool, force: bool) -> Result<Vec<String>, Refusal> {
        self.refresh();
        let p = self
            .ws
            .proposal(id)
            .cloned()
            .ok_or_else(|| Refusal::NotFound(format!("There is no proposal `{id}`.")))?;
        if applied {
            self.ws.remove_proposal(id);
            self.save_proposals();
            return Ok(Vec::new());
        }
        let view = self
            .ws
            .view_proposal(id)
            .map_err(|e| Refusal::NotFound(e.0))?;
        if !view.ok {
            return Err(Refusal::Conflict(format!(
                "Proposal `{id}` has errors. Fix them before you accept it."
            )));
        }
        if view.stale && !force {
            return Err(Refusal::Conflict(format!(
                "Proposal `{id}` is stale: a file it changes was changed after it was made. Accept with force to write it anyway."
            )));
        }
        let mut touched = Vec::new();
        for f in &p.files {
            let path = self.root.join(&f.path);
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| Refusal::Io(e.to_string()))?;
            }
            std::fs::write(&path, &f.text).map_err(|e| Refusal::Io(format!("{}: {e}", f.path)))?;
            touched.push(f.path.clone());
        }
        for d in &p.delete {
            match std::fs::remove_file(self.root.join(d)) {
                Ok(()) => touched.push(d.clone()),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(Refusal::Io(format!("{d}: {e}"))),
            }
        }
        self.ws.remove_proposal(id);
        self.save_proposals();
        self.refresh();
        Ok(touched)
    }

    /// Remove a proposal without a change to the project. A note says why, for the next request.
    pub fn reject(&mut self, id: &str, note: Option<&str>) -> Result<(), Refusal> {
        self.refresh();
        let p = self
            .ws
            .remove_proposal(id)
            .ok_or_else(|| Refusal::NotFound(format!("There is no proposal `{id}`.")))?;
        self.save_proposals();
        let line = json!({ "id": p.id, "summary": p.summary, "note": note }).to_string();
        let log = self.root.join(".lantern").join("rejected.jsonl");
        let written = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .and_then(|mut f| writeln!(f, "{line}"));
        written.map_err(|e| Refusal::Io(format!("cannot write {}: {e}", log.display())))
    }
}

/// The project files in a fixed order, so that two reads of an unchanged folder compare equal.
fn read_sorted(root: &Path) -> io::Result<Vec<lantern_project::SourceFile>> {
    let mut files = read_dir(root)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Why `accept` or `reject` did not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// There is no such proposal.
    NotFound(String),
    /// The proposal has errors or is stale.
    Conflict(String),
    /// A file could not be written.
    Io(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(m) | Self::Conflict(m) | Self::Io(m) => f.write_str(m),
        }
    }
}

fn error_reply(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> Server {
        Server::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse"))
            .unwrap()
    }

    fn send(s: &mut Server, msg: Value) -> Option<Value> {
        s.handle_line(&msg.to_string())
    }

    #[test]
    fn a_rejected_note_stays_on_one_line() {
        let dir = std::env::temp_dir().join(format!("lantern-rejected-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".lantern")).unwrap();
        let entry = json!({ "summary": "A pier", "note": "too\nmodern" });
        std::fs::write(dir.join(".lantern/rejected.jsonl"), format!("{entry}\n")).unwrap();
        let s = Server::new(dir.clone()).unwrap();
        assert_eq!(s.rejected(20), ["- A pier (rejected: too modern)"]);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn initialize_answers_with_a_known_version() {
        let mut s = server();
        let r = send(&mut s, json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18" } })).unwrap();
        assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
        let r = send(&mut s, json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize", "params": { "protocolVersion": "1999-01-01" } })).unwrap();
        assert_eq!(r["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
    }

    #[test]
    fn notifications_get_no_reply() {
        let mut s = server();
        assert!(
            send(
                &mut s,
                json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
            )
            .is_none()
        );
    }

    #[test]
    fn a_tool_call_returns_text_content() {
        let mut s = server();
        let r = send(&mut s, json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "input", "arguments": { "text": "n" } } })).unwrap();
        assert_eq!(r["result"]["isError"], false);
        let text = r["result"]["content"][0]["text"].as_str().unwrap();
        let turn: Value = serde_json::from_str(text).unwrap();
        assert_eq!(turn["scene"]["room"], "cottage");
    }

    #[test]
    fn a_failed_tool_call_is_a_tool_error_not_a_protocol_error() {
        let mut s = server();
        let r = send(&mut s, json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "nope" } })).unwrap();
        assert_eq!(r["result"]["isError"], true);
    }

    #[test]
    fn a_batch_gets_an_error() {
        let mut s = server();
        let r = s.handle_line("[]").unwrap();
        assert_eq!(r["error"]["code"], -32600);
    }

    #[test]
    fn accepting_a_missing_proposal_is_not_found() {
        let mut s = server();
        assert!(matches!(
            s.accept("p999", false, false),
            Err(Refusal::NotFound(_))
        ));
        assert!(matches!(s.reject("p999", None), Err(Refusal::NotFound(_))));
    }

    #[test]
    fn unknown_methods_are_errors() {
        let mut s = server();
        let r = send(
            &mut s,
            json!({ "jsonrpc": "2.0", "id": 5, "method": "sampling/nope" }),
        )
        .unwrap();
        assert_eq!(r["error"]["code"], -32601);
    }
}
