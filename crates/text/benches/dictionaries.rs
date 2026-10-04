// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compare the complete five baseline dictionaries in both physical forms.
//! Artifact bytes are canonical vocabulary data and therefore equal between
//! forms; construction, loading, retained memory, lookup and lattice latency are
//! measured separately. Selection requires a latency win with no other material
//! regression outside measurement uncertainty.

use purrdf_hash::hex;
use purrdf_lex::json;
use purrdf_testkit::bench::{Bench, Throughput, bench_group, bench_main, black_box};
use purrdf_text::segment::{Dictionary, DictionaryRepresentation, SegmentationScratch};
use std::path::Path;

fn dictionaries(bench: &mut Bench) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("lexicons/artifacts");
    let manifest = json::read(
        &std::fs::read_to_string(root.join("manifest.json")).expect("generated manifest"),
    )
    .expect("valid manifest");
    for artifact in manifest["artifacts"].as_array().expect("artifact array") {
        let name = artifact["name"].as_str().expect("artifact name");
        let bytes =
            std::fs::read(root.join(artifact["artifact"].as_str().expect("artifact filename")))
                .expect("artifact bytes");
        let identity = hex::decode_32(
            artifact["physical_blake3"]
                .as_str()
                .expect("physical identity"),
        )
        .expect("BLAKE3");
        let radix = Dictionary::from_artifact(identity, &bytes).expect("verified artifact");
        let entries: Vec<_> = radix
            .weighted_entries()
            .map(|(word, cost)| (word.to_owned(), cost))
            .collect();
        let probes: Vec<_> = radix.words().step_by(97).map(str::to_owned).collect();
        let input = probes
            .iter()
            .take(96)
            .map(String::as_str)
            .collect::<String>();
        for (label, representation) in [
            ("radix", DictionaryRepresentation::Radix),
            ("minimal", DictionaryRepresentation::MinimalAcyclic),
        ] {
            let dictionary = radix.clone().with_representation(representation);
            eprintln!(
                "{name}/{label}: {:?}; artifact_bytes={}",
                dictionary.storage_stats(),
                bytes.len()
            );
            let mut group = bench.benchmark_group(format!("dictionary/{name}/{label}"));
            group.throughput(Throughput::Elements(dictionary.len() as u64));
            group.bench_function("construct", |b| {
                b.iter(|| {
                    black_box(
                        Dictionary::with_costs(entries.clone())
                            .expect("entries")
                            .with_representation(representation),
                    )
                });
            });
            group.bench_function("load", |b| {
                b.iter(|| {
                    black_box(
                        Dictionary::from_artifact(identity, black_box(&bytes))
                            .expect("artifact")
                            .with_representation(representation),
                    )
                });
            });
            group.throughput(Throughput::Elements(probes.len() as u64));
            group.bench_function("lookup", |b| {
                b.iter(|| {
                    for probe in &probes {
                        black_box(dictionary.cost(black_box(probe)));
                    }
                });
            });
            group.throughput(Throughput::Bytes(input.len() as u64));
            let mut scratch = SegmentationScratch::default();
            group.bench_function("segment", |b| {
                b.iter(|| {
                    dictionary.segment_each_with_scratch(black_box(&input), &mut scratch, |word| {
                        black_box(word);
                    });
                });
            });
            group.finish();
        }
    }
}
bench_group!(benches, dictionaries);
bench_main!(benches);
