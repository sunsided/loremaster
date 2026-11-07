use loremaster_model::Story;
use sha2::{Digest, Sha256};

/// The canonical hash of a model: SHA-256 of the story as JSON, in hex.
///
/// Two sources that assemble to the same model give the same hash, whatever
/// their form (YAML, `world!`, a builder) and their file layout. The golden
/// slice keeps the hash of its YAML form in `expected/model.hash`, and
/// `slice_equivalence` compares the forms by it (DS-11 to DS-15).
pub fn model_hash(story: &Story) -> String {
    // A story is plain data with string keys (integer keys are written as
    // strings), so it always serializes.
    let json = serde_json::to_vec(story).expect("a story always serializes to JSON");
    Sha256::digest(&json)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
