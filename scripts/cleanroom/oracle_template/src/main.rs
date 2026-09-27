// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A clean-room oracle: record what a pinned crate ANSWERS, never how it works.
//!
//! The crate being replaced is called as a black box, and its answers are written
//! out as JSON lines that the replacement's tests read as answer vectors. Nothing
//! here reads or quotes the pinned crate's source; the only contact is its public
//! API, as documented.
//!
//! # Adding a recorded call
//!
//! 1. Write one function per call, `fn case_<name>(input: &[u8]) -> String`, that
//!    calls the pinned crate on `input` and renders the answer as text.
//! 2. Add `("<name>", case_<name>)` to [`CASES`], and any inputs it needs to
//!    [`INPUTS`].
//!
//! # Output
//!
//! One line per (case, input), in `CASES` order and then `INPUTS` order:
//!
//! ```text
//! {"case":"<name>","input_hex":"<hex of the input bytes>","output":"<answer>"}
//! ```
//!
//! The template carries one case over `std` alone, so it builds and runs as copied
//! and shows the format before any pinned crate is called.

use std::fmt::Write as _;
use std::io::{self, Write};

/// A recorded call: its name and the function that makes it.
type Case = (&'static str, fn(&[u8]) -> String);

/// Every recorded call.
const CASES: &[Case] = &[("std_byte_len", case_std_byte_len)];

/// The inputs every case is called with.
const INPUTS: &[&[u8]] = &[b"", b"a", b"example.org"];

/// The byte length of the input, from `std`: the template's worked example.
fn case_std_byte_len(input: &[u8]) -> String {
    input.len().to_string()
}

/// `text` as a JSON string literal.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                // Writing to a `String` cannot fail.
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `bytes` as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing to a `String` cannot fail.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for (name, call) in CASES {
        for input in INPUTS {
            let answer = call(input);
            writeln!(
                out,
                "{{\"case\":{},\"input_hex\":\"{}\",\"output\":{}}}",
                json_string(name),
                hex(input),
                json_string(&answer)
            )?;
        }
    }
    Ok(())
}
