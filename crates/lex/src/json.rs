// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one JSON reader, value and writer (RFC 8259).
//!
//! Every PurRDF component that reads or writes a JSON document does it here:
//! the reader is the pull [`Reader`], every other reading entry point
//! ([`read`], [`Reader::read_value`], [`Reader::skip_value`],
//! [`occurrences`]) is a loop over its events, and every writer
//! ([`write_compact`], [`write_pretty`], [`write`](fn@write)) is one loop over a
//! [`Value`]. A JSON grammar decision — which bytes are whitespace, where a
//! string ends, what a number may spell — is therefore made once, and every
//! reader in the workspace accepts exactly the same documents.
//!
//! # Contract
//!
//! * **Numbers keep their lexeme.** A [`Number`] is the exact text the document
//!   wrote, validated against the RFC 8259 §6 grammar and never rounded on the
//!   way in; [`Number::decimal`] is its exact value. [`Number::as_u64`], [`Number::as_i64`], [`Number::as_i128`] and
//!   [`Number::as_f64`] decide what it denotes when a caller asks; the writer
//!   emits the lexeme character for character.
//! * **Equality is by value.** `==` and `Hash` on a [`Value`] compare numbers by
//!   the exact decimal they denote (`1 == 1.0 == 1e0`) and objects without
//!   regard to member order (RFC 8259 §4), the definition JSON Schema's
//!   `equal` states; [`Value::same_text`] and [`Number::same_text`] are the
//!   spelling-and-order identity for a caller that needs it.
//! * **Objects keep every member, in document order.** RFC 8259 §4 permits a
//!   repeated name and says nothing about which occurrence wins, so an
//!   [`Object`] is the ordered list of members, duplicates included.
//!   [`Object::get`] states the first-match rule, [`Object::count`] and
//!   [`Object::first_duplicate`] let a caller refuse an ambiguity, and
//!   [`Limits::unique_members`] refuses one at the byte it occurs (RFC 7493
//!   §2.3, I-JSON).
//! * **Input is UTF-8 without a byte-order mark.** [`read_slice`] refuses bytes
//!   that are not UTF-8 (RFC 8259 §8.1), including UTF-16 and a leading BOM, and
//!   a `\u` escape naming an unpaired surrogate is refused; a paired escape is
//!   its character. The JSONTestSuite corpus pins each such outcome
//!   (`tests/json_test_suite.rs`).
//! * **Bounded, and bounded explicitly.** [`Limits`] caps container depth,
//!   value occurrences and decoded string length. The depth cap is always
//!   explicit: [`Limits::DEFAULT`] states 128 open containers.
//! * **No walk spends a stack frame per level.** The reader keeps its open
//!   containers on a heap stack, the writer keeps its pending values on
//!   another, and a [`Value`] drops, clones, compares and prints over work
//!   lists. A document nests as deep as the caller's [`Limits`] allow, and a
//!   deep document is refused with a typed error rather than an `abort`.
//! * **Strings are escaped and decoded by [`crate::json_escape`].** The reader
//!   scans a string's clean runs with [`crate::scan::find_first_json_string_special`]
//!   and decodes escapes with [`crate::json_escape::unescape`], which refuses an
//!   unpaired surrogate; the writer escapes with
//!   [`crate::json_escape::push_string`] in the caller's [`JsonEscapes`]
//!   spelling.
//! * **Byte offsets everywhere.** Every [`Event`], every [`Occurrence`] and
//!   every [`Error`] names the byte offset it concerns.
//! * **Deterministic output.** The writer is a pure function of the value:
//!   members in the object's order, numbers as their lexemes, strings in one
//!   spelling. [`Value::sort_keys`] reorders members by name for a caller whose
//!   bytes were pinned while its members lived in a sorted map.
//! * **One record law.** [`record`] decodes a [`Value`] into a caller's type:
//!   strict records, closed string vocabularies and the scalar and container
//!   conversions, refusing in one set of words with a JSON Pointer.
//!
//! ```rust
//! use purrdf_lex::json::{self, Object, Value};
//!
//! let value = json::read(r#"{"b": [1, 2.50], "a": "x", "b": null}"#).unwrap();
//! let object = value.as_object().unwrap();
//! assert_eq!(object.count("b"), 2);
//! assert_eq!(value["b"][1].as_number().unwrap().lexeme(), "2.50");
//! assert_eq!(json::write_compact(&value), r#"{"b":[1,2.50],"a":"x","b":null}"#);
//!
//! let built = Value::from(Object::new().with("id", 7_u64).with("tags", vec!["a", "b"]));
//! assert_eq!(json::write_pretty(&built), "{\n  \"id\": 7,\n  \"tags\": [\n    \"a\",\n    \"b\"\n  ]\n}");
//! ```
//!
//! [`JsonEscapes`]: crate::json_escape::JsonEscapes

mod error;
mod number;
mod read;
pub mod record;
mod value;
mod write;

pub use error::{Error, ErrorKind};
pub use number::{Decimal, Exponent, Number};
pub use read::{
    Event, Kind, Limits, Occurrence, Reader, Str, occurrences, read, read_slice, read_with,
};
pub use value::{Object, Value};
pub use write::{Format, write, write_compact, write_into, write_pretty};

#[cfg(test)]
mod tests;
