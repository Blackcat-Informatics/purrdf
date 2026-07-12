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

use purrdf_core::{
    RdfDataset, write_dataset_annotation, write_dataset_quad, write_dataset_reifier,
};

/// Serialize a CONSTRUCT-result dataset to N-Triples (plus RDF-1.2-star
/// annotations and reifiers). Each kernel `emit_*` call already terminates its
/// output with `\n`, so the parts are concatenated in order: quads, then
/// annotations, then reifiers.
// Consumed by the JSON CONSTRUCT-graph branch (`crate::json`) and, in Task 3, by
// the remaining result-document writers.
pub(crate) fn dataset_to_ntriples(dataset: &RdfDataset) -> String {
    let statement_count =
        dataset.quad_count() + dataset.annotations().count() + dataset.reifiers().count();
    let mut out = String::with_capacity(statement_count.saturating_mul(96));
    for quad in dataset.quads() {
        write_dataset_quad(dataset, quad, &mut out);
    }
    for (reifier, predicate, object) in dataset.annotations() {
        write_dataset_annotation(dataset, reifier, predicate, object, &mut out);
    }
    for (reifier, statement) in dataset.reifiers() {
        write_dataset_reifier(dataset, reifier, statement, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use purrdf_core::{
        BlankScope, RdfDatasetBuilder, RdfLiteral, RdfQuad, RdfTerm, emit_annotation, emit_quad,
        emit_reifier,
    };

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

    #[test]
    fn triple_term_object_serializes_as_non_asserting_delimiter() {
        // A CONSTRUCT graph whose object is a triple TERM (not an `rdf:reifies`
        // statement) must round-trip as `<<( s p o )>>`. The bare `<< s p o >>`
        // form is a *reifying, asserting* triple in the native parser — spelling
        // a plain triple-term object that way would silently grow the re-parsed
        // graph by one quad instead of preserving a single non-asserting term.
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri("http://example.org/s");
        let p = builder.intern_iri("http://example.org/p");
        let o = builder.intern_iri("http://example.org/o");
        let statement = builder.intern_triple(s, p, o);
        let outer_subject = builder.intern_iri("http://example.org/outer");
        let outer_predicate = builder.intern_iri("http://example.org/concludes");
        builder.push_quad(outer_subject, outer_predicate, statement, None);
        let dataset = builder.freeze().expect("dataset freezes");

        let nt = dataset_to_ntriples(&dataset);
        assert_eq!(
            nt,
            "<http://example.org/outer> <http://example.org/concludes> \
<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>> .\n"
        );
        assert!(
            !nt.contains("> << <"),
            "triple-term object must never use the bare reifying-triple delimiter: {nt}"
        );
    }

    #[test]
    fn borrowed_writer_is_byte_identical_to_owned_emitter() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank("subject", BlankScope(7));
        let predicate = builder.intern_iri("http://example.org/predicate");
        let object = builder.intern_literal(RdfLiteral {
            lexical_form: "quoted \"text\"".to_owned(),
            datatype: None,
            language: Some("en".to_owned()),
            direction: Some(purrdf_core::RdfTextDirection::Ltr),
        });
        let statement = builder.intern_triple(subject, predicate, object);
        let reifier = builder.intern_iri("http://example.org/reifier");
        builder.push_quad(subject, predicate, statement, None);
        builder.push_reifier(reifier, statement);
        builder.push_annotation(reifier, predicate, object);
        let dataset = builder.freeze().expect("dataset freezes");

        let mut expected = String::new();
        for quad in dataset.owned_quads() {
            expected.push_str(&emit_quad(&quad));
        }
        for annotation in dataset.owned_annotations() {
            expected.push_str(&emit_annotation(&annotation));
        }
        for reifier in dataset.owned_reifiers() {
            expected.push_str(&emit_reifier(&reifier, &[]));
        }

        assert_eq!(dataset_to_ntriples(&dataset), expected);
    }
}
