// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! The two core paths that reach a shared home: the literal escape scan and the
//! pack dictionary's front-coding prefix.
//!
//! **Report-only, and never a gate.** Wall-clock samples from a shared host are
//! not acceptance evidence; behaviour is fixed by the test suite.
//!
//! * `core_escape_scan/*` renders a 1 MiB literal through
//!   [`purrdf_core::turtle::emit_term`], the public entry to the Turtle
//!   literal escaper, over clean ASCII text (the scanner's chunked clean run is the
//!   cost), mixed text dense in escapes, and mixed text with one escape per KiB.
//! * `core_pack_dict_prefix/*` encodes and decodes the pack dictionary over
//!   100,000 sorted IRIs that share long prefixes (`EncodedDict::encode`, the
//!   front-coder's `common_prefix_len` per adjacent record; `decode` rebuilds
//!   each record from its shared prefix), and runs the first-mismatch kernel
//!   itself, `purrdf_deflate::common_prefix_len`, at 16 B, 256 B and 4 KiB.

use std::sync::Arc;

use purrdf_core::ir::pack::dict::{EncodedDict, PackDict};
use purrdf_core::turtle::emit_term;
use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTerm};
use purrdf_deflate::common_prefix_len;
use purrdf_testkit::bench::{Bench, BenchmarkId, Throughput, bench_group, bench_main, black_box};

const MIB: usize = 1 << 20;

fn literal(text: String) -> RdfTerm {
    RdfTerm::literal(RdfLiteral {
        lexical_form: text,
        datatype: None,
        language: None,
        direction: None,
    })
}

fn bench_escape_scan(c: &mut Bench) {
    let clean = "the quick brown fox jumps over the lazy dog 0123456789 "
        .repeat(MIB / 55 + 1)
        .chars()
        .take(MIB)
        .collect::<String>();
    let mixed = "caf\u{e9} \"quoted\"\tline\n\\slash\u{1}\u{7f}\u{85} \u{4e2d}\u{6587} \u{1f431} "
        .repeat(MIB / 40 + 1);
    let mixed = mixed.chars().take(MIB / 2).collect::<String>();
    // One escape per KiB: a clean run of 1023 bytes then a newline.
    let sparse = ("x".repeat(1023) + "\n").repeat(MIB / 1024);
    let mut group = c.benchmark_group("core_escape_scan");
    for (name, text) in [
        ("clean_1MiB", clean),
        ("mixed_dense", mixed),
        ("sparse_1MiB", sparse),
    ] {
        let term = literal(text);
        group.throughput(Throughput::Bytes(match &term {
            RdfTerm::Literal(l) => l.lexical_form.len() as u64,
            _ => 0,
        }));
        group.bench_function(name, |bencher| {
            bencher.iter(|| black_box(emit_term(black_box(&term))));
        });
    }
    group.finish();
}

/// `count` IRIs in sorted order, sharing a long path prefix that changes every
/// thousand entries, so adjacent records share tens of bytes.
fn dictionary_dataset(count: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri("http://example.org/o");
    for i in 0..count {
        let subject = builder.intern_iri(&format!(
            "http://example.org/data/section-{:04}/records/entity-{:08}",
            i / 1_000,
            i
        ));
        builder.push_quad(subject, predicate, object, None);
    }
    builder.freeze().expect("the dictionary fixture is valid")
}

fn bench_pack_dict_prefix(c: &mut Bench) {
    let dataset = dictionary_dataset(100_000);
    let encoded = PackDict::encode(&*dataset);
    let bytes = encoded.to_bytes();
    let mut group = c.benchmark_group("core_pack_dict_prefix");
    group.throughput(Throughput::Elements(encoded.n_terms()));
    group.bench_function("encode_100k_iris", |bencher| {
        bencher.iter(|| black_box(PackDict::encode(black_box(&*dataset))));
    });
    group.bench_function("decode_100k_iris", |bencher| {
        bencher.iter(|| {
            let parsed = EncodedDict::from_bytes(black_box(&bytes)).expect("parses");
            black_box(parsed.decode().expect("decodes"))
        });
    });
    group.finish();

    let mut group = c.benchmark_group("core_pack_dict_prefix_kernel");
    for len in [16_usize, 256, 4096] {
        let a = vec![b'a'; len];
        let mut b = a.clone();
        b[len - 1] = b'b';
        group.throughput(Throughput::Bytes(len as u64));
        group.bench_with_input(
            BenchmarkId::new("common_prefix_len", len),
            &(a, b),
            |bencher, (a, b)| bencher.iter(|| common_prefix_len(black_box(a), black_box(b))),
        );
    }
    group.finish();
}

bench_group!(benches, bench_escape_scan, bench_pack_dict_prefix);
bench_main!(benches);
