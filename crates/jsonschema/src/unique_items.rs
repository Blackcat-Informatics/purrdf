// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `uniqueItems`: the first two equal items of an array.
//!
//! "Equal" is JSON Schema's (2020-12 Core §4.2.2): the same type and the same
//! value, numbers by mathematical value (`1` equals `1.0`), objects by key set
//! and per-key value with member order irrelevant. That is exactly
//! [`purrdf_lex::json::Value`]'s `==` and [`Hash`], the workspace's one
//! definition of JSON value equality, so this module only searches; it defines
//! no equality of its own.

use std::hash::{Hash, Hasher};

use purrdf_lex::json::Value;

/// The index pair of the first two equal items, if any.
pub(crate) fn first_duplicate(items: &[Value]) -> Option<(usize, usize)> {
    if items.len() < 2 {
        return None;
    }
    if items.len() <= 16 {
        for (i, left) in items.iter().enumerate() {
            for (offset, right) in items[i + 1..].iter().enumerate() {
                if left == right {
                    return Some((i, i + 1 + offset));
                }
            }
        }
        return None;
    }
    // Bucket by a hash consistent with `==`, then confirm within a bucket.
    // `FixedHasher` is a fixed-key hasher, so the scan is deterministic.
    let mut buckets: std::collections::BTreeMap<u64, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let mut hasher = purrdf_hash::fixed::FixedHasher::default();
        item.hash(&mut hasher);
        let bucket = buckets.entry(hasher.finish()).or_default();
        if let Some(&first) = bucket.iter().find(|&&earlier| items[earlier] == *item) {
            return Some((first, index));
        }
        bucket.push(index);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::json::Object;

    fn json(text: &str) -> Value {
        purrdf_lex::json::read(text).expect("JSON")
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
        // Member order and number spelling never distinguish two items.
        large.push(json(r#"{"a": 1, "b": 2}"#));
        large.push(json(r#"{"b": 2.0, "a": 1e0}"#));
        assert_eq!(first_duplicate(&large), Some((7, 40)));
        large.remove(40);
        assert_eq!(first_duplicate(&large), Some((41 - 1, 41)));
    }
}
