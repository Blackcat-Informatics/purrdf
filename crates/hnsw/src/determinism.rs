// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The native half of `purrdf-hnsw`'s cross-target determinism proof.
//!
//! This crate's central claim is that a build is a pure function of its input: the same
//! corpus under the same parameters produces the **same canonical byte image** whether
//! one rayon worker or eight ran, and whether the target is native or
//! `wasm32-unknown-unknown`. The corpus and the level formula are already deterministic by
//! construction (splitmix64 over the stable row index, round-structured batches, a
//! canonical merge). This module turns that argument into **evidence**: it builds the
//! fixed corpus through the full build path, serializes the canonical payload image, and
//! folds the bytes into one `u64`.
//!
//! # Why the digest is hand-rolled FNV-1a
//!
//! The digest must be a function of the bytes and **nothing else**. `DefaultHasher` is
//! SipHash with an unspecified per-process key, and `ahash` is seed- and version-sensitive
//! by design; either would make the digest move for reasons that have nothing to do with
//! the graph, which is the exact false signal this harness exists to remove. FNV-1a is
//! six lines of integer arithmetic with published constants, so a golden that moves is a
//! serialization defect and never a hasher change.
//!
//! The corpus is generated from a seeded splitmix64 integer stream rather than committed
//! as a multi-hundred-megabyte binary: 5,000 rows at 4,096 dimensions. This is the
//! fixture the issue measured its own prototype on, and the one the cross-target gate in
//! `scripts/check-hnsw-determinism.sh` runs under `wasm32-unknown-unknown`.
//!
//! # The goldens
//!
//! [`digest`] is pinned natively by `crates/hnsw/tests/determinism.rs` against
//! `GOLDEN_DIGEST`, and the shell gate reads that constant out of the test source so
//! there is exactly one copy in the tree. The issue-author's prototype digests are cited
//! as prior evidence for the *property* (the round build differs from a serial insert);
//! they are not reproducible targets, because this crate's batch schedule and level
//! formula are specified here for the first time.

use purrdf_sparql_eval::knn::Kernel;

use crate::builder;
use crate::graph::VectorMatrix;
use crate::params::Params;

/// The number of rows in the digest corpus.
pub const CORPUS_ROWS: usize = 5_000;

/// The dimension of every row in the digest corpus.
pub const CORPUS_DIMS: usize = 4_096;

/// The seed of the splitmix64 stream the corpus is generated from.
///
/// A fixed constant, not a clock and not an entropy source: the corpus is a literal
/// function of the source text, which is what lets the native and wasm sides fold the
/// same bytes.
const CORPUS_SEED: u64 = 0x484e_5357_5f44_4947;

/// The pinned index identity the digest is computed under.
///
/// `ef_search` is part of the canonical image header even though it does not steer the
/// build, so it is part of the digest; it is fixed here rather than defaulted anywhere.
fn digest_params() -> Params {
    Params::new(16, 32, 200, 64).expect("the digest parameters are valid by construction")
}

/// The digest of the profile-rule build: the number the native test pins and the wasm gate
/// recomputes on the other target.
#[must_use]
pub fn digest() -> u64 {
    digest_with_batch(None)
}

/// The digest of a build forced to one row per round — a plain serial insertion.
///
/// The round-structured build pushes a whole batch against one frozen snapshot, so nodes
/// inside a batch cannot link to each other; a serial insertion lets every node see all
/// of its predecessors. The two graphs must therefore differ, and asserting that here is
/// what keeps the round structure load-bearing rather than an untested claim.
#[must_use]
pub fn digest_serial() -> u64 {
    digest_with_batch(Some(1))
}

/// The digest of a build whose round size is `batch`, or the profile rule when `None`.
///
/// Not part of the public contract; exposed so the determinism suite can drive the same
/// path with an explicit schedule without re-implementing the builder.
#[must_use]
pub fn digest_with_batch(batch: Option<usize>) -> u64 {
    let index =
        builder::build_with_batch(corpus(), Kernel::SquaredEuclidean, digest_params(), batch)
            .expect("the digest corpus is valid and builds");
    fnv1a_64(&index.canonical_image())
}

/// The number of corpus members the digest folds, so the harness can prove the digest is
/// not vacuous on the wasm side too.
#[must_use]
pub const fn corpus_len() -> usize {
    CORPUS_ROWS
}

/// The dimension of every corpus row.
#[must_use]
pub const fn corpus_dims() -> usize {
    CORPUS_DIMS
}

/// The fixed corpus: a uniform family in `[-1, 1)` generated from the seeded stream.
///
/// The generator is deterministic integer mixing followed by exactly-rounded binary64
/// arithmetic, so a native host and a wasm host produce bit-identical components. An
/// exact zero is displaced to `0.25` so the corpus is also well-formed under a
/// norm-dividing kernel, even though the digest builds under squared Euclidean.
#[must_use]
pub fn corpus() -> VectorMatrix {
    let mut state = CORPUS_SEED;
    let mut data = Vec::with_capacity(CORPUS_ROWS * CORPUS_DIMS);
    for _ in 0..CORPUS_ROWS * CORPUS_DIMS {
        state = crate::level::splitmix64(state);
        let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
        // `mul_add` is one correctly-rounded operation on every target, including wasm
        // without a fused-multiply-add instruction, so the corpus is bit-identical there.
        let value = unit.mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.25 } else { value });
    }
    VectorMatrix::new(CORPUS_ROWS, CORPUS_DIMS, data).expect("the corpus shape is valid")
}

/// FNV-1a over the canonical payload bytes: six lines, published constants, no state.
const fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut index = 0;
    while index < bytes.len() {
        hash = (hash ^ bytes[index] as u64).wrapping_mul(0x0000_0100_0000_01b3);
        index += 1;
    }
    hash
}
