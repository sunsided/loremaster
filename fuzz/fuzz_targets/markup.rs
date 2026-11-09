//! Fragment text with inline markup: compile against the Lighthouse, and the
//! editor tokens.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_lang::{Scope, compile_markup, markup_tokens};

fuzz_target!(|src: &str| {
    let scope = Scope::from_story(&loremaster_fuzz::lighthouse().story);
    if let Err(e) = compile_markup(src, &scope) {
        assert!(src.get(e.span.clone()).is_some(), "{e:?}");
    }
    for t in markup_tokens(src) {
        assert!(src.get(t.span.clone()).is_some(), "{t:?}");
    }
});
