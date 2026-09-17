// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The conformance gate: **every** HNSW result, compared against the exact oracle.
//!
//! This is the admission evidence the contract requires. It walks a deterministic fixture family —
//! a uniform corpus plus deliberately adversarial small ones (duplicates, equidistant
//! points, clusters, a hub, near-ties, a singular corpus, and a one-dimensional boundary)
//! — and, for every query, every `ef_search` and every `k`, it does three things:
//!
//! 1. **compares every returned row against the exact oracle.** The distance the index
//!    reports for a row must equal the distance the exact path's shared [`Kernel`]
//!    computes for that pair, bit for bit. This is what catches a wrapper that
//!    re-implements the arithmetic: a second implementation that rounds differently would
//!    still rank, and would be a silently different answer.
//! 2. **asserts the offer is well-formed.** Results are strictly increasing under the
//!    exact path's own [`Ranked`] total order `(distance, row)`, in bounds, and distinct.
//!    Ties therefore resolve by ascending row in the index exactly as they do in the
//!    oracle, because both read the same comparator.
//! 3. **emits a machine-checkable receipt.** For every run it records the index identity
//!    (implementation id, parameter encoding, media type, loss evidence, and the FNV-1a
//!    digest of the canonical payload image), the four parameters, `ef_search`, the
//!    visited distance-evaluation count, the offered rows and the exact rows, and
//!    `complete = false`. The receipt is written to `$CARGO_TARGET_TMPDIR/hnsw-conformance`
//!    (falling back to `target/hnsw-conformance`) and read back and checked, so the
//!    artifact is proved to exist and to parse rather than merely intended.
//!
//! # The approximation is not asserted away
//!
//! A suite where the approximation never misses would be watching an exact search. So the
//! family includes a deliberately sparse regime (`M = M0 = ef_construction = 2`,
//! `ef_search = 1`) and asserts that it *does* miss: at least one run must offer fewer
//! than the exact `k`, and a miss is recorded as an incomplete offer, never as absence.
//! `complete = false` on every run is the contract, not an occasional event.
//!
//! # Recall is comparable only within one `ef`
//!
//! Every figure is keyed by its `ef_search` and its build identity. The suite never divides
//! one regime's recall by another's — a comparison across `ef`, or across two different
//! `M` sets, would be a statement about two different indexes wearing one corpus, which is
//! exactly what the task forbids.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use purrdf_core::DistanceMetric;
use purrdf_hnsw::level::splitmix64;
use purrdf_hnsw::{HnswIndex, IMPLEMENTATION_ID, INDEX_MEDIA_TYPE, Params, VectorMatrix, profile};
use purrdf_sparql_eval::knn::{Kernel, Ranked, best, norm};
use std::fmt::Write as _;

use serde_json::{Value, json};

/// The one kernel both the index and the oracle rank by.
const KERNEL: Kernel = Kernel::SquaredEuclidean;

/// The metric the index is built under; must name [`KERNEL`].
const METRIC: DistanceMetric = DistanceMetric::SquaredEuclidean;

/// The seed of the deterministic uniform fixture stream.
const SEED: u64 = 0x0c0f_5e2a_9d71_4b83;

/// One named corpus in the conformance family.
struct Fixture {
    name: &'static str,
    matrix: VectorMatrix,
}

/// FNV-1a over the canonical payload bytes: fixed constants, no hasher choice to drift.
const fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut index = 0;
    while index < bytes.len() {
        hash = (hash ^ bytes[index] as u64).wrapping_mul(0x0000_0100_0000_01b3);
        index += 1;
    }
    hash
}

/// A uniform family in `[-1, 1)` from the splitmix64 stream, exact zero displaced.
fn uniform(rows: usize, dims: usize, seed: u64) -> VectorMatrix {
    let elements = rows
        .checked_mul(dims)
        .expect("the fixture shape fits usize");
    let mut state = seed;
    let mut data = Vec::with_capacity(elements);
    for _ in 0..elements {
        state = splitmix64(state);
        let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
        let value = unit.mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.25 } else { value });
    }
    VectorMatrix::new(rows, dims, data).expect("the generated matrix is finite and rectangular")
}

/// Two vectors repeated: a query is at distance zero from half the corpus and two from the
/// other half, so every distance is a tie shared by many rows.
fn duplicates(rows: usize, dims: usize) -> VectorMatrix {
    assert!(dims >= 2, "the duplicate pair needs two distinct axes");
    let mut first = vec![0.0; dims];
    first[0] = 1.0;
    let mut second = vec![0.0; dims];
    second[1] = 1.0;
    let data: Vec<Vec<f64>> = (0..rows)
        .map(|row| {
            if row % 2 == 0 {
                first.clone()
            } else {
                second.clone()
            }
        })
        .collect();
    VectorMatrix::from_rows(&data).expect("rectangular by construction")
}

/// An orthonormal basis: every pair of points is at squared distance two, so *all* pairs
/// tie and only the row tie-break can order them.
fn equidistant(rows: usize, dims: usize) -> VectorMatrix {
    assert!(rows <= dims, "an orthonormal basis needs one axis per row");
    let data: Vec<Vec<f64>> = (0..rows)
        .map(|row| {
            let mut vector = vec![0.0; dims];
            vector[row] = 1.0;
            vector
        })
        .collect();
    VectorMatrix::from_rows(&data).expect("rectangular by construction")
}

/// Four planted centroids with a small deterministic perturbation: structure the graph can
/// follow, which is the opposite regime to the near-equidistant corpora.
fn clusters(rows: usize, dims: usize) -> VectorMatrix {
    assert!(dims >= 4, "four centroids need four axes");
    let mut state = 0x5151_2a2a_9c9c_3e3e_u64;
    let data: Vec<Vec<f64>> = (0..rows)
        .map(|row| {
            let centroid = row % 4;
            (0..dims)
                .map(|axis| {
                    state = splitmix64(state);
                    let noise = ((state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(0.02, -0.01);
                    if axis == centroid { 3.0 + noise } else { noise }
                })
                .collect()
        })
        .collect();
    VectorMatrix::from_rows(&data).expect("rectangular by construction")
}

/// A dense hub around the origin with one far outlier: a single region holds most points.
fn hub(rows: usize, dims: usize) -> VectorMatrix {
    let mut state = 0x7777_1111_2222_3333_u64;
    let data: Vec<Vec<f64>> = (0..rows)
        .map(|row| {
            let scale = if row + 1 == rows { 10.0 } else { 0.01 };
            (0..dims)
                .map(|_| {
                    state = splitmix64(state);
                    ((state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0) * scale
                })
                .collect()
        })
        .collect();
    VectorMatrix::from_rows(&data).expect("rectangular by construction")
}

/// Rows separated by a vanishing increment: the exact ordering exists but is nearly flat,
/// which is the regime in which a graph with no gradient can return any of several rows.
fn near_ties(rows: usize, dims: usize) -> VectorMatrix {
    assert!(dims >= 2, "the near-tie fixture needs two distinct axes");
    let data: Vec<Vec<f64>> = (0..rows)
        .map(|row| {
            let mut vector = vec![0.0; dims];
            vector[0] = (row as f64).mul_add(1e-6, 1.0);
            vector[1] = 1.0;
            vector
        })
        .collect();
    VectorMatrix::from_rows(&data).expect("rectangular by construction")
}

/// Every row identical: all distances are zero, the maximal-tie corpus.
fn singular(rows: usize, dims: usize) -> VectorMatrix {
    let mut vector = vec![0.0; dims];
    vector[0] = 1.0;
    let data: Vec<Vec<f64>> = (0..rows).map(|_| vector.clone()).collect();
    VectorMatrix::from_rows(&data).expect("rectangular by construction")
}

/// The deterministic fixture family.
fn family() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "uniform-64x8",
            matrix: uniform(64, 8, SEED),
        },
        Fixture {
            name: "equidistant-16x16",
            matrix: equidistant(16, 16),
        },
        Fixture {
            name: "duplicates-32x8",
            matrix: duplicates(32, 8),
        },
        Fixture {
            name: "clusters-64x8",
            matrix: clusters(64, 8),
        },
        Fixture {
            name: "hub-48x4",
            matrix: hub(48, 4),
        },
        Fixture {
            name: "near-ties-32x4",
            matrix: near_ties(32, 4),
        },
        Fixture {
            name: "singular-16x4",
            matrix: singular(16, 4),
        },
        Fixture {
            name: "boundary-1d-16x1",
            matrix: uniform(16, 1, SEED ^ 0x00ff_00ff_00ff_00ff),
        },
    ]
}

/// Every row's L2 norm, computed once per fixture.
fn norms_of(matrix: &VectorMatrix) -> Vec<f64> {
    (0..matrix.rows())
        .map(|row| norm(matrix.row(row)))
        .collect()
}

/// Every row scored against `query`, in the exact path's order.
fn exact_scored(matrix: &VectorMatrix, norms: &[f64], query: usize) -> Vec<Ranked> {
    let vector = matrix.row(query);
    (0..matrix.rows())
        .map(|row| Ranked {
            distance: KERNEL
                .distance(vector, norms[query], matrix.row(row), norms[row])
                .expect("the fixture is finite and the kernel keeps it so"),
            row,
        })
        .collect()
}

/// One run's receipt value plus the two facts the caller asserts on.
struct Run {
    json: Value,
    missed: bool,
    /// The exact count of oracle rows the index offered across every query of the run.
    hits: u64,
    /// How many rows the run offered in total, so a hit count cannot be read without it.
    offered: u64,
}

/// Search one `(fixture, ef, k)` regime exhaustively and return its receipt entry.
///
/// Every returned row is compared to the exact oracle's distance for that pair, the offer's
/// order is checked against the shared `Ranked` total order, and recall is the fraction of
/// the exact `k` nearest that was offered.
fn run_regime(fixture: &Fixture, index: &HnswIndex, norms: &[f64], ef: usize, k: usize) -> Run {
    let params = index.params();
    let rows = fixture.matrix.rows();
    let mut observations = Vec::with_capacity(rows);
    let mut offered_total = 0_u64;
    let mut visited_total = 0_u64;
    let mut hits = 0_usize;
    let mut missed = false;

    for query in 0..rows {
        let mut ordered = exact_scored(&fixture.matrix, norms, query);
        ordered.sort_unstable();
        let exact_top = best(k, ordered.iter().copied());

        let mut rank_by_row = vec![usize::MAX; rows];
        for (rank, scored) in ordered.iter().enumerate() {
            rank_by_row[scored.row] = rank;
        }

        let (offered, visited) = index
            .search_rows_work(query, k)
            .expect("the query is an in-bounds row of the matrix");
        visited_total += visited;
        offered_total += offered.len() as u64;

        // (1) Every result compared against the exact oracle, and (2) well-formed order.
        for scored in &offered {
            assert!(
                scored.row < rows,
                "{}: the index offered row {} of {rows}",
                fixture.name,
                scored.row
            );
            let exact_distance = KERNEL
                .distance(
                    fixture.matrix.row(query),
                    norms[query],
                    fixture.matrix.row(scored.row),
                    norms[scored.row],
                )
                .expect("the fixture is finite and the kernel keeps it so");
            assert_eq!(
                scored.distance.to_bits(),
                exact_distance.to_bits(),
                "{}: ef={ef} k={k} query={query} row={} — the index reported {:#x} but the \
                 exact kernel computes {:#x}; a second distance implementation has drifted",
                fixture.name,
                scored.row,
                scored.distance.to_bits(),
                exact_distance.to_bits(),
            );
        }
        for pair in offered.windows(2) {
            assert!(
                pair[0] < pair[1],
                "{}: ef={ef} k={k} query={query} — the offer {} is not before {} under the \
                 shared (distance, row) order",
                fixture.name,
                pair[0].row,
                pair[1].row,
            );
        }
        assert!(
            offered.len() <= k.min(rows),
            "{}: ef={ef} k={k} query={query} — the offer holds {} rows, more than the {} asked \
             for",
            fixture.name,
            offered.len(),
            k,
        );

        let offered_rows: Vec<usize> = offered.iter().map(|scored| scored.row).collect();
        let exact_rows: Vec<usize> = exact_top.iter().map(|scored| scored.row).collect();
        let this_hits = offered_rows
            .iter()
            .filter(|row| rank_by_row[**row] < exact_rows.len())
            .count();
        hits += this_hits;
        if this_hits < exact_rows.len() {
            missed = true;
        }

        // Distinctness, asserted on the raw rows rather than inferred from the order.
        let distinct: BTreeSet<usize> = offered_rows.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            offered_rows.len(),
            "{}: ef={ef} k={k} query={query} — an offered row was repeated",
            fixture.name
        );

        observations.push(json!({
            "query": query,
            "offered": offered_rows.len(),
            "visited": visited,
            "hits": this_hits,
            "offered_rows": offered_rows,
            "exact_rows": exact_rows,
        }));
    }

    let recall = hits as f64 / (rows * k) as f64;
    let digest = fnv1a_64(&index.canonical_image());
    let run = json!({
        "fixture": fixture.name,
        "index_identity": {
            "implementation": IMPLEMENTATION_ID,
            "parameter_encoding": profile::PARAMETER_ENCODING,
            "payload_media_type": INDEX_MEDIA_TYPE,
            "loss_evidence": profile::LOSS_EVIDENCE,
            "canonical_image_digest": format!("{digest:016x}"),
        },
        "metric": "squared-euclidean",
        "parameters": {
            "M": params.m(),
            "M0": params.m0(),
            "ef_construction": params.ef_construction(),
            "ef_search": params.ef_search(),
        },
        "rows": rows,
        "dims": fixture.matrix.dims(),
        "k": k,
        "queries": rows,
        "offered_total": offered_total,
        "visited_total": visited_total,
        "visited_mean": visited_total as f64 / rows as f64,
        "recall_at_k": recall,
        "complete": false,
        "queries_detail": observations,
    });
    Run {
        json: run,
        missed,
        hits: hits as u64,
        offered: offered_total,
    }
}

/// The receipt directory: Cargo's per-target temp dir when the harness sets one, else the
/// workspace target directory beside the crate.
fn receipt_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_TMPDIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/hnsw-conformance"),
        |dir| PathBuf::from(dir).join("hnsw-conformance"),
    )
}

/// A receipt document: the schema, the profile, and every run.
fn receipt(name: &str, runs: &[Value]) -> Value {
    json!({
        "schema": "purrdf-hnsw-conformance-receipt-v1",
        "receipt": name,
        "implementation": IMPLEMENTATION_ID,
        "parameter_encoding": profile::PARAMETER_ENCODING,
        "payload_media_type": INDEX_MEDIA_TYPE,
        "loss_evidence": profile::LOSS_EVIDENCE,
        "runs": runs,
    })
}

/// Write a receipt and read it back, proving the artifact was emitted and parses.
fn emit(name: &str, document: &Value) -> PathBuf {
    let dir = receipt_dir();
    std::fs::create_dir_all(&dir).expect("the receipt directory is creatable");
    let path = dir.join(format!("{name}.json"));
    let text = serde_json::to_string_pretty(document).expect("the receipt serializes");
    std::fs::write(&path, &text).expect("the receipt is writable");
    assert!(
        path.is_file(),
        "the receipt artifact {} exists",
        path.display()
    );
    path
}

/// Read a receipt back and assert the schema every run must carry.
fn reread(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).expect("the emitted receipt reads back");
    let value: Value = serde_json::from_str(&text).expect("the emitted receipt is valid JSON");
    let runs = value["runs"]
        .as_array()
        .expect("the receipt holds a runs array");
    assert!(!runs.is_empty(), "a conformance receipt records its runs");
    for run in runs {
        assert_eq!(
            run["complete"],
            json!(false),
            "complete must be false on every run: an approximate offer never certifies absence"
        );
        assert_eq!(
            run["index_identity"]["implementation"],
            json!(IMPLEMENTATION_ID)
        );
        assert!(
            run["index_identity"]["canonical_image_digest"].is_string(),
            "every run records the index identity digest"
        );
        let ef = run["parameters"]["ef_search"]
            .as_u64()
            .expect("every run records ef_search");
        assert!(ef >= 1, "ef_search is at least one, got {ef}");
        assert!(
            run["visited_total"].as_u64().is_some(),
            "every run records the visited-work count"
        );
    }
    value
}

/// The build identity every regime shares above the `ef_search` axis.
fn params(m: usize, m0: usize, ef_construction: usize, ef_search: usize) -> Params {
    Params::new(m, m0, ef_construction, ef_search).expect("the regime parameters are valid")
}

/// The exact recall golden: `(fixture, regime ordinal, k, oracle rows offered, rows offered)`.
///
/// Recall is **deterministic** -- it is a pure function of the corpus, the parameters and
/// the algorithm, and no amount of machine load can move it -- so it is pinned as an exact
/// equality rather than floored. That is strictly stronger than a threshold: a floor is
/// silent when a change makes recall BETTER, and a change that improves recall is still a
/// change to a committed artifact's behaviour. It also matches the discipline this
/// repository already keeps everywhere else, where a golden is a byte, a lexical or a digest
/// and "a difference is a real defect, not a tolerance to widen". No threshold is asserted
/// anywhere in this workspace and none is introduced here.
///
/// `hits` counts rows by IDENTITY against the exact oracle's own top-`k`, which is the
/// strict reading: where several rows sit at exactly the same distance the oracle keeps the
/// lowest row numbers, and an index that returned equally-near rows with different numbers
/// is counted as missing them. The fixtures with heavy ties are therefore pessimistic here
/// by construction, and deliberately so -- the alternative is a metric that cannot see a
/// tie-break regression.
///
/// `offered` sits beside every hit count so no entry can be read without it. A hit count
/// alone is unfalsifiable: 0 of 0 and 64 of 64 are both "no misses".
///
/// To re-pin after a deliberate change, run this test; the failure prints the whole
/// measured table in this exact form.
const RECALL_GOLDEN: [(&str, usize, usize, u64, u64); 96] = [
    ("uniform-64x8", 0, 1, 49, 64),
    ("uniform-64x8", 0, 5, 62, 64),
    ("uniform-64x8", 0, 10, 64, 64),
    ("uniform-64x8", 1, 1, 59, 64),
    ("uniform-64x8", 1, 5, 244, 256),
    ("uniform-64x8", 1, 10, 256, 256),
    ("uniform-64x8", 2, 1, 63, 64),
    ("uniform-64x8", 2, 5, 314, 320),
    ("uniform-64x8", 2, 10, 624, 640),
    ("uniform-64x8", 3, 1, 13, 64),
    ("uniform-64x8", 3, 5, 46, 64),
    ("uniform-64x8", 3, 10, 59, 64),
    ("equidistant-16x16", 0, 1, 15, 16),
    ("equidistant-16x16", 0, 5, 16, 16),
    ("equidistant-16x16", 0, 10, 16, 16),
    ("equidistant-16x16", 1, 1, 15, 16),
    ("equidistant-16x16", 1, 5, 64, 64),
    ("equidistant-16x16", 1, 10, 64, 64),
    ("equidistant-16x16", 2, 1, 16, 16),
    ("equidistant-16x16", 2, 5, 80, 80),
    ("equidistant-16x16", 2, 10, 160, 160),
    ("equidistant-16x16", 3, 1, 6, 16),
    ("equidistant-16x16", 3, 5, 16, 16),
    ("equidistant-16x16", 3, 10, 16, 16),
    ("duplicates-32x8", 0, 1, 32, 32),
    ("duplicates-32x8", 0, 5, 32, 32),
    ("duplicates-32x8", 0, 10, 32, 32),
    ("duplicates-32x8", 1, 1, 32, 32),
    ("duplicates-32x8", 1, 5, 128, 128),
    ("duplicates-32x8", 1, 10, 128, 128),
    ("duplicates-32x8", 2, 1, 32, 32),
    ("duplicates-32x8", 2, 5, 160, 160),
    ("duplicates-32x8", 2, 10, 320, 320),
    ("duplicates-32x8", 3, 1, 0, 32),
    ("duplicates-32x8", 3, 5, 16, 32),
    ("duplicates-32x8", 3, 10, 32, 32),
    ("clusters-64x8", 0, 1, 53, 64),
    ("clusters-64x8", 0, 5, 64, 64),
    ("clusters-64x8", 0, 10, 64, 64),
    ("clusters-64x8", 1, 1, 63, 64),
    ("clusters-64x8", 1, 5, 254, 256),
    ("clusters-64x8", 1, 10, 256, 256),
    ("clusters-64x8", 2, 1, 64, 64),
    ("clusters-64x8", 2, 5, 320, 320),
    ("clusters-64x8", 2, 10, 640, 640),
    ("clusters-64x8", 3, 1, 15, 64),
    ("clusters-64x8", 3, 5, 44, 64),
    ("clusters-64x8", 3, 10, 50, 64),
    ("hub-48x4", 0, 1, 46, 48),
    ("hub-48x4", 0, 5, 48, 48),
    ("hub-48x4", 0, 10, 48, 48),
    ("hub-48x4", 1, 1, 48, 48),
    ("hub-48x4", 1, 5, 191, 192),
    ("hub-48x4", 1, 10, 192, 192),
    ("hub-48x4", 2, 1, 48, 48),
    ("hub-48x4", 2, 5, 240, 240),
    ("hub-48x4", 2, 10, 480, 480),
    ("hub-48x4", 3, 1, 16, 48),
    ("hub-48x4", 3, 5, 39, 48),
    ("hub-48x4", 3, 10, 43, 48),
    ("near-ties-32x4", 0, 1, 15, 32),
    ("near-ties-32x4", 0, 5, 22, 32),
    ("near-ties-32x4", 0, 10, 26, 32),
    ("near-ties-32x4", 1, 1, 27, 32),
    ("near-ties-32x4", 1, 5, 118, 128),
    ("near-ties-32x4", 1, 10, 128, 128),
    ("near-ties-32x4", 2, 1, 32, 32),
    ("near-ties-32x4", 2, 5, 159, 160),
    ("near-ties-32x4", 2, 10, 314, 320),
    ("near-ties-32x4", 3, 1, 10, 32),
    ("near-ties-32x4", 3, 5, 24, 32),
    ("near-ties-32x4", 3, 10, 26, 32),
    ("singular-16x4", 0, 1, 16, 16),
    ("singular-16x4", 0, 5, 16, 16),
    ("singular-16x4", 0, 10, 16, 16),
    ("singular-16x4", 1, 1, 16, 16),
    ("singular-16x4", 1, 5, 64, 64),
    ("singular-16x4", 1, 10, 64, 64),
    ("singular-16x4", 2, 1, 16, 16),
    ("singular-16x4", 2, 5, 80, 80),
    ("singular-16x4", 2, 10, 160, 160),
    ("singular-16x4", 3, 1, 0, 16),
    ("singular-16x4", 3, 5, 16, 16),
    ("singular-16x4", 3, 10, 16, 16),
    ("boundary-1d-16x1", 0, 1, 16, 16),
    ("boundary-1d-16x1", 0, 5, 16, 16),
    ("boundary-1d-16x1", 0, 10, 16, 16),
    ("boundary-1d-16x1", 1, 1, 16, 16),
    ("boundary-1d-16x1", 1, 5, 64, 64),
    ("boundary-1d-16x1", 1, 10, 64, 64),
    ("boundary-1d-16x1", 2, 1, 16, 16),
    ("boundary-1d-16x1", 2, 5, 80, 80),
    ("boundary-1d-16x1", 2, 10, 160, 160),
    ("boundary-1d-16x1", 3, 1, 12, 16),
    ("boundary-1d-16x1", 3, 5, 16, 16),
    ("boundary-1d-16x1", 3, 10, 16, 16),
];

/// The gate: the whole family, every regime, every result against the exact oracle.
#[test]
fn every_result_is_compared_to_the_exact_oracle() {
    let fixtures = family();
    let regimes = [
        params(4, 8, 16, 1),
        params(4, 8, 16, 4),
        params(4, 8, 16, 16),
        params(2, 2, 2, 1),
    ];
    let ks = [1_usize, 5, 10];
    let mut runs = Vec::new();
    let mut missed_anywhere = false;
    let mut measured: Vec<(&str, usize, usize, u64, u64)> = Vec::new();

    for fixture in &fixtures {
        let norms = norms_of(&fixture.matrix);
        for (ordinal, &regime) in regimes.iter().enumerate() {
            let index = HnswIndex::build(fixture.matrix.clone(), &METRIC, regime)
                .expect("the fixture builds");
            assert!(
                index.verify_rebuild().expect("the rebuild succeeds"),
                "{}: a freshly built index must be a pure function of its input",
                fixture.name
            );
            for &k in &ks {
                let run = run_regime(fixture, &index, &norms, regime.ef_search(), k);
                measured.push((fixture.name, ordinal, k, run.hits, run.offered));
                missed_anywhere |= run.missed;
                runs.push(run.json);
            }
        }
    }

    assert_eq!(
        measured.len(),
        RECALL_GOLDEN.len(),
        "the gate must grade exactly as many regimes as it pins"
    );
    if measured.as_slice() != RECALL_GOLDEN.as_slice() {
        let mut report = String::from(
            "the measured recall table does not match the pinned golden.\n\nRecall is \
             deterministic, so a difference here is a real behaviour change, never a \
             tolerance to widen. If the change was deliberate, re-pin RECALL_GOLDEN with \
             the table below and say in the commit WHICH rows moved and why.\n\n",
        );
        for (actual, expected) in measured.iter().zip(RECALL_GOLDEN.iter()) {
            let mark = if actual == expected { ' ' } else { '*' };
            let _ = writeln!(report, "{mark} {actual:?},");
        }
        panic!("{report}");
    }

    let path = emit("conformance-gate", &receipt("conformance-gate", &runs));
    let value = reread(&path);
    assert_eq!(
        value["runs"].as_array().expect("runs").len(),
        runs.len(),
        "the receipt records every run"
    );
    assert!(
        missed_anywhere,
        "the sparse regime must actually miss the exact oracle somewhere, or this suite is \
         watching an exact search; adjust the regime rather than deleting the assertion"
    );
}

/// A sparse index misses, and a miss is an incomplete offer — never a certification that the
/// missed row does not exist.
#[test]
fn a_sparse_index_misses_without_certifying_absence() {
    let fixture = Fixture {
        name: "sparse-uniform-128x8",
        matrix: uniform(128, 8, SEED ^ 0xaaaa_5555_aaaa_5555),
    };
    let norms = norms_of(&fixture.matrix);
    let regime = params(2, 2, 2, 1);
    let index = HnswIndex::build(fixture.matrix.clone(), &METRIC, regime).expect("builds");

    let k_values = [1_usize, 5];
    let mut runs = Vec::new();
    let mut misses = 0_usize;
    for &k in &k_values {
        let run = run_regime(&fixture, &index, &norms, regime.ef_search(), k);
        if run.missed {
            misses += 1;
        }
        runs.push(run.json);
    }

    let path = emit("sparse-miss", &receipt("sparse-miss", &runs));
    let value = reread(&path);
    assert!(
        misses > 0,
        "a graph with M = M0 = ef_construction = 2 and ef_search = 1 must miss the exact \
         oracle on a 128-row corpus; a suite where it never does proves nothing"
    );
    for (run, &k) in value["runs"]
        .as_array()
        .expect("runs")
        .iter()
        .zip(&k_values)
    {
        for observation in run["queries_detail"].as_array().expect("query detail") {
            let offered = observation["offered_rows"]
                .as_array()
                .expect("offered rows");
            let exact = observation["exact_rows"].as_array().expect("exact rows");
            // A missed exact row is simply absent from the offer; it is still a row of the
            // corpus, and the receipt says `complete = false` rather than claiming absence.
            for row in exact {
                assert!(
                    usize::try_from(row.as_u64().expect("row index")).expect("fits") < 128,
                    "a missed row is a real row of the space"
                );
            }
            assert!(offered.len() <= k, "an offer never exceeds the requested k");
        }
    }
}

/// Ties resolve by ascending row in the index exactly as in the oracle, because both read
/// the same `(distance, row)` comparator.
#[test]
fn tied_distances_break_by_row_in_both_paths() {
    let fixtures = [
        Fixture {
            name: "duplicates-32x8",
            matrix: duplicates(32, 8),
        },
        Fixture {
            name: "equidistant-16x16",
            matrix: equidistant(16, 16),
        },
        Fixture {
            name: "singular-16x4",
            matrix: singular(16, 4),
        },
    ];
    let regime = params(8, 16, 32, 16);
    let mut runs = Vec::new();

    for fixture in &fixtures {
        let norms = norms_of(&fixture.matrix);
        let index =
            HnswIndex::build(fixture.matrix.clone(), &METRIC, regime).expect("the fixture builds");
        let run = run_regime(fixture, &index, &norms, regime.ef_search(), 10).json;

        // The exact ordering is a strict total order, and where two distances are equal the
        // lower row comes first. The index's offer inherits that order, which `run_regime`
        // already asserted row by row; here the tie structure itself is pinned.
        let ordered = {
            let mut scored = exact_scored(&fixture.matrix, &norms, 0);
            scored.sort_unstable();
            scored
        };
        let has_tie = ordered
            .windows(2)
            .any(|pair| pair[0].distance == pair[1].distance);
        assert!(
            has_tie,
            "{}: this fixture exists to make distances tie, and none of its distances do",
            fixture.name
        );
        for pair in ordered.windows(2) {
            if pair[0].distance == pair[1].distance {
                assert!(
                    pair[0].row < pair[1].row,
                    "{}: two rows at the same distance are not in ascending row order ({} \
                     before {})",
                    fixture.name,
                    pair[0].row,
                    pair[1].row
                );
            }
        }
        runs.push(run);
    }

    let path = emit("ties", &receipt("ties", &runs));
    let value = reread(&path);
    assert_eq!(
        value["runs"].as_array().expect("runs").len(),
        fixtures.len()
    );
}
