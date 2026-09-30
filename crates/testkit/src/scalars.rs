// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Boundary sweeps of a scanner over Unicode scalars: the corpus read off a
//! membership predicate, and the three-valued check of where a token stops.
//!
//! A range table can only be wrong at a boundary, so a sweep's corpus is
//! `lo - 1`, `lo`, `hi`, `hi + 1` of every range a predicate admits —
//! [`derived_ranges`], read off the predicate itself rather than any table it is
//! implemented with. [`run`] then drops each candidate into each [`Sweep`]'s
//! probe and asserts the reading the production names: one token where the
//! production admits the candidate, the separated reading where the candidate is
//! a separator the position allows, and anything but the joined reading
//! otherwise. The predicates themselves stay with the caller: this crate depends
//! on no grammar.

use std::fmt::Debug;

/// Every Unicode scalar value, in order.
pub fn all_scalars() -> impl Iterator<Item = char> {
    (0..=0x0010_FFFF_u32).filter_map(char::from_u32)
}

/// The contiguous scalar ranges `admits` accepts, as inclusive `(lo, hi)` code
/// points, read off the **predicate** rather than off any table it happens to be
/// implemented with.
///
/// The surrogate gap is treated as a non-member run, so a class that spans it
/// shows here as two ranges.
pub fn derived_ranges(admits: fn(char) -> bool) -> Vec<(u32, u32)> {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for cp in 0..=0x0010_FFFF_u32 {
        if !char::from_u32(cp).is_some_and(admits) {
            continue;
        }
        match ranges.pop() {
            Some((lo, hi)) if hi + 1 == cp => ranges.push((lo, cp)),
            Some(previous) => {
                ranges.push(previous);
                ranges.push((cp, cp));
            }
            None => ranges.push((cp, cp)),
        }
    }
    ranges
}

/// The nine scalars `[18t] IRIREF` excludes by name, beyond its `#x00-#x20`
/// range.
pub const IRIREF_DELIMITERS: [char; 9] = ['<', '>', '"', '{', '}', '|', '^', '`', '\\'];

/// `[18t] IRIREF`'s content class, transcribed from the production rather than
/// from any scanner: everything above `#x20` that is not one of the nine
/// [`IRIREF_DELIMITERS`]. Non-ASCII is never forbidden raw, U+00A0 and U+007F
/// included. Turtle, SPARQL and ShExC share the production, so every scanner of
/// it is swept against this one oracle.
pub fn iriref_content(c: char) -> bool {
    u32::from(c) > 0x20 && !IRIREF_DELIMITERS.contains(&c)
}

/// A boxed per-candidate function of a [`Sweep`].
pub type PerScalar<T> = Box<dyn Fn(char) -> T>;

/// One production, swept at one position. `R` is what the scanner under test
/// made of a probe: its tokens, or whatever else it observably produces.
pub struct Sweep<R> {
    /// The production and position, cited by grammar number.
    pub cited: &'static str,
    /// The probe text a candidate scalar is dropped into.
    pub probe: PerScalar<String>,
    /// Whether the production admits the candidate **at that position**.
    pub admits: PerScalar<bool>,
    /// The joined reading an admitted candidate must produce.
    pub joined: PerScalar<R>,
    /// The reading a separator candidate must produce, where the position has
    /// one. `None` where the separator is what the production itself admits.
    pub separated: Option<PerScalar<R>>,
}

impl<R> Debug for Sweep<R> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Sweep")
            .field("cited", &self.cited)
            .finish_non_exhaustive()
    }
}

/// Drive every sweep over every candidate through `scan`, returning the number
/// of probes run. `separates` is the grammar's separator class (its `WS`).
///
/// Per probe the claim is three-valued: a candidate the production admits must
/// read as the joined reading; a separator, where the position has a separated
/// reading, must read as exactly that; any other candidate must NOT read as the
/// joined reading — it was not silently absorbed. The third case never pins an
/// error: which diagnostic fires is an accident of where the split landed.
pub fn run<R: PartialEq + Debug>(
    scan: fn(&str) -> R,
    sweeps: &[Sweep<R>],
    scalars: &[char],
    separates: fn(char) -> bool,
) -> usize {
    let mut cases = 0;
    for sweep in sweeps {
        let cited = sweep.cited;
        for &c in scalars {
            let probe = (sweep.probe)(c);
            let observed = scan(&probe);
            let joined = (sweep.joined)(c);
            if (sweep.admits)(c) {
                assert_eq!(
                    observed,
                    joined,
                    "U+{:04X} is admitted by {cited}, so {probe:?} reads as one",
                    u32::from(c)
                );
            } else if separates(c)
                && let Some(separated) = sweep.separated.as_ref()
            {
                assert_eq!(
                    observed,
                    separated(c),
                    "U+{:04X} is `WS`, so it separates in {probe:?}",
                    u32::from(c)
                );
            } else {
                assert_ne!(
                    observed,
                    joined,
                    "U+{:04X} is not admitted by {cited}, so {probe:?} must not absorb it",
                    u32::from(c)
                );
            }
            cases += 1;
        }
    }
    cases
}

#[cfg(test)]
mod tests {
    use super::{all_scalars, derived_ranges, iriref_content};

    #[test]
    fn ranges_are_read_off_the_predicate_and_split_at_the_surrogate_gap() {
        assert_eq!(derived_ranges(|c| c.is_ascii_digit()), [(0x30, 0x39)]);
        assert_eq!(
            derived_ranges(|c| ('\u{d7ff}'..='\u{e000}').contains(&c)),
            [(0xd7ff, 0xd7ff), (0xe000, 0xe000)]
        );
        assert_eq!(derived_ranges(|_| false), []);
    }

    #[test]
    fn every_scalar_is_enumerated_once() {
        assert_eq!(all_scalars().count(), 0x0011_0000 - 0x800);
    }

    #[test]
    fn iriref_content_excludes_exactly_the_controls_space_and_delimiters() {
        assert!(!iriref_content(' '));
        assert!(!iriref_content('<'));
        assert!(iriref_content('a'));
        assert!(iriref_content('\u{7f}'));
        assert!(iriref_content('\u{a0}'));
    }
}
