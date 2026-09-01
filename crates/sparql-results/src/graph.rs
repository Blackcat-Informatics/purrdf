// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! CONSTRUCT-graph → N-Quads serialization over the `purrdf-core` primitives
//! (wasm-clean; does **not** pull `crates/rdf`/oxigraph).
//!
//! A `SparqlResult::Graph` carries an [`RdfDataset`]. The standard N-Triples
//! lossy path would emit only the quads, in one graph; in keeping with the
//! project's maximal-information-flow goal and RDF-1.2-star, this writer emits
//! the dataset's annotations and reifiers too, AND spells out every row's graph
//! slot, so no carried structure is silently dropped. The kernel
//! `write_dataset_*_nquad` primitives are the single source of term/line syntax.
//!
//! # Why N-Quads and not N-Triples
//!
//! `CONSTRUCT { GRAPH ?g { … } }` — a first-party extension, NOT defined by SPARQL
//! 1.2 — is a quad template: the graph name
//! is in the query the caller wrote, one token at a time, so it is among the most
//! explicit things in the request. Rendering that result through a triple-only
//! writer produced a well-formed document that silently omitted exactly what was
//! asked for — the failure mode `purrdf_core::named_graph` exists to refuse.
//!
//! Widening the writer rather than refusing is right HERE (and only here) because
//! this egress names no RDF syntax for the caller to have chosen: it is the
//! `{"graph": "…"}` member of PurRDF's own JSON envelope, not a W3C SPARQL-Results
//! member and not a format a caller can select. With no request to contradict, the
//! honest answer is the one that carries everything — and it costs nothing, because
//! an N-Quads line with no graph term IS the N-Triples line, so every
//! default-graph-only result renders byte-for-byte as it did before.

use purrdf_core::{
    RdfDataset, write_dataset_annotation_nquad, write_dataset_nquad, write_dataset_reifier_nquad,
};

/// Serialize a CONSTRUCT-result dataset to N-Quads (plus RDF-1.2-star
/// annotations and reifiers, each in the graph it was asserted in). Each kernel
/// writer already terminates its line with `\n`, so the parts are concatenated
/// in order: quads, then annotations, then reifiers.
///
/// Total: every emitted line re-lexes as N-Quads because the kernel writers
/// escape a scope-qualified blank-node label outside the Turtle
/// `BLANK_NODE_LABEL` alphabet into it (deterministically and injectively), so
/// no dataset can fail to serialize on label syntax.
pub(crate) fn dataset_to_nquads(dataset: &RdfDataset) -> String {
    let statement_count =
        dataset.quad_count() + dataset.annotations().count() + dataset.reifiers().count();
    let mut out = String::with_capacity(statement_count.saturating_mul(96));
    for quad in dataset.quads() {
        write_dataset_nquad(dataset, quad, &mut out);
    }
    for (reifier, predicate, object, graph) in dataset.annotations_with_graph() {
        write_dataset_annotation_nquad(dataset, reifier, predicate, object, graph, &mut out);
    }
    for (reifier, statement, graph) in dataset.reifiers_with_graph() {
        write_dataset_reifier_nquad(dataset, reifier, statement, graph, &mut out);
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

    /// A default-graph row carries no fourth term, so the N-Quads line IS the
    /// N-Triples line — the byte-identity that makes widening this writer free.
    #[test]
    fn default_graph_quad_emits_the_bare_ntriples_line() {
        let mut builder = RdfDatasetBuilder::new();
        builder.push_owned_quad(&RdfQuad {
            subject: RdfTerm::iri("http://example.org/s"),
            predicate: "http://example.org/p".to_string(),
            object: RdfTerm::iri("http://example.org/o"),
            graph_name: None,
            location: None,
        });
        let dataset = builder.freeze().expect("dataset freezes");

        let nt = dataset_to_nquads(&dataset);
        assert_eq!(
            nt,
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n"
        );
    }

    #[test]
    fn empty_dataset_is_empty_string() {
        let dataset = RdfDatasetBuilder::new().freeze().expect("empty freezes");
        assert_eq!(dataset_to_nquads(&dataset), "");
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

        let nt = dataset_to_nquads(&dataset);
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

        assert_eq!(dataset_to_nquads(&dataset), expected);
    }

    /// The regression this writer exists to close: a quad-template `CONSTRUCT`
    /// result carries its graph names into the envelope instead of losing them.
    ///
    /// All three layers are exercised, because the RDF 1.2 statement layer is keyed
    /// PER GRAPH and each layer has its own slot: a base quad in `<g1>`, a reifier
    /// declaration in `<g2>`, and an annotation in `<g2>`. A triple-only writer
    /// emits the same three lines with every graph term missing — well-formed, and
    /// silently short of exactly what the query asked for.
    #[test]
    fn every_layer_carries_its_named_graph() {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri("http://example.org/s");
        let p = builder.intern_iri("http://example.org/p");
        let o = builder.intern_iri("http://example.org/o");
        let g1 = builder.intern_iri("http://example.org/g1");
        let g2 = builder.intern_iri("http://example.org/g2");
        let reifier = builder.intern_iri("http://example.org/r");
        let statement = builder.intern_triple(s, p, o);
        builder.push_quad(s, p, o, Some(g1));
        builder.push_reifier_in_graph(reifier, statement, Some(g2));
        builder.push_annotation_in_graph(reifier, p, o, Some(g2));
        let dataset = builder.freeze().expect("dataset freezes");

        assert_eq!(
            dataset_to_nquads(&dataset),
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> \
<http://example.org/g1> .\n\
<http://example.org/r> <http://example.org/p> <http://example.org/o> \
<http://example.org/g2> .\n\
<http://example.org/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>> \
<http://example.org/g2> .\n"
        );
    }
}
