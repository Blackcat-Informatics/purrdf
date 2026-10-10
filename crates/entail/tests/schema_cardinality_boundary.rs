// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The actual production boundary, without the unit-only concept-tree oracle's
//! exhaustive sub-cardinality interning. No reference algorithm is weakened.

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_entail::{EntailError, Reasoner, ServiceProof, Verdict};
use purrdf_iri::vocab::{owl, rdf, rdfs};
use purrdf_xsd::datatype::XSD_NON_NEGATIVE_INTEGER;
use std::sync::Arc;

const CLASS: &str = "https://example.org/A";

fn ontology(count: u32, asserted: bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let class = builder.intern_iri(CLASS);
    let role = builder.intern_iri("https://example.org/p");
    let filler = builder.intern_iri("https://example.org/F");
    for (label, predicate, count) in [
        ("minimum", owl::MIN_QUALIFIED_CARDINALITY, count),
        ("maximum", owl::MAX_QUALIFIED_CARDINALITY, 0),
    ] {
        let restriction = builder.intern_blank(label, BlankScope::DEFAULT);
        let number = builder.intern_literal(RdfLiteral {
            lexical_form: count.to_string(),
            datatype: Some(XSD_NON_NEGATIVE_INTEGER.to_owned()),
            language: None,
            direction: None,
        });
        for (subject, predicate, object) in [
            (class, rdfs::SUB_CLASS_OF, restriction),
            (restriction, owl::ON_PROPERTY, role),
            (restriction, owl::ON_CLASS, filler),
            (restriction, predicate, number),
        ] {
            let predicate = builder.intern_iri(predicate);
            builder.push_quad(subject, predicate, object, None);
        }
    }
    if asserted {
        let individual = builder.intern_iri("https://example.org/i");
        let predicate = builder.intern_iri(rdf::TYPE);
        builder.push_quad(individual, predicate, class, None);
    }
    builder.freeze().expect("authored boundary ontology")
}

#[test]
fn largest_supported_bound_uses_prepared_clash_and_next_count_is_a_typed_refusal() {
    // The original representation retains n+1 when negating a maximum.
    for (count, asserted) in [
        (u32::MAX - 1, false),
        (u32::MAX - 1, true),
        (u32::MAX, true),
    ] {
        let dataset = ontology(count, asserted);
        let constructed = Reasoner::with_proofs(&dataset);
        if count == u32::MAX {
            let Err(EntailError::Parse(detail)) = constructed else {
                panic!("an unrepresentable count must retain the typed native refusal");
            };
            assert!(
                detail.contains("4294967295") && detail.contains("4294967294"),
                "{detail}"
            );
            continue;
        }
        let mut reasoner = constructed.unwrap();
        assert!(
            reasoner
                .schema_bounds(&TermValue::Iri(CLASS.to_owned()))
                .is_some()
        );
        let consistent = if asserted {
            Verdict::False
        } else {
            Verdict::True
        };
        let consistency = reasoner.consistency();
        let satisfiability = reasoner.class_satisfiability(&TermValue::Iri(CLASS.to_owned()));
        let satisfiability = if asserted {
            assert!(matches!(satisfiability, Err(EntailError::Unsatisfiable)));
            None
        } else {
            Some(satisfiability.unwrap())
        };
        for (answer, expected) in core::iter::once((consistency, consistent))
            .chain(satisfiability.map(|answer| (answer, Verdict::False)))
        {
            assert_eq!(*answer.answer(), expected);
            assert!(
                answer.certificate().peak_nodes() < 16,
                "the prepared empty class must never construct billions of successors"
            );
            let encoded = answer.proof().expect("recorded public service").encode();
            let proof = ServiceProof::decode(&encoded).expect("canonical native proof");
            assert_eq!(proof.encode(), encoded);
            if expected == Verdict::False {
                assert!(
                    proof
                        .runs()
                        .iter()
                        .filter_map(|run| run.proof())
                        .any(|trace| {
                            trace
                                .clashes()
                                .iter()
                                .any(|clash| clash.schema_evidence().is_some())
                        })
                );
            }
            let mut consumer = Reasoner::with_proofs(&dataset).unwrap();
            consumer.prepare(proof.question());
            let context = consumer.proof_context().unwrap();
            let replay = proof
                .verify(
                    &dataset,
                    proof.question(),
                    Some(answer.certificate()),
                    &context,
                )
                .expect("independently check every necessary bound and inhabitance premise");
            assert_eq!(replay.checks().unattested(), 0);
        }
    }
}
