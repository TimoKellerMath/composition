//! The exports for JavaScript. A request goes into the input buffer, the answer comes from the
//! output buffer; both live in the module's memory, which JavaScript reads and writes between
//! the calls:
//!
//! ```js
//! const bytes = new TextEncoder().encode(JSON.stringify(request));
//! const at = exports.fux_input(bytes.length);
//! new Uint8Array(exports.memory.buffer, at, bytes.length).set(bytes);
//! const length = exports.fux_run();
//! const answer = new Uint8Array(exports.memory.buffer, exports.fux_output(), length);
//! ```
#![allow(unsafe_code)]

use std::cell::RefCell;

thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Makes room for a request of `length` bytes; returns where to write it.
#[unsafe(no_mangle)]
pub extern "C" fn fux_input(length: usize) -> *mut u8 {
    INPUT.with(|input| {
        let mut input = input.borrow_mut();
        *input = vec![0; length];
        input.as_mut_ptr()
    })
}

/// Answers the request in the input buffer; returns the length of the answer.
#[unsafe(no_mangle)]
pub extern "C" fn fux_run() -> usize {
    let request = INPUT.with(|input| String::from_utf8_lossy(&input.borrow()).into_owned());
    let answer = crate::answer_json(&request);
    OUTPUT.with(|output| {
        let mut output = output.borrow_mut();
        *output = answer.into_bytes();
        output.len()
    })
}

/// Where the answer lies.
#[unsafe(no_mangle)]
pub extern "C" fn fux_output() -> *const u8 {
    OUTPUT.with(|output| output.borrow().as_ptr())
}
