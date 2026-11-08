//! The asset checks (ME-3).

use loremaster_project::{Severity, SourceFile, check_files, load, missing_asset_files};

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

fn files(extra: Vec<SourceFile>) -> Vec<SourceFile> {
    let mut all = vec![
        file("story.yaml", "title: T\nstart: a\n"),
        file(
            "rooms/a.yaml",
            "name: A\nmedia: { illustration: { asset: pic, when: 'nope' }, music: gone }\n",
        ),
        file(
            "rules/r.yaml",
            "- { id: r, on: start, then: [{ play_sound: boom }] }\n",
        ),
        file("assets/pic.yaml", "file: assets/pic.png\n"),
        file("assets/boom.yaml", "file: ../boom.wav\n"),
        file(
            "assets/idle.yaml",
            "file: assets/idle.wav\ncaption: hm\nlicence: { name: CC0-1.0, author: me }\n",
        ),
    ];
    all.extend(extra);
    all
}

fn found(severity: Severity, needle: &str, key: &str) -> bool {
    check_files(files(vec![]))
        .diagnostics
        .iter()
        .any(|d| d.severity == severity && d.message.contains(needle) && d.at.contains(key))
}

#[test]
fn the_checks_report_each_problem() {
    assert!(found(Severity::Error, "unknown asset `gone`", "room:a"));
    assert!(found(Severity::Error, "no path inside", "asset:boom"));
    assert!(found(Severity::Error, "nope", "room:a/media/illustration"));
    assert!(found(Severity::Warning, "no `licence`", "asset:pic"));
    assert!(found(Severity::Warning, "no `caption`", "asset:pic"));
    assert!(found(Severity::Warning, "no `caption`", "asset:boom"));
    assert!(found(Severity::Warning, "no slot", "asset:idle"));
}

#[test]
fn a_decorative_image_needs_no_caption() {
    let checked = check_files(vec![
        file("story.yaml", "title: T\nstart: a\n"),
        file("rooms/a.yaml", "name: A\nmedia: { illustration: pic }\n"),
        file(
            "assets/pic.yaml",
            "file: assets/pic.png\ndecorative: true\nlicence: { name: CC0-1.0, author: me }\n",
        ),
    ]);
    assert_eq!(checked.diagnostics, vec![]);
}

#[test]
fn a_missing_file_is_reported_with_the_disk() {
    let project = load(vec![
        file("story.yaml", "title: T\nstart: a\n"),
        file("rooms/a.yaml", "name: A\nmedia: { illustration: pic }\n"),
        file("assets/pic.yaml", "file: assets/pic.png\n"),
    ])
    .unwrap();
    let dir = std::env::temp_dir().join(format!("loremaster-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let missing = missing_asset_files(&project, &dir);
    assert_eq!(missing[0].severity, Severity::Error);
    assert_eq!(missing[0].file, "assets/pic.yaml");
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("assets/pic.png"), b"x").unwrap();
    assert_eq!(missing_asset_files(&project, &dir), vec![]);
    std::fs::remove_dir_all(dir).unwrap();
}
