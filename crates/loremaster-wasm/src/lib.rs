//! WebAssembly bindings: the web editor and player drive the same runtime as
//! the CLI through this API. Values cross the boundary as plain JS objects
//! shaped like the runtime's serde types.
//!
//! Two classes mirror the split of ADR 0003: `Session` plays a story and
//! returns steps of effects (the Session API); `Workbench` owns a session and
//! adds the authoring views of the editor, and its play methods return whole
//! turns with the rule log for the editor's transcript (the Workbench API).
//! Their methods call only `Session` and `Workbench`.

use loremaster_model::{Value, Walkthrough};
use loremaster_project::{Diagnostic, SourceFile, check_files, load};
use loremaster_runtime::{HintBook, Input, Session, Snapshot, World};
use loremaster_workbench::{AuthorCommand, Budget, MarkupCheck, Workbench};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// A byte offset as a UTF-16 offset, for JavaScript strings.
fn utf16(text: &str, byte: usize) -> usize {
    text.get(..byte.min(text.len()))
        .map_or(0, |s| s.encode_utf16().count())
}

fn utf16_error(text: &str, mut check: MarkupCheck) -> MarkupCheck {
    if let Some(e) = &mut check.error {
        e.start = utf16(text, e.start);
        e.end = utf16(text, e.end);
    }
    check
}

fn from_js<T: serde::de::DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn or_null<T: Serialize>(value: Option<T>) -> Result<JsValue, JsValue> {
    match value {
        Some(v) => to_js(&v),
        None => Ok(JsValue::NULL),
    }
}

/// A compiled story, the file of each entity, and the walkthroughs.
struct Story {
    world: Arc<World>,
    origins: BTreeMap<String, String>,
    walkthroughs: Vec<(String, Walkthrough)>,
}

/// Build the story of project files (`[{ path, text }]`). Throws an array of
/// diagnostics if the project has errors.
fn compile(files: JsValue) -> Result<Story, JsValue> {
    let files: Vec<SourceFile> = from_js(files)?;
    let checked = check_files(files);
    let project = match checked.project {
        Some(project) if !checked.has_errors() => project,
        _ => return Err(to_js(&checked.diagnostics)?),
    };
    let world = World::compile(project.story.clone())
        .map_err(|e| to_js(&vec![project.locate(Diagnostic::from(&e))]).unwrap_or(JsValue::NULL))?;
    Ok(Story {
        world: Arc::new(world),
        walkthroughs: project.walkthroughs,
        // The editor opens files by path. Origins in Rust (DSL step L6) get
        // their own field when the editor shows them as read-only.
        origins: project
            .origins
            .into_iter()
            .map(|(key, origin)| (key, origin.file().to_string()))
            .collect(),
    })
}

/// Restore a save into a new session of `world`, also an older save or one
/// of an earlier version of the story.
fn load_save(world: &Arc<World>, save: JsValue) -> Result<(Session, JsValue), JsValue> {
    let save: serde_json::Value = from_js(save)?;
    let (session, loaded) =
        Session::load(world.clone(), save).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok((session, to_js(&loaded)?))
}

/// Highlighted spans of fragment text (or of a condition), in UTF-16 offsets.
#[wasm_bindgen(js_name = markupTokens)]
pub fn markup_tokens(text: &str, condition: bool) -> Result<JsValue, JsValue> {
    let tokens = if condition {
        loremaster_lang::condition_tokens(text)
    } else {
        loremaster_lang::markup_tokens(text)
    };
    let out: Vec<_> = tokens
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "from": utf16(text, t.span.start),
                "to": utf16(text, t.span.end),
                "kind": t.kind,
            })
        })
        .collect();
    to_js(&out)
}

#[wasm_bindgen(start)]
fn init() {
    console_error_panic_hook::set_once();
}

/// Load and check project files (`[{ path, text }]`). Returns diagnostics.
#[wasm_bindgen(js_name = checkProject)]
pub fn check_project(files: JsValue) -> Result<JsValue, JsValue> {
    let files: Vec<SourceFile> = from_js(files)?;
    to_js(&check_files(files).diagnostics)
}

/// The files of a new story with this title (`[{ path, text }]`).
#[wasm_bindgen(js_name = newStory)]
pub fn new_story(title: &str) -> Result<JsValue, JsValue> {
    to_js(&loremaster_project::scaffold(title))
}

/// All author notes of the project files, with their place. Empty when the files do not load.
#[wasm_bindgen(js_name = projectNotes)]
pub fn project_notes(files: JsValue) -> Result<JsValue, JsValue> {
    let files: Vec<SourceFile> = from_js(files)?;
    match load(files) {
        Ok(project) => to_js(&loremaster_project::notes(&project)),
        Err(_) => to_js(&Vec::<loremaster_project::NoteRef>::new()),
    }
}

/// The quality report of the project files (CK-8): `{ findings, widths, verbs,
/// deferred }`. Null when the files do not load.
#[wasm_bindgen(js_name = qualityReport)]
pub fn quality_report(files: JsValue) -> Result<JsValue, JsValue> {
    let files: Vec<SourceFile> = from_js(files)?;
    match load(files) {
        Ok(project) => to_js(&loremaster_project::quality(&project)),
        Err(_) => Ok(JsValue::NULL),
    }
}

/// All threads of the project files, with their beats. Empty when the files do not load.
#[wasm_bindgen(js_name = projectThreads)]
pub fn project_threads(files: JsValue) -> Result<JsValue, JsValue> {
    let files: Vec<SourceFile> = from_js(files)?;
    match load(files) {
        Ok(project) => to_js(&loremaster_project::threads(&project)),
        Err(_) => to_js(&Vec::<loremaster_project::ThreadView>::new()),
    }
}

/// A running game for a player (the Session API, ADR 0003). Every call that
/// plays returns a step: `{ effects, status }`.
#[wasm_bindgen(js_name = Session)]
pub struct JsSession {
    world: Arc<World>,
    session: Session,
    /// The hint ladder of the walkthroughs, also for a session from a save.
    hints: HintBook,
}

#[wasm_bindgen(js_class = Session)]
impl JsSession {
    /// A new game of project files. Throws an array of diagnostics if the
    /// project has errors.
    #[wasm_bindgen(constructor)]
    pub fn new(files: JsValue) -> Result<JsSession, JsValue> {
        let story = compile(files)?;
        let hints = HintBook::new(story.world.clone(), &story.walkthroughs);
        let mut session = Session::new(story.world.clone());
        session.set_hints(hints.clone());
        Ok(JsSession {
            session,
            world: story.world,
            hints,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn title(&self) -> String {
        self.session.world().story.title.clone()
    }

    /// The opening. A second call starts nothing again.
    pub fn start(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.session.start())
    }

    /// One input: `{ type: "text", value }`, `{ type: "command", value }`,
    /// `{ type: "choice", value }` or `{ type: "tick" }`.
    pub fn submit(&mut self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: Input = from_js(input)?;
        to_js(&self.session.submit(input))
    }

    /// Choose an answer of the talk in progress, by its index in `choices`.
    pub fn choose(&mut self, index: usize) -> Result<JsValue, JsValue> {
        to_js(&self.session.choose(index))
    }

    /// Answer the request of `Status::Await`, also the death prompt. Throws
    /// when the id or the value does not fit.
    pub fn resume(&mut self, await_id: &str, value: JsValue) -> Result<JsValue, JsValue> {
        let value: Value = from_js(value)?;
        let step = self
            .session
            .resume(await_id, value)
            .map_err(|e| JsValue::from_str(&e))?;
        to_js(&step)
    }

    /// Go back to the state before the last input.
    pub fn undo(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.session.undo())
    }

    /// How many inputs `undo` can go back now.
    #[wasm_bindgen(js_name = undoSteps)]
    pub fn undo_steps(&self) -> usize {
        self.session.undo_steps()
    }

    /// How many inputs `undo` can go back. 0 turns undo off.
    #[wasm_bindgen(js_name = setUndoDepth)]
    pub fn set_undo_depth(&mut self, depth: usize) {
        self.session.set_undo_depth(depth);
    }

    /// Let the host answer rolls with `resume`.
    #[wasm_bindgen(js_name = setHostRolls)]
    pub fn set_host_rolls(&mut self, host_rolls: bool) {
        self.session.set_host_rolls(host_rolls);
    }

    /// Start the story again. `undo` can go back to the state before it.
    pub fn restart(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.session.restart())
    }

    /// Throws what differs when the host was built for another interface
    /// than the story has (IF-5). `built_for` is the interface as JSON.
    #[wasm_bindgen(js_name = checkInterface)]
    pub fn check_interface(&self, built_for: &str) -> Result<(), JsValue> {
        let built = loremaster_model::Interface::from_json(built_for)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.session
            .check_interface(&built)
            .map_err(|m| JsValue::from_str(&m.to_string()))
    }

    /// The state now, to put back later with `restore`.
    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.snapshot())
    }

    /// Put a snapshot back. Throws for a snapshot of another story.
    pub fn restore(&mut self, snapshot: JsValue) -> Result<JsValue, JsValue> {
        let snapshot: Snapshot = from_js(snapshot)?;
        let step = self
            .session
            .restore(&snapshot)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        to_js(&step)
    }

    /// The game as a save: an envelope with the format version, the story and the state.
    pub fn save(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.save())
    }

    /// Continue from a save. Returns `{ story_changed, from_version, dropped }`.
    /// Throws when the save cannot be read.
    pub fn load(&mut self, save: JsValue) -> Result<JsValue, JsValue> {
        let (mut session, loaded) = load_save(&self.world, save)?;
        session.set_hints(self.hints.clone());
        self.session = session;
        Ok(loaded)
    }

    /// What the session waits for now.
    pub fn status(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.status())
    }

    /// The hint ladder of the current room: `[{ step, shown, command?, levels }]`,
    /// the steps not done yet whose needs are done.
    pub fn hints(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.hints())
    }

    /// Show the next hint level, of `step` or of the first step with a level
    /// left. Returns a step with the hint as a note. It is no turn.
    pub fn hint(&mut self, step: Option<String>) -> Result<JsValue, JsValue> {
        to_js(&self.session.hint(step.as_deref()))
    }

    pub fn scene(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.scene())
    }

    /// The quest journal: open quests first, then finished ones.
    pub fn journal(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.journal())
    }

    /// The combat in progress, or null.
    pub fn combat(&self) -> Result<JsValue, JsValue> {
        or_null(self.session.combat())
    }

    /// The adventure sheet: `[{ var, label, short?, value, max? }]`.
    pub fn stats(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.stats())
    }

    /// The active illustration, portrait, music and ambience.
    pub fn media(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.media())
    }

    pub fn verbs(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.verbs())
    }

    /// What a partial command refers to: `{ verb, objects, candidates, direction, complete }`.
    pub fn preview(&self, text: &str) -> Result<JsValue, JsValue> {
        to_js(&self.session.preview(text))
    }

    /// The talk in progress, or null.
    pub fn talk(&self) -> Result<JsValue, JsValue> {
        or_null(self.session.talk())
    }

    /// The answers the player can choose now: `[{ index, text }]`. Empty outside a talk.
    pub fn choices(&self) -> Result<JsValue, JsValue> {
        to_js(&self.session.choices())
    }
}

/// A running game with the authoring views of the editor (the Workbench API,
/// ADR 0003). It owns a session. Its play methods return whole turns with the
/// rule log, for the editor's transcript.
#[wasm_bindgen(js_name = Workbench)]
pub struct JsWorkbench {
    world: Arc<World>,
    origins: BTreeMap<String, String>,
    walkthroughs: Vec<(String, Walkthrough)>,
    bench: Workbench,
}

#[wasm_bindgen(js_class = Workbench)]
impl JsWorkbench {
    /// A new game of project files. Throws an array of diagnostics if the
    /// project has errors.
    #[wasm_bindgen(constructor)]
    pub fn new(files: JsValue) -> Result<JsWorkbench, JsValue> {
        let story = compile(files)?;
        let mut bench = Workbench::new(story.world.clone());
        bench.set_walkthroughs(story.walkthroughs.clone());
        Ok(JsWorkbench {
            bench,
            world: story.world,
            origins: story.origins,
            walkthroughs: story.walkthroughs,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn title(&self) -> String {
        self.bench.world().story.title.clone()
    }

    /// The file of each entity, for example `room:shore` to `rooms/shore.yaml`.
    pub fn origins(&self) -> Result<JsValue, JsValue> {
        to_js(&self.origins)
    }

    // ---- play: whole turns for the transcript ------------------------------

    /// The opening as a turn. A second call starts nothing again.
    pub fn start(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.start())
    }

    /// One input as a turn: `{ type: "text", value }`, `{ type: "command", value }`,
    /// `{ type: "choice", value }` or `{ type: "tick" }`.
    pub fn play(&mut self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: Input = from_js(input)?;
        to_js(&self.bench.play(input))
    }

    /// Answer the request of `Status::Await`, also the death prompt, as a turn.
    /// Throws when the id or the value does not fit.
    pub fn resume(&mut self, await_id: &str, value: JsValue) -> Result<JsValue, JsValue> {
        let value: Value = from_js(value)?;
        let turn = self
            .bench
            .resume(await_id, value)
            .map_err(|e| JsValue::from_str(&e))?;
        to_js(&turn)
    }

    /// Go back to the state before the last input, as a turn.
    pub fn undo(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.undo())
    }

    /// How many inputs `undo` can go back now.
    #[wasm_bindgen(js_name = undoSteps)]
    pub fn undo_steps(&self) -> usize {
        self.bench.session().undo_steps()
    }

    /// Start the story again, as the opening turn. `undo` can go back to the state before it.
    pub fn restart(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.restart())
    }

    /// An author command: `{ type: "goto", room }`, `{ type: "set", name, value }`,
    /// `{ type: "time", time }` or `{ type: "give", thing }`.
    pub fn author(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let command: AuthorCommand = from_js(command)?;
        to_js(&self.bench.author(command))
    }

    // ---- the session's queries ---------------------------------------------

    /// What the session waits for now.
    pub fn status(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().status())
    }

    pub fn scene(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().scene())
    }

    /// The quest journal of the session.
    pub fn journal(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().journal())
    }

    /// The combat in progress, or null.
    pub fn combat(&self) -> Result<JsValue, JsValue> {
        or_null(self.bench.session().combat())
    }

    /// The adventure sheet: `[{ var, label, short?, value, max? }]`.
    pub fn stats(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().stats())
    }

    /// The active illustration, portrait, music and ambience.
    pub fn media(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().media())
    }

    pub fn verbs(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().verbs())
    }

    /// What a partial command refers to: `{ verb, objects, candidates, direction, complete }`.
    pub fn preview(&self, text: &str) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().preview(text))
    }

    /// The answers the player can choose now: `[{ index, text }]`. Empty outside a talk.
    pub fn choices(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().choices())
    }

    /// The game as a save: an envelope with the format version, the story and the state.
    pub fn save(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().save())
    }

    /// Continue from a save. Returns `{ story_changed, from_version, dropped }`.
    /// Throws when the save cannot be read.
    pub fn load(&mut self, save: JsValue) -> Result<JsValue, JsValue> {
        let (session, loaded) = load_save(&self.world, save)?;
        self.bench = Workbench::from_session(session);
        self.bench.set_walkthroughs(self.walkthroughs.clone());
        Ok(loaded)
    }

    /// Continue the game of a workbench with an earlier version of the story, after an edit.
    #[wasm_bindgen(js_name = carryOver)]
    pub fn carry_over(&mut self, from: &JsWorkbench) {
        self.bench = from
            .bench
            .carried_to_with(self.world.clone(), self.walkthroughs.clone());
    }

    /// The story has hints, also when none is open in this room now.
    #[wasm_bindgen(js_name = hasHints)]
    pub fn has_hints(&self) -> bool {
        self.bench.session().has_hints()
    }

    /// The hint ladder of the current room, as `Session.hints`.
    pub fn hints(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().hints())
    }

    /// Show the next hint level, as `Session.hint`, as a turn.
    pub fn hint(&mut self, step: Option<String>) -> Result<JsValue, JsValue> {
        to_js(&self.bench.hint(step.as_deref()))
    }

    /// The dependency chart of each walkthrough with step ids (CK-7):
    /// `[{ walkthrough, name, steps: [{ id, command, index, at?, needs, depth, width, hints }], max_width }]`.
    pub fn charts(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.charts())
    }

    // ---- views -------------------------------------------------------------

    /// The talk in progress for the inspector, with hidden options, or null.
    #[wasm_bindgen(js_name = talkView)]
    pub fn talk_view(&self) -> Result<JsValue, JsValue> {
        or_null(self.bench.talk_view())
    }

    /// Every dialogue as an outline of nodes, options and their targets.
    pub fn dialogues(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.dialogues())
    }

    /// How the current room's description is assembled, for the assembly panel.
    pub fn assembly(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.assembly())
    }

    /// The things and exits of the current room, for the assembly panel.
    #[wasm_bindgen(js_name = roomParts)]
    pub fn room_parts(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.room_parts())
    }

    /// Facts about a thing (`kind` "thing", `id` its id) or an exit of the current room
    /// (`kind` "exit", `id` its direction), for an author tooltip. Null when there is none.
    pub fn inspect(&self, kind: &str, id: &str) -> Result<JsValue, JsValue> {
        or_null(match kind {
            "thing" => self.bench.inspect_thing(id),
            "exit" => self.bench.inspect_exit(id),
            _ => None,
        })
    }

    /// All variables and thing properties with their values.
    pub fn vars(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.vars())
    }

    /// The phases of the day with their start times.
    pub fn phases(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.phases())
    }

    /// The action frame used for the last action, or null.
    #[wasm_bindgen(js_name = lastFrame)]
    pub fn last_frame(&self) -> Result<JsValue, JsValue> {
        or_null(self.bench.last_frame())
    }

    /// Walk all reachable states from the start of the story and report coverage.
    pub fn explore(&self, max_states: usize, max_depth: usize) -> Result<JsValue, JsValue> {
        to_js(&self.bench.explore(Budget {
            max_states,
            max_depth,
        }))
    }

    /// The text of a room in every combination of the values that the text depends on.
    #[wasm_bindgen(js_name = roomMatrix)]
    pub fn room_matrix(&self, room: &str) -> Result<JsValue, JsValue> {
        or_null(self.bench.room_matrix(room))
    }

    /// All rooms with what is in them now, for the Atlas.
    pub fn atlas(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.atlas())
    }

    /// All rules with their state now, for the rules view.
    pub fn rules(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.rules())
    }

    /// The puzzle machines with their states and steps, for the Atlas.
    pub fn machines(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.machines())
    }

    /// All rooms and exits, for the map panel.
    pub fn map(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.map())
    }

    /// Check fragment text and render it in the current state. Error offsets are UTF-16.
    #[wasm_bindgen(js_name = tryMarkup)]
    pub fn try_markup(&self, text: &str) -> Result<JsValue, JsValue> {
        to_js(&utf16_error(text, self.bench.try_markup(text)))
    }

    /// Check fragment text and render it as it would read in other values. Error offsets are UTF-16.
    /// Null for an unknown room.
    #[wasm_bindgen(js_name = tryMarkupIn)]
    pub fn try_markup_in(
        &self,
        room: &str,
        values: JsValue,
        text: &str,
    ) -> Result<JsValue, JsValue> {
        let values: Vec<(String, String)> = from_js(values)?;
        or_null(
            self.bench
                .try_markup_in(room, &values, text)
                .map(|check| utf16_error(text, check)),
        )
    }

    /// Check a condition and tell whether it holds now. Error offsets are UTF-16.
    #[wasm_bindgen(js_name = tryCondition)]
    pub fn try_condition(&self, text: &str) -> Result<JsValue, JsValue> {
        to_js(&utf16_error(text, self.bench.try_condition(text)))
    }

    /// Every name that conditions and markup can use, with its value now.
    pub fn symbols(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.symbols())
    }

    /// One fragment text in all combinations of the values that it depends on.
    #[wasm_bindgen(js_name = textMatrix)]
    pub fn text_matrix(&self, text: &str, when: Option<String>) -> Result<JsValue, JsValue> {
        or_null(self.bench.text_matrix(text, when.as_deref()))
    }

    /// The room description in another combination of values. It does not change the game.
    #[wasm_bindgen(js_name = previewSituation)]
    pub fn preview_situation(&self, room: &str, values: JsValue) -> Result<JsValue, JsValue> {
        let values: Vec<(String, String)> = from_js(values)?;
        or_null(self.bench.preview_situation(room, &values))
    }

    /// The room description as it would show now, without changing the game.
    pub fn describe(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.describe())
    }
}
