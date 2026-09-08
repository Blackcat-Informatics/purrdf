// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Prepared constants and substitutions must address the same RDF 1.2 literal.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, RdfTextDirection, SparqlResult};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

#[test]
fn directional_constants_datatype_and_substitutions_agree_with_the_store() {
    let engine = NativeSparqlEngine::new();
    let bound = engine
        .prepare_query(
            "ASK { <http://example.org/s> <http://example.org/p> ?value }",
            None,
        )
        .unwrap();
    for (direction, suffix, datatype) in [
        (None, "", "langString"),
        (Some(RdfTextDirection::Ltr), "--ltr", "dirLangString"),
        (Some(RdfTextDirection::Rtl), "--rtl", "dirLangString"),
    ] {
        for explicit in [None, Some("http://example.org/overridden".to_owned())] {
            let mut builder = RdfDatasetBuilder::new();
            let s = builder.intern_iri("http://example.org/s");
            let p = builder.intern_iri("http://example.org/p");
            let o = builder.intern_literal(RdfLiteral {
                lexical_form: "claim".into(),
                language: Some("AR".into()),
                direction,
                datatype: explicit,
            });
            builder.push_quad(s, p, o, None);
            let dataset = builder.freeze().unwrap();
            let query = format!(
                "ASK {{ <http://example.org/s> <http://example.org/p> \"claim\"@ar{suffix} . \
                 FILTER (DATATYPE(\"claim\"@ar{suffix}) = \
                 <http://www.w3.org/1999/02/22-rdf-syntax-ns#{datatype}>) }}"
            );
            let prepared = engine.prepare_query(&query, None).unwrap();
            let result = engine
                .query_prepared(&dataset, &prepared, &[], QueryOptions::EMPTY)
                .unwrap();
            assert!(
                matches!(result, SparqlResult::Boolean(true)),
                "{query}: {result:?}"
            );
            let substitutions = [("value".into(), dataset.term_value(o))];
            let result = engine
                .query_prepared(&dataset, &bound, &substitutions, QueryOptions::EMPTY)
                .unwrap();
            assert!(matches!(result, SparqlResult::Boolean(true)), "{result:?}");
        }
    }
}
