// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered association lists: a `[(K, V)]` whose order is part of the value, read
//! by FIRST match and written by replacing the first match in place or appending.
//!
//! A JSON object's members, a GTS graph's metadata and blob tables and a SHACL
//! node expression's argument scope are all such lists: the order the document
//! wrote is the order they keep, so they are not maps, and each is searched
//! linearly because it is short. These are the one lookup and the one
//! replace-or-append every one of them uses, so a first match means the same
//! thing wherever a list of pairs is read.
//!
//! ```rust
//! use purrdf_lex::assoc;
//!
//! let mut pairs = vec![("a".to_owned(), 1), ("b".to_owned(), 2), ("a".to_owned(), 3)];
//! assert_eq!(assoc::get(&pairs, "a"), Some(&1));
//! assert_eq!(assoc::insert(&mut pairs, "a".to_owned(), 10), Some(1));
//! assert_eq!(assoc::insert(&mut pairs, "c".to_owned(), 4), None);
//! assert_eq!(pairs.len(), 4);
//! ```

use core::borrow::Borrow;
use core::mem;

/// The value of the first pair whose key is `key`, or `None`.
pub fn get<'a, K, V, Q>(pairs: &'a [(K, V)], key: &Q) -> Option<&'a V>
where
    K: Borrow<Q>,
    Q: PartialEq + ?Sized,
{
    pairs
        .iter()
        .find(|(candidate, _)| candidate.borrow() == key)
        .map(|(_, value)| value)
}

/// [`get`], mutably.
pub fn get_mut<'a, K, V, Q>(pairs: &'a mut [(K, V)], key: &Q) -> Option<&'a mut V>
where
    K: Borrow<Q>,
    Q: PartialEq + ?Sized,
{
    pairs
        .iter_mut()
        .find(|(candidate, _)| candidate.borrow() == key)
        .map(|(_, value)| value)
}

/// Set the first pair keyed `key` to `value` in place, returning the value it
/// replaces, or append the pair when there is none: the list keeps its order and
/// its length grows only for a new key.
pub fn insert<K: PartialEq, V>(pairs: &mut Vec<(K, V)>, key: K, value: V) -> Option<V> {
    match get_mut(pairs, &key) {
        Some(slot) => Some(mem::replace(slot, value)),
        None => {
            pairs.push((key, value));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{get, get_mut, insert};

    #[test]
    fn get_answers_the_first_pair_with_the_key() {
        let pairs = [("k", 1), ("j", 2), ("k", 3)];
        assert_eq!(get(&pairs, &"k"), Some(&1));
        assert_eq!(get(&pairs, &"j"), Some(&2));
        assert_eq!(get(&pairs, &"absent"), None);
        assert_eq!(get::<&str, i32, &str>(&[], &"k"), None);
    }

    #[test]
    fn get_reads_owned_keys_through_their_borrowed_form() {
        let pairs = vec![("k".to_owned(), 1)];
        assert_eq!(get(&pairs, "k"), Some(&1));
    }

    #[test]
    fn get_mut_edits_the_first_pair_only() {
        let mut pairs = vec![("k", 1), ("k", 2)];
        *get_mut(&mut pairs, &"k").expect("present") = 10;
        assert_eq!(pairs, [("k", 10), ("k", 2)]);
        assert!(get_mut(&mut pairs, &"absent").is_none());
    }

    #[test]
    fn insert_replaces_in_place_or_appends() {
        let mut pairs = vec![("a", 1), ("b", 2), ("a", 3)];
        assert_eq!(insert(&mut pairs, "a", 10), Some(1));
        assert_eq!(pairs, [("a", 10), ("b", 2), ("a", 3)]);
        assert_eq!(insert(&mut pairs, "c", 4), None);
        assert_eq!(pairs, [("a", 10), ("b", 2), ("a", 3), ("c", 4)]);
    }
}
