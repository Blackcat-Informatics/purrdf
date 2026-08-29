// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RDF 1.2 **statement layer** as ShEx arcs.
//!
//! RDF 1.2 reifiers and statement annotations do NOT live in the frozen
//! dataset's quad table — they live in two separate side-tables. A matcher
//! that only walks `quads_for_pattern` is therefore blind to them: a shape
//! whose focus node IS a reifier would see an empty neighbourhood, and a
//! shape-map selector over an annotation predicate would select nothing.
//!
//! [`visit_quads`] is the statement-layer twin of
//! [`purrdf_core::DatasetView::quads_for_pattern`]: it yields the two
//! side-tables AS the virtual triples they denote,
//!
//! * `(reifier, rdf:reifies, <<triple>>)` — one per reifier binding;
//! * `(reifier, annotationPredicate, annotationObject)` — one per annotation;
//!
//! filtered by the same `(s, p, o)` probe. Both layers put the **reifier** in
//! subject position, so only a focus node that is itself a reifier gains
//! forward arcs; an ordinary subject's neighbourhood is unchanged.
//!
//! The rows are strictly additive — the side-tables are disjoint from `quads`,
//! so nothing is double-counted. Output order is the side-tables' frozen sort
//! order (deterministic), and every call site sorts and de-duplicates the
//! merged arc list anyway.
//!
//! ShEx 2.1 has no graph dimension (the validator probes every graph, i.e.
//! [`purrdf_core::GraphMatch::Any`]), so each row's graph slot is dropped here
//! exactly as the quad-table walk drops it.
//!
//! **Not to be confused with ShEx *schema* annotations** ([`crate::ast::Annotation`],
//! the `// predicate object` syntax), which are schema metadata and entirely
//! unrelated to the RDF 1.2 statement annotations this module reads.

use purrdf_core::{RdfDataset, TermId, TermRef};

/// `rdf:reifies` — the RDF 1.2 indirection edge from a reifier resource to the
/// triple term it reifies. A well-known RDF vocabulary IRI (PurRDF mints none
/// of its own); the reifier side-table denotes exactly this predicate, so it is
/// the only predicate a reifier row can carry.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Call `emit(s, p, o)` for every RDF 1.2 statement-layer virtual triple
/// matching the `(s, p, o)` probe (`None` = unconstrained).
///
/// Reifier rows come first, then annotation rows, each in its side-table's
/// frozen sorted order. The probe is applied with the same id-equality the
/// quad-table scan uses; unlike `quads_for_pattern`, the side-table walks are
/// not pre-narrowed by the probe, so the filtering is residual.
///
/// The walks are gated rather than blind, mirroring the SPARQL BGP matcher
/// (`purrdf-sparql-eval`'s `emit_virtual_candidates`):
///
/// * the reifier table is walked only when the probe's predicate *can* be
///   `rdf:reifies` and its object *can* be a triple term — a predicate bound
///   to any other IRI, or an object bound to an IRI/blank/literal, can match no
///   reifier row at all;
/// * the annotation table is indexed straight to the reifier's run
///   ([`RdfDataset::annotations_of_with_graph`], `O(log n)`) when the subject is
///   bound, and only scanned in full when it is not.
///
/// A dataset with no statement layer costs nothing: `rdf:reifies` is unknown
/// (so the reifier gate short-circuits) and the annotation table is empty.
pub(crate) fn visit_quads(
    data: &RdfDataset,
    s: Option<TermId>,
    p: Option<TermId>,
    o: Option<TermId>,
    mut emit: impl FnMut(TermId, TermId, TermId),
) {
    // ── reifier rows: (reifier, rdf:reifies, <<triple>>) ────────────────────
    // A dataset holding any reifier always interns `rdf:reifies` (the builder
    // does so on push), so a `None` here coincides with an empty reifier table.
    if let Some(reifies) = data.term_id_by_iri(RDF_REIFIES) {
        let predicate_can_reify = p.is_none_or(|id| id == reifies);
        // Every reifier row's object is a triple term, so a bound object that
        // is not one makes the whole walk pointless.
        let object_can_be_triple_term =
            o.is_none_or(|id| matches!(data.resolve(id), TermRef::Triple { .. }));
        if predicate_can_reify && object_can_be_triple_term {
            for quad in data.reifier_quads() {
                if s.is_none_or(|id| quad.s == id) && o.is_none_or(|id| quad.o == id) {
                    emit(quad.s, quad.p, quad.o);
                }
            }
        }
    }

    // ── annotation rows: (reifier, predicate, object) ───────────────────────
    // The subject of an annotation row IS its reifier key, so a bound subject
    // addresses one contiguous run instead of the whole table.
    match s {
        Some(reifier) => {
            for (pred, obj, _graph) in data.annotations_of_with_graph(reifier) {
                if p.is_none_or(|id| pred == id) && o.is_none_or(|id| obj == id) {
                    emit(reifier, pred, obj);
                }
            }
        }
        None => {
            for quad in data.annotation_quads() {
                if p.is_none_or(|id| quad.p == id) && o.is_none_or(|id| quad.o == id) {
                    emit(quad.s, quad.p, quad.o);
                }
            }
        }
    }
}
