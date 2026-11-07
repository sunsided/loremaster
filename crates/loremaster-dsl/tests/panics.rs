//! A malformed time panics at the line of the story code that wrote it, not
//! in the library. One test in its own binary, because it sets the panic hook.

use loremaster_dsl::prelude::*;
use std::sync::{Arc, Mutex};

#[test]
fn a_bad_phase_time_panics_at_the_callers_line() {
    let at = Arc::new(Mutex::new(None));
    let seen = at.clone();
    std::panic::set_hook(Box::new(move |info| {
        *seen.lock().unwrap() = info.location().map(|l| (l.file().to_string(), l.line()));
    }));
    // The line of `.phases(..)`, 4 below.
    let line = line!();
    let result = std::panic::catch_unwind(|| {
        let mut p = Pack::new("clock");
        p.story("Clock", "hall")
            .phases([("day", "07:00"), ("night", "25:00")]);
    });
    let _ = std::panic::take_hook();
    assert!(result.is_err());
    assert_eq!(
        at.lock().unwrap().clone(),
        Some((file!().to_string(), line + 4))
    );
}
