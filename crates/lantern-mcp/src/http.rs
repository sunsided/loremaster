//! `lantern serve`: the MCP server and the proposal endpoints over local HTTP.
//!
//! The server listens on 127.0.0.1 only. Endpoints:
//!
//! | Method and path | What it does |
//! | --- | --- |
//! | `POST /mcp` | One MCP JSON-RPC message. The reply is JSON (streamable HTTP without streams). |
//! | `GET /proposals` | All open proposals, each checked against the project now. |
//! | `GET /proposals/<id>` | One proposal, checked. |
//! | `POST /proposals/<id>/accept` | Write the proposal into the project files. |
//! | `POST /proposals/<id>/reject` | Remove the proposal. Body: `{ "note": "why" }`, optional. |
//!
//! A web page on any other site must not use these endpoints. So the server
//! accepts a request with an `Origin` header only from `localhost` or
//! `127.0.0.1`, and a `POST` only with `Content-Type: application/json`,
//! which makes a browser ask first (CORS preflight).

use crate::Server;
use serde_json::{Value, json};
use std::io;
use std::path::Path;
use tiny_http::{Header, Method, Request, Response};

/// The port when the command line gives none.
pub const DEFAULT_PORT: u16 = 4710;

/// Serve the project in `root` on `127.0.0.1:<port>` until the process stops.
pub fn serve_http(root: &Path, port: u16) -> io::Result<()> {
    let mut server = Server::new(root.to_path_buf())?;
    let http = tiny_http::Server::http(("127.0.0.1", port)).map_err(io::Error::other)?;
    eprintln!(
        "lantern: serving {} on http://127.0.0.1:{port} (MCP at /mcp)",
        root.display()
    );
    for request in http.incoming_requests() {
        handle(&mut server, request);
    }
    Ok(())
}

fn handle(server: &mut Server, mut request: Request) {
    let origin = header(&request, "Origin");
    if let Some(o) = &origin
        && !local_origin(o)
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
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    let response = match (is_post, parts.as_slice()) {
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
        (true, ["proposals", id, "accept"]) => match server.accept(id) {
            Ok(files) => ok(json!({ "accepted": id, "files": files })),
            Err(e) => error(409, &e),
        },
        (true, ["proposals", id, "reject"]) => {
            let note: Option<String> = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| v.get("note")?.as_str().map(str::to_string));
            match server.reject(id, note.as_deref()) {
                Ok(()) => ok(json!({ "rejected": id })),
                Err(e) => error(404, &e),
            }
        }
        _ => error(404, "Not found."),
    };
    let _ = request.respond(cors(response));
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

fn error(status: u16, message: &str) -> Response<io::Cursor<Vec<u8>>> {
    Response::from_string(json!({ "error": message }).to_string())
        .with_status_code(status)
        .with_header(h("Content-Type", "application/json"))
}

#[cfg(test)]
mod tests {
    use super::local_origin;

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
