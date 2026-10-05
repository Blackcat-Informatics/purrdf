// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The public engine retains the parser's exact failure rather than a coarse code.

use purrdf_core::RdfDiagnostic;
use purrdf_lex::json::Value;
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

#[test]
fn query_admission_retains_each_original_parser_condition() {
    let arity = format!(
        "SELECT (<{}>(\"key\") AS ?map) WHERE {{}}",
        purrdf_cdt::CdtFn::MapConstructor.iri()
    );
    let cases = [
        (
            "SELECT * WHERE { _:b ?p ?o OPTIONAL { _:b ?q ?v } }",
            "sparql-parse-syntax",
        ),
        (
            "SELECT * WHERE { ?s ?p \"unterminated }",
            "sparql-parse-lex",
        ),
        ("ASK {} ORDER BY ?x", "sparql-parse-unsupported"),
        ("SELECT * WHERE { <relative> ?p ?o }", "sparql-parse-iri"),
        (arity.as_str(), "sparql-parse-cdt-arity"),
    ];
    let engine = NativeSparqlEngine::new();
    for (query, identity) in cases {
        let source = SparqlParser::new()
            .parse_query(query)
            .expect_err("source refusal");
        assert_eq!(source.presentation().message_id(), identity);
        let diagnostic = engine
            .prepare_execution(query, None, &[], QueryOptions::EMPTY)
            .expect_err("engine refusal");
        assert_eq!(diagnostic.code, "native-sparql-query-parse");
        assert_eq!(diagnostic.message, source.to_string());
        assert_eq!(
            diagnostic.presentation().expect("original typed source"),
            &source.presentation()
        );
        let mut record = diagnostic.to_json();
        assert_eq!(
            record.get("presentation"),
            Some(&source.presentation().to_json())
        );
        // The presentation is the record's only addition: without it, the record is
        // exactly the message-only diagnostic's.
        let Value::Object(members) = &mut record else {
            panic!("a diagnostic record is an object");
        };
        members.remove("presentation");
        assert_eq!(
            record,
            RdfDiagnostic::error("native-sparql-query-parse", source.to_string()).to_json()
        );
    }
    assert!(
        engine
            .prepare_execution("ASK {}", None, &[], QueryOptions::EMPTY)
            .is_ok()
    );
}
