// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! CONSTRUCT-graph → N-Triples serialization over the `purrdf-core`
//! primitives (wasm-clean; does **not** pull `crates/rdf`/oxigraph).
//!
//! A `SparqlResult::Graph` carries an [`RdfDataset`]. The standard N-Triples
//! lossy path would emit only the quads; in keeping with the project's
//! maximal-information-flow goal and RDF-1.2-star, this writer also emits the
//! dataset's annotations and reifiers so no carried structure is silently
//! dropped. The kernel `emit_*` primitives are the single source of term/line
//! syntax.

use purrdf_core::{emit_annotation, emit_quad, emit_reifier, RdfDataset};

/// Serialize a CONSTRUCT-result dataset to N-Triples (plus RDF-1.2-star
/// annotations and reifiers). Each kernel `emit_*` call already terminates its
/// output with `\n`, so the parts are concatenated in order: quads, then
/// annotations, then reifiers.
// Consumed by the JSON CONSTRUCT-graph branch (`crate::json`) and, in Task 3, by
// the remaining result-document writers.
pub(crate) fn dataset_to_ntriples(dataset: &RdfDataset) -> String {
    let mut out = String::new();
    for quad in dataset.owned_quads() {
        out.push_str(&emit_quad(&quad));
    }
    for annotation in dataset.owned_annotations() {
        out.push_str(&emit_annotation(&annotation));
    }
    for reifier in dataset.owned_reifiers() {
        out.push_str(&emit_reifier(&reifier, &[]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use purrdf_core::{RdfDatasetBuilder, RdfQuad, RdfTerm};

    #[test]
    fn single_quad_dataset_emits_ntriples_line() {
        let mut builder = RdfDatasetBuilder::new();
        builder.push_owned_quad(&RdfQuad {
            subject: RdfTerm::iri("http://example.org/s"),
            predicate: "http://example.org/p".to_string(),
            object: RdfTerm::iri("http://example.org/o"),
            graph_name: None,
            location: None,
        });
        let dataset = builder.freeze().expect("dataset freezes");

        let nt = dataset_to_ntriples(&dataset);
        assert_eq!(
            nt,
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n"
        );
    }

    #[test]
    fn empty_dataset_is_empty_string() {
        let dataset = RdfDatasetBuilder::new().freeze().expect("empty freezes");
        assert_eq!(dataset_to_ntriples(&dataset), "");
    }
}
