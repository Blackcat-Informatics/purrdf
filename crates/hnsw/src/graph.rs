// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! At any moment the entry point is **the minimum row index among the nodes committed at
//! the current maximum level**. Standard HNSW leaves the tie-break to insertion order,
//! which is exactly the kind of hidden state determinism cannot tolerate; pinning it to
//! the row index makes the entry point a function of the committed set alone.

use crate::error::{HnswError, Result};
use crate::params::Params;
use purrdf_sparql_eval::knn::{Kernel, Ranked};

/// The canonical image's magic marker; identifies the format before any length is trusted.
pub(crate) const IMAGE_MAGIC: [u8; 8] = *b"PURHNSW1";

/// The canonical image's format version.
pub(crate) const IMAGE_VERSION: u32 = 1;

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
    data: Vec<f64>,
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
        for (index, value) in data.iter().enumerate() {
            if !value.is_finite() {
                return Err(HnswError::NonFiniteComponent {
                    row: index / dims,
                    column: index % dims,
                });
            }
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

    /// Row `row`'s components.
    ///
    /// # Panics
    ///
    /// Panics if `row` is not a valid row index; callers validate the index first.
    #[must_use]
    pub fn row(&self, row: usize) -> &[f64] {
        let start = row * self.dims;
        &self.data[start..start + self.dims]
    }

    /// The whole buffer, row-major.
    #[must_use]
    pub fn as_slice(&self) -> &[f64] {
        &self.data
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

    /// Promote `row` to the entry point at `level`.
    ///
    /// Called only when `level` strictly exceeds the current maximum, so the first row to
    /// attain a new maximum wins and the tie-break is "minimum row", as the module docs
    /// require.
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
            let mut merged = self.layers[node][layer as usize].clone();
            merged.extend(edges[start..end].iter().map(|edge| edge.neighbor));
            merged.sort_unstable();
            merged.dedup();
            merged.truncate(params.degree_bound(layer));
            self.layers[node][layer as usize] = merged;
            start = end;
        }
    }

    /// Encode the graph into its canonical byte image.
    pub(crate) fn canonical_image(&self, kernel: Kernel, params: &Params) -> Vec<u8> {
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
        push_u32(&mut out, 0);
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
}

/// Decode a canonical image, validating every structural invariant it must satisfy.
pub(crate) fn decode_image(bytes: &[u8]) -> Result<GraphImage> {
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
    let reserved = cursor.u32()?;
    if reserved != 0 {
        return Err(HnswError::InvalidPayload {
            reason: format!("the header's reserved field is {reserved}, not zero"),
        });
    }
    let entry_raw = cursor.u64()?;

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

    fn params() -> Params {
        Params::new(4, 8, 16, 8).expect("valid")
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
        let image = graph.canonical_image(Kernel::SquaredEuclidean, &params);
        let decoded = decode_image(&image).expect("the image decodes");
        assert_eq!(decoded.kernel, Kernel::SquaredEuclidean);
        assert_eq!(decoded.params, params);
        assert_eq!(decoded.graph, graph, "the graph survives decode");
        let reencoded = decoded
            .graph
            .canonical_image(decoded.kernel, &decoded.params);
        assert_eq!(image, reencoded, "decode then encode is the identity");
    }

    #[test]
    fn truncated_and_corrupt_images_are_refused() {
        let graph = sample_graph();
        let image = graph.canonical_image(Kernel::Cosine, &params());
        assert!(decode_image(&image[..image.len() - 1]).is_err());
        let mut trailing = image.clone();
        trailing.push(0);
        assert!(decode_image(&trailing).is_err());
        let mut bad_magic = image.clone();
        bad_magic[0] ^= 0xff;
        assert!(decode_image(&bad_magic).is_err());
        let mut bad_version = image;
        bad_version[8] = 0xff;
        assert!(decode_image(&bad_version).is_err());
    }
}
