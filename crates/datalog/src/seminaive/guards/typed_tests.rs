// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::BTreeSet;

use purrdf_core::evidence::{Information, SearchCompleteness};
use purrdf_core::literal_comparison::{
    DigestAlgorithm, DigestDatatype, LiteralComparisonOp as Op, LiteralComparisonProfile,
    LiteralComparisonRefusal, LiteralComparisonVerdict, LiteralIncomparability, LiteralOperandSide,
};
use purrdf_core::{FastMap, RdfLiteral};
use purrdf_lex::allocation::Resident;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause};
use crate::dictionary::IntegerDictionary;
use crate::evidence::DerivationEvidence;
use crate::guard::{Comparison, Guard, Negation, NoGuards};
use crate::seminaive::{
    EvalError, EvalOptions, EvidencedOutcome, compile, evaluate, evaluate_evidenced_outcome,
};
use crate::store::RelationStore;
use crate::test_support::evidence_scope;

const TIME: &str = purrdf_core::datatype::XsdDatatype::DateTime.iri();
const STAMP: &str = purrdf_core::datatype::XsdDatatype::DateTimeStamp.iri();
const BLAKE: &str = "urn:example:blake3";
const SHA: &str = "urn:example:sha256";
const SYMBOLS: [(&str, u32); 5] = [
    ("<urn:example:pair>", 0),
    ("<urn:example:answer>", 1),
    ("<urn:example:time>", 2),
    ("<urn:example:digest>", 3),
    ("<urn:example:violation>", 4),
];

fn profile() -> LiteralComparisonProfile {
    LiteralComparisonProfile::new(vec![
        DigestDatatype {
            datatype: BLAKE.into(),
            algorithm: DigestAlgorithm::Blake3,
        },
        DigestDatatype {
            datatype: SHA.into(),
            algorithm: DigestAlgorithm::Sha256,
        },
    ])
    .unwrap()
}

fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom<3> {
    ClauseAtom::triple(
        ClauseTerm::var(subject),
        ClauseTerm::iri(format!("urn:example:{predicate}")),
        ClauseTerm::var(object),
    )
}

fn guard(left: &str, right: &str, operation: Op, negated: bool) -> Guard {
    Guard::compare_literals(
        left,
        right,
        Comparison {
            operation,
            negated,
            profile: profile(),
        },
    )
}

fn input<'a>(
    literals: &'a FastMap<u32, RdfLiteral>,
    rows: impl IntoIterator<Item = [u32; 3]>,
) -> RelationStore<IntegerDictionary<'a>, 3> {
    let dictionary = IntegerDictionary::with_symbols(&SYMBOLS)
        .unwrap()
        .with_rdf_literals(literals);
    let mut store = RelationStore::with_dictionary(dictionary);
    for row in rows {
        store.insert_ids(row).unwrap();
    }
    store
}

#[test]
fn rules_and_constraints_agree_on_truth_and_original_undetermined_operands() {
    use LiteralComparisonRefusal::{Incomparable, Invalid, Unsupported};
    use LiteralComparisonVerdict::{Decided, Undetermined};
    use LiteralIncomparability::{DigestAlgorithms, TimezoneStatus};
    use LiteralOperandSide::Right;

    let utc = RdfLiteral::typed("2026-04-04T09:12:00Z", STAMP);
    let equivalent = RdfLiteral::typed("2026-04-04T11:12:00+02:00", TIME);
    let padded = RdfLiteral::typed(" \t2026-04-04T11:12:00+02:00\r\n", TIME);
    let local = RdfLiteral::typed("2026-01-01T00:00:00", TIME);
    let bad_time = RdfLiteral::typed("2026-04-04T09:12:00", STAMP);
    let large_time = RdfLiteral::typed("9223372036854775807-01-01T00:00:00Z", STAMP);
    let unrepresented_time = RdfLiteral::typed("9223372036854775808-01-01T00:00:00Z", STAMP);
    let non_xml_space = RdfLiteral::typed("\u{a0}2026-04-04T09:12:00Z", STAMP);
    let low = RdfLiteral::typed("a1".repeat(32), BLAKE);
    let upper = RdfLiteral::typed("A1".repeat(32), BLAKE);
    let high = RdfLiteral::typed("b0".repeat(32), BLAKE);
    let other_algorithm = RdfLiteral::typed("A1".repeat(32), SHA);
    let bad_digest = RdfLiteral::typed("a1".repeat(31), BLAKE);
    let opaque = RdfLiteral {
        lexical_form: "opaque".into(),
        datatype: Some("urn:example:not-a-digest".into()),
        language: Some("en-US".into()),
        direction: None,
    };
    for (left, right, op, positive) in [
        (&utc, &equivalent, Op::Equal, Decided(true)),
        (&utc, &padded, Op::Equal, Decided(true)),
        (&utc, &equivalent, Op::Less, Decided(false)),
        (&utc, &equivalent, Op::LessOrEqual, Decided(true)),
        (
            &local,
            &utc,
            Op::Less,
            Undetermined(Incomparable(TimezoneStatus)),
        ),
        (&utc, &bad_time, Op::Equal, Undetermined(Invalid(Right))),
        (
            &utc,
            &non_xml_space,
            Op::Equal,
            Undetermined(Invalid(Right)),
        ),
        (&utc, &large_time, Op::Less, Decided(true)),
        (
            &utc,
            &unrepresented_time,
            Op::Less,
            Undetermined(Unsupported(Right)),
        ),
        (&low, &high, Op::Less, Decided(true)),
        (&low, &upper, Op::Equal, Decided(true)),
        (&low, &upper, Op::Less, Decided(false)),
        (&low, &upper, Op::LessOrEqual, Decided(true)),
        (
            &low,
            &other_algorithm,
            Op::Equal,
            Undetermined(Incomparable(DigestAlgorithms)),
        ),
        (&low, &bad_digest, Op::Less, Undetermined(Invalid(Right))),
        (&low, &opaque, Op::Equal, Undetermined(Unsupported(Right))),
    ] {
        let literals = [(10, left.clone()), (11, right.clone())]
            .into_iter()
            .collect();
        for negated in [false, true] {
            let expected = if negated {
                positive.negated()
            } else {
                positive
            };
            let body = vec![atom("left", "pair", "right")];
            let comparison = guard("left", "right", op, negated);
            let rule = DlClause::datalog(atom("left", "answer", "right"), body.clone())
                .with_guards(vec![comparison.clone()]);
            let constraint = DlClause::inconsistency(body)
                .with_guards(vec![comparison])
                .with_constraint_witness(atom("left", "violation", "right"))
                .unwrap();
            for (program, relation) in [(rule, 1), (constraint, 4)] {
                let executable = compile(vec![program]).unwrap();
                let mut scope = evidence_scope();
                scope.closure_declared = relation != 4;
                let result = evaluate_evidenced_outcome(
                    &executable,
                    input(&literals, [[10, 0, 11]]),
                    &EvalOptions::default(),
                    &NoGuards,
                    None,
                    DerivationEvidence::with_admission(1, Resident),
                    scope,
                )
                .unwrap();
                match expected {
                    Decided(keep) => {
                        let EvidencedOutcome::Complete(result) = result else {
                            panic!("expected a complete decision, got {result:?}");
                        };
                        assert_eq!(
                            result
                                .evaluation
                                .facts()
                                .rows_from(0)
                                .any(|(_, row)| row == [10, relation, 11]),
                            keep,
                        );
                    }
                    Undetermined(reason) => {
                        let EvidencedOutcome::Undetermined(result) = result else {
                            panic!("expected typed undetermined evidence, got {result:?}")
                        };
                        assert_eq!(result.rule, 0);
                        let record = result.search_record().unwrap();
                        assert_eq!(record.completeness(), SearchCompleteness::Undetermined);
                        assert_eq!(record.information().fallback(), Information::Neither);
                        let evidence = result.comparison;
                        assert_eq!(evidence.reason, reason);
                        assert_eq!(evidence.operation, op);
                        assert_eq!(evidence.negated, negated);
                        assert_eq!(evidence.operands, [Some(left.clone()), Some(right.clone())]);
                    }
                }
            }
        }
    }
}

#[test]
fn comparison_without_typed_dictionary_is_a_configuration_failure() {
    let rule = DlClause::datalog(
        atom("left", "answer", "right"),
        vec![atom("left", "pair", "right")],
    )
    .with_guards(vec![guard("left", "right", Op::Less, false)]);
    let executable = compile(vec![rule]).unwrap();
    let mut untyped =
        RelationStore::with_dictionary(IntegerDictionary::with_symbols(&SYMBOLS).unwrap());
    untyped.insert_ids([10, 0, 11]).unwrap();
    assert!(matches!(
        evaluate(&executable, untyped),
        Err(EvalError::ComparisonDictionaryUnavailable { .. })
    ));
    let literals = [
        (10, RdfLiteral::typed("2026-01-01T00:00:00Z", STAMP)),
        (11, RdfLiteral::typed("2026-02-01T00:00:00Z", STAMP)),
    ]
    .into_iter()
    .collect();
    let proven = evaluate(&executable, input(&literals, [[10, 0, 11]])).unwrap();
    assert!(
        proven
            .facts()
            .rows_from(0)
            .any(|(_, row)| row == [10, 1, 11])
    );
}

#[test]
fn refusal_inside_negative_conjunction_cannot_prove_absence() {
    let negative = Negation::new(
        vec![atom("left", "time", "candidate")],
        vec![guard("left", "candidate", Op::Less, false)],
    );
    let rule = DlClause::datalog(
        atom("left", "answer", "right"),
        vec![atom("left", "pair", "right")],
    )
    .with_negations(vec![negative]);
    let executable = compile(vec![rule]).unwrap();
    let literals = [
        (10, RdfLiteral::typed("2026-01-01T00:00:00Z", STAMP)),
        (11, RdfLiteral::typed("2026-02-01T00:00:00", TIME)),
        (12, RdfLiteral::typed("2026-02-01T00:00:00Z", STAMP)),
    ]
    .into_iter()
    .collect();
    assert!(matches!(
        evaluate(&executable, input(&literals, [[10, 0, 11], [10, 2, 11]])),
        Err(EvalError::ComparisonUndetermined { .. })
    ));
    let result = evaluate(&executable, input(&literals, [[10, 0, 11]])).unwrap();
    assert!(
        result
            .facts()
            .rows_from(0)
            .any(|(_, row)| row == [10, 1, 11])
    );
    let result = evaluate(
        &executable,
        input(&literals, [[10, 0, 11], [10, 2, 11], [10, 2, 12]]),
    )
    .unwrap();
    assert!(
        !result
            .facts()
            .rows_from(0)
            .any(|(_, row)| row == [10, 1, 11])
    );
}

#[test]
fn native_rule_orders_by_instant_then_by_digest_only_on_a_tie() {
    let head = atom("first", "answer", "second");
    let times = [atom("first", "time", "ta"), atom("second", "time", "tb")];
    let chronological = DlClause::datalog(head.clone(), times.to_vec()).with_guards(vec![guard(
        "ta",
        "tb",
        Op::Less,
        false,
    )]);
    let body = [
        crate::clause::BodyLiteral::Atom(times[0].clone()),
        crate::clause::BodyLiteral::Atom(times[1].clone()),
        crate::clause::BodyLiteral::Guard(guard("ta", "tb", Op::Equal, false)),
        crate::clause::BodyLiteral::Atom(atom("first", "digest", "da")),
        crate::clause::BodyLiteral::Atom(atom("second", "digest", "db")),
        crate::clause::BodyLiteral::Guard(guard("da", "db", Op::Less, false)),
    ];
    let tie_break = DlClause::datalog(head, vec![]).with_ordered_body(body.into());
    let executable = compile(vec![chronological, tie_break]).unwrap();
    let literals = [
        (20, RdfLiteral::typed("2026-04-04T09:12:00Z", STAMP)),
        (21, RdfLiteral::typed("2026-04-04T11:12:00+02:00", TIME)),
        (22, RdfLiteral::typed("2026-04-04T11:13:00+02:00", TIME)),
        (30, RdfLiteral::typed("10".repeat(32), BLAKE)),
        (31, RdfLiteral::typed("20".repeat(32), BLAKE)),
        // A different algorithm cannot compare. The time barrier must prevent
        // the digest guard from running on non-ties.
        (32, RdfLiteral::typed("00".repeat(32), SHA)),
    ]
    .into_iter()
    .collect();
    let result = evaluate(
        &executable,
        input(
            &literals,
            [
                [10, 2, 20],
                [11, 2, 21],
                [12, 2, 22],
                [10, 3, 31],
                [11, 3, 30],
                [12, 3, 32],
            ],
        ),
    )
    .unwrap();
    let ordered: BTreeSet<_> = result
        .facts()
        .rows_from(0)
        .filter_map(|(_, [left, relation, right])| (relation == 1).then_some((left, right)))
        .collect();
    assert_eq!(ordered, BTreeSet::from([(11, 10), (10, 12), (11, 12)]));
}
