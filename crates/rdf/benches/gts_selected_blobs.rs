// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! One-pass selected GTS import against the two-reader path it replaces.
//!
//! Report-only, `cargo bench -p purrdf-rdf --bench gts_selected_blobs`. It
//! asserts nothing. The selected importer exists to remove a second container
//! fold — the caller previously ran `import_gts_events` for the dataset and
//! then `reader::read` again to reach the blob bodies — so the comparison worth
//! recording is one fold against two over identical bytes, not a speedup claim.
//! Machine noise dominates small deltas; read the two curves, not their ratio.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_gts::wire::digest_str;
use purrdf_gts::writer::Writer;
use purrdf_rdf::{GtsBlobLimits, GtsBlobSelector, import_gts_events, import_gts_events_with_blobs};

/// A container holding `blobs` payloads of `size` bytes plus a little RDF, so
/// both paths fold the same dataset and reach the same bodies.
fn container(blobs: usize, size: usize) -> (Vec<u8>, String) {
    let mut writer = Writer::new("generic");
    let mut wanted = String::new();
    for index in 0..blobs {
        let mut payload = vec![b'x'; size];
        payload[..8.min(size)].copy_from_slice(&(index as u64).to_be_bytes()[..8.min(size)]);
        if index == 0 {
            wanted = digest_str(&payload);
        }
        writer.add_blob(&payload, None, Some(&format!("rep-{index}")));
    }
    (writer.into_bytes(), wanted)
}

fn selected_import(c: &mut Criterion) {
    let mut group = c.benchmark_group("gts_selected_blobs");
    for (blobs, size) in [(4_usize, 4_096_usize), (32, 4_096), (4, 262_144)] {
        let (bytes, wanted) = container(blobs, size);
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        let id = format!("{blobs}x{size}");

        // One fold: dataset and the selected body together.
        group.bench_with_input(BenchmarkId::new("one_pass", &id), &bytes, |b, bytes| {
            let limits = GtsBlobLimits::new(size * 2, size * 4);
            b.iter(|| {
                let result = import_gts_events_with_blobs(
                    black_box(bytes),
                    &[GtsBlobSelector::Digest(&wanted)],
                    limits,
                )
                .expect("selected import");
                black_box(result.blobs[0].bytes.len())
            });
        });

        // The path it replaces: fold for the dataset, then fold again for bytes.
        group.bench_with_input(BenchmarkId::new("two_readers", &id), &bytes, |b, bytes| {
            b.iter(|| {
                let bundle = import_gts_events(black_box(bytes)).expect("native import");
                let mut graph = purrdf_gts::reader::read(black_box(bytes), true, None);
                let body = graph
                    .blob_bytes_cloned(&wanted)
                    .expect("blob decode")
                    .expect("blob present");
                black_box((bundle.dataset.quad_count(), body.len()))
            });
        });
    }
    group.finish();
}

criterion_group!(benches, selected_import);
criterion_main!(benches);
