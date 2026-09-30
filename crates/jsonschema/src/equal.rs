// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON value equality as JSON Schema defines it (2020-12 Core §4.2.2).
//!
//! Two values are equal when they are the same type and the same value, where
//! numbers compare by mathematical value (`1` equals `1.0`), strings by code
//! points, arrays item by item, and objects by key set and per-key value. Key
//! order and number spelling never matter. An instance object that repeats a
//! name (which RFC 8259 §4 leaves undefined) compares that name's occurrences
//! in document order.

use std::hash::{Hash, Hasher};

use purrdf_lex::json::{Object, Value};

use crate::number::exact;

/// An object's members ordered by name, repeats in document order: the order
/// [`equal`] pairs members in and [`hash_value`] feeds them in.
fn by_name(object: &Object) -> Vec<&(String, Value)> {
    let mut members: Vec<_> = object.members().iter().collect();
    members.sort_by(|(left, _), (right, _)| left.cmp(right));
    members
}

/// JSON Schema equality.
///
/// The walk keeps the pairs still to compare on a heap stack rather than
/// recursing, so two values of any nesting depth compare without touching
/// more than a constant amount of the thread's stack — a wasm32 host's
/// included, which cannot grow it.
pub(crate) fn equal(left: &Value, right: &Value) -> bool {
    let mut pending: Vec<(&Value, &Value)> = Vec::new();
    let (mut left, mut right) = (left, right);
    loop {
        let same = match (left, right) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => exact(a) == exact(b),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len() && {
                    pending.extend(a.iter().zip(b));
                    true
                }
            }
            (Value::Object(a), Value::Object(b)) => {
                a.len() == b.len()
                    && by_name(a).into_iter().zip(by_name(b)).all(
                        |((name, value), (other_name, other))| {
                            name == other_name && {
                                pending.push((value, other));
                                true
                            }
                        },
                    )
            }
            _ => false,
        };
        if !same {
            return false;
        }
        match pending.pop() {
            Some((next_left, next_right)) => (left, right) = (next_left, next_right),
            None => return true,
        }
    }
}

/// One step of [`hash_value`]'s walk: a value to hash, or an object member's
/// key, which hashes just before its value.
enum Hashing<'v> {
    Value(&'v Value),
    Key(&'v String),
}

/// Feed `value` into `state` so that [`equal`] values hash alike: numbers hash
/// their exact value, and object members hash in name order whatever order
/// the document wrote them in. Like [`equal`], it walks on a heap stack, so
/// any nesting depth hashes.
pub(crate) fn hash_value<H: Hasher>(value: &Value, state: &mut H) {
    let mut pending = vec![Hashing::Value(value)];
    while let Some(step) = pending.pop() {
        let value = match step {
            Hashing::Key(key) => {
                key.hash(state);
                continue;
            }
            Hashing::Value(value) => value,
        };
        match value {
            Value::Null => state.write_u8(0),
            Value::Bool(flag) => {
                state.write_u8(1);
                flag.hash(state);
            }
            Value::Number(number) => {
                state.write_u8(2);
                exact(number).hash(state);
            }
            Value::String(text) => {
                state.write_u8(3);
                text.hash(state);
            }
            Value::Array(items) => {
                state.write_u8(4);
                state.write_usize(items.len());
                // Reversed onto the stack, so the items hash first to last.
                pending.extend(items.iter().rev().map(Hashing::Value));
            }
            Value::Object(map) => {
                state.write_u8(5);
                state.write_usize(map.len());
                for (key, member) in by_name(map).into_iter().rev() {
                    pending.push(Hashing::Value(member));
                    pending.push(Hashing::Key(key));
                }
            }
        }
    }
}

/// The index pair of the first two equal items, if any.
pub(crate) fn first_duplicate(items: &[Value]) -> Option<(usize, usize)> {
    if items.len() < 2 {
        return None;
    }
    if items.len() <= 16 {
        for (i, left) in items.iter().enumerate() {
            for (offset, right) in items[i + 1..].iter().enumerate() {
                if equal(left, right) {
                    return Some((i, i + 1 + offset));
                }
            }
        }
        return None;
    }
    // Bucket by a hash consistent with `equal`, then confirm within a bucket.
    // `FixedHasher` is a fixed-key hasher, so the scan is deterministic.
    let mut buckets: std::collections::BTreeMap<u64, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let mut hasher = purrdf_hash::fixed::FixedHasher::default();
        hash_value(item, &mut hasher);
        let bucket = buckets.entry(hasher.finish()).or_default();
        if let Some(&first) = bucket.iter().find(|&&earlier| equal(&items[earlier], item)) {
            return Some((first, index));
        }
        bucket.push(index);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(text: &str) -> Value {
        purrdf_lex::json::read(text).expect("JSON")
    }

    /// `depth` arrays nested inside one another around `leaf`.
    fn nested(depth: usize, leaf: Value) -> Value {
        let mut value = leaf;
        for _ in 0..depth {
            value = Value::Array(vec![value]);
        }
        value
    }

    fn hash_of(value: &Value) -> u64 {
        let mut hasher = purrdf_hash::fixed::FixedHasher::default();
        hash_value(value, &mut hasher);
        hasher.finish()
    }

    #[test]
    fn numbers_compare_by_value_and_structures_recurse() {
        assert!(equal(&json("1"), &json("1.0")));
        assert!(equal(
            &json(r#"{"a": [1, {"b": 2.0}]}"#),
            &json(r#"{"a": [1.0, {"b": 2}]}"#)
        ));
        assert!(!equal(&json("[1]"), &json("[true]")));
        assert!(!equal(&json(r#"{"a": 1}"#), &json(r#"{"a": 1, "b": 1}"#)));
        assert!(!equal(&json(r#"{"a": 1}"#), &json(r#"{"b": 1}"#)));
        assert!(!equal(&json("0"), &json("false")));
        assert!(!equal(&json("[1, 2]"), &json("[2, 1]")));
    }

    #[test]
    fn a_repeated_name_is_compared_occurrence_by_occurrence() {
        // Same length, and every name of the left is present on the right.
        assert!(!equal(
            &json(r#"{"a": 1, "a": 1}"#),
            &json(r#"{"a": 1, "b": 1}"#)
        ));
        assert!(equal(
            &json(r#"{"a": 1, "b": 0, "a": 2}"#),
            &json(r#"{"b": 0.0, "a": 1, "a": 2}"#)
        ));
        assert_eq!(
            hash_of(&json(r#"{"a": 1, "b": 0, "a": 2}"#)),
            hash_of(&json(r#"{"b": 0.0, "a": 1, "a": 2}"#))
        );
    }

    #[test]
    fn equal_values_hash_alike_whatever_their_spelling_or_member_order() {
        assert_eq!(hash_of(&json("1")), hash_of(&json("1.0")));
        let forward = json(r#"{"a": [1, {"x": 2, "y": 3}], "b": null}"#);
        let backward = json(r#"{"b": null, "a": [1.0, {"y": 3, "x": 2.0}]}"#);
        assert!(equal(&forward, &backward));
        assert_eq!(hash_of(&forward), hash_of(&backward));
        // Structure is part of the hash: the same leaves nested differently
        // are neither equal nor, here, hashed alike.
        assert_ne!(hash_of(&json("[[1], 2]")), hash_of(&json("[1, [2]]")));
        assert_ne!(
            hash_of(&json(r#"{"a": "b"}"#)),
            hash_of(&json(r#"{"ab": ""}"#))
        );
    }

    #[test]
    fn values_a_hundred_thousand_levels_deep_compare_and_hash_on_a_small_stack() {
        // 256 KiB of stack: comparing and hashing must not recurse per level.
        purrdf_stack::on_stack(256 * 1024, || {
            const DEPTH: usize = 100_000;
            let left = nested(DEPTH, json("1"));
            let same = nested(DEPTH, json("1.0"));
            let differs = nested(DEPTH, json("2"));
            assert!(equal(&left, &same));
            assert_eq!(hash_of(&left), hash_of(&same));
            assert!(!equal(&left, &differs));
            assert_ne!(hash_of(&left), hash_of(&differs));
        })
        .expect("thread");
    }

    #[test]
    fn duplicates_are_found_in_small_and_large_arrays() {
        assert_eq!(
            first_duplicate(&[json("1"), json("2"), json("1.0")]),
            Some((0, 2))
        );
        assert_eq!(first_duplicate(&[json("1"), json("true")]), None);
        let mut large: Vec<Value> = (0..40_u8)
            .map(|n| Value::from(Object::new().with("n", n)))
            .collect();
        assert_eq!(first_duplicate(&large), None);
        large.push(json(r#"{"n": 7.0}"#));
        assert_eq!(first_duplicate(&large), Some((7, 40)));
    }
}
