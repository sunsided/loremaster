//! Protocol mode (`loremaster protocol`, JSON-RPC on stdio): each line of
//! the input is one message to a server of the Salt Tower. The server is
//! opened once; each input starts with `session/start`, a new session.
#![no_main]

use libfuzzer_sys::fuzz_target;
use loremaster_mcp::protocol::Protocol;
use std::cell::RefCell;

thread_local! {
    static SERVER: RefCell<Protocol> = RefCell::new(
        Protocol::open(&loremaster_fuzz::example_dir("salt-tower")).unwrap(),
    );
}

const START: &str = r#"{"jsonrpc":"2.0","id":0,"method":"session/start","params":{}}"#;

fuzz_target!(|text: &str| {
    SERVER.with_borrow_mut(|server| {
        server.handle_line(START).unwrap();
        for line in text.lines().take(32) {
            server.handle_line(line);
        }
    });
});
