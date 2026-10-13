// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Identical public-API workloads compiled against each repository snapshot.
//! Input construction and output verification are outside the timed interval.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SerializeGraph, SparqlEngine,
    SparqlRequest, SparqlResult,
};
use purrdf_gts::{model::Graph, reader::read, writer::Writer};
use purrdf_sparql_eval::NativeSparqlEngine;

const ROWS: usize = 10_000;
const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const QUERY: &str = "CONSTRUCT { ?s <http://example.org/selected> ?label } WHERE { \
    ?s <http://example.org/value> ?value . ?s <http://example.org/label> ?label . \
    FILTER(?value > 48) }";

pub enum Product {
    Dataset(Arc<RdfDataset>),
    Bytes(Vec<u8>),
    Graph(Graph),
}

fn digest(bytes: &[u8]) -> String {
    purrdf_gts::wire::blake3_256(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl Product {
    pub fn signature(&self) -> String {
        match self {
            Self::Dataset(dataset) => {
                let bytes = purrdf_rdf::serialize_dataset(
                    dataset.as_ref(),
                    "application/n-quads",
                    SerializeGraph::Dataset,
                )
                .expect("serialize benchmark output");
                format!("dataset:{}:{}", dataset.quad_count(), digest(&bytes))
            }
            Self::Bytes(bytes) => format!("bytes:{}:{}", bytes.len(), digest(bytes)),
            Self::Graph(graph) => {
                assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
                let bytes = Writer::deterministic(graph, "generic")
                    .expect("re-author benchmark graph")
                    .into_bytes();
                format!(
                    "graph:{}:{}:{}",
                    graph.quads.len(),
                    graph.blobs.len(),
                    digest(&bytes)
                )
            }
        }
    }
}

struct Records {
    iris: Vec<String>,
    blanks: Vec<String>,
    labels: Vec<String>,
    numbers: Vec<String>,
}

impl Records {
    fn new() -> Self {
        Self {
            iris: (0..ROWS)
                .map(|i| format!("http://example.org/r/{i}"))
                .collect(),
            blanks: (0..ROWS).map(|i| format!("b{i}")).collect(),
            labels: (0..ROWS).map(|i| format!("record {i}")).collect(),
            numbers: (0..ROWS).map(|i| (i % 97).to_string()).collect(),
        }
    }

    fn build(&self, mixed: bool) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let value = builder.intern_iri("http://example.org/value");
        let label = builder.intern_iri("http://example.org/label");
        let quoted = builder.intern_iri("http://example.org/quoted");
        for (i, iri) in self.iris.iter().enumerate() {
            let subject = builder.intern_iri(iri);
            let object = if mixed {
                builder.intern_literal(RdfLiteral::typed(self.numbers[i].clone(), INTEGER))
            } else {
                builder.intern_iri(&self.iris[(i + 1) % self.iris.len()])
            };
            builder.push_quad(subject, value, object, None);
            if mixed {
                let text = builder
                    .intern_literal(RdfLiteral::language_tagged(self.labels[i].clone(), "en"));
                builder.push_quad(subject, label, text, None);
                let blank = builder.intern_blank(&self.blanks[i], BlankScope((i % 4) as u32));
                let triple = builder.intern_triple(subject, value, object);
                builder.push_quad(blank, quoted, triple, None);
            }
            // A real dedup hit follows each insert, including typed literals,
            // so the benchmark covers lookup as well as table growth.
            assert_eq!(builder.intern_iri(iri), subject);
        }
        builder.freeze().expect("freeze benchmark dataset")
    }
}

fn payloads(length: usize, count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|seed| {
            let mut state = seed as u64 + 1;
            (0..length)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    state as u8
                })
                .collect()
        })
        .collect()
}

fn author(blobs: &[Vec<u8>]) -> Vec<u8> {
    let mut writer = Writer::new("generic");
    for blob in blobs {
        writer.add_blob(blob, Some("application/octet-stream"), Some("payload"));
    }
    writer.add_index_with_mmr();
    writer.into_bytes()
}

pub fn workload(case: &str) -> Box<dyn Fn() -> Product> {
    match case {
        "intern-iri" | "intern-mixed" => {
            let records = Records::new();
            let mixed = case == "intern-mixed";
            Box::new(move || Product::Dataset(records.build(mixed)))
        }
        "parse-nquads" | "parse-turtle" => {
            use std::fmt::Write as _;
            let mut text = String::new();
            for i in 0..ROWS {
                writeln!(
                    text,
                    "<http://example.org/r/{i}> <http://example.org/value> \
                    \"{}\"^^<{INTEGER}> .",
                    i % 97
                )
                .expect("write fixture");
                writeln!(
                    text,
                    "<http://example.org/r/{i}> <http://example.org/label> \
                    \"record {i}\"@en ."
                )
                .expect("write fixture");
            }
            let media = if case == "parse-nquads" {
                "application/n-quads"
            } else {
                "text/turtle"
            };
            Box::new(move || {
                Product::Dataset(
                    purrdf_rdf::parse_dataset(text.as_bytes(), media, None)
                        .expect("parse benchmark RDF"),
                )
            })
        }
        "query-join" => {
            let dataset = Records::new().build(true);
            let engine = NativeSparqlEngine::new();
            Box::new(move || {
                match engine
                    .query(
                        &dataset,
                        SparqlRequest {
                            query: QUERY,
                            base_iri: None,
                            substitutions: &[],
                        },
                    )
                    .expect("evaluate benchmark query")
                {
                    SparqlResult::Graph(graph) => Product::Dataset(graph),
                    _ => panic!("CONSTRUCT must return a graph"),
                }
            })
        }
        "gts-author-4k" | "gts-read-4k" | "gts-author-1m" | "gts-read-1m" => {
            let blobs = if case.ends_with("4k") {
                payloads(4096, 256)
            } else {
                payloads(1 << 20, 8)
            };
            if case.starts_with("gts-read") {
                let bytes = author(&blobs);
                Box::new(move || Product::Graph(read(&bytes, false, None)))
            } else {
                Box::new(move || Product::Bytes(author(&blobs)))
            }
        }
        _ => panic!("unknown workload: {case}"),
    }
}

/// One source file supplies both executables; only the allocation executable
/// installs the counting allocator. Timing therefore uses the system allocator.
pub fn entry(allocations: bool) {
    use std::{hint::black_box, time::Instant};
    let args: Vec<String> = std::env::args().collect();
    if allocations {
        assert_eq!(args.len(), 2, "CASE");
        let run = workload(&args[1]);
        for _ in 0..3 {
            black_box(run());
        }
        let window = purrdf_alloc_probe::WholeProcessWindow::open();
        let product = black_box(run());
        let stats = window.close();
        println!(
            "{},{},{},{},{}",
            stats.allocations,
            stats.requested_bytes,
            stats.peak_working_bytes,
            stats.retained_bytes,
            product.signature()
        );
        return;
    }
    assert_eq!(args.len(), 3, "CASE ITERATIONS (zero verifies output only)");
    let iterations: u64 = args[2].parse().expect("iteration count");
    let run = workload(&args[1]);
    if iterations == 0 {
        println!("{}", run().signature());
        return;
    }
    for _ in 0..3 {
        black_box(run());
    }
    #[cfg(target_os = "linux")]
    let mut counters =
        purrdf_testkit::bench::counter::CounterControl::from_env().expect("native counter control");
    #[cfg(target_os = "linux")]
    if let Some(control) = &mut counters {
        control
            .command(purrdf_testkit::bench::counter::CounterCommand::Enable)
            .expect("enable native counter");
    }
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(run());
    }
    let elapsed = start.elapsed().as_nanos();
    #[cfg(target_os = "linux")]
    if let Some(control) = &mut counters {
        control
            .command(purrdf_testkit::bench::counter::CounterCommand::Disable)
            .expect("disable native counter");
    }
    println!("{elapsed}");
}
