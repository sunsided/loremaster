//! Dice such as `2d6+1` and `calc` sums such as `r + skill - 2`.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_model::{Dice, parse_sum};

fuzz_target!(|src: &str| {
    if let Some(d) = Dice::parse(src) {
        assert!(d.min() <= d.max());
    }
    if let Err((_, span)) = parse_sum(src) {
        assert!(src.get(span.clone()).is_some(), "{span:?}");
    }
});
