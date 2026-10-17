// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source anchors at the original parser and later schema-ingress boundaries.

use super::AxiomSource;
use crate::owl_dl::{Kb, concept::Concept, parser::constrain_bottom_roles};
use crate::reasoner::{
    Reasoner,
    schema_tests::{Fixture, shared_schema},
};
use purrdf_core::TermValue;
use purrdf_iri::vocab::{owl, rdf, rdfs};

/// Every parsed inclusion has an explicit source operation at the same ordinal.
fn complete(kb: &Kb) {
    assert_eq!(kb.tbox.len(), kb.tbox_sources.len());
    assert!(
        kb.tbox_sources
            .iter()
            .all(|s| *s != AxiomSource::Unrecorded)
    );
}

#[test]
fn source_triple_origins_preserve_order_and_original_expression_nodes() {
    let input = shared_schema(2, 1, true);
    let kb = Kb::from_dataset(&input).unwrap();
    complete(&kb);
    assert_eq!(kb.tbox.len(), 4);
    for source in &kb.tbox_sources {
        let AxiomSource::Triple(s, p, o) = *source else {
            panic!("authored triple origin")
        };
        assert_eq!(
            kb.interner.value(s),
            &TermValue::Iri("https://example.org/A".to_owned())
        );
        assert_eq!(
            kb.interner.value(p),
            &TermValue::Iri(rdfs::SUB_CLASS_OF.to_owned())
        );
        assert!(matches!(kb.interner.value(o), TermValue::Blank { .. }));
    }
    // Retaining source provenance changes neither the decision nor the certificate.
    let plain = Reasoner::new(&input).unwrap().consistency();
    let recorded = Reasoner::with_proofs(&input).unwrap().consistency();
    assert_eq!(plain.answer(), recorded.answer());
    assert_eq!(plain.certificate(), recorded.certificate());
}

#[test]
fn nary_axioms_keep_their_actual_node_and_typing_trigger() {
    let mut fixture = Fixture::default();
    let a = fixture.name("A");
    let b = fixture.name("B");
    let axiom = fixture.blank();
    let class = fixture
        .builder
        .intern_iri(owl::ALL_DISJOINT_CLASSES)
        .unwrap();
    let members = fixture.list(&[a, b]);
    fixture.edge(axiom, rdf::TYPE, class);
    fixture.edge(axiom, owl::MEMBERS, members);
    let kb = Kb::from_dataset(&fixture.freeze()).unwrap();
    complete(&kb);
    assert_eq!(kb.tbox.len(), 1);
    let AxiomSource::Node {
        node,
        kind: Some(kind),
    } = kb.tbox_sources[0]
    else {
        panic!("n-ary source axiom")
    };
    assert!(matches!(kb.interner.value(node), TermValue::Blank { .. }));
    assert_eq!(
        kb.interner.value(kind),
        &TermValue::Iri(owl::ALL_DISJOINT_CLASSES.to_owned())
    );
}

#[test]
fn recursive_source_equations_are_distinct_from_the_authored_axiom() {
    let mut fixture = Fixture::default();
    let a = fixture.name("A");
    let b = fixture.name("B");
    let c = fixture.name("C");
    let members = fixture.list(&[a, b]);
    fixture.edge(a, owl::INTERSECTION_OF, members);
    fixture.edge(a, rdfs::SUB_CLASS_OF, c);
    let kb = Kb::from_dataset(&fixture.freeze()).unwrap();
    complete(&kb);
    let equations: Vec<_> = kb
        .tbox_sources
        .iter()
        .filter_map(|s| {
            if let AxiomSource::RecursiveDefinition(node) = s {
                Some(*node)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(equations.len(), 2);
    assert_eq!(equations[0], equations[1]);
    assert_eq!(
        kb.interner.value(equations[0]),
        &TermValue::Iri("https://example.org/A".to_owned())
    );
    assert!(
        kb.tbox_sources
            .iter()
            .any(|s| matches!(s, AxiomSource::Triple(_, _, _)))
    );
}

#[test]
fn bottom_role_law_keeps_its_native_origin_and_invalidates_prepared_products() {
    let input = shared_schema(1, 1, true);
    let mut kb = Kb::from_dataset(&input).unwrap();
    assert!(kb.schema.stats.classes > 0);
    let role = kb.interner.intern_iri(owl::BOTTOM_OBJECT_PROPERTY);
    let before = kb.tbox.len();
    assert!(constrain_bottom_roles(&mut kb, [role]));
    assert_eq!(kb.tbox.len(), before + 1);
    assert_eq!(kb.tbox_sources.last(), Some(&AxiomSource::EmptyRole(role)));
    assert!(!kb.encoded);
    assert_eq!(kb.schema.stats.classes, 0);
    kb.finalize().unwrap();
    assert!(kb.schema.stats.classes > 0);
    assert!(!constrain_bottom_roles(&mut kb, [role]));
    complete(&kb);
}

#[test]
fn query_equations_and_synthetic_ingress_never_claim_source_assertions() {
    let mut kb = Kb::empty();
    let query = kb.interner.intern_iri("https://example.org/query");
    kb.push_gci_from(
        Concept::Named(query),
        Concept::Top,
        AxiomSource::QueryDefinition(query),
    );
    kb.push_gci(Concept::Top, Concept::Bottom);
    assert_eq!(
        kb.tbox_sources,
        [AxiomSource::QueryDefinition(query), AxiomSource::Unrecorded]
    );
    assert_eq!(kb.tbox.len(), kb.tbox_sources.len());
}
