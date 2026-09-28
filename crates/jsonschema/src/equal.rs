// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON value equality as JSON Schema defines it (2020-12 Core §4.2.2).
//!
//! Two values are equal when they are the same type and the same value, where
//! numbers compare by mathematical value (`1` equals `1.0`), strings by code
//! points, arrays item by item, and objects by key set and per-key value. Key
//! order and number spelling never matter.

use std::hash::{BuildHasher as _, Hash, Hasher};

use serde_json::Value;

use crate::number::Decimal;

/// JSON Schema equality.
///
/// The walk keeps its own stack of pairs still to compare, so a deeply nested
/// pair of values costs heap, never call-stack: an instance that arrived through
/// the reader's own depth bound is safe here, and so is one a caller built by
/// hand at any depth.
pub(crate) fn equal(left: &Value, right: &Value) -> bool {
    let mut pending = vec![(left, right)];
    while let Some((left, right)) = pending.pop() {
        match (left, right) {
            (Value::Null, Value::Null) => {}
            (Value::Bool(a), Value::Bool(b)) => {
                if a != b {
                    return false;
                }
            }
            (Value::Number(a), Value::Number(b)) => {
                if Decimal::from_number(a) != Decimal::from_number(b) {
                    return false;
                }
            }
            (Value::String(a), Value::String(b)) => {
                if a != b {
                    return false;
                }
            }
            (Value::Array(a), Value::Array(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                pending.extend(a.iter().zip(b));
            }
            (Value::Object(a), Value::Object(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for (key, value) in a {
                    match b.get(key) {
                        Some(other) => pending.push((value, other)),
                        None => return false,
                    }
                }
            }
            _ => return false,
        }
    }
    true
}

/// What the hashing walk still has to feed into the hasher: a value, or an
/// object member's key, which is hashed just before that member's value.
enum Frame<'a> {
    Value(&'a Value),
    Key(&'a str),
}

/// Feed `value` into `state` so that [`equal`] values hash alike: numbers hash
/// their exact [`Decimal`], and object members hash in key order (the value
/// irrespective of the consumer's `serde_json/preserve_order` feature).
///
/// Like [`equal`], the walk keeps its own stack, so nesting depth never reaches
/// the call stack.
pub(crate) fn hash_value<H: Hasher>(value: &Value, state: &mut H) {
    let mut pending = vec![Frame::Value(value)];
    while let Some(frame) = pending.pop() {
        let value = match frame {
            Frame::Key(key) => {
                key.hash(state);
                continue;
            }
            Frame::Value(value) => value,
        };
        match value {
            Value::Null => state.write_u8(0),
            Value::Bool(flag) => {
                state.write_u8(1);
                flag.hash(state);
            }
            Value::Number(number) => {
                state.write_u8(2);
                Decimal::from_number(number).hash(state);
            }
            Value::String(text) => {
                state.write_u8(3);
                text.hash(state);
            }
            Value::Array(items) => {
                state.write_u8(4);
                state.write_usize(items.len());
                // Pushed last-to-first so the first item is hashed first.
                pending.extend(items.iter().rev().map(Frame::Value));
            }
            Value::Object(map) => {
                state.write_u8(5);
                state.write_usize(map.len());
                let mut members: Vec<_> = map.iter().collect();
                members.sort_unstable_by_key(|(left, _)| *left);
                // Pushed last-to-first, value under key, so each key is hashed
                // immediately before its value, in key order.
                for (key, member) in members.into_iter().rev() {
                    pending.push(Frame::Value(member));
                    pending.push(Frame::Key(key));
                }
            }
        }
    }
}

impl Hash for Decimal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // `Decimal` is normalized, so equal values have equal fields.
        let (negative, coefficient, exponent) = self.parts();
        negative.hash(state);
        coefficient.hash(state);
        exponent.hash(state);
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
    // The workspace's fixed-key hasher: the scan is deterministic, and the
    // answer never depends on the hash value, only on `equal`.
    let mut buckets: std::collections::BTreeMap<u64, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let mut hasher = purrdf_hash::fixed::FixedState::new().build_hasher();
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
    use serde_json::json;

    #[test]
    fn numbers_compare_by_value_and_structures_recurse() {
        assert!(equal(&json!(1), &json!(1.0)));
        assert!(equal(
            &json!({"a": [1, {"b": 2.0}]}),
            &json!({"a": [1.0, {"b": 2}]})
        ));
        assert!(!equal(&json!([1]), &json!([true])));
        assert!(!equal(&json!({"a": 1}), &json!({"a": 1, "b": 1})));
        assert!(!equal(&json!(0), &json!(false)));
    }

    /// `depth` nested single-item arrays around `leaf`.
    fn nested(depth: usize, leaf: Value) -> Value {
        (0..depth).fold(leaf, |inner, _| Value::Array(vec![inner]))
    }

    #[test]
    fn deep_values_compare_and_hash_on_a_small_stack() {
        // Deep enough that a recursive walk would need far more than the thread's
        // stack; the values are dropped on the test thread, whose stack is large.
        let left = nested(4_000, json!(1));
        let same = nested(4_000, json!(1.0));
        let other = nested(4_000, json!(2));
        std::thread::scope(|scope| {
            let worker = std::thread::Builder::new()
                .stack_size(16 * 1024)
                .spawn_scoped(scope, || {
                    assert!(equal(&left, &same));
                    assert!(!equal(&left, &other));
                    let mut a = std::hash::DefaultHasher::new();
                    let mut b = std::hash::DefaultHasher::new();
                    hash_value(&left, &mut a);
                    hash_value(&same, &mut b);
                    assert_eq!(a.finish(), b.finish());
                })
                .expect("spawn");
            worker
                .join()
                .expect("the walks must not overflow the stack");
        });
    }

    #[test]
    fn duplicates_are_found_in_small_and_large_arrays() {
        assert_eq!(
            first_duplicate(&[json!(1), json!(2), json!(1.0)]),
            Some((0, 2))
        );
        assert_eq!(first_duplicate(&[json!(1), json!(true)]), None);
        let mut large: Vec<Value> = (0..40).map(|n| json!({"n": n})).collect();
        assert_eq!(first_duplicate(&large), None);
        large.push(json!({"n": 7.0}));
        assert_eq!(first_duplicate(&large), Some((7, 40)));
    }
}
