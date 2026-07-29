// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
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

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId};
use purrdf_entail::reasoner::Reasoner;

/// The fixture namespace. `example.org` per the project rule: a bench mints no
/// vocabulary of its own, and a reserved-for-documentation authority is the only
/// one it may put in a term.
const EX: &str = "http://example.org/";

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_OBJECTPROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const OWL_TRANSITIVEPROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_INTERSECTIONOF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
const OWL_ONPROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
const OWL_SOMEVALUESFROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";

/// A synthetic `EL` terminology over `classes` named classes.
///
/// A binary subclass tree (`C{i} ⊑ C{(i - 1) / 2}`) gives the taxonomy depth; one defined
/// class per eighth of the signature (`D{k} ≡ C{a} ⊓ C{b}`) gives the classifier real
/// conjunction work rather than a chain to walk; and `Parent ≡ ∃hasChild.C0` under
/// `hasChild ⊑ ancestorOf` with `ancestorOf` transitive exercises the existential,
/// role-hierarchy and role-composition rules.
fn terminology(classes: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(RDF_TYPE);
    let first = b.intern_iri(RDF_FIRST);
    let rest = b.intern_iri(RDF_REST);
    let nil = b.intern_iri(RDF_NIL);
    let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
    let sub_property = b.intern_iri(RDFS_SUBPROPERTYOF);
    let class = b.intern_iri(OWL_CLASS);
    let object_property = b.intern_iri(OWL_OBJECTPROPERTY);
    let transitive = b.intern_iri(OWL_TRANSITIVEPROPERTY);
    let equivalent = b.intern_iri(OWL_EQUIVALENTCLASS);
    let intersection_of = b.intern_iri(OWL_INTERSECTIONOF);
    let restriction = b.intern_iri(OWL_RESTRICTION);
    let on_property = b.intern_iri(OWL_ONPROPERTY);
    let some_values = b.intern_iri(OWL_SOMEVALUESFROM);

    let named: Vec<TermId> = (0..classes)
        .map(|i| {
            let c = b.intern_iri(&format!("{EX}C{i}"));
            b.push_quad(c, ty, class, None);
            c
        })
        .collect();
    for i in 1..classes {
        b.push_quad(named[i], sub_class, named[(i - 1) / 2], None);
    }

    // A defined class per eighth of the signature, each an intersection of two tree nodes.
    for k in 0..classes / 8 {
        let defined = b.intern_iri(&format!("{EX}D{k}"));
        b.push_quad(defined, ty, class, None);
        let left = named[k % classes];
        let right = named[(k * 7 + 3) % classes];
        let tail = b.intern_blank(&format!("conj{k}b"), BlankScope::DEFAULT);
        let head = b.intern_blank(&format!("conj{k}a"), BlankScope::DEFAULT);
        b.push_quad(tail, first, right, None);
        b.push_quad(tail, rest, nil, None);
        b.push_quad(head, first, left, None);
        b.push_quad(head, rest, tail, None);
        let conjunction = b.intern_blank(&format!("conj{k}"), BlankScope::DEFAULT);
        b.push_quad(conjunction, intersection_of, head, None);
        b.push_quad(defined, equivalent, conjunction, None);
    }

    let has_child = b.intern_iri(&format!("{EX}hasChild"));
    let ancestor_of = b.intern_iri(&format!("{EX}ancestorOf"));
    b.push_quad(has_child, ty, object_property, None);
    b.push_quad(ancestor_of, ty, object_property, None);
    b.push_quad(ancestor_of, ty, transitive, None);
    b.push_quad(has_child, sub_property, ancestor_of, None);
    let parent = b.intern_iri(&format!("{EX}Parent"));
    b.push_quad(parent, ty, class, None);
    let some = b.intern_blank("someChild", BlankScope::DEFAULT);
    b.push_quad(some, ty, restriction, None);
    b.push_quad(some, on_property, has_child, None);
    b.push_quad(some, some_values, named[0], None);
    b.push_quad(parent, equivalent, some, None);

    b.freeze().expect("freeze")
}

fn bench_classify(c: &mut Criterion) {
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

criterion_group!(benches, bench_classify);
criterion_main!(benches);
