// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON at the RDF codec boundaries: the strict reader every codec shares, and the
//! binary64 spelling of a JSON number.
//!
//! Every JSON document is read by [`purrdf_lex::json`], whose numbers keep their
//! lexemes, so a number reaches a literal, serialized bytes or an identity exactly as
//! the document wrote it. [`parse_strict`] is that reader with the codecs' shared
//! contract: bounded depth and value count, and duplicate member names refused (RFC
//! 7493 §2.3). Its objects are ordered by member name, the order every codec walk and
//! every emitted byte of the RDF codecs is defined over.
//!
//! [`binary64`] is the one place a JSON number is rounded to binary64: integers in the
//! `i64`/`u64` domain stay integers, and every other number becomes the shortest
//! lexeme of the nearest finite binary64. Its conversion runs inside a
//! [`Binary64Scope`], so the arithmetic fast path rounds once even on x87.

use purrdf_lex::json::{self, Limits, Number, Value};
use purrdf_xsd::ieee::Binary64Scope;

/// The most containers any strict read admits open at once. The JSON-LD walkers over a
/// parsed document recurse per nesting level, and this is the depth they are built for.
const MAX_STRICT_DEPTH: usize = 128;

/// Read one JSON document: at most `values` values, at most `depth` arrays and objects
/// open at once (never more than 128), duplicate member names refused, and every
/// object's members ordered by name.
pub(crate) fn parse_strict(
    bytes: &[u8],
    values: usize,
    depth: usize,
) -> Result<Value, json::Error> {
    let mut value = json::read_slice(
        bytes,
        Limits {
            max_depth: depth.min(MAX_STRICT_DEPTH),
            max_values: u64::try_from(values).unwrap_or(u64::MAX),
            max_string_bytes: usize::MAX,
            unique_members: true,
        },
    )?;
    value.sort_keys();
    Ok(value)
}

/// Run a read whose numbers reach a literal, serialized bytes or an identity, with every
/// binary64 operation inside it rounded once.
#[inline]
pub(crate) fn read_json<T>(read: impl FnOnce() -> T) -> T {
    let _binary64 = Binary64Scope::enter();
    read()
}

/// A JSON number beyond the finite binary64 range, which [`binary64`] cannot spell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NonFiniteNumber(pub(crate) String);

impl std::fmt::Display for NonFiniteNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON number `{}` exceeds finite binary64", self.0)
    }
}

impl std::error::Error for NonFiniteNumber {}

/// The binary64 spelling of `number`: an integer lexeme in the `i64`/`u64` domain as its
/// decimal integer, any other number as the shortest lexeme that reads back as the
/// nearest binary64 ([`Number::from_f64`]).
pub(crate) fn binary64_lexeme(number: &Number) -> Result<String, NonFiniteNumber> {
    if let Some(value) = number.as_i64() {
        return Ok(value.to_string());
    }
    if let Some(value) = number.as_u64() {
        return Ok(value.to_string());
    }
    read_json(|| Number::from_f64(number.as_f64()))
        .map(Number::into_lexeme)
        .ok_or_else(|| NonFiniteNumber(number.lexeme().to_owned()))
}

/// `value` with every number respelt by [`binary64_lexeme`], over a heap work list.
pub(crate) fn binary64(value: &Value) -> Result<Value, NonFiniteNumber> {
    let mut copy = value.clone();
    let mut work: Vec<&mut Value> = vec![&mut copy];
    while let Some(value) = work.pop() {
        match value {
            Value::Number(number) => {
                let lexeme = binary64_lexeme(number)?;
                *number = Number::from_lexeme(lexeme)
                    .expect("an integer or shortest binary64 lexeme is a JSON number");
            }
            Value::Array(items) => work.extend(items.iter_mut()),
            Value::Object(object) => work.extend(object.values_mut()),
            Value::Null | Value::Bool(_) | Value::String(_) => {}
        }
    }
    Ok(copy)
}

#[cfg(test)]
mod tests {
    use super::{binary64, binary64_lexeme, parse_strict};
    use purrdf_lex::json::{self, Number};

    #[test]
    fn exact_numbers_are_single_values_and_marker_keys_stay_objects() {
        for lexical in ["0.25", "18446744073709551616", "1e+400", "-0.00001"] {
            let value = parse_strict(lexical.as_bytes(), 1, 0).expect("one number");
            assert_eq!(json::write_compact(&value), lexical);
        }
        let value = parse_strict(br#"{"$private::Number":"123"}"#, 2, 1).expect("an object");
        assert_eq!(value["$private::Number"].as_str(), Some("123"));
    }

    #[test]
    fn framing_limits_and_decoded_duplicate_names_are_enforced() {
        for bytes in [
            &b"[1,]"[..],
            &b"{\"a\":1,}"[..],
            &b"01"[..],
            &b"true false"[..],
            &b"[1 2]"[..],
            &b"{\"a\" 1}"[..],
            &b"\x0b1"[..],
            &br#"{"a":1,"a":2}"#[..],
        ] {
            assert!(parse_strict(bytes, 10, 8).is_err(), "{bytes:?}");
        }
        assert!(parse_strict(br#"{"a":1,"b":2}"#, 10, 8).is_ok());
        assert!(parse_strict(b"[1,2]", 2, 8).is_err());
        assert!(parse_strict(b"[1,2]", 3, 1).is_ok());
        assert!(parse_strict(b"[[0]]", 3, 1).is_err());
        assert!(parse_strict(b"[[0]]", 3, 2).is_ok());
        let deep = format!("{}0{}", "[".repeat(130), "]".repeat(130));
        assert!(parse_strict(deep.as_bytes(), 1_000, usize::MAX).is_err());
        let limit = format!("{}0{}", "[".repeat(128), "]".repeat(128));
        assert!(parse_strict(limit.as_bytes(), 1_000, usize::MAX).is_ok());
    }

    #[test]
    fn members_are_ordered_by_name() {
        let value = parse_strict(br#"{"b":1,"a":{"d":2,"c":3}}"#, 10, 8).expect("an object");
        assert_eq!(json::write_compact(&value), r#"{"a":{"c":3,"d":2},"b":1}"#);
    }

    #[test]
    fn binary64_keeps_integers_and_rounds_the_rest() {
        let spell = |lexeme: &str| binary64_lexeme(&Number::from_lexeme(lexeme).expect("number"));
        assert_eq!(spell("-0").as_deref(), Ok("0"));
        assert_eq!(
            spell("18446744073709551615").as_deref(),
            Ok("18446744073709551615")
        );
        assert_eq!(
            spell("18446744073709551616").as_deref(),
            Ok("1.8446744073709552e+19")
        );
        assert_eq!(spell("1.50").as_deref(), Ok("1.5"));
        assert_eq!(spell("1e2").as_deref(), Ok("100.0"));
        assert!(spell("1e400").is_err());
        assert!(spell("1e308").is_ok());
        let tree = json::read(r#"{"a":[1.0,2],"b":"1.0"}"#).expect("json");
        assert_eq!(
            json::write_compact(&binary64(&tree).expect("finite")),
            r#"{"a":[1.0,2],"b":"1.0"}"#
        );
    }
}
