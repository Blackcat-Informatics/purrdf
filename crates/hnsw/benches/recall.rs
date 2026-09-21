// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The admission evidence: recall, work, and latency, measured **only against the exact
//! oracle**.
//!
//! This harness answers the question the crate's contract refuses to answer by assertion:
//! how good is the approximation, and what does it cost? It builds the index under varying
//! `ef_search`, queries it, and reports every figure beside the exact scan over the same
//! corpus:
//!
//! * **recall@k** — the fraction of the exact `k` nearest rows the index offered;
//! * **the exact-rank distribution** — where each offered row actually sits in the exact
//!   ordering, which is what distinguishes "missed the top ten" from "missed everything";
//! * **visited-work counts** — the distance evaluations each search performed, the work
//!   the relation reports through `PfCursor::take_work`;
//! * **p50/p99 query latency** — the index against the exact scan.
//!
//! # Recall is comparable only within one `ef`
//!
//! Every figure here is a function of `ef_search`, and the index identity *includes*
//! `ef_search` — the parameter is not a query-time override. So the harness builds one
//! index per `ef`, prints each on its own row, and never divides one row's numbers by
//! another's. A ratio across rows would be a comparison between two different indexes
//! wearing the same corpus, which is exactly the misreading this rule avoids.
//!
//! # The oracle is the exact path, not a second implementation
//!
//! The exact scan here is [`Kernel::distance`] over every row, selected by [`best`] — the
//! same public kernel and the same total order the `purrdf-sparql-eval` kNN relation runs,
//! consumed from that crate rather than re-derived. It is the sole comparator: no
//! "expected" recall is hard-coded, and no figure is compared to anything but the exact
//! answer over the same query and the same corpus.
//!
//! # Why one-shot rather than criterion
//!
//! The output this evidence needs — p50/p99 percentiles over a fixed query set, an
//! exact-rank histogram, and a visited-work total — is not criterion's output, and the
//! point of the target is a readable table rather than a statistical estimate. Timings are
//! wall-clock samples from a shared host and are disclosed as such; nothing here asserts a
//! latency, and no gate runs it.

#![allow(missing_docs)]

use std::hint::black_box;
use std::time::Instant;

use purrdf_core::DistanceMetric;
#[path = "../tests/support/corpus.rs"]
mod corpus;

use corpus::CorpusShape;
use purrdf_hnsw::level::splitmix64;
use purrdf_hnsw::{HnswIndex, Params, VectorMatrix};
use purrdf_sparql_eval::knn::{Kernel, Ranked, best, norm};

/// The one kernel both paths rank by.
const KERNEL: Kernel = Kernel::SquaredEuclidean;

/// The query set: the first [`QUERIES`] rows, and how many repetitions each latency sample
/// takes. Recall and rank are computed once per query; latency is sampled `REPEATS` times.
const QUERIES: usize = 64;
const REPEATS: usize = 5;

/// The requested neighbour count.
const K: usize = 10;

/// The `ef_search` values reported, each a distinct index identity.
const EF_VALUES: [usize; 6] = [16, 32, 64, 128, 256, 512];

/// The build identity, held fixed so `ef_search` is the only thing that moves.
const M: usize = 16;
const M0: usize = 32;
const EF_CONSTRUCTION: usize = 200;

/// The seed of the fixture stream.
const SEED: u64 = 0x484e_5357_5f52_4543;

/// A uniform family in `[-1, 1)` from the fixed splitmix64 stream.
fn uniform(rows: usize, dims: usize) -> VectorMatrix {
    let elements = rows
        .checked_mul(dims)
        .expect("the fixture shape fits usize");
    let mut state = SEED;
    let mut data = Vec::with_capacity(elements);
    for _ in 0..elements {
        state = splitmix64(state);
        let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
        let value = unit.mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.25 } else { value });
    }
    VectorMatrix::new(rows, dims, data).expect("the generated matrix is finite and rectangular")
}

/// Every row scored against `query_row`, in the exact path's order.
fn exact_scored(vectors: &VectorMatrix, norms: &[f64], query_row: usize) -> Vec<Ranked> {
    let query = vectors.row(query_row);
    (0..vectors.rows())
        .map(|row| Ranked {
            distance: KERNEL
                .distance(query, norms[query_row], vectors.row(row), norms[row])
                .expect("the fixture is finite and the kernel keeps it so"),
            row,
        })
        .collect()
}

/// The `p`-th percentile of `samples`, in the samples' unit. `p` is a percentage.
fn percentile(samples: &mut [u64], p: f64) -> u64 {
    samples.sort_unstable();
    if samples.is_empty() {
        return 0;
    }
    let rank = ((p / 100.0) * samples.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(samples.len() - 1);
    samples[index]
}

/// The exact-rank bucket of a returned row, or `None` when it is outside the 32·k window
/// the histogram tracks.
fn rank_bucket(rank: usize) -> usize {
    let mut edge = K;
    let mut bucket = 0;
    while bucket < 5 {
        if rank < edge {
            return bucket;
        }
        edge *= 2;
        bucket += 1;
    }
    bucket
}

/// The exact-rank histogram's bucket labels.
const BUCKETS: [&str; 6] = ["0..k", "k..2k", "2k..4k", "4k..8k", "8k..16k", ">=16k"];

/// The corpus families this harness reports, and why both are present.
///
/// `uniform` is the CONTROL, not the subject. Independent coordinates make pairwise
/// distances concentrate, so at this width the nearest and hundredth-nearest rows differ by
/// about a percent of a typical distance and no graph has a gradient to descend. A recall
/// figure measured there is a statement about the generator. It is reported anyway, beside
/// the structured family, because the comparison is the evidence: a number with nothing to
/// compare it against cannot show whether an index or a corpus is responsible for it.
///
/// `embedding-like` is the subject: low intrinsic dimension carried into the full width, a
/// decaying spectrum so leading coordinates dominate, power-law cluster sizes, and cluster
/// tightness fixed at an intended cosine rather than an absolute noise amplitude. See
/// the shared corpus harness for why each of those is load-bearing.
const FAMILIES: [&str; 2] = ["uniform", "embedding-like"];

/// Build the named family at `rows x dims`.
fn family(name: &str, rows: usize, dims: usize) -> VectorMatrix {
    match name {
        "uniform" => uniform(rows, dims),
        _ => corpus::embedding_like(CorpusShape::embedding_like(rows, dims), SEED)
            .expect("the shaped corpus is finite and rectangular"),
    }
}

fn main() {
    println!("purrdf-hnsw recall / work / latency against the exact oracle (report-only; no gate)");
    println!(
        "queries={QUERIES} k={K} metric=squared-euclidean M={M} M0={M0} \
         ef_construction={EF_CONSTRUCTION}"
    );
    println!("The exact oracle is Kernel::distance + best() over every row.");
    println!("Recall is comparable ONLY within one ef; each row is a distinct index identity.");
    println!(
        "Every rung of the shared declared ladder is reported. Nothing here is gated off by \
         default: a point that is not measured is a point this harness does not claim."
    );
    println!();

    // The ladder is `corpus::LADDER`, shared with `benches/build.rs` so that one
    // `PURRDF_HNSW_BENCH_SCALES` value narrows BOTH harnesses. Two private tables meant the
    // one variable had two vocabularies, and the documented narrowing example was a command
    // that panicked in whichever harness the writer had not run.
    for (rows, dims) in corpus::declared_scales() {
        for name in FAMILIES {
            report(name, rows, dims);
        }
    }

    println!(
        "The exact oracle returns recall 1.000 at every ef by construction; an index that \
         beats it on latency by missing its rows is the trade this table prices."
    );
}

/// Measure one `(family, rows, dims)` across the whole `ef` sweep.
///
/// The graph is built ONCE. `ef_search` is search-side -- the build never reads it -- so
/// rebuilding per `ef` would pay the dominant cost six times over for six identical graphs.
/// Each sweep step rebinds the declared parameter instead, which mints a new artifact
/// identity (its canonical image differs) without touching the graph, and is emphatically
/// not a query-time override: every search still runs at whatever its index declares.
fn report(name: &str, rows: usize, dims: usize) {
    let vectors = family(name, rows, dims);
    let norms: Vec<f64> = (0..rows).map(|row| norm(vectors.row(row))).collect();

    // The exact ordering for every query, computed once: it is the oracle the index is
    // scored against, and recomputing it per `ef` would not change a bit of it.
    let exact_ordered: Vec<Vec<Ranked>> = (0..QUERIES)
        .map(|query| {
            let mut scored = exact_scored(&vectors, &norms, query);
            scored.sort_unstable();
            scored
        })
        .collect();

    println!("--- corpus={name} {rows}x{dims} ---");

    let seed_params = Params::new(M, M0, EF_CONSTRUCTION, EF_VALUES[0]).expect("valid parameters");
    let mut index = HnswIndex::build(vectors, &DistanceMetric::SquaredEuclidean, seed_params)
        .expect("the fixture builds");
    // The rank table does not depend on `ef`, and `ordered` covers every row, so one buffer
    // overwritten in place replaces `queries * EF_VALUES.len()` allocate-and-fill passes.
    let mut rank_by_row = vec![usize::MAX; rows];

    for ef in EF_VALUES {
        index = index.rebind_ef_search(ef).expect("a valid beam width");
        // The index owns the vectors now; read them back rather than keeping a second copy.
        // Re-borrowed each round because the rebind above consumes the index.
        let vectors = index.matrix();

        let mut hits = 0_usize;
        let mut searched = 0_usize;
        let mut visited_total = 0_u64;
        let mut visited_max = 0_u64;
        let mut buckets = [0_usize; BUCKETS.len()];

        for (query, ordered) in exact_ordered.iter().enumerate() {
            for (rank, scored) in ordered.iter().enumerate() {
                rank_by_row[scored.row] = rank;
            }
            let (offered, visited) = index
                .search_rows_work(query, K)
                .expect("the query is in bounds");
            visited_total += visited;
            visited_max = visited_max.max(visited);
            searched += offered.len();
            for scored in &offered {
                let rank = rank_by_row[scored.row];
                if rank < K {
                    hits += 1;
                }
                buckets[rank_bucket(rank)] += 1;
            }
        }

        // Latency, sampled over the same query set for both paths. Reported, never
        // asserted: a wall-clock number from a machine under load measures the machine.
        let mut index_ns = Vec::with_capacity(QUERIES * REPEATS);
        let mut exact_ns = Vec::with_capacity(QUERIES * REPEATS);
        for query in 0..QUERIES {
            for _ in 0..REPEATS {
                let start = Instant::now();
                let offered = index.search_rows(query, K).expect("the query is in bounds");
                index_ns.push(start.elapsed().as_nanos() as u64);
                black_box(&offered);

                let start = Instant::now();
                let scored = exact_scored(vectors, &norms, query);
                let answer = best(K, scored);
                exact_ns.push(start.elapsed().as_nanos() as u64);
                black_box(&answer);
            }
        }

        let recall = hits as f64 / (QUERIES * K) as f64;
        let index_p50 = percentile(&mut index_ns, 50.0);
        let index_p99 = percentile(&mut index_ns, 99.0);
        let exact_p50 = percentile(&mut exact_ns, 50.0);
        let exact_p99 = percentile(&mut exact_ns, 99.0);
        let visited_mean = visited_total as f64 / QUERIES as f64;

        println!(
            "ef={ef:<4} recall@{K}={recall:.4}  offered/query={:.1}  visited/query mean={:.1} \
             max={visited_max}",
            searched as f64 / QUERIES as f64,
            visited_mean,
        );
        let histogram: Vec<String> = BUCKETS
            .iter()
            .zip(buckets)
            .map(|(label, count)| format!("{label}={count}"))
            .collect();
        println!("       exact-rank of offered rows: {}", histogram.join(" "));
        println!(
            "       latency  index p50={:.0}us p99={:.0}us   exact p50={:.0}us p99={:.0}us",
            index_p50 as f64 / 1_000.0,
            index_p99 as f64 / 1_000.0,
            exact_p50 as f64 / 1_000.0,
            exact_p99 as f64 / 1_000.0,
        );
        println!();
    }
}
