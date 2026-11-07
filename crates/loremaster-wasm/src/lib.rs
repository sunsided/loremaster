//! WebAssembly bindings: the web editor and player drive the same runtime as
//! the CLI through this API. Values cross the boundary as plain JS objects
//! shaped like the runtime's serde types.

use loremaster_project::{Diagnostic, SourceFile, check_files, load};
use loremaster_runtime::{Command, Input, Session, World};
use loremaster_workbench::{AuthorCommand, Budget, MarkupCheck, Workbench};
use serde::Serialize;
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

/// All threads of the project files, with their beats. Empty when the files do not load.
#[wasm_bindgen(js_name = projectThreads)]
pub fn project_threads(files: JsValue) -> Result<JsValue, JsValue> {
    let files: Vec<SourceFile> = from_js(files)?;
    match load(files) {
        Ok(project) => to_js(&loremaster_project::threads(&project)),
        Err(_) => to_js(&Vec::<loremaster_project::ThreadView>::new()),
    }
}

/// A running game with the authoring views of the editor.
#[wasm_bindgen]
pub struct Engine {
    world: Arc<World>,
    bench: Workbench,
    origins: std::collections::BTreeMap<String, String>,
}

#[wasm_bindgen]
impl Engine {
    /// Build a game from project files. Throws an array of diagnostics if the
    /// project has errors.
    #[wasm_bindgen(constructor)]
    pub fn new(files: JsValue) -> Result<Engine, JsValue> {
        let files: Vec<SourceFile> = from_js(files)?;
        let checked = check_files(files);
        let project = match checked.project {
            Some(project) if !checked.has_errors() => project,
            _ => return Err(to_js(&checked.diagnostics)?),
        };
        let world = World::compile(project.story.clone()).map_err(|e| {
            to_js(&vec![project.locate(Diagnostic::from(&e))]).unwrap_or(JsValue::NULL)
        })?;
        let world = Arc::new(world);
        Ok(Engine {
            bench: Workbench::new(world.clone()),
            world,
            // The editor opens files by path. Origins in Rust (DSL step L6) get
            // their own field when the editor shows them as read-only.
            origins: project
                .origins
                .into_iter()
                .map(|(key, origin)| (key, origin.file().to_string()))
                .collect(),
        })
    }

    /// The file of each entity, for example `room:shore` to `rooms/shore.yaml`.
    pub fn origins(&self) -> Result<JsValue, JsValue> {
        to_js(&self.origins)
    }

    pub fn title(&self) -> String {
        self.world.story.title.clone()
    }

    /// Opening text. Call once.
    pub fn start(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.start())
    }

    /// What a partial command refers to: `{ verb, objects, candidates, direction, complete }`.
    pub fn preview(&self, text: &str) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().preview(text))
    }

    /// A typed command.
    pub fn input(&mut self, text: &str) -> Result<JsValue, JsValue> {
        to_js(&self.bench.play(Input::Text(text.to_string())))
    }

    /// A command from the verb bar: `{ type: "verb", verb, objects }` or
    /// `{ type: "go", direction }`.
    pub fn act(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let command: Command = from_js(command)?;
        to_js(&self.bench.play(Input::Command(command)))
    }

    /// Choose an answer in the talk in progress, by its index in `choices`.
    pub fn choose(&mut self, index: usize) -> Result<JsValue, JsValue> {
        to_js(&self.bench.play(Input::Choice(index)))
    }

    /// Leave the talk in progress.
    pub fn leave(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.play(Input::Command(Command::Leave)))
    }

    /// The answers the player can choose now: `[{ index, text }]`. Empty outside a talk.
    pub fn choices(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().choices())
    }

    /// The talk in progress for the inspector, with hidden options, or null.
    #[wasm_bindgen(js_name = talkView)]
    pub fn talk_view(&self) -> Result<JsValue, JsValue> {
        match self.bench.talk_view() {
            Some(v) => to_js(&v),
            None => Ok(JsValue::NULL),
        }
    }

    /// Every dialogue as an outline of nodes, options and their targets.
    pub fn dialogues(&self) -> Result<JsValue, JsValue> {
        to_js(&self.world.dialogue_outlines())
    }

    pub fn scene(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().scene())
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

    /// An author command: `{ type: "goto", room }`, `{ type: "set", name, value }`,
    /// `{ type: "time", time }` or `{ type: "give", thing }`.
    pub fn author(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let command: AuthorCommand = from_js(command)?;
        to_js(&self.bench.author(command))
    }

    /// Facts about a thing (`kind` "thing", `id` its id) or an exit of the current room
    /// (`kind` "exit", `id` its direction), for an author tooltip. Null when there is none.
    pub fn inspect(&self, kind: &str, id: &str) -> Result<JsValue, JsValue> {
        let found = match kind {
            "thing" => self.bench.inspect_thing(id),
            "exit" => self.bench.inspect_exit(id),
            _ => None,
        };
        match found {
            Some(i) => to_js(&i),
            None => Ok(JsValue::NULL),
        }
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
        match self.bench.last_frame() {
            Some(f) => to_js(f),
            None => Ok(JsValue::NULL),
        }
    }

    /// Walk all reachable states from the start of the story and report coverage.
    pub fn explore(&self, max_states: usize, max_depth: usize) -> Result<JsValue, JsValue> {
        to_js(&loremaster_workbench::explore(
            self.world.clone(),
            Budget {
                max_states,
                max_depth,
            },
        ))
    }

    /// The text of a room in every combination of the values that the text depends on.
    #[wasm_bindgen(js_name = roomMatrix)]
    pub fn room_matrix(&self, room: &str) -> Result<JsValue, JsValue> {
        match self.bench.room_matrix(room) {
            Some(m) => to_js(&m),
            None => Ok(JsValue::NULL),
        }
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

    pub fn verbs(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().verbs())
    }

    /// Continue the game of an engine with an earlier version of the story, after an edit.
    #[wasm_bindgen(js_name = carryOver)]
    pub fn carry_over(&mut self, from: &Engine) {
        self.bench = from.bench.carried_to(self.world.clone());
    }

    /// Highlighted spans of fragment text (or of a condition), in UTF-16 offsets.
    pub fn tokens(&self, text: &str, condition: bool) -> Result<JsValue, JsValue> {
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
        match self.bench.try_markup_in(room, &values, text) {
            Some(check) => to_js(&utf16_error(text, check)),
            None => Ok(JsValue::NULL),
        }
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
        match self.bench.text_matrix(text, when.as_deref()) {
            Some(m) => to_js(&m),
            None => Ok(JsValue::NULL),
        }
    }

    /// The room description in another combination of values. It does not change the game.
    #[wasm_bindgen(js_name = previewSituation)]
    pub fn preview_situation(&self, room: &str, values: JsValue) -> Result<JsValue, JsValue> {
        let values: Vec<(String, String)> = from_js(values)?;
        match self.bench.preview_situation(room, &values) {
            Some(turn) => to_js(&turn),
            None => Ok(JsValue::NULL),
        }
    }

    /// The room description as it would show now, without changing the game.
    pub fn describe(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.describe())
    }

    /// The game as a save: an envelope with the format version, the story and the state.
    pub fn save(&self) -> Result<JsValue, JsValue> {
        to_js(&self.bench.session().save())
    }

    /// Restore a save, also an older version or one of an earlier version of the
    /// story. Returns `{ storyChanged, dropped }`: the ids that the story no longer has.
    pub fn restore(&mut self, save: JsValue) -> Result<JsValue, JsValue> {
        let save: serde_json::Value = from_js(save)?;
        let (session, loaded) = Session::load(self.world.clone(), save)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.bench = Workbench::from_session(session);
        to_js(&serde_json::json!({
            "storyChanged": loaded.story_changed,
            "dropped": loaded.dropped,
        }))
    }

    /// Start over from the beginning.
    pub fn reset(&mut self) {
        self.bench = Workbench::new(self.world.clone());
    }
}
