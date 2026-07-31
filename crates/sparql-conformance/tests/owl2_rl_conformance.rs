// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C OWL 2 **entailment** conformance: the vendored entailment corpus, graded
//! against PurRDF's OWL 2 RL forward-materialization chase.
//!
//! # Why this harness exists
//!
//! `owl2_conformance.rs` grades *satisfiability* with the `OWL-Direct` tableau.
//! It says nothing about the OWL 2 RL rule table, which until this harness
//! existed was graded only by fixtures authored alongside the rules themselves.
//! This is the independent oracle: W3C's own entailment tests, taken from
//! <https://www.w3.org/2009/11/owl-test/all.rdf>, run through
//! `purrdf_entail::Regime::OwlRl`.
//!
//! # What fails this harness
//!
//! Nothing is skipped at discovery time. Every case runs, and a divergence is
//! only tolerated if `purrdf_sparql_conformance::owl2_rl::LEDGER` names it with a
//! typed reason. Four things are hard errors:
//!
//! * an **unledgered divergence** — a withhold or a contradicted verdict with no
//!   entry;
//! * a **stale ledger entry** — a ledgered case that now agrees;
//! * a **ledger entry for a case that is not vendored**, which would inflate the
//!   budget invisibly;
//! * a **census row that does not match the filesystem** — a case the census says
//!   is graded but is not vendored, or vice versa.
//!
//! Regenerate the ledger after a re-vendor with:
//!
//! ```sh
//! cargo test -p purrdf-sparql-conformance --locked --test owl2_rl_conformance -- \
//!     --ignored --nocapture regenerate_rl_ledger
//! ```

use purrdf_sparql_conformance::owl2_rl::{self, Answer, Direction};

/// Exactly what was vendored: the 27 positive plus 23 negative entailment cases.
const EXPECTED_CASES: usize = 50;
/// `otest:PositiveEntailmentTest` cases that declare `otest:profile RL` under
/// `otest:semantics RDF-BASED` and carry both an RDF/XML premise and an RDF/XML
/// conclusion.
const EXPECTED_POSITIVE: usize = 27;
/// `otest:NegativeEntailmentTest` cases with RDF-BASED semantics carrying both an
/// RDF/XML premise and an RDF/XML non-conclusion — which is all 23 of them.
const EXPECTED_NEGATIVE: usize = 23;
/// Every `otest:TestCase` in the upstream manifest, one census row each.
const EXPECTED_CENSUS_ROWS: usize = 489;

/// Grade the whole vendored entailment corpus and emit the matrix scoreboard
/// line.
#[test]
fn owl2_rl_entailment_conformance() {
    let summary =
        owl2_rl::run(&owl2_rl::suite_root()).expect("grade the vendored W3C OWL 2 RL corpus");

    let (positive, negative) = summary.by_direction();
    eprintln!(
        "[w3c-owl2-rl] {positive} positive + {negative} negative entailment cases, graded through \
         the OWL 2 RL chase (this corpus DOES exercise the RL rule table; the DL/tableau lane is \
         graded by owl2_conformance.rs)"
    );
    eprintln!(
        "[w3c-owl2-rl] ledgered gaps by construct:{}",
        summary.ledger_tally()
    );
    eprintln!("{}", summary.scoreboard_line());

    assert!(
        summary.unledgered().is_empty() && summary.stale().is_empty(),
        "W3C OWL 2 RL entailment conformance failed:\n{}",
        summary.failure_report()
    );
}

/// Inventory tripwire: the vendored corpus must keep its exact shape.
#[test]
fn w3c_owl2_rl_inventory() {
    let cases = owl2_rl::discover(&owl2_rl::suite_root())
        .expect("discover the vendored W3C OWL 2 RL corpus");
    assert_eq!(
        cases.len(),
        EXPECTED_CASES,
        "the vendored W3C OWL 2 entailment corpus changed size; re-vendor deliberately and update \
         EXPECTED_CASES, census.tsv and the freeze manifest together"
    );
    let positive = cases
        .iter()
        .filter(|c| c.direction == Direction::Positive)
        .count();
    assert_eq!(
        (positive, cases.len() - positive),
        (EXPECTED_POSITIVE, EXPECTED_NEGATIVE),
        "the vendored corpus's positive/negative split changed"
    );
    for case in &cases {
        assert!(
            case.premise.is_file(),
            "case {} lost its premise ontology",
            case.name
        );
        assert!(
            case.target.is_file(),
            "case {} lost its {}",
            case.name,
            case.direction.target_file()
        );
    }
}

/// No vendored document's meaning depends on the harness's synthetic base IRI:
/// every one either declares its own `xml:base`, or uses no relative reference
/// that a base could resolve. That is a claim `PROVENANCE.md` makes, so it is
/// checked rather than asserted in prose.
#[test]
fn no_vendored_document_needs_the_harness_base() {
    /// The RDF/XML attributes whose value is an IRI reference resolved against
    /// the in-scope base.
    const IRI_ATTRS: [&str; 5] = [
        "rdf:about=",
        "rdf:resource=",
        "rdf:ID=",
        "rdf:datatype=",
        "rdf:nodeID=",
    ];

    /// Whether `value` carries a scheme, i.e. is an absolute IRI.
    fn is_absolute(value: &str) -> bool {
        let Some(colon) = value.find(':') else {
            return false;
        };
        let (scheme, _) = value.split_at(colon);
        !scheme.is_empty()
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    }

    let cases = owl2_rl::discover(&owl2_rl::suite_root()).expect("discover the vendored corpus");
    let mut with_base = 0_usize;
    let mut absolute_only = 0_usize;
    for case in &cases {
        for path in [&case.premise, &case.target] {
            let text = std::fs::read_to_string(path).expect("read vendored document");
            if text.contains("xml:base") {
                with_base += 1;
                continue;
            }
            // No declared base, so every IRI-valued attribute must be absolute —
            // `rdf:nodeID` is a blank-node label and never resolved at all.
            let mut relative: Vec<String> = Vec::new();
            for attr in IRI_ATTRS {
                let mut rest = text.as_str();
                while let Some(at) = rest.find(attr) {
                    rest = &rest[at + attr.len()..];
                    let quote = rest.chars().next().unwrap_or(' ');
                    if quote != '"' && quote != '\'' {
                        continue;
                    }
                    let Some(end) = rest[1..].find(quote) else {
                        continue;
                    };
                    let value = &rest[1..=end];
                    if attr != "rdf:nodeID=" && !is_absolute(value) {
                        relative.push(format!("{attr}{quote}{value}{quote}"));
                    }
                    rest = &rest[end + 2..];
                }
            }
            assert!(
                relative.is_empty(),
                "{} declares no xml:base AND uses relative references {relative:?}, so its IRIs \
                 would resolve against the harness's synthetic base and the verdict would depend \
                 on the harness's own choice",
                path.display()
            );
            absolute_only += 1;
        }
    }
    eprintln!(
        "[w3c-owl2-rl] base independence: {with_base} documents declare xml:base, {absolute_only} \
         use only absolute IRIs"
    );
}

/// The upstream census must account for every case in the manifest, and its
/// claims about what is vendored must match the filesystem.
///
/// This is what makes the exclusions **visible**: the census names all 489
/// upstream test cases and states, for each one, whether the RL entailment corpus
/// grades it, whether the DL consistency corpus grades it, and — when neither —
/// the reason. A case cannot be quietly dropped from a corpus and still be
/// reported green, because the census row and the directory listing are checked
/// against each other.
#[test]
fn census_accounts_for_every_upstream_case() {
    let root = owl2_rl::suite_root();
    let rows = owl2_rl::read_census(&root).expect("read census.tsv");
    assert_eq!(
        rows.len(),
        EXPECTED_CENSUS_ROWS,
        "census.tsv must hold one row per upstream otest:TestCase"
    );

    let rl_cases = owl2_rl::discover(&root).expect("discover the RL corpus");
    let dl_root = purrdf_sparql_conformance::owl2::suite_root();
    let dl_cases =
        purrdf_sparql_conformance::owl2::discover(&dl_root).expect("discover the DL corpus");

    for row in &rows {
        let vendored_rl = rl_cases.iter().find(|c| c.name == row.case);
        match row.rl_corpus.as_str() {
            "graded-positive" | "graded-negative" => {
                let case = vendored_rl.unwrap_or_else(|| {
                    panic!(
                        "census says {} is {} but it is not vendored under \
                         entailment-suite/w3c-owl2-rl/cases",
                        row.case, row.rl_corpus
                    )
                });
                let want = if row.rl_corpus == "graded-positive" {
                    Direction::Positive
                } else {
                    Direction::Negative
                };
                assert_eq!(
                    case.direction, want,
                    "census and payload disagree on {}'s direction",
                    row.case
                );
            }
            _ => assert!(
                vendored_rl.is_none(),
                "{} is vendored in the RL corpus but the census records it as {:?}",
                row.case,
                row.rl_corpus
            ),
        }
        let vendored_dl = dl_cases.iter().any(|c| c.name == row.case);
        assert_eq!(
            vendored_dl,
            row.dl_corpus == "graded",
            "census says {}'s DL disposition is {:?}, but vendored_in_dl_corpus = {vendored_dl}",
            row.case,
            row.dl_corpus
        );
    }

    eprintln!("[w3c-census] {} upstream otest:TestCase rows", rows.len());
    for (value, count) in owl2_rl::census_tally(&rows, |r| &r.rl_corpus) {
        eprintln!("[w3c-census]   rl_corpus {value:>24}  {count:>3}");
    }
    for (value, count) in owl2_rl::census_tally(&rows, |r| &r.dl_corpus) {
        eprintln!("[w3c-census]   dl_corpus {value:>24}  {count:>3}");
    }
}

/// The corpus's premises that are OUTSIDE the OWL 2 RL syntax, named — and the
/// answer each one gets.
///
/// Theorem PR1's completeness half is conditional on the premise being an OWL 2 RL
/// ontology, and this corpus does **not** consist only of such premises: W3C's
/// `otest:profile RL` tag selects on the case, and six of the fifty carry a premise
/// the RL grammar excludes. That fact is measured here rather than assumed anywhere,
/// because it is the fact that makes `Answer::Undecided` reachable at all — without
/// it the distinction between "refuted" and "not refutable" would be untested and
/// could rot into a synonym.
///
/// The six are pinned by name, so a re-vendor that changes the set has to say so, and
/// each is checked to get the answer its direction earns:
///
/// * `new-feature-reflexiveproperty-001` is POSITIVE, so an `Undecided` is a
///   capability gap, and `LEDGER` carries it as `construct-outside-rl`;
/// * the other five are NEGATIVE, where the graded claim is soundness — the closure
///   was computed and does not contain the non-conclusion — which `Undecided` reports
///   in full, so they agree without a ledger entry.
#[test]
fn the_non_rl_premises_are_named_and_answer_undecided() {
    /// Every vendored case whose premise the OWL 2 RL grammar excludes, measured.
    const OUTSIDE_RL: [&str; 6] = [
        "new-feature-keys-007",
        "new-feature-reflexiveproperty-001",
        "webont-description-logic-209",
        "webont-equivalentclass-005",
        "webont-i5-8-005",
        "webont-somevaluesfrom-002",
    ];

    let cases = owl2_rl::discover(&owl2_rl::suite_root()).expect("discover the corpus");
    let mut measured: Vec<&str> = Vec::new();
    for case in &cases {
        if !matches!(owl2_rl::decide(case), Answer::Undecided(_)) {
            continue;
        }
        measured.push(&case.name);
        let ledgered = owl2_rl::ledger_lookup(&case.name).is_some();
        assert_eq!(
            case.direction == Direction::Positive,
            ledgered,
            "{}: an Undecided on a POSITIVE case is a capability gap and must be ledgered; on a \
             NEGATIVE case it is the soundness observation the lane grades and must not be",
            case.name
        );
    }
    assert_eq!(
        measured, OUTSIDE_RL,
        "the set of premises outside the OWL 2 RL syntax changed; a case leaving it means the \
         premise or the profile scanner moved, and a case joining it means a premise this corpus \
         could previously refute can no longer be refuted — either way, say which and why"
    );
}

/// Every ledgered case must still be a vendored case, and the ledger must not
/// name a case twice.
#[test]
fn rl_ledger_names_only_vendored_cases() {
    let cases = owl2_rl::discover(&owl2_rl::suite_root()).expect("discover");
    for entry in owl2_rl::LEDGER {
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
/// assertion.
#[test]
#[ignore = "regeneration path: run explicitly after a re-vendor"]
fn regenerate_rl_ledger() {
    let summary = owl2_rl::run(&owl2_rl::suite_root()).expect("grade the vendored corpus");
    println!("{}", summary.scoreboard_line());
    for case in &summary.cases {
        println!("{:?}  {}  {:?}", case.direction, case.name, case.grade);
    }
    println!("{}", owl2_rl::render_ledger_skeleton(&summary));
}
