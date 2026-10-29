//! WebAssembly bindings: the web editor and player drive the same runtime as
//! the CLI through this API. Values cross the boundary as plain JS objects
//! shaped like the runtime's serde types.

use lantern_project::{Diagnostic, Severity, SourceFile, check, load};
use lantern_runtime::{Command, Game, State, World};
use serde::Serialize;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsValue::from_str(&e.to_string()))
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
        })
    }

    pub fn title(&self) -> String {
        self.world.story.title.clone()
    }

    /// Opening text. Call once.
    pub fn start(&mut self) -> Result<JsValue, JsValue> {
        to_js(&self.game.start())
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

    /// All rooms and exits, for the map panel.
    pub fn map(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.map())
    }

    pub fn verbs(&self) -> Result<JsValue, JsValue> {
        to_js(&self.game.verbs())
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
