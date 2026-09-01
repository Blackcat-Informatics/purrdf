// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! GTS adapter surface for `purrdf`.
//!
//! The oxigraph-free reader half (`read_graph`, `read_all_segments`,
//! `lookaside_from_graph`, …) lives in this adapter crate so `purrdf-core`
//! remains independent of transport. The oxigraph-FREE
//! [`flattened_dataset_from_bytes`] ( Task 4) is the load path the native
//! SPARQL conformance gate replays against the frozen goldens.

pub use crate::gts_core::*;
pub use crate::gts_verify::{ContentChainVerification, verify_content_chain};

use crate::RdfDiagnostic;
use crate::gts_resolve::ensure_terms_terminate;
use crate::native_codecs::ser_model::{SerGraph, SerReifierRow, SerTerm, SerTermKind};

/// Copy a real `purrdf_gts::model::Graph` (read from a GTS bundle) into the first-party
/// [`SerGraph`] the native statement-layer fold consumes. The two shapes mirror each
/// other field-for-field; this is a faithful, lossless copy of the terms, base quads,
/// reifier rows, and annotation rows (the only members the fold reads).
///
/// This is the GtsGraph→SerGraph bridge for the CONTAINER read path. purrdf_gts use is
/// allow-listed in this container file (the purrdf.gts bundle reader legitimately yields
/// a real `purrdf_gts::model::Graph`); the codec seam never sees the purrdf-gts model.
///
/// # Quoted triples
///
/// The two models spell a quoted triple's components differently and the difference is
/// load-bearing. `purrdf_gts::model::Term` states them in its own `triple` slot (wire
/// `"tt"`) — that is what [`crate::gts_write`] writes and what
/// `Graph::triple_of` prefers. A [`SerGraph`] has no such slot: EVERY producer of one
/// (the text parsers, the IR lowering in `serialize.rs`) spells a triple TERM as the
/// self-reifier sentinel — the term's `reifier` is its OWN id, and `reifiers` carries
/// `(id, (s, p, o), None)` — which the serializers render inline as `<<( s p o )>>` and
/// skip as an `rdf:reifies` output row. So `tt` is TRANSLATED into that spelling here.
/// Copying the term without it dropped the components on the floor: a modern `tt`-only
/// triple term rendered as `_:unbound_triple_N` on the text path and hard-failed the IR
/// fold with `native-codec-unbound-triple-term`.
///
/// The translated rows are placed AHEAD of the statement layer because
/// `SerGraph::reifier` returns the first row for an id and `tt` takes precedence over
/// the reifier indirection, exactly as `Graph::triple_of` orders them.
///
/// # Errors
///
/// Returns `gts-self-reaching-term` when the graph's term table lets a term resolve
/// through itself. Everything downstream of this bridge — the IR fold's `SerInterner`,
/// the N-Triples/TriG writers, the RDF/XML and JSON-LD serializers — walks a quoted
/// triple's components recursively with no depth bound, so an unchecked graph here is
/// a stack overflow, which aborts the process. A graph off the wire cannot contain such
/// a term; a caller-built one reaching the public `dataset_from_gts_graph` /
/// `flattened_dataset_from_gts_graph` can, and is refused here.
pub(crate) fn gts_to_ser(g: &purrdf_gts::model::Graph) -> Result<SerGraph, RdfDiagnostic> {
    ensure_terms_terminate(g)?;

    let mut self_bound: Vec<SerReifierRow> = Vec::new();
    let mut terms = Vec::with_capacity(g.terms.len());
    for (id, t) in g.terms.iter().enumerate() {
        let reifier = if t.kind != purrdf_gts::model::TermKind::Triple {
            t.reifier
        } else if let Some(triple) = t.triple {
            // Self-describing (`tt`): re-spell it under the term's own id.
            self_bound.push((id, triple, None));
            Some(id)
        } else if t.reifier.is_some_and(|rid| g.reifier(rid).is_some()) {
            // The legacy indirection already resolves through the statement layer;
            // leave the spelling alone so a genuine `rdf:reifies` row still emits.
            t.reifier
        } else if g.reifier(id).is_some() {
            // §7.1: a self-bound triple term may leave `rf` implicit, and the binding
            // is then keyed by the term's OWN id. The row is already in the statement
            // layer under that id, so naming it is the whole translation.
            Some(id)
        } else {
            // Genuinely unbound; the fold and the serializers report it as such.
            t.reifier
        };
        terms.push(SerTerm {
            kind: match t.kind {
                purrdf_gts::model::TermKind::Iri => SerTermKind::Iri,
                purrdf_gts::model::TermKind::Bnode => SerTermKind::Bnode,
                purrdf_gts::model::TermKind::Literal => SerTermKind::Literal,
                purrdf_gts::model::TermKind::Triple => SerTermKind::Triple,
            },
            value: t.value.clone(),
            datatype: t.datatype,
            lang: t.lang.clone(),
            direction: t.direction.clone(),
            reifier,
        });
    }

    let mut reifiers = self_bound;
    reifiers.extend(g.reifiers.iter().copied());
    Ok(SerGraph {
        terms,
        quads: g.quads.clone(),
        reifiers,
        annotations: g.annotations.clone(),
    })
}

/// Load a real `purrdf_gts::model::Graph` (read from a GTS bundle) into a frozen
/// [`RdfDataset`](crate::RdfDataset), **preserving** every named graph on its base
/// quads. This is the lossless container→dataset bridge for the release fold:
/// `purrdf_gts::model::Graph` → `gts_to_ser` → the native statement-layer fold
/// (`dataset_from_ser_graph`),
/// which re-binds the `rdf:reifies` reifier/annotation side-tables and keeps each base
/// quad's graph component. It is the native inverse of the old `to_nquads(&graph)` +
/// `parse_dataset(nquads, …)` round-trip — the SAME dataset content, with no codec text
/// in the middle — so a snapshot rebuilt from it is byte-identical to the round-trip's.
pub fn dataset_from_gts_graph(
    g: &purrdf_gts::model::Graph,
) -> Result<std::sync::Arc<crate::RdfDataset>, RdfDiagnostic> {
    let ser = gts_to_ser(g)?;
    crate::native_codecs::parse::dataset_from_ser_graph(&ser)
}

/// Load a GTS bundle into a frozen [`RdfDataset`](crate::RdfDataset) with **every**
/// named graph folded into the default graph. This is the load path the native SPARQL
/// conformance gate (`crates/sparql-conformance`) replays against the frozen goldens,
/// which were captured over the same flatten-to-default-graph view. Implemented
/// entirely on the oxigraph-free `gts` reader path: `read_all_segments` (re-exported
/// from the `purrdf-core` kernel) → `gts_to_ser` → the native statement-layer
/// fold (`flattened_dataset_from_ser_graph`), which re-homes each base quad's graph
/// component to the default graph (`None`) before `freeze()`.
pub fn flattened_dataset_from_gts_graph(
    g: &purrdf_gts::model::Graph,
) -> Result<std::sync::Arc<crate::RdfDataset>, RdfDiagnostic> {
    let ser = gts_to_ser(g)?;
    crate::native_codecs::parse::flattened_dataset_from_ser_graph(&ser)
}

/// Load a GTS bundle's bytes into a flattened frozen [`RdfDataset`](crate::RdfDataset).
/// See [`flattened_dataset_from_gts_graph`].
pub fn flattened_dataset_from_bytes(
    bytes: &[u8],
) -> Result<std::sync::Arc<crate::RdfDataset>, RdfDiagnostic> {
    let graph = read_all_segments(bytes)?;
    flattened_dataset_from_gts_graph(&graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::value::Value;
    use purrdf_gts::model::{Graph, Term, TermKind};
    use purrdf_gts::writer::Writer;

    fn private_lang_named_graph() -> Graph {
        let mut graph = Graph::default();
        graph.terms.push(Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/s".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        });
        graph.terms.push(Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/p".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        });
        graph.terms.push(Term {
            kind: TermKind::Literal,
            value: Some("hallo".to_owned()),
            datatype: None,
            lang: Some("x-purrdf-afrikaans".to_owned()),
            direction: None,
            reifier: None,
            triple: None,
        });
        graph.terms.push(Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/graph".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        });
        graph
            .meta
            .push(("producer".to_owned(), Value::Text("purrdf-test".to_owned())));
        graph.segment_profiles.push("rdf12".to_owned());
        graph.quads.push((0, 1, 2, Some(3)));
        graph
    }

    /// The oxigraph-free [`flattened_dataset_from_bytes`] folds the one named-graph
    /// quad into the DEFAULT graph (graph component `None`). This is the load contract
    /// the  Task-4 native conformance gate relies on, and it accepts a
    /// private (`x-purrdf-…`) language tag.
    #[test]
    fn flattened_dataset_from_bytes_folds_named_graph_into_default() {
        let graph = private_lang_named_graph();
        let writer =
            Writer::deterministic(&graph, "purrdf-test").expect("deterministic GTS writer");
        let bytes = writer.to_bytes();

        let dataset = flattened_dataset_from_bytes(&bytes).expect("native flattened dataset");
        let quads: Vec<_> = dataset.quads().collect();
        assert_eq!(quads.len(), 1, "the single source quad survives the fold");
        assert!(
            quads[0].g.is_none(),
            "the named graph was re-homed to the default graph (None)"
        );
    }
}
