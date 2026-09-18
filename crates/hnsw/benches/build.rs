// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The build-cost harness: how long one deterministic HNSW build takes, at scale.
//!
//! This target exists to disclose the shipped build path's wall-clock cost at scale, rather
//! than leave it unmeasured. It builds the index through the **shipped build path** —
//! [`HnswIndex::build`], not a private shortcut — at every rung of the shared declared
//! ladder (`corpus::LADDER`: 5,000, 50,000, 200,000 and 1,000,000 rows at 4,096
//! dimensions), times each once, and prints the table.
//!
//! It is **report-only**. Nothing here asserts a timing, no gate invokes it, and a slow
//! sample on a shared host is not a failure. The number's job is disclosure, not
//! admission.
//!
//! # Why one-shot rather than criterion
//!
//! Criterion's method is to repeat a closure until the sampling distribution settles.
//! That is exactly wrong for a build whose cost grows with row count and dimension: ten
//! samples at the largest scale would multiply an already-long build by ten and measure
//! the host's thermal state rather than the code. A harness that times one build per scale
//! is the honest instrument here, and the reason `[[bench]] harness = false` is deliberate.
//!
//! # The 10^6-row point
//!
//! Build cost is expected to grow superlinearly with row count: each insertion's search
//! visits more of an already-larger graph, so the per-row cost rises as the corpus does.
//!
//! **Every declared scale runs by default, including 10^6.** A default that skipped the one
//! scale the offer is about is how a missing measurement gets reported as a completed run.
//! That has a real cost: 10^6 rows at 4,096 `f64` components is about 30.5 GiB resident for
//! the matrix alone, before the graph, so a plain run of this harness needs a host that can
//! hold it. `PURRDF_HNSW_BENCH_SCALES` narrows the run for a smoke test on a host that
//! cannot; it can only take scales away, and `corpus::declared_scales` ENFORCES that by
//! filtering the parsed list against the declared ladder rather than trusting this sentence.
//! The ladder is shared with `benches/recall.rs`, so one documented value narrows both.
//!
//! ```text
//! cargo bench -p purrdf-hnsw --bench build                     # 5k / 50k / 200k / 1M
//! PURRDF_HNSW_BENCH_SCALES=5000 cargo bench -p purrdf-hnsw --bench build   # smoke only
//! ```
//!
//! The vectors of each scale are dropped before the next scale is generated, so the peak
//! footprint is one scale's matrix plus its graph, not the sum of every scale.
//!
//! # Determinism of the fixture
//!
//! The corpus is generated from one fixed splitmix64 stream over the stable row-major
//! index, with the exact binary64 expression the crate's own digest corpus uses. No
//! random-number crate, no clock and no entropy source is consulted, so the same scale
//! measures the same vectors on every run and every host. The reported `graph` column is
//! an FNV-1a fold of the canonical payload image, which proves each scale built a real,
//! non-empty graph rather than timing an allocation.

#![allow(missing_docs)]

use std::hint::black_box;
use std::time::Instant;

use purrdf_core::DistanceMetric;

#[path = "../tests/support/corpus.rs"]
mod corpus;

use corpus::CorpusShape;
use purrdf_hnsw::{HnswIndex, Params, VectorMatrix};

/// The index identity every scale is built under.
///
/// These are required configuration, not tuning knobs: a build under different numbers is a
/// different index, so the table would be meaningless without them printed beside it. They
/// mirror the determinism corpus (`crates/hnsw/src/determinism.rs`) so the two harnesses
/// are measuring the same profile.
const M: usize = 16;
const M0: usize = 32;
const EF_CONSTRUCTION: usize = 200;
const EF_SEARCH: usize = 64;

/// The seed of the fixture stream.
const SEED: u64 = 0x484e_5357_5f42_5549;

/// FNV-1a over the canonical payload bytes: six lines, fixed constants, no state.
///
/// The same doctrine as `purrdf_hnsw::determinism` and `purrdf_geo::determinism`: the
/// digest must be a function of the bytes and nothing else, so it is hand-rolled rather
/// than a `DefaultHasher` or `ahash` whose value is a property of the toolchain.
const fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut index = 0;
    while index < bytes.len() {
        hash = (hash ^ bytes[index] as u64).wrapping_mul(0x0000_0100_0000_01b3);
        index += 1;
    }
    hash
}

fn main() {
    let params = Params::new(M, M0, EF_CONSTRUCTION, EF_SEARCH)
        .expect("the harness parameters satisfy the validation matrix");
    println!("purrdf-hnsw build cost (report-only; no assertion, no gate)");
    println!(
        "metric=squared-euclidean M={M} M0={M0} ef_construction={EF_CONSTRUCTION} \
         ef_search={EF_SEARCH} threads={}",
        rayon::current_num_threads()
    );
    println!();
    println!(
        "{:>10}  {:>6}  {:>12}  {:>12}  {:>10}  {:>16}",
        "rows", "dims", "generate", "build", "img MiB", "graph digest"
    );
    println!("{}", "-".repeat(76));

    // Every declared rung runs. A point that is not measured is a point this harness does
    // not claim, and a default that silently skips the one scale the offer is about is how a
    // missing measurement gets reported as a completed run. `PURRDF_HNSW_BENCH_SCALES`
    // narrows the run for a smoke test; `declared_scales` filters it against the ladder, so
    // it cannot widen one.
    for (scale, dims) in corpus::declared_scales() {
        let generate = Instant::now();
        let vectors = matrix(scale, dims);
        let generate = generate.elapsed();

        let start = Instant::now();
        let index = HnswIndex::build(vectors, &DistanceMetric::SquaredEuclidean, params)
            .expect("the generated corpus builds");
        let build = start.elapsed();

        // A digest is proof the build produced a graph rather than timing an allocation; a
        // scale whose digest is zero or equal to another scale's would be measuring
        // nothing. Computing it is outside the build timing and happens before the index
        // is dropped, so the peak footprint stays one scale at a time.
        let image = index.canonical_image();
        let digest = fnv1a_64(black_box(&image));
        println!(
            "{scale:>10}  {dims:>6}  {:>12.3}  {:>12.3}  {:>10.1}  {:>16}",
            generate.as_secs_f64(),
            build.as_secs_f64(),
            image.len() as f64 / (1024.0 * 1024.0),
            format!("{digest:016x}")
        );
        drop(index);
        drop(image);
    }

    println!();
    println!(
        "Totals are single samples from a shared host; treat them as the disclosed build \
         cost, not as an acceptance threshold."
    );
}

/// The corpus every scale is built over.
///
/// The SAME generator `benches/recall.rs` scores against, deliberately: this crate's central
/// finding is that the distribution decides the recall, so a build-cost harness drawing from
/// a different distribution than the recall harness would be timing a different question.
fn matrix(rows: usize, dims: usize) -> VectorMatrix {
    corpus::embedding_like(CorpusShape::embedding_like(rows, dims), SEED)
        .expect("the generated corpus is finite and rectangular")
}
