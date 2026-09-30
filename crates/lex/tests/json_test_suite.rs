// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Nicolas Seriot's JSONTestSuite `test_parsing/` corpus (vendored byte-frozen in
//! `vectors/JSONTestSuite/`; `PROVENANCE.md` names the pinned commit) against
//! [`purrdf_lex::json`].
//!
//! * every `y_*` document RFC 8259 requires a parser to accept is accepted, and
//!   round-trips: parse, write (compact and pretty), parse again gives an equal
//!   and identically spelled value;
//! * every `n_*` document RFC 8259 requires a parser to refuse is refused, on a
//!   256 KiB stack even when the depth cap is lifted (a hundred-thousand-deep
//!   unterminated document is a typed refusal, not an abort);
//! * every `i_*` document is implementation-defined, and each one's outcome
//!   here is pinned by file name with the reason, consistent with the reader's
//!   documented policy: UTF-8 input only and no byte-order mark, an unpaired
//!   surrogate escape refused, numbers kept as lexemes and never range-checked,
//!   and a container-depth cap that is always explicit.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use purrdf_lex::json::{self, ErrorKind, Limits, Value};
use purrdf_testkit::paths::workspace_root;

fn corpus() -> PathBuf {
    workspace_root().join("vectors/JSONTestSuite/test_parsing")
}

/// Every vendored document as `(file name, bytes)`, sorted by name.
fn documents() -> Vec<(String, Arc<Vec<u8>>)> {
    let mut all: Vec<_> = std::fs::read_dir(corpus())
        .expect("vendored JSONTestSuite directory")
        .map(|entry| {
            let entry = entry.expect("directory entry");
            let name = entry.file_name().into_string().expect("UTF-8 file name");
            let bytes = std::fs::read(entry.path()).expect("document bytes");
            (name, Arc::new(bytes))
        })
        .collect();
    all.sort_by(|(left, _), (right, _)| left.cmp(right));
    all
}

fn with_prefix(prefix: &str) -> Vec<(String, Arc<Vec<u8>>)> {
    documents()
        .into_iter()
        .filter(|(name, _)| name.starts_with(prefix))
        .collect()
}

/// Read `bytes` on a 256 KiB stack: no document may need a frame per level.
fn read_small_stack(bytes: &Arc<Vec<u8>>, limits: Limits) -> Result<Value, json::Error> {
    let bytes = Arc::clone(bytes);
    purrdf_stack::on_stack(256 * 1024, move || json::read_slice(&bytes, limits))
        .expect("the reader completes on a small stack")
}

#[test]
fn the_vendored_corpus_has_the_pinned_census() {
    assert_eq!(with_prefix("y_").len(), 95);
    assert_eq!(with_prefix("n_").len(), 188);
    assert_eq!(with_prefix("i_").len(), 35);
    assert_eq!(documents().len(), 318, "every document is y_, n_ or i_");
}

#[test]
fn every_y_document_is_accepted() {
    for (name, bytes) in with_prefix("y_") {
        if let Err(error) = read_small_stack(&bytes, Limits::DEFAULT) {
            panic!("{name} must be accepted, refused: {error}");
        }
    }
}

#[test]
fn every_n_document_is_refused_at_the_default_and_at_an_unbounded_depth() {
    for (name, bytes) in with_prefix("n_") {
        for limits in [Limits::DEFAULT, Limits::with_depth(usize::MAX)] {
            assert!(
                read_small_stack(&bytes, limits).is_err(),
                "{name} must be refused (limits {limits:?})"
            );
        }
    }
}

#[test]
fn every_y_document_round_trips_to_an_equal_value() {
    for (name, bytes) in with_prefix("y_") {
        let first = json::read_slice(&bytes, Limits::DEFAULT).expect("a y_ document reads");
        for (style, text) in [
            ("compact", json::write_compact(&first)),
            ("pretty", json::write_pretty(&first)),
        ] {
            let again = json::read(&text)
                .unwrap_or_else(|error| panic!("{name}: the {style} write reads back: {error}"));
            assert_eq!(first, again, "{name}: {style} round trip (by value)");
            assert!(
                first.same_text(&again),
                "{name}: {style} round trip (spelling and member order)"
            );
        }
        // The write is a fixed point: writing what was read back gives the same text.
        let compact = json::write_compact(&first);
        let reread = json::read(&compact).expect("compact reads");
        assert_eq!(json::write_compact(&reread), compact, "{name}");
    }
}

/// `value` with the members of every object in descending name order. The
/// sort is stable, so a repeated name keeps its occurrences in document order,
/// the order equality pairs them in.
fn reordered(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(reordered).collect()),
        Value::Object(object) => {
            let mut members: Vec<_> = object
                .iter()
                .map(|(name, member)| (name.to_owned(), reordered(member)))
                .collect();
            members.sort_by(|(left, _), (right, _)| right.cmp(left));
            Value::Object(json::Object::from(members))
        }
        other => other.clone(),
    }
}

/// `value` with every integer respelled as `n.0`, the same number.
fn respelled(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(respelled).collect()),
        Value::Object(object) => {
            let members: Vec<_> = object
                .iter()
                .map(|(name, member)| (name.to_owned(), respelled(member)))
                .collect();
            Value::Object(json::Object::from(members))
        }
        Value::Number(number) if number.is_integer() => Value::Number(
            json::Number::from_lexeme(format!("{}.0", number.lexeme()))
                .expect("an integer lexeme with a fraction"),
        ),
        other => other.clone(),
    }
}

fn hash_of(value: &Value) -> u64 {
    let mut hasher = purrdf_hash::fixed::FixedHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn value_equality_and_hash_ignore_member_order_and_number_spelling_on_every_y_document() {
    for (name, bytes) in with_prefix("y_") {
        let value = json::read_slice(&bytes, Limits::DEFAULT).expect("a y_ document reads");
        let flipped = reordered(&value);
        let respelled = respelled(&value);
        assert_eq!(
            value, flipped,
            "{name}: member order is not part of a value"
        );
        assert_eq!(value, respelled, "{name}: `1` and `1.0` are one number");
        assert_eq!(hash_of(&value), hash_of(&flipped), "{name}");
        assert_eq!(hash_of(&value), hash_of(&respelled), "{name}");
        assert!(value.same_text(&value), "{name}");
    }
}

/// The pinned outcome of each `i_*` document, with the reason.
const IMPLEMENTATION_DEFINED: &[(&str, bool, &str)] = &[
    // Numbers are kept as their lexemes and never range-checked (`Number`).
    (
        "i_number_double_huge_neg_exp.json",
        true,
        "a number is its lexeme; no range check",
    ),
    (
        "i_number_huge_exp.json",
        true,
        "a number is its lexeme; no range check",
    ),
    (
        "i_number_neg_int_huge_exp.json",
        true,
        "a number is its lexeme; no range check",
    ),
    (
        "i_number_pos_double_huge_exp.json",
        true,
        "a number is its lexeme; no range check",
    ),
    (
        "i_number_real_neg_overflow.json",
        true,
        "a number is its lexeme; no range check",
    ),
    (
        "i_number_real_pos_overflow.json",
        true,
        "a number is its lexeme; no range check",
    ),
    (
        "i_number_real_underflow.json",
        true,
        "a number is its lexeme; no range check",
    ),
    (
        "i_number_too_big_neg_int.json",
        true,
        "a number is its lexeme; no range check",
    ),
    (
        "i_number_too_big_pos_int.json",
        true,
        "a number is its lexeme; no range check",
    ),
    (
        "i_number_very_big_negative_int.json",
        true,
        "a number is its lexeme; no range check",
    ),
    // An escape naming an unpaired or inverted surrogate denotes no character.
    (
        "i_object_key_lone_2nd_surrogate.json",
        false,
        "unpaired surrogate escape in a name",
    ),
    (
        "i_string_1st_surrogate_but_2nd_missing.json",
        false,
        "unpaired surrogate escape",
    ),
    (
        "i_string_1st_valid_surrogate_2nd_invalid.json",
        false,
        "unpaired surrogate escape",
    ),
    (
        "i_string_incomplete_surrogate_and_escape_valid.json",
        false,
        "unpaired surrogate escape",
    ),
    (
        "i_string_incomplete_surrogate_pair.json",
        false,
        "unpaired surrogate escape",
    ),
    (
        "i_string_incomplete_surrogates_escape_valid.json",
        false,
        "unpaired surrogate escape",
    ),
    (
        "i_string_invalid_lonely_surrogate.json",
        false,
        "unpaired surrogate escape",
    ),
    (
        "i_string_invalid_surrogate.json",
        false,
        "unpaired surrogate escape",
    ),
    (
        "i_string_inverted_surrogates_U+1D11E.json",
        false,
        "surrogates in the wrong order",
    ),
    (
        "i_string_lone_second_surrogate.json",
        false,
        "unpaired low surrogate escape",
    ),
    // A surrogate code point written as raw bytes is not UTF-8 (RFC 3629).
    (
        "i_string_UTF8_surrogate_U+D800.json",
        false,
        "a raw surrogate is not UTF-8",
    ),
    // The input is UTF-8 (RFC 8259 §8.1), and a byte-order mark is not JSON text.
    (
        "i_string_UTF-16LE_with_BOM.json",
        false,
        "UTF-16 is not UTF-8",
    ),
    ("i_string_utf16BE_no_BOM.json", false, "UTF-16 is not UTF-8"),
    ("i_string_utf16LE_no_BOM.json", false, "UTF-16 is not UTF-8"),
    (
        "i_structure_UTF-8_BOM_empty_object.json",
        false,
        "a byte-order mark precedes the value",
    ),
    (
        "i_string_UTF-8_invalid_sequence.json",
        false,
        "invalid UTF-8",
    ),
    ("i_string_invalid_utf-8.json", false, "invalid UTF-8"),
    ("i_string_iso_latin_1.json", false, "Latin-1 is not UTF-8"),
    (
        "i_string_lone_utf8_continuation_byte.json",
        false,
        "invalid UTF-8",
    ),
    (
        "i_string_not_in_unicode_range.json",
        false,
        "beyond U+10FFFF is not UTF-8",
    ),
    (
        "i_string_overlong_sequence_2_bytes.json",
        false,
        "overlong UTF-8",
    ),
    (
        "i_string_overlong_sequence_6_bytes.json",
        false,
        "overlong UTF-8",
    ),
    (
        "i_string_overlong_sequence_6_bytes_null.json",
        false,
        "overlong UTF-8",
    ),
    ("i_string_truncated-utf-8.json", false, "truncated UTF-8"),
    // Depth is an explicit cap (`Limits`): 128 open containers by default.
    (
        "i_structure_500_nested_arrays.json",
        false,
        "500 open arrays exceed the default depth of 128",
    ),
];

#[test]
fn every_i_document_has_its_outcome_pinned() {
    let pinned: BTreeSet<&str> = IMPLEMENTATION_DEFINED
        .iter()
        .map(|(name, ..)| *name)
        .collect();
    assert_eq!(
        pinned.len(),
        IMPLEMENTATION_DEFINED.len(),
        "no file is pinned twice"
    );
    let present = with_prefix("i_");
    let present: BTreeSet<&str> = present.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        pinned, present,
        "the pinned table names exactly the i_ documents"
    );
    for (name, bytes) in with_prefix("i_") {
        let &(_, accepted, reason) = IMPLEMENTATION_DEFINED
            .iter()
            .find(|(pinned, ..)| *pinned == name)
            .expect("pinned");
        let outcome = read_small_stack(&bytes, Limits::DEFAULT);
        assert_eq!(outcome.is_ok(), accepted, "{name} ({reason}): {outcome:?}");
    }
}

#[test]
fn the_depth_refusal_is_the_cap_and_its_valid_neighbour_is_accepted() {
    let (_, bytes) = with_prefix("i_structure_500_nested_arrays")
        .pop()
        .expect("the nested-arrays document");
    let refused = json::read_slice(&bytes, Limits::DEFAULT).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Depth { limit: 128 });
    // One level short of the document's depth still refuses; exactly its depth reads.
    assert!(json::read_slice(&bytes, Limits::with_depth(499)).is_err());
    let read = read_small_stack(&bytes, Limits::with_depth(500)).expect("500 levels read");
    let mut levels = 0;
    let mut cursor = &read;
    while let Value::Array(items) = cursor {
        levels += 1;
        match items.first() {
            Some(inner) => cursor = inner,
            None => break,
        }
    }
    assert_eq!(levels, 500);
}

#[test]
fn a_refused_encoding_has_an_accepted_neighbour() {
    let bom_free = b"{}".to_vec();
    let with_bom = [b"\xEF\xBB\xBF".as_slice(), b"{}"].concat();
    assert!(json::read_slice(&bom_free, Limits::DEFAULT).is_ok());
    assert!(json::read_slice(&with_bom, Limits::DEFAULT).is_err());
    // A paired surrogate escape is a character; each half alone is not.
    assert_eq!(json::read(r#""\ud834\udd1e""#).unwrap(), "\u{1D11E}");
    assert!(json::read(r#""\ud834""#).is_err());
    assert!(json::read(r#""\udd1e""#).is_err());
    assert!(json::read(r#""\udd1e\ud834""#).is_err());
    // The same character written directly is UTF-8 and accepted.
    assert_eq!(json::read("\"\u{1D11E}\"").unwrap(), "\u{1D11E}");
}
