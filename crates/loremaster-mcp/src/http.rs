//! `loremaster serve`: the MCP server and the proposal endpoints over local HTTP.
//!
//! The server listens on 127.0.0.1 only. Endpoints:
//!
//! | Method and path | What it does |
//! | --- | --- |
//! | `POST /mcp` | One MCP JSON-RPC message. The reply is JSON (streamable HTTP without streams). |
//! | `GET /proposals` | All open proposals, each checked against the project now. |
//! | `GET /proposals/<id>` | One proposal, checked. |
//! | `POST /proposals/<id>/accept` | Write the proposal into the project files. Body: `{ "applied": true }` when the editor wrote the change itself, `{ "force": true }` to write a stale proposal. |
//! | `POST /proposals/<id>/reject` | Remove the proposal. Body: `{ "note": "why" }`, optional. |
//! | `POST /jobs` | Start a job: `{ "task": "imagine_exit", "args": { "room": "shore", "direction": "down" }, "state": { ... } }`. |
//! | `GET /jobs`, `GET /jobs/<id>` | The jobs, with status, progress events, result and proposals. |
//! | `POST /jobs/<id>/cancel` | Stop a running job. |
//! | `POST /bootstrap` | Start a new story from an idea: `{ "title": "...", "idea": "...", "rooms": 3 }`. Writes the new story, then starts a `seed_story` job. 409 when the folder has a story. |
//! | `GET /health` | `{ "loremaster": "<version>", "ui": true }`. The editor uses it to find the server. |
//!
//! Each endpoint is also at `/api/...`, for example `GET /api/proposals`. With the
//! feature `embed-ui`, all other `GET` paths give the web editor.
//!
//! A web page on any other site must not use these endpoints. So the server
//! accepts a request with an `Origin` header only from `localhost`,
//! `127.0.0.1` or an origin in `--allow-origin`, and a `POST` only with
//! `Content-Type: application/json`, which makes a browser ask first (CORS preflight).

use crate::jobs::{Agent, Jobs};
use crate::{Refusal, Server};
use loremaster_tools::{render_prompt, render_prompt_with_context};
use serde_json::{Value, json};
use std::io;
use std::path::Path;
use tiny_http::{Header, Method, Request, Response};

/// The port when the command line gives none.
pub const DEFAULT_PORT: u16 = 4710;

/// How `loremaster serve` runs.
#[derive(Debug, Clone)]
pub struct Options {
    pub port: u16,
    /// More origins that may call the endpoints, for example `https://sunsided.github.io`.
    pub allow_origins: Vec<String>,
    /// Open the web editor in the browser. Only with the feature `embed-ui`.
    pub open: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            allow_origins: Vec::new(),
            open: true,
        }
    }
}

/// True when the binary has the web editor in it (the feature `embed-ui`).
pub const HAS_UI: bool = cfg!(feature = "embed-ui");

/// Serve the project in `root` on `127.0.0.1:<port>` until the process stops.
pub fn serve_http(root: &Path, options: &Options) -> io::Result<()> {
    let port = options.port;
    let mut server = Server::new(root.to_path_buf())?;
    let mut jobs = Jobs::new(Agent::from_env(), &format!("http://127.0.0.1:{port}/mcp"));
    let http = tiny_http::Server::http(("127.0.0.1", port)).map_err(io::Error::other)?;
    let url = format!("http://127.0.0.1:{port}/");
    eprintln!(
        "loremaster: serving {} on {url} (MCP at /mcp)",
        root.display()
    );
    if HAS_UI {
        eprintln!("loremaster: the editor is at {url}");
        if options.open {
            open_browser(&url);
        }
    }
    for request in http.incoming_requests() {
        handle(&mut server, &mut jobs, &options.allow_origins, request);
    }
    Ok(())
}

/// Open `url` in the default browser. A failure only prints a note.
fn open_browser(url: &str) {
    let program = if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else if cfg!(windows) {
        ("cmd", vec!["/C", "start", "", url])
    } else {
        ("xdg-open", vec![url])
    };
    if std::process::Command::new(program.0)
        .args(program.1)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_err()
    {
        eprintln!("loremaster: open {url} in your browser");
    }
}

fn handle(server: &mut Server, jobs: &mut Jobs, allowed: &[String], mut request: Request) {
    let origin = header(&request, "Origin");
    if let Some(o) = &origin
        && !local_origin(o)
        && !allowed.iter().any(|a| a == o)
    {
        let _ = request.respond(Response::from_string("Forbidden origin").with_status_code(403));
        return;
    }
    let cors = |r: Response<io::Cursor<Vec<u8>>>| match &origin {
        Some(o) => r
            .with_header(h("Access-Control-Allow-Origin", o))
            .with_header(h("Vary", "Origin")),
        None => r,
    };
    if *request.method() == Method::Options {
        let r = Response::from_data(Vec::new())
            .with_status_code(204)
            .with_header(h("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
            .with_header(h(
                "Access-Control-Allow-Headers",
                "Content-Type, Mcp-Session-Id, Mcp-Protocol-Version",
            ));
        let _ = request.respond(cors(r));
        return;
    }
    let is_post = *request.method() == Method::Post;
    if is_post
        && !header(&request, "Content-Type").is_some_and(|c| c.starts_with("application/json"))
    {
        let _ = request.respond(cors(error(415, "Use Content-Type: application/json.")));
        return;
    }
    let mut body = String::new();
    if is_post && request.as_reader().read_to_string(&mut body).is_err() {
        let _ = request.respond(cors(error(400, "The body is not UTF-8.")));
        return;
    }
    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or_default();
    let mut parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    let api = parts.first() == Some(&"api");
    if api {
        parts.remove(0);
    }
    let response = match (is_post, parts.as_slice()) {
        (false, ["health"]) => ok(json!({ "loremaster": env!("CARGO_PKG_VERSION"), "ui": HAS_UI })),
        (true, ["mcp"]) => match server.handle_line(&body) {
            Some(reply) => ok(reply),
            None => Response::from_data(Vec::new()).with_status_code(202),
        },
        (false, ["proposals"]) => {
            server.refresh();
            let ws = server.workspace();
            let views: Vec<Value> = ws
                .proposals()
                .filter_map(|p| ws.view_proposal(&p.id).ok())
                .map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
                .collect();
            ok(json!(views))
        }
        (false, ["proposals", id]) => {
            server.refresh();
            match server.workspace().view_proposal(id) {
                Ok(v) => ok(serde_json::to_value(v).unwrap_or(Value::Null)),
                Err(e) => error(404, &e.0),
            }
        }
        (true, ["proposals", id, "accept"]) => {
            match server.accept(id, flag(&body, "applied"), flag(&body, "force")) {
                Ok(files) => ok(json!({ "accepted": id, "files": files })),
                Err(e) => refused(&e),
            }
        }
        (true, ["proposals", id, "reject"]) => {
            let note: Option<String> = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| v.get("note")?.as_str().map(str::to_string));
            match server.reject(id, note.as_deref()) {
                Ok(()) => ok(json!({ "rejected": id })),
                Err(e) => refused(&e),
            }
        }
        (true, ["bootstrap"]) => match bootstrap(server, jobs, &body) {
            Ok(id) => ok(json!({ "id": id })).with_status_code(202),
            Err((status, e)) => error(status, &e),
        },
        (true, ["jobs"]) => match start_job(server, jobs, &body) {
            Ok(id) => ok(json!({ "id": id })).with_status_code(202),
            Err(e) => error(400, &e),
        },
        (false, ["jobs"]) => ok(serde_json::to_value(jobs.list()).unwrap_or(Value::Null)),
        (false, ["jobs", id]) => match jobs.get(id) {
            Some(j) => ok(serde_json::to_value(j).unwrap_or(Value::Null)),
            None => error(404, "There is no such job."),
        },
        (true, ["jobs", id, "cancel"]) => match jobs.cancel(id) {
            true => ok(json!({ "cancelled": id })),
            false => error(404, "There is no running job with this id."),
        },
        #[cfg(feature = "embed-ui")]
        (false, _) if !api => match crate::ui::file(path) {
            Some((bytes, kind)) => {
                Response::from_data(bytes.to_vec()).with_header(h("Content-Type", kind))
            }
            None => error(404, "Not found."),
        },
        _ => error(404, "Not found."),
    };
    let _ = request.respond(cors(response));
}

/// Start a job from a body `{ "task": "imagine_exit", "args": { ... }, "state": { ... } }`.
/// With `state` (the editor's save), the job plays in its own session from that state.
fn start_job(server: &mut Server, jobs: &mut Jobs, body: &str) -> Result<String, String> {
    let req: Value =
        serde_json::from_str(body).map_err(|e| format!("The body is not JSON: {e}"))?;
    let task = req
        .get("task")
        .and_then(Value::as_str)
        .ok_or("The body needs `task`.")?;
    let mut args = req.get("args").cloned().unwrap_or_else(|| json!({}));
    if !args.is_object() {
        return Err("`args` must be an object.".into());
    }
    let state = req.get("state");
    let session = format!("job-{}", jobs.next_id());
    if state.is_some() {
        args["session"] = json!(session);
    }
    if matches!(task, "imagine_exit" | "seed_story") && args.get("rejected").is_none() {
        let rejected = server.rejected(20);
        if !rejected.is_empty() {
            args["rejected"] = json!(rejected.join("\n"));
        }
    }
    let model = req.get("model").and_then(Value::as_str);
    // Check the task and its arguments first, so that a bad request leaves no session behind.
    render_prompt(task, &args).map_err(|e| e.0)?;
    if let Some(state) = state {
        server.call_tool(
            "session_start",
            json!({ "session": session, "state": state }),
        )?;
    }
    // The server gives the context in the prompt: that saves the agent several turns.
    let context = server.prompt_context(task, &args);
    let prompt = render_prompt_with_context(task, &args, Some(&context)).map_err(|e| e.0)?;
    jobs.start(task, args, &prompt, model).inspect_err(|_| {
        if state.is_some() {
            server.end_session(&session);
        }
    })
}

/// `POST /bootstrap`: write a new story with the title, then start `seed_story` with the idea.
fn bootstrap(server: &mut Server, jobs: &mut Jobs, body: &str) -> Result<String, (u16, String)> {
    let req: Value =
        serde_json::from_str(body).map_err(|e| (400, format!("The body is not JSON: {e}")))?;
    let text = |k: &str| {
        req.get(k)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let idea = text("idea").ok_or((400, "The body needs `idea`.".to_string()))?;
    server
        .bootstrap(text("title").unwrap_or("Untitled"))
        .map_err(|e| match e {
            Refusal::Conflict(m) => (409, m),
            other => (500, other.to_string()),
        })?;
    let mut args = json!({ "idea": idea });
    if let Some(rooms) = req.get("rooms").and_then(Value::as_u64) {
        args["rooms"] = json!(rooms.to_string());
    }
    let mut job = json!({ "task": "seed_story", "args": args });
    if let Some(model) = text("model") {
        job["model"] = json!(model);
    }
    start_job(server, jobs, &job.to_string()).map_err(|e| (400, e))
}

/// A boolean field of a JSON body; false when the body or the field is missing.
fn flag(body: &str, name: &str) -> bool {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get(name)?.as_bool())
        .unwrap_or(false)
}

/// The origin of a URL as a browser sends it in `Origin`: scheme, host and port, no path.
/// `https://example.com/app/` gives `https://example.com`. None when there is no `http` or `https` scheme or no host.
pub fn origin_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.trim().split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    (matches!(scheme.as_str(), "http" | "https") && !host.is_empty() && !host.contains('@'))
        .then(|| format!("{scheme}://{}", host.to_ascii_lowercase()))
}

/// True for `http://localhost`, `http://127.0.0.1` and `http://[::1]` on any port.
fn local_origin(origin: &str) -> bool {
    let Some(rest) = origin.strip_prefix("http://") else {
        return false;
    };
    let host = match rest.strip_prefix('[') {
        Some(v6) => v6.split(']').next().map(|h| format!("[{h}]")),
        None => rest.split(':').next().map(str::to_string),
    };
    matches!(host.as_deref(), Some("localhost" | "127.0.0.1" | "[::1]"))
}

fn header(request: &Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|hd| hd.field.equiv(name))
        .map(|hd| hd.value.to_string())
}

fn h(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid header")
}

fn ok(value: Value) -> Response<io::Cursor<Vec<u8>>> {
    Response::from_string(value.to_string()).with_header(h("Content-Type", "application/json"))
}

fn refused(r: &Refusal) -> Response<io::Cursor<Vec<u8>>> {
    let status = match r {
        Refusal::NotFound(_) => 404,
        Refusal::Conflict(_) => 409,
        Refusal::Io(_) => 500,
    };
    error(status, &r.to_string())
}

fn error(status: u16, message: &str) -> Response<io::Cursor<Vec<u8>>> {
    Response::from_string(json!({ "error": message }).to_string())
        .with_status_code(status)
        .with_header(h("Content-Type", "application/json"))
}

#[cfg(test)]
mod tests {
    use super::{local_origin, origin_of};

    #[test]
    fn origin_of_drops_the_path() {
        for (url, origin) in [
            (
                "https://sunsided.github.io/loremaster/",
                "https://sunsided.github.io",
            ),
            ("https://example.com", "https://example.com"),
            (
                "HTTPS://Example.com:8443/app?x=1",
                "https://example.com:8443",
            ),
            ("http://[::1]:5173/", "http://[::1]:5173"),
        ] {
            assert_eq!(origin_of(url).as_deref(), Some(origin), "{url}");
        }
        for url in [
            "example.com",
            "ftp://example.com",
            "https://",
            "https://user@evil.example",
        ] {
            assert_eq!(origin_of(url), None, "{url}");
        }
    }

    #[test]
    fn only_local_origins_pass() {
        for o in [
            "http://localhost:5173",
            "http://127.0.0.1",
            "http://[::1]:8080",
        ] {
            assert!(local_origin(o), "{o}");
        }
        for o in [
            "https://evil.example",
            "http://localhost.evil.example",
            "http://127.0.0.1.evil.example:80",
            "null",
        ] {
            assert!(!local_origin(o), "{o}");
        }
    }
}
