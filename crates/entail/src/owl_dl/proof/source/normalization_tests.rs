// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original source identity across the actual native rewrite/absorption/compiler.

use super::*;
use crate::reasoner::{Reasoner, schema_tests::Fixture};
use purrdf_core::BlankScope;
use purrdf_iri::vocab::{owl, rdfs};

/// Name an original syntax node explicitly so expectations do not borrow compiler IDs.
fn blank(fixture: &mut Fixture, label: &str) -> purrdf_core::TermId {
    fixture.builder.intern_blank(label, BlankScope(9)).unwrap()
}

/// Expected input identity, independent of native interning and concept normalization.
fn blank_value(label: &str) -> TermValue {
    TermValue::Blank {
        label: label.to_owned(),
        scope: BlankScope(9),
    }
}

#[test]
fn both_sides_split_without_renumbering_or_replacing_the_original_source() {
    let mut f = Fixture::default();
    let a = f.name("A");
    let b = f.name("B");
    let c = f.name("C");
    let d = f.name("D");
    let e = f.name("E");
    let g = f.name("G");
    let union = blank(&mut f, "union");
    let intersection = blank(&mut f, "intersection");
    let members = f.list(&[a, b]);
    f.edge(union, owl::UNION_OF, members);
    let members = f.list(&[c, d]);
    f.edge(intersection, owl::INTERSECTION_OF, members);
    f.edge(a, rdfs::SUB_CLASS_OF, a); // Dropped tautology must not compact source ordinals.
    f.edge(union, rdfs::SUB_CLASS_OF, intersection);
    f.edge(e, rdfs::SUB_CLASS_OF, g);
    let input = f.freeze();
    let ctx = Reasoner::with_proofs(&input)
        .unwrap()
        .proof_context()
        .unwrap();
    let mut split = 0;
    let mut other = 0;
    for index in 0..ctx.clause_count() {
        let Some(origin) = ctx.clause_tbox_origin(index).unwrap() else {
            continue;
        };
        let ResolvedAxiomSource::Triple {
            subject: ResolvedTerm::Source(subject),
            predicate: ResolvedTerm::Source(predicate),
            object: ResolvedTerm::Source(object),
        } = origin
        else {
            panic!("original RDF axiom")
        };
        assert_eq!(predicate, &TermValue::Iri(rdfs::SUB_CLASS_OF.to_owned()));
        if subject == &blank_value("union") {
            split += 1;
            assert_eq!(object, &blank_value("intersection"));
        } else {
            other += 1;
            assert_eq!(subject, &TermValue::Iri("https://example.org/E".to_owned()));
            assert_eq!(object, &TermValue::Iri("https://example.org/G".to_owned()));
        }
    }
    assert_eq!((split, other), (4, 1));
    assert_eq!(ctx.internalized_axioms().len(), 0);
    assert!(matches!(
        ctx.clause_tbox_origin(ctx.clause_count()),
        Err(DlProofError::UnknownClause { .. })
    ));
}

#[test]
fn internalization_partial_absorption_and_range_keep_separate_source_licenses() {
    let mut f = Fixture::default();
    let a = f.name("A");
    let b = f.name("B");
    let c = f.name("C");
    let d = f.name("D");
    let p = f.name("p");
    let negative = blank(&mut f, "negative");
    f.edge(negative, owl::COMPLEMENT_OF, a);
    f.edge(negative, rdfs::SUB_CLASS_OF, b);
    let universal = blank(&mut f, "universal");
    f.edge(universal, owl::ON_PROPERTY, p);
    f.edge(universal, owl::ALL_VALUES_FROM, a);
    let partial = blank(&mut f, "partial");
    let members = f.list(&[c, universal]);
    f.edge(partial, owl::INTERSECTION_OF, members);
    f.edge(partial, rdfs::SUB_CLASS_OF, d);
    f.edge(p, rdfs::RANGE, a);
    let ctx = Reasoner::with_proofs(&f.freeze())
        .unwrap()
        .proof_context()
        .unwrap();
    let mut formulas = ctx.internalized_axioms();
    assert_eq!(formulas.len(), 1);
    let (formula, origin) = formulas.next().unwrap();
    assert!(matches!(formula.kind(), ConceptKind::Or(_)));
    let ResolvedAxiomSource::Triple {
        subject: ResolvedTerm::Source(subject),
        ..
    } = origin
    else {
        panic!("source internalization")
    };
    assert_eq!(subject, &blank_value("negative"));
    let mut partials = 0;
    let mut ranges = 0;
    for index in 0..ctx.clause_count() {
        let Some(origin) = ctx.clause_tbox_origin(index).unwrap() else {
            continue;
        };
        let ResolvedAxiomSource::Triple {
            subject: ResolvedTerm::Source(subject),
            predicate: ResolvedTerm::Source(predicate),
            ..
        } = origin
        else {
            panic!("source absorption")
        };
        if subject == &blank_value("partial") {
            partials += 1;
            assert_eq!(predicate, &TermValue::Iri(rdfs::SUB_CLASS_OF.to_owned()));
        } else {
            ranges += 1;
            assert_eq!(subject, &TermValue::Iri("https://example.org/p".to_owned()));
            assert_eq!(predicate, &TermValue::Iri(rdfs::RANGE.to_owned()));
        }
    }
    assert_eq!((partials, ranges), (1, 1));
}

#[test]
fn equivalent_and_subclass_axioms_do_not_lose_their_distinct_original_spellings() {
    let mut f = Fixture::default();
    let a = f.name("A");
    let b = f.name("B");
    f.edge(a, rdfs::SUB_CLASS_OF, b);
    f.edge(a, owl::EQUIVALENT_CLASS, b);
    let ctx = Reasoner::with_proofs(&f.freeze())
        .unwrap()
        .proof_context()
        .unwrap();
    let mut equivalences = 0;
    let mut subclasses = 0;
    for index in 0..ctx.clause_count() {
        if let Some(ResolvedAxiomSource::Triple {
            predicate: ResolvedTerm::Source(predicate),
            ..
        }) = ctx.clause_tbox_origin(index).unwrap()
        {
            if predicate == &TermValue::Iri(owl::EQUIVALENT_CLASS.to_owned()) {
                equivalences += 1;
            } else {
                assert_eq!(predicate, &TermValue::Iri(rdfs::SUB_CLASS_OF.to_owned()));
                subclasses += 1;
            }
        }
    }
    assert_eq!((equivalences, subclasses), (2, 1));
    assert_eq!(
        size_of::<Option<std::num::NonZeroUsize>>(),
        size_of::<usize>()
    );
}
