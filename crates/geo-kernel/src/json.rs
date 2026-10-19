// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GeoJSON's JSON layer: the workspace's one RFC 8259 reader, value and writer,
//! [`purrdf_lex::json`], under this crate's reading policy.
//!
//! # Numbers are lexemes
//!
//! A general-purpose JSON reader that decides a number at parse time, into `f64`
//! (or `i64` when it fits), would end this crate's exactness guarantee before any
//! geometry code ran: a GeoJSON coordinate such as `-83.42391749999999` would
//! already have been rounded to the nearest double by the time [`crate::geojson`]
//! saw it, `0.1` and `0.10000000000000000555` would have become the same value,
//! and a `wasm32-unknown-unknown` build and a native build could disagree about a
//! predicate that sits near a boundary. The crate root denies
//! `clippy::float_arithmetic` precisely so that no such path can be reintroduced.
//!
//! [`JsonValue::Number`] holds a [`purrdf_lex::json::Number`], which is the
//! **source lexeme verbatim**, grammar-checked and never rounded;
//! [`crate::exact::Rat::parse_decimal`] decides what it denotes exactly, digit by
//! digit, with integer arithmetic alone, and the writer emits it character for
//! character.
//!
//! # Objects are ordered pairs, not a map
//!
//! RFC 8259 §4 permits an object to repeat a member name and says nothing about
//! which occurrence wins. A [`purrdf_lex::json::Object`] keeps every member in
//! document order, [`purrdf_lex::json::Object::get`] states the first-match rule,
//! and [`count`] lets a consumer notice the ambiguity and refuse it.
//! [`crate::geojson`] refuses it for the members that decide a geometry,
//! counting them as it streams the literal.
//!
//! # Nesting is bounded by memory alone
//!
//! A document nests as deep as its author writes it. The reader keeps its open
//! containers on a heap stack and a [`JsonValue`] drops, clones, compares and
//! prints over work lists, so no walk spends a stack frame per level. [`parse`]
//! therefore lifts the depth cap (`usize::MAX`): a literal arrives from the
//! dataset, which is untrusted input, and a stack overflow would be an `abort` no
//! host can catch — but a fixed cap would refuse conforming data below any figure
//! that protected the stack, and with no per-level stack there is no such figure
//! to pick. Depth is bounded by memory and by nothing else, identically on every
//! host.

use purrdf_lex::json::{self, Limits};

use crate::error::GeoError;

pub use purrdf_lex::json::Value as JsonValue;

/// This crate's reading policy: every [`Limits::DEFAULT`] bound, and no cap on
/// nesting (see the module documentation). [`crate::geojson`]'s streaming
/// reader reads under it too.
pub(crate) const LIMITS: Limits = Limits::with_depth(usize::MAX);

/// Parse `text` as a single RFC 8259 JSON document.
///
/// # Errors
///
/// [`GeoError::JsonSyntax`] retaining the byte offset, what was expected there and what
/// was found, for any departure from RFC 8259: a malformed number (a leading `+`,
/// a leading zero, a bare `.`, `NaN`, `Infinity`), an unterminated string, array
/// or object, a trailing comma, an unescaped control character in a string, an
/// unpaired UTF-16 surrogate, or content after the top-level value.
pub fn parse(text: &str) -> Result<JsonValue, GeoError> {
    json::read_with(text, LIMITS).map_err(|error| refusal(text, error))
}

/// A refusal of `text` by the JSON reader, as a [`GeoError::JsonSyntax`] retaining
/// the byte offset, what was expected there and what was found.
pub(crate) fn refusal(text: &str, error: json::Error) -> GeoError {
    refusal_with_storage(text, error, None)
}

pub(crate) fn refusal_with_storage(
    text: &str,
    error: json::Error,
    storage: Option<&crate::carrier::CarrierStorage<'_>>,
) -> GeoError {
    use purrdf_lex::json::ErrorKind;
    match error.kind() {
        ErrorKind::Storage(error)
        | ErrorKind::Escape(purrdf_lex::json_escape::JsonEscapeErrorKind::Storage(error)) => {
            return GeoError::Storage(error);
        }
        _ => {}
    }
    let found = text
        .get(error.offset()..)
        .and_then(|rest| rest.chars().next());
    crate::carrier::with_storage(storage, |memory| {
        memory.try_scope(|memory| {
            memory
                .boxed(crate::JsonSyntaxError {
                    cause: error,
                    found,
                })
                .map(GeoError::JsonSyntax)
        })
    })
    .unwrap_or_else(GeoError::Storage)
}

pub(crate) struct Found(pub(crate) Option<char>);
impl core::fmt::Display for Found {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            Some(character) => write!(f, "`{character}`"),
            None => f.write_str("the end of the text"),
        }
    }
}

/// Render a value as compact JSON, deterministically: members in the object's
/// order, a number as its lexeme, and a string escaping only what RFC 8259
/// requires ([`purrdf_lex::json::write_compact`]).
#[must_use]
pub fn write(value: &JsonValue) -> String {
    json::write_compact(value)
}

/// How many members of `value` are named `name`; `0` when it is not an object.
#[must_use]
pub fn count(value: &JsonValue, name: &str) -> usize {
    value.as_object().map_or(0, |object| object.count(name))
}

/// The name of `value`'s kind, for diagnostics: `null`, `a boolean`, `a number`,
/// `a string`, `an array` or `an object`.
#[must_use]
pub const fn kind_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "a boolean",
        JsonValue::Number(_) => "a number",
        JsonValue::String(_) => "a string",
        JsonValue::Array(_) => "an array",
        JsonValue::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::{JsonValue, count, kind_name, parse, write};
    use crate::error::GeoError;
    use crate::geom::arbitrary as deep;

    fn refusal(text: &str) -> String {
        match parse(text) {
            Err(GeoError::JsonSyntax(cause)) => cause.to_string(),
            Err(other) => panic!("expected a native JSON refusal for {text:?}, got {other:?}"),
            Ok(value) => panic!("expected a refusal for {text:?}, parsed {value:?}"),
        }
    }

    #[test]
    fn native_json_causes_and_source_characters_survive_input_destruction() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        for text in ["[1,]", "[1", "é", r#""\uD800""#] {
            let input = text.to_owned();
            let original = purrdf_lex::json::read_with(&input, super::LIMITS).unwrap_err();
            let found = input
                .get(original.offset()..)
                .and_then(|rest| rest.chars().next());
            let error = parse(&input).unwrap_err();
            drop(input);
            let clone = error.clone();
            drop(error);
            let GeoError::JsonSyntax(cause) = &clone else {
                panic!("original JSON reader cause")
            };
            assert_eq!(cause.cause, original);
            assert_eq!(cause.found, found);
            let fields = Capture::read(&clone).child("cause");
            assert_eq!(fields.code, "geo-json-syntax");
            assert!(
                matches!(fields.child("cause").field("offset"), ErrorValue::ByteCount(value) if value == original.offset() as u128)
            );
            match found {
                Some(original) => assert!(
                    matches!(fields.field("found"), ErrorValue::Character(value) if value == original)
                ),
                None => assert!(matches!(fields.field("found"), ErrorValue::Absent)),
            }
            assert!(clone.is_expression_error());
            assert_eq!(
                std::error::Error::source(&clone)
                    .unwrap()
                    .source()
                    .unwrap()
                    .downcast_ref::<purrdf_lex::json::Error>(),
                Some(&original)
            );
        }
    }

    #[test]
    fn native_json_refusal_preserves_wide_limits_and_physical_failure_priority() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_lex::allocation::StorageError;
        use purrdf_lex::json::{Error, ErrorKind};
        use purrdf_testkit::error_capture::Capture;

        let error = super::refusal(
            "",
            Error::new(ErrorKind::Values { limit: u64::MAX }, usize::MAX),
        );
        let fields = Capture::read(&error).child("cause").child("cause");
        assert!(
            matches!(fields.field("offset"), ErrorValue::ByteCount(value) if value == usize::MAX as u128)
        );
        assert!(
            matches!(fields.child("kind").field("limit"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        let error = super::refusal(
            "",
            Error::new(ErrorKind::Storage(StorageError::AdmissionFailed), 0),
        );
        assert_eq!(error, GeoError::Storage(StorageError::AdmissionFailed));
        let error = super::refusal(
            "",
            Error::new(
                ErrorKind::Escape(purrdf_lex::json_escape::JsonEscapeErrorKind::Storage(
                    StorageError::SizeOverflow,
                )),
                0,
            ),
        );
        assert_eq!(error, GeoError::Storage(StorageError::SizeOverflow));

        let mut resident = purrdf_lex::allocation::Resident;
        let storage = crate::carrier::CarrierStorage::resume(&mut resident, usize::MAX);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let error = super::refusal_with_storage(
            "",
            Error::new(ErrorKind::Storage(StorageError::AdmissionFailed), 0),
            Some(&storage),
        );
        let allocations = window.close();
        assert_eq!(error, GeoError::Storage(StorageError::AdmissionFailed));
        assert_eq!(storage.admitted_bytes(), usize::MAX);
        assert_eq!(allocations.allocations, 0);
    }

    #[test]
    fn a_refusal_names_the_byte_offset_what_was_expected_and_what_was_found() {
        assert_eq!(
            refusal("[1,]"),
            "JSON byte 3: expected a JSON value, found `]`"
        );
        assert_eq!(
            refusal("[1"),
            "JSON byte 2: expected `,` or `]` in an array, found the end of the text"
        );
        assert!(parse("[1]").is_ok());
    }

    #[test]
    fn a_number_is_kept_as_its_source_lexeme_verbatim() {
        let value = parse("[-83.42391749999999, 0.10000000000000000555, 1E+2]").expect("JSON");
        let lexemes: Vec<&str> = value
            .as_array()
            .expect("an array")
            .iter()
            .map(|item| item.as_number().expect("a number").lexeme())
            .collect();
        assert_eq!(
            lexemes,
            ["-83.42391749999999", "0.10000000000000000555", "1E+2"]
        );
        assert_eq!(
            write(&value),
            "[-83.42391749999999,0.10000000000000000555,1E+2]"
        );
    }

    #[test]
    fn count_and_kind_name_describe_a_value() {
        let value = parse(r#"{"a": 1, "a": 2, "b": null}"#).expect("JSON");
        assert_eq!(count(&value, "a"), 2);
        assert_eq!(count(&value, "b"), 1);
        assert_eq!(count(&value, "c"), 0);
        assert_eq!(value.get("a").and_then(JsonValue::as_u64), Some(1));
        assert_eq!(count(&JsonValue::Null, "a"), 0);
        for (text, name) in [
            ("null", "null"),
            ("true", "a boolean"),
            ("1", "a number"),
            ("\"\"", "a string"),
            ("[]", "an array"),
            ("{}", "an object"),
        ] {
            assert_eq!(kind_name(&parse(text).expect("JSON")), name);
        }
    }

    /// A hundred thousand nested arrays, and a hundred thousand nested objects,
    /// are read and written back byte for byte on a 128 KiB stack: this crate
    /// lifts the reader's depth cap, and a refusal at the bottom of such a
    /// document is still a refusal at its own byte.
    #[test]
    fn nesting_is_bounded_by_memory_alone() {
        let depth = deep::DEEP;
        purrdf_stack::on_stack(deep::SMALL_STACK, move || {
            let arrays = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
            let value = parse(&arrays).expect("a hundred thousand nested arrays parse");
            assert_eq!(write(&value), arrays, "and write back byte for byte");
            drop(value);

            let objects = format!("{}1{}", "{\"a\":".repeat(depth), "}".repeat(depth));
            let value = parse(&objects).expect("a hundred thousand nested objects parse");
            assert_eq!(write(&value), objects, "and write back byte for byte");
            drop(value);

            let mut unfinished = "[".repeat(depth);
            assert_eq!(
                refusal(&unfinished),
                format!("JSON byte {depth}: expected a JSON value, found the end of the text")
            );
            unfinished.push('x');
            assert_eq!(
                refusal(&unfinished),
                format!("JSON byte {depth}: expected a JSON value, found `x`")
            );
        })
        .expect("the thread starts");
    }
}
