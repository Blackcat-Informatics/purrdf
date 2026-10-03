// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Portable scope cases execute their independently enumerated result files.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use purrdf_core::datatype::XSD_INTEGER;
use purrdf_lex::json::{self, Value};
use purrdf_rdf::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfTerm, SparqlEngine, SparqlRequest, SparqlResult,
    TermValue, parse_dataset,
};
use purrdf_sparql_algebra::{SparqlParser, try_pattern_to_select_query};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
use purrdf_sparql_results::{ParsedSolutions, from_json};

fn text_member<'a>(record: &'a Value, name: &str) -> &'a str {
    record[name]
        .as_str()
        .unwrap_or_else(|| panic!("inventory member {name} must be a string"))
}

fn case_dataset(directory: &Path, inputs: &Value) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (index, input) in inputs
        .as_array()
        .expect("data input array")
        .iter()
        .enumerate()
    {
        let bytes = std::fs::read(directory.join(text_member(input, "path")))
            .expect("read the named RDF input");
        let dataset = parse_dataset(&bytes, "text/turtle", None).expect("RDF input parses");
        let graph = input["graph"].as_str();
        for mut quad in dataset.owned_quads() {
            quad.graph_name = graph.map(|iri| RdfTerm::Iri(iri.to_owned()));
            builder.push_owned_quad_scoped(
                &quad,
                BlankScope(u32::try_from(index).expect("fixture source index fits")),
            );
        }
    }
    builder.freeze().expect("fixture dataset freezes")
}

fn assert_solution_bag(result: SparqlResult, expected: &ParsedSolutions, context: &str) {
    let (variables, mut rows) = result.into_solutions().expect("SELECT solution shape");
    assert_eq!(variables, expected.variables, "visible columns: {context}");
    let mut expected_rows = expected.rows.clone();
    let key = |row: &Vec<Option<TermValue>>| {
        row.iter()
            .map(|cell| cell.as_ref().map(TermValue::to_canonical_bytes))
            .collect::<Vec<_>>()
    };
    rows.sort_by_cached_key(key);
    expected_rows.sort_by_cached_key(key);
    assert_eq!(rows, expected_rows, "exact typed multiset: {context}");
}

#[test]
fn portable_inventory_preserves_scope_bags_columns_and_carrier_identity() {
    let directory = purrdf_testkit::paths::workspace_root().join("corpora/community/sparql/scope");
    let inventory = json::read_slice(
        &std::fs::read(directory.join("inventory.json")).expect("inventory file"),
        json::Limits::DEFAULT,
    )
    .expect("inventory JSON");
    let cases = inventory["cases"].as_array().expect("case inventory");
    assert_eq!(cases.len(), 21, "the entire accepted scope inventory runs");
    let mut normative_queries = BTreeSet::new();
    let mut evaluated = 0;
    let mut rejected = 0;
    let engine = NativeSparqlEngine::new();
    for case in cases {
        let id = text_member(case, "id");
        assert!(
            !text_member(case, "rationale").is_empty(),
            "rationale: {id}"
        );
        assert!(
            !case["specifications"]
                .as_array()
                .expect("specification links")
                .is_empty(),
            "normative basis: {id}"
        );
        let filename = text_member(case, "query");
        if text_member(case, "profile") == "sparql11-query-over-rdf12" {
            normative_queries.insert(format!("http://example.org/scope/{filename}"));
        }
        let text = std::fs::read_to_string(directory.join(filename)).expect("query file");
        if text_member(case, "kind") == "negative-syntax" {
            assert!(
                SparqlParser::new().parse_query(&text).is_err(),
                "parser: {id}"
            );
            assert!(
                engine.prepare_query(&text, None).is_err(),
                "preparation: {id}"
            );
            rejected += 1;
            continue;
        }
        let expected = from_json(
            &std::fs::read(directory.join(text_member(case, "expected")))
                .expect("independently enumerated result file"),
        )
        .expect("SPARQL Results JSON oracle parses");
        let dataset = case_dataset(&directory, &case["data"]);
        let parsed = SparqlParser::new()
            .parse_query(&text)
            .expect("source query parses");
        parsed.validate().expect("raw structural admission");
        parsed
            .validate_hidden_variables()
            .expect("source witness observer admission");
        let prepared = engine.prepare_query(&text, None).expect("text preparation");
        let raw = engine
            .prepare_algebra(parsed.clone(), QueryOptions::EMPTY)
            .expect("compiler-algebra preparation");
        let carrier = try_pattern_to_select_query(parsed.pattern()).expect("legal carrier");
        let reparsed = SparqlParser::new()
            .parse_query(&carrier)
            .expect("carrier reparses");
        let normalized = try_pattern_to_select_query(reparsed.pattern()).expect("second rendering");
        let repeated = SparqlParser::new()
            .parse_query(&normalized)
            .expect("normalized carrier");
        assert_eq!(
            try_pattern_to_select_query(repeated.pattern()).expect("third rendering"),
            normalized,
            "carrier normalization is idempotent: {id}"
        );
        let request = SparqlRequest {
            query: &text,
            base_iri: None,
            substitutions: &[],
        };
        assert_solution_bag(
            engine.query(&dataset, request).expect("native query"),
            &expected,
            id,
        );
        // Reuse each retained plan: preparation must not retain a previous witness row.
        for repetition in 0..3 {
            for plan in [&prepared, &raw] {
                assert_solution_bag(
                    engine
                        .query_prepared(&dataset, plan, &[], QueryOptions::EMPTY)
                        .expect("retained query executes"),
                    &expected,
                    &format!("{id}, retained execution {repetition}"),
                );
            }
        }
        for rendering in [&carrier, &normalized] {
            let carried = engine
                .query(
                    &dataset,
                    SparqlRequest {
                        query: rendering,
                        base_iri: None,
                        substitutions: &[],
                    },
                )
                .expect("carrier execution");
            if expected.variables.is_empty() {
                // Legal SELECT syntax cannot project zero variables explicitly.
                // The documented carrier uses one fresh unit column; the SERVICE
                // adapter restores the source schema after this transport step.
                let (variables, rows) = carried.into_solutions().expect("unit solutions");
                assert_eq!(variables.len(), 1, "one transport unit column: {id}");
                assert!(
                    variables[0].starts_with("__purrdf_hidden_") && variables[0].ends_with("unit")
                );
                assert!(
                    rows.iter()
                        .all(|row| row == &[Some(TermValue::typed_literal("1", XSD_INTEGER))])
                );
                let restored: Vec<Vec<Option<TermValue>>> = rows.iter().map(|_| vec![]).collect();
                assert_eq!(
                    restored, expected.rows,
                    "explicit source-schema restoration: {id}"
                );
            } else {
                assert_solution_bag(carried, &expected, &format!("{id}, carrier"));
            }
        }
        evaluated += 1;
    }
    assert_eq!((evaluated, rejected), (18, 3));

    // The portable W3C-vocabulary manifest names exactly the SPARQL 1.1 queries
    // in the executed inventory; the RDF 1.2 syntax case has its separate profile.
    let manifest = parse_dataset(
        &std::fs::read(directory.join("manifest.ttl")).expect("W3C-compatible manifest"),
        "text/turtle",
        Some("http://example.org/scope/manifest.ttl"),
    )
    .expect("manifest RDF parses");
    let manifest_queries: BTreeSet<_> = manifest
        .quads()
        .filter_map(|quad| match manifest.term_value(quad.o) {
            TermValue::Iri(iri)
                if Path::new(&iri)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("rq")) =>
            {
                Some(iri)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        manifest_queries, normative_queries,
        "manifest inventory parity"
    );
}
