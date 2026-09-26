// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! XPath case variants, the relation the `i` flag is defined by.
//!
//! XPath F&O 3.1 §5.6.2: "a character C2 is considered to be a case-variant of
//! another character C1 if the following XPath expression returns true when
//! the two characters are considered as strings of length one, and the Unicode
//! codepoint collation is used: `fn:lower-case(C1) eq fn:lower-case(C2) or
//! fn:upper-case(C1) eq fn:upper-case(C2)`." `fn:lower-case` and
//! `fn:upper-case` are Unicode's default full case mappings (F&O 3.1 §5.4.7,
//! §5.4.8), which are exactly [`char::to_lowercase`] and
//! [`char::to_uppercase`] of a single character (no context: a character on
//! its own has none).
//!
//! The relation is neither Unicode simple case folding nor transitive, which is
//! why the Rust engine's `(?i)` cannot stand in for it: dotless `ı` is a case
//! variant of `i` (both upper-case to `I`) although neither folds to the other,
//! and the relation's closure is never taken.
//!
//! Under `i` a normal character used as an atom, and every character a
//! character range matches, "represents the set containing [it] and all its
//! case-variants"; "all other constructs are unaffected" — `\p{Lu}` keeps
//! matching upper-case letters only.

use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Every character with at least one case variant besides itself, sorted, with
/// its complete variant set (itself included, sorted).
pub(super) fn table() -> &'static [(char, Box<[char]>)] {
    static TABLE: OnceLock<Vec<(char, Box<[char]>)>> = OnceLock::new();
    TABLE.get_or_init(build)
}

/// The case variants of `c`, itself included, or `None` when `c` has none but
/// itself.
pub(super) fn of(c: char) -> Option<&'static [char]> {
    let table = table();
    table
        .binary_search_by_key(&c, |(key, _)| *key)
        .ok()
        .map(|index| &*table[index].1)
}

/// A single-character case mapping as a comparable key: the mapping's
/// characters, `None` when the mapping is the character itself.
fn mapping(c: char, lower: bool) -> Option<String> {
    let mapped: String = if lower {
        c.to_lowercase().collect()
    } else {
        c.to_uppercase().collect()
    };
    let mut chars = mapped.chars();
    if chars.next() == Some(c) && chars.next().is_none() {
        None
    } else {
        Some(mapped)
    }
}

fn build() -> Vec<(char, Box<[char]>)> {
    // `groups[lower?][mapping]` = every character whose lower- (upper-) case
    // mapping is `mapping`. Only characters whose mapping is not themselves are
    // inserted while scanning; a character that maps to itself joins its own
    // one-character key afterwards, and only when some other character
    // already maps there (otherwise it is a variant of nothing but itself).
    let mut groups: [BTreeMap<String, Vec<char>>; 2] = [BTreeMap::new(), BTreeMap::new()];
    for c in (0..=u32::from(char::MAX)).filter_map(char::from_u32) {
        for (index, lower) in [(0, true), (1, false)] {
            if let Some(key) = mapping(c, lower) {
                groups[index].entry(key).or_default().push(c);
            }
        }
    }
    for (index, lower) in [(0, true), (1, false)] {
        for (key, members) in &mut groups[index] {
            let mut chars = key.chars();
            if let (Some(only), None) = (chars.next(), chars.next())
                && mapping(only, lower).is_none()
            {
                members.push(only);
            }
        }
    }
    let group_of = |c: char, index: usize, lower: bool| -> &[char] {
        let key = mapping(c, lower).unwrap_or_else(|| c.to_string());
        groups[index].get(&key).map_or(&[], Vec::as_slice)
    };
    let mut candidates: Vec<char> = groups
        .iter()
        .flat_map(|group| group.values().flatten().copied())
        .collect();
    candidates.sort_unstable();
    candidates.dedup();
    let mut table = Vec::with_capacity(candidates.len());
    for c in candidates {
        let mut variants: Vec<char> = group_of(c, 0, true)
            .iter()
            .chain(group_of(c, 1, false))
            .copied()
            .chain(std::iter::once(c))
            .collect();
        variants.sort_unstable();
        variants.dedup();
        if variants.len() > 1 {
            table.push((c, variants.into_boxed_slice()));
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use super::of;

    #[test]
    fn variants_follow_the_xpath_definition_not_simple_folding() {
        // The specification's own examples: `z` and `Z`; KELVIN SIGN lowers to
        // `k`, so it is a variant of `K` and `k`; `Q` has only `q`.
        assert_eq!(of('z'), Some(&['Z', 'z'][..]));
        assert_eq!(of('K'), Some(&['K', 'k', '\u{212a}'][..]));
        assert_eq!(of('Q'), Some(&['Q', 'q'][..]));
        // Dotless `ı` and `i` both upper-case to `I`; simple folding omits it.
        assert!(of('i').expect("i has variants").contains(&'ı'));
        assert!(of('ı').expect("ı has variants").contains(&'i'));
        // `İ` lower-cases to two characters, so it is no variant of `i`.
        assert!(!of('i').expect("i has variants").contains(&'İ'));
        // Uncased characters have no variants.
        assert_eq!(of('7'), None);
        assert_eq!(of('-'), None);
    }

    #[test]
    fn long_s_is_a_variant_of_both_cases_of_s() {
        // `ſ` (LONG S) upper-cases to `S`, as `s` does, so the three are
        // pairwise variants by the upper-case clause alone.
        for c in ['s', 'S', 'ſ'] {
            let variants = of(c).expect("variants");
            for other in ['s', 'S', 'ſ'] {
                assert!(variants.contains(&other), "{c:?} ~ {other:?}");
            }
        }
    }
}
