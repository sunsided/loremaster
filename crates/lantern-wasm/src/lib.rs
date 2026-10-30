//! WebAssembly bindings: the web editor and player drive the same runtime as
//! the CLI through this API. Values cross the boundary as plain JS objects
//! shaped like the runtime's serde types.

use lantern_project::{Diagnostic, Severity, SourceFile, check, load};
use lantern_runtime::{AuthorCommand, Budget, Command, Game, State, World};
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

fn utf16_error(
    text: &str,
    mut check: lantern_runtime::MarkupCheck,
) -> lantern_runtime::MarkupCheck {
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
    let diags = match load(files) {
        Ok(project) => check(&project),
        Err(diags) => diags,
    };
    to_js(&diags)
}

/// A running game.
#[wasm_bindgen]
pub struct Engine {
    world: Arc<World>,
    game: Game,
    origins: std::collections::BTreeMap<String, String>,
}

#[wasm_bindgen]
impl Engine {
    /// Build a game from project files. Throws an array of diagnostics if the
    /// project has errors.
    #[wasm_bindgen(constructor)]
    pub fn new(files: JsValue) -> Result<Engine, JsValue> {
        let files: Vec<SourceFile> = from_js(files)?;
        let project = load(files).map_err(|d| to_js(&d).unwrap_or(JsValue::NULL))?;
        let diags = check(&project);
        if diags.iter().any(|d| d.severity == Severity::Error) {
            return Err(to_js(&diags)?);
        }
        let world = World::compile(project.story).map_err(|e| {
            to_js(&vec![Diagnostic {
                severity: Severity::Error,
                file: String::new(),
                at: e.at.clone(),
                message: e.error.message.clone(),
                line: None,
                column: None,
            }])
            .unwrap_or(JsValue::NULL)
        })?;
        let world = Arc::new(world);
        Ok(Engine {
            game: Game::new(world.clone()),
            world,
            origins: project.origins,
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
        to_js(&self.game.start())
    }

    /// What a partial command refers to: `{ verb, objects, candidates, direction, complete }`.
    pub fn preview(&self, text: &str) -> Result<JsValue, JsValue> {
        to_js(&self.game.preview(text))
    }

    /// A typed command.
    pub fn input(&mut self, text: &str) -> Result<JsValue, JsValue> {
        to_js(&self.game.input(text))
    }

    /// A command from the verb bar: `{ type: "verb", verb, objects }` or
    /// `{ type: "go", direction }`.
    pub fn act(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let command: Command = from_js(command)?;
        to_js(&self.game.act(command))
    }

    pub fn scene(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.scene())
    }

    /// How the current room's description is assembled, for the assembly panel.
    pub fn assembly(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.assembly())
    }

    /// The things and exits of the current room, for the assembly panel.
    #[wasm_bindgen(js_name = roomParts)]
    pub fn room_parts(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.room_parts())
    }

    /// An author command: `{ type: "goto", room }`, `{ type: "set", name, value }`,
    /// `{ type: "time", time }` or `{ type: "give", thing }`.
    pub fn author(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let command: AuthorCommand = from_js(command)?;
        to_js(&self.game.author(command))
    }

    /// Facts about a thing (`kind` "thing", `id` its id) or an exit of the current room
    /// (`kind` "exit", `id` its direction), for an author tooltip. Null when there is none.
    pub fn inspect(&self, kind: &str, id: &str) -> Result<JsValue, JsValue> {
        let found = match kind {
            "thing" => self.game.inspect_thing(id),
            "exit" => self.game.inspect_exit(id),
            _ => None,
        };
        match found {
            Some(i) => to_js(&i),
            None => Ok(JsValue::NULL),
        }
    }

    /// All variables and thing properties with their values.
    pub fn vars(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.vars())
    }

    /// The phases of the day with their start times.
    pub fn phases(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.phases())
    }

    /// The action frame used for the last action, or null.
    #[wasm_bindgen(js_name = lastFrame)]
    pub fn last_frame(&self) -> Result<JsValue, JsValue> {
        match self.game.last_frame() {
            Some(f) => to_js(f),
            None => Ok(JsValue::NULL),
        }
    }

    /// Walk all reachable states from the start of the story and report coverage.
    pub fn explore(&self, max_states: usize, max_depth: usize) -> Result<JsValue, JsValue> {
        let fresh = Game::new(self.world.clone());
        to_js(&lantern_runtime::explore(
            &fresh,
            Budget {
                max_states,
                max_depth,
            },
        ))
    }

    /// The text of a room in every combination of the values that the text depends on.
    #[wasm_bindgen(js_name = roomMatrix)]
    pub fn room_matrix(&self, room: &str) -> Result<JsValue, JsValue> {
        match lantern_runtime::room_matrix(&self.game, room) {
            Some(m) => to_js(&m),
            None => Ok(JsValue::NULL),
        }
    }

    /// All rooms with what is in them now, for the Atlas.
    pub fn atlas(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.atlas())
    }

    /// All rules with their state now, for the rules view.
    pub fn rules(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.rules())
    }

    /// All rooms and exits, for the map panel.
    pub fn map(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.map())
    }

    pub fn verbs(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.verbs())
    }

    /// Continue the game of an engine with an earlier version of the story, after an edit.
    #[wasm_bindgen(js_name = carryOver)]
    pub fn carry_over(&mut self, from: &Engine) {
        self.game = from.game.carry_over(self.world.clone());
    }

    /// Highlighted spans of fragment text (or of a condition), in UTF-16 offsets.
    pub fn tokens(&self, text: &str, condition: bool) -> Result<JsValue, JsValue> {
        let tokens = if condition {
            lantern_lang::condition_tokens(text)
        } else {
            lantern_lang::markup_tokens(text)
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
        to_js(&utf16_error(text, self.game.try_markup(text)))
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
        match self.game.try_markup_situation(room, &values, text) {
            Some(check) => to_js(&utf16_error(text, check)),
            None => Ok(JsValue::NULL),
        }
    }

    /// Check a condition and tell whether it holds now. Error offsets are UTF-16.
    #[wasm_bindgen(js_name = tryCondition)]
    pub fn try_condition(&self, text: &str) -> Result<JsValue, JsValue> {
        to_js(&utf16_error(text, self.game.try_condition(text)))
    }

    /// Every name that conditions and markup can use, with its value now.
    pub fn symbols(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.symbols())
    }

    /// One fragment text in all combinations of the values that it depends on.
    #[wasm_bindgen(js_name = textMatrix)]
    pub fn text_matrix(&self, text: &str, when: Option<String>) -> Result<JsValue, JsValue> {
        match lantern_runtime::text_matrix(&self.game, text, when.as_deref()) {
            Some(m) => to_js(&m),
            None => Ok(JsValue::NULL),
        }
    }

    /// The room description in another combination of values. It does not change the game.
    #[wasm_bindgen(js_name = previewSituation)]
    pub fn preview_situation(&self, room: &str, values: JsValue) -> Result<JsValue, JsValue> {
        let values: Vec<(String, String)> = from_js(values)?;
        match self.game.preview_situation(room, &values) {
            Some(turn) => to_js(&turn),
            None => Ok(JsValue::NULL),
        }
    }

    /// The room description as it would show now, without changing the game.
    pub fn describe(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.describe_preview())
    }

    pub fn save(&self) -> Result<JsValue, JsValue> {
        to_js(self.game.state())
    }

    pub fn restore(&mut self, state: JsValue) -> Result<(), JsValue> {
        let state: State = from_js(state)?;
        self.game = Game::restore(self.world.clone(), state);
        Ok(())
    }

    /// Start over from the beginning.
    pub fn reset(&mut self) {
        self.game = Game::new(self.world.clone());
    }
}
