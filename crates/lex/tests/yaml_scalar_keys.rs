// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Non-string mapping keys in the YAML reader: refused by default (JSON object
//! names are strings, and YAML-LD requires the refusal), accepted as their
//! source text under `Limits::scalar_keys`, and a collection is refused either
//! way. Each refusal is paired with the neighbour that must still be read.

use purrdf_lex::json::write_compact;
use purrdf_lex::yaml::{ErrorKind, Limits, read, read_with};

const SCALAR: Limits = Limits {
    scalar_keys: true,
    ..Limits::DEFAULT
};

fn strict(text: &str) -> Result<String, ErrorKind> {
    read(text)
        .map(|value| write_compact(&value))
        .map_err(|error| error.kind())
}

fn scalar(text: &str) -> Result<String, ErrorKind> {
    read_with(text, SCALAR)
        .map(|value| write_compact(&value))
        .map_err(|error| error.kind())
}

#[test]
fn by_default_a_number_boolean_or_null_key_is_refused_and_its_string_spelling_is_read() {
    for refused in [
        "1: a",
        "true: a",
        "null: a",
        "1.5: a",
        "~: a",
        "0x1F: a",
        "True: a",
        "? 1\n: a",
        "!!int 1: a",
        "{1: a}",
        "[1: a]",
        "a: {true: b}",
    ] {
        assert_eq!(strict(refused), Err(ErrorKind::NonStringKey), "{refused:?}");
    }
    // The neighbours: the same spellings as strings, and string-shaped plain keys.
    for (accepted, json) in [
        ("'1': a", r#"{"1":"a"}"#),
        ("\"true\": a", r#"{"true":"a"}"#),
        ("!!str 1: a", r#"{"1":"a"}"#),
        ("? '1'\n: a", r#"{"1":"a"}"#),
        ("{'1': a}", r#"{"1":"a"}"#),
        ("007: a", r#"{"007":"a"}"#),
        ("1a: b", r#"{"1a":"b"}"#),
        ("nulls: a", r#"{"nulls":"a"}"#),
        ("a: 1", r#"{"a":1}"#),
    ] {
        assert_eq!(strict(accepted).as_deref(), Ok(json), "{accepted:?}");
    }
}

#[test]
fn with_scalar_keys_a_number_boolean_or_null_key_is_its_source_text() {
    for (text, json) in [
        ("1: a", r#"{"1":"a"}"#),
        ("-3: a", r#"{"-3":"a"}"#),
        ("true: a", r#"{"true":"a"}"#),
        ("True: a", r#"{"True":"a"}"#),
        ("null: a", r#"{"null":"a"}"#),
        ("~: a", r#"{"~":"a"}"#),
        ("1.5: a", r#"{"1.5":"a"}"#),
        ("1.50: a", r#"{"1.50":"a"}"#),
        ("0x1F: a", r#"{"0x1F":"a"}"#),
        ("-.inf: a", r#"{"-.inf":"a"}"#),
        (".nan: a", r#"{".nan":"a"}"#),
        ("!!int 1: a", r#"{"1":"a"}"#),
        ("!!null : a", r#"{"":"a"}"#),
        ("? \n: a", r#"{"":"a"}"#),
        ("? 1\n: a", r#"{"1":"a"}"#),
        ("? 1", r#"{"1":null}"#),
        ("{1: a}", r#"{"1":"a"}"#),
        (
            "{true: a, null: b, ~: c}",
            r#"{"true":"a","null":"b","~":"c"}"#,
        ),
        ("[1: a]", r#"[{"1":"a"}]"#),
        ("k:\n  - 1: a", r#"{"k":[{"1":"a"}]}"#),
        // An alias names the key by what it denotes.
        ("a: &x 1\n*x : b", r#"{"a":1,"1":"b"}"#),
        // An anchor on a key still names the number it denotes.
        ("&x 1: a\nb: *x", r#"{"1":"a","b":1}"#),
        // Values are unaffected, and the keys stay distinct texts.
        (
            "1: 1\ntrue: true\nnull: null",
            r#"{"1":1,"true":true,"null":null}"#,
        ),
        ("1.0: a\n1: b", r#"{"1.0":"a","1":"b"}"#),
    ] {
        assert_eq!(scalar(text).as_deref(), Ok(json), "{text:?}");
    }
}

#[test]
fn with_scalar_keys_a_repeated_key_is_still_refused_and_a_collection_is_never_a_key() {
    // `1` and `"1"` spell one key.
    assert_eq!(scalar("1: a\n\"1\": b"), Err(ErrorKind::DuplicateKey));
    assert_eq!(scalar("1: a\n1: b"), Err(ErrorKind::DuplicateKey));
    for refused in [
        "[x]: a",
        "{x: y}: a",
        "? [x]\n: a",
        "? - x\n: a",
        "? {x: y}\n: a",
        "k: {[x]: a}",
        "[[x]: a]",
        ": a",
    ] {
        assert_eq!(scalar(refused), Err(ErrorKind::NonStringKey), "{refused:?}");
        assert_eq!(strict(refused), Err(ErrorKind::NonStringKey), "{refused:?}");
    }
    // A tag must match its key, as it must match a value.
    assert_eq!(scalar("!!int abc: a"), Err(ErrorKind::Tag));
    assert_eq!(scalar("!!bool 1: a"), Err(ErrorKind::Tag));
    assert_eq!(scalar("!!str 1: a").as_deref(), Ok(r#"{"1":"a"}"#));
    // `.inf` stays refused as a value.
    assert_eq!(scalar("a: .inf"), Err(ErrorKind::NonFinite));
}
