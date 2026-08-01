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

use purrdf_entail::UndecidedReason;
use purrdf_sparql_conformance::owl2_rl::{self, Answer, Direction, EntailmentOutcome};

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
    // The lane split, made EXECUTABLE rather than derived. Deriving it from an empty
    // ledger is subtracting zero from fifty; printing which mechanism reached each
    // agreement is the part that is not trivially true.
    eprintln!("{}", summary.mechanism_line());
    // …and what the negative lane's own number is MADE OF. `negative 23/23` reads as
    // twenty-three of one thing and is three decided refutations plus twenty named
    // admissions, which a reader cannot tell from the total.
    eprintln!("{}", summary.negative_lane_line());

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

    // EVERY vendored document, not the case documents alone. Sweeping `[premise, target]`
    // would have stopped covering the payload the moment one arrived anywhere else, which
    // is what `imports/` is.
    let documents =
        owl2_rl::vendored_documents(&owl2_rl::suite_root()).expect("sweep the vendored payload");
    let cases = owl2_rl::discover(&owl2_rl::suite_root()).expect("discover the vendored corpus");
    assert!(
        documents.len() > cases.len() * 2,
        "the sweep must reach past the {} case documents; it found {}",
        cases.len() * 2,
        documents.len()
    );
    let mut with_base = 0_usize;
    let mut absolute_only = 0_usize;
    for path in &documents {
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
    eprintln!(
        "[w3c-owl2-rl] base independence: {} documents swept — {with_base} declare xml:base, \
         {absolute_only} use only absolute IRIs",
        documents.len()
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

/// The corpus's cases that answer `Undecided`, named — and why each is entitled to.
///
/// Theorem PR1's completeness half is conditional on the premise being an OWL 2 RL
/// ontology, and this corpus does **not** consist only of such premises: W3C's
/// `otest:profile RL` tag selects on the case, and six of the fifty carry a premise
/// the RL grammar excludes. That fact is measured here rather than assumed anywhere,
/// because it is the fact that makes `Answer::Undecided` reachable at all — without
/// it the distinction between "refuted" and "not refutable" would be untested and
/// could rot into a synonym.
///
/// FIVE of those six answer `Undecided(PremiseOutsideRl)`, and they are pinned by name
/// so a re-vendor that changes the set has to say so. All five are NEGATIVE cases, where
/// the graded claim is soundness — the closure was computed and does not contain the
/// non-conclusion — which `Undecided` reports in full, so they agree without a ledger
/// entry.
///
/// The sixth, `new-feature-reflexiveproperty-001`, is POSITIVE and is **not** here:
/// its premise is still outside the RL syntax, and its conclusion is established
/// POSITIVELY from `owl:ReflexiveProperty`'s semantic condition rather than by a
/// match, which needs no completeness theorem and therefore no profile membership.
/// That is the distinction this test now also pins: being outside the syntax costs a
/// case the ability to REFUTE, not the ability to be entailed.
///
/// The filter is `PremiseOutsideRl` specifically, and narrowing it is a claim rather than
/// a convenience. Theorem PR1's hypothesis has a CONCLUSION-side half too, and a lane may
/// additionally recognize a construct and decline it; both also answer `Undecided`, both
/// are about the question rather than about the premise's syntax, and
/// `every_negative_case_is_answered_under_an_unexhausted_certificate` is where their split
/// is pinned. A filter that lumped all three together would report a case moving between
/// two different reasons as no change at all.
#[test]
fn the_non_rl_premises_are_named_and_answer_undecided() {
    /// Every vendored case whose PREMISE is outside the OWL 2 RL syntax, measured.
    const OUTSIDE_RL: [&str; 5] = [
        "new-feature-keys-007",
        "webont-description-logic-209",
        "webont-equivalentclass-005",
        "webont-i5-8-005",
        "webont-somevaluesfrom-002",
    ];

    let root = owl2_rl::suite_root();
    let cases = owl2_rl::discover(&root).expect("discover the corpus");
    let imports = owl2_rl::vendored_imports(&root).expect("read the vendored support documents");
    let mut measured: Vec<&str> = Vec::new();
    for case in &cases {
        let Answer::Undecided(reason) = owl2_rl::decide(case, &imports) else {
            continue;
        };
        let ledgered = owl2_rl::ledger_lookup(&case.name).is_some();
        assert_eq!(
            case.direction == Direction::Positive,
            ledgered,
            "{}: an Undecided on a POSITIVE case is a capability gap and must be ledgered; on a \
             NEGATIVE case it is the soundness observation the lane grades and must not be",
            case.name
        );
        if matches!(reason, UndecidedReason::PremiseOutsideRl(_)) {
            measured.push(&case.name);
        }
    }
    assert_eq!(
        measured, OUTSIDE_RL,
        "the set of premises outside the OWL 2 RL syntax changed; a case leaving it means the \
         premise or the profile scanner moved, and a case joining it means a premise this corpus \
         could previously refute can no longer be refuted — either way, say which and why"
    );
}

/// NO case diverges from the published verdict, and the lane split is PINNED.
///
/// This is one half of what replaced `only_missing_rules_are_actionable`, which iterated
/// `LEDGER`. With that table empty it had become a tautology: "no entry is actionable" is
/// true of an empty table whatever the classifications are, so it asserted nothing about
/// the corpus it was named for. The claim an empty ledger actually encodes is the one
/// below — every one of the fifty graded cases AGREES — and it is stated over the graded
/// cases rather than over the absence of entries.
///
/// The mechanism scoreboard is pinned beside it, per lane, because that is the part of
/// `50 / 50` that is not arithmetic. The two lanes grade different clauses of Theorem
/// PR1: the positive lane grades COMPLETENESS and carries the discrimination (a reasoner
/// that derives nothing fails all 27), while the negative lane grades SOUNDNESS, which is
/// owed unconditionally and which a reasoner that derives nothing passes trivially. Which
/// mechanism reached each agreement is what says how the corpus was closed.
///
/// So is the NEGATIVE lane's composition, and pinning it three ways is deliberate. The
/// aggregate `negative 23/23` is unchanged by a case moving between a decided refutation and
/// an admission, which is exactly how twenty admissions came to be printed as though they
/// were twenty-three refutations; so this test pins the two-way count on the mechanism line,
/// the six-way split of the admissions on the negative-lane line, and the NAMES of the three
/// refutations. The names are what catches a SWAP — one case gaining a refutation while
/// another loses one leaves every count on both lines untouched.
#[test]
fn no_case_diverges_from_the_published_verdict() {
    let summary =
        owl2_rl::run(&owl2_rl::suite_root()).expect("grade the vendored W3C OWL 2 RL corpus");
    let diverged: Vec<&str> = summary
        .cases
        .iter()
        .filter(|case| !matches!(case.grade, owl2_rl::Grade::Agree))
        .map(|case| case.name.as_str())
        .collect();
    assert_eq!(
        diverged,
        [] as [&str; 0],
        "the ledger is EMPTY, which is the claim that every vendored case answers as W3C \
         published it. A case here means that claim is false and the divergence owes a typed \
         RlGap:\n{}",
        summary.failure_report()
    );
    assert_eq!(summary.agreed(), EXPECTED_CASES);
    assert_eq!(summary.ledgered(), 0);

    // The mechanism scoreboard, pinned. `strict-table 12/18` is the normative rule table
    // reaching 12 positives on its own and REPORTING the absence of 18 of the 23
    // non-conclusions; `composite 0/0` is the seventh mechanism, which no vendored case
    // needs — every case here is reached by one lane or by none.
    //
    // The five negatives NOT in the table's bucket are the ones where a lane RECOGNIZED a
    // construct of the non-conclusion and declined to read it, so the lane's own admission
    // is what the answer carries. A lane on a negative case is not an unsoundness and never
    // was one: these five are `Undecided`, not `Entailed`, and no mechanism beyond the table
    // refutes anything —
    // `every_negative_case_is_answered_under_an_unexhausted_certificate` pins that per case.
    //
    // * `refutation 8/1` — `webont-class-005` states `[ owl:complementOf #c ]` as an OPERAND
    //   of a union. The refutation lane reads a complement class only when its every other
    //   mention is `x rdf:type` it, so it recognizes the construct and declines.
    // * `freeze 1/2` — `webont-description-logic-902` and `-904` state
    //   `_:c rdfs:subClassOf _:r` between two ANONYMOUS class expressions. Freeze reads
    //   `rdfs:subClassOf` and decides it by generalisation on constants over the two NAMED
    //   terms it relates; an existential in either position is a different question.
    // * `comprehension 2/2` — `webont-i5-5-007` nests an anonymous intersection inside a
    //   union's operand list, and `webont-restriction-005` asserts a MEMBERSHIP in an
    //   anonymous restriction, which is a counting question minting the restriction does not
    //   answer. Both are refusals this lane's whitelist has always made; what changed is
    //   that they now travel as `Undecided` naming the construct instead of falling out as
    //   "not applicable" and being answered by the fall-through as a proof.
    assert_eq!(
        summary.mechanism_line(),
        "OWL2-RL-MECHANISMS: positive 27/27 negative 23/23 (refuted 3, admitted 20) \
         strict-table 12/18 refutation 8/1 freeze 1/2 comprehension 2/2 reflexivity 1/0 \
         data-range 3/0 composite 0/0 withheld 0",
        "the mechanism x PR1-clause split moved; say WHICH lane and WHICH mechanism, because \
         the aggregate 50/50 is unchanged by a case moving between two of them"
    );

    // The negative lane's composition, pinned bucket by bucket. `refuted 3` is the only part
    // of `negative 23/23` with discriminating power: a reasoner that derived nothing at all
    // would score `negative 23/23` too, with `refuted 0` and every one of its twenty-three
    // agreements sitting in an admission bucket. The six admission buckets print including
    // the three that are empty — a lane arriving in `refutation-budget` would mean a negative
    // case answered by a run that stopped, which is not a soundness observation at all.
    assert_eq!(
        summary.negative_lane_line(),
        "OWL2-RL-NEGATIVE: total 23 = refuted 3 + admitted 20 (premise-outside-rl 5, \
         conclusion-outside-rl 10, construct-not-read 5, refutation-budget 0, freeze-budget 0, \
         data-range-containment 0) + unsound 0 + withheld 0",
        "the negative lane's composition moved; `negative 23/23` is unchanged by a case moving \
         between a decided refutation and a named admission, so say WHICH case moved and why \
         its new answer is the more truthful one"
    );

    // …and WHICH three, because a swap moves no count on either line above.
    let refuted: Vec<&str> = summary
        .cases
        .iter()
        .filter(|case| {
            case.direction == Direction::Negative
                && case.disposition == owl2_rl::Disposition::Refuted
        })
        .map(|case| case.name.as_str())
        .collect();
    assert_eq!(
        refuted,
        [
            "new-feature-keys-004",
            "webont-imports-002",
            "webont-miscellaneous-301"
        ],
        "these are the negative cases whose non-entailment PurRDF actually DECIDES — both \
         halves of Theorem PR1's hypothesis hold, so the closure's silence is a proof. A case \
         leaving this list is a refutation lost; a case joining it is a refutation claimed, \
         and a claimed refutation needs the hypothesis that licenses it"
    );
}

/// Every case the ledger once held, and the mechanism that closed it — read off the
/// RENDERED report a caller actually sees.
///
/// The sixteen entries below are the whole of what [`owl2_rl::LEDGER`] used to contain. An
/// empty ledger is trivially green, so what makes the closure a claim rather than a
/// tautology is naming, per case, WHICH of the seven mechanisms answered it. Fifteen are
/// answered by one of the five that exist because no head in Tables 4–9 has the
/// conclusion's shape, and the assertion for those is exactly the plan's: not
/// `strict-table`.
///
/// The sixteenth, `webont-imports-011`, IS `strict-table`, and pinning it as such is the
/// point rather than an exception smuggled in. Its ledger entry was never a reasoning gap:
/// the upstream export carries one document per case and its premise `owl:imports` a second
/// one, so the vendored premise was not the whole premise and no reasoner could have reached
/// the conclusion from it. Vendoring the support document under `imports/` made the premise
/// whole, and the profile's own rule table then reached the conclusion in one run — which is
/// what `strict-table` says. A test that demanded a non-`strict-table` mechanism here would
/// be demanding that a corpus defect be answered by a reasoning mechanism.
///
/// The rendering is [`purrdf_validate::regime::render_reasoning_report`] — the one renderer
/// Python, WASM, the C ABI and the CLI all read a report through — so this asserts what a
/// CALLER sees rather than what an accessor returns. Beside the mechanism it requires a
/// `boundary` line: the mechanism names the semantic boundary of the rule table it crossed,
/// and the boundary lines name what the run underneath could not fully handle, and a report
/// that carried the first without the second would be describing a run it never made.
#[test]
fn every_previously_ledgered_case_names_the_mechanism_that_closed_it() {
    /// Every case `LEDGER` once held, and the mechanism that answers it now.
    const CLOSED: [(&str, &str); 16] = [
        // Eight negative conclusions — an `owl:differentFrom`, a membership in an
        // `owl:complementOf` class, an `owl:AllDifferent` collection — reached by asserting
        // the conclusion's negation into the premise and re-running the SAME table.
        ("disjointclasses-001", "refutation"),
        ("disjointclasses-003", "refutation"),
        ("new-feature-disjointdataproperties-002", "refutation"),
        ("new-feature-disjointobjectproperties-001", "refutation"),
        ("new-feature-disjointobjectproperties-002", "refutation"),
        ("new-feature-objectqcr-002", "refutation"),
        ("owl2-rl-rules-fp-differentfrom", "refutation"),
        ("owl2-rl-rules-ifp-differentfrom", "refutation"),
        // One schema axiom that abbreviates a universally quantified Horn implication.
        ("chain2trans1", "freeze"),
        // Two anonymous class expressions the RDF-Based comprehension conditions license.
        ("webont-i5-26-010", "comprehension"),
        ("webont-i5-5-005", "comprehension"),
        // One self-loop off a construct outside the OWL 2 RL syntax.
        ("new-feature-reflexiveproperty-001", "reflexivity"),
        // Three `rdfs:range` widenings decided over the XSD value spaces.
        ("webont-i5-8-006", "data-range"),
        ("webont-i5-8-008", "data-range"),
        ("webont-i5-8-009", "data-range"),
        // …and the one whose entry was a CORPUS defect rather than a reasoning gap. See this
        // test's doc for why `strict-table` is the correct — and required — answer here.
        ("webont-imports-011", "strict-table"),
    ];

    let root = owl2_rl::suite_root();
    let cases = owl2_rl::discover(&root).expect("discover the corpus");
    let imports = owl2_rl::vendored_imports(&root).expect("read the vendored support documents");
    for (name, expected) in CLOSED {
        let case = cases
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("{name} is no longer vendored"));
        let certificate = owl2_rl::certify(case, &imports)
            .unwrap_or_else(|e| panic!("{name} must answer rather than refuse: {e}"));
        assert!(
            matches!(certificate.outcome(), EntailmentOutcome::Entailed(_)),
            "{name} is a published entailment and must be reached"
        );
        assert_eq!(
            certificate.mechanism().as_str(),
            expected,
            "{name} changed the mechanism that answers it"
        );
        assert!(
            !certificate.is_budget_exhausted(),
            "{name} answered inside every budget, so the certificate must not report one \
             exhausted"
        );

        // What a CALLER sees, through the shared renderer.
        let rendered = purrdf_validate::regime::render_reasoning_report(certificate.report());
        let mechanism_line = rendered
            .lines()
            .find(|line| line.starts_with("mechanism "))
            .unwrap_or_else(|| panic!("{name}: the rendered report states no mechanism"));
        assert!(
            mechanism_line.starts_with(&format!("mechanism {expected} ")),
            "{name}: {mechanism_line}"
        );
        // …and the semantic boundary the run crossed, beside it.
        assert!(
            rendered.lines().any(|line| line.starts_with("boundary ")),
            "{name}: a report that names a mechanism and no boundary describes a run it did \
             not make:\n{rendered}"
        );
    }

    let mechanism_answered = CLOSED
        .iter()
        .filter(|(_, mechanism)| *mechanism != "strict-table")
        .count();
    assert_eq!(
        mechanism_answered, 15,
        "fifteen of the sixteen are reached by a mechanism the rule table has no head for; a \
         change to that count is a change to what the profile's own table reaches"
    );
}

/// Every negative case is answered under a certificate that is NOT budget-exhausted, and
/// every one of them names its mechanism.
///
/// The negative lane is the soundness gate: deriving one of these non-conclusions would be
/// PurRDF asserting something W3C contradicts. What this pins is that the lane actually RAN
/// — a budget that ran out would produce the same "did not derive it" with none of the
/// meaning, and that is exactly the failure a corpus reporting green cannot otherwise see.
///
/// THREE of the twenty-three are decided `NotEntailed`, and the other twenty are
/// `Undecided` under one of three named reasons. The split is pinned case by case, because
/// which reason a case carries is the whole of what this library is entitled to say about
/// it.
///
/// * `NotEntailed` — the premise is inside the OWL 2 RL syntax AND the non-conclusion is an
///   assertional graph over named terms, so BOTH halves of Theorem PR1's hypothesis hold and
///   the absence of a match IS a refutation. `new-feature-keys-004` (`Peter owl:sameAs
///   StPeter`), `webont-imports-002` (`Socrates rdf:type Mortal`) and
///   `webont-miscellaneous-301` (`a first:prop "bar"`) are exactly that shape.
/// * `Undecided(PremiseOutsideRl)` — the premise breaks the first half; five cases, the same
///   five `the_non_rl_premises_are_named_and_answer_undecided` names.
/// * `Undecided(ConclusionOutsideRl)` — the NON-CONCLUSION breaks the second half: it states
///   a declaration (`Man rdf:type owl:Class`), a schema axiom (`p rdfs:range
///   xsd:unsignedByte`, `c1 owl:equivalentClass c2`), a collection cell, or an anonymous
///   class expression. No head in Tables 4–9 has any of those shapes, so the closure's
///   silence about one is the table having no rule for it and never evidence about the
///   premise. Ten cases.
/// * `Undecided(ConstructNotRead)` — a lane RECOGNIZED a construct and declined to read it:
///   a complement class nested inside a union operand, an inclusion between two anonymous
///   class expressions, a nested anonymous operand, a membership in a restriction. Five
///   cases.
///
/// The negative lane grades SOUNDNESS — the closure was computed and does not contain the
/// non-conclusion — and all twenty-three still report exactly that observation. What differs
/// between the four buckets is only what may be CLAIMED beyond it, which is why demanding
/// `NotEntailed` from all twenty-three would be demanding that this library claim a
/// completeness theorem whose hypothesis twenty of them break.
///
/// This table is the per-case form of the `OWL2-RL-NEGATIVE` scoreboard line, which reports
/// the same 3 / 5 / 10 / 5 as counts and is pinned in
/// `no_case_diverges_from_the_published_verdict`. Both exist because they fail on different
/// things: the counts catch a lane's composition changing without anyone naming a case, and
/// this table catches a case changing bucket while the counts stay put.
///
/// Eighteen report `strict-table` and five report the lane that declined. A lane appearing
/// on a negative case used to be impossible and is not any more: the five mechanisms beyond
/// the table still only ever ESTABLISH a conclusion — never refute one — and what a lane's
/// name on a negative case says now is that the lane ADMITTED it could not read the
/// question. That is checked below by requiring every non-`strict-table` negative to be an
/// `Undecided`, which is the executable form of "no mechanism beyond the table refutes".
#[test]
fn every_negative_case_is_answered_under_an_unexhausted_certificate() {
    /// Every negative case, and the answer it is entitled to: the outcome, then the reason
    /// kind (`-` for a decided refutation), then the mechanism that reports it.
    const ANSWERS: [(&str, &str, &str, &str); EXPECTED_NEGATIVE] = [
        ("new-feature-keys-004", "not-entailed", "-", "strict-table"),
        (
            "new-feature-keys-007",
            "undecided",
            "premise-outside-rl",
            "strict-table",
        ),
        (
            "new-feature-objectpropertychain-bjp-004",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        (
            "webont-allvaluesfrom-002",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        (
            "webont-class-004",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        (
            "webont-class-005",
            "undecided",
            "construct-not-read",
            "refutation",
        ),
        (
            "webont-description-logic-209",
            "undecided",
            "premise-outside-rl",
            "strict-table",
        ),
        (
            "webont-description-logic-902",
            "undecided",
            "construct-not-read",
            "freeze",
        ),
        (
            "webont-description-logic-904",
            "undecided",
            "construct-not-read",
            "freeze",
        ),
        (
            "webont-equivalentclass-005",
            "undecided",
            "premise-outside-rl",
            "strict-table",
        ),
        (
            "webont-equivalentclass-008",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        (
            "webont-i4-6-004",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        (
            "webont-i4-6-005",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        (
            "webont-i5-5-006",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        (
            "webont-i5-5-007",
            "undecided",
            "construct-not-read",
            "comprehension",
        ),
        (
            "webont-i5-8-005",
            "undecided",
            "premise-outside-rl",
            "strict-table",
        ),
        (
            "webont-i5-8-007",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        ("webont-imports-002", "not-entailed", "-", "strict-table"),
        (
            "webont-miscellaneous-301",
            "not-entailed",
            "-",
            "strict-table",
        ),
        (
            "webont-miscellaneous-302",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        (
            "webont-ontology-003",
            "undecided",
            "conclusion-outside-rl",
            "strict-table",
        ),
        (
            "webont-restriction-005",
            "undecided",
            "construct-not-read",
            "comprehension",
        ),
        (
            "webont-somevaluesfrom-002",
            "undecided",
            "premise-outside-rl",
            "strict-table",
        ),
    ];

    let root = owl2_rl::suite_root();
    let cases = owl2_rl::discover(&root).expect("discover the corpus");
    let imports = owl2_rl::vendored_imports(&root).expect("read the vendored support documents");
    let mut measured: Vec<(String, &str, &str, String)> = Vec::new();
    for case in cases.iter().filter(|c| c.direction == Direction::Negative) {
        let certificate = owl2_rl::certify(case, &imports)
            .unwrap_or_else(|e| panic!("{} must answer rather than refuse: {e}", case.name));
        assert!(
            !certificate.is_budget_exhausted(),
            "{}: a negative case answered by an exhausted budget is not a soundness \
             observation, it is a run that stopped",
            case.name
        );
        let mechanism = certificate.mechanism().as_str();
        let rendered = purrdf_validate::regime::render_reasoning_report(certificate.report());
        assert!(
            rendered.contains(&format!("\nmechanism {mechanism} ")),
            "{}: the rendered report must name the mechanism a caller is reading:\n{rendered}",
            case.name
        );
        let (outcome, reason) = match certificate.outcome() {
            EntailmentOutcome::NotEntailed(_) => {
                assert_eq!(
                    mechanism, "strict-table",
                    "{}: only the rule table has a completeness theorem, so only it can refute",
                    case.name
                );
                ("not-entailed", "-")
            }
            EntailmentOutcome::Undecided(reason) => (
                "undecided",
                match reason {
                    UndecidedReason::PremiseOutsideRl(_) => "premise-outside-rl",
                    UndecidedReason::ConclusionOutsideRl(_) => "conclusion-outside-rl",
                    UndecidedReason::ConstructNotRead { .. } => "construct-not-read",
                    other => panic!("{}: unexpected reason {other}", case.name),
                },
            ),
            EntailmentOutcome::Entailed(_) => panic!(
                "{}: W3C published a NEGATIVE entailment and the OWL-RL lane reached it — the \
                 rule table is UNSOUND",
                case.name
            ),
        };
        measured.push((case.name.clone(), outcome, reason, mechanism.to_owned()));
    }
    let expected: Vec<(String, &str, &str, String)> = ANSWERS
        .iter()
        .map(|(name, outcome, reason, mechanism)| {
            (
                (*name).to_owned(),
                *outcome,
                *reason,
                (*mechanism).to_owned(),
            )
        })
        .collect();
    assert_eq!(
        measured, expected,
        "the negative lane's split between a proven refutation and a NAMED admission moved; \
         say which case, which reason and why the new answer is the more truthful one"
    );
    assert_eq!(measured.len(), EXPECTED_NEGATIVE);
}

/// The committed golden render of one case's report — the bytes a caller sees.
fn mechanism_golden_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens/entailment/disjointclasses-001.report")
}

/// The rendered report of `disjointclasses-001`, byte for byte.
///
/// The two tests above assert PROPERTIES of a rendering — that it names this mechanism,
/// that it carries a boundary. A property holds of many strings, and the one thing a
/// property cannot tell anyone is what the report actually LOOKS like: the field order, the
/// spelling of the mechanism, the fact that its reason travels on the same line. So one case
/// is committed whole.
///
/// `disjointclasses-001` is the case chosen because it is the refutation lane's plainest
/// shape — two disjoint classes, a shared instance, a conclusion the rule table has no head
/// for — and because its report exercises everything the grammar can carry short of an
/// inconsistency: the extension line, seven fired rules, a boundary, the mechanism, all
/// three budget coordinates, the contract hash, and both `none` lines.
///
/// It is byte-stable by construction: every number in it is a count rather than a clock
/// reading, and the contract hash is a digest of the CALCULUS rather than of a run. A diff
/// here therefore means one of three things — the rendering moved, the calculus moved, or
/// this case's chase does different work than it did — and all three are things to be told
/// about rather than to absorb. Regenerate with `regenerate_mechanism_golden`.
#[test]
fn the_mechanism_golden_render_matches() {
    let root = owl2_rl::suite_root();
    let cases = owl2_rl::discover(&root).expect("discover the corpus");
    let imports = owl2_rl::vendored_imports(&root).expect("read the vendored support documents");
    let case = cases
        .iter()
        .find(|c| c.name == "disjointclasses-001")
        .expect("disjointclasses-001 is vendored");
    let certificate = owl2_rl::certify(case, &imports).expect("answers");
    let rendered = purrdf_validate::regime::render_reasoning_report(certificate.report());
    let golden = std::fs::read_to_string(mechanism_golden_path()).expect("read the golden");
    assert_eq!(
        rendered, golden,
        "the rendered report moved; if that is intended, regenerate the golden with:\n  cargo \
         test -p purrdf-sparql-conformance --locked --test owl2_rl_conformance -- --ignored \
         regenerate_mechanism_golden"
    );
    // …and the line the golden exists for is in it, so a regeneration that lost the
    // mechanism could not be committed green.
    assert!(
        golden.contains("\nmechanism refutation NO HEAD IN TABLES 4-9 IS A NEGATIVE FACT."),
        "the golden must carry the mechanism line and the semantic boundary it names"
    );
}

/// Regeneration path for [`the_mechanism_golden_render_matches`]. Ignored by default
/// because it WRITES the committed artifact.
#[test]
#[ignore = "regeneration path: writes the committed golden render"]
fn regenerate_mechanism_golden() {
    let root = owl2_rl::suite_root();
    let cases = owl2_rl::discover(&root).expect("discover the corpus");
    let imports = owl2_rl::vendored_imports(&root).expect("read the vendored support documents");
    let case = cases
        .iter()
        .find(|c| c.name == "disjointclasses-001")
        .expect("disjointclasses-001 is vendored");
    let certificate = owl2_rl::certify(case, &imports).expect("answers");
    let path = mechanism_golden_path();
    std::fs::write(
        &path,
        purrdf_validate::regime::render_reasoning_report(certificate.report()),
    )
    .expect("write the golden");
    println!("wrote {}", path.display());
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
