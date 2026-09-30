// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one YAML 1.2 emitter and reader, over the JSON data model
//! ([`crate::json::Value`]).
//!
//! PurRDF reads and writes YAML only as a spelling of JSON-shaped data — LinkML
//! schemas, OKF frontmatter, YAML-LD — so both directions work over
//! [`crate::json::Value`], and one scalar resolver decides both what a plain
//! scalar denotes when it is read and whether a string must be quoted when it
//! is written: whatever [`write`](fn@write) leaves plain, [`read`] reads back as the same
//! string.
//!
//! # The emitter
//!
//! [`write`](fn@write) is deterministic and produces the block layout `serde_yaml`
//! produces (its layout decisions follow the reference emitter it drives), with
//! these deliberate differences:
//!
//! * **Numbers are written as their lexemes.** `serde_yaml` wrote a JSON number
//!   through binary64, so `1.50` became `1.5` and `1e+20` became `1e20`; here
//!   the lexeme is written as it is, and nothing is rounded.
//! * **A string holding U+2028 or U+2029 is double-quoted**, with `\L` and
//!   `\P`. The reference emitter wrote those characters raw inside a
//!   single-quoted or literal scalar, where a YAML 1.2 reader keeps them as
//!   content and misreads the indentation that follows them.
//! * **A string a YAML 1.2 reader would take for a number is quoted even when
//!   `serde_yaml` would have read it as a string**: a float spelling beyond the
//!   binary64 range (`1e400`) is single-quoted.
//!
//! # The reader
//!
//! [`read`] accepts one YAML 1.2 document in block or flow style, with
//! comments, directives, document markers, plain, single-quoted, double-quoted,
//! literal and folded scalars, explicit (`?`) keys, and anchors and aliases
//! (expanded, and counted against [`Limits::max_nodes`]; refused when
//! [`Limits::aliases`] is off, as YAML-LD's JSON profile requires). What JSON
//! cannot hold is refused with a typed [`Error`] at a byte offset: a tag other
//! than the core schema's, a key that is not a string (a number, boolean or null
//! key is accepted, as its source text, only under [`Limits::scalar_keys`]; a
//! collection key never is), a repeated key (YAML 1.2 §3.2.1.1 requires mapping
//! keys to be unique), `.inf` and `.nan`, and a second document. A stream with
//! no document reads as `null`.
//!
//! The reader is graded against the official yaml-test-suite
//! (`crates/lex/tests/yaml_test_suite.rs`, over the vendored
//! `vectors/yaml-test-suite`): every valid case inside the subset reads as the
//! suite's JSON, every invalid case is refused, and every valid case outside it
//! is refused by one of the kinds above, never read wrongly.
//!
//! Plain scalars resolve by the core schema as `serde_yaml` applies it, except that a number
//! keeps its lexeme where the lexeme is a JSON number (`1.50` stays `1.50`) and
//! is otherwise respelt as the JSON number it denotes (`0x1F` is `31`, `+.5` is
//! `0.5`), and that a float spelling beyond the binary64 range is a number, not
//! a string. Mapping keys stay in document order. Nesting is bounded by
//! [`Limits::max_depth`], and neither direction spends a machine-stack frame
//! per nesting level.

mod read;
mod resolve;
mod write;

pub use read::{Error, ErrorKind, Limits, read, read_with};
pub use write::write;

#[cfg(test)]
mod tests;
