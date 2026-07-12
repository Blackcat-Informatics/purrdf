// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Dense ownership-analysis profiling harness.
//!
//! Wall-clock samples from the shared development host are not acceptance
//! evidence. Correctness tests guard the one-pass traversal and cache behavior;
//! this target exists for controlled-host allocation and profile collection.

use std::fmt::Write as _;
use std::path::Path;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_slice::{OwnershipAnalyzer, SliceCatalog, SliceVocab};
use tempfile::TempDir;

const NS: &str = "https://example.org/vocab/";
const SLICE_COUNT: usize = 24;
const TERMS_PER_SLICE: usize = 24;

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture dir");
    std::fs::write(path, content).expect("write fixture");
}

fn fixture() -> (TempDir, SliceCatalog) {
    let temp = TempDir::new().expect("tempdir");
    for slice_index in 0..SLICE_COUNT {
        let dir = temp.path().join(format!("slice-{slice_index}"));
        let slice_iri = format!("{NS}slice-{slice_index}");
        let mut manifest = format!(
            "@prefix ex: <{NS}> .\n\
             <{slice_iri}> a ex:Slice"
        );
        if slice_index > 0 {
            let _ = write!(
                manifest,
                " ; ex:sliceDependsOn <{NS}slice-{}>",
                slice_index - 1
            );
        }
        manifest.push_str(" .\n");
        write(&dir.join("manifest.ttl"), &manifest);

        let mut module = String::from(
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
        );
        for term_index in 0..TERMS_PER_SLICE {
            let term_iri = format!("{NS}term-{slice_index}-{term_index}");
            let _ = writeln!(
                module,
                "<{term_iri}> a owl:Class ; rdfs:isDefinedBy <{slice_iri}> ."
            );
            if slice_index > 0 {
                let _ = writeln!(
                    module,
                    "<{term_iri}> rdfs:subClassOf <{NS}term-{}-{term_index}> .",
                    slice_index - 1
                );
            }
        }
        write(&dir.join("module.ttl"), &module);
    }

    let vocab = SliceVocab::for_namespace(NS);
    let catalog = SliceCatalog::discover(temp.path(), vocab).expect("discover fixture");
    (temp, catalog)
}

fn benchmark(c: &mut Criterion) {
    let (_temp, catalog) = fixture();
    let mut group = c.benchmark_group("slice_ownership");
    group.throughput(Throughput::Elements(
        u64::try_from(SLICE_COUNT * TERMS_PER_SLICE).expect("fixture size fits u64"),
    ));
    group.bench_function("analyze_dense_576_terms", |b| {
        b.iter(|| {
            OwnershipAnalyzer::new(black_box(&catalog))
                .analyze()
                .expect("analyze fixture")
        });
    });
    group.finish();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
