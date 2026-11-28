// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! OWL-Direct classification benchmark: building a whole class taxonomy.
//!
//! Classification is the DL service whose cost is superlinear in the SIGNATURE rather than in
//! the axiom count, so it is the one worth watching as an ontology grows classes. The fixture
//! is a synthetic `EL` terminology — a branching subclass tree, a defined class per level
//! stated as an intersection, and one existential restriction with a sub-property under a
//! transitive super-property — which is the shape a real terminology has and which lies
//! inside the fragment the classifying saturation is complete for.
//!
//! # Report-only
//!
//! This asserts nothing and gates nothing. It exists so that a later change to the calculus,
//! the normalization or the residual-refutation policy has a NUMBER to move, not so a speedup
//! can be claimed: the measuring machine is not quiet, so the timings are indicative only.
//! The claim that a derived taxonomy is cheaper than a refuted one is made where it can be
//! made exactly — by the decision counter in
//! `the_derived_taxonomy_costs_strictly_fewer_decisions_than_the_refuted_one`, which compares
//! tableau runs rather than wall time.
//!
//! # Why the signature is the parameter
//!
//! `classes` counts the named classes the reasoner ranges over, and the answer is an
//! `n × n` verdict matrix whatever the reasoning underneath costs. Sweeping it is therefore
//! the only way to see whether the taxonomy is being DERIVED once or decided pair by pair:
//! the first grows with the axiom set, the second with the square of this parameter.

use std::sync::Arc;

use purrdf_testkit::bench::{Bench, BenchmarkId, bench_group, bench_main};

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId};
use purrdf_entail::reasoner::Reasoner;

/// The fixture namespace. `example.org` per the project rule: a bench mints no
/// vocabulary of its own, and a reserved-for-documentation authority is the only
/// one it may put in a term.
const EX: &str = "http://example.org/";

use purrdf_iri::vocab::owl::CLASS as OWL_CLASS;
use purrdf_iri::vocab::owl::EQUIVALENT_CLASS as OWL_EQUIVALENTCLASS;
use purrdf_iri::vocab::owl::INTERSECTION_OF as OWL_INTERSECTIONOF;
use purrdf_iri::vocab::owl::OBJECT_PROPERTY as OWL_OBJECTPROPERTY;
use purrdf_iri::vocab::owl::ON_PROPERTY as OWL_ONPROPERTY;
use purrdf_iri::vocab::owl::RESTRICTION as OWL_RESTRICTION;
use purrdf_iri::vocab::owl::SOME_VALUES_FROM as OWL_SOMEVALUESFROM;
use purrdf_iri::vocab::owl::TRANSITIVE_PROPERTY as OWL_TRANSITIVEPROPERTY;
use purrdf_iri::vocab::rdf::FIRST as RDF_FIRST;
use purrdf_iri::vocab::rdf::NIL as RDF_NIL;
use purrdf_iri::vocab::rdf::REST as RDF_REST;
use purrdf_iri::vocab::rdf::TYPE as RDF_TYPE;
use purrdf_iri::vocab::rdfs::SUB_CLASS_OF as RDFS_SUBCLASSOF;
use purrdf_iri::vocab::rdfs::SUB_PROPERTY_OF as RDFS_SUBPROPERTYOF;

/// A synthetic `EL` terminology over `classes` named classes.
///
/// A binary subclass tree (`C{i} ⊑ C{(i - 1) / 2}`) gives the taxonomy depth; one defined
/// class per eighth of the signature (`D{k} ≡ C{a} ⊓ C{b}`) gives the classifier real
/// conjunction work rather than a chain to walk; and `Parent ≡ ∃hasChild.C0` under
/// `hasChild ⊑ ancestorOf` with `ancestorOf` transitive exercises the existential,
/// role-hierarchy and role-composition rules.
fn terminology(classes: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(RDF_TYPE).unwrap();
    let first = b.intern_iri(RDF_FIRST).unwrap();
    let rest = b.intern_iri(RDF_REST).unwrap();
    let nil = b.intern_iri(RDF_NIL).unwrap();
    let sub_class = b.intern_iri(RDFS_SUBCLASSOF).unwrap();
    let sub_property = b.intern_iri(RDFS_SUBPROPERTYOF).unwrap();
    let class = b.intern_iri(OWL_CLASS).unwrap();
    let object_property = b.intern_iri(OWL_OBJECTPROPERTY).unwrap();
    let transitive = b.intern_iri(OWL_TRANSITIVEPROPERTY).unwrap();
    let equivalent = b.intern_iri(OWL_EQUIVALENTCLASS).unwrap();
    let intersection_of = b.intern_iri(OWL_INTERSECTIONOF).unwrap();
    let restriction = b.intern_iri(OWL_RESTRICTION).unwrap();
    let on_property = b.intern_iri(OWL_ONPROPERTY).unwrap();
    let some_values = b.intern_iri(OWL_SOMEVALUESFROM).unwrap();

    let named: Vec<TermId> = (0..classes)
        .map(|i| {
            let c = b.intern_iri(&format!("{EX}C{i}")).unwrap();
            b.push_quad(c, ty, class, None).unwrap();
            c
        })
        .collect();
    for i in 1..classes {
        b.push_quad(named[i], sub_class, named[(i - 1) / 2], None)
            .unwrap();
    }

    // A defined class per eighth of the signature, each an intersection of two tree nodes.
    for k in 0..classes / 8 {
        let defined = b.intern_iri(&format!("{EX}D{k}")).unwrap();
        b.push_quad(defined, ty, class, None).unwrap();
        let left = named[k % classes];
        let right = named[(k * 7 + 3) % classes];
        let tail = b
            .intern_blank(&format!("conj{k}b"), BlankScope::DEFAULT)
            .unwrap();
        let head = b
            .intern_blank(&format!("conj{k}a"), BlankScope::DEFAULT)
            .unwrap();
        b.push_quad(tail, first, right, None).unwrap();
        b.push_quad(tail, rest, nil, None).unwrap();
        b.push_quad(head, first, left, None).unwrap();
        b.push_quad(head, rest, tail, None).unwrap();
        let conjunction = b
            .intern_blank(&format!("conj{k}"), BlankScope::DEFAULT)
            .unwrap();
        b.push_quad(conjunction, intersection_of, head, None)
            .unwrap();
        b.push_quad(defined, equivalent, conjunction, None).unwrap();
    }

    let has_child = b.intern_iri(&format!("{EX}hasChild")).unwrap();
    let ancestor_of = b.intern_iri(&format!("{EX}ancestorOf")).unwrap();
    b.push_quad(has_child, ty, object_property, None).unwrap();
    b.push_quad(ancestor_of, ty, object_property, None).unwrap();
    b.push_quad(ancestor_of, ty, transitive, None).unwrap();
    b.push_quad(has_child, sub_property, ancestor_of, None)
        .unwrap();
    let parent = b.intern_iri(&format!("{EX}Parent")).unwrap();
    b.push_quad(parent, ty, class, None).unwrap();
    let some = b.intern_blank("someChild", BlankScope::DEFAULT).unwrap();
    b.push_quad(some, ty, restriction, None).unwrap();
    b.push_quad(some, on_property, has_child, None).unwrap();
    b.push_quad(some, some_values, named[0], None).unwrap();
    b.push_quad(parent, equivalent, some, None).unwrap();

    b.freeze().expect("freeze")
}

fn bench_classify(c: &mut Bench) {
    let mut group = c.benchmark_group("owl_direct_classify");
    for classes in [32usize, 128, 512] {
        let dataset = terminology(classes);
        let reasoner = Reasoner::new(&dataset).expect("reverse-map the terminology");
        let signature = reasoner.signature().len();
        group.bench_with_input(
            BenchmarkId::from_parameter(signature),
            &reasoner,
            |bencher, reasoner| {
                bencher.iter(|| reasoner.classify().expect("consistent"));
            },
        );
    }
    group.finish();
}

bench_group!(benches, bench_classify);
bench_main!(benches);
