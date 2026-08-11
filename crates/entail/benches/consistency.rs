// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! OWL-Direct CONSISTENCY benchmark over the shape whose search cost was the defect.
//!
//! The fixture is the reported ontology, replicated: an `owl:equivalentClass` over two
//! untyped restrictions — a `∀`-restriction whose filler is an intersection, and an exact
//! cardinality — beside an `owl:inverseOf` and an `rdfs:range`, with one typed individual per
//! block. Seventeen triples of it once exhausted the search budget outright, because the
//! CONVERSE direction of that equivalence has an antecedent no faithful absorption can guard
//! and so reaches the search as a disjunction every node must resolve.
//!
//! # Two shapes, and the difference between them is the whole point
//!
//! Each block is generated twice: once as the `owl:equivalentClass` the report carried, and
//! once as the CONTROL — the same restrictions asserted with `rdfs:subClassOf`, which is one
//! direction rather than two and absorbs into guarded clauses that never branch. Benching only
//! the expensive shape would show a number with nothing to read it against; benching both
//! shows what the case splits cost, at each size, in the same run.
//!
//! # Report-only
//!
//! This asserts nothing and gates nothing. It exists so a later change to the clausification,
//! the absorption pass or the `⊔`-rule's disjunct order has a NUMBER to move — not so that a
//! speedup can be claimed. The measuring machine is not quiet, so the timings are indicative
//! only, and every claim this workspace makes about the search's cost is made where it can be
//! made exactly: `purrdf-validate`'s step ledger pins the rounds, peak nodes, case splits and
//! branch depth of these very shapes as literals, and the differential suite in
//! `owl_dl::oracle` ceilings each generated corpus's round total. Those are counts over a
//! deterministic search; this is a clock.
//!
//! # Why blocks are the parameter, and what the sweep actually shows
//!
//! `blocks` is how many independent copies of the shape the ontology carries. Each copy has
//! its own class and its own individual and shares the two role axioms, so the ABox and the
//! TBox grow together the way a real ontology's do.
//!
//! The two shapes answer that sweep very differently, and the honest statement of the
//! difference is not "the equivalence is now cheap". The control is FLAT in rounds — three,
//! at every size, because a guarded clause fires where its guard holds and never splits — and
//! grows only in nodes. The equivalence is not: its case splits go 3, 48, 768 across the three
//! sizes benched, which is quadratic in the blocks, because the converse inclusion is a
//! disjunction every node must resolve and the blocks are independent, so their splits nest.
//! Deciding 17 triples of it costs 11 rounds where it once cost the entire budget; deciding
//! sixteen copies costs 821. That is a search whose cost is bounded and legible rather than
//! one that is linear, and this bench is here so the shape of that curve has somewhere to be
//! seen rather than being re-derived from a report.

use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId};
use purrdf_entail::reasoner::Reasoner;

/// The fixture namespace. `example.org` per the project rule: a bench mints no vocabulary of
/// its own, and a reserved-for-documentation authority is the only one it may put in a term.
const EX: &str = "http://example.org/";

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
const OWL_INTERSECTIONOF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
const OWL_ONPROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
const OWL_ALLVALUESFROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
const OWL_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#cardinality";
const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";

/// Which way each block states its restrictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// `owl:equivalentClass` — two inclusions, one of which cannot be absorbed.
    Equivalence,
    /// `rdfs:subClassOf` — one inclusion, absorbed into guarded clauses. The control.
    SubClass,
}

impl Shape {
    /// The name the benchmark group reports this shape under.
    const fn label(self) -> &'static str {
        match self {
            Self::Equivalence => "owl_direct_consistency_equivalence",
            Self::SubClass => "owl_direct_consistency_subclass",
        }
    }
}

/// `blocks` copies of the reported shape, stated as `shape` says.
///
/// The restrictions deliberately carry NO `rdf:type owl:Restriction`: they are restrictions by
/// their `owl:onProperty` / `owl:allValuesFrom` / `owl:cardinality` triples alone, which is
/// legal OWL 2 RDF and is what the reported ontology looked like. Retyping them would bench a
/// different parse.
fn ontology(blocks: usize, shape: Shape) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(RDF_TYPE);
    let first = b.intern_iri(RDF_FIRST);
    let rest = b.intern_iri(RDF_REST);
    let nil = b.intern_iri(RDF_NIL);
    let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
    let range = b.intern_iri(RDFS_RANGE);
    let equivalent = b.intern_iri(OWL_EQUIVALENTCLASS);
    let inverse_of = b.intern_iri(OWL_INVERSEOF);
    let intersection_of = b.intern_iri(OWL_INTERSECTIONOF);
    let on_property = b.intern_iri(OWL_ONPROPERTY);
    let all_values = b.intern_iri(OWL_ALLVALUESFROM);
    let cardinality = b.intern_iri(OWL_CARDINALITY);

    // The predicate each block states its two restrictions under — the one difference
    // between the two shapes.
    let states: TermId = match shape {
        Shape::Equivalence => equivalent,
        Shape::SubClass => sub_class,
    };

    // The role axioms, shared by every block: `r ≡ ri⁻` with a range on `ri`, so the
    // universal obligations a block derives flow back through an inverse rather than
    // dead-ending at the successor.
    let r = b.intern_iri(&format!("{EX}r"));
    let ri = b.intern_iri(&format!("{EX}ri"));
    let p = b.intern_iri(&format!("{EX}p"));
    let c = b.intern_iri(&format!("{EX}c"));
    let s = b.intern_iri(&format!("{EX}S"));
    let d = b.intern_iri(&format!("{EX}D"));
    b.push_quad(r, inverse_of, ri, None);
    b.push_quad(ri, range, s, None);

    let one = b.intern_literal(RdfLiteral {
        lexical_form: "1".to_owned(),
        datatype: Some(XSD_NON_NEGATIVE_INTEGER.to_owned()),
        language: None,
        direction: None,
    });

    for k in 0..blocks {
        let class = b.intern_iri(&format!("{EX}A{k}"));
        let individual = b.intern_iri(&format!("{EX}a{k}"));

        // `∀r.(S ⊓ ∀p.D)`, with the intersection as an RDF collection.
        let inner = b.intern_blank(&format!("inner{k}"), BlankScope::DEFAULT);
        b.push_quad(inner, on_property, p, None);
        b.push_quad(inner, all_values, d, None);
        let tail = b.intern_blank(&format!("tail{k}"), BlankScope::DEFAULT);
        b.push_quad(tail, first, inner, None);
        b.push_quad(tail, rest, nil, None);
        let head = b.intern_blank(&format!("head{k}"), BlankScope::DEFAULT);
        b.push_quad(head, first, s, None);
        b.push_quad(head, rest, tail, None);
        let conjunction = b.intern_blank(&format!("and{k}"), BlankScope::DEFAULT);
        b.push_quad(conjunction, intersection_of, head, None);
        let universal = b.intern_blank(&format!("all{k}"), BlankScope::DEFAULT);
        b.push_quad(universal, on_property, r, None);
        b.push_quad(universal, all_values, conjunction, None);

        // `=1 c` — an exact cardinality on a second property.
        let counted = b.intern_blank(&format!("exactly{k}"), BlankScope::DEFAULT);
        b.push_quad(counted, on_property, c, None);
        b.push_quad(counted, cardinality, one, None);

        b.push_quad(class, states, universal, None);
        b.push_quad(class, states, counted, None);
        b.push_quad(class, sub_class, s, None);
        b.push_quad(individual, ty, class, None);
    }

    b.freeze().expect("freeze")
}

fn bench_consistency(c: &mut Criterion) {
    for shape in [Shape::Equivalence, Shape::SubClass] {
        let mut group = c.benchmark_group(shape.label());
        for blocks in [1usize, 4, 16] {
            let dataset = ontology(blocks, shape);
            let reasoner = Reasoner::new(&dataset).expect("reverse-map the ontology");
            group.bench_with_input(
                BenchmarkId::from_parameter(blocks),
                &reasoner,
                |bencher, reasoner| {
                    bencher.iter(|| reasoner.consistency());
                },
            );
        }
        group.finish();
    }
}

criterion_group!(benches, bench_consistency);
criterion_main!(benches);
