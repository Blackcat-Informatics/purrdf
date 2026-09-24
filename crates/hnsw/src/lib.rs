// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **A deterministic HNSW index.**
//!
//! HNSW (Hierarchical Navigable Small World) is an approximate nearest-neighbour graph.
//! Almost every implementation draws each inserted node's level from a random number
//! generator, which makes the graph a function of an RNG state instead of a function of
//! the data. This crate is the same algorithm with the RNG removed:
//!
//! * **Levels** are `splitmix64(stable_row_index)` run through a fixed formula — a pure
//!   function of the row, with no seed and no entropy ([`level`]).
//! * **The build** runs in finite batches; each batch proposes against a frozen snapshot
//!   of the graph and the proposals are merged canonically, so the graph does not depend
//!   on insertion order or on how many rayon workers ran.
//! * **The entry point** is the minimum row index at the maximum level — computed from the
//!   levels before any link exists, so it too is a pure function of the data.
//! * **Neighbour selection** keeps a diverse set rather than the nearest `M`, so the graph
//!   stays navigable; the admission rule is a total function of the beam.
//! * **Distances** come from the shared kernels of [`purrdf_sparql_eval::knn`], computed
//!   under the index's [`Arithmetic`] -- by default [`purrdf_core::distance::Exact`], the
//!   sixteen-lane binary64 fold with a fixed pairwise tree, the same bits on every target
//!   and dispatch path -- and candidates are ordered by the shared [`Ranked`] type
//!   `(distance, row)`, so there is no second comparator to drift out of step. Every call
//!   site (build, neighbour selection, search, decode and rebuild verification) runs under
//!   an arithmetic resolved for its thread, which refuses a flush-to-zero or re-rounding
//!   float environment with [`HnswError::FloatEnvironment`]; a beam expands each node by
//!   one call to the batch kernel. The public per-pair methods of [`VectorMatrix`] take the
//!   same resolved handle, so no distance is computed on a thread that was not checked.
//!
//! The exact index's **canonical byte image** is byte-identical across thread counts,
//! across dispatch paths and across `wasm32-unknown-unknown` with or without `+simd128`.
//! `rayon` runs inline-sequentially on wasm, so the wasm build is slower but not
//! different. The image header records the arithmetic its distances were folded under
//! ([`INDEX_VERSION`] 2); a version-1 image, folded sequentially, is refused with
//! [`HnswError::VersionMismatch`].
//!
//! # The reassociated index
//!
//! [`HnswIndex::build_reassociated`] builds the same graph algorithm under
//! [`purrdf_core::distance::Reassociated`], whose sums may be reassociated and contracted
//! to fused multiply-add along the dispatch path a build resolves, the widest this
//! process runs. It is a second type, `HnswIndex<Reassociated>`, not a mode of the first.
//! Its image is still byte-identical across thread counts, because every call site on one
//! path computes one pair's distance with the same compiled copy; but it is **bound to its
//! build and its dispatch path**. The header records the path's image code and the
//! build's [`BuildShape`] -- its target architecture and the target features that decide
//! what the path compiles to -- the implementation is [`IMPLEMENTATION_ID_REASSOCIATED`]
//! and its evidence revision names the path ([`profile::loss_evidence_reassociated`]).
//! Every decode, rebuild verification and search runs the recorded path, whichever path is
//! widest here: an image built on `x86_64`'s SSE2 or AVX2+FMA path runs on that path on an
//! AVX-512F processor, whose binary holds every `x86_64` compilation. A process that cannot
//! run the recorded path -- its compilation belongs to another target, or the processor
//! lacks a feature it needs -- is refused with [`HnswError::ArithmeticPathUnavailable`],
//! and a build of another shape with [`HnswError::ArithmeticBuildMismatch`], rather than
//! answered with bits from a different compilation. Equal path and shape are still not
//! the same compiled artifact: CPU tuning and the compiler version are invisible to the
//! source, so a rebuild that diverges under them is refused with
//! [`HnswError::ArithmeticRebuildDiverged`] rather than answered `false`.
//!
//! # The approximation contract, stated honestly
//!
//! This index is *approximate*. Its **recall is unmeasured on realistic corpora**, and it
//! never replaces the exact kNN path: `purrdf-sparql-eval`'s kNN relation remains the
//! oracle, and a result from this crate is an **offer of candidates**, never a proof that
//! no closer candidate exists. Parameters are required with no defaults, because `M`,
//! `M0`, `ef_construction`, and `ef_search` are the index identity rather than tuning
//! knobs.
//!
//! # Example
//!
//! ```
//! use purrdf_hnsw::{build, VectorMatrix, Params};
//! use purrdf_core::DistanceMetric;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let matrix = VectorMatrix::from_rows(&[
//!     vec![1.0, 0.0],
//!     vec![0.0, 1.0],
//!     vec![0.9, 0.1],
//! ])?;
//! let params = Params::new(2, 4, 8, 4)?;
//! let index = build(matrix, &DistanceMetric::SquaredEuclidean, params)?;
//! let nearest = index.search_rows(0, 2)?;
//! assert_eq!(nearest.len(), 2);
//! # Ok(())
//! # }
//! ```

mod builder;
mod graph;
mod search;
mod select;

pub mod determinism;
pub mod error;
pub mod guard;
pub mod level;
pub mod params;
pub mod profile;
pub mod relation;

pub use error::{HnswError, Result};
pub use graph::VectorMatrix;
pub use params::Params;

// The distance kernels and the rank order are consumed, never re-implemented: this module
// re-exports the exact path's types so a caller names one kernel and one comparator.
pub use purrdf_sparql_eval::knn::{Kernel, Ranked};

use std::sync::Mutex;

use purrdf_core::DistanceMetric;
use purrdf_core::distance::{
    Arithmetic, BuildShape, Exact, Reassociated, RecordedPathError, Resolved,
};
use rayon::prelude::*;

use crate::graph::{Graph, Recorded, decode_image};
use crate::search::{DistanceCache, Query, Visited, greedy_descend, search_layer};

/// The canonical image format version.
///
/// Version 2 folds every recorded distance with a named arithmetic and records that
/// arithmetic, and for a reassociated index its dispatch path, in the header.
pub const INDEX_VERSION: u32 = 2;

/// The media type of an HNSW index payload.
pub const INDEX_MEDIA_TYPE: &str = "application/vnd.blackcatinformatics.purrdf.hnsw";

/// The derived-index implementation identifier of an index whose distances are computed
/// under the [`Exact`] arithmetic.
pub const IMPLEMENTATION_ID: &str = "hnsw-v2";

/// The derived-index implementation identifier of an index whose distances are computed
/// under the [`Reassociated`] arithmetic.
///
/// A separate implementation rather than a flag on the exact one: the two publish
/// different evidence, and a guard naming one can never be read as the other.
pub const IMPLEMENTATION_ID_REASSOCIATED: &str = "hnsw-reassociated-v2";

/// Arithmetic `A`, resolved for the calling thread on the dispatch path an image recorded
/// as `recorded`.
///
/// The recorded path is resolved itself, not the widest this process runs: a processor
/// runs every compilation its binary holds whose features it reports, so an image recorded
/// on `x86_64`'s SSE2 or AVX2+FMA path is searched, verified and rebuilt on that path by an
/// AVX-512F processor. Only a path this process cannot run is refused.
///
/// # Errors
///
/// [`HnswError::FloatEnvironment`] when the thread's float environment is not the IEEE
/// one the arithmetic defines; [`HnswError::ArithmeticMismatch`] for a code that is not
/// one of `A`'s; and [`HnswError::ArithmeticPathUnavailable`] when the recorded path's
/// compilation is not in this build or the processor does not report a feature it needs.
pub(crate) fn resolve_recorded<A: Arithmetic>(recorded: u32) -> Result<Resolved<A>> {
    #[cfg(test)]
    if let Some(widest) = path_hook::lacking(recorded) {
        A::resolve()?;
        return Err(HnswError::ArithmeticPathUnavailable {
            recorded,
            available: widest,
        });
    }
    match A::resolve_recorded(recorded) {
        Ok(resolved) => Ok(resolved),
        Err(RecordedPathError::FloatEnvironment(error)) => Err(error.into()),
        Err(RecordedPathError::Unavailable { .. }) => Err(HnswError::ArithmeticPathUnavailable {
            recorded,
            available: A::resolve()?.image_code(),
        }),
        Err(RecordedPathError::UnknownCode { .. }) => Err(HnswError::ArithmeticMismatch {
            arithmetic: A::ID,
            actual: recorded,
        }),
    }
}

/// Refuse an image whose recorded build shape is not this build's.
///
/// `recorded` is the shape the image header carries, present exactly when `A` has one, so
/// an exact image always passes and a reassociated one passes only in a build of its own
/// shape.
///
/// # Errors
///
/// [`HnswError::ArithmeticBuildMismatch`] naming both shapes.
pub(crate) fn check_build<A: Arithmetic>(recorded: Option<BuildShape>) -> Result<()> {
    match (recorded, A::build_shape()) {
        (Some(recorded), Some(here)) if recorded != here => {
            Err(HnswError::ArithmeticBuildMismatch { recorded, here })
        }
        (Some(_), Some(_)) | (None, None) => Ok(()),
        (recorded, here) => unreachable!(
            "the decoder reads a build shape exactly when {} has one; recorded {recorded:?}, \
             here {here:?}",
            A::ID
        ),
    }
}

/// The refusal of a rebuild that produced another image, or `None` for an arithmetic whose
/// rebuild inequality is an honest "no".
///
/// For an arithmetic with a build shape the rebuild ran on the recorded path under the
/// recorded shape, which [`check_build`] proved is this build's; what can still differ is
/// only what no shape records, so the answer is [`HnswError::ArithmeticRebuildDiverged`].
fn diverged<A: Arithmetic>(recorded: u32) -> Option<HnswError> {
    A::build_shape().map(|shape| HnswError::ArithmeticRebuildDiverged { recorded, shape })
}

/// The test hook that stands the calling thread on a processor that cannot run a path.
///
/// No test can make the processor it runs on stop reporting a feature, so a unit test
/// that needs a refusal on a path this host really runs names it here.
#[cfg(test)]
pub(crate) mod path_hook {
    use std::cell::Cell;

    thread_local! {
        /// The recorded code the calling thread cannot run, and the widest code it
        /// reports instead.
        static LACKING: Cell<Option<(u32, u32)>> = const { Cell::new(None) };
    }

    /// While alive, the calling thread cannot run the path it was told it lacks.
    pub(crate) struct Lacking {
        previous: Option<(u32, u32)>,
    }

    impl Drop for Lacking {
        fn drop(&mut self) {
            LACKING.with(|lacking| lacking.set(self.previous));
        }
    }

    /// Refuse `recorded` on this thread, reporting `widest` as the widest path it runs,
    /// until the guard drops.
    pub(crate) fn lack(recorded: u32, widest: u32) -> Lacking {
        Lacking {
            previous: LACKING.with(|lacking| lacking.replace(Some((recorded, widest)))),
        }
    }

    /// The widest code to report when a test made `recorded` unrunnable on this thread.
    pub(crate) fn lacking(recorded: u32) -> Option<u32> {
        LACKING
            .with(Cell::get)
            .and_then(|(code, widest)| (code == recorded).then_some(widest))
    }
}

/// One search traversal, as [`Compiled`] holds it.
pub(crate) type Traverse<A> =
    for<'q> fn(&HnswIndex<A>, &Query<'q, A>, usize, &mut Visited) -> Result<Vec<Ranked>>;

/// One graph build, as [`Compiled`] holds it.
pub(crate) type BuildGraph<A> =
    fn(&VectorMatrix, Resolved<A>, Kernel, Params, Option<usize>) -> Result<(Graph, Vec<f64>)>;

/// The two walks whose cost is the index's -- the search traversal and the graph build --
/// compiled for arithmetic `A` in this crate.
///
/// Every index holds one, and only the non-generic constructors fill it in
/// ([`HnswIndex::build`], [`HnswIndex::build_reassociated`], [`HnswIndex::decode`],
/// [`HnswIndex::decode_reassociated`] and the determinism digest), where `A` is concrete.
/// So both walks, under both arithmetics, are instantiated here, in `purrdf-hnsw`, rather
/// than in whichever downstream crate first calls a generic search method: the copy the
/// asm evidence gate measures is the copy every search, batch and rebuild verification
/// runs, whoever calls it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Compiled<A: Arithmetic> {
    /// Greedy descent, then the layer-0 beam.
    traverse: Traverse<A>,
    /// The round-structured build and connectivity repair.
    pub(crate) build_graph: BuildGraph<A>,
}

impl<A: Arithmetic> Compiled<A> {
    /// This crate's compilation of both walks under `A`.
    ///
    /// Called only where `A` is concrete, which is what places the instantiation here.
    pub(crate) fn here() -> Self {
        Self {
            traverse: HnswIndex::<A>::traverse,
            build_graph: builder::build_graph::<A>,
        }
    }
}

/// The kernel `metric` names, or the refusal of a metric no kernel evaluates.
fn kernel_of(metric: &DistanceMetric) -> Result<Kernel> {
    Kernel::of(metric).ok_or_else(|| HnswError::UnsupportedMetric {
        metric: format!("{metric:?}"),
    })
}

/// Build an index over `matrix` under `metric` and `params`.
///
/// A free-function spelling of [`HnswIndex::build`] for callers that prefer it.
///
/// # Errors
///
/// As [`HnswIndex::build`].
pub fn build(matrix: VectorMatrix, metric: &DistanceMetric, params: Params) -> Result<HnswIndex> {
    HnswIndex::build(matrix, metric, params)
}

/// Build an index over `matrix` under `metric` and `params`, computing every distance under
/// the [`Reassociated`] arithmetic.
///
/// A free-function spelling of [`HnswIndex::build_reassociated`].
///
/// # Errors
///
/// As [`HnswIndex::build_reassociated`].
pub fn build_reassociated(
    matrix: VectorMatrix,
    metric: &DistanceMetric,
    params: Params,
) -> Result<HnswIndex<Reassociated>> {
    HnswIndex::build_reassociated(matrix, metric, params)
}

/// A built HNSW index: the graph, the matrix it was built over, and its identity.
///
/// The index owns its matrix. Search is therefore self-contained (no external matrix to
/// pass back in) and the vectors cannot drift out from under the graph they describe.
///
/// # The arithmetic is a type parameter, and the default is the exact one
///
/// `A` is the [`Arithmetic`] every distance the index computes -- at build, in neighbour
/// selection, in search and in rebuild verification -- runs under. [`HnswIndex::build`]
/// builds the [`Exact`] index, whose canonical image is the same bytes on every target and
/// dispatch path; [`HnswIndex::build_reassociated`] builds the [`Reassociated`] one, whose
/// distances may be reassociated and contracted along the dispatch path its build
/// resolved, so its image is bound to that build and that path. They are two types, not
/// one index with a mode, and neither ever computes a distance under the other's law.
#[derive(Debug)]
pub struct HnswIndex<A: Arithmetic = Exact> {
    /// The vectors the graph indexes, row-major.
    matrix: VectorMatrix,
    /// The metric the graph was built under.
    kernel: Kernel,
    /// The declared identity.
    params: Params,
    /// The graph itself.
    graph: Graph,
    /// Per-row L2 norms, empty for a kernel that does not divide by one.
    norms: Vec<f64>,
    /// The dispatch path every recorded distance was computed on, whose code the image
    /// header records.
    arithmetic: Resolved<A>,
    /// The walks this crate compiled for `A`.
    compiled: Compiled<A>,
    /// Reusable visited scratch, one buffer per concurrent search.
    ///
    /// `Visited` is generation-stamped: `O(rows)` to allocate once and `O(1)` to reset
    /// thereafter. Allocating a fresh one per search means zeroing a buffer the size of the
    /// corpus in order to visit a beam's worth of it -- four megabytes at a million rows,
    /// per query, to touch a few thousand nodes. The pool grows to the number of searches
    /// that were ever concurrent and no further, and its lock is taken twice per search
    /// rather than anywhere near a distance.
    scratch: Mutex<Vec<Visited>>,
}

impl HnswIndex {
    /// Build an index over `matrix` under `metric`, computing every distance under the
    /// [`Exact`] arithmetic.
    ///
    /// # Errors
    ///
    /// [`HnswError::UnsupportedMetric`] if `metric` is a caller-defined extension metric
    /// with no kernel this crate can evaluate.
    /// Otherwise the errors of the builder: the parameter validation matrix, a zero-norm
    /// row under a norm-dividing kernel, a non-finite distance, or
    /// [`HnswError::FloatEnvironment`] for a thread whose float environment is not the IEEE
    /// one.
    pub fn build(matrix: VectorMatrix, metric: &DistanceMetric, params: Params) -> Result<Self> {
        builder::build_with_batch(matrix, kernel_of(metric)?, params, None, Compiled::here())
    }

    /// Decode an exact index from its canonical image, over the matrix it describes.
    ///
    /// The matrix is supplied rather than embedded: the image is the graph and its
    /// identity, while the vectors are the source data the graph was derived from. The
    /// decoded graph must have one node per matrix row.
    ///
    /// # Errors
    ///
    /// [`HnswError::InvalidPayload`] for malformed or structurally invalid bytes,
    /// [`HnswError::VersionMismatch`] for an unimplemented version (a version-1 image among
    /// them), [`HnswError::ArithmeticMismatch`] for a header naming another arithmetic (a
    /// reassociated image among them: decode it with [`HnswIndex::decode_reassociated`]),
    /// [`HnswError::ParameterValidation`] / [`HnswError::ArithmeticOverflow`] if the
    /// decoded parameters do not describe the matrix, [`HnswError::ZeroNorm`] if the
    /// decoded kernel needs norms and a row has none, and [`HnswError::FloatEnvironment`]
    /// if the calling thread's float environment is not the IEEE one.
    pub fn decode(matrix: VectorMatrix, bytes: &[u8]) -> Result<Self> {
        Self::decode_with(matrix, bytes, Compiled::here())
    }

    /// Whether the graph `bytes` encodes is what `matrix` and its declared identity build,
    /// under the exact arithmetic.
    ///
    /// The borrowing form of [`HnswIndex::verify_rebuild`], for a caller that already holds
    /// the vectors and only wants the verdict. It decodes the image, rebuilds from the
    /// borrow, and compares -- so neither side copies a matrix that may be tens of
    /// gigabytes. A structurally invalid payload, a row-count disagreement or a parameter
    /// disagreement are all `Ok(None)`: this answers a question, it does not raise.
    ///
    /// # Errors
    ///
    /// [`HnswError::VersionMismatch`] for an image of another format version and
    /// [`HnswError::ArithmeticMismatch`] for one recorded under another arithmetic. Those
    /// are not a "no": a version-1 image is a real index whose distances were folded
    /// under a different law, and answering `false` would report it as tampered with when
    /// it is merely a format this build does not rebuild. Otherwise only what the rebuild
    /// itself can fail on -- a zero norm under a norm-dividing kernel, a non-finite
    /// distance, or a float environment the arithmetic refuses.
    pub(crate) fn verify_bytes_against(
        matrix: &VectorMatrix,
        bytes: &[u8],
    ) -> Result<Option<Params>> {
        Self::verify_bytes_with(matrix, bytes, Compiled::here())
    }
}

impl HnswIndex<Reassociated> {
    /// Build an index over `matrix` under `metric`, computing every distance -- at build,
    /// in neighbour selection, in search and in rebuild verification -- under the
    /// [`Reassociated`] arithmetic.
    ///
    /// The dispatch path is resolved here -- the widest this process runs -- and the
    /// image records it. Its distances may differ in their last bits from
    /// [`HnswIndex::build`]'s, so the graph may differ too wherever two candidates nearly
    /// tie; its recall is measured against the exact oracle exactly as the exact index's
    /// is. Its canonical image is reproducible only by the compiled build that made it, on
    /// the same dispatch path, so the image records the path and the build's
    /// [`BuildShape`]; every later decode, rebuild verification and search runs that path,
    /// on any process that can run it, and refuses one that cannot with
    /// [`HnswError::ArithmeticPathUnavailable`] and a build of another shape with
    /// [`HnswError::ArithmeticBuildMismatch`].
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::build`], whose [`HnswError::FloatEnvironment`] refusal is the
    /// reassociated arithmetic's here: it has a compilation for every target, so the
    /// thread's float environment is the only thing that refuses it.
    pub fn build_reassociated(
        matrix: VectorMatrix,
        metric: &DistanceMetric,
        params: Params,
    ) -> Result<Self> {
        builder::build_with_batch(matrix, kernel_of(metric)?, params, None, Compiled::here())
    }

    /// Decode a reassociated index from its canonical image, over the matrix it describes.
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::decode`], with [`HnswError::ArithmeticMismatch`] for a header that
    /// records a code of another arithmetic (an exact image among them), and
    /// [`HnswError::ArithmeticPathUnavailable`] for one recorded on a dispatch path this
    /// process cannot run, and [`HnswError::ArithmeticBuildMismatch`] for one recorded by a
    /// build of another shape. A path narrower than the widest this process runs is not
    /// refused: the decoded index searches and verifies on the path it recorded.
    pub fn decode_reassociated(matrix: VectorMatrix, bytes: &[u8]) -> Result<Self> {
        Self::decode_with(matrix, bytes, Compiled::here())
    }

    /// [`HnswIndex::verify_bytes_against`] for a reassociated image.
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::verify_bytes_against`], and
    /// [`HnswError::ArithmeticPathUnavailable`] for an image recorded on a dispatch path
    /// this process cannot run, or [`HnswError::ArithmeticBuildMismatch`] for one recorded
    /// by a build of another shape: rebuilding it here would recompute every distance with
    /// other last bits, so the answer would be a "no" that says nothing about the payload.
    /// [`HnswError::ArithmeticRebuildDiverged`] when the rebuild, on the recorded path under
    /// the recorded shape, produced another image: that is never `Ok(None)`, because a
    /// build that differs from the one that built the image only in what no shape records
    /// cannot be told apart from a payload that differs.
    pub(crate) fn verify_bytes_against_reassociated(
        matrix: &VectorMatrix,
        bytes: &[u8],
    ) -> Result<Option<Params>> {
        Self::verify_bytes_with(matrix, bytes, Compiled::here())
    }
}

impl<A: Arithmetic> HnswIndex<A> {
    /// Assemble an index from its parts.
    pub(crate) fn new(
        matrix: VectorMatrix,
        kernel: Kernel,
        params: Params,
        graph: Graph,
        norms: Vec<f64>,
        arithmetic: Resolved<A>,
        compiled: Compiled<A>,
    ) -> Self {
        Self {
            matrix,
            kernel,
            params,
            graph,
            norms,
            arithmetic,
            compiled,
            scratch: Mutex::new(Vec::new()),
        }
    }

    /// Arithmetic `A` resolved for the calling thread, on the path this index recorded.
    ///
    /// The build shape is not held here: an index in memory was built, or decoded and
    /// checked, by this build, so its shape is this build's.
    fn resolve_here(&self) -> Result<Resolved<A>> {
        resolve_recorded::<A>(self.arithmetic.image_code())
    }

    /// Run `search` against a pooled visited buffer, returning it afterwards.
    fn with_scratch<T>(&self, search: impl FnOnce(&mut Visited) -> T) -> T {
        let mut visited = self
            .scratch
            .lock()
            .map_or(None, |mut pool| pool.pop())
            .unwrap_or_else(|| Visited::new(self.matrix.rows()));
        let outcome = search(&mut visited);
        if let Ok(mut pool) = self.scratch.lock() {
            pool.push(visited);
        }
        outcome
    }

    /// The same graph under a different declared `ef_search`.
    ///
    /// `ef_search` is part of the artifact's identity, so this produces a **different
    /// artifact** -- its canonical image differs and a guard committing the old one will no
    /// longer verify. That is the point: advancing a parameter is a compatibility event, and
    /// this is the supported way to make one without paying to rebuild a graph that does not
    /// depend on the parameter being changed.
    ///
    /// It is emphatically not a query-time override. A search still runs at whatever
    /// `ef_search` its index declares; the caller who wants a different beam holds a
    /// different index.
    ///
    /// # Errors
    ///
    /// [`HnswError::InvalidParameter`] if `ef_search` is not a valid beam width.
    pub fn rebind_ef_search(self, ef_search: usize) -> Result<Self> {
        let params = Params::new(
            self.params.m(),
            self.params.m0(),
            self.params.ef_construction(),
            ef_search,
        )?;
        Ok(Self { params, ..self })
    }

    /// The `k` nearest rows to `query_row`, in rank order, nearest first.
    ///
    /// The beam width is the declared `ef_search`, capped at the row count — never widened
    /// to fit `k`. A request for more rows than the beam holds is answered with fewer than
    /// `k` rows rather than by searching a wider graph than the artifact declares. An empty
    /// `k` returns no rows.
    ///
    /// # Errors
    ///
    /// [`HnswError::RowOutOfBounds`] if `query_row` is not a row of the matrix.
    /// [`HnswError::NonFiniteDistance`] if a kernel result leaves the finite range.
    /// [`HnswError::FloatEnvironment`] if the calling thread's float environment is not the
    /// IEEE one. [`HnswError::ArithmeticPathUnavailable`] if the calling thread cannot run
    /// the dispatch path this index recorded, which only a reassociated index can raise.
    pub fn search_rows(&self, query_row: usize, k: usize) -> Result<Vec<Ranked>> {
        let arithmetic = self.resolve_here()?;
        let cache = DistanceCache::new();
        self.with_scratch(|visited| self.search_with(arithmetic, query_row, k, &cache, visited))
    }

    /// The `k` nearest rows to `query_row`, together with the number of candidate
    /// distance evaluations the search performed.
    ///
    /// This is [`HnswIndex::search_rows`] with the work it cost made observable: one unit
    /// per candidate distance the kernel computes, counted through the search's memo, so a
    /// candidate whose distance was already computed is not charged twice. The count is
    /// what a property-function cursor reports through `PfCursor::take_work` — the rows a
    /// search returns are `k`, and `k` says nothing about the size of the graph they were
    /// selected from.
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::search_rows`].
    pub fn search_rows_work(&self, query_row: usize, k: usize) -> Result<(Vec<Ranked>, u64)> {
        let arithmetic = self.resolve_here()?;
        let cache = DistanceCache::new();
        let ranked = self
            .with_scratch(|visited| self.search_with(arithmetic, query_row, k, &cache, visited))?;
        Ok((ranked, cache.evaluations()))
    }

    /// The `k` nearest rows to an arbitrary query vector, in rank order, nearest first.
    ///
    /// The general form of [`HnswIndex::search_rows`]. `query` need not be a row of the
    /// matrix and usually is not: an embedding search takes free text, embeds it, and asks
    /// for the neighbours of a vector that was never stored. Searching from a stored row is
    /// the special case where the vector happens to be one the index holds.
    ///
    /// This is also the honest case to measure recall on. A stored row is its own nearest
    /// neighbour at distance zero, so a member query asks the graph only to arrive at a node
    /// it already sits on; a held-out query is where descent actually has to navigate.
    ///
    /// The beam is the declared `ef_search`, exactly as for a member query — nothing about
    /// the query's provenance changes the artifact's identity.
    ///
    /// # Errors
    ///
    /// [`HnswError::ParameterValidation`] if `query` does not have the matrix's dimension.
    /// [`HnswError::NonFiniteDistance`] if a kernel result leaves the finite range, which
    /// includes a query vector whose own components are extreme enough to overflow the fold.
    /// [`HnswError::FloatEnvironment`] if the calling thread's float environment is not the
    /// IEEE one: the query's norm and every distance would already differ.
    /// [`HnswError::ArithmeticPathUnavailable`] as for [`HnswIndex::search_rows`].
    pub fn search_vector(&self, query: &[f64], k: usize) -> Result<Vec<Ranked>> {
        let cache = DistanceCache::new();
        let bound = self.bind_vector(query, &cache)?;
        self.with_scratch(|visited| (self.compiled.traverse)(self, &bound, k, visited))
    }

    /// [`HnswIndex::search_vector`] with the candidate distance evaluations it cost.
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::search_vector`].
    pub fn search_vector_work(&self, query: &[f64], k: usize) -> Result<(Vec<Ranked>, u64)> {
        let cache = DistanceCache::new();
        let bound = self.bind_vector(query, &cache)?;
        let ranked =
            self.with_scratch(|visited| (self.compiled.traverse)(self, &bound, k, visited))?;
        Ok((ranked, cache.evaluations()))
    }

    /// Validate a query vector's shape once and bind it to this index's kernel, the
    /// arithmetic resolved for this thread, and the memo.
    fn bind_vector<'a>(
        &'a self,
        query: &'a [f64],
        cache: &'a DistanceCache,
    ) -> Result<Query<'a, A>> {
        let arithmetic = self.resolve_here()?;
        if query.len() != self.matrix.dims() {
            return Err(HnswError::ParameterValidation {
                description: format!(
                    "the query has {} component(s) but this index ranks {}-dimensional \
                     vectors",
                    query.len(),
                    self.matrix.dims()
                ),
            });
        }
        Ok(Query::from_vector(
            &self.matrix,
            self.kernel,
            arithmetic,
            &self.norms,
            cache,
            query,
        ))
    }

    /// The `k` nearest rows to each of `query_rows`, one result per query in input order.
    ///
    /// The order is the input order, not completion order: the batch is mapped by rayon's
    /// indexed adaptor, so the answers line up with the questions even when workers
    /// finish out of order. Each worker reuses its visited scratch across the batch and
    /// resolves the arithmetic on its own thread; every query gets its own distance memo,
    /// because the memo is keyed by candidate row alone and a memo carried from one query
    /// to the next would hand the second query the first one's distances.
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::search_rows`], for the first failing query. A calling thread that
    /// cannot run the dispatch path recorded is refused before any query runs.
    pub fn search_batch(&self, query_rows: &[usize], k: usize) -> Result<Vec<Vec<Ranked>>> {
        self.resolve_here()?;
        let rows = self.matrix.rows();
        query_rows
            .par_iter()
            .map_init(
                || (Visited::new(rows), self.resolve_here()),
                |(visited, arithmetic), &query_row| {
                    let arithmetic = arithmetic.clone()?;
                    let cache = DistanceCache::new();
                    self.search_with(arithmetic, query_row, k, &cache, visited)
                },
            )
            .collect()
    }

    /// One search against caller-supplied scratch, from a stored row.
    fn search_with(
        &self,
        arithmetic: Resolved<A>,
        query_row: usize,
        k: usize,
        cache: &DistanceCache,
        visited: &mut Visited,
    ) -> Result<Vec<Ranked>> {
        let rows = self.matrix.rows();
        if query_row >= rows {
            return Err(HnswError::RowOutOfBounds {
                index: query_row,
                max: rows.saturating_sub(1),
            });
        }
        let query = Query::new(
            &self.matrix,
            self.kernel,
            arithmetic,
            &self.norms,
            cache,
            query_row,
        );
        (self.compiled.traverse)(self, &query, k, visited)
    }

    /// The traversal both entry points share: greedy descent, then the beam at layer 0.
    ///
    /// Takes a bound [`Query`] rather than a row, because the graph does not care whether
    /// the vector being ranked against is one it holds. Reached only through
    /// [`Compiled`], so the copy that runs is the one this crate compiled.
    fn traverse(
        &self,
        query: &Query<'_, A>,
        k: usize,
        visited: &mut Visited,
    ) -> Result<Vec<Ranked>> {
        if k == 0 {
            return Ok(Vec::new());
        }
        let Some(entry) = self.graph.entry() else {
            return Ok(Vec::new());
        };
        // `ef_search` is part of the declared artifact identity, so it is never widened to
        // fit a request. A `k` larger than the beam is answered with fewer than `k` rows:
        // this index offers candidates and never certifies absence, so a short answer is a
        // legal answer, while a widened beam would silently search a different graph than
        // the guard committed to.
        let ef = self.params.ef_search().min(self.matrix.rows());
        let (start, _) = greedy_descend(&self.graph, query, entry, self.graph.max_level(), 1)?;
        let mut result = search_layer(&self.graph, query, visited, &[start], 0, ef)?;
        result.truncate(k);
        Ok(result)
    }

    /// The index's canonical byte image.
    ///
    /// This is the image a PURREMB guard commits and the byte string a determinism digest
    /// folds: two builds that agree here are the same index. Its header records the
    /// arithmetic's image code for the path the build ran, and for an arithmetic whose
    /// bits depend on the build, the build's shape.
    #[must_use]
    pub fn canonical_image(&self) -> Vec<u8> {
        self.graph
            .canonical_image(self.kernel, &self.params, Recorded::of(self.arithmetic))
    }

    /// Decode an index under `A` from its canonical image.
    fn decode_with(matrix: VectorMatrix, bytes: &[u8], compiled: Compiled<A>) -> Result<Self> {
        let image = decode_image::<A>(bytes)?;
        // The recorded path, which every later search and rebuild verification runs; the
        // norms computed below are arithmetic under the thread's environment, which this
        // checks too.
        let here = resolve_recorded::<A>(image.arithmetic.code)?;
        check_build::<A>(image.arithmetic.shape)?;
        if image.graph.node_count() != matrix.rows() {
            return Err(HnswError::InvalidPayload {
                reason: format!(
                    "the payload holds {} node(s) but the matrix holds {} row(s)",
                    image.graph.node_count(),
                    matrix.rows()
                ),
            });
        }
        image
            .params
            .validate_against(matrix.rows(), matrix.dims())?;
        let norms = builder::compute_norms(&matrix, here.exact(), image.kernel)?;
        Ok(Self::new(
            matrix,
            image.kernel,
            image.params,
            image.graph,
            norms,
            here,
            compiled,
        ))
    }

    /// Decode `bytes` under `A`, rebuild from `matrix`, and compare.
    fn verify_bytes_with(
        matrix: &VectorMatrix,
        bytes: &[u8],
        compiled: Compiled<A>,
    ) -> Result<Option<Params>> {
        // The thread's environment is checked before the payload is read, so a refused
        // environment is never reported as a payload that failed to decode.
        A::resolve()?;
        let image = match decode_image::<A>(bytes) {
            Ok(image) => image,
            Err(
                error @ (HnswError::VersionMismatch { .. } | HnswError::ArithmeticMismatch { .. }),
            ) => return Err(error),
            Err(_) => return Ok(None),
        };
        // The rebuild runs on the recorded path, in a build of the recorded shape. A path
        // this process cannot run, or a build of another shape, cannot rebuild the image,
        // and saying `false` would report a real index as a tampered one.
        let here = resolve_recorded::<A>(image.arithmetic.code)?;
        check_build::<A>(image.arithmetic.shape)?;
        if image.graph.node_count() != matrix.rows()
            || image
                .params
                .validate_against(matrix.rows(), matrix.dims())
                .is_err()
        {
            return Ok(None);
        }
        let (rebuilt, _) = (compiled.build_graph)(matrix, here, image.kernel, image.params, None)?;
        if rebuilt.canonical_image(image.kernel, &image.params, image.arithmetic)
            == image
                .graph
                .canonical_image(image.kernel, &image.params, image.arithmetic)
        {
            Ok(Some(image.params))
        } else {
            diverged::<A>(image.arithmetic.code).map_or(Ok(None), Err)
        }
    }

    /// Rebuild the index from its own matrix and identity, and report whether the result
    /// is the same index.
    ///
    /// This turns "the payload is rebuildable" into a checkable claim: a `true` means the
    /// canonical image is a pure function of the source data and the declared parameters,
    /// while a `false` would mean the stored graph disagrees with what the declared
    /// identity builds. A reassociated index is a pure function of them only *in the
    /// compiled artifact that built it, on the dispatch path it recorded*, so the rebuild
    /// runs there or not at all, and a rebuild that differs is not a `false`.
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::build`], and [`HnswError::ArithmeticPathUnavailable`] if the calling
    /// thread cannot run the dispatch path this index recorded.
    /// [`HnswError::ArithmeticRebuildDiverged`] for a reassociated index whose rebuild, on
    /// its recorded path in a build of its recorded shape, produced another image: CPU
    /// tuning or the compiler version, which no build shape records, differ from the build
    /// that computed its distances, or its graph differs from the one that build produced,
    /// and the two cannot be told apart. For an exact index `false` is that answer, and it
    /// is evidence the graph was altered.
    pub fn verify_rebuild(&self) -> Result<bool> {
        // Rebuilt from a borrow: the vectors are already here, and copying a
        // million-row matrix in order to compare against it is the largest avoidable
        // allocation in the crate.
        let arithmetic = self.resolve_here()?;
        let (graph, _) =
            (self.compiled.build_graph)(&self.matrix, arithmetic, self.kernel, self.params, None)?;
        let recorded = Recorded::of(self.arithmetic);
        if graph.canonical_image(self.kernel, &self.params, recorded) == self.canonical_image() {
            return Ok(true);
        }
        diverged::<A>(recorded.code).map_or(Ok(false), Err)
    }

    /// The arithmetic every distance of this index is computed under, on the dispatch
    /// path its image records.
    ///
    /// [`Resolved::evidence`] is that arithmetic's divergence along the path, `None` for
    /// [`Exact`]; [`Resolved::image_code`] is the code the image header carries.
    #[must_use]
    pub const fn arithmetic(&self) -> Resolved<A> {
        self.arithmetic
    }

    /// The vectors this index was built over.
    ///
    /// The index owns its matrix so the graph and the vectors cannot drift apart. Exposing
    /// it by reference means a caller that also needs the vectors -- a harness scoring the
    /// index against an exact scan, say -- does not have to hand in a second copy.
    #[must_use]
    pub const fn matrix(&self) -> &VectorMatrix {
        &self.matrix
    }

    /// The exact distance between two stored rows, computed **without touching the
    /// graph**.
    ///
    /// The one arithmetic path: the same [`Kernel`], the same stored components and the
    /// same precomputed norms a traversal binds its query with, so the value is
    /// bit-identical to the one the beam would have produced for this pair had it
    /// visited it. There is no second distance formula in this crate and this does not
    /// add one.
    ///
    /// What it is *not* is a search. It visits no node, consults no layer and claims no
    /// rank: a rank is one plus the number of rows nearer the query, which is a fact
    /// about every other row. A caller that needs a rank searches; a caller that has
    /// already named both rows and needs only the number between them asks here.
    ///
    /// # Errors
    ///
    /// [`HnswError::RowOutOfBounds`] if either row is not a row of the matrix,
    /// [`HnswError::NonFiniteDistance`] if the kernel result leaves the finite range,
    /// [`HnswError::FloatEnvironment`] if the calling thread's float environment is not the
    /// IEEE one, and [`HnswError::ArithmeticPathUnavailable`] if the calling thread cannot
    /// run the dispatch path this index recorded, which only a reassociated index can raise.
    pub fn row_distance(&self, a: usize, b: usize) -> Result<f64> {
        let rows = self.matrix.rows();
        for index in [a, b] {
            if index >= rows {
                return Err(HnswError::RowOutOfBounds {
                    index,
                    max: rows.saturating_sub(1),
                });
            }
        }
        let arithmetic = self.resolve_here()?;
        self.matrix
            .distance(
                arithmetic,
                self.kernel,
                a,
                search::norm_of(&self.norms, a),
                b,
                search::norm_of(&self.norms, b),
            )
            .ok_or(HnswError::NonFiniteDistance { row: b })
    }

    /// The number of indexed rows.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.matrix.rows()
    }

    /// The vector dimension.
    #[must_use]
    pub fn dims(&self) -> usize {
        self.matrix.dims()
    }

    /// The kernel the index ranks with.
    #[must_use]
    pub const fn kernel(&self) -> Kernel {
        self.kernel
    }

    /// The declared identity.
    #[must_use]
    pub const fn params(&self) -> Params {
        self.params
    }

    /// The maximum level any committed node reaches.
    #[must_use]
    pub fn max_level(&self) -> u32 {
        self.graph.max_level()
    }

    /// The entry point, or `None` for an index with no rows.
    #[must_use]
    pub fn entry(&self) -> Option<usize> {
        self.graph.entry()
    }

    /// The level assigned to `row`.
    ///
    /// # Panics
    ///
    /// Panics if `row` is out of bounds; callers pass rows of the matrix.
    #[must_use]
    pub fn level(&self, row: usize) -> u32 {
        self.graph.level(row)
    }

    /// `row`'s neighbours at `layer`, in rank order.
    ///
    /// # Panics
    ///
    /// Panics if `row` is out of bounds or `layer` exceeds `row`'s level.
    #[must_use]
    pub fn neighbors(&self, row: usize, layer: u32) -> &[Ranked] {
        self.graph.neighbors(row, layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(rows: usize, dims: usize) -> VectorMatrix {
        let mut state = 0x0dd0_c0de_1234_5678_u64;
        let mut data = Vec::with_capacity(rows * dims);
        for _ in 0..rows * dims {
            state = level::splitmix64(state);
            let value = ((state >> 11) as f64 / (1_u64 << 53) as f64).mul_add(2.0, -1.0);
            data.push(if value == 0.0 { 0.25 } else { value });
        }
        VectorMatrix::new(rows, dims, data).expect("valid fixture")
    }

    fn params() -> Params {
        Params::new(4, 8, 16, 8).expect("valid")
    }

    #[test]
    fn a_query_finds_itself_and_respects_k() {
        let index = HnswIndex::build(fixture(32, 4), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        let nearest = index.search_rows(5, 3).expect("searches");
        assert_eq!(nearest.len(), 3);
        assert_eq!(
            nearest[0].row, 5,
            "the query is at distance zero from itself"
        );
        assert!(nearest[0].distance <= nearest[1].distance);
        assert!(nearest[1].distance <= nearest[2].distance);
    }

    #[test]
    fn a_zero_request_is_an_empty_answer_not_an_error() {
        let index = HnswIndex::build(fixture(16, 3), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        assert_eq!(
            index.search_rows(0, 0).expect("searches"),
            Vec::<Ranked>::new(),
            "zero neighbours is an empty answer, not an error"
        );
    }

    #[test]
    fn a_batch_query_preserves_input_order() {
        let index = HnswIndex::build(fixture(40, 4), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        let queries = [7_usize, 0, 39, 12];
        let results = index.search_batch(&queries, 2).expect("searches");
        assert_eq!(results.len(), queries.len());
        for (result, &query) in results.iter().zip(queries.iter()) {
            // The batch is an offer of candidates, not a guarantee that a given row is
            // found; what must hold is that each answer lines up with its own question and
            // equals the single-query answer exactly.
            assert_eq!(
                *result,
                index.search_rows(query, 2).expect("searches"),
                "answer {query} is out of place or disagrees with the single-query search"
            );
        }
    }

    #[test]
    fn an_index_verifies_its_own_rebuild_and_round_trips_through_bytes() {
        let index = HnswIndex::build(fixture(48, 4), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        assert!(index.verify_rebuild().expect("rebuilds"));

        let image = index.canonical_image();
        let decoded = HnswIndex::decode(fixture(48, 4), &image).expect("decodes");
        assert_eq!(decoded.canonical_image(), image);
        assert_eq!(
            decoded.search_rows(3, 4).expect("searches"),
            index.search_rows(3, 4).expect("searches")
        );
    }

    #[test]
    fn a_batch_answers_every_query_exactly_as_a_single_search_does() {
        // On one worker a rayon batch runs many queries through one `map_init` state. The
        // distance memo is keyed by candidate row alone, so a memo carried across queries
        // would hand each later query the earlier queries' distances. Every answer must
        // equal its own single search, row for row and bit for bit.
        let index = HnswIndex::build(fixture(64, 20), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        let queries: Vec<usize> = (0..64).collect();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("a one-worker pool");
        let batch = pool
            .install(|| index.search_batch(&queries, 5))
            .expect("searches");
        for (answer, &query) in batch.iter().zip(&queries) {
            let single = index.search_rows(query, 5).expect("searches");
            assert_eq!(answer, &single, "query {query}");
            assert_eq!(
                answer[0].row, query,
                "a stored row is its own nearest neighbour"
            );
            assert_eq!(answer[0].distance.to_bits(), 0.0_f64.to_bits());
        }
    }

    #[test]
    fn a_version_one_image_is_refused_by_name_rather_than_answered_false() {
        let matrix = fixture(48, 20);
        let index = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        let image = index.canonical_image();
        // What a version-1 encoder wrote: version 1, and zero in the field version 2 uses
        // for the arithmetic.
        let mut version_one = image.clone();
        version_one[8..12].copy_from_slice(&1_u32.to_le_bytes());
        version_one[60..64].copy_from_slice(&0_u32.to_le_bytes());
        let refusal = HnswError::VersionMismatch {
            expected: 2,
            actual: 1,
        };
        assert_eq!(
            HnswIndex::decode(matrix.clone(), &version_one).expect_err("refused"),
            refusal
        );
        assert_eq!(
            HnswIndex::verify_bytes_against(&matrix, &version_one).expect_err("refused"),
            refusal,
            "a version-1 image is an index folded under another law, not a tampered one"
        );
        // Another arithmetic's code is refused by name on the verification path too.
        let mut other_arithmetic = image.clone();
        other_arithmetic[60..64].copy_from_slice(&2_u32.to_le_bytes());
        assert!(matches!(
            HnswIndex::verify_bytes_against(&matrix, &other_arithmetic),
            Err(HnswError::ArithmeticMismatch { actual: 2, .. })
        ));

        // The neighbours: the image as built verifies, and a structurally corrupt one of
        // the current version is still an honest "no".
        assert_eq!(
            HnswIndex::verify_bytes_against(&matrix, &image).expect("verifies"),
            Some(params())
        );
        let mut truncated = image;
        truncated.pop();
        assert_eq!(
            HnswIndex::verify_bytes_against(&matrix, &truncated).expect("answers"),
            None
        );
    }

    #[test]
    fn the_versions_and_identifier_name_the_exact_arithmetic() {
        assert_eq!(INDEX_VERSION, 2);
        assert_eq!(IMPLEMENTATION_ID, "hnsw-v2");
        assert_eq!(IMPLEMENTATION_ID_REASSOCIATED, "hnsw-reassociated-v2");
        assert_eq!(profile::PAYLOAD_VERSION, INDEX_VERSION);
    }

    fn fixture_f32(rows: usize, dims: usize) -> VectorMatrix {
        let wide = fixture(rows, dims);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the fixture is narrowed on purpose, to exercise the binary32 storage path"
        )]
        let narrow: Vec<f32> = wide.as_slice().iter().map(|&value| value as f32).collect();
        VectorMatrix::from_f32(rows, dims, narrow).expect("valid fixture")
    }

    fn reassociated(matrix: VectorMatrix) -> HnswIndex<Reassociated> {
        HnswIndex::build_reassociated(matrix, &DistanceMetric::SquaredEuclidean, params())
            .expect("builds")
    }

    /// A reassociated code other than `recorded`: a path some other processor runs.
    fn another_path(recorded: u32) -> u32 {
        *Reassociated::IMAGE_CODES
            .iter()
            .find(|code| **code != recorded)
            .expect("the reassociated arithmetic has more than one path")
    }

    fn header_code(image: &[u8]) -> u32 {
        u32::from_le_bytes(image[60..64].try_into().expect("four header bytes"))
    }

    #[test]
    fn same_path_rebuild_verifies() {
        let matrix = fixture(48, 70);
        let index = reassociated(matrix.clone());
        let recorded = index.arithmetic().image_code();
        assert!(Reassociated::IMAGE_CODES.contains(&recorded));
        assert_eq!(
            recorded,
            Reassociated::resolve()
                .expect("the test thread runs the default float environment")
                .image_code(),
            "the image records the path the build resolved"
        );
        let image = index.canonical_image();
        assert_eq!(header_code(&image), recorded);

        assert!(index.verify_rebuild().expect("the same path rebuilds"));
        let decoded = HnswIndex::decode_reassociated(matrix.clone(), &image).expect("decodes");
        assert_eq!(decoded.canonical_image(), image);
        assert_eq!(
            decoded.search_rows(3, 4).expect("searches"),
            index.search_rows(3, 4).expect("searches")
        );
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &image).expect("verifies"),
            Some(params())
        );

        // The hook refusing a path other than the recorded one changes nothing, so the
        // refusal `other_path_refused_named` observes is the recorded path's and not the
        // hook's.
        let lacking = path_hook::lack(another_path(recorded), recorded);
        assert!(index.verify_rebuild().expect("the same path rebuilds"));
        assert!(HnswIndex::decode_reassociated(matrix, &image).is_ok());
        drop(lacking);
    }

    #[test]
    fn other_path_refused_named() {
        let matrix = fixture(48, 70);
        let index = reassociated(matrix.clone());
        let image = index.canonical_image();
        let recorded = index.arithmetic().image_code();
        let available = another_path(recorded);
        let refusal = HnswError::ArithmeticPathUnavailable {
            recorded,
            available,
        };

        let lacking = path_hook::lack(recorded, available);
        assert_eq!(
            HnswIndex::decode_reassociated(matrix.clone(), &image).expect_err("refused"),
            refusal
        );
        assert_eq!(
            index
                .verify_rebuild()
                .expect_err("refused by name, never `Ok(false)`"),
            refusal
        );
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &image)
                .expect_err("refused by name, never `Ok(None)`"),
            refusal
        );
        assert_eq!(index.search_rows(0, 3).expect_err("refused"), refusal);
        assert_eq!(index.search_rows_work(0, 3).expect_err("refused"), refusal);
        assert_eq!(
            index
                .search_vector(&matrix.row_to_vec(0), 3)
                .expect_err("refused"),
            refusal
        );
        assert_eq!(
            index.search_batch(&[0, 1], 3).expect_err("refused"),
            refusal
        );
        let message = refusal.to_string();
        assert!(
            message.contains(&profile::path_label(recorded))
                && message.contains(&profile::path_label(available)),
            "the refusal names both paths: {message}"
        );
        drop(lacking);

        // The neighbour: able to run the path it recorded, every one of those calls
        // answers.
        assert!(index.verify_rebuild().expect("rebuilds"));
        assert!(HnswIndex::decode_reassociated(matrix.clone(), &image).is_ok());
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &image).expect("verifies"),
            Some(params())
        );
        assert_eq!(index.search_rows(0, 3).expect("searches").len(), 3);
    }

    /// A reassociated index built on the path `code` names, which this host runs.
    fn reassociated_on(matrix: VectorMatrix, code: u32) -> HnswIndex<Reassociated> {
        let arithmetic =
            Reassociated::resolve_recorded(code).expect("the host runs the recorded path");
        let kernel = Kernel::SquaredEuclidean;
        let compiled = Compiled::here();
        let (graph, norms) =
            (compiled.build_graph)(&matrix, arithmetic, kernel, params(), None).expect("builds");
        HnswIndex::new(matrix, kernel, params(), graph, norms, arithmetic, compiled)
    }

    /// Every reassociated code this host runs, narrowest first, as the processor reports.
    fn host_codes() -> Vec<u32> {
        #[cfg(target_arch = "x86_64")]
        {
            let mut codes = vec![2];
            if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {
                codes.push(3);
                if std::is_x86_feature_detected!("avx512f") {
                    codes.push(4);
                }
            }
            codes
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            vec![
                Reassociated::resolve()
                    .expect("the test thread runs the default float environment")
                    .image_code(),
            ]
        }
    }

    /// An image recorded on a narrower path than the widest this host runs is decoded,
    /// searched and verified on that path, never refused and never recomputed on the
    /// widest. The oracle is the recorded path's own kernel: every offered distance equals
    /// it bit for bit, and where the widest path's bits differ on the fixture, the widest
    /// is shown to disagree -- so a run on the widest path could not pass. Where the two
    /// compilations agree on every pair, the resolved handle's path is what is observed.
    #[test]
    fn a_narrower_recorded_path_is_run_not_refused() {
        let codes = host_codes();
        let widest = Reassociated::resolve()
            .expect("the test thread runs the default float environment")
            .image_code();
        assert_eq!(
            codes.last(),
            Some(&widest),
            "the widest is the host's last path"
        );
        let mut exercised = Vec::new();
        for &code in &codes[..codes.len() - 1] {
            let matrix = fixture(48, 150);
            let built = reassociated_on(matrix.clone(), code);
            let recorded = built.arithmetic();
            assert_eq!(recorded.image_code(), code);
            let image = built.canonical_image();
            assert_eq!(
                header_code(&image),
                code,
                "the image records the narrower path"
            );

            let decoded = HnswIndex::decode_reassociated(matrix.clone(), &image)
                .expect("a host that runs the recorded path decodes it");
            assert_eq!(
                decoded.arithmetic(),
                recorded,
                "decoded onto the recorded path"
            );
            assert_eq!(
                decoded.canonical_image(),
                image,
                "re-encoded with its own code"
            );
            assert!(
                decoded
                    .verify_rebuild()
                    .expect("rebuilds on the recorded path")
            );
            assert!(
                built
                    .verify_rebuild()
                    .expect("rebuilds on the recorded path")
            );
            assert_eq!(
                HnswIndex::verify_bytes_against_reassociated(&matrix, &image)
                    .expect("verifies on the recorded path"),
                Some(params())
            );

            let wide = Reassociated::resolve_recorded(widest).expect("the widest path");
            let pair = |handle: Resolved<Reassociated>, a: usize, b: usize| {
                matrix
                    .distance(handle, decoded.kernel(), a, 0.0, b, 0.0)
                    .expect("finite")
            };
            let mut told_apart = 0_usize;
            for row in 0..matrix.rows() {
                let offered = decoded.search_rows(row, 8).expect("searches");
                assert_eq!(
                    decoded.search_batch(&[row], 8).expect("searches"),
                    vec![offered.clone()]
                );
                for candidate in &offered {
                    let on_recorded = pair(recorded, row, candidate.row);
                    assert_eq!(
                        candidate.distance.to_bits(),
                        on_recorded.to_bits(),
                        "code {code}: search ran another path for ({row}, {})",
                        candidate.row
                    );
                    if pair(wide, row, candidate.row).to_bits() != on_recorded.to_bits() {
                        told_apart += 1;
                    }
                }
            }
            let query = matrix.row_to_vec(0);
            for candidate in decoded.search_vector(&query, 8).expect("searches") {
                assert_eq!(
                    candidate.distance.to_bits(),
                    pair(recorded, 0, candidate.row).to_bits()
                );
            }
            let how = if told_apart > 0 {
                "told apart from the widest by its bits"
            } else {
                "agrees with the widest bitwise; path observed"
            };
            exercised.push(format!("{} ({how})", profile::path_label(code)));
        }
        // The cases this host could exercise, from its own report.
        #[cfg(target_arch = "x86_64")]
        if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {
            assert!(
                exercised.iter().any(|case| case.contains("along sse2")),
                "an AVX2+FMA host runs an SSE2 image: {exercised:?}"
            );
            if std::is_x86_feature_detected!("avx512f") {
                assert!(
                    exercised.iter().any(|case| case.contains("along avx2+fma")),
                    "an AVX-512F host runs an AVX2+FMA image: {exercised:?}"
                );
            }
        }
        if codes.len() == 1 {
            assert_eq!(
                exercised,
                Vec::<String>::new(),
                "no narrower path to exercise"
            );
            exercised.push(format!(
                "none: this host runs only {}",
                profile::path_label(widest)
            ));
        }
        println!("a_narrower_recorded_path_is_run_not_refused exercised: {exercised:?}");
    }

    /// The variable a CI job sets to name the dispatch paths its host must execute, as a
    /// comma-separated list of path names. Read by the test harness only.
    const REQUIRE_PATHS_VAR: &str = "PURRDF_REQUIRE_DISPATCH_PATHS";

    /// The paths [`REQUIRE_PATHS_VAR`] names, or none when it is unset. A name that is no
    /// path, or a variable that names none, panics: a misspelt requirement must fail
    /// rather than require nothing.
    fn required_paths() -> Vec<purrdf_core::distance::Path> {
        use purrdf_core::distance::Path;

        let Some(value) = std::env::var_os(REQUIRE_PATHS_VAR) else {
            return Vec::new();
        };
        let value = value
            .into_string()
            .unwrap_or_else(|raw| panic!("{REQUIRE_PATHS_VAR} is not UTF-8: {raw:?}"));
        let known = profile::PATHS.map(Path::name).join(", ");
        let mut paths = Vec::new();
        for name in value
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            let path = profile::PATHS
                .into_iter()
                .find(|path| path.name() == name)
                .unwrap_or_else(|| {
                    panic!(
                        "{REQUIRE_PATHS_VAR} names `{name}`, which is not a dispatch path; \
                         the paths are: {known}"
                    )
                });
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        assert!(
            !paths.is_empty(),
            "{REQUIRE_PATHS_VAR} is set but names no path ({value:?}); the paths are: {known}"
        );
        paths
    }

    /// An index is built, searched and verified on every reassociated path this host
    /// runs, each search answering with that path's own kernel bits; and every path
    /// [`REQUIRE_PATHS_VAR`] names that this build compiles the reassociated arithmetic
    /// for is among them, so a job that names a path proves the index ran on it.
    ///
    /// A required exact path is not this test's: every exact path computes the same bits,
    /// so the exact index runs the widest and `purrdf-core`'s tests execute each one. A
    /// required path neither arithmetic of this build compiles fails by name.
    #[test]
    fn an_index_runs_on_every_host_path_and_every_required_one() {
        use purrdf_core::distance::{Path, PathUnavailable};

        let mut executed = Vec::new();
        for code in host_codes() {
            let matrix = fixture(48, 70);
            let built = reassociated_on(matrix.clone(), code);
            let recorded = built.arithmetic();
            assert_eq!(recorded.image_code(), code, "built on the path it names");
            for row in 0..matrix.rows() {
                for candidate in built.search_rows(row, 4).expect("searches") {
                    let own = matrix
                        .distance(recorded, built.kernel(), row, 0.0, candidate.row, 0.0)
                        .expect("finite");
                    assert_eq!(
                        candidate.distance.to_bits(),
                        own.to_bits(),
                        "code {code}: search ran another path for ({row}, {})",
                        candidate.row
                    );
                }
            }
            assert!(built.verify_rebuild().expect("rebuilds on its own path"));
            executed.push(recorded.path());
        }
        // Every exact path computes the same bits, so an exact path this build compiles is
        // held by `purrdf-core`'s tests rather than by an index.
        let exact_here = |path: Path| {
            path == Path::Portable || (cfg!(target_arch = "x86_64") && path == Path::Avx2)
        };
        for path in required_paths() {
            let refusal = match Reassociated::image_code(path) {
                Some(code) => match Reassociated::resolve_recorded(code) {
                    Ok(_) => {
                        assert!(
                            executed.contains(&path),
                            "{REQUIRE_PATHS_VAR} requires {path}, which the host runs but no \
                             index was built on (executed: {executed:?})"
                        );
                        continue;
                    }
                    Err(refusal) => Some(refusal),
                },
                None => None,
            };
            let not_compiled = matches!(
                refusal,
                None | Some(RecordedPathError::Unavailable {
                    reason: PathUnavailable::NotCompiled,
                    ..
                })
            );
            assert!(
                not_compiled && exact_here(path),
                "{REQUIRE_PATHS_VAR} requires {path}, which a reassociated index on this {} \
                 build cannot run{}",
                std::env::consts::ARCH,
                refusal.map_or_else(String::new, |refusal| format!(": {refusal}"))
            );
        }
        println!("an_index_runs_on_every_host_path_and_every_required_one executed: {executed:?}");
    }

    /// An image recorded on a path this build holds no compilation of is refused by its
    /// real resolution, naming the widest path this process runs; the neighbour, the same
    /// image with the code this host built it under, decodes.
    #[test]
    fn a_path_this_build_cannot_run_is_refused() {
        let matrix = fixture(48, 70);
        let index = reassociated(matrix.clone());
        let widest = index.arithmetic().image_code();
        let codes = host_codes();
        let foreign = *Reassociated::IMAGE_CODES
            .iter()
            .find(|code| !codes.contains(code))
            .expect("every build lacks some target's compilation");
        #[cfg(target_arch = "x86_64")]
        assert_eq!(
            foreign, 5,
            "NEON is the first code an x86_64 build cannot run"
        );
        assert!(matches!(
            Reassociated::resolve_recorded(foreign),
            Err(RecordedPathError::Unavailable { .. })
        ));
        let mut image = index.canonical_image();
        image[60..64].copy_from_slice(&foreign.to_le_bytes());
        let refusal = HnswError::ArithmeticPathUnavailable {
            recorded: foreign,
            available: widest,
        };
        assert_eq!(
            HnswIndex::decode_reassociated(matrix.clone(), &image).expect_err("refused"),
            refusal
        );
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &image)
                .expect_err("refused by name, never `Ok(None)`"),
            refusal
        );
        let message = refusal.to_string();
        assert!(
            message.contains(&profile::path_label(foreign))
                && message.contains(&profile::path_label(widest)),
            "the refusal names both paths: {message}"
        );

        // The neighbour: the bytes as built decode and verify.
        let built = index.canonical_image();
        assert!(HnswIndex::decode_reassociated(matrix.clone(), &built).is_ok());
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &built).expect("verifies"),
            Some(params())
        );
    }

    #[test]
    fn a_portable_path_image_decodes_only_on_a_portable_path_host() {
        let portable = Reassociated::image_code(purrdf_core::distance::Path::Portable)
            .expect("the portable path is one of the reassociated arithmetic's");
        assert_eq!(portable, 8);
        let matrix = fixture(48, 70);
        let index = reassociated(matrix.clone());
        let here = index.arithmetic().image_code();
        let portable_target = cfg!(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "wasm32",
            target_arch = "wasm64"
        )));
        // A target with no named reassociated path records the portable code itself.
        assert_eq!(here == portable, portable_target);
        let mut image = index.canonical_image();
        image[60..64].copy_from_slice(&portable.to_le_bytes());
        assert_eq!(header_code(&image), portable);

        // This host answers as the paths it really runs: it decodes the image exactly
        // when its build holds the portable compilation, and otherwise refuses it naming
        // both paths.
        let decoded = HnswIndex::decode_reassociated(matrix.clone(), &image);
        if portable_target {
            let decoded = decoded.expect("a portable-path host decodes a portable-path image");
            assert_eq!(decoded.canonical_image(), image);
        } else {
            let refusal = HnswError::ArithmeticPathUnavailable {
                recorded: portable,
                available: here,
            };
            assert_eq!(decoded.expect_err("another target refuses"), refusal);
            assert_eq!(
                HnswIndex::verify_bytes_against_reassociated(&matrix, &image)
                    .expect_err("refused by name, never `Ok(None)`"),
                refusal
            );
            let message = refusal.to_string();
            assert!(
                message.contains("along portable") && message.contains(&profile::path_label(here)),
                "the refusal names both paths: {message}"
            );
        }

        // The exact decoder never reads the portable reassociated code as its own.
        assert!(matches!(
            HnswIndex::decode(matrix, &image),
            Err(HnswError::ArithmeticMismatch { actual: 8, .. })
        ));
    }

    /// The eight bytes after a reassociated header's code: the build shape.
    fn header_shape(image: &[u8]) -> BuildShape {
        BuildShape::from_bits(u64::from_le_bytes(
            image[64..72].try_into().expect("eight header bytes"),
        ))
    }

    /// A shape this build is not: this build's with one more feature of its row, or, on an
    /// architecture whose row records none, another architecture's.
    fn another_shape() -> BuildShape {
        let here = BuildShape::here();
        let arch = here.architecture().expect("every CI target is a named one");
        let mut features = here.features();
        let extra = [
            "fma",
            "avx512f",
            "sve",
            "neon",
            "vfp4",
            "simd128",
            "relaxed-simd",
            "v",
        ]
        .into_iter()
        .find(|feature| {
            !features.contains(feature) && {
                let mut with = features.clone();
                with.push(feature);
                BuildShape::encode(arch, &with) != here
            }
        });
        let other = extra.map_or_else(
            || BuildShape::encode(if arch == "x86_64" { "x86" } else { "x86_64" }, &[]),
            |feature| {
                features.push(feature);
                BuildShape::encode(arch, &features)
            },
        );
        assert_ne!(other, here);
        other
    }

    /// A reassociated image records this build's shape after its code, and a build of
    /// another shape refuses it by name on every path that reads it; the neighbour, the
    /// same bytes with this build's shape, decodes, verifies and rebuilds.
    #[test]
    fn a_foreign_build_shape_is_refused_named() {
        let matrix = fixture(48, 70);
        let index = reassociated(matrix.clone());
        let image = index.canonical_image();
        let here = BuildShape::here();
        assert_eq!(index.arithmetic().build_shape(), Some(here));
        assert_eq!(header_shape(&image), here, "the image records this build");

        let foreign = another_shape();
        let mut patched = image.clone();
        patched[64..72].copy_from_slice(&foreign.bits().to_le_bytes());
        let refusal = HnswError::ArithmeticBuildMismatch {
            recorded: foreign,
            here,
        };
        assert_eq!(
            HnswIndex::decode_reassociated(matrix.clone(), &patched).expect_err("refused"),
            refusal
        );
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &patched)
                .expect_err("refused by name, never `Ok(None)`"),
            refusal
        );
        let message = refusal.to_string();
        assert!(
            message.contains(&foreign.to_string()) && message.contains(&here.to_string()),
            "the refusal names both shapes: {message}"
        );
        for feature in foreign.features() {
            assert!(message.contains(feature), "{feature} in {message}");
        }

        // The neighbour: the same-build round trip.
        let decoded =
            HnswIndex::decode_reassociated(matrix.clone(), &image).expect("this build decodes");
        assert_eq!(decoded.canonical_image(), image);
        assert!(decoded.verify_rebuild().expect("rebuilds"));
        assert!(index.verify_rebuild().expect("rebuilds"));
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &image).expect("verifies"),
            Some(params())
        );
        assert_eq!(
            decoded.search_rows(3, 4).expect("searches"),
            index.search_rows(3, 4).expect("searches")
        );
    }

    /// The image with the last bit of node 0's first layer-0 distance flipped: a payload
    /// that decodes, and that no rebuild reproduces.
    fn with_a_perturbed_distance(image: &[u8], header: usize) -> Vec<u8> {
        // Node 0: row u64, level u32, reserved u32; layer 0: layer u32, reserved u32,
        // count u64; then its first neighbour's row u64 and distance u64.
        let count = header + 16 + 8;
        assert!(
            u64::from_le_bytes(image[count..count + 8].try_into().expect("eight bytes")) > 0,
            "node 0 has a layer-0 neighbour"
        );
        let distance = count + 8 + 8;
        let mut perturbed = image.to_vec();
        perturbed[distance] ^= 1;
        perturbed
    }

    /// A reassociated rebuild that produces another image under the recorded path and
    /// shape is refused by name, never `false`; the same perturbation of an exact image is
    /// the honest `false` it always was, and both unperturbed images verify.
    #[test]
    fn a_diverged_rebuild_is_named_for_reassociated_and_false_for_exact() {
        let matrix = fixture(48, 70);
        let fast = reassociated(matrix.clone());
        let image = fast.canonical_image();
        let perturbed = with_a_perturbed_distance(&image, 80);
        let refusal = HnswError::ArithmeticRebuildDiverged {
            recorded: fast.arithmetic().image_code(),
            shape: BuildShape::here(),
        };
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &perturbed)
                .expect_err("named, never `Ok(None)`"),
            refusal
        );
        let decoded = HnswIndex::decode_reassociated(matrix.clone(), &perturbed)
            .expect("a perturbed distance still decodes");
        assert_eq!(
            decoded.verify_rebuild().expect_err("named, never `false`"),
            refusal
        );
        let message = refusal.to_string();
        assert!(
            message.contains(&profile::path_label(fast.arithmetic().image_code()))
                && message.contains(&BuildShape::here().to_string())
                && message.contains("only by the compiled artifact that built it"),
            "{message}"
        );
        // The neighbour: unperturbed, it verifies.
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &image).expect("verifies"),
            Some(params())
        );
        assert!(fast.verify_rebuild().expect("rebuilds"));

        // The exact index keeps `false` as its tamper evidence.
        let exact = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        let exact_image = exact.canonical_image();
        let exact_perturbed = with_a_perturbed_distance(&exact_image, 72);
        assert_eq!(
            HnswIndex::verify_bytes_against(&matrix, &exact_perturbed).expect("answers"),
            None
        );
        let decoded = HnswIndex::decode(matrix.clone(), &exact_perturbed).expect("decodes");
        assert!(!decoded.verify_rebuild().expect("answers"));
        assert_eq!(
            HnswIndex::verify_bytes_against(&matrix, &exact_image).expect("verifies"),
            Some(params())
        );
    }

    /// The exact arithmetic records one code, and every host runs it on the path a new
    /// exact result runs on.
    #[test]
    fn an_exact_image_resolves_its_one_code() {
        let matrix = fixture(40, 20);
        let exact = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        let image = exact.canonical_image();
        assert_eq!(header_code(&image), Exact::IMAGE_CODE);
        assert_eq!(
            exact.arithmetic().build_shape(),
            None,
            "no shape is recorded"
        );
        let decoded = HnswIndex::decode(matrix, &image).expect("decodes");
        assert_eq!(
            decoded.arithmetic(),
            Exact::resolve().expect("the test thread runs the default float environment")
        );
        assert!(decoded.verify_rebuild().expect("rebuilds"));
        assert_eq!(decoded.canonical_image(), image);
    }

    #[test]
    fn decode_refuses_fast_code_as_exact() {
        let matrix = fixture(40, 20);
        let fast = reassociated(matrix.clone());
        let image = fast.canonical_image();
        let refusal = HnswError::ArithmeticMismatch {
            arithmetic: Exact::ID,
            actual: fast.arithmetic().image_code(),
        };
        assert_eq!(
            HnswIndex::decode(matrix.clone(), &image).expect_err("refused"),
            refusal
        );
        assert_eq!(
            HnswIndex::verify_bytes_against(&matrix, &image).expect_err("refused"),
            refusal
        );
        // The neighbour: the same bytes decode as the arithmetic that wrote them.
        assert!(HnswIndex::decode_reassociated(matrix, &image).is_ok());
    }

    #[test]
    fn decode_refuses_exact_code_as_fast() {
        let matrix = fixture(40, 20);
        let exact = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        let image = exact.canonical_image();
        assert_eq!(header_code(&image), Exact::IMAGE_CODE);
        let refusal = HnswError::ArithmeticMismatch {
            arithmetic: Reassociated::ID,
            actual: Exact::IMAGE_CODE,
        };
        assert_eq!(
            HnswIndex::decode_reassociated(matrix.clone(), &image).expect_err("refused"),
            refusal
        );
        assert_eq!(
            HnswIndex::verify_bytes_against_reassociated(&matrix, &image).expect_err("refused"),
            refusal
        );
        // The neighbour: the same bytes decode as the arithmetic that wrote them.
        assert!(HnswIndex::decode(matrix, &image).is_ok());
    }

    #[test]
    fn one_distance_per_pair_across_call_sites() {
        use purrdf_sparql_eval::knn::{Bound, Bounded};

        use crate::search::norm_of;

        let metrics = [
            DistanceMetric::SquaredEuclidean,
            DistanceMetric::NegativeDot,
            DistanceMetric::Cosine,
        ];
        // 150 components: two whole 64-element blocks and a tail, at both stored widths.
        for metric in &metrics {
            for matrix in [fixture(40, 150), fixture_f32(40, 150)] {
                let index = HnswIndex::build_reassociated(matrix.clone(), metric, params())
                    .expect("builds");
                let arithmetic = index.arithmetic();
                let kernel = index.kernel();
                let norms = &index.norms;
                let pair = |a: usize, b: usize| {
                    matrix
                        .distance(
                            arithmetic,
                            kernel,
                            a,
                            norm_of(norms, a),
                            b,
                            norm_of(norms, b),
                        )
                        .expect("finite")
                };
                let mut edges = 0_usize;
                for row in 0..matrix.rows() {
                    for layer in 0..=index.level(row) {
                        for neighbor in index.neighbors(row, layer) {
                            edges += 1;
                            let expected = pair(row, neighbor.row);
                            // Build: the distance the graph recorded for the edge.
                            assert_eq!(
                                neighbor.distance.to_bits(),
                                expected.to_bits(),
                                "{kernel:?}: build recorded another value for ({row}, {})",
                                neighbor.row
                            );
                            assert_eq!(
                                expected.to_bits(),
                                pair(neighbor.row, row).to_bits(),
                                "{kernel:?}: the pair is not symmetric"
                            );
                            // Select: the bounded form, in full and at its tightest bound.
                            for bound in [
                                Bound::Above(f64::INFINITY),
                                Bound::AtOrAbove(expected.next_up()),
                            ] {
                                match matrix.distance_bounded(
                                    arithmetic,
                                    kernel,
                                    row,
                                    norm_of(norms, row),
                                    neighbor.row,
                                    norm_of(norms, neighbor.row),
                                    bound,
                                ) {
                                    Bounded::Below(value) => assert_eq!(
                                        value.to_bits(),
                                        expected.to_bits(),
                                        "{kernel:?}: selection computed another value"
                                    ),
                                    other => panic!("{kernel:?}: {other:?} under {bound:?}"),
                                }
                            }
                        }
                    }
                }
                assert!(edges > 0, "the graph holds edges to compare");
                // Search: every offered distance, from the stored-row and the batch path.
                for row in 0..matrix.rows() {
                    for offered in index.search_rows(row, 10).expect("searches") {
                        assert_eq!(
                            offered.distance.to_bits(),
                            pair(row, offered.row).to_bits(),
                            "{kernel:?}: search computed another value for ({row}, {})",
                            offered.row
                        );
                    }
                }
                // An external query is always binary64, so over a binary64 matrix it is
                // the same pair as the stored row and must score the same bits.
                if let Some(query) = matrix.row_f64(0) {
                    let query_norm = if kernel.needs_norms() {
                        arithmetic.exact().norm(query)
                    } else {
                        0.0
                    };
                    for offered in index.search_vector(query, 10).expect("searches") {
                        let expected = kernel
                            .distance_reassociated(
                                arithmetic,
                                query,
                                query_norm,
                                matrix.row(offered.row),
                                norm_of(norms, offered.row),
                            )
                            .expect("finite");
                        assert_eq!(offered.distance.to_bits(), expected.to_bits());
                        assert_eq!(offered.distance.to_bits(), pair(0, offered.row).to_bits());
                    }
                }
            }
        }
    }

    #[test]
    fn loss_contract_unchanged_for_both_arithmetics() {
        use purrdf_core::IndexUseRole;

        let exact = HnswIndex::build(fixture(40, 8), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds");
        let fast = reassociated(fixture(40, 8));
        let exact_contract = guard::guard_contract_for(&exact, IndexUseRole::Generic);
        let fast_contract = guard::guard_contract_for(&fast, IndexUseRole::Generic);
        for contract in [&exact_contract, &fast_contract] {
            assert_eq!(contract.loss, profile::loss_contract());
            assert!(!contract.loss.transforms_vectors);
            assert!(contract.loss.loss_encoding.is_none());
            assert!(contract.loss.loss_parameters.is_none());
        }
        assert_eq!(
            exact_contract,
            guard::guard_contract(params(), IndexUseRole::Generic),
            "the exact index's guard is the one it always was"
        );
        // The control: the two guards do differ, in the implementation that records the
        // arithmetic, so the equal loss contracts above are two contracts agreeing and not
        // one contract read twice.
        assert_ne!(exact_contract.implementation, fast_contract.implementation);
        assert_eq!(exact_contract.implementation.identifier, IMPLEMENTATION_ID);
        assert_eq!(
            fast_contract.implementation.identifier,
            IMPLEMENTATION_ID_REASSOCIATED
        );
        assert_eq!(
            fast_contract.implementation.revision.as_deref(),
            Some(profile::loss_evidence_reassociated(fast.arithmetic().path()).as_bytes())
        );
    }

    #[test]
    fn an_extension_metric_is_refused() {
        let error = HnswIndex::build(
            fixture(8, 2),
            &DistanceMetric::Extension {
                identifier: "https://example.org/metric".to_owned(),
                parameter_encoding: "application/cbor".to_owned(),
                parameters: vec![1, 2, 3],
            },
            params(),
        )
        .expect_err("no kernel");
        assert!(matches!(error, HnswError::UnsupportedMetric { .. }));
    }
}
