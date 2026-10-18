// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original hierarchy source owners on tiny RDF, never a repository corpus.

use super::Input;
use crate::{
    owl_dl::{Kb, concept::Role},
    reasoner::schema_tests::Fixture,
};
use purrdf_core::TermValue;
use purrdf_iri::vocab::{owl, rdf, rdfs};

#[test]
fn hierarchy_sources_retain_signed_inputs_lists_and_inverse_syntax() {
    let mut f = Fixture::default();
    let p = f.name("p");
    let q = f.name("q");
    let r = f.name("r");
    let s = f.name("s");
    let inverse = f.blank();
    f.edge(inverse, owl::INVERSE_OF, p);
    let list = f.list(&[inverse, q]);
    f.edge(r, owl::PROPERTY_CHAIN_AXIOM, list);
    f.edge(p, rdfs::SUB_PROPERTY_OF, s);
    f.edge(p, owl::EQUIVALENT_PROPERTY, q);
    for (name, kind) in [
        ("transitive", owl::TRANSITIVE_PROPERTY),
        ("symmetric", owl::SYMMETRIC_PROPERTY),
        ("reflexive", owl::REFLEXIVE_PROPERTY),
    ] {
        let property = f.name(name);
        let kind = f.builder.intern_iri(kind).unwrap();
        f.edge(property, rdf::TYPE, kind);
    }
    let kb = Kb::from_dataset(&f.freeze()).unwrap();
    let hierarchy = &kb.role_program.as_ref().unwrap().source;
    for ordinal in 0..hierarchy.chains.len() {
        let source = hierarchy.sources.get(Input::Chain(ordinal)).unwrap();
        let TermValue::Iri(predicate) = kb.interner.value(source.anchor.1) else {
            panic!("source predicate");
        };
        assert!([owl::PROPERTY_CHAIN_AXIOM, rdf::TYPE].contains(&predicate.as_str()));
        if predicate == owl::PROPERTY_CHAIN_AXIOM {
            let reads = &hierarchy.sources.reads[source.reads.clone()];
            assert_eq!(reads.len(), 5);
            assert_eq!(reads.iter().filter(|&&(_, p, _)| matches!(kb.interner.value(p), TermValue::Iri(iri) if iri == owl::INVERSE_OF)).count(), 1);
        }
    }
    for &(sub, sup) in &hierarchy.inclusions {
        assert!(hierarchy.sources.get(Input::Inclusion(sub, sup)).is_some());
    }
    for &role in &hierarchy.reflexive {
        let source = hierarchy.sources.get(Input::Reflexive(role)).unwrap();
        assert!(
            matches!(kb.interner.value(source.anchor.2), TermValue::Iri(iri) if iri == owl::REFLEXIVE_PROPERTY)
        );
    }
    let p = Role::Named(kb.interner.id_of_iri("https://example.org/p").unwrap());
    let q = Role::Named(kb.interner.id_of_iri("https://example.org/q").unwrap());
    let forward = hierarchy.sources.get(Input::Inclusion(p, q)).unwrap();
    let inverse = hierarchy
        .sources
        .get(Input::Inclusion(p.inverse(), q.inverse()))
        .unwrap();
    assert!(
        std::ptr::eq(forward, inverse),
        "signed copies share one original source"
    );
}
