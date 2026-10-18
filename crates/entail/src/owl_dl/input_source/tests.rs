// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original parser owners and atomic native metadata publication.

use super::{Input, Inputs, Source};
use crate::owl_dl::{AxiomSource, Kb};
use crate::reasoner::schema_tests::Fixture;
use purrdf_core::TermValue;
use purrdf_iri::vocab::{owl, rdf, rdfs};
use purrdf_lex::allocation::StorageError;

#[test]
fn parsed_input_sources_cover_assertions_rules_nodes_and_literal_semantics() {
    let mut f = Fixture::default();
    let class = f.name("A");
    let a = f.instance("a", class);
    let b = f.instance("b", class);
    let c = f.name("c");
    let p = f.name("p");
    let q = f.name("q");
    let r = f.name("r");
    let u = f.name("u");
    let v = f.name("v");
    f.edge(p, owl::INVERSE_OF, q);
    f.edge(q, owl::EQUIVALENT_PROPERTY, r);
    f.edge(q, rdfs::SUB_PROPERTY_OF, p);
    for (property, kind) in [
        (p, owl::TRANSITIVE_PROPERTY),
        (r, owl::SYMMETRIC_PROPERTY),
        (u, owl::ASYMMETRIC_PROPERTY),
    ] {
        let kind = f.builder.intern_iri(kind).unwrap();
        f.edge(property, rdf::TYPE, kind);
    }
    f.edge(u, owl::PROPERTY_DISJOINT_WITH, v);
    f.edge(a, owl::SAME_AS, b);
    f.edge(a, owl::DIFFERENT_FROM, b);
    f.builder.push_quad(a, p, b, None).unwrap();
    let data = f.name("data");
    let literal = crate::interner::intern_into(
        &mut f.builder,
        &TermValue::Literal {
            lexical_form: "01".to_owned(),
            datatype: purrdf_xsd::datatype::XSD_INTEGER.to_owned(),
            language: None,
            direction: None,
        },
    )
    .unwrap();
    f.builder.push_quad(a, data, literal, None).unwrap();
    let all_different = f.blank();
    let kind = f.builder.intern_iri(owl::ALL_DIFFERENT).unwrap();
    let members = f.list(&[a, b, c]);
    f.edge(all_different, rdf::TYPE, kind);
    f.edge(all_different, owl::DISTINCT_MEMBERS, members);
    for typed in [false, true] {
        let node = f.blank();
        if typed {
            let kind = f
                .builder
                .intern_iri(owl::NEGATIVE_PROPERTY_ASSERTION)
                .unwrap();
            f.edge(node, rdf::TYPE, kind);
        }
        f.edge(node, owl::SOURCE_INDIVIDUAL, a);
        f.edge(node, owl::ASSERTION_PROPERTY, p);
        f.edge(node, owl::TARGET_INDIVIDUAL, c);
    }
    let kb = Kb::from_dataset(&f.freeze()).unwrap();
    let mut literals = 0;
    let mut node_assertions = [false; 2];
    for (index, &(individual, _)) in kb.abox_types.iter().enumerate() {
        match kb.inputs.get(Input::Type(index)).unwrap() {
            Source::LiteralValue(term) => {
                literals += 1;
                assert_eq!(term, individual);
                assert!(matches!(kb.interner.value(term), TermValue::Literal { .. }));
            }
            Source::Anchor(AxiomSource::Node { kind, .. }) => {
                node_assertions[usize::from(kind.is_some())] = true;
            }
            Source::Anchor(AxiomSource::Triple(s, predicate, _)) => {
                assert_eq!(s, individual);
                assert!(
                    matches!(kb.interner.value(predicate), TermValue::Iri(iri) if iri == rdf::TYPE)
                );
            }
            other => panic!("unexpected type origin {other:?}"),
        }
    }
    assert_eq!(literals, 1);
    assert_eq!(node_assertions, [true, true]);
    for (index, &(s, p, o)) in kb.abox_roles.iter().enumerate() {
        assert_eq!(
            kb.inputs.get(Input::Role(index)),
            Some(Source::Anchor(AxiomSource::Triple(s, p, o)))
        );
    }
    for index in 0..kb.same_as.len() {
        assert!(matches!(
            kb.inputs.get(Input::Equality(index)),
            Some(Source::Anchor(AxiomSource::Triple(..)))
        ));
    }
    for index in 0..kb.different_from.len() {
        assert!(kb.inputs.get(Input::Distinct(index)).is_some());
    }
    for (&left, rights) in &kb.inverses {
        for &right in rights {
            assert!(kb.inputs.get(Input::Inverse(left, right)).is_some());
        }
    }
    for (&sup, subs) in &kb.role_sub {
        for &sub in subs {
            assert!(kb.inputs.get(Input::SubRole { sub, sup }).is_some());
        }
    }
    for &property in &kb.transitive {
        assert!(kb.inputs.get(Input::Transitive(property)).is_some());
    }
    for &property in &kb.asymmetric {
        assert!(kb.inputs.get(Input::Asymmetric(property)).is_some());
    }
    for &(left, right) in &kb.disjoint_roles {
        assert!(kb.inputs.get(Input::DisjointRole(left, right)).is_some());
    }
}

#[test]
fn input_source_key_batches_rollback_refusal_and_stop_without_losing_prior_inputs() {
    let batch = [(0, 10, 11), (0, 10, 12), (1, 11, 12)];
    for after in 0..batch.len() {
        let mut sources = Inputs::default();
        let original = Source::Anchor(AxiomSource::Triple(1, 2, 3));
        sources.record(Input::Equality(0), original).unwrap();
        sources.refusal = Some((after, StorageError::AllocationFailed));
        assert!(matches!(
            sources.record_key_equalities(1, &batch, || Ok(())),
            Err(crate::EntailError::Storage(StorageError::AllocationFailed))
        ));
        assert_eq!(sources.get(Input::Equality(0)), Some(original));
        for index in 1..=batch.len() {
            assert_eq!(sources.get(Input::Equality(index)), None);
        }
        sources.record_key_equalities(1, &batch, || Ok(())).unwrap();
        assert_eq!(
            sources.get(Input::Equality(1)),
            Some(Source::KeyInference {
                key: 0,
                left: 10,
                right: 11
            })
        );
    }
    let mut sources = Inputs::default();
    let mut polls = 0;
    assert!(matches!(
        sources.record_key_equalities(0, &batch, || {
            polls += 1;
            if polls == 2 {
                Err(crate::EntailError::Stopped)
            } else {
                Ok(())
            }
        }),
        Err(crate::EntailError::Stopped)
    ));
    assert_eq!(sources.get(Input::Equality(0)), None);
    assert!(matches!(
        sources.record_key_equalities(usize::MAX, &batch, || Ok(())),
        Err(crate::EntailError::Storage(StorageError::SizeOverflow))
    ));
}

/// Compare real admitted buffers, not host memory or an elapsed-time estimate.
#[test]
fn input_source_ordinal_storage_uses_less_admitted_memory_than_a_sparse_index() {
    use purrdf_lex::allocation::{Memory, OrderedTable, Resident};
    let mut dense = Inputs::default();
    let mut sparse = OrderedTable::default();
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let source = Source::Anchor(AxiomSource::Triple(1, 2, 3));
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    for index in 0..1000 {
        dense.record(Input::Role(index), source).unwrap();
    }
    let dense_allocations = window.close();
    assert!(
        dense_allocations.allocations < 32,
        "ordinal metadata grows geometrically, not once per row"
    );
    for index in 0..1000 {
        sparse
            .insert(Input::Role(index), source, &mut memory)
            .unwrap();
    }
    assert!(dense.live < memory.admitted_bytes());
    assert_eq!(
        dense.live,
        dense.roles.capacity() * size_of::<Option<Source>>()
    );
    for index in 0..1000 {
        assert_eq!(
            dense.get(Input::Role(index)),
            sparse.get(&Input::Role(index)).copied()
        );
    }
    eprintln!(
        "native admitted metadata bytes: dense={}, sparse={}; dense allocations={}",
        dense.live,
        memory.admitted_bytes(),
        dense_allocations.allocations
    );
}
