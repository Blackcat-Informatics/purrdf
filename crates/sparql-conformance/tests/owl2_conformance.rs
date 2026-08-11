// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C OWL 2 conformance: the vendored corpus, graded against PurRDF's
//! `OWL-Direct` SHOIQ(D) tableau.
//!
//! # Read the row correctly
//!
//! This corpus is **consistency**-shaped. All 261 vendored cases are
//! `otest:ConsistencyTest` (226) or `otest:InconsistencyTest` (35); there is not
//! one `otest:PositiveEntailmentTest` or `otest:NegativeEntailmentTest` in it,
//! because none was vendored into it. The upstream W3C material has 206 positive
//! and 23 negative entailment tests — they are vendored and graded by
//! `owl2_rl_conformance.rs`.
//!
//! So it validates **the DL / tableau lane's verdicts** and nothing else. It does
//! **not** validate the OWL 2 RL rule table — that lane is a forward
//! materialization chase, graded against W3C's entailment tests by
//! `owl2_rl_conformance.rs`. The conformance matrix's `Entailment` row is fed from
//! here and must be read as "open-world DL consistency", not "rule coverage".
//!
//! It is also a **subset**: 261 of the 482 consistency-shaped cases upstream. The
//! harness prints the exclusion tally next to the scoreboard and pins it with
//! `EXPECTED_*_EXCLUDED` constants below, so the 221 left out — and in particular
//! the ones the tableau cannot decide — cannot become invisible.
//!
//! # What fails this harness
//!
//! Nothing is skipped at discovery time. Every case runs, and a divergence is
//! only tolerated if `purrdf_sparql_conformance::owl2::LEDGER` names it with a
//! typed reason. Three things are hard errors:
//!
//! * an **unledgered divergence** — a withhold or a wrong verdict with no entry;
//! * a **stale ledger entry** — a ledgered case that now agrees, which must be
//!   removed so the gain is locked in;
//! * a **ledger entry for a case that no longer exists**, which would otherwise
//!   inflate the matrix budget invisibly.
//!
//! Regenerate the ledger after a re-vendor with:
//!
//! ```sh
//! cargo test -p purrdf-sparql-conformance --locked --test owl2_conformance -- \
//!     --ignored --nocapture regenerate_ledger
//! ```

use purrdf_sparql_conformance::owl2::{self, Verdict};
use purrdf_sparql_conformance::owl2_rl;

/// Exactly what was vendored. A case directory silently dropped or added on a
/// re-vendor would otherwise change the measured totals with nothing failing.
const EXPECTED_CASES: usize = 261;
/// The published-verdict split of the vendored corpus: `otest:ConsistencyTest`
/// and `otest:InconsistencyTest` respectively.
const EXPECTED_CONSISTENT: usize = 226;
/// See [`EXPECTED_CONSISTENT`].
const EXPECTED_INCONSISTENT: usize = 35;

/// Consistency-shaped upstream cases this corpus does NOT vendor.
const EXPECTED_EXCLUDED: usize = 221;
/// …of which PurRDF's tableau does not terminate on. These are the cases the
/// reasoner genuinely cannot decide; they are pinned here so their number is a
/// checked fact on the scoreboard rather than a claim in a document. Zero as of
/// the most recent probe: the search's whole-TBox clausification and
/// refinement made every previously non-terminating case resolve — some
/// decide outright, the rest reach the search's own budget and answer
/// `budget-exhausted` rather than exhausting the wall-clock ceiling — so this
/// is the tightest this count can read without a new non-terminating case
/// entering the excluded set.
const EXPECTED_EXCLUDED_NON_TERMINATING: usize = 0;

/// Grade the whole vendored corpus and emit the matrix scoreboard line.
#[test]
fn owl2_dl_consistency_conformance() {
    let summary = owl2::run(&owl2::suite_root()).expect("grade the vendored W3C OWL 2 corpus");

    let (consistent, inconsistent) = summary.by_published();
    eprintln!(
        "[w3c-owl2] {consistent} consistency + {inconsistent} inconsistency cases (DL/tableau \
         lane only; this corpus does NOT exercise the OWL 2 RL rule table)"
    );
    eprintln!(
        "[w3c-owl2] ledgered gaps by construct:{}",
        summary.ledger_tally()
    );
    eprintln!("{}", summary.scoreboard_line());

    // The exclusions, next to the score, so the denominator is never read as
    // "what W3C published".
    let (excluded, non_terminating) =
        owl2::exclusions(&owl2_rl::suite_root()).expect("tally the DL corpus's exclusions");
    eprintln!(
        "[w3c-owl2] this corpus vendors {} of the {} consistency-shaped cases upstream; the other \
         {} are NOT graded here",
        summary.cases.len(),
        summary.cases.len() + excluded.total,
        excluded.total
    );
    eprintln!(
        "[w3c-owl2] of those {}: {} the tableau could not decide when probed (non-terminating \
         under the ceiling), {} it decided when probed, {} it withheld on, {} carry no RDF/XML \
         premise — a recorded measurement read from census.tsv's dl_probe column, not a live run",
        excluded.total,
        excluded.non_terminating,
        excluded.decides,
        excluded.withholds,
        excluded.no_rdfxml_premise
    );
    for case in &non_terminating {
        eprintln!("[w3c-owl2]   non-terminating: {case}");
    }
    eprintln!("{}", excluded.scoreboard_line());
    assert_eq!(
        (excluded.total, excluded.non_terminating),
        (EXPECTED_EXCLUDED, EXPECTED_EXCLUDED_NON_TERMINATING),
        "the DL corpus's exclusion set changed; a case may not leave or enter it without the \
         census, this constant and the corpus PROVENANCE.md moving together"
    );

    assert!(
        summary.unledgered().is_empty() && summary.stale().is_empty(),
        "W3C OWL 2 conformance failed:\n{}",
        summary.failure_report()
    );
}

/// Inventory tripwire: the vendored corpus must keep its exact shape.
#[test]
fn w3c_owl2_inventory() {
    let root = owl2::suite_root();
    let cases = owl2::discover(&root).expect("discover the vendored W3C OWL 2 corpus");
    assert_eq!(
        cases.len(),
        EXPECTED_CASES,
        "the vendored W3C OWL 2 corpus changed size; re-vendor deliberately and update \
         EXPECTED_CASES, the freeze manifest, and the matrix budget together"
    );
    let consistent = cases
        .iter()
        .filter(|c| c.published == Verdict::Consistent)
        .count();
    assert_eq!(
        (consistent, cases.len() - consistent),
        (EXPECTED_CONSISTENT, EXPECTED_INCONSISTENT),
        "the vendored corpus's published-verdict split changed"
    );
    for case in &cases {
        assert!(
            case.premise.is_file(),
            "case {} lost its premise ontology",
            case.name
        );
    }
}

/// Every ledgered case must still be a vendored case, and the ledger must not
/// name a case twice. `owl2::run` enforces the first of these too; asserting it
/// standalone keeps the diagnosis obvious when a re-vendor drops a case.
#[test]
fn ledger_names_only_vendored_cases() {
    let cases = owl2::discover(&owl2::suite_root()).expect("discover");
    for entry in owl2::LEDGER {
        assert!(
            cases.iter().any(|c| c.name == entry.case),
            "LEDGER names {:?} ({}), which is not a vendored case",
            entry.case,
            entry.gap.label()
        );
    }
}

/// Regeneration path: print a paste-ready `LEDGER` skeleton for the measured
/// divergences. Ignored by default because it is a maintenance action, not an
/// assertion — it is how a re-vendor's new divergence set is written down.
#[test]
#[ignore = "regeneration path: run explicitly after a re-vendor"]
fn regenerate_ledger() {
    let summary = owl2::run(&owl2::suite_root()).expect("grade the vendored W3C OWL 2 corpus");
    println!("{}", summary.scoreboard_line());
    println!("{}", owl2::render_ledger_skeleton(&summary));
}
