// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compare joined groups against the same rows in a flat triple block.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_lex::allocation::{Admission, Memory, Resident, SharedText, StorageError};
use purrdf_sparql_algebra::error::ParseError;
use purrdf_sparql_algebra::parser::ParserAdmission;
use purrdf_sparql_algebra::{ParserOptions, Query, SparqlParser};
use purrdf_testkit::bench::{Bench, BenchmarkId, bench_group, bench_main, black_box};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

struct AssemblyAdmission {
    ceiling: usize,
    peak: usize,
    work: u64,
}

impl Admission for AssemblyAdmission {
    fn progress_enabled(&self) -> bool {
        true
    }

    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        self.work += units;
        Ok(())
    }

    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        if live > self.ceiling {
            return Err(StorageError::AdmissionFailed);
        }
        self.peak = self.peak.max(live);
        Ok(())
    }
}

impl ParserAdmission for AssemblyAdmission {
    fn text(&mut self, text: &dyn core::fmt::Display) -> Result<SharedText, StorageError> {
        // Shared lexical owners have their own account. The physical allocator
        // window counts those alongside the parser's admitted containers.
        let mut resident = Resident;
        let text = Memory::new(&mut resident).format(text)?;
        SharedText::try_from_admitted(text, ()).map_err(|_| StorageError::AllocationFailed)
    }
}

fn query_text(depth: usize, nested: bool) -> String {
    const TRIPLE: &str = "?s <https://example.org/p> ?o";
    const LEAF: &str = "?s <https://example.org/q> ?z";

    let mut query = String::from("SELECT ?s WHERE { ");
    for _ in 0..depth {
        query.push_str(TRIPLE);
        query.push_str(if nested { " { " } else { " . " });
    }
    query.push_str(LEAF);
    for _ in 0..depth * usize::from(nested) {
        query.push_str(" }");
    }
    query.push_str(" }");
    query
}

fn profile(text: &str, ceiling: usize) -> (Result<Query, ParseError>, AssemblyAdmission, usize) {
    let mut admission = AssemblyAdmission {
        ceiling,
        peak: 0,
        work: 0,
    };
    let mut memory = Memory::new(&mut admission as &mut dyn ParserAdmission);
    let query =
        SparqlParser::parse_query_admitted(text, &ParserOptions::default(), None, &[], &mut memory);
    let retained = memory.admitted_bytes();
    (query, admission, retained)
}

fn report(kind: &str, depth: usize, text: &str) {
    let window = CurrentThreadWindow::open();
    let (query, AssemblyAdmission { work, peak, .. }, retained) = profile(text, usize::MAX);
    let query = query.expect("the admitted query parses");
    let allocation = window.close();
    println!(
        "{kind} depth={depth} work={work} peak_admitted_bytes={peak} retained_admitted_bytes={retained} {allocation}",
    );

    if kind == "nested" && depth == 2_048 {
        assert!(matches!(
            profile(text, peak - 1).0,
            Err(ParseError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(
            profile(text, peak)
                .0
                .expect("adjacent capacity admits the same query"),
            query,
        );
        println!(
            "nested capacity refusal at {} bytes and acceptance at {peak} bytes",
            peak - 1,
        );
    }
}

fn bench_bgp_assembly(bench: &mut Bench) {
    let parser = SparqlParser::new();
    let mut group = bench.benchmark_group("bgp_assembly");
    for (kind, nested) in [("nested", true), ("flat", false)] {
        for depth in [2_048, 4_096, 8_192] {
            let text = query_text(depth, nested);
            let mut reported = false;
            group.bench_function(BenchmarkId::new(kind, depth), |case| {
                if !reported {
                    report(kind, depth, &text);
                    reported = true;
                }
                case.iter(|| parser.parse_query(black_box(&text)).expect("query parses"));
            });
        }
    }
    group.finish();
}

bench_group!(benches, bench_bgp_assembly);
bench_main!(benches);
