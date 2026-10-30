//! An MCP server for one lantern project, over stdio.
//!
//! The server speaks JSON-RPC 2.0, one message per line, as the MCP stdio
//! transport asks. It offers the tools of [`lantern_tools`] and the story
//! writing guide as a resource. Before each tool call it reads the project
//! folder again, so edits made in the editor or by hand show at once.

use lantern_project::read_dir;
use lantern_tools::{Workspace, tools};
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

These tools read, play and verify the project. They do not change files.
- Call `overview` first. Read the guide resource lantern://guide/writing-stories before you write YAML.
- Use `get_entity` and `neighbors` to see a room in context; `room_matrix` shows its text in all states.
- Play with `input`. For your own tests, use a session name other than \"main\" (`session_start`), \
so that you do not move a session someone else uses.
- Check a draft with `try_markup`, `try_condition` and `text_matrix` before you suggest it.
- `check` and `explore` are the ground truth: a suggestion must keep the project free of errors and the ending reachable.";

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
        Ok(Self { root, ws })
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
                    "capabilities": { "tools": {}, "resources": {} },
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
                let (text, is_error) = match self.ws.call(name, args) {
                    Ok(v) => (v.to_string(), false),
                    Err(e) => (e.0, true),
                };
                Ok(json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }))
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

    /// Read the project folder again and reload when a file changed.
    fn refresh(&mut self) {
        // stderr is free for logs: stdout carries the protocol.
        match read_sorted(&self.root) {
            Ok(files) if self.ws.is_stale(&files) => self.ws.reload(files),
            Ok(_) => {}
            Err(e) => eprintln!(
                "lantern mcp: cannot read {}: {e}; the tools use the files read before",
                self.root.display()
            ),
        }
    }
}

/// The project files in a fixed order, so that two reads of an unchanged folder compare equal.
fn read_sorted(root: &Path) -> io::Result<Vec<lantern_project::SourceFile>> {
    let mut files = read_dir(root)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
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
    fn unknown_methods_are_errors() {
        let mut s = server();
        let r = send(
            &mut s,
            json!({ "jsonrpc": "2.0", "id": 5, "method": "prompts/list" }),
        )
        .unwrap();
        assert_eq!(r["error"]["code"], -32601);
    }
}
