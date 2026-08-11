// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! OWL-Direct CONSISTENCY benchmark over a SCHEMA-HEAVY ontology: many disjoint
//! `rdfs:domain`/`rdfs:range` pairs beside a data-range cardinality restriction, over a small
//! FIXED ABox.
//!
//! # What this shape is, and why it is not the `consistency` bench's shape
//!
//! `consistency` sweeps the ontology that once exhausted the round budget — a shape whose cost
//! is in CASE SPLITS, one disjunction per equivalence per node. This bench sweeps a different
//! axis: an ontology with no disjunctions at all, whose cost is in how many SCHEMA axioms a
//! search must consult per node rather than in how the search branches. Two mechanisms are
//! stacked here, because both are driven by the same knob — axiom count against a FIXED node
//! count — and both used to be linear in it:
//!
//! * `pairs` disjoint properties, each declared `rdfs:domain :Dᵢ` / `rdfs:range :Rᵢ` and never
//!   used by the ABox. Neither axiom has a class in its antecedent, so neither can be indexed
//!   by concept the way every other absorbed inclusion is; both land in the UNTRIGGERED clause
//!   set the search retries at every node of every round, over a role the completion graph
//!   usually has no edge for. Reading one asks the graph for that role's NEIGHBOURHOOD, which
//!   resolves the role's sub-role/inverse closure (its "achievers") before it can even report
//!   there is no edge — and before that closure was cached per role for the run, resolving it
//!   cost the same walk on the thousandth retry as on the first.
//! * ONE data property `:dp`, ALSO `rdfs:range xsd:integer`, that every one of the FIXED ABox's
//!   individuals is restricted by (`∃dp.xsd:integer`, i.e. `≥1 dp.xsd:integer` in the label).
//!   Deciding that restriction asks whether `dp`'s data range is narrowed by an unguarded
//!   `rdfs:range` clause — which, before those clauses were indexed by role, meant a scan of
//!   the WHOLE absorbed table, `pairs` entries and growing, at EVERY one of the fixed
//!   individuals, every round.
//!
//! # The axis: axiom count against a FIXED node count
//!
//! The ABox is the same ten individuals at every `pairs` value, each with its own `∃dp.xsd:
//! integer` restriction, so the completion graph's size does not move as the sweep runs — only
//! the schema does. That isolates the cost this bench exists to show: `work` (the exact,
//! deterministic figure `purrdf-validate`'s step ledger pins for this same shape) drops sharply
//! between the two mechanisms above and this wall-clock reading, because the dominant cost at
//! these sizes is the untriggered clause set itself — `pairs` axioms retried at every one of
//! the twenty completion-graph nodes, every round, which is `O(pairs)` by construction on
//! EITHER side of the achiever cache and was never the defect. The range-clause scan the
//! second mechanism narrows is the one that moved from `O(pairs)` to `O(1)` per restricted
//! individual; with only ten such individuals against up to eight hundred axioms, its share of
//! the total is real but not the dominant term here — a fixture with more Min-labelled nodes
//! relative to its axiom count would show it larger, at the cost of no longer holding the node
//! count fixed.
//!
//! # Report-only
//!
//! This asserts nothing and gates nothing, for the reasons `consistency`'s module doc gives at
//! length: the measuring machine is not quiet, so a number here is read as a curve's SHAPE
//! against itself run to run, never as a claimed speedup. What IS pinned as an exact count is
//! `schema-heavy-domain-range-pairs` in `purrdf-validate`'s step ledger — the same family of
//! shape at a size small enough to hand-verify, with `work` (not wall time) as the pinned
//! figure.
//!
//! The measured wall-clock curve, stated as it came out, `pairs` = 5 / 100 / 400 / 800 over the
//! fixed ten-individual ABox: 24 µs / 355 µs / 1.43 ms / 2.87 ms with the achiever cache and the
//! range-clause index both in place, against 29 µs / 420 µs / 1.61 ms / 3.38 ms with both
//! reverted — a modest, real reduction at this shape's ratio of axioms to restricted
//! individuals, not the dramatic end of the curve either mechanism can produce at a different
//! ratio.

use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder};
use purrdf_entail::reasoner::Reasoner;

/// The fixture namespace. `example.org` per the project rule: a bench mints no vocabulary of
/// its own, and a reserved-for-documentation authority is the only one it may put in a term.
const EX: &str = "http://example.org/";

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// How many individuals the FIXED ABox holds, whatever `pairs` is.
const ABOX_INDIVIDUALS: usize = 10;

/// `pairs` disjoint `rdfs:domain`/`rdfs:range` axioms (dead schema weight the achiever cache
/// bounds), plus ONE `rdfs:range xsd:integer` data property every one of the FIXED ABox's
/// individuals is restricted by (the range-clause scan the pre-index bounds).
fn ontology(pairs: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(RDF_TYPE);
    let domain = b.intern_iri(RDFS_DOMAIN);
    let range = b.intern_iri(RDFS_RANGE);
    let on_property = b.intern_iri(OWL_ON_PROPERTY);
    let some_values = b.intern_iri(OWL_SOME_VALUES_FROM);
    let xsd_integer = b.intern_iri(XSD_INTEGER);

    for i in 0..pairs {
        let p = b.intern_iri(&format!("{EX}p{i}"));
        let d = b.intern_iri(&format!("{EX}D{i}"));
        let r = b.intern_iri(&format!("{EX}R{i}"));
        b.push_quad(p, domain, d, None);
        b.push_quad(p, range, r, None);
    }

    // ONE data property, its own `rdfs:range xsd:integer` — an unguarded, absorbed range
    // clause exactly like every `p_i` above, except this is the one a Min restriction actually
    // asks `data_clashes` to narrow against.
    let dp = b.intern_iri(&format!("{EX}dp"));
    b.push_quad(dp, range, xsd_integer, None);

    // The FIXED ABox: `ABOX_INDIVIDUALS` individuals, each restricted `∃dp.xsd:integer` — a
    // `Min(1, dp, Data(xsd:integer))` label, one per individual, so the number of nodes
    // `data_clashes` must resolve the range clause for does not move as `pairs` grows.
    for i in 0..ABOX_INDIVIDUALS {
        let a = b.intern_iri(&format!("{EX}a{i}"));
        let restriction = b.intern_blank(&format!("some{i}"), BlankScope::DEFAULT);
        b.push_quad(restriction, on_property, dp, None);
        b.push_quad(restriction, some_values, xsd_integer, None);
        b.push_quad(a, ty, restriction, None);
    }

    b.freeze().expect("freeze")
}

fn bench_schema_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("owl_direct_consistency_schema_heavy");
    for &pairs in &[5_usize, 100, 400, 800] {
        let dataset = ontology(pairs);
        let reasoner = Reasoner::new(&dataset).expect("reverse-map the ontology");
        group.bench_with_input(
            BenchmarkId::from_parameter(pairs),
            &reasoner,
            |bencher, reasoner| {
                bencher.iter(|| reasoner.consistency());
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_schema_scan);
criterion_main!(benches);
