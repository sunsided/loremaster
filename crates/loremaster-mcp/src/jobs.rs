//! Jobs: an assistant runs one task prompt against the server's own MCP tools.
//!
//! The first agent is Claude Code, started as `claude -p` with the user's own
//! login (for example a Max plan). It sees only the loremaster tools, so it can
//! read, play and propose, but not edit files or run commands. A job runs in
//! its own thread; the server stays free to answer the agent's MCP calls.
//!
//! Set `LOREMASTER_CLAUDE` to the `claude` program to use (default: `claude` on
//! the `PATH`) and `LOREMASTER_CLAUDE_MODEL` to pick a model (default: `sonnet`).

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

/// How far a job is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Running,
    Done,
    Failed,
    Cancelled,
}

/// One step of a job, for a progress line in the editor.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobEvent {
    /// `tool` (the agent called a loremaster tool), `text` (the agent wrote text) or `error`.
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Job {
    pub id: String,
    pub task: String,
    pub args: Value,
    pub status: JobStatus,
    pub events: Vec<JobEvent>,
    /// The agent's final reply.
    pub result: Option<String>,
    /// The proposals the agent made and did not withdraw.
    pub proposals: Vec<String>,
    /// The agent's session, to continue it later.
    pub agent_session: Option<String>,
    /// The model of the job: the `model` of the request, or else `LOREMASTER_CLAUDE_MODEL`
    /// (default `sonnet`). `None` when `LOREMASTER_CLAUDE_MODEL` is empty, so that Claude Code
    /// uses its own default model.
    pub model: Option<String>,
    pub cost_usd: Option<f64>,
}

/// The built-in Claude Code tools that a job can use. All others are off.
const BUILT_IN_TOOLS: &str = "ToolSearch,ListMcpResourcesTool,ReadMcpResourceTool";

/// The program that runs a job.
#[derive(Debug, Clone)]
pub struct Agent {
    pub program: String,
    pub model: Option<String>,
    /// Claude Code's `--effort`. Low effort thinks less, and a job writes its
    /// proposals sooner.
    pub effort: Option<String>,
}

impl Agent {
    /// Claude Code, from `LOREMASTER_CLAUDE`, `LOREMASTER_CLAUDE_MODEL` (default `sonnet`)
    /// and `LOREMASTER_CLAUDE_EFFORT` (default `low`). An empty model or effort gives
    /// Claude Code's own default.
    ///
    /// Sonnet is the default because a job is a short, tool-heavy task: it is about
    /// as fast as larger models and uses less than half of the plan's quota.
    pub fn from_env() -> Self {
        Self {
            program: std::env::var("LOREMASTER_CLAUDE").unwrap_or_else(|_| "claude".into()),
            model: setting(std::env::var("LOREMASTER_CLAUDE_MODEL").ok(), DEFAULT_MODEL),
            effort: setting(std::env::var("LOREMASTER_CLAUDE_EFFORT").ok(), "low"),
        }
    }

    /// The command line: headless, stream of JSON events, loremaster tools only.
    ///
    /// `--tools` turns off the built-in tools (Read, Grep, Bash and the others)
    /// except the three that the loremaster server needs: the MCP resources (the
    /// writing guide) and the search for deferred tools.
    fn command(&self, prompt: &str, mcp_config: &str, model: Option<&str>) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(["-p", prompt])
            .args(["--output-format", "stream-json", "--verbose"])
            .args(["--mcp-config", mcp_config, "--strict-mcp-config"])
            .args(["--tools", BUILT_IN_TOOLS])
            .args([
                "--allowedTools",
                &format!("mcp__loremaster__*,{BUILT_IN_TOOLS}"),
            ]);
        if let Some(model) = pick_model(model, self.model.as_deref()) {
            cmd.args(["--model", model]);
        }
        if let Some(effort) = &self.effort {
            cmd.args(["--effort", effort]);
        }
        // Load the loremaster tools at the start, without a ToolSearch round trip.
        cmd.env("ENABLE_TOOL_SEARCH", "false");
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    }
}

/// The model of a job when neither the request nor `LOREMASTER_CLAUDE_MODEL` names one.
pub const DEFAULT_MODEL: &str = "sonnet";

/// A setting from the environment: the default when it is not set, none when it is empty.
fn setting(value: Option<String>, default: &str) -> Option<String> {
    match value {
        Some(v) => Some(v).filter(|v| !v.trim().is_empty()),
        None => Some(default.into()),
    }
}

/// The model of a job: the request's, else the agent's. An empty name counts as none,
/// so that no `--model ""` reaches Claude Code.
fn pick_model<'a>(job: Option<&'a str>, agent: Option<&'a str>) -> Option<&'a str> {
    let named = |m: Option<&'a str>| m.map(str::trim).filter(|m| !m.is_empty());
    named(job).or(named(agent))
}

/// All jobs of one server.
pub struct Jobs {
    agent: Agent,
    /// The MCP configuration that points the agent at this server.
    mcp_config: String,
    next: u64,
    jobs: Arc<Mutex<BTreeMap<String, Job>>>,
    children: Arc<Mutex<BTreeMap<String, Child>>>,
}

impl Jobs {
    pub fn new(agent: Agent, mcp_url: &str) -> Self {
        let mcp_config = serde_json::json!({
            "mcpServers": { "loremaster": { "type": "http", "url": mcp_url } }
        })
        .to_string();
        Self {
            agent,
            mcp_config,
            next: 1,
            jobs: Arc::default(),
            children: Arc::default(),
        }
    }

    /// Jobs whose agent starts its own `loremaster mcp <folder>` on stdio, for a
    /// command line run without `loremaster serve`.
    pub fn stdio(agent: Agent, program: &str, folder: &str) -> Self {
        let mut jobs = Self::new(agent, "");
        jobs.mcp_config = serde_json::json!({
            "mcpServers": { "loremaster": { "type": "stdio", "command": program, "args": ["mcp", folder] } }
        })
        .to_string();
        jobs
    }

    /// The id the next job gets, so the caller can prepare its play session first.
    pub fn next_id(&self) -> String {
        format!("j{}", self.next)
    }

    /// Start a job with a rendered prompt. Returns its id.
    /// `model` overrides the agent's model for this job.
    pub fn start(
        &mut self,
        task: &str,
        args: Value,
        prompt: &str,
        model: Option<&str>,
    ) -> Result<String, String> {
        let id = self.next_id();
        self.next += 1;
        let mut child = self
            .agent
            .command(prompt, &self.mcp_config, model)
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", self.agent.program))?;
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        self.jobs.lock().unwrap().insert(
            id.clone(),
            Job {
                id: id.clone(),
                task: task.into(),
                args,
                status: JobStatus::Running,
                events: Vec::new(),
                result: None,
                proposals: Vec::new(),
                agent_session: None,
                model: pick_model(model, self.agent.model.as_deref()).map(str::to_string),
                cost_usd: None,
            },
        );
        self.children.lock().unwrap().insert(id.clone(), child);
        let (jobs, children, job) = (self.jobs.clone(), self.children.clone(), id.clone());
        // Read stderr on its own thread, so that a full pipe cannot block the agent.
        let err_text = std::thread::spawn(move || {
            let mut err = String::new();
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                err.push_str(&line);
                err.push('\n');
            }
            err
        });
        std::thread::spawn(move || {
            let mut reader = StreamReader::default();
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(j) = jobs.lock().unwrap().get_mut(&job) {
                    reader.line(&line, j);
                }
            }
            let err = err_text.join().unwrap_or_default();
            // Hold the children lock until the status is final, so that a
            // cancel() in between does not see a job that runs but has no child.
            let mut children = children.lock().unwrap();
            let success = children
                .remove(&job)
                .and_then(|mut c| c.wait().ok())
                .is_some_and(|s| s.success());
            if let Some(j) = jobs.lock().unwrap().get_mut(&job)
                && j.status == JobStatus::Running
            {
                j.status = if success && j.result.is_some() {
                    JobStatus::Done
                } else {
                    JobStatus::Failed
                };
                if !success && !err.trim().is_empty() {
                    j.events.push(JobEvent {
                        kind: "error".into(),
                        text: err.trim().to_string(),
                    });
                }
            }
        });
        Ok(id)
    }

    pub fn get(&self, id: &str) -> Option<Job> {
        self.jobs.lock().unwrap().get(id).cloned()
    }

    /// All jobs, oldest first.
    pub fn list(&self) -> Vec<Job> {
        let mut list: Vec<Job> = self.jobs.lock().unwrap().values().cloned().collect();
        list.sort_by_key(|j| j.id[1..].parse::<u64>().unwrap_or(u64::MAX));
        list
    }

    /// Stop a running job. Proposals it made stay.
    pub fn cancel(&self, id: &str) -> bool {
        let Some(mut child) = self.children.lock().unwrap().remove(id) else {
            return false;
        };
        let _ = child.kill();
        let _ = child.wait();
        if let Some(j) = self.jobs.lock().unwrap().get_mut(id) {
            j.status = JobStatus::Cancelled;
        }
        true
    }
}

/// Reads Claude Code's `stream-json` lines into a job.
#[derive(Default)]
struct StreamReader {
    /// Tool call id to tool name, to read the results of `propose` and `withdraw_proposal`.
    calls: BTreeMap<String, String>,
}

impl StreamReader {
    fn line(&mut self, line: &str, job: &mut Job) {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            return;
        };
        let content = v.pointer("/message/content").and_then(Value::as_array);
        match v.get("type").and_then(Value::as_str) {
            Some("system") => {
                if let Some(s) = v.get("session_id").and_then(Value::as_str) {
                    job.agent_session = Some(s.to_string());
                }
            }
            Some("assistant") => {
                for c in content.into_iter().flatten() {
                    match c.get("type").and_then(Value::as_str) {
                        Some("text") => {
                            let text = c.get("text").and_then(Value::as_str).unwrap_or_default();
                            if !text.trim().is_empty() {
                                job.events.push(JobEvent {
                                    kind: "text".into(),
                                    text: text.trim().to_string(),
                                });
                            }
                        }
                        Some("tool_use") => {
                            let name = c.get("name").and_then(Value::as_str).unwrap_or_default();
                            let name = name.strip_prefix("mcp__loremaster__").unwrap_or(name);
                            if let Some(id) = c.get("id").and_then(Value::as_str) {
                                self.calls.insert(id.to_string(), name.to_string());
                            }
                            if name == "withdraw_proposal"
                                && let Some(p) = c.pointer("/input/id").and_then(Value::as_str)
                            {
                                job.proposals.retain(|x| x != p);
                            }
                            job.events.push(JobEvent {
                                kind: "tool".into(),
                                text: name.to_string(),
                            });
                        }
                        _ => {}
                    }
                }
            }
            Some("user") => {
                for c in content.into_iter().flatten() {
                    let call = c
                        .get("tool_use_id")
                        .and_then(Value::as_str)
                        .and_then(|id| self.calls.remove(id));
                    if call.as_deref() != Some("propose")
                        || c.get("is_error").and_then(Value::as_bool) == Some(true)
                    {
                        continue;
                    }
                    if let Some(id) = proposal_id(c.get("content"))
                        && !job.proposals.contains(&id)
                    {
                        job.proposals.push(id);
                    }
                }
            }
            Some("result") => {
                job.result = v.get("result").and_then(Value::as_str).map(str::to_string);
                job.cost_usd = v.get("total_cost_usd").and_then(Value::as_f64);
                if let Some(s) = v.get("session_id").and_then(Value::as_str) {
                    job.agent_session = Some(s.to_string());
                }
                if v.get("is_error").and_then(Value::as_bool) == Some(true) {
                    job.status = JobStatus::Failed;
                }
            }
            _ => {}
        }
    }
}

/// The proposal id in the result of a `propose` call: a text block with the view as JSON.
fn proposal_id(content: Option<&Value>) -> Option<String> {
    let text = match content? {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect(),
        _ => return None,
    };
    let view: Value = serde_json::from_str(&text).ok()?;
    view.get("id")?.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn job() -> Job {
        Job {
            id: "j1".into(),
            task: "imagine_exit".into(),
            args: Value::Null,
            status: JobStatus::Running,
            events: Vec::new(),
            result: None,
            proposals: Vec::new(),
            agent_session: None,
            model: None,
            cost_usd: None,
        }
    }

    #[test]
    fn the_stream_gives_events_proposals_and_the_result() {
        let mut r = StreamReader::default();
        let mut j = job();
        let lines = [
            json!({ "type": "system", "subtype": "init", "session_id": "s1" }),
            json!({ "type": "assistant", "message": { "content": [
                { "type": "text", "text": "Looking at the shore." },
                { "type": "tool_use", "id": "t1", "name": "mcp__loremaster__propose", "input": {} }
            ] } }),
            json!({ "type": "user", "message": { "content": [
                { "type": "tool_result", "tool_use_id": "t1", "content": [{ "type": "text", "text": "{\"id\":\"p4\",\"ok\":true}" }] }
            ] } }),
            json!({ "type": "assistant", "message": { "content": [
                { "type": "tool_use", "id": "t2", "name": "mcp__loremaster__propose", "input": {} }
            ] } }),
            json!({ "type": "user", "message": { "content": [
                { "type": "tool_result", "tool_use_id": "t2", "content": "{\"id\":\"p5\",\"ok\":false}" }
            ] } }),
            json!({ "type": "assistant", "message": { "content": [
                { "type": "tool_use", "id": "t3", "name": "mcp__loremaster__withdraw_proposal", "input": { "id": "p5" } }
            ] } }),
            json!({ "type": "result", "result": "p4: a pier", "total_cost_usd": 0.01, "session_id": "s1" }),
        ];
        for l in lines {
            r.line(&l.to_string(), &mut j);
        }
        assert_eq!(j.proposals, ["p4"]);
        assert_eq!(j.result.as_deref(), Some("p4: a pier"));
        assert_eq!(j.agent_session.as_deref(), Some("s1"));
        let tools: Vec<&str> = j
            .events
            .iter()
            .filter(|e| e.kind == "tool")
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(tools, ["propose", "propose", "withdraw_proposal"]);
    }

    #[test]
    fn the_agent_gets_no_built_in_tools_that_read_or_run() {
        let agent = Agent {
            program: "claude".into(),
            model: Some("haiku".into()),
            effort: Some("low".into()),
        };
        let cmd = agent.command("hello", "{}", None);
        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        let after = |flag: &str| {
            let i = args.iter().position(|a| a == flag).unwrap();
            args[i + 1].to_string()
        };
        assert_eq!(
            after("--tools"),
            "ToolSearch,ListMcpResourcesTool,ReadMcpResourceTool"
        );
        assert!(after("--allowedTools").starts_with("mcp__loremaster__*,"));
        assert_eq!(after("--model"), "haiku");
        assert_eq!(after("--effort"), "low");
        let own = agent.command("hello", "{}", Some("sonnet"));
        assert!(own.get_args().any(|a| a == "sonnet"));
        let blank = agent.command("hello", "{}", Some(" "));
        assert!(blank.get_args().any(|a| a == "haiku"));
        assert_eq!(pick_model(Some(""), Some("")), None);
        assert_eq!(setting(None, DEFAULT_MODEL).as_deref(), Some("sonnet"));
        assert_eq!(setting(Some("".into()), DEFAULT_MODEL), None);
        assert_eq!(
            setting(Some("opus".into()), DEFAULT_MODEL).as_deref(),
            Some("opus")
        );
        assert!(args.iter().any(|a| a == "--strict-mcp-config"));
    }

    #[test]
    fn the_list_is_in_creation_order() {
        let jobs = Jobs::new(Agent::from_env(), "http://127.0.0.1:1/mcp");
        for n in [10, 2, 1] {
            let mut j = job();
            j.id = format!("j{n}");
            jobs.jobs.lock().unwrap().insert(j.id.clone(), j);
        }
        let ids: Vec<String> = jobs.list().into_iter().map(|j| j.id).collect();
        assert_eq!(ids, ["j1", "j2", "j10"]);
    }

    #[test]
    fn a_fake_agent_runs_to_the_end() {
        let dir = std::env::temp_dir().join(format!("loremaster-job-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("fake-claude");
        let result = json!({ "type": "result", "result": "done" }).to_string();
        // A quoted heredoc passes the JSON as it is, also with quotes in it.
        std::fs::write(&script, format!("#!/bin/sh\ncat <<'EOF'\n{result}\nEOF\n")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let agent = Agent {
            program: script.to_string_lossy().into(),
            model: None,
            effort: None,
        };
        let mut jobs = Jobs::new(agent, "http://127.0.0.1:1/mcp");
        let id = jobs.start("review_room", json!({}), "hello", None).unwrap();
        for _ in 0..100 {
            if jobs.get(&id).unwrap().status != JobStatus::Running {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let j = jobs.get(&id).unwrap();
        assert_eq!(j.status, JobStatus::Done);
        assert_eq!(j.result.as_deref(), Some("done"));
        std::fs::remove_dir_all(dir).ok();
    }
}
