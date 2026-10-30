use loremaster_project::{SourceFile, load};
use loremaster_runtime::{Budget, Game, World, explore};
use std::sync::Arc;

fn game(files: &[(&str, &str)]) -> Game {
    let files = files
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).into(),
            text: (*text).into(),
        })
        .collect();
    let project = load(files).unwrap_or_else(|e| panic!("{e:#?}"));
    Game::new(Arc::new(World::compile(project.story).unwrap()))
}

/// A hall with a cellar to the north and an attic to the east. The cellar has the payoff,
/// the attic the setup: going north first shows the payoff before the setup.
const HALL: &str = "name: Hall\ndescription:\n  - \"A hall.\"\nexits:\n  - { direction: north, to: cellar }\n  - { direction: east, to: attic }\n";
const CELLAR: &str = "name: Cellar\ndescription:\n  - text: \"The key fits.\"\n    notes:\n      - { text: payoff, thread: key, role: payoff }\nexits:\n  - { direction: south, to: hall }\n";
const ATTIC: &str = "name: Attic\ndescription:\n  - text: \"A key on a nail.\"\n    notes:\n      - { text: setup, thread: key, role: setup }\nexits:\n  - { direction: west, to: hall }\n";

#[test]
fn a_payoff_before_its_setup_is_found() {
    let g = game(&[
        ("story.yaml", "title: T\nstart: hall\n"),
        ("rooms/hall.yaml", HALL),
        ("rooms/cellar.yaml", CELLAR),
        ("rooms/attic.yaml", ATTIC),
        ("threads/key.yaml", "name: The key\n"),
    ]);
    let found = explore(&g, Budget::default());
    let key = found.threads.iter().find(|t| t.thread == "key").unwrap();
    assert!(key.paid_off);
    assert_eq!(key.payoff_before_setup, ["go north"], "{key:?}");
    assert_eq!(key.payoffs, ["room:cellar#1"]);
}

#[test]
fn a_payoff_that_never_shows_is_found() {
    // No way into the cellar: its payoff never shows.
    let hall =
        "name: Hall\ndescription:\n  - \"A hall.\"\nexits:\n  - { direction: east, to: attic }\n";
    let g = game(&[
        ("story.yaml", "title: T\nstart: hall\n"),
        ("rooms/hall.yaml", hall),
        ("rooms/cellar.yaml", CELLAR),
        ("rooms/attic.yaml", ATTIC),
        ("threads/key.yaml", "name: The key\n"),
    ]);
    let found = explore(&g, Budget::default());
    let key = found.threads.iter().find(|t| t.thread == "key").unwrap();
    assert!(!key.paid_off, "{key:?}");
    assert!(key.payoff_before_setup.is_empty());
}

#[test]
fn a_dropped_thread_is_not_checked() {
    let g = game(&[
        ("story.yaml", "title: T\nstart: hall\n"),
        ("rooms/hall.yaml", HALL),
        ("rooms/cellar.yaml", CELLAR),
        ("rooms/attic.yaml", ATTIC),
        ("threads/key.yaml", "name: The key\nstatus: dropped\n"),
    ]);
    assert!(explore(&g, Budget::default()).threads.is_empty());
}
