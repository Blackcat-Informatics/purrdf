// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! OWL-Direct CONSISTENCY benchmark over the shape whose search cost was the defect.
//!
//! The fixture is the equivalence-over-untyped-restrictions ontology, replicated: an
//! `owl:equivalentClass` over two untyped restrictions — a `∀`-restriction whose filler is an
//! intersection, and an exact cardinality — beside an `owl:inverseOf` and an `rdfs:range`, with
//! one typed individual per block. Seventeen triples of it once exhausted the search budget
//! outright, because the CONVERSE direction of that equivalence has an antecedent no faithful
//! absorption can guard and so reaches the search as a disjunction every node must resolve.
//!
//! # Two shapes, and the difference between them is the whole point
//!
//! Each block is generated twice: once as the `owl:equivalentClass` the equivalence-over-
//! untyped-restrictions ontology states, and once as the CONTROL — the same restrictions
//! asserted with `rdfs:subClassOf`, which is one
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
//! `blocks` is how many copies of the shape the ontology carries. Each copy has its own class
//! and its own individual and shares the two role axioms, so the ABox and the TBox grow
//! together the way a real ontology's do.
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
//! seen rather than needing to be re-measured by hand every time it matters.
//!
//! # The third group: the same blocks CO-TYPED on one individual
//!
//! The two groups above give every block its own individual, so the blocks stand beside each
//! other. The `stacked` group asserts all `n` of them of ONE individual, over `n` disjoint
//! vocabularies, and that single change is a different cost class: the disjunctions interleave
//! on one node instead of nesting under separate roots.
//!
//! The measured curve, stated as it came out rather than as a speedup. Rounds and WORK units
//! at 1/2/4/8 blocks: independent 14/27/71/231 rounds and 1,791 / 6,108 / 26,337 / 144,363
//! units; stacked 14/84/836/10,500 rounds and 1,791 / 23,283 / 471,649 / 12,724,975 units
//! (the two-block cost is the ledger's `co-typed-equivalence-blocks` row). Every stacked size
//! here decides inside its work cap (`work_cap` in the decision core); the curve grows by
//! about one and a half per added block, 38.4 million units at ten, and a caller who narrows
//! the cap gets `unknown` under `completeness budget-exhausted` with `work` exactly equal to
//! `work-budget`.
//!
//! So the eight-block stacked timing below is a decision, as the eight-block independent one
//! is: the shape whose cost the round count could not see has a number, the number is bounded
//! by the work cap, and at eight blocks it is far inside it.

use std::sync::Arc;

use purrdf_testkit::bench::{Bench, BenchmarkId, bench_group, bench_main};

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId};
use purrdf_entail::reasoner::{Reasoner, Verdict};

/// The fixture namespace. `example.org` per the project rule: a bench mints no vocabulary of
/// its own, and a reserved-for-documentation authority is the only one it may put in a term.
const EX: &str = "http://example.org/";

use purrdf_iri::vocab::owl::ALL_VALUES_FROM as OWL_ALLVALUESFROM;
use purrdf_iri::vocab::owl::CARDINALITY as OWL_CARDINALITY;
use purrdf_iri::vocab::owl::EQUIVALENT_CLASS as OWL_EQUIVALENTCLASS;
use purrdf_iri::vocab::owl::INTERSECTION_OF as OWL_INTERSECTIONOF;
use purrdf_iri::vocab::owl::INVERSE_OF as OWL_INVERSEOF;
use purrdf_iri::vocab::owl::MAX_QUALIFIED_CARDINALITY as OWL_MAXQUALIFIEDCARDINALITY;
use purrdf_iri::vocab::owl::ON_CLASS as OWL_ONCLASS;
use purrdf_iri::vocab::owl::ON_PROPERTY as OWL_ONPROPERTY;
use purrdf_iri::vocab::rdf::FIRST as RDF_FIRST;
use purrdf_iri::vocab::rdf::NIL as RDF_NIL;
use purrdf_iri::vocab::rdf::REST as RDF_REST;
use purrdf_iri::vocab::rdf::TYPE as RDF_TYPE;
use purrdf_iri::vocab::rdfs::RANGE as RDFS_RANGE;
use purrdf_iri::vocab::rdfs::SUB_CLASS_OF as RDFS_SUBCLASSOF;
use purrdf_xsd::datatype::XSD_NON_NEGATIVE_INTEGER;

/// Which way each block states its restrictions, and whose individual it is asserted of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// `owl:equivalentClass` — two inclusions, one of which cannot be absorbed. One
    /// individual per block, so the blocks stand beside each other.
    Equivalence,
    /// `rdfs:subClassOf` — one inclusion, absorbed into guarded clauses. The control.
    SubClass,
    /// `owl:equivalentClass` again, but every block asserted of ONE individual, over its own
    /// vocabulary — the co-typed shape whose per-round work the round cap cannot see.
    Stacked,
}

impl Shape {
    /// The name the benchmark group reports this shape under.
    const fn label(self) -> &'static str {
        match self {
            Self::Equivalence => "owl_direct_consistency_equivalence",
            Self::SubClass => "owl_direct_consistency_subclass",
            Self::Stacked => "owl_direct_consistency_stacked",
        }
    }

    /// The predicate this shape states its two restrictions under.
    const fn states(self, equivalent: TermId, sub_class: TermId) -> TermId {
        match self {
            Self::Equivalence | Self::Stacked => equivalent,
            Self::SubClass => sub_class,
        }
    }

    /// Whether every block is asserted of the SAME individual.
    const fn co_typed(self) -> bool {
        matches!(self, Self::Stacked)
    }
}

/// `blocks` copies of the ∀-equivalence shape, stated as `shape` says.
///
/// The restrictions deliberately carry NO `rdf:type owl:Restriction`: they are restrictions by
/// their `owl:onProperty` / `owl:allValuesFrom` / `owl:cardinality` triples alone, which is
/// legal OWL 2 RDF and is what the equivalence-over-untyped-restrictions ontology states.
/// Retyping them would bench a different parse.
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

    let states: TermId = shape.states(equivalent, sub_class);

    let one = b.intern_literal(RdfLiteral {
        lexical_form: "1".to_owned(),
        datatype: Some(XSD_NON_NEGATIVE_INTEGER.to_owned()),
        language: None,
        direction: None,
    });

    for k in 0..blocks {
        // The role axioms: `r ≡ ri⁻` with a range on `ri`, so the universal obligations a
        // block derives flow back through an inverse rather than dead-ending at the
        // successor. SHARED by every block in the two side-by-side groups, and per-block in
        // the co-typed one — a co-typed block needs its own vocabulary, or the `n` copies
        // collapse into one concept and the shape being benched disappears.
        let tag = if shape.co_typed() {
            k.to_string()
        } else {
            String::new()
        };
        let r = b.intern_iri(&format!("{EX}r{tag}"));
        let ri = b.intern_iri(&format!("{EX}ri{tag}"));
        let p = b.intern_iri(&format!("{EX}p{tag}"));
        let c = b.intern_iri(&format!("{EX}c{tag}"));
        let s = b.intern_iri(&format!("{EX}S{tag}"));
        let d = b.intern_iri(&format!("{EX}D{tag}"));
        b.push_quad(r, inverse_of, ri, None);
        b.push_quad(ri, range, s, None);

        let class = b.intern_iri(&format!("{EX}A{k}"));
        // The one difference the third group is about: every block on ONE individual.
        let individual = if shape.co_typed() {
            b.intern_iri(&format!("{EX}a"))
        } else {
            b.intern_iri(&format!("{EX}a{k}"))
        };

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

/// A spy-point ontology exercising the nominal-introduction (`NN`/`NI`) rule.
///
/// `p owl:inverseOf invP`; everything is `p`-related to the nominal `spy`
/// (`⊤ ⊑ ∃p.{spy}`); `spy` bounds its `invP`-successors at `bound` (`≤bound invP.⊤`, i.e. at
/// most `bound` `p`-predecessors, so the domain has at most `bound` elements); and an individual
/// `u` is forced to `bound` pairwise-distinct `r`-successors (`≥bound r.⊤`). The bound fits, so
/// the ontology is CONSISTENT and the search runs the rule to completion — minting `bound`
/// reserved roots and folding the blockable predecessors into them — rather than short-circuiting
/// on a clash. This is the cost the nominal-introduction path adds, with somewhere to be seen.
fn nn_ontology(bound: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(RDF_TYPE);
    let first = b.intern_iri(RDF_FIRST);
    let rest = b.intern_iri(RDF_REST);
    let nil = b.intern_iri(RDF_NIL);
    let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
    let inverse_of = b.intern_iri(OWL_INVERSEOF);
    let one_of = b.intern_iri("http://www.w3.org/2002/07/owl#oneOf");
    let on_property = b.intern_iri(OWL_ONPROPERTY);
    let some_values = b.intern_iri("http://www.w3.org/2002/07/owl#someValuesFrom");
    let max_cardinality = b.intern_iri("http://www.w3.org/2002/07/owl#maxCardinality");
    let min_cardinality = b.intern_iri("http://www.w3.org/2002/07/owl#minCardinality");
    let thing = b.intern_iri("http://www.w3.org/2002/07/owl#Thing");

    let p = b.intern_iri(&format!("{EX}p"));
    let inv_p = b.intern_iri(&format!("{EX}invP"));
    let r = b.intern_iri(&format!("{EX}r"));
    let spy = b.intern_iri(&format!("{EX}spy"));
    let u = b.intern_iri(&format!("{EX}u"));
    b.push_quad(p, inverse_of, inv_p, None);

    let count = |b: &mut RdfDatasetBuilder, n: usize| {
        b.intern_literal(RdfLiteral {
            lexical_form: n.to_string(),
            datatype: Some(XSD_NON_NEGATIVE_INTEGER.to_owned()),
            language: None,
            direction: None,
        })
    };

    // ⊤ ⊑ ∃p.{spy}.
    let one = b.intern_blank("oneof", BlankScope::DEFAULT);
    b.push_quad(one, first, spy, None);
    b.push_quad(one, rest, nil, None);
    let enum_class = b.intern_blank("enum", BlankScope::DEFAULT);
    b.push_quad(enum_class, one_of, one, None);
    let some = b.intern_blank("some", BlankScope::DEFAULT);
    b.push_quad(some, on_property, p, None);
    b.push_quad(some, some_values, enum_class, None);
    b.push_quad(thing, sub_class, some, None);

    // spy : ≤bound invP.⊤.
    let bound_lit = count(&mut b, bound);
    let at_most = b.intern_blank("atmost", BlankScope::DEFAULT);
    b.push_quad(at_most, on_property, inv_p, None);
    b.push_quad(at_most, max_cardinality, bound_lit, None);
    b.push_quad(spy, ty, at_most, None);

    // u : ≥bound r.⊤.
    let min_lit = count(&mut b, bound);
    let at_least = b.intern_blank("atleast", BlankScope::DEFAULT);
    b.push_quad(at_least, on_property, r, None);
    b.push_quad(at_least, min_cardinality, min_lit, None);
    b.push_quad(u, ty, at_least, None);

    b.freeze().expect("freeze")
}

/// `n` individuals in one `p`-chain, every one typed `C`, with `C ⊑ ∀p.C`: the universal fires
/// along every edge, so every round reads every node's `p`-neighbourhood.
///
/// The shape a large ABox reaches the search in. A neighbourhood read once walked the WHOLE
/// edge vector, so a round cost the node count times the edge count; it now walks the node's
/// own indexed edges, so a round costs the node count times the degree (here, two).
fn role_edge_ontology(n: usize) -> Arc<RdfDataset> {
    role_ontology(n, (1..n).map(|i| (i - 1, i)))
}

/// `n` individuals on a ring lattice of even degree `k`: individual `i` is `p`-linked to the
/// `k / 2` individuals after it (modulo `n`), so every node has exactly `k` incident edges and
/// the ABox holds `n · k / 2` of them. Same TBox as [`role_edge_ontology`].
fn regular_role_ontology(n: usize, k: usize) -> Arc<RdfDataset> {
    role_ontology(
        n,
        (0..n).flat_map(move |i| (1..=k / 2).map(move |j| (i, (i + j) % n))),
    )
}

/// `n` individuals in a `p`-star: one hub linked to every other individual, so the hub's
/// degree is `n − 1` and every leaf's is one. Same TBox as [`role_edge_ontology`].
fn star_role_ontology(n: usize) -> Arc<RdfDataset> {
    role_ontology(n, (1..n).map(|i| (0, i)))
}

/// `n` individuals typed `C`, with `C ⊑ ∀p.C`, and one `p` assertion per `(from, to)` pair.
fn role_ontology(n: usize, links: impl IntoIterator<Item = (usize, usize)>) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(RDF_TYPE);
    let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
    let on_property = b.intern_iri(OWL_ONPROPERTY);
    let all_values = b.intern_iri(OWL_ALLVALUESFROM);
    let p = b.intern_iri(&format!("{EX}p"));
    let c = b.intern_iri(&format!("{EX}C"));
    let every = b.intern_blank("every", BlankScope::DEFAULT);
    b.push_quad(every, on_property, p, None);
    b.push_quad(every, all_values, c, None);
    b.push_quad(c, sub_class, every, None);
    let individuals: Vec<TermId> = (0..n).map(|i| b.intern_iri(&format!("{EX}i{i}"))).collect();
    for &individual in &individuals {
        b.push_quad(individual, ty, c, None);
    }
    for (from, to) in links {
        b.push_quad(individuals[from], p, individuals[to], None);
    }
    b.freeze().expect("freeze")
}

/// `abox` individuals in one `p`-chain, every one `Q` with `Q ⊑ ∀p.Q`, beside `choices`
/// individuals each bounded `≤2 r.F` — a qualified at-most restriction — over three asserted
/// `r`-successors typed `F`: each bound is a case split over which two successors to identify,
/// so deciding it makes `choices` choices while the chain sits saturated beside them.
fn choices_ontology(abox: usize, choices: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(RDF_TYPE);
    let sub_class = b.intern_iri(RDFS_SUBCLASSOF);
    let on_property = b.intern_iri(OWL_ONPROPERTY);
    let all_values = b.intern_iri(OWL_ALLVALUESFROM);
    let max_qualified = b.intern_iri(OWL_MAXQUALIFIEDCARDINALITY);
    let on_class = b.intern_iri(OWL_ONCLASS);
    let p = b.intern_iri(&format!("{EX}p"));
    let r = b.intern_iri(&format!("{EX}r"));
    let q = b.intern_iri(&format!("{EX}Q"));
    let f = b.intern_iri(&format!("{EX}F"));
    let every = b.intern_blank("every", BlankScope::DEFAULT);
    b.push_quad(every, on_property, p, None);
    b.push_quad(every, all_values, q, None);
    b.push_quad(q, sub_class, every, None);
    let two = b.intern_literal(RdfLiteral {
        lexical_form: "2".to_owned(),
        datatype: Some(XSD_NON_NEGATIVE_INTEGER.to_owned()),
        language: None,
        direction: None,
    });
    let bounded = b.intern_blank("bounded", BlankScope::DEFAULT);
    b.push_quad(bounded, on_property, r, None);
    b.push_quad(bounded, max_qualified, two, None);
    b.push_quad(bounded, on_class, f, None);
    let chain: Vec<TermId> = (0..abox)
        .map(|i| b.intern_iri(&format!("{EX}i{i}")))
        .collect();
    for &individual in &chain {
        b.push_quad(individual, ty, q, None);
    }
    for pair in chain.windows(2) {
        b.push_quad(pair[0], p, pair[1], None);
    }
    for c in 0..choices {
        let a = b.intern_iri(&format!("{EX}a{c}"));
        b.push_quad(a, ty, bounded, None);
        for k in 0..3 {
            let successor = b.intern_iri(&format!("{EX}a{c}s{k}"));
            b.push_quad(successor, ty, f, None);
            b.push_quad(a, r, successor, None);
        }
    }
    b.freeze().expect("freeze")
}

/// Report-only bench of a CHOICE-HEAVY search swept over the size of the ABox beside it and the
/// number of choices it makes. See [`choices_ontology`].
///
/// The quantity to read is per choice: `(t(abox, choices) − t(abox, 0)) / choices`, the time the
/// choices add over the same ABox without them. A choice clones its level — one pointer per
/// persistent structure in the completion graph — re-matches the region its assertion reaches,
/// re-blocks what it wrote, and asks the open-disjunction index for the next branch point. The
/// decision core's test `a_choice_touches_the_same_beside_a_small_and_a_large_abox` pins the
/// deterministic side of that: the nodes a choice touches and the work it spends are
/// IDENTICAL beside 1,000 and 16,000 nodes.
///
/// The clock is not flat, and the measured growth is stated rather than smoothed. Counted with
/// `perf stat` over 512 choices (user space, per decision, construction subtracted): about 97
/// thousand instructions a choice beside 1,000 nodes, 185 thousand beside 16,000 and 175
/// thousand beside 64,000 — doubling once, as the persistent vectors' radix trees grow a level
/// past sixteen thousand elements, then holding — and 28 / 56 / 92 thousand cycles. The cycles
/// grow faster than the instructions because the same few dozen nodes a choice touches sit in
/// a larger, colder working set; the wall clock grows further still, as copy-on-write leaves
/// take fresh pages. On a busy host the 16,000-node row reads four to six times the
/// 1,000-node one per choice.
fn bench_choices(c: &mut Bench) {
    let mut group = c.benchmark_group("owl_direct_consistency_choices");
    for &abox in &[1_000usize, 4_000, 16_000] {
        for &choices in &[0usize, 64, 256] {
            let dataset = choices_ontology(abox, choices);
            let reasoner = Reasoner::new(&dataset).expect("reverse-map the choice ontology");
            group.bench_with_input(
                BenchmarkId::new(format!("abox{abox}"), format!("choices{choices}")),
                &reasoner,
                |bencher, reasoner| {
                    bencher.iter(|| reasoner.consistency());
                },
            );
        }
    }
    group.finish();
}

/// Report-only bench of a role-edge ABox of growing size: the per-round cost of reading
/// neighbourhoods. See [`role_edge_ontology`].
fn bench_role_edges(c: &mut Bench) {
    let mut group = c.benchmark_group("owl_direct_consistency_role_edges");
    for &n in &[1_000usize, 4_000, 16_000] {
        let dataset = role_edge_ontology(n);
        let reasoner =
            decided(Reasoner::new(&dataset).expect("reverse-map the role-edge ontology"));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &reasoner,
            |bencher, reasoner| {
                bencher.iter(|| reasoner.consistency());
            },
        );
    }
    group.finish();
}

/// Refuse to time a role-edge fixture that does not DECIDE: every one is consistent, and a run
/// that reached the work cap and answered `unknown` would time the cap, not the reads.
fn decided(reasoner: Reasoner) -> Reasoner {
    assert_eq!(
        reasoner.consistency().into_answer(),
        Verdict::True,
        "a role-edge fixture must decide consistent, not reach a ceiling"
    );
    reasoner
}

/// The individual count the degree sweep holds fixed.
const DEGREE_SWEEP_NODES: usize = 2_000;

/// Report-only bench of the same TBox at a FIXED individual count while the degree varies:
/// ring lattices of degree 2, 8 and 32 (see [`regular_role_ontology`]), and a star whose hub
/// has degree `n − 1` (see [`star_role_ontology`]).
///
/// [`bench_role_edges`] grows the node count and the edge count together, so on its own it
/// cannot separate the two factors of a round's read cost. This group holds `n` fixed. A round
/// of per-node reads costs the sum of the degrees — `n · k` on the lattice — where a
/// whole-edge-vector read costs `n` times the edge count, `n² · k / 2`: both grow with `k` here,
/// and the difference is the factor of `n` that [`bench_role_edges`] exposes. The star has the
/// degree-2 lattice's edge count and degree sum but one node of degree `n − 1`, so it should
/// cost what that lattice costs: a hub is paid for once, on the hub, not on every node.
///
/// Measured once with `--quick` (indicative only; the machine is not quiet), medians, indexed
/// reads against the same build with `step` reverted to the whole-edge scan: regular 2 / 8 / 32
/// at 1.04 / 1.54 / 3.74 ms against 8.40 / 32.1 / 127 ms, star 0.93 ms against 8.47 ms. The
/// indexed curve is below linear in `k` because the per-node cost of a round dominates at low
/// degree; past it both grow linearly in `k`, the scan at about 4 ms per unit of degree and the
/// indexed reads at about 0.09 ms.
fn bench_role_degree(c: &mut Bench) {
    let mut group = c.benchmark_group("owl_direct_consistency_role_degree");
    let n = DEGREE_SWEEP_NODES;
    for &k in &[2usize, 8, 32] {
        let dataset = regular_role_ontology(n, k);
        let reasoner =
            decided(Reasoner::new(&dataset).expect("reverse-map the ring-lattice ontology"));
        group.bench_with_input(
            BenchmarkId::new("regular", k),
            &reasoner,
            |bencher, reasoner| {
                bencher.iter(|| reasoner.consistency());
            },
        );
    }
    let dataset = star_role_ontology(n);
    let reasoner = decided(Reasoner::new(&dataset).expect("reverse-map the star ontology"));
    group.bench_with_input(
        BenchmarkId::new("star", n - 1),
        &reasoner,
        |bencher, reasoner| {
            bencher.iter(|| reasoner.consistency());
        },
    );
    group.finish();
}

/// Report-only bench of the nominal-introduction path over spy-point ontologies of growing bound.
fn bench_nominal_introduction(c: &mut Bench) {
    let mut group = c.benchmark_group("owl_direct_consistency_nominal_introduction");
    for &bound in &[1usize, 2, 4] {
        let dataset = nn_ontology(bound);
        let reasoner = Reasoner::new(&dataset).expect("reverse-map the spy-point ontology");
        group.bench_with_input(
            BenchmarkId::from_parameter(bound),
            &reasoner,
            |bencher, reasoner| {
                bencher.iter(|| reasoner.consistency());
            },
        );
    }
    group.finish();
}

fn bench_consistency(c: &mut Bench) {
    for shape in [Shape::Equivalence, Shape::SubClass, Shape::Stacked] {
        let mut group = c.benchmark_group(shape.label());
        // The co-typed group stops at eight: its work grows about one and a half times per
        // added block, so sixteen would spend most of a sample budget deciding one case the
        // eight-block case already characterizes.
        let sizes: &[usize] = if shape.co_typed() {
            &[1, 2, 4, 8]
        } else {
            &[1, 4, 16]
        };
        for &blocks in sizes {
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

/// Report-only bench of what PROOF RECORDING costs, both modes side by side.
///
/// Recording is opt-in, so the interesting number is the difference between the two arms — the
/// RDFC-1.0 canonicalization `Reasoner::with_proofs` pays for the ontology identity, the
/// clausification contract each session derives, and the instrumented search itself. Both arms
/// measure construction AND one consistency call, because the canonicalization happens at
/// construction and a bench that reused a reasoner would hide the larger half.
///
/// Nothing is asserted. A saving is a number this prints, not a claim a test makes; the
/// obligation the tests DO carry is that the two arms decide identically, which
/// `a_proofs_off_service_answer_is_identical_to_a_proofs_on_one` pins.
fn bench_proof_recording(c: &mut Bench) {
    let mut group = c.benchmark_group("owl_direct_consistency_proof_recording");
    for &blocks in &[1usize, 4, 16] {
        let dataset = ontology(blocks, Shape::SubClass);
        group.bench_with_input(
            BenchmarkId::new("off", blocks),
            &dataset,
            |bencher, dataset| {
                bencher.iter(|| {
                    Reasoner::new(dataset)
                        .expect("reverse-map the ontology")
                        .consistency()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("on", blocks),
            &dataset,
            |bencher, dataset| {
                bencher.iter(|| {
                    Reasoner::with_proofs(dataset)
                        .expect("reverse-map the ontology")
                        .consistency()
                });
            },
        );
    }
    group.finish();
}

bench_group!(
    benches,
    bench_consistency,
    bench_role_edges,
    bench_role_degree,
    bench_nominal_introduction,
    bench_proof_recording,
    bench_choices
);
bench_main!(benches);
