//! Condition text, as in a `when`: parse, compile against the Lighthouse,
//! and the editor tokens.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_lang::{Scope, compile_condition, condition_tokens, parse_expr};

fuzz_target!(|src: &str| {
    let scope = Scope::from_story(&loremaster_fuzz::lighthouse().story);
    for span in [
        parse_expr(src).err().map(|e| e.span),
        compile_condition(src, &scope).err().map(|e| e.span),
    ]
    .into_iter()
    .flatten()
    {
        assert!(src.get(span.clone()).is_some(), "{span:?}");
    }
    for t in condition_tokens(src) {
        assert!(src.get(t.span.clone()).is_some(), "{t:?}");
    }
});
