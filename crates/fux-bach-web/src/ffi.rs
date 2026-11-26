//! The exports for JavaScript, as those of `fux-web`. A request goes into the input buffer, the
//! answer comes from the output buffer; both live in the module's memory, which JavaScript reads
//! and writes between the calls:
//!
//! ```js
//! const bytes = new TextEncoder().encode(JSON.stringify(request));
//! const at = exports.fuge_input(bytes.length);
//! new Uint8Array(exports.memory.buffer, at, bytes.length).set(bytes);
//! const length = exports.fuge_run();
//! const answer = new Uint8Array(exports.memory.buffer, exports.fuge_output(), length);
//! ```
#![allow(unsafe_code)]

use std::cell::RefCell;

thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Makes room for a request of `length` bytes; returns where to write it.
#[unsafe(no_mangle)]
pub extern "C" fn fuge_input(length: usize) -> *mut u8 {
    INPUT.with(|input| {
        let mut input = input.borrow_mut();
        *input = vec![0; length];
        input.as_mut_ptr()
    })
}

/// Answers the request in the input buffer; returns the length of the answer.
#[unsafe(no_mangle)]
pub extern "C" fn fuge_run() -> usize {
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
pub extern "C" fn fuge_output() -> *const u8 {
    OUTPUT.with(|output| output.borrow().as_ptr())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A request through the buffers, as the page sends it.
    fn ask(request: &str) -> String {
        let at = fuge_input(request.len());
        // SAFETY: `fuge_input` made room for `request.len()` bytes at `at`, and the buffer stays
        // where it is until the next `fuge_input` of this thread.
        unsafe { std::slice::from_raw_parts_mut(at, request.len()) }
            .copy_from_slice(request.as_bytes());
        let length = fuge_run();
        // SAFETY: `fuge_run` left `length` bytes at `fuge_output()`, where they stay until the
        // next `fuge_run` of this thread.
        let answer = unsafe { std::slice::from_raw_parts(fuge_output(), length) };
        String::from_utf8(answer.to_vec()).expect("an answer is UTF-8")
    }

    #[test]
    fn a_request_goes_through_the_buffers() {
        let themes = r#"{"command": "themes"}"#;
        assert_eq!(ask(themes), crate::answer_json(themes));
        // A second, shorter request leaves nothing of the first behind, and text beyond ASCII
        // arrives whole.
        let own = r#"{"command": "fugue", "subject": "c' d'", "key": "C", "title": "Übung „1“"}"#;
        assert_eq!(ask(own), crate::answer_json(own));
        assert!(ask(own).contains(r#""ok":false"#), "{}", ask(own));
        assert!(ask("").contains(r#""ok":false"#));
    }
}
