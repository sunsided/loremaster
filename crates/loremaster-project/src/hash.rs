use loremaster_model::Story;
use sha2::{Digest, Sha256};

/// The canonical hash of a model: SHA-256 of the canonical story as JSON, in
/// hex.
///
/// Two sources that assemble to the same model give the same hash, whatever
/// their form (YAML, `world!`, a builder) and however their entities are
/// spread over files. To get there, the lists whose order means nothing
/// (rooms, things, endings, timers, threads, dialogues, machines, variables,
/// relations) are sorted by id first. The lists whose order the runtime uses
/// keep it: rules (file order breaks ties between rules of one priority),
/// verbs (the order of their words for the parser) and behaviors (the order
/// in which they run).
///
/// The golden slice keeps the hash of its YAML form in `expected/model.hash`,
/// and `slice_equivalence` compares the forms by it (DS-11 to DS-15).
pub fn model_hash(story: &Story) -> String {
    let story = canonical(story);
    // A story is plain data with string keys (integer keys are written as
    // strings), so it always serializes.
    let json = serde_json::to_vec(&story).expect("a story always serializes to JSON");
    Sha256::digest(&json)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn canonical(story: &Story) -> Story {
    let mut s = story.clone();
    s.rooms.sort_by(|a, b| a.id.cmp(&b.id));
    s.things.sort_by(|a, b| a.id.cmp(&b.id));
    s.endings.sort_by(|a, b| a.id.cmp(&b.id));
    s.timers.sort_by(|a, b| a.id.cmp(&b.id));
    s.threads.sort_by(|a, b| a.id.cmp(&b.id));
    s.dialogues.sort_by(|a, b| a.id.cmp(&b.id));
    s.machines.sort_by(|a, b| a.id.cmp(&b.id));
    s.vars.sort_by(|a, b| a.id.cmp(&b.id));
    s.relations.sort();
    s
}
