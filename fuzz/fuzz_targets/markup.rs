//! Fragment text with inline markup: compile against the Lighthouse, and the
//! editor tokens.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_lang::{compile_markup, markup_tokens};

fuzz_target!(|src: &str| {
    if let Err(e) = compile_markup(src, loremaster_fuzz::lighthouse_scope()) {
        assert!(src.get(e.span.clone()).is_some(), "{e:?}");
    }
    for t in markup_tokens(src) {
        assert!(src.get(t.span.clone()).is_some(), "{t:?}");
    }
});
