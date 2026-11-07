//! Lines in YAML (P3-15, #111): every `say` is a text or a list of segments,
//! `line:` is the same key, and a line that is one text keeps the model hash.

use assert2::{assert, check};
use loremaster_model::{Effect, Line, LineSegment, Story};
use loremaster_project::{SourceFile, load, model_hash};

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}

/// A story with a line in each place: a rule, a machine step, a timer, a
/// dialogue node and an option. `key` is `say` or `line`, `text` the value.
fn story(key: &str, text: &str) -> Story {
    let files = vec![
        file(
            "story.yaml",
            &format!(
                "title: Lines\nstart: bar\ntimers:\n  - {{ id: t, turns: 2, at: {{ 1: [{{ {key}: {text} }}] }} }}\n"
            ),
        ),
        file("rooms/bar.yaml", "name: Bar\n"),
        file("things/bb.yaml", "name: man\nlocation: { room: bar }\n"),
        file(
            "rules/r.yaml",
            &format!("- id: r\n  on: {{ verb: [wait] }}\n  then:\n    - {key}: {text}\n"),
        ),
        file(
            "machines/m.yaml",
            &format!(
                "start: a\nsteps:\n  - {{ from: a, on: {{ verb: [wait] }}, to: b, {key}: {text} }}\n"
            ),
        ),
        file(
            "dialogues/bb.yaml",
            &format!(
                "with: bb\nstart: [hi]\nnodes:\n  hi:\n    {key}: {text}\n    options:\n      - {{ {key}: {text} }}\n"
            ),
        ),
    ];
    load(files).unwrap_or_else(|e| panic!("{e:#?}")).story
}

/// The line in each place of [`story`].
fn lines(s: &Story) -> Vec<Line> {
    let say = |e: &Effect| match e {
        Effect::Say(l) => l.clone(),
        other => panic!("not a say: {other:?}"),
    };
    let rule = s.rules.iter().find(|r| r.id.as_str() == "r").unwrap();
    let node = &s.dialogues[0].nodes[0];
    vec![
        say(&rule.then[0]),
        s.machines[0].steps[0].say.clone().unwrap(),
        say(&s.timers[0].at[&1][0]),
        node.say.clone(),
        node.options[0].say.clone(),
    ]
}

#[test]
fn a_text_is_a_plain_line_everywhere() {
    let s = story("say", "\"A lamp.\"");
    for line in lines(&s) {
        check!(line == Line::plain("A lamp."));
    }
}

#[test]
fn a_list_is_segments_everywhere() {
    let s = story(
        "say",
        "[\"He nods.\", { says: \"Yes.\", by: bb }, { narrate: \"Late.\", when: \"turns > 3\" }]",
    );
    let expected = Line::new(vec![
        LineSegment::plain("He nods."),
        LineSegment::says("Yes.").by("bb"),
        LineSegment::narrate("Late.").when("turns > 3"),
    ]);
    for line in lines(&s) {
        check!(line == expected);
    }
}

#[test]
fn line_is_an_alias_of_say_everywhere() {
    let text = "[\"He nods.\", { says: \"Yes.\" }]";
    check!(lines(&story("line", text)) == lines(&story("say", text)));
    check!(model_hash(&story("line", text)) == model_hash(&story("say", text)));
}

#[test]
fn a_plain_line_keeps_the_model_hash() {
    // Before P3-15 a `say` was a string. A line of one text must write out as
    // that string, or every story's model hash changes.
    let s = story("say", "\"A lamp.\"");
    let json = serde_json::to_value(&s).unwrap();
    check!(json["rules"][0]["then"][0]["say"] == "A lamp.");
    check!(json["machines"][0]["steps"][0]["say"] == "A lamp.");
    check!(json["timers"][0]["at"]["1"][0]["say"] == "A lamp.");
    check!(json["dialogues"][0]["nodes"]["hi"]["say"] == "A lamp.");
    check!(json["dialogues"][0]["nodes"]["hi"]["options"][0]["say"] == "A lamp.");

    // The same story with the says as strings in a JSON document of the old
    // shape reads back to the same model, and so to the same hash.
    let back: Story = serde_json::from_value(json).unwrap();
    check!(model_hash(&back) == model_hash(&s));

    // Segments are part of the model: the same text as a segment list is
    // another model.
    let listed = story("say", "[{ narrate: \"A lamp.\" }]");
    check!(model_hash(&listed) != model_hash(&s));
}

#[test]
fn a_bad_line_does_not_load() {
    let load_rule = |line: &str| {
        load(vec![
            file("story.yaml", "title: L\nstart: bar\n"),
            file("rooms/bar.yaml", "name: Bar\n"),
            file(
                "rules/r.yaml",
                &format!("- id: r\n  on: {{ verb: [wait] }}\n  then:\n    - say: {line}\n"),
            ),
        ])
    };
    assert!(load_rule("[{ says: \"A.\", narrate: \"B.\" }]").is_err());
    assert!(load_rule("[{ says: \"A.\", by: bb, how: loud }]").is_err());
    assert!(load_rule("[]").is_err());
    assert!(load_rule("{ says: \"A.\" }").is_err());
    assert!(load_rule("\"Fine.\"").is_ok());
}
