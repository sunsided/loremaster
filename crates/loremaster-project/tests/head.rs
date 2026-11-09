//! The head noun of a thing (#234): a declared `head` is a word of the name
//! or an alias.

use assert2::check;
use loremaster_project::{SourceFile, check, load};

fn diagnostics(thing: &str) -> Vec<loremaster_project::Diagnostic> {
    let files = vec![
        SourceFile {
            path: "story.yaml".into(),
            text: "title: Boots\nstart: hall\n".into(),
        },
        SourceFile {
            path: "rooms/hall.yaml".into(),
            text: "name: Hall\n".into(),
        },
        SourceFile {
            path: "things/boots.yaml".into(),
            text: thing.into(),
        },
    ];
    check(&load(files).expect("loads"))
}

#[test]
fn a_head_from_the_name_or_a_word_of_an_alias_is_fine() {
    let d = diagnostics(
        "name: pair of boots\naliases: [footwear]\nhead: Boots\nnames:\n  - { when: true, text: old shoes, head: handle, aliases: [iron handle] }\n  - { when: true, text: rags, head: \" \" }\nlocation: { room: hall }\n",
    );
    check!(d == vec![]);
}

#[test]
fn a_head_that_is_not_a_word_of_the_name_is_an_error() {
    let d = diagnostics(
        "name: pair of boots\nhead: shoes\nnames:\n  - { when: true, text: old shoes, head: boots }\nlocation: { room: hall }\n",
    );
    check!(d.len() == 2, "{d:#?}");
    check!(d[0].at == "head");
    check!(
        d[0].message
            == "the head `shoes` is not a word of \"pair of boots\" or of one of its aliases"
    );
    check!(d[1].at == "names[0].head");
}
