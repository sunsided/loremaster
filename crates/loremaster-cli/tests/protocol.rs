//! Drives protocol mode over a real pipe: the binary runs as a child
//! process, and the test talks JSON-RPC to it on stdin and stdout.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const STORY: &str = r#"
title: Pipe
start: hall
interface:
  actions:
    - { id: lockpick, args: { difficulty: int }, results: [success, broken_pick] }
  events:
    - { id: ping, args: { n: int } }
vars:
  - { id: lock, type: { enum: [closed, success, broken_pick] }, initial: closed }
"#;

const RULES: &str = r#"
- id: pick
  on: { verb: [pull, lever] }
  instead: true
  then:
    - await: { action: lockpick, args: { difficulty: 2 }, into: lock }
    - say: "{lock == success: It opens.}{lock == broken_pick: Snap.}"
- { id: hear, on: { event: ping }, then: [{ say: "Ping {ping.n}." }] }
"#;

fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lm-protocol-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["rooms", "things", "verbs", "rules"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    std::fs::write(dir.join("story.yaml"), STORY).unwrap();
    std::fs::write(
        dir.join("rooms/hall.yaml"),
        "name: Hall\ndescription: [\"A hall.\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("things/lever.yaml"),
        "name: lever\nlocation: { room: hall }\n",
    )
    .unwrap();
    std::fs::write(dir.join("verbs/pull.yaml"), "label: Pull\nwords: [pull]\n").unwrap();
    std::fs::write(dir.join("rules/r.yaml"), RULES).unwrap();
    dir
}

struct Host {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: i64,
}

impl Host {
    fn start(dir: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_loremaster"))
            .arg("protocol")
            .arg(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
            next: 0,
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let msg = json!({ "jsonrpc": "2.0", "id": self.next, "method": method, "params": params });
        writeln!(self.stdin, "{msg}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        let reply: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(reply["id"], json!(self.next), "{reply}");
        reply
    }

    /// Send one raw line; read a reply only when `reply` is set.
    fn raw(&mut self, line: &str, reply: bool) -> Option<Value> {
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
        reply.then(|| {
            let mut line = String::new();
            self.stdout.read_line(&mut line).unwrap();
            serde_json::from_str(&line).unwrap()
        })
    }

    fn result(&mut self, method: &str, params: Value) -> Value {
        let reply = self.call(method, params);
        assert!(reply.get("error").is_none(), "{reply}");
        reply["result"].clone()
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn a_host_plays_the_story_over_the_pipe() {
    let dir = project("play");
    let mut host = Host::start(&dir);
    let info = host.result("initialize", json!({}));
    assert_eq!(info["title"], "Pipe");
    assert_eq!(info["interface_hash"].as_str().unwrap().len(), 64);

    let refused = host.call("session/scene", json!({}));
    assert_eq!(refused["error"]["code"], -32002, "{refused}");

    let start = host.result("session/start", json!({}));
    assert_eq!(start["status"]["type"], "ready");

    let scene = host.result("session/scene", json!({}));
    assert_eq!(scene["room"], "hall", "{scene}");

    let snapshot = host.result("session/snapshot", json!({}));
    let waiting = host.result(
        "session/submit",
        json!({ "input": { "type": "text", "value": "pull lever" } }),
    );
    assert_eq!(waiting["status"]["type"], "await", "{waiting}");
    let awaiting = &waiting["status"]["awaiting"];
    assert_eq!(awaiting["request"]["name"], "lockpick");
    let id = awaiting["id"].as_str().unwrap().to_string();

    let bad = host.call("session/resume", json!({ "await_id": id, "value": "nope" }));
    assert!(bad.get("error").is_some(), "{bad}");

    let done = host.result(
        "session/resume",
        json!({ "await_id": id, "value": "success" }),
    );
    assert_eq!(done["status"]["type"], "ready", "{done}");
    assert!(done.to_string().contains("It opens"), "{done}");

    let event = host.result(
        "session/event",
        json!({ "name": "ping", "args": { "n": 7 } }),
    );
    assert!(event.to_string().contains("Ping 7"), "{event}");

    let restored = host.result("session/restore", json!({ "snapshot": snapshot }));
    assert_eq!(restored["status"]["type"], "ready", "{restored}");

    let status = host.result("session/status", json!({}));
    assert_eq!(status["type"], "ready");

    let unknown = host.call("session/fly", json!({}));
    assert_eq!(unknown["error"]["code"], -32601, "{unknown}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_host_built_for_another_interface_is_refused() {
    let dir = project("refuse");
    let mut host = Host::start(&dir);
    let reply = host.call(
        "session/start",
        json!({ "interface": { "actions": [{ "id": "dance" }] } }),
    );
    let message = reply["error"]["message"].as_str().unwrap();
    assert!(message.contains("added: action lockpick"), "{message}");
    assert!(message.contains("removed: action dance"), "{message}");
    assert_eq!(reply["error"]["data"]["removed"], json!(["action dance"]));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn notifications_run_and_bad_requests_get_an_error() {
    let dir = project("shape");
    let mut host = Host::start(&dir);
    // A notification gets no reply, but it runs: the session is started.
    host.raw(r#"{ "jsonrpc": "2.0", "method": "session/start" }"#, false);
    host.result("session/status", json!({}));
    for bad in [
        "42",
        r#"{ "jsonrpc": "2.0", "id": 9 }"#,
        r#"{ "jsonrpc": "2.0", "id": 9, "method": "session/status", "params": 1 }"#,
    ] {
        let reply = host.raw(bad, true).unwrap();
        assert_eq!(reply["error"]["code"], -32600, "{reply}");
    }
    let seed = host.call("session/start", json!({ "seed": "7" }));
    assert_eq!(seed["error"]["code"], -32602, "{seed}");
    std::fs::remove_dir_all(&dir).unwrap();
}
