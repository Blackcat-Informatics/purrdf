// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C OWL 2 conformance: the vendored corpus, graded against PurRDF's
//! `OWL-Direct` ALCOIQ tableau.
//!
//! # Read the row correctly
//!
//! This corpus is **consistency**-shaped. All 261 vendored cases are
//! `otest:ConsistencyTest` (226) or `otest:InconsistencyTest` (35); there is not
//! one `otest:PositiveEntailmentTest` or `otest:NegativeEntailmentTest` in it,
//! because the upstream W3C material it was flattened from contains none.
//!
//! So it validates **the DL / tableau lane's verdicts** and nothing else. It does
//! **not** validate the OWL 2 RL rule table — that lane is a forward
//! materialization chase and is covered by authored per-rule fixtures in
//! `purrdf-entail`. The conformance matrix's `Entailment` row is fed from here
//! and must be read as "open-world DL consistency", not "rule coverage".
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

/// Exactly what was vendored. A case directory silently dropped or added on a
/// re-vendor would otherwise change the measured totals with nothing failing.
const EXPECTED_CASES: usize = 261;
/// The published-verdict split of the vendored corpus: `otest:ConsistencyTest`
/// and `otest:InconsistencyTest` respectively.
const EXPECTED_CONSISTENT: usize = 226;
/// See [`EXPECTED_CONSISTENT`].
const EXPECTED_INCONSISTENT: usize = 35;

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
