// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! * **Distances** come from the exact path's kernels via
//!   [`purrdf_sparql_eval::knn`], and candidates are ordered by the shared [`Ranked`] type
//!   `(distance, row)` — there is no second comparator to drift out of step.
//!
//! The result is an index whose **canonical byte image** is byte-identical across thread
//! counts and across `wasm32-unknown-unknown`. `rayon` runs inline-sequentially on wasm,
//! so the wasm build is slower but not different.
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
//! ```ignore
//! use purrdf_hnsw::{build, VectorMatrix, Params};
//! use purrdf_core::DistanceMetric;
//!
//! let matrix = VectorMatrix::from_rows(&[
//!     vec![1.0, 0.0],
//!     vec![0.0, 1.0],
//!     vec![0.9, 0.1],
//! ])?;
//! let params = Params::new(2, 4, 8, 4)?;
//! let index = build(matrix, &DistanceMetric::SquaredEuclidean, params)?;
//! let nearest = index.search_rows(0, 2)?;
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

use purrdf_core::DistanceMetric;
use rayon::prelude::*;

use crate::graph::{Graph, decode_image};
use crate::search::{DistanceCache, Query, Visited, greedy_descend, search_layer};

/// The canonical image format version.
pub const INDEX_VERSION: u32 = 1;

/// The media type of an HNSW index payload.
pub const INDEX_MEDIA_TYPE: &str = "application/vnd.blackcatinformatics.purrdf.hnsw";

/// The derived-index implementation identifier.
pub const IMPLEMENTATION_ID: &str = "hnsw-v1";

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

/// A built HNSW index: the graph, the matrix it was built over, and its identity.
///
/// The index owns its matrix. Search is therefore self-contained (no external matrix to
/// pass back in) and the vectors cannot drift out from under the graph they describe.
#[derive(Debug)]
pub struct HnswIndex {
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
}

impl HnswIndex {
    /// Assemble an index from its parts.
    pub(crate) fn new(
        matrix: VectorMatrix,
        kernel: Kernel,
        params: Params,
        graph: Graph,
        norms: Vec<f64>,
    ) -> Self {
        Self {
            matrix,
            kernel,
            params,
            graph,
            norms,
        }
    }

    /// Build an index over `matrix` under `metric`.
    ///
    /// # Errors
    ///
    /// [`HnswError::UnsupportedMetric`] if `metric` is a caller-defined extension metric
    /// with no kernel this crate can evaluate.
    /// Otherwise the errors of the builder: the parameter validation matrix, a zero-norm
    /// row under a norm-dividing kernel, or a non-finite distance.
    pub fn build(matrix: VectorMatrix, metric: &DistanceMetric, params: Params) -> Result<Self> {
        let kernel = Kernel::of(metric).ok_or_else(|| HnswError::UnsupportedMetric {
            metric: format!("{metric:?}"),
        })?;
        builder::build(matrix, kernel, params)
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
    pub fn search_rows(&self, query_row: usize, k: usize) -> Result<Vec<Ranked>> {
        let cache = DistanceCache::new();
        let mut visited = Visited::new(self.matrix.rows());
        self.search_with(query_row, k, &cache, &mut visited)
    }

    /// The `k` nearest rows to `query_row`, together with the number of candidate
    /// distance evaluations the search performed.
    ///
    /// This is [`HnswIndex::search_rows`] with the work it cost made observable: one unit
    /// per `Kernel::distance` call, counted through the search's memo, so a candidate whose
    /// distance was already computed is not charged twice. The count is what a
    /// property-function cursor reports through `PfCursor::take_work` — the rows a search
    /// returns are `k`, and `k` says nothing about the size of the graph they were selected
    /// from.
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::search_rows`].
    pub fn search_rows_work(&self, query_row: usize, k: usize) -> Result<(Vec<Ranked>, u64)> {
        let cache = DistanceCache::new();
        let mut visited = Visited::new(self.matrix.rows());
        let ranked = self.search_with(query_row, k, &cache, &mut visited)?;
        Ok((ranked, cache.evaluations()))
    }

    /// The `k` nearest rows to each of `query_rows`, one result per query in input order.
    ///
    /// The order is the input order, not completion order: the batch is mapped by rayon's
    /// indexed adaptor, so the answers line up with the questions even when workers
    /// finish out of order. Per-worker scratch is reused across the batch.
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::search_rows`], for the first failing query.
    pub fn search_batch(&self, query_rows: &[usize], k: usize) -> Result<Vec<Vec<Ranked>>> {
        let rows = self.matrix.rows();
        query_rows
            .par_iter()
            .map_init(
                || (DistanceCache::new(), Visited::new(rows)),
                |(cache, visited), &query_row| self.search_with(query_row, k, cache, visited),
            )
            .collect()
    }

    /// One search against caller-supplied scratch.
    fn search_with(
        &self,
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
        let ef = self.params.ef_search().min(rows);
        let query = Query::new(&self.matrix, self.kernel, &self.norms, cache, query_row);
        // Greedy descent through the layers above 0, then the beam at layer 0.
        let (start, _) = greedy_descend(&self.graph, &query, entry, self.graph.max_level(), 1)?;
        let mut result = search_layer(&self.graph, &query, visited, &[start], 0, ef)?;
        result.truncate(k);
        Ok(result)
    }

    /// The index's canonical byte image.
    ///
    /// This is the image a PURREMB guard commits and the byte string a determinism digest
    /// folds: two builds that agree here are the same index.
    #[must_use]
    pub fn canonical_image(&self) -> Vec<u8> {
        self.graph.canonical_image(self.kernel, &self.params)
    }

    /// Decode an index from its canonical image, over the matrix it describes.
    ///
    /// The matrix is supplied rather than embedded: the image is the graph and its
    /// identity, while the vectors are the source data the graph was derived from. The
    /// decoded graph must have one node per matrix row.
    ///
    /// # Errors
    ///
    /// [`HnswError::InvalidPayload`] for malformed or structurally invalid bytes,
    /// [`HnswError::VersionMismatch`] for an unimplemented version,
    /// [`HnswError::ParameterValidation`] / [`HnswError::ArithmeticOverflow`] if the
    /// decoded parameters do not describe the matrix, and [`HnswError::ZeroNorm`] if the
    /// decoded kernel needs norms and a row has none.
    pub fn decode(matrix: VectorMatrix, bytes: &[u8]) -> Result<Self> {
        let image = decode_image(bytes)?;
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
        let norms = builder::compute_norms(&matrix, image.kernel)?;
        Ok(Self::new(
            matrix,
            image.kernel,
            image.params,
            image.graph,
            norms,
        ))
    }

    /// Rebuild the index from its own matrix and identity, and report whether the result
    /// is the same index.
    ///
    /// This turns "the payload is rebuildable" into a checkable claim: a `true` means the
    /// canonical image is a pure function of the source data and the declared parameters,
    /// while a `false` would mean the stored graph disagrees with what the declared
    /// identity builds.
    ///
    /// # Errors
    ///
    /// As [`HnswIndex::build`].
    pub fn verify_rebuild(&self) -> Result<bool> {
        let rebuilt = builder::build(self.matrix.clone(), self.kernel, self.params)?;
        Ok(rebuilt.canonical_image() == self.canonical_image())
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
