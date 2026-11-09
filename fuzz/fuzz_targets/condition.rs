//! Condition text, as in a `when`: compile against the Lighthouse (which
//! parses it first), and the editor tokens.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_lang::{compile_condition, condition_tokens};

fuzz_target!(|src: &str| {
    if let Err(e) = compile_condition(src, loremaster_fuzz::lighthouse_scope()) {
        assert!(src.get(e.span.clone()).is_some(), "{e:?}");
    }
    for t in condition_tokens(src) {
        assert!(src.get(t.span.clone()).is_some(), "{t:?}");
    }
});
