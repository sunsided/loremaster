//! Emphasis as a run attribute, and the story bundle (#194).

use loremaster_project::{SourceFile, load, read_dir};
use loremaster_runtime::{BUNDLE_VERSION, Bundle, Effect, SaveError, Session, Step, World};
use std::path::Path;
use std::sync::Arc;

fn harbour() -> Arc<World> {
    let files = [
        ("story.yaml", "title: Harbour\nstart: quay\n"),
        (
            "rooms/quay.yaml",
            "name: Quay\ndescription: [\"The *Kestrel* is moored by the *old crane*. 2 * 3 * 4 is 24, *quite*.\"]\n",
        ),
        (
            "things/kestrel.yaml",
            "name: Kestrel\nproper: true\nlocation: { room: quay }\nlisted: false\n",
        ),
    ]
    .map(|(path, text)| SourceFile {
        path: path.into(),
        text: text.into(),
    });
    let project = load(files.into()).unwrap_or_else(|e| panic!("{e:#?}"));
    Arc::new(World::compile(project.story).unwrap())
}

fn prose(step: &Step) -> &loremaster_runtime::Block {
    (step.effects.iter())
        .find_map(|e| match e {
            Effect::Text { block } if block.plain_text().contains("moored") => Some(block),
            _ => None,
        })
        .expect("the room prose")
}

#[test]
fn emphasis_is_a_run_attribute_without_stars() {
    let mut s = Session::new(harbour());
    let step = s.start();
    let block = prose(&step);
    let emphasized: Vec<(&str, bool)> = (block.runs.iter())
        .filter(|r| r.emphasis)
        .map(|r| (r.text.as_str(), r.link.is_some()))
        .collect();
    // The link inside the emphasis keeps it.
    assert_eq!(
        emphasized,
        [("Kestrel", true), ("old crane", false), ("quite", false)]
    );
    assert!(block.runs.iter().all(|r| !r.text.contains("*K")));
    // A lone star stays text, and the plain text keeps the markup.
    assert_eq!(
        block.plain_text(),
        "The *Kestrel* is moored by the *old crane*. 2 * 3 * 4 is 24, *quite*."
    );
    let json = serde_json::to_value(block).unwrap();
    assert_eq!(json["runs"][1]["emphasis"], true);
    assert!(json["runs"][0].get("emphasis").is_none());
}

#[test]
fn a_bundle_round_trips_and_starts_a_session_with_hints() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/lighthouse");
    let project = load(read_dir(&dir).unwrap()).unwrap();
    let bundle = Bundle::new(project.story, project.walkthroughs);
    let json = bundle.to_json();
    let back = Bundle::from_json(&json).unwrap();
    assert_eq!(back, bundle);
    assert_eq!(back.version, BUNDLE_VERSION);
    let mut session = back.session().unwrap();
    assert!(session.has_hints());
    assert!(session.start().plain_text().contains("ferry"));
}

#[test]
fn a_bundle_of_another_format_or_a_newer_version_is_refused() {
    assert!(matches!(
        Bundle::from_json(r#"{ "format": "loremaster-save", "version": 1 }"#),
        Err(SaveError::UnknownFormat(_))
    ));
    let newer = format!(
        r#"{{ "format": "loremaster-bundle", "version": {} }}"#,
        BUNDLE_VERSION + 1
    );
    assert!(matches!(
        Bundle::from_json(&newer),
        Err(SaveError::NewerVersion { .. })
    ));
}
