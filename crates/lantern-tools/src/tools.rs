//! The tool list and what each tool does.

use crate::{Args, ToolResult, Workspace, err, json};
use lantern_model::{Effect, Layer, Location};
use lantern_project::SourceFile;
use lantern_runtime::{AuthorCommand, Budget, Command, Game, State, Turn};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};

/// One tool: its name, what it does, and a JSON Schema for its arguments.
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
    /// True when the tool changes nothing, not even a play session.
    pub read_only: bool,
}

const SESSION: &str = "The play session. Default: \"main\", which starts on first use.";

fn schema(props: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": props, "required": required })
}

fn session_only() -> Value {
    schema(
        json!({ "session": { "type": "string", "description": SESSION } }),
        &[],
    )
}

fn def(
    name: &'static str,
    description: &'static str,
    input_schema: Value,
    read_only: bool,
) -> ToolDef {
    ToolDef {
        name,
        description,
        input_schema,
        read_only,
    }
}

/// All tools, in the order a model should learn them.
pub fn tools() -> Vec<ToolDef> {
    vec![
        // The world as written.
        def(
            "overview",
            "Start here. The story: title, start room, clock phases, variables, verbs, and an index of all rooms (with exits), things (with their start location and the rules that move them) and rules. Also the number of problems that `check` finds.",
            schema(json!({}), &[]),
            true,
        ),
        def(
            "list_files",
            "The paths of all project files. Rooms are in rooms/, things in things/, rules in rules/, and story.yaml holds the rest.",
            schema(json!({}), &[]),
            true,
        ),
        def(
            "read_file",
            "The YAML source of one project file.",
            schema(
                json!({ "path": { "type": "string", "description": "For example rooms/shore.yaml." } }),
                &["path"],
            ),
            true,
        ),
        def(
            "get_entity",
            "The YAML source of a room, thing or rule, with the file it is in. For a room, also the things that start in it.",
            schema(
                json!({
                    "kind": { "type": "string", "enum": ["room", "thing", "rule"] },
                    "id": { "type": "string" }
                }),
                &["kind", "id"],
            ),
            true,
        ),
        def(
            "neighbors",
            "The rooms near a room, up to `hops` exits away in either direction, with the path from the room, region, level, tags and the opening sentence of each. Use it to keep a new or changed room consistent with its surroundings.",
            schema(
                json!({
                    "room": { "type": "string" },
                    "hops": { "type": "integer", "minimum": 1, "maximum": 4, "description": "Default 1." }
                }),
                &["room"],
            ),
            true,
        ),
        // Verify.
        def(
            "check",
            "All problems in the project: errors and warnings with file, location and line. A project with errors cannot be played.",
            schema(json!({}), &[]),
            true,
        ),
        def(
            "explore",
            "Play every way through the story from the start and report: the text variants of each room and whether it was reached, endings with the shortest command list, stuck states, fragments never shown and rules never fired. Can take some seconds on a large story.",
            schema(
                json!({
                    "max_states": { "type": "integer", "minimum": 1, "description": "Default 2000." },
                    "max_depth": { "type": "integer", "minimum": 1, "description": "Default 40." }
                }),
                &[],
            ),
            true,
        ),
        def(
            "room_matrix",
            "The text of one room in every combination of the values that the text depends on, with the combinations that give each text and marks for possible problems. Use it to review a room in all its states, not only the one being played.",
            schema(
                json!({ "room": { "type": "string" }, "session": { "type": "string", "description": SESSION } }),
                &["room"],
            ),
            true,
        ),
        def(
            "text_matrix",
            "Like room_matrix, but for a piece of text that is not in the project yet: render a draft fragment (with inline markup) in all combinations of the values it uses.",
            schema(
                json!({
                    "text": { "type": "string" },
                    "when": { "type": "string", "description": "An optional condition for the fragment." },
                    "session": { "type": "string", "description": SESSION }
                }),
                &["text"],
            ),
            true,
        ),
        def(
            "try_markup",
            "Check a draft text with inline markup, such as \"{night: dark | bright}\", and render it in the current state of the session. Returns the first error, if any.",
            schema(
                json!({ "text": { "type": "string" }, "session": { "type": "string", "description": SESSION } }),
                &["text"],
            ),
            true,
        ),
        def(
            "try_condition",
            "Check a draft condition, such as \"lamp.lit and night\". Returns the first error, if any.",
            schema(
                json!({ "condition": { "type": "string" }, "session": { "type": "string", "description": SESSION } }),
                &["condition"],
            ),
            true,
        ),
        def(
            "symbols",
            "All names that conditions and markup can use (variables, thing properties, names, derived values such as time.phase), with their values now.",
            session_only(),
            true,
        ),
        // Propose.
        def(
            "propose",
            "Suggest a change to the project. Give whole files: each new or changed file with its complete YAML text, and the paths of files to delete. In a changed file, keep every line you do not change byte for byte (read it with `read_file` first), so that the person sees only your change. Nothing changes until a person accepts it. Returns the proposal id and its check: the problems it adds, the changes per entity and fragment, and the text of each changed room in every state. Fix the problems with a new proposal, then `withdraw_proposal` the old one.",
            schema(
                json!({
                    "summary": { "type": "string", "description": "One line: what the change does and why." },
                    "files": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": { "path": { "type": "string" }, "text": { "type": "string" } },
                            "required": ["path", "text"]
                        },
                        "description": "For example [{\"path\":\"rooms/pier.yaml\",\"text\":\"name: The Pier\\n...\"}]."
                    },
                    "delete": { "type": "array", "items": { "type": "string" } }
                }),
                &["summary"],
            ),
            false,
        ),
        def(
            "list_proposals",
            "The open proposals, with id, summary, files and whether each has no errors now.",
            schema(json!({}), &[]),
            true,
        ),
        def(
            "get_proposal",
            "One proposal, checked against the project as it is now.",
            schema(json!({ "id": { "type": "string" } }), &["id"]),
            true,
        ),
        def(
            "withdraw_proposal",
            "Remove one of your proposals, for example after you made a better one.",
            schema(json!({ "id": { "type": "string" } }), &["id"]),
            false,
        ),
        // Play.
        def(
            "session_start",
            "Start a play session from the beginning, or from a saved state. Replaces a session of the same name. Returns the opening text. With `proposal`, the session plays the project with that proposal applied, so you can walk into a draft before anyone accepts it.",
            schema(
                json!({
                    "session": { "type": "string", "description": SESSION },
                    "state": { "type": "object", "description": "A state from `save`, or from the editor." },
                    "proposal": { "type": "string", "description": "The id of a proposal to play." }
                }),
                &[],
            ),
            false,
        ),
        def(
            "input",
            "Type one player command, such as \"go north\" or \"light lamp\", and get the text of the turn.",
            schema(
                json!({ "text": { "type": "string" }, "session": { "type": "string", "description": SESSION } }),
                &["text"],
            ),
            false,
        ),
        def(
            "act",
            "Do one player action without the parser: {\"type\":\"verb\",\"verb\":\"take\",\"objects\":[\"lamp\"]} or {\"type\":\"go\",\"direction\":\"north\"}.",
            schema(
                json!({ "command": { "type": "object" }, "session": { "type": "string", "description": SESSION } }),
                &["command"],
            ),
            false,
        ),
        def(
            "author",
            "Change the session state directly, as an author: {\"type\":\"goto\",\"room\":\"cellar\"}, {\"type\":\"set\",\"name\":\"lamp.lit\",\"value\":true}, {\"type\":\"time\",\"time\":\"22:00\"}, {\"type\":\"give\",\"thing\":\"key\"}, or {\"type\":\"situation\",\"room\":\"shore\",\"values\":[[\"time.phase\",\"night\"]]} for one room_matrix combination.",
            schema(
                json!({ "command": { "type": "object" }, "session": { "type": "string", "description": SESSION } }),
                &["command"],
            ),
            false,
        ),
        def(
            "scene",
            "Where the session is now: room, time, phase, day, turn, the things here, the inventory and the exits.",
            session_only(),
            true,
        ),
        def(
            "assembly",
            "How the description of the current room is put together now: each fragment with its layer, condition and whether it shows.",
            session_only(),
            true,
        ),
        def(
            "room_parts",
            "The things and exits of the current room, with the text each adds and whether it shows now.",
            session_only(),
            true,
        ),
        def(
            "inspect",
            "Facts about a thing (kind \"thing\", id its id) or an exit of the current room (kind \"exit\", id its direction).",
            schema(
                json!({
                    "kind": { "type": "string", "enum": ["thing", "exit"] },
                    "id": { "type": "string" },
                    "session": { "type": "string", "description": SESSION }
                }),
                &["kind", "id"],
            ),
            true,
        ),
        def(
            "vars",
            "All variables and thing properties with their values now.",
            session_only(),
            true,
        ),
        def(
            "rules",
            "All rules with their state now: whether each has fired and whether it can fire.",
            session_only(),
            true,
        ),
        def(
            "verbs",
            "The verbs the player can use, with their words.",
            session_only(),
            true,
        ),
        def(
            "map",
            "All rooms with position and level, and all exits as links, as the session knows them now.",
            session_only(),
            true,
        ),
        def(
            "save",
            "The state of the session, to give back to `session_start` later.",
            session_only(),
            true,
        ),
    ]
}

pub(crate) fn dispatch(ws: &mut Workspace, name: &str, a: &Args) -> ToolResult {
    match name {
        "overview" => overview(ws),
        "list_files" => Ok(json!(ws.files.iter().map(|f| &f.path).collect::<Vec<_>>())),
        "read_file" => {
            let path = a.str("path")?;
            ws.files
                .iter()
                .find(|f| f.path == path)
                .map(|f| json!({ "path": f.path, "text": f.text }))
                .ok_or_else(|| err(format!("There is no file `{path}`. Call `list_files`.")))
        }
        "get_entity" => get_entity(ws, a.str("kind")?, a.str("id")?),
        "neighbors" => neighbors(ws, a.str("room")?, a.usize_or("hops", 1).clamp(1, 4)),
        "check" => Ok(json!({
            "errors": ws.diagnostics.iter().filter(|d| d.severity == lantern_project::Severity::Error).count(),
            "problems": ws.diagnostics,
        })),
        "explore" => {
            let budget = Budget {
                max_states: a
                    .usize_or("max_states", Budget::default().max_states)
                    .max(1),
                max_depth: a.usize_or("max_depth", Budget::default().max_depth).max(1),
            };
            let fresh = Game::new(ws.world()?.clone());
            json(&lantern_runtime::explore(&fresh, budget))
        }
        "room_matrix" => {
            let room = a.str("room")?;
            let game = ws.session(a.session())?;
            match lantern_runtime::room_matrix(game, room) {
                Some(m) => json(&m),
                None => Err(err(format!("There is no room `{room}`."))),
            }
        }
        "text_matrix" => {
            let text = a.str("text")?;
            let when = a.opt_str("when");
            let game = ws.session(a.session())?;
            match lantern_runtime::text_matrix(game, text, when) {
                Some(m) => json(&m),
                None => Err(err(
                    "The text or the condition does not parse. Call `try_markup` or `try_condition` to see why.",
                )),
            }
        }
        "try_markup" => {
            let text = a.str("text")?;
            json(&ws.session(a.session())?.try_markup(text))
        }
        "try_condition" => {
            let cond = a.str("condition")?;
            json(&ws.session(a.session())?.try_condition(cond))
        }
        "symbols" => json(&ws.session(a.session())?.symbols()),
        "session_start" => {
            let world = match a.opt_str("proposal") {
                Some(id) => ws.proposal_world(id)?,
                None => ws.world()?.clone(),
            };
            let state: Option<State> = a.opt_value("state")?;
            let name = a.session().to_string();
            let (game, opening) = match state {
                Some(state) => {
                    let game = Game::restore(world, state);
                    let text = game.describe_preview().plain_text();
                    (game, text)
                }
                None => {
                    let mut game = Game::new(world);
                    let text = game.start().plain_text();
                    (game, text)
                }
            };
            ws.sessions.insert(name.clone(), game);
            let game = ws.session(&name)?;
            Ok(json!({ "session": name, "text": opening, "scene": brief(game) }))
        }
        "input" => {
            let text = a.str("text")?;
            let game = ws.session(a.session())?;
            let turn = game.input(text);
            Ok(turn_json(&turn, game))
        }
        "act" => {
            let cmd: Command = a.value("command")?;
            let game = ws.session(a.session())?;
            let turn = game.act(cmd);
            Ok(turn_json(&turn, game))
        }
        "author" => {
            let cmd: AuthorCommand = a.value("command")?;
            let game = ws.session(a.session())?;
            let turn = game.author(cmd);
            Ok(turn_json(&turn, game))
        }
        "propose" => {
            let files: Vec<SourceFile> = a.opt_value("files")?.unwrap_or_default();
            let delete: Vec<String> = a.opt_value("delete")?.unwrap_or_default();
            let id = ws.add_proposal(a.str("summary")?.to_string(), files, delete)?;
            json(&ws.view_proposal(&id)?)
        }
        "list_proposals" => {
            let list: Vec<Value> = ws
                .proposals()
                .map(|p| {
                    let ok = ws.view_proposal(&p.id).is_ok_and(|v| v.ok);
                    json!({ "id": p.id, "summary": p.summary, "files": p.files.iter().map(|f| &f.path).collect::<Vec<_>>(), "delete": p.delete, "ok": ok })
                })
                .collect();
            Ok(json!(list))
        }
        "get_proposal" => json(&ws.view_proposal(a.str("id")?)?),
        "withdraw_proposal" => {
            let id = a.str("id")?;
            ws.find_proposal(id)?;
            ws.remove_proposal(id);
            Ok(json!({ "withdrawn": id }))
        }
        "scene" => json(&ws.session(a.session())?.scene()),
        "assembly" => json(&ws.session(a.session())?.assembly()),
        "room_parts" => json(&ws.session(a.session())?.room_parts()),
        "inspect" => {
            let (kind, id) = (a.str("kind")?, a.str("id")?);
            let game = ws.session(a.session())?;
            let found = match kind {
                "thing" => game.inspect_thing(id),
                "exit" => game.inspect_exit(id),
                _ => return Err(err("`kind` must be \"thing\" or \"exit\".")),
            };
            found
                .map(|i| json(&i))
                .unwrap_or_else(|| Err(err(format!("There is no {kind} `{id}` here."))))
        }
        "vars" => json(&ws.session(a.session())?.vars()),
        "rules" => json(&ws.session(a.session())?.rules()),
        "verbs" => json(&ws.session(a.session())?.verbs()),
        "map" => json(&ws.session(a.session())?.map()),
        "save" => json(ws.session(a.session())?.state()),
        _ => Err(err(format!("There is no tool `{name}`."))),
    }
}

/// The result of one turn: its text, whether the game ended, the log of what
/// changed, and where the player is now.
fn turn_json(turn: &Turn, game: &Game) -> Value {
    json!({
        "text": turn.plain_text(),
        "ended": turn.ended,
        "log": turn.log,
        "scene": brief(game),
    })
}

fn brief(game: &Game) -> Value {
    let s = game.scene();
    json!({ "room": s.room, "room_name": s.room_name, "time": s.time, "phase": s.phase, "day": s.day, "turns": s.turns })
}

fn overview(ws: &Workspace) -> ToolResult {
    let story = &ws.project()?.story;
    let rooms: Vec<Value> = story
        .rooms
        .iter()
        .map(|r| {
            let exits: BTreeMap<String, &str> = r
                .exits
                .iter()
                .map(|e| (e.direction.to_string(), e.to.as_str()))
                .collect();
            json!({ "id": r.id, "name": r.name, "region": r.region, "tags": r.tags, "exits": exits })
        })
        .collect();
    let things: Vec<Value> = story
        .things
        .iter()
        .map(|t| {
            // A thing can start nowhere and come into play through a rule; name those rules.
            let moved_by: Vec<&str> = story
                .rules
                .iter()
                .filter(|r| {
                    r.then
                        .iter()
                        .any(|e| matches!(e, Effect::Move(m) if m.contains_key(&t.id)))
                })
                .map(|r| r.id.as_str())
                .collect();
            json!({ "id": t.id, "name": t.name, "location": location(&t.location), "moved_by": moved_by })
        })
        .collect();
    let errors = ws
        .diagnostics
        .iter()
        .filter(|d| d.severity == lantern_project::Severity::Error)
        .count();
    Ok(json!({
        "title": story.title,
        "author": story.author,
        "start": story.start,
        "clock": story.clock,
        "vars": story.vars,
        "verbs": story.verbs.iter().map(|v| &v.id).collect::<Vec<_>>(),
        "rooms": rooms,
        "things": things,
        "rules": story.rules.iter().map(|r| &r.id).collect::<Vec<_>>(),
        "problems": { "errors": errors, "warnings": ws.diagnostics.len() - errors },
    }))
}

fn location(loc: &Location) -> String {
    match loc {
        Location::Room(id) => id.to_string(),
        Location::Carried => "carried".into(),
        Location::Nowhere => "nowhere".into(),
    }
}

fn get_entity(ws: &Workspace, kind: &str, id: &str) -> ToolResult {
    if !matches!(kind, "room" | "thing" | "rule") {
        return Err(err("`kind` must be \"room\", \"thing\" or \"rule\"."));
    }
    let project = ws.project()?;
    let path = project
        .origins
        .get(&format!("{kind}:{id}"))
        .ok_or_else(|| {
            err(format!(
                "There is no {kind} `{id}`. Call `overview` for the ids."
            ))
        })?;
    let text = project
        .file(path)
        .map(|f| f.text.as_str())
        .unwrap_or_default();
    let mut out = json!({ "kind": kind, "id": id, "path": path, "text": text });
    if kind == "room" {
        let things: Vec<&str> = project
            .story
            .things
            .iter()
            .filter(|t| matches!(&t.location, Location::Room(r) if r.as_str() == id))
            .map(|t| t.id.as_str())
            .collect();
        out["things"] = json!(things);
    }
    Ok(out)
}

/// Rooms within `hops` exits of `room`, following exits both ways.
fn neighbors(ws: &Workspace, room: &str, hops: usize) -> ToolResult {
    let story = &ws.project()?.story;
    if story.room(room).is_none() {
        return Err(err(format!("There is no room `{room}`.")));
    }
    // Own exits first, so a path names the exit the player takes. A room reached
    // only through an exit that points here reads as "from <room>: <direction> leads here".
    let mut forward: BTreeMap<&str, Vec<(String, &str)>> = BTreeMap::new();
    let mut backward: BTreeMap<&str, Vec<(String, &str)>> = BTreeMap::new();
    for r in &story.rooms {
        for e in &r.exits {
            forward
                .entry(r.id.as_str())
                .or_default()
                .push((e.direction.to_string(), e.to.as_str()));
            backward.entry(e.to.as_str()).or_default().push((
                format!("from {}: {} leads here", r.id, e.direction),
                r.id.as_str(),
            ));
        }
    }
    let mut paths: BTreeMap<&str, Vec<String>> = BTreeMap::from([(room, Vec::new())]);
    let mut queue = VecDeque::from([room]);
    while let Some(at) = queue.pop_front() {
        let path = paths[at].clone();
        if path.len() >= hops {
            continue;
        }
        for (dir, to) in [&forward, &backward]
            .into_iter()
            .flat_map(|m| m.get(at).into_iter().flatten())
        {
            if !paths.contains_key(to) {
                let mut p = path.clone();
                p.push(dir.clone());
                paths.insert(to, p);
                queue.push_back(to);
            }
        }
    }
    let world = ws.world.as_ref();
    let rooms: Vec<Value> = story
        .rooms
        .iter()
        .filter(|r| r.id.as_str() != room)
        .filter_map(|r| {
            let path = paths.get(r.id.as_str())?;
            let opening = r
                .description
                .iter()
                .find(|f| f.when.is_none() && f.layer == Layer::Base)
                .and_then(|f| f.variants.first());
            Some(json!({
                "id": r.id,
                "name": r.name,
                "hops": path.len(),
                "path": path,
                "region": r.region,
                "level": world.map(|w| w.level_of(r.id.as_str())),
                "tags": r.tags,
                "opening": opening,
            }))
        })
        .collect();
    Ok(json!({ "room": room, "neighbors": rooms }))
}
