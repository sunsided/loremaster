//! Task prompts: the requests that the editor sends to an assistant.
//!
//! The same prompts serve the editor's buttons (through the jobs of
//! `loremaster serve`) and plain MCP clients (as MCP prompts), so a task works
//! the same from both.

use crate::{ToolError, err};
use serde_json::{Value, json};

/// One task prompt: its name, what it does, and its arguments.
#[derive(Debug, Clone)]
pub struct PromptDef {
    pub name: &'static str,
    pub description: &'static str,
    /// Argument names, with a description and whether each is required.
    pub arguments: &'static [(&'static str, &'static str, bool)],
}

pub fn prompts() -> Vec<PromptDef> {
    vec![
        PromptDef {
            name: "imagine_exit",
            description: "The player tried to go in a direction that has no exit. Suggest what is there, as checked proposals of different kinds.",
            arguments: &[
                ("room", "The room id.", true),
                (
                    "direction",
                    "The direction the player tried, for example \"down\".",
                    true,
                ),
                (
                    "session",
                    "The play session that holds the player's state. Default \"main\".",
                    false,
                ),
                (
                    "rejected",
                    "Ideas the author rejected before, one per line.",
                    false,
                ),
            ],
        },
        PromptDef {
            name: "seed_story",
            description: "Expand the author's idea for a new story into 2 or 3 connected rooms, as one checked proposal with author notes and threads.",
            arguments: &[
                ("idea", "The author's idea for the story, free text.", true),
                ("rooms", "How many rooms: 2 or 3. Default 3.", false),
                (
                    "rejected",
                    "Ideas the author rejected before, one per line.",
                    false,
                ),
            ],
        },
        PromptDef {
            name: "review_room",
            description: "Review one room in all its states and in its surroundings. Report findings and propose the clear fixes.",
            arguments: &[
                ("room", "The room id.", true),
                (
                    "session",
                    "The play session that holds the player's state. Default \"main\".",
                    false,
                ),
            ],
        },
    ]
}

/// The tool calls that give the context of a task prompt: a title and a call
/// for each. A server that runs the task itself can make these calls first and
/// give the results with [`render_prompt_with_context`], which saves the
/// assistant several turns.
pub fn prompt_context(name: &str, args: &Value) -> Vec<(&'static str, &'static str, Value)> {
    if name == "seed_story" {
        return vec![
            (
                "The story file",
                "read_file",
                json!({ "path": "story.yaml" }),
            ),
            (
                "The start room",
                "read_file",
                json!({ "path": "rooms/start.yaml" }),
            ),
        ];
    }
    let room = args.get("room").cloned().unwrap_or(Value::Null);
    // The same session as the prompt text: an empty one is the default.
    let session = args
        .get("session")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(crate::DEFAULT_SESSION);
    let mut calls = vec![
        (
            "The story file",
            "read_file",
            json!({ "path": "story.yaml" }),
        ),
        (
            "The room file",
            "get_entity",
            json!({ "kind": "room", "id": room }),
        ),
        (
            "The neighbours, 1 hop",
            "neighbors",
            json!({ "room": room, "hops": 1 }),
        ),
        (
            "The scene of the session",
            "scene",
            json!({ "session": session }),
        ),
    ];
    match name {
        "imagine_exit" => calls.push((
            "The variables of the session",
            "vars",
            json!({ "session": session }),
        )),
        "review_room" => calls.push((
            "The room in all its states",
            "room_matrix",
            json!({ "room": room }),
        )),
        _ => calls.clear(),
    }
    calls
}

/// The text of a task prompt with its arguments filled in.
pub fn render_prompt(name: &str, args: &Value) -> Result<String, ToolError> {
    render_prompt_with_context(name, args, None)
}

/// The text of a task prompt, with the results of [`prompt_context`] and the
/// writing guide at the end. Then the assistant does not fetch them again.
pub fn render_prompt_with_context(
    name: &str,
    args: &Value,
    context: Option<&str>,
) -> Result<String, ToolError> {
    let get = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let need =
        |key: &str| get(key).ok_or_else(|| err(format!("The prompt `{name}` needs `{key}`.")));
    let session = get("session").unwrap_or(crate::DEFAULT_SESSION);
    match name {
        "imagine_exit" => {
            let (room, dir) = (need("room")?, need("direction")?);
            let look = match context {
                Some(_) => GIVEN.to_string(),
                None => format!(
                    "Look at the context: `get_entity` for the room, `neighbors` with 1 hop, and `scene` of the session `{session}`. Read the guide resource loremaster://guide/writing-stories if you have not."
                ),
            };
            let rejected = match get("rejected") {
                Some(r) => format!(
                    "\nThe author rejected these ideas before. Do not offer them again:\n{r}\n"
                ),
                None => String::new(),
            };
            Ok(format!(
                "In the room `{room}`, the player tried to go {dir}. There is no exit {dir}. Imagine what is there.\n\
                 \n\
                 Work fast: the author waits for the reply. Use few tool calls and do not test how the engine works.\n\
                 \n\
                 1. {look}\n\
                 2. Make 3 proposals with `propose`, each of a different kind. Kinds to choose from: a new room with an exit pair and a map position; an exit to a room that exists; an exit that is hidden (`visible`) or blocked (`passable` and `blocked`); a message exit, when the way should lead nowhere: `{{ direction: {dir}, blocked: \"<text>\" }}` with no `to`, no `door`, no `passable` and no `traversal`. The move only shows the `blocked` text, which can have conditions.\n\
                 3. Keep each proposal small and consistent with the tone, the period and the neighbours. A new way must not go around a locked door or another obstacle. {edits}\n\
                 4. A proposal must have `ok: true`. If it has problems, make a fixed one and withdraw the old one. Do not play the proposals: the author does that.\n\
                 5. Do not change the session `{session}`.\n\
                 {rejected}\n\
                 {terse}Reply with one line for each proposal: its id, then a pitch of at most 15 words.{context}",
                edits = EDITS,
                terse = TERSE,
                context = with_context(context),
            ))
        }
        "seed_story" => {
            let idea = need("idea")?;
            let rooms = match get("rooms") {
                Some("2") => 2,
                _ => 3,
            };
            let look = match context {
                Some(_) => GIVEN.to_string(),
                None => "Read `story.yaml` and `rooms/start.yaml` with `read_file`, and the guide resource loremaster://guide/writing-stories.".to_string(),
            };
            let rejected = match get("rejected") {
                Some(r) => format!(
                    "\nThe author rejected these drafts before. Do not offer them again:\n{r}\n"
                ),
                None => String::new(),
            };
            Ok(format!(
                "The author starts a new story. The project is the empty start: `story.yaml` and one room `rooms/start.yaml` with a placeholder text. Expand this idea into the first {rooms} rooms of the story:\n\
                 \n\
                 > {idea}\n\
                 \n\
                 Work fast: the author waits for the reply. Use few tool calls and do not test how the engine works.\n\
                 \n\
                 1. {look}\n\
                 2. Make one proposal with `propose`. It has:\n\
                 - `story.yaml`: keep the title when it is not \"Untitled\"; else give a title. Write an `intro` of 1 or 2 sentences. Add at most 2 variables, only if the rooms use them.\n\
                 - `rooms/start.yaml`: replace the placeholder with the first room of the story. Keep the file name and `start: start`, and give the room a real `name`.\n\
                 - {more} more rooms in `rooms/`. Connect all rooms with exits in both directions, and give each room a map `position` ([x, y], neighbours one step apart). One exit can be locked, hidden or conditional, so that the player has a first small goal.\n\
                 - At most 2 things in `things/`. The room text names each one with markup, for example `{{brass_key.the}}`. Give such a thing `listed: false`, so that the game does not add a line \"There is … here.\" for it.\n\
                 - Author notes: the premise as a note in `story.yaml`, one `todo` in each room for a decision that the author must make, and 1 or 2 open threads in `threads/` that the idea suggests, each with a `setup` note on the fragment that plants it. Do not resolve the threads.\n\
                 3. Match the tone and the period of the idea. Keep the texts short: 2 to 4 sentences for each room.\n\
                 4. The proposal must have `ok: true`. If it has problems, make a fixed one and withdraw the old one. Do not play it: the author does that.\n\
                 {rejected}\n\
                 {terse}Reply with the proposal id, then one line for each room: its name and a pitch of at most 15 words.{context}",
                more = rooms - 1,
                terse = TERSE,
                context = with_context(context),
            ))
        }
        "review_room" => {
            let room = need("room")?;
            let look = match context {
                Some(_) => GIVEN.to_string(),
                None => format!(
                    "Read it: `get_entity`, `neighbors` with 1 hop, and `room_matrix` for its text in all states. Also look at `scene` of the session `{session}`."
                ),
            };
            Ok(format!(
                "Review the room `{room}`.\n\
                 \n\
                 1. {look}\n\
                 2. Look for: text that reads badly in one state; states with no variant (for example only night text); repeated words across variants; exits that the text names but `exits` does not have, and the reverse; a tone that does not match the neighbours; problems that `check` or the matrix marks.\n\
                 3. For the clear fixes, make one proposal with `propose`. It must have `ok: true`. {edits}\n\
                 4. Do not change the session `{session}`.\n\
                 \n\
                 Reply with a list of findings, one line each, in this form (the editor shows them next to the text):\n\
                 `- high | room:{room} #2 | the problem | the fix`\n\
                 The first part is how serious it is: high, medium or low. The second part is the fragment: its owner and its index in the list, counting from 0 as `get_entity` lists it. Leave out the index for a finding about the whole room. Do not use `|` in the problem or the fix. Then give the id of your proposal, if you made one.{context}",
                edits = EDITS,
                context = with_context(context),
            ))
        }
        _ => Err(err(format!("There is no prompt `{name}`."))),
    }
}

/// How to give changes: edits are much shorter than whole files, and the
/// agent's time goes mostly into the text it writes.
const EDITS: &str = "For a file that exists, give `edits` with small `old` and `new` pieces, not the whole file. Give whole files only for new files.";

/// The agent writes no text that nobody reads.
const TERSE: &str = "Write no text between the tool calls. ";

/// Step 1 when the context is in the prompt.
const GIVEN: &str = "The context and the writing guide are at the end of this request. You have them, so do not fetch them again. Use other tools only when you need more.";

fn with_context(context: Option<&str>) -> String {
    context.map_or(String::new(), |c| format!("\n\n# Context\n\n{c}"))
}

/// The MCP form of the prompt list.
pub fn prompts_json() -> Value {
    json!(
        prompts()
            .iter()
            .map(|p| json!({
                "name": p.name,
                "description": p.description,
                "arguments": p.arguments.iter().map(|(name, description, required)| json!({
                    "name": name, "description": description, "required": required,
                })).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imagine_exit_names_the_room_and_the_direction() {
        let text = render_prompt(
            "imagine_exit",
            &json!({ "room": "shore", "direction": "down", "rejected": "a puddle" }),
        )
        .unwrap();
        assert!(text.contains("`shore`"));
        assert!(text.contains("go down"));
        assert!(text.contains("{ direction: down, blocked:"));
        assert!(text.contains("a puddle"));
        assert!(text.contains("session `main`"));
    }

    #[test]
    fn with_context_the_prompt_says_not_to_fetch_it() {
        let args = json!({ "room": "shore", "direction": "down" });
        let text = render_prompt_with_context("imagine_exit", &args, Some("ROOM FILE")).unwrap();
        assert!(text.contains("do not fetch them again"));
        assert!(text.ends_with("ROOM FILE"));
        assert!(!text.contains("`neighbors` with 1 hop"));
        let tools: Vec<&str> = prompt_context("review_room", &args)
            .iter()
            .map(|c| c.1)
            .collect();
        assert!(tools.contains(&"room_matrix"));
        assert!(prompt_context("nope", &args).is_empty());
        let blank = json!({ "room": "shore", "session": "  " });
        let scene = prompt_context("imagine_exit", &blank);
        assert!(
            scene
                .iter()
                .any(|c| c.1 == "scene" && c.2["session"] == crate::DEFAULT_SESSION)
        );
    }

    #[test]
    fn review_room_asks_for_one_finding_per_line_flush_left() {
        let text = render_prompt("review_room", &json!({ "room": "shore" })).unwrap();
        assert!(
            text.lines()
                .any(|l| l == "`- high | room:shore #2 | the problem | the fix`"),
            "{text}"
        );
        assert!(text.lines().all(|l| !l.starts_with(' ')), "{text}");
    }

    #[test]
    fn seed_story_has_the_idea_and_the_room_count() {
        let args = json!({ "idea": "A drowned bell rings at low tide.", "rooms": "2" });
        let text = render_prompt("seed_story", &args).unwrap();
        assert!(text.contains("> A drowned bell rings at low tide."));
        assert!(text.contains("first 2 rooms"));
        assert!(text.contains("- 1 more rooms"));
        assert!(text.contains("threads/"));
        let calls = prompt_context("seed_story", &args);
        assert!(calls.iter().all(|c| c.1 == "read_file"));
        assert!(render_prompt("seed_story", &json!({})).is_err());
    }

    #[test]
    fn a_missing_argument_is_an_error() {
        assert!(render_prompt("imagine_exit", &json!({ "room": "shore" })).is_err());
        assert!(render_prompt("nope", &json!({})).is_err());
    }
}
