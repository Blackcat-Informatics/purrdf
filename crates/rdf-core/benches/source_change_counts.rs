// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Fixed work of the segmented seal and the source change feed: allocations,
//! requested and retained bytes, and retired user-mode instructions.
//!
//! `cargo bench -p purrdf-core --bench source_change_counts [-- <row filter>]`
//! prints one allocation line per row, measured in process. With `--counts`
//! (Linux and `perf`), each row also runs in fresh child processes under the
//! test kit's counter protocol and prints its instruction samples. Rows:
//!
//! * `seal`: `SegmentedBuilder::seal`, which writes and certifies the image.
//! * `capture`: the first source version of a one-image stack.
//! * `feed_layers`: the change feed after replacing that image with one whose
//!   middle row changed.
//! * `feed_head`: the same change in a resident head.
//!
//! The fixture has `PURRDF_SOURCE_CHANGE_ROWS` rows (2048 by default), each a
//! quad, a reifier and an annotation. Fixture construction, sealing of the
//! compared images and the previous version are outside every window. The
//! counting allocator also runs in the instruction samples; its work is the
//! same for every build compared.

use std::hint::black_box;
use std::ops::ControlFlow;
use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_core::source::SourceVisit;
use purrdf_core::{
    DatasetView, RdfDataset, SegmentedBuildLimits, SegmentedBuilder, SegmentedLayer,
    SegmentedLayerMetadata, SegmentedReadLimits, SegmentedResidentHead, SegmentedStack,
};

#[path = "../tests/support/source_snapshot.rs"]
mod source_snapshot;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const ROWS: [&str; 4] = ["seal", "capture", "feed_layers", "feed_head"];

fn rows() -> usize {
    std::env::var("PURRDF_SOURCE_CHANGE_ROWS").map_or(2048, |raw| {
        raw.parse()
            .expect("PURRDF_SOURCE_CHANGE_ROWS is an integer")
    })
}

fn limits() -> SegmentedReadLimits {
    SegmentedReadLimits::new(1 << 30, 64, 20_000_000, 1 << 34, 32)
}

fn builder(dataset: &RdfDataset) -> SegmentedBuilder {
    let mut builder = SegmentedBuilder::new(
        SegmentedBuildLimits::new(1 << 20, 1 << 28, 1 << 20, 4096, 64).unwrap(),
    )
    .unwrap();
    for (stream, rows) in [
        dataset.quads().collect::<Vec<_>>(),
        dataset.reifier_quads().collect(),
        dataset.annotation_quads().collect(),
    ]
    .into_iter()
    .enumerate()
    {
        for row in rows {
            let mut values = vec![
                dataset.term_value(row.s),
                dataset.term_value(row.p),
                dataset.term_value(row.o),
            ];
            values.extend(row.g.map(|id| dataset.term_value(id)));
            let mut ids = Vec::new();
            builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
            let quad = purrdf_core::QuadIds {
                s: ids[0],
                p: ids[1],
                o: ids[2],
                g: ids.get(3).copied(),
            };
            match stream {
                0 => builder.push_quad(quad).unwrap(),
                1 => builder.push_reifier(quad.s, quad.o, quad.g).unwrap(),
                _ => builder.push_annotation(quad).unwrap(),
            }
        }
    }
    builder
}

fn layer(dataset: &RdfDataset) -> Arc<SegmentedLayer> {
    let image = builder(dataset).seal().unwrap();
    Arc::new(
        SegmentedLayer::new(
            Arc::new(image.provider()),
            image.receipt().clone(),
            SegmentedLayerMetadata::default(),
        )
        .unwrap(),
    )
}

fn head(dataset: Arc<RdfDataset>) -> SegmentedResidentHead {
    SegmentedResidentHead::new(dataset, SegmentedLayerMetadata::default()).unwrap()
}

/// Prepare one row outside the window; the returned closure is the measured
/// work and returns its witness.
fn prepare(row: &str, count: usize) -> Box<dyn FnOnce() -> u64> {
    let original = source_snapshot::dataset(count, false, false);
    let changed = source_snapshot::dataset(count, true, false);
    match row {
        "seal" => {
            let builder = builder(&original);
            Box::new(move || black_box(builder.seal().unwrap()).bytes().len() as u64)
        }
        "capture" => {
            let mut stack = SegmentedStack::default();
            stack.publish(vec![layer(&original)], limits()).unwrap();
            let snapshot = stack.snapshot().unwrap();
            Box::new(move || {
                let query = snapshot.query(limits()).unwrap();
                let version = black_box(query.source_version().unwrap().unwrap());
                u64::try_from(version.statement_count()).unwrap()
            })
        }
        "feed_layers" | "feed_head" => {
            let mut stack = SegmentedStack::default();
            if row == "feed_layers" {
                stack.publish(vec![layer(&original)], limits()).unwrap();
            } else {
                stack.set_head(Some(head(original)));
            }
            let previous = stack
                .snapshot()
                .unwrap()
                .query(limits())
                .unwrap()
                .source_version()
                .unwrap()
                .unwrap();
            if row == "feed_layers" {
                stack.publish(vec![layer(&changed)], limits()).unwrap();
            } else {
                stack.set_head(Some(head(changed)));
            }
            let snapshot = stack.snapshot().unwrap();
            Box::new(move || {
                let query = snapshot.query(limits()).unwrap();
                let mut emitted = 0_u64;
                let observed = query
                    .visit_source_changes(&previous, |_, _, _| {
                        emitted += 1;
                        ControlFlow::<()>::Continue(())
                    })
                    .unwrap();
                let SourceVisit::Complete { version, .. } = observed else {
                    panic!("the change feed completes");
                };
                black_box(version);
                emitted
            })
        }
        _ => panic!("unknown row {row}"),
    }
}

#[cfg(target_os = "linux")]
fn child(row: &str, count: usize) {
    use purrdf_testkit::bench::counter::{CounterCommand, CounterControl};
    let work = prepare(row, count);
    let mut control = CounterControl::required().expect("admitted instruction counter");
    control.command(CounterCommand::Enable).expect("enable");
    let witness = work();
    control.command(CounterCommand::Disable).expect("disable");
    println!("witness={witness}");
}

#[cfg(target_os = "linux")]
fn counts(row: &str, count: usize, witness: u64) {
    use purrdf_testkit::bench::counter::collect_instructions;
    let samples: usize = std::env::var("PURRDF_SOURCE_CHANGE_COUNT_SAMPLES").map_or(3, |raw| {
        raw.parse().expect("the sample count is an integer")
    });
    let executable = std::env::current_exe().expect("the bench binary path");
    let home = std::env::temp_dir().join(format!("source_change_counts_{}", std::process::id()));
    std::fs::create_dir_all(&home).expect("the scratch directory");
    let mut taken = Vec::new();
    for sample in 0..samples {
        let directory = home.join(format!("{row}_{sample}"));
        let args = ["--child".into(), row.into(), count.to_string().into()];
        let sample = collect_instructions(&executable, &args, &directory)
            .unwrap_or_else(|error| panic!("{row}: {error}"));
        let stdout =
            std::fs::read_to_string(directory.join("workload.stdout")).expect("the child's output");
        assert_eq!(stdout.trim(), format!("witness={witness}"), "{row}");
        taken.push(sample.instructions.to_string());
    }
    let _ = std::fs::remove_dir_all(&home);
    println!(
        "[source_change_counts] {row}: rows={count} instructions={}",
        taken.join(",")
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--child") {
        #[cfg(target_os = "linux")]
        child(&args[1], args[2].parse().expect("the row count"));
        return;
    }
    let instructions = args.iter().any(|arg| arg == "--counts");
    let filters: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    let count = rows();
    for row in ROWS {
        if !filters.is_empty() && !filters.iter().any(|filter| row.contains(filter.as_str())) {
            continue;
        }
        let work = prepare(row, count);
        let window = WholeProcessWindow::open();
        let witness = work();
        let measured = window.close();
        println!(
            "[source_change_counts] {row}: rows={count} witness={witness} allocations={} \
             requested_bytes={} retained_bytes={} peak_working_bytes={}",
            measured.allocations,
            measured.requested_bytes,
            measured.retained_bytes,
            measured.peak_working_bytes,
        );
        if instructions {
            #[cfg(target_os = "linux")]
            counts(row, count, witness);
            #[cfg(not(target_os = "linux"))]
            panic!("native instruction counting requires Linux perf");
        }
    }
}
