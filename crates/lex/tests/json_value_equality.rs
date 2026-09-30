// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The equality contract of [`purrdf_lex::json::Value`] and
//! [`purrdf_lex::json::Number`]: numbers by exact decimal value, objects
//! unordered, a repeated name pairing its occurrences in document order, `Hash`
//! consistent with `==`, and `same_text` the spelling-and-order identity.

use std::hash::{Hash, Hasher};

use purrdf_lex::json::{self, Number, Object, Value};

fn read(text: &str) -> Value {
    json::read(text).expect("JSON")
}

fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = purrdf_hash::fixed::FixedHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

/// `depth` arrays nested inside one another around `leaf`.
fn nested(depth: usize, leaf: Value) -> Value {
    let mut value = leaf;
    for _ in 0..depth {
        value = Value::Array(vec![value]);
    }
    value
}

#[test]
fn numbers_are_equal_when_they_denote_one_decimal() {
    let same = [
        ("1", "1.0"),
        ("1", "1e0"),
        ("1e2", "100"),
        ("1.0", "1.00"),
        ("1.5", "15e-1"),
        ("0", "-0"),
        ("0.0", "-0.0"),
        ("0", "0e99999999999999999999"),
        ("100", "1E+2"),
        ("0.001", "1e-3"),
        ("12345678901234567890123", "1.2345678901234567890123e22"),
        // Exponents past `i64`, on both sides of its edge.
        ("1e999999999999999999999", "10e999999999999999999998"),
        ("1e9223372036854775808", "100e9223372036854775806"),
        ("1e9223372036854775807", "0.1e9223372036854775808"),
        ("1e-9223372036854775809", "0.1e-9223372036854775808"),
        ("1e-9223372036854775808", "10e-9223372036854775809"),
        ("1.5e99999999999999999999", "15e99999999999999999998"),
    ];
    for (left, right) in same {
        let (a, b) = (read(left), read(right));
        assert_eq!(a, b, "{left} == {right}");
        assert_eq!(b, a, "{right} == {left}");
        assert_eq!(hash_of(&a), hash_of(&b), "{left} hashes like {right}");
    }
    // The valid neighbours: numbers that differ by a digit, a sign or an exponent.
    let different = [
        ("1", "2"),
        ("1", "-1"),
        ("1e2", "101"),
        ("1e2", "1e3"),
        ("1.5", "15"),
        ("0.1", "0.10000000000000001"),
        ("1e400", "1e401"),
        ("1e999999999999999999999", "1e999999999999999999998"),
        ("1e9223372036854775808", "1e9223372036854775807"),
        ("1e-9223372036854775809", "1e-9223372036854775808"),
        ("1e9223372036854775808", "1e-9223372036854775808"),
        ("100", "10"),
    ];
    for (left, right) in different {
        assert_ne!(read(left), read(right), "{left} != {right}");
    }
}

#[test]
fn number_equality_is_by_value_and_same_text_is_by_spelling() {
    let number = |text: &str| Number::from_lexeme(text).expect("number");
    assert_eq!(number("1.0"), number("1"));
    assert!(!number("1.0").same_text(&number("1")));
    assert!(number("1.0").same_text(&number("1.0")));
    assert_eq!(hash_of(&number("1e2")), hash_of(&number("100")));
    assert_eq!(read("1.0"), 1_u8);
    assert_eq!(read("1e2"), 100_i64);
    assert_eq!(read("-0"), 0_i8);
    assert_ne!(read("1.5"), 1_u8);
    assert_ne!(read("\"1\""), 1_u8);
}

#[test]
fn objects_are_equal_whatever_their_member_order() {
    assert_eq!(
        read(r#"{"a":1,"b":2}"#),
        read(r#"{"b":2,"a":1}"#),
        "member order"
    );
    assert_eq!(
        read(r#"{"a":[1,{"x":2,"y":3}],"b":null}"#),
        read(r#"{"b":null,"a":[1.0,{"y":3,"x":2e0}]}"#)
    );
    assert_ne!(read(r#"{"a":1}"#), read(r#"{"a":1,"b":1}"#));
    assert_ne!(read(r#"{"a":1}"#), read(r#"{"b":1}"#));
    assert_ne!(read(r#"{"a":1,"b":2}"#), read(r#"{"a":2,"b":1}"#));
    // Arrays keep their order.
    assert_ne!(read("[1,2]"), read("[2,1]"));
    // Kinds differ.
    assert_ne!(read("0"), read("false"));
    assert_ne!(read("[1]"), read("[true]"));
    assert_ne!(read("null"), read("false"));
    assert_ne!(read(r#""1""#), read("1"));
    // `Object` compares as its value does.
    let left = Object::new().with("a", 1_u8).with("b", 2_u8);
    let right = Object::new().with("b", 2_u8).with("a", 1_u8);
    assert_eq!(left, right);
    assert_eq!(hash_of(&left), hash_of(&right));
    assert_ne!(left, Object::new().with("a", 1_u8));
}

#[test]
fn a_repeated_name_is_compared_occurrence_by_occurrence() {
    // The order of the two `a` occurrences is significant; other names float.
    assert_eq!(
        read(r#"{"a":1,"b":0,"a":2}"#),
        read(r#"{"b":0.0,"a":1,"a":2}"#)
    );
    assert_eq!(
        hash_of(&read(r#"{"a":1,"b":0,"a":2}"#)),
        hash_of(&read(r#"{"b":0.0,"a":1,"a":2}"#))
    );
    assert_ne!(read(r#"{"a":1,"a":2}"#), read(r#"{"a":2,"a":1}"#));
    // Same length, every name on the left present on the right.
    assert_ne!(read(r#"{"a":1,"a":1}"#), read(r#"{"a":1,"b":1}"#));
    assert_ne!(read(r#"{"a":1,"a":1}"#), read(r#"{"a":1}"#));
}

#[test]
fn same_text_sees_spelling_and_member_order() {
    assert!(
        read(r#"{"a":[1.5,"x"],"b":null}"#).same_text(&read(r#"{ "a" : [1.5, "x"], "b":null }"#))
    );
    assert!(!read("1.0").same_text(&read("1")));
    assert!(!read("1e2").same_text(&read("100")));
    assert!(!read(r#"{"a":1,"b":2}"#).same_text(&read(r#"{"b":2,"a":1}"#)));
    assert!(!read(r#"[{"a":1.0}]"#).same_text(&read(r#"[{"a":1}]"#)));
    assert!(!read("[1]").same_text(&read("[1,2]")));
    assert!(!read("true").same_text(&read("1")));
}

#[test]
fn equal_values_hash_alike_and_structure_is_part_of_the_hash() {
    let forward = read(r#"{"a":[1,{"x":2,"y":3}],"b":null}"#);
    let backward = read(r#"{"b":null,"a":[1.0,{"y":3,"x":2.0}]}"#);
    assert_eq!(forward, backward);
    assert_eq!(hash_of(&forward), hash_of(&backward));
    assert_ne!(hash_of(&read("[[1],2]")), hash_of(&read("[1,[2]]")));
    assert_ne!(
        hash_of(&read(r#"{"a":"b"}"#)),
        hash_of(&read(r#"{"ab":""}"#))
    );
    assert_ne!(hash_of(&read("0")), hash_of(&read("false")));
    // A `HashSet` deduplicates by value.
    let set: std::collections::HashSet<Value> = [
        "1",
        "1.0",
        "1e0",
        "2",
        r#"{"a":1,"b":2}"#,
        r#"{"b":2,"a":1}"#,
    ]
    .into_iter()
    .map(read)
    .collect();
    assert_eq!(set.len(), 3);
}

#[test]
fn values_a_hundred_thousand_levels_deep_compare_and_hash_on_a_small_stack() {
    // 256 KiB of stack: comparing and hashing must not recurse per level.
    purrdf_stack::on_stack(256 * 1024, || {
        const DEPTH: usize = 100_000;
        let left = nested(DEPTH, read("1"));
        let same = nested(DEPTH, read("1.0"));
        let differs = nested(DEPTH, read("2"));
        assert_eq!(left, same);
        assert_eq!(hash_of(&left), hash_of(&same));
        assert_ne!(left, differs);
        assert_ne!(hash_of(&left), hash_of(&differs));
        assert!(!left.same_text(&same));
        assert!(left.same_text(&left.clone()));
    })
    .expect("thread");
}
