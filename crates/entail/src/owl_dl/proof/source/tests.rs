// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact source views at the checked public replay boundary.

use super::*;
use crate::reasoner::schema_tests::Fixture;
use crate::reasoner::{Reasoner, Verdict};
use purrdf_alloc_probe::CurrentThreadWindow;
use purrdf_iri::vocab::{owl, rdf, rdfs};

/// Independent preparation for a tiny authored input; no corpus factory is involved.
fn checked_fixture(input: &purrdf_core::RdfDataset) -> (DlProof, DlProofContext<'static>) {
    let producer = Reasoner::with_proofs(input).unwrap();
    let result = producer.consistency();
    assert_eq!(*result.answer(), Verdict::False);
    let proof = result
        .proof()
        .unwrap()
        .runs()
        .last()
        .unwrap()
        .proof()
        .unwrap()
        .clone();
    let consumer = Reasoner::with_proofs(input).unwrap();
    (proof, consumer.proof_context().unwrap())
}

/// A real derived clash; the consumer prepares the original input independently.
fn derived() -> (DlProof, DlProofContext<'static>) {
    derived_for(None)
}

/// Retain either a named or scoped-blank source inhabitant through real replay.
fn derived_for(individual: Option<TermValue>) -> (DlProof, DlProofContext<'static>) {
    let mut fixture = Fixture::default();
    let class = fixture.name("A");
    let filler = fixture.name("F");
    let property = fixture.name("p");
    fixture.bounds(class, property, filler, 2, true, false);
    fixture.bounds(class, property, filler, 1, false, false);
    let edge = fixture.name("edge");
    let individual = match individual {
        Some(value) => crate::interner::intern_into(&mut fixture.builder, &value).unwrap(),
        None => fixture.name("i"),
    };
    let other = fixture.name("other");
    fixture.edge(edge, rdfs::RANGE, class);
    fixture
        .builder
        .push_quad(other, edge, individual, None)
        .unwrap();
    let input = fixture.freeze();
    checked_fixture(&input)
}

#[test]
fn legacy_structural_success_is_not_checked_source_support() {
    let mut fixture = Fixture::default();
    let a = fixture.name("A");
    let b = fixture.name("B");
    fixture.edge(a, owl::DISJOINT_WITH, b);
    let individual = fixture.instance("i", a);
    fixture.edge(individual, rdf::TYPE, b);
    let (mut proof, context) = checked_fixture(&fixture.freeze());
    for step in &mut proof.clashes {
        assert!(step.schema.is_none());
        step.current = None;
    }
    assert!(!proof.replay_clash(0, &context).unwrap().current_supported());
    assert!(matches!(
        proof.resolve_clash(0, &context),
        Err(DlProofError::Malformed { .. })
    ));
}

#[test]
fn actual_role_clash_resolves_both_endpoints_and_property() {
    let mut fixture = Fixture::default();
    let property = fixture.name("p");
    let asymmetric = fixture
        .builder
        .intern_iri(owl::ASYMMETRIC_PROPERTY)
        .unwrap();
    fixture.edge(property, rdf::TYPE, asymmetric);
    let a = fixture.name("a");
    let b = fixture.name("b");
    fixture.builder.push_quad(a, property, b, None).unwrap();
    fixture.builder.push_quad(b, property, a, None).unwrap();
    let (proof, context) = checked_fixture(&fixture.freeze());
    let index = proof
        .clashes
        .iter()
        .position(|step| {
            step.witness
                .iter()
                .any(|f| matches!(f, ProofFact::Edge { .. }))
        })
        .unwrap();
    let checked = proof.resolve_clash(index, &context).unwrap();
    assert!(checked.schema().is_none());
    let mut edges = 0;
    for fact in checked.witness() {
        if let ResolvedFact::Edge(
            ResolvedNode::Individual(ResolvedTerm::Source(from)),
            r,
            ResolvedNode::Individual(ResolvedTerm::Source(to)),
        ) = fact
        {
            edges += 1;
            assert_ne!(from, to);
            assert_eq!(
                r.property,
                ResolvedTerm::Source(&TermValue::Iri("https://example.org/p".to_owned()))
            );
            assert!(!r.inverse);
        }
    }
    assert_eq!(edges, 2);
}

#[test]
fn actual_nominal_clash_resolves_denotation_without_renaming() {
    let mut fixture = Fixture::default();
    let class = fixture.name("A");
    let individual = fixture.instance("i", class);
    let nominal = fixture.blank();
    let list = fixture.list(&[individual]);
    fixture.edge(nominal, owl::ONE_OF, list);
    fixture.edge(class, owl::COMPLEMENT_OF, nominal);
    let (proof, context) = checked_fixture(&fixture.freeze());
    let index = proof
        .clashes
        .iter()
        .position(|step| {
            step.witness
                .iter()
                .any(|f| matches!(f, ProofFact::Denotes { .. }))
        })
        .unwrap();
    let checked = proof.resolve_clash(index, &context).unwrap();
    assert!(checked.witness().any(|f| matches!(f,
        ResolvedFact::Denotes(ResolvedNode::Individual(ResolvedTerm::Source(node)), ResolvedTerm::Source(individual))
        if node == individual)));
}

#[test]
fn resolution_preserves_the_consumers_typed_stop() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    #[derive(Debug)]
    struct Stop(AtomicBool);
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            self.0.load(Ordering::Relaxed)
        }
    }
    let input = crate::reasoner::schema_tests::shared_schema(1, 1, true);
    let (proof, _) = checked_fixture(&input);
    let stop = Arc::new(Stop(AtomicBool::new(false)));
    let consumer = Reasoner::with_proofs_and_stop(
        &input,
        crate::reasoner::SchemaPreparationBudget::default(),
        stop.clone(),
    )
    .unwrap();
    let context = consumer.borrowed_proof_context().unwrap();
    stop.0.store(true, Ordering::Relaxed);
    assert!(matches!(
        proof.resolve_clash(0, &context),
        Err(DlProofError::Stopped)
    ));
}

#[test]
fn checked_clash_resolves_actual_derived_source_membership_without_copying() {
    let (proof, context) = derived();
    let index = proof
        .clashes
        .iter()
        .position(|step| step.schema.is_some())
        .unwrap();
    let checked = proof.resolve_clash(index, &context).unwrap();
    assert!(checked.replay().current_supported());
    let mut count = 0;
    for fact in checked.witness() {
        count += 1;
        let ResolvedFact::Concept(
            ResolvedNode::Individual(ResolvedTerm::Source(individual)),
            class,
        ) = fact
        else {
            panic!("schema clash must retain its source inhabitant and class");
        };
        assert_eq!(
            individual,
            &TermValue::Iri("https://example.org/i".to_owned())
        );
        let ConceptKind::Named(ResolvedTerm::Source(class)) = class.kind() else {
            panic!("schema clash must resolve its native class");
        };
        assert_eq!(class, &TermValue::Iri("https://example.org/A".to_owned()));
    }
    assert!(count > 0);
    // The strings above are test expectations, so measure the iterator separately.
    let window = CurrentThreadWindow::open();
    assert_eq!(checked.witness().count(), count);
    assert_eq!(window.sample().allocations, 0);
}

#[test]
fn source_resolution_refuses_wrong_source_and_absent_current_support() {
    let (mut proof, context) = derived();
    let wrong = Reasoner::with_proofs(&Fixture::default().freeze())
        .unwrap()
        .proof_context()
        .unwrap();
    assert!(proof.resolve_clash(0, &wrong).is_err());
    assert!(proof.resolve_clash(proof.clashes.len(), &context).is_err());
    for step in &mut proof.clashes {
        step.current = None;
        step.schema = None;
    }
    for index in 0..proof.clashes.len() {
        assert!(proof.resolve_clash(index, &context).is_err());
    }
}

#[test]
fn checked_source_blank_scope_and_schema_bounds_survive_resolution() {
    let original = TermValue::Blank {
        label: "inhabitant".to_owned(),
        scope: purrdf_core::BlankScope(71),
    };
    let (proof, context) = derived_for(Some(original.clone()));
    let index = proof
        .clashes
        .iter()
        .position(|s| s.schema.is_some())
        .unwrap();
    let checked = proof.resolve_clash(index, &context).unwrap();
    assert_eq!(checked.replay().source_node(), Some(&original));
    assert!(checked.witness().any(|fact| matches!(fact,
        ResolvedFact::Concept(ResolvedNode::Individual(ResolvedTerm::Source(t)), _) if t == &original)));
    let schema = checked.schema().unwrap();
    assert!(matches!(schema.class().kind(), ConceptKind::Named(_)));
    assert!(matches!(schema.lower().kind(), ConceptKind::Min(2, _, _)));
    let SchemaUpper::Restriction(upper) = schema.upper() else {
        panic!("actual maximum restriction")
    };
    assert!(matches!(upper.kind(), ConceptKind::Max(1, _, _)));
    assert!(matches!(schema.qualifier(), QualifierKind::Identity));
    assert_eq!(schema.qualifier_derivation().len(), 0);
    let steps: Vec<_> = schema.class_derivation().collect();
    assert!(
        steps
            .iter()
            .any(|s| matches!(s.reason, SchemaReason::Assume))
    );
    assert_eq!(
        steps
            .iter()
            .filter(|s| matches!(s.reason, SchemaReason::Inclusion { .. }))
            .count(),
        2
    );
    for s in steps {
        if let SchemaReason::Inclusion { sub, sup } = s.reason {
            assert!(matches!(sub.kind(), ConceptKind::Named(_)));
            assert!(matches!(
                sup.kind(),
                ConceptKind::Min(_, _, _) | ConceptKind::Max(_, _, _)
            ));
        }
    }
}

#[test]
fn exact_literal_metadata_is_borrowed_without_reinterpretation() {
    let mut kb = Kb::empty();
    let literal = TermValue::Literal {
        lexical_form: "مرحبا".to_owned(),
        datatype: rdf::DIR_LANG_STRING.to_owned(),
        language: Some("ar".to_owned()),
        direction: Some(purrdf_core::RdfTextDirection::Rtl),
    };
    let id = kb.interner.intern(literal.clone());
    kb.source_terms = u32::try_from(kb.interner.len()).unwrap();
    let ResolvedTerm::Source(value) = term(&kb, id) else {
        panic!("source literal")
    };
    assert_eq!(value, &literal);
    assert!(std::ptr::eq(value, kb.interner.value(id)));
}

#[test]
fn every_native_constructor_has_a_borrowed_structural_view() {
    let mut kb = Kb::empty();
    let name = kb.interner.intern_iri("https://example.org/A");
    let property = kb.interner.intern_iri("https://example.org/p");
    kb.source_terms = u32::try_from(kb.interner.len()).unwrap();
    let filler = kb.table.intern(Concept::Named(name));
    let range = kb.data_ranges.intern(DataRange::Any);
    let named = || Box::new(Concept::Named(name));
    let concepts = [
        Concept::Top,
        Concept::Bottom,
        Concept::Named(name),
        Concept::Not(named()),
        Concept::And(vec![Concept::Named(name), Concept::Top]),
        Concept::Or(vec![Concept::Named(name), Concept::Bottom]),
        Concept::Some(Role::Named(property), named()),
        Concept::All(Role::Inv(property), named()),
        Concept::Min(2, Role::Named(property), named()),
        Concept::Max(1, Role::Inv(property), named()),
        Concept::Nominal(vec![name]),
        Concept::Data(range),
        Concept::SelfRestriction(Role::Inv(property)),
        Concept::RoleState {
            machine: 3,
            state: 7,
            filler,
            negated: true,
        },
    ];
    let mut seen = Vec::new();
    for concept in &concepts {
        let view = ResolvedConcept { kb: &kb, concept };
        let tag = match view.kind() {
            ConceptKind::Top => 0,
            ConceptKind::Bottom => 1,
            ConceptKind::Named(t) => {
                assert!(matches!(t, ResolvedTerm::Source(_)));
                2
            }
            ConceptKind::Not(c) => {
                assert!(matches!(c.kind(), ConceptKind::Named(_)));
                3
            }
            ConceptKind::And(mut cs) => {
                assert_eq!(cs.len(), 2);
                assert!(matches!(cs.next().unwrap().kind(), ConceptKind::Named(_)));
                4
            }
            ConceptKind::Or(cs) => {
                assert_eq!(cs.count(), 2);
                5
            }
            ConceptKind::Some(r, c) => {
                assert!(!r.inverse);
                assert!(matches!(c.kind(), ConceptKind::Named(_)));
                6
            }
            ConceptKind::All(r, _) => {
                assert!(r.inverse);
                7
            }
            ConceptKind::Min(n, _, _) => {
                assert_eq!(n, 2);
                8
            }
            ConceptKind::Max(n, r, _) => {
                assert_eq!(n, 1);
                assert!(r.inverse);
                9
            }
            ConceptKind::Nominal(mut ids) => {
                assert_eq!(ids.len(), 1);
                assert_eq!(ids.next(), Some(term(&kb, name)));
                assert!(ids.next().is_none());
                10
            }
            ConceptKind::Data(r) => {
                assert!(std::ptr::eq(r, kb.data_ranges.range(range)));
                11
            }
            ConceptKind::SelfRestriction(r) => {
                assert!(r.inverse);
                12
            }
            ConceptKind::RoleState {
                machine,
                state,
                filler,
                negated,
            } => {
                assert_eq!((machine, state, negated), (3, 7, true));
                assert!(matches!(filler.kind(), ConceptKind::Named(_)));
                13
            }
        };
        seen.push(tag);
    }
    assert_eq!(seen, (0..14).collect::<Vec<_>>());
}

#[test]
fn query_symbols_and_recursive_reserved_roots_never_gain_source_identity() {
    let mut kb = Kb::empty();
    let source = kb.interner.intern_iri("https://example.org/source");
    let property = kb.interner.intern_iri("https://example.org/p");
    kb.source_terms = u32::try_from(kb.interner.len()).unwrap();
    let query = kb.interner.intern_iri("https://example.org/query");
    let filler = kb.table.intern(Concept::Top);
    let nested = NodeRef::Reserved(Box::new(super::super::ReservedRef {
        origin: NodeRef::Individual(query),
        role: super::super::ProofRole::of(Role::Inv(property)),
        filler,
        index: 2,
    }));
    let root = NodeRef::Reserved(Box::new(super::super::ReservedRef {
        origin: nested,
        role: super::super::ProofRole::of(Role::Named(property)),
        filler,
        index: 1,
    }));
    assert!(matches!(
        node(&kb, &NodeRef::Individual(source)),
        ResolvedNode::Individual(ResolvedTerm::Source(_))
    ));
    assert!(matches!(
        node(&kb, &NodeRef::Anonymous(5)),
        ResolvedNode::Anonymous(5)
    ));
    let ResolvedNode::Reserved(root) = node(&kb, &root) else {
        panic!("reserved root")
    };
    assert_eq!(root.index(), 1);
    assert!(!root.role().inverse);
    assert!(matches!(root.filler().kind(), ConceptKind::Top));
    let ResolvedNode::Reserved(parent) = root.origin() else {
        panic!("recursive origin")
    };
    assert_eq!(parent.index(), 2);
    assert!(parent.role().inverse);
    assert!(matches!(
        parent.origin(),
        ResolvedNode::Individual(ResolvedTerm::QuerySymbol(_))
    ));
}
