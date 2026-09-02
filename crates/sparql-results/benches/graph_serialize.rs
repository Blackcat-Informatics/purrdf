// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! CONSTRUCT-graph N-Triples serialization benchmark.
//!
//! Wall-clock samples from the shared development host are not acceptance
//! evidence. Correctness is pinned by `graph::dataset_to_ntriples`'s own tests
//! (notably `borrowed_writer_is_byte_identical_to_owned_emitter`); this target
//! exists for controlled-host allocation and profile collection of the borrowed
//! streaming writer.
//!
//! `graph::dataset_to_ntriples` itself is `pub(crate)`, so this bench drives it
//! through the crate's public production entry point instead of reaching into
//! internals: [`to_json`] on a [`SparqlResult::Graph`] takes the exact same
//! `Graph` branch (`json::write_base`) that a CONSTRUCT/DESCRIBE query result
//! goes through en route to SPARQL Results JSON. That branch calls the kernel
//! `write_dataset_quad`/`write_dataset_annotation`/`write_dataset_reifier`
//! writers directly into a pre-sized `String` using borrowed term references —
//! no owned-term materialization per statement — which is what this bench
//! measures.
//!
//! The fixture mixes IRI and blank-node subjects, language-tagged literals with
//! quote/backslash/newline bytes (so the writer's escaping path is exercised,
//! not just the fast IRI copy), a sparse named-graph split, and a 10% subset of
//! statements additionally carrying an RDF-1.2-star reifier + annotation, so all
//! three kernel writers (`write_dataset_quad`, `write_dataset_annotation`,
//! `write_dataset_reifier`) run in every sample.
//!
//! A second group serializes a SELECT solution table with escaping-heavy
//! literals to all four W3C formats (JSON/XML/CSV/TSV) and parses the JSON
//! output back through [`from_json`], so the escapers' bulk-copy paths and the
//! SRJ reader's ASCII-run fast path are measurable too. Report-only.

use std::sync::Arc;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_sparql_results::{
    ResultProvenance, SparqlResult, from_json, to_csv, to_json, to_tsv, to_xml,
};

/// Quad count. Large enough that the writer's amortized behavior (not fixture
/// construction) dominates each sample.
const ROWS: usize = 10_000;

/// Subset of rows (the first `REIFIED`) that also get an RDF-1.2-star reifier +
/// annotation, so `write_dataset_annotation`/`write_dataset_reifier` are
/// exercised alongside the quad writer on every sample.
const REIFIED: usize = 1_000;

const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// Solution rows in the SELECT-table fixture.
const SELECT_ROWS: usize = 5_000;

/// A SELECT result whose cells lean on every escaper trigger the four formats
/// share: quotes, backslashes, tabs, newlines, `&<>` (XML), commas (CSV), and
/// multibyte text — plus a clean IRI column and a typed-integer column so the
/// bulk-copy runs are interleaved with escape arms, not one or the other.
fn build_select_result(rows: usize) -> SparqlResult {
    let variables = vec![
        "s".to_owned(),
        "label".to_owned(),
        "note".to_owned(),
        "n".to_owned(),
    ];
    let rows = (0..rows)
        .map(|i| {
            vec![
                Some(TermValue::Iri(format!("https://example.org/s{i}"))),
                Some(TermValue::Literal {
                    lexical_form: format!(
                        "label \"{i}\" with \\backslash, comma, <tag> & amp\tand\nnewline"
                    ),
                    datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                    language: Some("en".to_owned()),
                    direction: None,
                }),
                Some(TermValue::Literal {
                    lexical_form: format!(
                        "caf\u{e9} \u{4e2d}\u{6587} \u{1f431} note {i} > \"end\""
                    ),
                    datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    language: None,
                    direction: None,
                }),
                (i % 3 != 0).then(|| TermValue::Literal {
                    lexical_form: i.to_string(),
                    datatype: XSD_INTEGER.to_owned(),
                    language: None,
                    direction: None,
                }),
            ]
        })
        .collect();
    SparqlResult::Solutions {
        variables,
        rows,
        aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
    }
}

/// SELECT table -> JSON / XML / CSV / TSV, and JSON -> parsed solutions.
fn bench_results_serialize(c: &mut Criterion) {
    let result = build_select_result(SELECT_ROWS);
    let provenance = ResultProvenance::default();

    // Sanity pass: every format must actually emit the table, and the JSON
    // must round-trip through the reader with the row count intact.
    let json = to_json(&result, &provenance, None).expect("json sanity pass");
    let parsed = from_json(&json.bytes).expect("json round-trip parses");
    assert_eq!(parsed.rows.len(), SELECT_ROWS, "SRJ round-trip lost rows");
    for (name, bytes) in [
        (
            "xml",
            to_xml(&result, &provenance, None).expect("xml").bytes,
        ),
        ("csv", to_csv(&result, &provenance).expect("csv").bytes),
        ("tsv", to_tsv(&result, &provenance).expect("tsv").bytes),
    ] {
        assert!(
            bytes.len() > SELECT_ROWS * 32,
            "{name} output is implausibly small ({} bytes for {SELECT_ROWS} rows)",
            bytes.len()
        );
    }

    let mut group = c.benchmark_group("sparql_results_select_serialize");
    group.throughput(Throughput::Elements(SELECT_ROWS as u64));
    group.bench_function("to_json_5k_rows_escaping", |bencher| {
        bencher.iter(|| {
            let outcome =
                to_json(black_box(&result), black_box(&provenance), None).expect("serialize");
            black_box(outcome);
        });
    });
    group.bench_function("to_xml_5k_rows_escaping", |bencher| {
        bencher.iter(|| {
            let outcome =
                to_xml(black_box(&result), black_box(&provenance), None).expect("serialize");
            black_box(outcome);
        });
    });
    group.bench_function("to_csv_5k_rows_escaping", |bencher| {
        bencher.iter(|| {
            let outcome = to_csv(black_box(&result), black_box(&provenance)).expect("serialize");
            black_box(outcome);
        });
    });
    group.bench_function("to_tsv_5k_rows_escaping", |bencher| {
        bencher.iter(|| {
            let outcome = to_tsv(black_box(&result), black_box(&provenance)).expect("serialize");
            black_box(outcome);
        });
    });
    group.bench_function("from_json_5k_rows_escaping", |bencher| {
        bencher.iter(|| {
            let parsed = from_json(black_box(&json.bytes)).expect("parse");
            black_box(parsed);
        });
    });
    group.finish();
}

fn build_dataset(rows: usize, reified: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let named_graph = b.intern_iri("https://example.org/g");
    let certainty = b.intern_iri("https://example.org/certainty");

    for i in 0..rows {
        // Every fourth subject is a blank node instead of an IRI, so the writer
        // takes both the `_:` and `<...>` subject-emission paths.
        let s = if i % 4 == 0 {
            b.intern_blank(&format!("s{i}"), BlankScope(0))
        } else {
            b.intern_iri(&format!("https://example.org/s{i}"))
        };
        let o = b.intern_literal(RdfLiteral {
            lexical_form: format!("value \"{i}\" with \\backslash and\nnewline"),
            datatype: None,
            language: Some("en".to_owned()),
            direction: None,
        });
        // A sparse named-graph split (~20%), so the writer's optional GRAPH
        // term is also on the hot path, not just the default-graph quad.
        let graph = if i % 5 == 0 { Some(named_graph) } else { None };
        b.push_quad(s, p, o, graph);

        if i < reified {
            let statement = b.intern_triple(s, p, o);
            let reifier = b.intern_iri(&format!("https://example.org/r{i}"));
            let confidence = b.intern_literal(RdfLiteral::typed("0.9", XSD_DECIMAL));
            b.push_reifier(reifier, statement);
            b.push_annotation(reifier, certainty, confidence);
        }
    }

    b.freeze().expect("bench dataset freezes")
}

fn bench_graph_serialize(c: &mut Criterion) {
    let dataset = build_dataset(ROWS, REIFIED);
    let result = SparqlResult::Graph(dataset);
    let provenance = ResultProvenance::default();

    // Sanity pass: the CONSTRUCT branch must actually emit the graph body, so a
    // regression collapsing it to `{"graph":""}` doesn't silently benchmark a
    // no-op.
    let outcome = to_json(&result, &provenance, None).expect("serialize sanity pass");
    assert!(
        outcome.bytes.len() > ROWS * 32,
        "serialized graph is implausibly small ({} bytes for {ROWS} quads)",
        outcome.bytes.len()
    );

    let mut group = c.benchmark_group("sparql_results_graph_serialize");
    group.throughput(Throughput::Elements(ROWS as u64));
    group.bench_function("to_json_ntriples_10k_quads", |bencher| {
        bencher.iter(|| {
            let outcome =
                to_json(black_box(&result), black_box(&provenance), None).expect("serialize");
            black_box(outcome);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_graph_serialize, bench_results_serialize);
criterion_main!(benches);
