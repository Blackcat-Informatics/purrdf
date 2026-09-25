// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The graph: its in-memory shape, its canonical byte image, and the entry-point rule.
//!
//! # The two representations, and why both are canonical
//!
//! In memory a node's neighbours are held **sorted by `(distance, row)`**, the rank order
//! the shared [`Ranked`] type defines. Truncation to the degree bound and the invariant
//! tests both depend on that order being the stored order, not a re-sort performed at
//! read time.
//!
//! The canonical byte image instead stores each layer's adjacency **sorted by neighbour
//! row**. It is the image that is hashed for cross-target determinism and committed by a
//! PURREMB guard, and a row-ordered image is the one that survives a change of distance
//! kernel without renumbering (the row set is the identity; the distances are derived).
//! Decoding re-sorts by rank before the in-memory graph is handed back, so the two
//! representations are two views of one graph rather than two graphs.
//!
//! # The entry-point rule, pinned
//!
//! The entry point is **the minimum row index among the nodes at the maximum level**.
//! Standard HNSW leaves the tie-break to insertion order, which is exactly the kind of
//! hidden state determinism cannot tolerate; pinning it to the row index makes the entry
//! point a function of the level assignment alone.
//!
//! Because levels are themselves a pure function of the row index, the entry point is
//! known **before any link exists** and the builder fixes it before its first round. That
//! ordering is load-bearing rather than incidental: a builder that discovered its entry
//! point as it went would have to start from an empty snapshot, and every node proposed
//! against an empty snapshot links to nothing and can never be reached afterwards.

use std::collections::BTreeSet;

use purrdf_core::distance::{
    Arithmetic, BuildIdentity, BuildShape, Exact, Resolved, RowsRef, Selected,
};

use crate::error::{HnswError, Result};
use crate::params::Params;
use purrdf_sparql_eval::knn::{Bound, Bounded, Kernel, Ranked};

/// The canonical image's magic marker; identifies the format before any length is trusted.
pub(crate) const IMAGE_MAGIC: [u8; 8] = *b"PURHNSW1";

/// The canonical image's format version.
///
/// Version 2 is the first whose distances are folded by a named arithmetic, and the
/// first whose header records that arithmetic's image code (in the `u32` that version 1
/// reserved as zero): `1` for the sixteen-lane exact arithmetic, and for the
/// reassociated one the code of the dispatch path the build ran, followed by the build's
/// [`BuildShape`]: its bits, then its identity's digest. A version-1 image's distances were folded sequentially, so its recorded
/// bits are not the ones this build computes; it is refused with
/// [`HnswError::VersionMismatch`] rather than decoded.
pub(crate) const IMAGE_VERSION: u32 = 2;

/// What an image header records about the arithmetic its distances were computed under:
/// the image code, and for an arithmetic whose bits depend on the build, the build's shape.
///
/// The shape is present exactly when the arithmetic has one ([`Arithmetic::build_shape`]),
/// so an exact image carries none and its bytes are the ones it always had.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Recorded {
    /// The image code of the arithmetic and dispatch path.
    pub code: u32,
    /// The build shape, for an arithmetic whose bits depend on the build.
    pub shape: Option<BuildShape>,
}

impl Recorded {
    /// What an image computed on `arithmetic`'s path in this build records.
    pub(crate) fn of<A: Arithmetic>(arithmetic: Selected<A>) -> Self {
        Self {
            code: arithmetic.image_code(),
            shape: A::build_shape(),
        }
    }
}

/// A deterministic, finite-valued, row-major matrix of `f64` vectors.
///
/// The index consumes vectors through this type rather than a slice so that the shape is
/// carried with the values and can be validated once. Construction rejects a non-finite
/// component: the distance kernels report a non-finite *result* as an error, but a
/// non-finite *input* would poison every candidate it touched, and a matrix that cannot
/// be ranked should not exist.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorMatrix {
    rows: usize,
    dims: usize,
    data: Vectors,
}

/// A matrix's components, at the width PURREMB stored them.
///
/// PURREMB defines both `binary32` and `binary64` matrices (§12), and an embedding corpus is
/// usually the former. Widening one to `f64` at load is EXACT, so it changes no arithmetic
/// -- and costs exactly twice the resident memory for that privilege. At a million rows of
/// 4,096 components that is sixteen gigabytes spent on nothing, and on `wasm32`, whose
/// address space stops at four gigabytes, it is the difference between a corpus loading and
/// being refused.
///
/// So the width is kept and the widening moved into the distance fold, where it is free. The
/// answer is bit-identical either way -- `purrdf_sparql_eval::knn::Scalar` and its tests pin
/// that -- which is what makes this a memory decision rather than a numerical one.
///
/// There is deliberately no narrowing constructor. `f64` to `f32` loses bits, so a genuine
/// `binary64` artifact stays `binary64`.
#[derive(Debug, Clone, PartialEq)]
enum Vectors {
    /// `binary32`, as PURREMB's `f32_row` yields it.
    F32(Vec<f32>),
    /// `binary64`.
    F64(Vec<f64>),
}

impl Vectors {
    fn len(&self) -> usize {
        match self {
            Self::F32(data) => data.len(),
            Self::F64(data) => data.len(),
        }
    }

    /// Whether every component is finite. A non-finite input would poison every candidate it
    /// touched, so a matrix that cannot be ranked should not exist.
    fn first_non_finite(&self) -> Option<usize> {
        match self {
            Self::F32(data) => data.iter().position(|value| !value.is_finite()),
            Self::F64(data) => data.iter().position(|value| !value.is_finite()),
        }
    }
}

impl VectorMatrix {
    /// Assemble a matrix from a flat row-major buffer.
    ///
    /// # Errors
    ///
    /// [`HnswError::ParameterValidation`] if `rows` or `dims` is zero, or if `data` does
    /// not hold exactly `rows * dims` values.
    /// [`HnswError::ArithmeticOverflow`] if that product does not fit `usize`.
    /// [`HnswError::NonFiniteComponent`] if any component is not finite.
    pub fn new(rows: usize, dims: usize, data: Vec<f64>) -> Result<Self> {
        Self::assemble(rows, dims, Vectors::F64(data))
    }

    /// Assemble a matrix from a flat row-major `binary32` buffer.
    ///
    /// The width is kept rather than widened, which halves what the corpus costs resident.
    /// Every distance is still computed in binary64, in the same order, with the same
    /// separate roundings.
    ///
    /// # Errors
    ///
    /// As [`VectorMatrix::new`].
    pub fn from_f32(rows: usize, dims: usize, data: Vec<f32>) -> Result<Self> {
        Self::assemble(rows, dims, Vectors::F32(data))
    }

    fn assemble(rows: usize, dims: usize, data: Vectors) -> Result<Self> {
        if rows == 0 || dims == 0 {
            return Err(HnswError::ParameterValidation {
                description: format!("a {rows}x{dims} matrix has an empty axis"),
            });
        }
        let expected = rows
            .checked_mul(dims)
            .ok_or(HnswError::ArithmeticOverflow)?;
        if data.len() != expected {
            return Err(HnswError::ParameterValidation {
                description: format!(
                    "a {rows}x{dims} matrix holds {expected} values, but {} were supplied",
                    data.len()
                ),
            });
        }
        if let Some(index) = data.first_non_finite() {
            return Err(HnswError::NonFiniteComponent {
                row: index / dims,
                column: index % dims,
            });
        }
        Ok(Self { rows, dims, data })
    }

    /// Assemble a matrix from equally-sized rows.
    ///
    /// # Errors
    ///
    /// As [`VectorMatrix::new`], plus [`HnswError::ParameterValidation`] if the rows are
    /// ragged.
    pub fn from_rows(rows: &[Vec<f64>]) -> Result<Self> {
        let dims = rows.first().map_or(0, Vec::len);
        for (index, row) in rows.iter().enumerate() {
            if row.len() != dims {
                return Err(HnswError::ParameterValidation {
                    description: format!(
                        "row {index} holds {} values while row 0 holds {dims}; the matrix \
                         would be ragged",
                        row.len()
                    ),
                });
            }
        }
        let data: Vec<f64> = rows.iter().flatten().copied().collect();
        Self::new(rows.len(), dims, data)
    }

    /// The number of rows.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// The number of components per row.
    #[must_use]
    pub const fn dims(&self) -> usize {
        self.dims
    }

    /// Row `row`'s components, if this matrix stores `binary64`.
    ///
    /// Returns `None` for a `binary32` matrix, where no `&[f64]` exists to borrow. A caller
    /// that wants a distance should ask for the distance -- [`VectorMatrix::distance`] and
    /// its siblings work at either width and allocate nothing.
    ///
    /// # Panics
    ///
    /// Panics if `row` is not a valid row index; callers validate the index first.
    #[must_use]
    pub fn row_f64(&self, row: usize) -> Option<&[f64]> {
        let start = row * self.dims;
        match &self.data {
            Vectors::F64(data) => Some(&data[start..start + self.dims]),
            Vectors::F32(_) => None,
        }
    }

    /// Row `row`'s components, for a caller that knows this matrix stores `binary64`.
    ///
    /// A convenience for code that built the matrix itself and therefore knows its width --
    /// fixtures, generators, harnesses. Nothing on a search path should use it: a production
    /// caller does not know an artifact's width and should ask for a distance instead, which
    /// [`VectorMatrix::distance`] and its siblings answer at either width.
    ///
    /// # Panics
    ///
    /// Panics if `row` is not a valid row index, or if this matrix stores `binary32` -- in
    /// which case no `&[f64]` exists to return and the caller's assumption was wrong.
    #[must_use]
    pub fn row(&self, row: usize) -> &[f64] {
        self.row_f64(row).expect(
            "this matrix stores binary32, so there is no `&[f64]` to borrow; ask for a \
             distance, or use `row_to_vec`",
        )
    }

    /// The whole buffer, for a caller that knows this matrix stores `binary64`.
    ///
    /// # Panics
    ///
    /// Panics if this matrix stores `binary32`.
    #[must_use]
    pub fn as_slice(&self) -> &[f64] {
        match &self.data {
            Vectors::F64(data) => data,
            Vectors::F32(_) => {
                panic!("this matrix stores binary32; `as_slice` has no `&[f64]` to return")
            }
        }
    }

    /// Row `row`'s components widened into an owned buffer.
    ///
    /// Allocates, and is for inspection rather than for ranking: nothing on a search path
    /// should call it.
    ///
    /// # Panics
    ///
    /// Panics if `row` is not a valid row index.
    #[must_use]
    pub fn row_to_vec(&self, row: usize) -> Vec<f64> {
        let start = row * self.dims;
        match &self.data {
            Vectors::F64(data) => data[start..start + self.dims].to_vec(),
            Vectors::F32(data) => data[start..start + self.dims]
                .iter()
                .copied()
                .map(f64::from)
                .collect(),
        }
    }

    /// The L2 norm of row `row`, at whatever width it is stored, by PURREMB's normative
    /// fold ([`Resolved::<Exact>::norm`]).
    ///
    /// `arithmetic` is the exact handle, for the reason every distance here takes one: the
    /// fold is binary64 arithmetic, and on a thread that flushes subnormals it would
    /// return different bits for a row whose squared ratios are subnormal, which a cosine
    /// kernel then divides by. Such a thread cannot obtain the handle. An index running
    /// under another arithmetic obtains it with [`Resolved::exact`]: the norm has one
    /// written order, whatever arithmetic ranks the distances.
    ///
    /// # Panics
    ///
    /// Panics if `row` is not a valid row index.
    #[must_use]
    pub fn norm_of_row(&self, arithmetic: Resolved<Exact>, row: usize) -> f64 {
        let start = row * self.dims;
        match &self.data {
            Vectors::F64(data) => arithmetic.norm(&data[start..start + self.dims]),
            Vectors::F32(data) => arithmetic.norm(&data[start..start + self.dims]),
        }
    }
}

impl VectorMatrix {
    /// The distance between two stored rows under `kernel`, along `arithmetic`'s
    /// resolved dispatch path, or `None` if it left the finite range.
    ///
    /// `arithmetic` comes from [`Arithmetic::resolve`] (or
    /// [`Arithmetic::resolve_recorded`]), called once per scan or call site on the thread
    /// that computes. That call is where a thread that flushes subnormals or rounds other
    /// than to nearest is refused, by name; every distance this type computes takes the
    /// handle it returns, so none is computed on a thread that was not checked. The handle
    /// is `Copy`, and passing it costs nothing per pair.
    #[must_use]
    pub fn distance<A: Arithmetic>(
        &self,
        arithmetic: Resolved<A>,
        kernel: Kernel,
        a: usize,
        a_norm: f64,
        b: usize,
        b_norm: f64,
    ) -> Option<f64> {
        let (a_start, b_start) = (a * self.dims, b * self.dims);
        let measure = kernel.measure();
        match &self.data {
            Vectors::F64(data) => arithmetic.distance(
                measure,
                &data[a_start..a_start + self.dims],
                a_norm,
                &data[b_start..b_start + self.dims],
                b_norm,
            ),
            Vectors::F32(data) => arithmetic.distance(
                measure,
                &data[a_start..a_start + self.dims],
                a_norm,
                &data[b_start..b_start + self.dims],
                b_norm,
            ),
        }
    }

    /// The distance from an external `binary64` query to stored row `row`, along
    /// `arithmetic`'s resolved dispatch path.
    ///
    /// The query is `f64` because it was computed rather than stored -- an embedding produced
    /// at query time has no artifact width. A mixed-width pair is the ordinary case, and under
    /// an arithmetic whose bits do not depend on the path it is bit-identical to a matched one.
    /// `arithmetic` is the checked handle [`VectorMatrix::distance`] takes, for the same
    /// reason.
    #[must_use]
    pub fn distance_from_query<A: Arithmetic>(
        &self,
        arithmetic: Resolved<A>,
        kernel: Kernel,
        query: &[f64],
        query_norm: f64,
        row: usize,
        row_norm: f64,
    ) -> Option<f64> {
        let start = row * self.dims;
        let measure = kernel.measure();
        match &self.data {
            Vectors::F64(data) => arithmetic.distance(
                measure,
                query,
                query_norm,
                &data[start..start + self.dims],
                row_norm,
            ),
            Vectors::F32(data) => arithmetic.distance(
                measure,
                query,
                query_norm,
                &data[start..start + self.dims],
                row_norm,
            ),
        }
    }

    /// [`VectorMatrix::distance`], permitted to stop once it cannot clear `bound`.
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "the resolved arithmetic, the kernel, both endpoints with their norms and \
                  the bound are each an independent input of one distance"
    )]
    pub fn distance_bounded<A: Arithmetic>(
        &self,
        arithmetic: Resolved<A>,
        kernel: Kernel,
        a: usize,
        a_norm: f64,
        b: usize,
        b_norm: f64,
        bound: Bound,
    ) -> Bounded {
        let (a_start, b_start) = (a * self.dims, b * self.dims);
        let measure = kernel.measure();
        match &self.data {
            Vectors::F64(data) => arithmetic.distance_bounded(
                measure,
                &data[a_start..a_start + self.dims],
                a_norm,
                &data[b_start..b_start + self.dims],
                b_norm,
                bound,
            ),
            Vectors::F32(data) => arithmetic.distance_bounded(
                measure,
                &data[a_start..a_start + self.dims],
                a_norm,
                &data[b_start..b_start + self.dims],
                b_norm,
                bound,
            ),
        }
    }

    /// The distances from stored row `seed` to each row of `ids`, in `ids` order, by one
    /// call to the batch kernel.
    ///
    /// `norms` is the index's per-row norm table: empty for a kernel that does not divide
    /// by one, one per row otherwise.
    #[allow(
        clippy::too_many_arguments,
        reason = "the resolved arithmetic, the kernel, the seed with its norm, the norm \
                  table, the ids and the output are each an independent input of one batch"
    )]
    pub(crate) fn distances_from_row<A: Arithmetic>(
        &self,
        arithmetic: Resolved<A>,
        kernel: Kernel,
        seed: usize,
        seed_norm: f64,
        norms: &[f64],
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        let start = seed * self.dims;
        let measure = kernel.measure();
        match &self.data {
            Vectors::F64(data) => arithmetic.distances_indexed(
                measure,
                &data[start..start + self.dims],
                seed_norm,
                self.rows_ref(data, norms),
                ids,
                out,
            ),
            Vectors::F32(data) => arithmetic.distances_indexed(
                measure,
                &data[start..start + self.dims],
                seed_norm,
                self.rows_ref(data, norms),
                ids,
                out,
            ),
        }
    }

    /// The distances from an external `binary64` query to each row of `ids`, in `ids`
    /// order, by one call to the batch kernel.
    #[allow(
        clippy::too_many_arguments,
        reason = "the resolved arithmetic, the kernel, the query with its norm, the norm \
                  table, the ids and the output are each an independent input of one batch"
    )]
    pub(crate) fn distances_from_query<A: Arithmetic>(
        &self,
        arithmetic: Resolved<A>,
        kernel: Kernel,
        query: &[f64],
        query_norm: f64,
        norms: &[f64],
        ids: &[usize],
        out: &mut [Option<f64>],
    ) {
        let measure = kernel.measure();
        match &self.data {
            Vectors::F64(data) => arithmetic.distances_indexed(
                measure,
                query,
                query_norm,
                self.rows_ref(data, norms),
                ids,
                out,
            ),
            Vectors::F32(data) => arithmetic.distances_indexed(
                measure,
                query,
                query_norm,
                self.rows_ref(data, norms),
                ids,
                out,
            ),
        }
    }

    /// This matrix's buffer as the batch kernels' row view.
    fn rows_ref<'a, T: purrdf_core::distance::Scalar>(
        &self,
        data: &'a [T],
        norms: &'a [f64],
    ) -> RowsRef<'a, T> {
        RowsRef::new(data, self.rows, self.dims, norms).expect(
            "a validated matrix holds rows * dims values, and its norm table is empty or \
             one per row",
        )
    }
}

/// One directed adjacency update: `node` gains `neighbor` at `layer`.
///
/// The build emits both directions of every proposed link, so a commit applies them by
/// grouping on `(node, layer)`, merging with the frozen adjacency, and truncating.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Edge {
    /// The node whose adjacency this touches.
    pub node: usize,
    /// The layer the edge lives at.
    pub layer: u32,
    /// The neighbour and its distance.
    pub neighbor: Ranked,
}

/// The graph structure: per-node level, per-layer sorted adjacency, and the entry point.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Graph {
    levels: Vec<u32>,
    /// `layers[row][layer]`, sorted by [`Ranked`].
    layers: Vec<Vec<Vec<Ranked>>>,
    entry: Option<usize>,
    max_level: u32,
}

impl Graph {
    /// An empty graph over `levels.len()` nodes, each with its layers allocated.
    pub(crate) fn with_levels(levels: Vec<u32>) -> Self {
        let layers = levels
            .iter()
            .map(|&level| vec![Vec::new(); level as usize + 1])
            .collect();
        Self {
            levels,
            layers,
            entry: None,
            max_level: 0,
        }
    }

    /// How many nodes the graph holds.
    pub(crate) fn node_count(&self) -> usize {
        self.levels.len()
    }

    /// The level assigned to `row`.
    pub(crate) fn level(&self, row: usize) -> u32 {
        self.levels[row]
    }

    /// `row`'s neighbours at `layer`, in rank order.
    ///
    /// # Panics
    ///
    /// Panics if `row` is invalid or `layer` exceeds `row`'s level. Both are programming
    /// errors; the build and search only ever ask for layers a node occupies.
    pub(crate) fn neighbors(&self, row: usize, layer: u32) -> &[Ranked] {
        &self.layers[row][layer as usize]
    }

    /// The entry point, if any node is committed.
    pub(crate) fn entry(&self) -> Option<usize> {
        self.entry
    }

    /// The maximum level among committed nodes.
    pub(crate) fn max_level(&self) -> u32 {
        self.max_level
    }

    /// Install `row` as the entry point at `level`.
    ///
    /// Called once, before the first round, with the minimum row at the maximum level —
    /// both computable from the level assignment alone, as the module docs require.
    pub(crate) fn promote_entry(&mut self, row: usize, level: u32) {
        self.entry = Some(row);
        self.max_level = level;
    }

    /// Apply one round's proposal edges.
    ///
    /// Each node's new neighbourhood is `frozen neighbours ∪ inbound proposals`, sorted by
    /// `(distance, row)` and truncated to the per-layer degree bound. Because the merge is
    /// a set union followed by a total sort, the result does not depend on the order the
    /// proposals were produced in — which is what makes a parallel proposal phase
    /// byte-identical to a serial one.
    ///
    /// # Panics
    ///
    /// Panics only on an internally inconsistent edge (a node or layer the graph does not
    /// hold); the builder produces them from the graph itself.
    pub(crate) fn commit(&mut self, mut edges: Vec<Edge>, params: &Params) {
        edges.sort_unstable_by_key(|edge| (edge.node, edge.layer));
        let mut start = 0;
        while start < edges.len() {
            let (node, layer) = (edges[start].node, edges[start].layer);
            let mut end = start;
            while end < edges.len() && edges[end].node == node && edges[end].layer == layer {
                end += 1;
            }
            // Move the frozen adjacency out rather than cloning it: the merge writes the
            // result straight back into this slot, so the clone was a copy of a buffer that
            // was about to be overwritten, and its capacity is reused by the extend below.
            let mut merged = core::mem::take(&mut self.layers[node][layer as usize]);
            merged.extend(edges[start..end].iter().map(|edge| edge.neighbor));
            merged.sort_unstable();
            merged.dedup();
            merged.truncate(params.degree_bound(layer));
            self.layers[node][layer as usize] = merged;
            start = end;
        }
    }

    /// The rows a walk from `entry` can arrive at, following adjacency at `layer`.
    ///
    /// Search reaches a node only by following an *inbound* edge from a node it has already
    /// reached, so this is exactly the set of rows the beam can ever return.
    pub(crate) fn reachable_from(&self, entry: usize, layer: u32) -> Vec<bool> {
        let mut seen = vec![false; self.node_count()];
        seen[entry] = true;
        let mut frontier = vec![entry];
        while let Some(row) = frontier.pop() {
            for neighbor in &self.layers[row][layer as usize] {
                if !seen[neighbor.row] {
                    seen[neighbor.row] = true;
                    frontier.push(neighbor.row);
                }
            }
        }
        seen
    }

    /// Give `node` an inbound edge from `host` at `layer`, evicting if the bound is met.
    ///
    /// Returns `false` when every entry in `host`'s adjacency is protected, so the caller
    /// must pick a different host. An already-present edge is a no-op and succeeds.
    ///
    /// Eviction takes the farthest **unprotected** entry, which keeps the degree bound
    /// exact and keeps every edge a previous repair depended on.
    pub(crate) fn link_protected(
        &mut self,
        host: usize,
        layer: u32,
        neighbor: Ranked,
        bound: usize,
        protected: &BTreeSet<usize>,
    ) -> bool {
        let list = &mut self.layers[host][layer as usize];
        if list.iter().any(|entry| entry.row == neighbor.row) {
            return true;
        }
        list.push(neighbor);
        list.sort_unstable();
        if list.len() > bound {
            let Some(victim) = list
                .iter()
                .rposition(|entry| entry.row != neighbor.row && !protected.contains(&entry.row))
            else {
                let restored = list
                    .iter()
                    .position(|entry| entry.row == neighbor.row)
                    .expect("the edge was just inserted");
                list.remove(restored);
                return false;
            };
            list.remove(victim);
        }
        true
    }

    /// Encode the graph into its canonical byte image, recording `arithmetic` -- the
    /// image code of the arithmetic and dispatch path its distances were computed
    /// on, and the build shape of one whose bits depend on the build -- in the header.
    pub(crate) fn canonical_image(
        &self,
        kernel: Kernel,
        params: &Params,
        arithmetic: Recorded,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&IMAGE_MAGIC);
        push_u32(&mut out, IMAGE_VERSION);
        push_u32(&mut out, kernel_tag(kernel));
        push_u64(&mut out, as_u64(params.m()));
        push_u64(&mut out, as_u64(params.m0()));
        push_u64(&mut out, as_u64(params.ef_construction()));
        push_u64(&mut out, as_u64(params.ef_search()));
        push_u64(&mut out, as_u64(self.node_count()));
        push_u32(&mut out, self.max_level);
        // The arithmetic, and path, the recorded distances were folded under, and the
        // shape of the build that compiled that path, where the arithmetic has one.
        push_u32(&mut out, arithmetic.code);
        if let Some(shape) = arithmetic.shape {
            push_u64(&mut out, shape.bits());
            push_u64(&mut out, shape.identity().digest());
        }
        push_u64(&mut out, self.entry.map_or(u64::MAX, as_u64));

        for row in 0..self.node_count() {
            push_u64(&mut out, as_u64(row));
            push_u32(&mut out, self.level(row));
            push_u32(&mut out, 0);
            for layer in 0..=self.level(row) {
                push_u32(&mut out, layer);
                push_u32(&mut out, 0);
                let mut neighbors: Vec<Ranked> = self.neighbors(row, layer).to_vec();
                neighbors.sort_unstable_by_key(|neighbor| neighbor.row);
                push_u64(&mut out, as_u64(neighbors.len()));
                for neighbor in neighbors {
                    push_u64(&mut out, as_u64(neighbor.row));
                    push_u64(&mut out, neighbor.distance.to_bits());
                }
            }
        }
        out
    }

    /// Rebuild the graph's entry point and maximum level from its levels.
    ///
    /// Used by the decoder to prove the decoded entry rule rather than trust the bytes.
    /// The minimum-row tie-break is structural, not conventional: a decoded graph whose
    /// recorded entry disagreed with it would be a graph this crate did not produce.
    fn recompute_entry(levels: &[u32]) -> (Option<usize>, u32) {
        let max_level = levels.iter().copied().max().unwrap_or(0);
        if levels.is_empty() {
            return (None, 0);
        }
        let entry = levels
            .iter()
            .enumerate()
            .filter(|(_, level)| **level == max_level)
            .map(|(row, _)| row)
            .min();
        (entry, max_level)
    }

    /// Reconstruct a graph from its decoded parts, rejecting anything malformed.
    pub(crate) fn from_decoded(
        levels: Vec<u32>,
        layers: Vec<Vec<Vec<Ranked>>>,
        recorded_entry: Option<usize>,
    ) -> Result<Self> {
        let n = levels.len();
        if layers.len() != n {
            return Err(HnswError::InvalidPayload {
                reason: format!("payload holds {n} levels but {} node records", layers.len()),
            });
        }
        for row in 0..n {
            if layers[row].len() != levels[row] as usize + 1 {
                return Err(HnswError::InvalidPayload {
                    reason: format!(
                        "node {row} has level {} but {} layer(s)",
                        levels[row],
                        layers[row].len()
                    ),
                });
            }
            for (layer, neighbors) in layers[row].iter().enumerate() {
                for neighbor in neighbors {
                    if neighbor.row >= n {
                        return Err(HnswError::InvalidPayload {
                            reason: format!(
                                "node {row} layer {layer} names row {}, beyond the {n} rows \
                                 the payload holds",
                                neighbor.row
                            ),
                        });
                    }
                    if neighbor.row == row {
                        return Err(HnswError::InvalidPayload {
                            reason: format!("node {row} links to itself at layer {layer}"),
                        });
                    }
                    if !neighbor.distance.is_finite() {
                        return Err(HnswError::InvalidPayload {
                            reason: format!("node {row} layer {layer} holds a non-finite distance"),
                        });
                    }
                    // Row ordering is an encoding-time property, checked against the raw
                    // bytes in `decode_image`; the adjacency handed here has already been
                    // re-sorted into the in-memory rank order, so only the identity of each
                    // neighbour is validated at this point.
                }
            }
        }

        let (entry, max_level) = Self::recompute_entry(&levels);
        if recorded_entry != entry {
            return Err(HnswError::InvalidPayload {
                reason: format!(
                    "payload records entry point {recorded_entry:?}, but the minimum row at \
                     the maximum level is {entry:?}"
                ),
            });
        }
        Ok(Self {
            levels,
            layers,
            entry,
            max_level,
        })
    }

    /// Check every layer's degree against `params`.
    pub(crate) fn validate_degrees(&self, params: &Params) -> Result<()> {
        for row in 0..self.node_count() {
            for layer in 0..=self.level(row) {
                let bound = params.degree_bound(layer);
                let degree = self.neighbors(row, layer).len();
                if degree > bound {
                    return Err(HnswError::InvalidPayload {
                        reason: format!(
                            "node {row} layer {layer} holds {degree} neighbours, over the \
                             {bound} the parameters admit"
                        ),
                    });
                }
            }
        }
        Ok(())
    }
}

/// The decoded canonical image: the graph plus the identity it was built under.
pub(crate) struct GraphImage {
    pub graph: Graph,
    pub kernel: Kernel,
    pub params: Params,
    /// What the header records about the arithmetic: one of the decoding arithmetic's
    /// codes, and a build shape exactly when that arithmetic has one.
    pub arithmetic: Recorded,
}

/// Decode a canonical image as one computed under arithmetic `A`, validating every
/// structural invariant it must satisfy.
///
/// A header whose arithmetic field is not one of `A`'s image codes is refused with
/// [`HnswError::ArithmeticMismatch`]: its distances were folded under another law. Which
/// of `A`'s codes it records, and the build shape it records when `A` has one, are
/// returned for the caller to hold against the path it runs and the build it is.
pub(crate) fn decode_image<A: Arithmetic>(bytes: &[u8]) -> Result<GraphImage> {
    let mut cursor = Cursor::new(bytes);
    if cursor.take(IMAGE_MAGIC.len())? != IMAGE_MAGIC.as_slice() {
        return Err(HnswError::InvalidPayload {
            reason: "the payload does not begin with the HNSW image marker".to_owned(),
        });
    }
    let version = cursor.u32()?;
    if version != IMAGE_VERSION {
        return Err(HnswError::VersionMismatch {
            expected: IMAGE_VERSION,
            actual: version,
        });
    }
    let kernel = kernel_of_tag(cursor.u32()?)?;
    let params = Params::new(
        cursor.usize()?,
        cursor.usize()?,
        cursor.usize()?,
        cursor.usize()?,
    )?;
    let node_count = cursor.usize()?;
    let max_level = cursor.u32()?;
    let code = cursor.u32()?;
    if !A::IMAGE_CODES.contains(&code) {
        return Err(HnswError::ArithmeticMismatch {
            arithmetic: A::ID,
            actual: code,
        });
    }
    // The code is one of `A`'s, so the fields follow exactly when `A` has a shape.
    let shape = match A::build_shape() {
        Some(_) => {
            let bits = cursor.u64()?;
            Some(BuildShape::from_parts(
                bits,
                BuildIdentity::from_digest(cursor.u64()?),
            ))
        }
        None => None,
    };
    let arithmetic = Recorded { code, shape };
    let entry_raw = cursor.u64()?;

    // A node record is at least 16 bytes of fixed fields (row, level, reserved) plus one
    // 16-byte layer record (layer, reserved, count), so a payload claiming more nodes than
    // its remaining bytes could hold is malformed. The check runs BEFORE any allocation:
    // without it a hostile count turns a bad payload into an allocator abort, which is not
    // a typed error.
    const MIN_NODE_BYTES: usize = 16 + 16;
    if node_count > cursor.remaining() / MIN_NODE_BYTES {
        return Err(HnswError::InvalidPayload {
            reason: format!(
                "the payload claims {node_count} node(s) but holds only {} byte(s) after the \
                 header",
                cursor.remaining()
            ),
        });
    }
    // The level cap is a function of the node count and M, and the encoder cannot record a
    // level above it. Bounding `max_level` here bounds every per-node layer allocation.
    let level_ceiling = crate::level::level_cap(node_count, params.m());
    if max_level > level_ceiling {
        return Err(HnswError::InvalidPayload {
            reason: format!(
                "the header records max level {max_level}, above the {level_ceiling} the node \
                 count and M admit"
            ),
        });
    }

    let mut levels = Vec::with_capacity(node_count);
    let mut layers: Vec<Vec<Vec<Ranked>>> = Vec::with_capacity(node_count);
    for row in 0..node_count {
        let recorded_row = cursor.usize()?;
        if recorded_row != row {
            return Err(HnswError::InvalidPayload {
                reason: format!("node record {row} carries row index {recorded_row}"),
            });
        }
        let level = cursor.u32()?;
        if level > max_level {
            return Err(HnswError::InvalidPayload {
                reason: format!("node {row} declares level {level}, above the cap {max_level}"),
            });
        }
        let reserved = cursor.u32()?;
        if reserved != 0 {
            return Err(HnswError::InvalidPayload {
                reason: format!("node {row}'s reserved field is {reserved}, not zero"),
            });
        }
        let mut node_layers = Vec::with_capacity(level as usize + 1);
        for layer in 0..=level {
            let recorded_layer = cursor.u32()?;
            if recorded_layer != layer {
                return Err(HnswError::InvalidPayload {
                    reason: format!(
                        "node {row} layer record {layer} carries layer index {recorded_layer}"
                    ),
                });
            }
            let reserved = cursor.u32()?;
            if reserved != 0 {
                return Err(HnswError::InvalidPayload {
                    reason: format!("node {row} layer {layer}'s reserved field is not zero"),
                });
            }
            let count = cursor.usize()?;
            // Each neighbour is a row (u64) plus a distance (u64): 16 bytes. Reject a count
            // the remaining payload cannot possibly hold before allocating.
            if count > cursor.remaining() / 16 {
                return Err(HnswError::InvalidPayload {
                    reason: format!(
                        "node {row} layer {layer} claims {count} neighbour(s) but only {} \
                         byte(s) remain",
                        cursor.remaining()
                    ),
                });
            }
            let mut neighbors = Vec::with_capacity(count);
            let mut previous: Option<usize> = None;
            for _ in 0..count {
                let neighbor_row = cursor.usize()?;
                let distance = f64::from_bits(cursor.u64()?);
                if neighbor_row >= node_count {
                    return Err(HnswError::InvalidPayload {
                        reason: format!(
                            "node {row} layer {layer} names row {neighbor_row}, beyond the \
                             {node_count} rows the payload holds"
                        ),
                    });
                }
                if previous.is_some_and(|p| neighbor_row <= p) {
                    return Err(HnswError::InvalidPayload {
                        reason: format!(
                            "node {row} layer {layer} is not sorted strictly by neighbour row"
                        ),
                    });
                }
                previous = Some(neighbor_row);
                if !distance.is_finite() {
                    return Err(HnswError::InvalidPayload {
                        reason: format!("node {row} layer {layer} holds a non-finite distance"),
                    });
                }
                // Re-sort into the in-memory rank order.
                neighbors.push(Ranked {
                    distance,
                    row: neighbor_row,
                });
            }
            neighbors.sort_unstable();
            node_layers.push(neighbors);
        }
        levels.push(level);
        layers.push(node_layers);
    }
    cursor.finish()?;

    let entry = if entry_raw == u64::MAX {
        None
    } else {
        let entry = usize::try_from(entry_raw).map_err(|_| HnswError::InvalidPayload {
            reason: format!("entry point {entry_raw} does not fit this platform"),
        })?;
        if entry >= node_count {
            return Err(HnswError::InvalidPayload {
                reason: format!("entry point {entry} is beyond the {node_count} rows"),
            });
        }
        Some(entry)
    };

    let graph = Graph::from_decoded(levels, layers, entry)?;
    graph.validate_degrees(&params)?;
    Ok(GraphImage {
        graph,
        kernel,
        params,
        arithmetic,
    })
}

/// The stable tag a [`Kernel`] is written as.
fn kernel_tag(kernel: Kernel) -> u32 {
    match kernel {
        Kernel::Cosine => 0,
        Kernel::NegativeDot => 1,
        Kernel::SquaredEuclidean => 2,
    }
}

/// The [`Kernel`] a tag names.
fn kernel_of_tag(tag: u32) -> Result<Kernel> {
    match tag {
        0 => Ok(Kernel::Cosine),
        1 => Ok(Kernel::NegativeDot),
        2 => Ok(Kernel::SquaredEuclidean),
        other => Err(HnswError::InvalidPayload {
            reason: format!("unknown distance kernel tag {other}"),
        }),
    }
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Widen a `usize` for the fixed-width little-endian image.
///
/// On a 32-bit target the conversion is infallible; on 64-bit it is still exact. The
/// payload is only ever produced for indexes that fit the target, so this never truncates.
fn as_u64(value: usize) -> u64 {
    value as u64
}

/// A bounds-checked reader over the canonical image.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| HnswError::InvalidPayload {
                reason: "payload offset overflowed".to_owned(),
            })?;
        if end > self.bytes.len() {
            return Err(HnswError::InvalidPayload {
                reason: format!(
                    "payload ends after {} bytes but a {count}-byte field was expected at \
                     offset {}",
                    self.bytes.len(),
                    self.at
                ),
            });
        }
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("take returned four bytes"),
        ))
    }

    fn u64(&mut self) -> Result<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("take returned eight bytes"),
        ))
    }

    fn usize(&mut self) -> Result<usize> {
        let value = self.u64()?;
        usize::try_from(value).map_err(|_| HnswError::InvalidPayload {
            reason: format!("payload value {value} does not fit this platform's index range"),
        })
    }

    /// The number of bytes not yet consumed.
    ///
    /// Used to bound an untrusted length before it is handed to an allocator.
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.at)
    }

    /// Fail if any bytes remain; trailing bytes are a corrupt payload, not padding.
    fn finish(&self) -> Result<()> {
        if self.at != self.bytes.len() {
            return Err(HnswError::InvalidPayload {
                reason: format!(
                    "payload holds {} trailing byte(s) after the last node record",
                    self.bytes.len() - self.at
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::distance::{Exact, Reassociated};

    fn params() -> Params {
        Params::new(4, 8, 16, 8).expect("valid")
    }

    /// What an exact image records: its one code and no shape.
    const fn exact() -> Recorded {
        Recorded {
            code: Exact::IMAGE_CODE,
            shape: None,
        }
    }

    fn sample_graph() -> Graph {
        // levels [0, 1, 0, 2, 1]; entry must be row 3 (min row at max level 2).
        let mut graph = Graph::with_levels(vec![0, 1, 0, 2, 1]);
        graph.commit(
            vec![
                Edge {
                    node: 1,
                    layer: 0,
                    neighbor: Ranked {
                        distance: 1.0,
                        row: 2,
                    },
                },
                Edge {
                    node: 2,
                    layer: 0,
                    neighbor: Ranked {
                        distance: 1.0,
                        row: 1,
                    },
                },
                Edge {
                    node: 3,
                    layer: 0,
                    neighbor: Ranked {
                        distance: 0.5,
                        row: 0,
                    },
                },
                Edge {
                    node: 0,
                    layer: 0,
                    neighbor: Ranked {
                        distance: 0.5,
                        row: 3,
                    },
                },
            ],
            &params(),
        );
        graph.promote_entry(3, 2);
        graph
    }

    #[test]
    fn a_matrix_rejects_ragged_and_non_finite_input() {
        assert!(VectorMatrix::new(2, 2, vec![1.0, 2.0, 3.0]).is_err());
        assert!(VectorMatrix::new(2, 2, vec![1.0, 2.0, 3.0, f64::NAN]).is_err());
        assert!(VectorMatrix::new(0, 2, Vec::new()).is_err());
        assert!(VectorMatrix::from_rows(&[vec![1.0, 2.0], vec![3.0]]).is_err());
        assert!(VectorMatrix::from_rows(&[vec![1.0, 2.0], vec![3.0, 4.0]]).is_ok());
    }

    #[test]
    fn commit_sorts_truncates_and_dedups_neighbours() {
        let mut graph = Graph::with_levels(vec![0, 0]);
        graph.commit(
            vec![
                Edge {
                    node: 0,
                    layer: 0,
                    neighbor: Ranked {
                        distance: 5.0,
                        row: 1,
                    },
                },
                Edge {
                    node: 0,
                    layer: 0,
                    neighbor: Ranked {
                        distance: 5.0,
                        row: 1,
                    },
                },
            ],
            &Params::new(2, 2, 2, 1).expect("valid"),
        );
        assert_eq!(graph.neighbors(0, 0).len(), 1, "duplicates are removed");
        assert_eq!(graph.neighbors(0, 0)[0].row, 1);
    }

    #[test]
    fn entry_point_is_the_min_row_at_the_max_level() {
        let graph = sample_graph();
        assert_eq!(graph.max_level(), 2);
        assert_eq!(graph.entry(), Some(3));
    }

    #[test]
    fn a_canonical_image_round_trips_byte_for_byte() {
        let graph = sample_graph();
        let params = params();
        let image = graph.canonical_image(Kernel::SquaredEuclidean, &params, exact());
        let decoded = decode_image::<Exact>(&image).expect("the image decodes");
        assert_eq!(decoded.kernel, Kernel::SquaredEuclidean);
        assert_eq!(decoded.params, params);
        assert_eq!(decoded.arithmetic, exact());
        assert_eq!(decoded.graph, graph, "the graph survives decode");
        let reencoded =
            decoded
                .graph
                .canonical_image(decoded.kernel, &decoded.params, decoded.arithmetic);
        assert_eq!(image, reencoded, "decode then encode is the identity");
    }

    #[test]
    fn the_header_code_is_decoded_only_by_its_own_arithmetic() {
        let graph = sample_graph();
        let params = params();
        for &code in Reassociated::IMAGE_CODES {
            let recorded = Recorded {
                code,
                shape: Some(BuildShape::here()),
            };
            let image = graph.canonical_image(Kernel::SquaredEuclidean, &params, recorded);
            assert_eq!(
                decode_image::<Reassociated>(&image)
                    .expect("a reassociated code decodes as reassociated")
                    .arithmetic,
                recorded
            );
            assert!(matches!(
                decode_image::<Exact>(&image),
                Err(HnswError::ArithmeticMismatch { actual, .. }) if actual == code
            ));
        }
        let exact_image = graph.canonical_image(Kernel::SquaredEuclidean, &params, exact());
        assert!(matches!(
            decode_image::<Reassociated>(&exact_image),
            Err(HnswError::ArithmeticMismatch { actual: 1, .. })
        ));
    }

    #[test]
    fn only_a_reassociated_header_carries_a_build_shape() {
        let graph = sample_graph();
        let params = params();
        let exact_image = graph.canonical_image(Kernel::SquaredEuclidean, &params, exact());
        let shape = BuildShape::from_parts(
            0x0123_4567_89ab_cdef,
            BuildIdentity::from_digest(0xfedc_ba98_7654_3210),
        );
        let reassociated = Recorded {
            code: 3,
            shape: Some(shape),
        };
        let image = graph.canonical_image(Kernel::SquaredEuclidean, &params, reassociated);
        // The shape is the sixteen bytes after the arithmetic code, its bits then its
        // identity's digest; everything else is the exact image's, shifted by them.
        assert_eq!(image.len(), exact_image.len() + 16);
        assert_eq!(image[..60], exact_image[..60]);
        assert_eq!(image[60..64], 3_u32.to_le_bytes());
        assert_eq!(image[64..72], shape.bits().to_le_bytes());
        assert_eq!(image[72..80], shape.identity().digest().to_le_bytes());
        assert_eq!(image[80..], exact_image[64..]);
        // The decoder reads the shape back verbatim, whatever build it names; holding it
        // against this build is the caller's refusal, not the decoder's.
        let decoded = decode_image::<Reassociated>(&image).expect("decodes");
        assert_eq!(decoded.arithmetic, reassociated);
        assert_eq!(
            decoded
                .graph
                .canonical_image(decoded.kernel, &decoded.params, decoded.arithmetic),
            image
        );
        // An image whose shape field is missing is truncated, not an exact image.
        let mut shapeless = exact_image;
        shapeless[60..64].copy_from_slice(&3_u32.to_le_bytes());
        assert!(matches!(
            decode_image::<Reassociated>(&shapeless),
            Err(HnswError::InvalidPayload { .. })
        ));
    }

    #[test]
    fn truncated_and_corrupt_images_are_refused() {
        let graph = sample_graph();
        let image = graph.canonical_image(Kernel::Cosine, &params(), exact());
        assert!(decode_image::<Exact>(&image[..image.len() - 1]).is_err());
        let mut trailing = image.clone();
        trailing.push(0);
        assert!(decode_image::<Exact>(&trailing).is_err());
        let mut bad_magic = image.clone();
        bad_magic[0] ^= 0xff;
        assert!(decode_image::<Exact>(&bad_magic).is_err());
        let mut bad_version = image;
        bad_version[8] = 0xff;
        assert!(decode_image::<Exact>(&bad_version).is_err());
    }
}
