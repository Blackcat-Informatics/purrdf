// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON value equality as JSON Schema defines it (2020-12 Core §4.2.2).
//!
//! Two values are equal when they are the same type and the same value, where
//! numbers compare by mathematical value (`1` equals `1.0`), strings by code
//! points, arrays item by item, and objects by key set and per-key value. Key
//! order and number spelling never matter.

use std::hash::{Hash, Hasher};

use serde_json::Value;

use crate::number::Decimal;

/// JSON Schema equality.
pub(crate) fn equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => Decimal::from_number(a) == Decimal::from_number(b),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| equal(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| equal(value, other)))
        }
        _ => false,
    }
}

/// Feed `value` into `state` so that [`equal`] values hash alike: numbers hash
/// their exact [`Decimal`], and object members hash in key order (the value
/// model keeps keys sorted, so this is a canonical order).
pub(crate) fn hash_value<H: Hasher>(value: &Value, state: &mut H) {
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
            for item in items {
                hash_value(item, state);
            }
        }
        Value::Object(map) => {
            state.write_u8(5);
            state.write_usize(map.len());
            for (key, member) in map {
                key.hash(state);
                hash_value(member, state);
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
    // `DefaultHasher::new` is a fixed-key hasher, so the scan is deterministic.
    let mut buckets: std::collections::BTreeMap<u64, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let mut hasher = std::hash::DefaultHasher::new();
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
