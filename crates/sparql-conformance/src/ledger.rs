// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The divergence ledger both OWL 2 corpora grade against.
//!
//! [`crate::owl2`] and [`crate::owl2_rl`] keep a typed table of every vendored case
//! PurRDF does not answer as the W3C published it, and each run is scored against that
//! table the same way: an agreeing, unledgered case counts; a diverging, ledgered case is
//! an expected failure; a diverging case with no entry, or a ledgered case that now
//! agrees, fails the harness. That bookkeeping is one rule, so it has one body here, and
//! each corpus supplies only what differs — its gap type and its graded case.

use std::fmt::Write as _;

/// One ledgered divergence: the case's directory name plus its typed gap.
#[derive(Debug)]
pub struct LedgerEntry<G> {
    /// The case directory name under the corpus's `cases/` directory.
    pub case: &'static str,
    /// Why PurRDF diverges.
    pub gap: G,
}

/// The gap `case` is ledgered under in `ledger`, if it has an entry.
#[must_use]
pub fn lookup<G: Copy>(ledger: &[LedgerEntry<G>], case: &str) -> Option<G> {
    ledger
        .iter()
        .find(|entry| entry.case == case)
        .map(|entry| entry.gap)
}

/// A graded case as the ledger bookkeeping reads it.
pub trait LedgeredCase {
    /// Whether PurRDF's answer matched the published one.
    fn agrees(&self) -> bool;
    /// The case's ledger entry, as its gap's `(label, is_unsound)`, if it has one.
    fn ledgered(&self) -> Option<(&'static str, bool)>;
}

/// Cases that agreed with the published answer and are not ledgered.
#[must_use]
pub fn agreed<C: LedgeredCase>(cases: &[C]) -> usize {
    cases
        .iter()
        .filter(|case| case.agrees() && case.ledgered().is_none())
        .count()
}

/// Cases that diverged (withheld or disagreed) and are ledgered.
#[must_use]
pub fn ledgered<C: LedgeredCase>(cases: &[C]) -> usize {
    cases
        .iter()
        .filter(|case| !case.agrees() && case.ledgered().is_some())
        .count()
}

/// Cases that diverged with no ledger entry.
#[must_use]
pub fn unledgered<C: LedgeredCase>(cases: &[C]) -> Vec<&C> {
    cases
        .iter()
        .filter(|case| !case.agrees() && case.ledgered().is_none())
        .collect()
}

/// Ledgered cases that now agree: stale entries.
#[must_use]
pub fn stale<C: LedgeredCase>(cases: &[C]) -> Vec<&C> {
    cases
        .iter()
        .filter(|case| case.agrees() && case.ledgered().is_some())
        .collect()
}

/// A per-gap tally of the ledgered divergences, in label order, one `\n  NNN  label`
/// line per gap, with ` [UNSOUND]` after an unsound gap's label.
#[must_use]
pub fn tally<C: LedgeredCase>(cases: &[C]) -> String {
    let mut counts: Vec<(&'static str, usize, bool)> = Vec::new();
    for (label, unsound) in cases
        .iter()
        .filter(|case| !case.agrees())
        .filter_map(LedgeredCase::ledgered)
    {
        if let Some(slot) = counts.iter_mut().find(|(seen, _, _)| *seen == label) {
            slot.1 += 1;
        } else {
            counts.push((label, 1, unsound));
        }
    }
    counts.sort_unstable();
    let mut out = String::new();
    for (label, n, unsound) in counts {
        let mark = if unsound { " [UNSOUND]" } else { "" };
        let _ = write!(out, "\n  {n:>3}  {label}{mark}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A case: whether it agrees, and its gap as `(label, is_unsound)`.
    struct Case(bool, Option<(&'static str, bool)>);

    impl LedgeredCase for Case {
        fn agrees(&self) -> bool {
            self.0
        }
        fn ledgered(&self) -> Option<(&'static str, bool)> {
            self.1
        }
    }

    fn cases() -> Vec<Case> {
        vec![
            Case(true, None),
            Case(true, None),
            Case(false, Some(("b-gap", false))),
            Case(false, Some(("a-gap", true))),
            Case(false, Some(("b-gap", false))),
            Case(false, None),
            Case(true, Some(("a-gap", true))),
        ]
    }

    /// Every case lands in exactly one of the four columns.
    #[test]
    fn the_four_columns_partition_the_cases() {
        let cases = cases();
        assert_eq!(agreed(&cases), 2);
        assert_eq!(ledgered(&cases), 3);
        assert_eq!(unledgered(&cases).len(), 1);
        assert_eq!(stale(&cases).len(), 1);
    }

    /// The tally counts only diverging ledgered cases, in label order, and marks the
    /// unsound gap.
    #[test]
    fn the_tally_counts_diverging_entries_per_gap_in_label_order() {
        assert_eq!(tally(&cases()), "\n    1  a-gap [UNSOUND]\n    2  b-gap");
        assert_eq!(tally::<Case>(&[]), "");
    }

    /// A case resolves to its entry's gap; an absent case has none.
    #[test]
    fn lookup_finds_the_entry_by_case_name() {
        let ledger = [
            LedgerEntry {
                case: "one",
                gap: 1,
            },
            LedgerEntry {
                case: "two",
                gap: 2,
            },
        ];
        assert_eq!(lookup(&ledger, "two"), Some(2));
        assert_eq!(lookup(&ledger, "three"), None);
    }
}
