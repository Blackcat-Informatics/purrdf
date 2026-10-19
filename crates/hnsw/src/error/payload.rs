// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original canonical-image validation operands.

/// A malformed image, with the values at the rejecting computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadRefusal {
    /// Level and node-record counts disagree.
    NodeCount {
        /// Number of levels.
        levels: usize,
        /// Number of node records.
        records: usize,
    },
    /// A node's recorded level disagrees with its layer count.
    LayerCount {
        /// Node row.
        row: usize,
        /// Recorded level.
        level: u32,
        /// Number of layers.
        layers: usize,
    },
    /// An edge names a row outside the image.
    Neighbor {
        /// Origin node.
        row: usize,
        /// Origin layer.
        layer: usize,
        /// Named neighbor.
        neighbor: usize,
        /// Number of nodes.
        rows: usize,
    },
    /// A node links to itself.
    SelfLink {
        /// Node row.
        row: usize,
        /// Edge layer.
        layer: usize,
    },
    /// An edge distance is not finite.
    NonFinite {
        /// Origin node.
        row: usize,
        /// Origin layer.
        layer: usize,
        /// Exact original distance bits.
        bits: u64,
    },
    /// Recorded and derived entry points disagree.
    EntryMismatch {
        /// Recorded entry point.
        recorded: Option<usize>,
        /// Derived entry point.
        expected: Option<usize>,
    },
    /// A layer exceeds its admitted degree bound.
    Degree {
        /// Node row.
        row: usize,
        /// Layer index.
        layer: u32,
        /// Actual degree.
        degree: usize,
        /// Admitted degree bound.
        bound: usize,
    },
    /// Incorrect image marker.
    Magic {
        /// Original marker bytes.
        actual: Vec<u8>,
        /// Required marker bytes.
        expected: &'static [u8],
    },
    /// Remaining bytes cannot hold the claimed nodes.
    NodeBytes {
        /// Claimed node count.
        nodes: usize,
        /// Remaining image bytes.
        remaining: usize,
        /// Minimum bytes per node.
        minimum: usize,
    },
    /// Header level exceeds the level law.
    MaximumLevel {
        /// Header's maximum level.
        actual: u32,
        /// Derived maximum level.
        maximum: u32,
        /// Input node count.
        nodes: usize,
        /// Input degree parameter.
        m: usize,
    },
    /// Node records are out of sequence.
    RowIndex {
        /// Expected node position.
        expected: usize,
        /// Recorded node position.
        actual: usize,
    },
    /// A node exceeds the header's maximum level.
    NodeLevel {
        /// Node row.
        row: usize,
        /// Recorded node level.
        actual: u32,
        /// Header's maximum level.
        maximum: u32,
    },
    /// A node's reserved field is not zero.
    ReservedNode {
        /// Node row.
        row: usize,
        /// Original reserved value.
        value: u32,
    },
    /// Layer records are out of sequence.
    LayerIndex {
        /// Node row.
        row: usize,
        /// Expected layer position.
        expected: u32,
        /// Recorded layer position.
        actual: u32,
    },
    /// A layer's reserved field is not zero.
    ReservedLayer {
        /// Node row.
        row: usize,
        /// Layer index.
        layer: u32,
        /// Original reserved value.
        value: u32,
    },
    /// Remaining bytes cannot hold the claimed neighbors.
    NeighborBytes {
        /// Node row.
        row: usize,
        /// Layer index.
        layer: u32,
        /// Claimed neighbor count.
        count: usize,
        /// Remaining bytes.
        remaining: usize,
    },
    /// Neighbor rows are not strictly ascending.
    NeighborOrder {
        /// Origin node.
        row: usize,
        /// Origin layer.
        layer: u32,
        /// Previous neighbor row.
        previous: usize,
        /// Current neighbor row.
        actual: usize,
    },
    /// Entry point cannot be addressed on this platform.
    EntryPlatform {
        /// Original entry point.
        value: u64,
        /// Maximum platform index.
        maximum: usize,
    },
    /// Entry point is outside the node table.
    EntryRows {
        /// Recorded entry point.
        entry: usize,
        /// Node-table length.
        rows: usize,
    },
    /// Unknown distance kernel tag.
    KernelTag(u32),
    /// Advancing the cursor would overflow.
    Offset {
        /// Current offset.
        at: usize,
        /// Requested bytes.
        count: usize,
    },
    /// A field exceeds the remaining bytes.
    Truncated {
        /// Original image length.
        available: usize,
        /// Current byte offset.
        at: usize,
        /// Required field width.
        count: usize,
    },
    /// A payload index cannot be addressed on this platform.
    ValuePlatform {
        /// Original payload index.
        value: u64,
        /// Maximum platform index.
        maximum: usize,
    },
    /// Bytes remain after the last node.
    Trailing {
        /// Number of trailing bytes.
        count: usize,
    },
    /// Image node count differs from the source matrix.
    MatrixRows {
        /// Image node count.
        nodes: usize,
        /// Source matrix row count.
        rows: usize,
    },
}

impl purrdf_hash::error::StructuredError for PayloadRefusal {
    purrdf_hash::error_visit! {
        NodeCount { levels, records } => "hnsw-payload-node-count" { "levels" => Unsigned(*levels as u128), "records" => Unsigned(*records as u128) },
        LayerCount { row, level, layers } => "hnsw-payload-layer-count" { "row" => Unsigned(*row as u128), "level" => Unsigned(u128::from(*level)), "layers" => Unsigned(*layers as u128) },
        Neighbor { row, layer, neighbor, rows } => "hnsw-payload-neighbor" { "row" => Unsigned(*row as u128), "layer" => Unsigned(*layer as u128), "neighbor" => Unsigned(*neighbor as u128), "rows" => Unsigned(*rows as u128) },
        SelfLink { row, layer } => "hnsw-payload-self-link" { "row" => Unsigned(*row as u128), "layer" => Unsigned(*layer as u128) },
        NonFinite { row, layer, bits } => "hnsw-payload-non-finite" { "row" => Unsigned(*row as u128), "layer" => Unsigned(*layer as u128), "distance" => Float64Bits(*bits) },
        EntryMismatch { recorded, expected } => "hnsw-payload-entry-mismatch" {
            "recorded" => Value(recorded.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(value as u128))),
            "expected" => Value(expected.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(value as u128))),
        },
        Degree { row, layer, degree, bound } => "hnsw-payload-degree" { "row" => Unsigned(*row as u128), "layer" => Unsigned(u128::from(*layer)), "degree" => Unsigned(*degree as u128), "bound" => Unsigned(*bound as u128) },
        Magic { actual, expected } => "hnsw-payload-magic" { "actual" => Bytes(actual), "expected" => Bytes(expected) },
        NodeBytes { nodes, remaining, minimum } => "hnsw-payload-node-bytes" { "nodes" => Unsigned(*nodes as u128), "remaining" => ByteCount(*remaining as u128), "minimum" => ByteCount(*minimum as u128) },
        MaximumLevel { actual, maximum, nodes, m } => "hnsw-payload-maximum-level" { "actual" => Unsigned(u128::from(*actual)), "maximum" => Unsigned(u128::from(*maximum)), "nodes" => Unsigned(*nodes as u128), "m" => Unsigned(*m as u128) },
        RowIndex { expected, actual } => "hnsw-payload-row-index" { "expected" => Unsigned(*expected as u128), "actual" => Unsigned(*actual as u128) },
        NodeLevel { row, actual, maximum } => "hnsw-payload-node-level" { "row" => Unsigned(*row as u128), "actual" => Unsigned(u128::from(*actual)), "maximum" => Unsigned(u128::from(*maximum)) },
        ReservedNode { row, value } => "hnsw-payload-reserved-node" { "row" => Unsigned(*row as u128), "value" => Unsigned(u128::from(*value)) },
        LayerIndex { row, expected, actual } => "hnsw-payload-layer-index" { "row" => Unsigned(*row as u128), "expected" => Unsigned(u128::from(*expected)), "actual" => Unsigned(u128::from(*actual)) },
        ReservedLayer { row, layer, value } => "hnsw-payload-reserved-layer" { "row" => Unsigned(*row as u128), "layer" => Unsigned(u128::from(*layer)), "value" => Unsigned(u128::from(*value)) },
        NeighborBytes { row, layer, count, remaining } => "hnsw-payload-neighbor-bytes" { "row" => Unsigned(*row as u128), "layer" => Unsigned(u128::from(*layer)), "count" => Unsigned(*count as u128), "remaining" => ByteCount(*remaining as u128) },
        NeighborOrder { row, layer, previous, actual } => "hnsw-payload-neighbor-order" { "row" => Unsigned(*row as u128), "layer" => Unsigned(u128::from(*layer)), "previous" => Unsigned(*previous as u128), "actual" => Unsigned(*actual as u128) },
        EntryPlatform { value, maximum } => "hnsw-payload-entry-platform" { "value" => Unsigned(u128::from(*value)), "maximum" => Unsigned(*maximum as u128) },
        EntryRows { entry, rows } => "hnsw-payload-entry-rows" { "entry" => Unsigned(*entry as u128), "rows" => Unsigned(*rows as u128) },
        KernelTag(value) => "hnsw-payload-kernel-tag" { "value" => Unsigned(u128::from(*value)) },
        Offset { at, count } => "hnsw-payload-offset" { "at" => Unsigned(*at as u128), "count" => ByteCount(*count as u128) },
        Truncated { available, at, count } => "hnsw-payload-truncated" { "available" => ByteCount(*available as u128), "at" => Unsigned(*at as u128), "count" => ByteCount(*count as u128) },
        ValuePlatform { value, maximum } => "hnsw-payload-value-platform" { "value" => Unsigned(u128::from(*value)), "maximum" => Unsigned(*maximum as u128) },
        Trailing { count } => "hnsw-payload-trailing" { "count" => ByteCount(*count as u128) },
        MatrixRows { nodes, rows } => "hnsw-payload-matrix-rows" { "nodes" => Unsigned(*nodes as u128), "rows" => Unsigned(*rows as u128) },
    }
}

impl core::fmt::Display for PayloadRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NodeCount { levels, records } => write!(
                f,
                "payload holds {levels} levels but {records} node records"
            ),
            Self::LayerCount { row, level, layers } => {
                write!(f, "node {row} has level {level} but {layers} layer(s)")
            }
            Self::Neighbor {
                row,
                layer,
                neighbor,
                rows,
            } => write!(
                f,
                "node {row} layer {layer} names row {neighbor}, beyond the {rows} rows the payload holds"
            ),
            Self::SelfLink { row, layer } => {
                write!(f, "node {row} links to itself at layer {layer}")
            }
            Self::NonFinite {
                row,
                layer,
                bits: _,
            } => write!(f, "node {row} layer {layer} holds a non-finite distance"),
            Self::EntryMismatch { recorded, expected } => write!(
                f,
                "payload records entry point {recorded:?}, but the minimum row at the maximum level is {expected:?}"
            ),
            Self::Degree {
                row,
                layer,
                degree,
                bound,
            } => write!(
                f,
                "node {row} layer {layer} holds {degree} neighbours, over the {bound} the parameters admit"
            ),
            Self::Magic {
                actual: _,
                expected: _,
            } => f.write_str("the payload does not begin with the HNSW image marker"),
            Self::NodeBytes {
                nodes,
                remaining,
                minimum: _,
            } => write!(
                f,
                "the payload claims {nodes} node(s) but holds only {remaining} byte(s) after the header"
            ),
            Self::MaximumLevel {
                actual,
                maximum,
                nodes: _,
                m: _,
            } => write!(
                f,
                "the header records max level {actual}, above the {maximum} the node count and M admit"
            ),
            Self::RowIndex { expected, actual } => {
                write!(f, "node record {expected} carries row index {actual}")
            }
            Self::NodeLevel {
                row,
                actual,
                maximum,
            } => write!(
                f,
                "node {row} declares level {actual}, above the cap {maximum}"
            ),
            Self::ReservedNode { row, value } => {
                write!(f, "node {row}'s reserved field is {value}, not zero")
            }
            Self::LayerIndex {
                row,
                expected,
                actual,
            } => write!(
                f,
                "node {row} layer record {expected} carries layer index {actual}"
            ),
            Self::ReservedLayer {
                row,
                layer,
                value: _,
            } => write!(f, "node {row} layer {layer}'s reserved field is not zero"),
            Self::NeighborBytes {
                row,
                layer,
                count,
                remaining,
            } => write!(
                f,
                "node {row} layer {layer} claims {count} neighbour(s) but only {remaining} byte(s) remain"
            ),
            Self::NeighborOrder {
                row,
                layer,
                previous: _,
                actual: _,
            } => write!(
                f,
                "node {row} layer {layer} is not sorted strictly by neighbour row"
            ),
            Self::EntryPlatform { value, maximum: _ } => {
                write!(f, "entry point {value} does not fit this platform")
            }
            Self::EntryRows { entry, rows } => {
                write!(f, "entry point {entry} is beyond the {rows} rows")
            }
            Self::KernelTag(value) => write!(f, "unknown distance kernel tag {value}"),
            Self::Offset { at: _, count: _ } => f.write_str("payload offset overflowed"),
            Self::Truncated {
                available,
                at,
                count,
            } => write!(
                f,
                "payload ends after {available} bytes but a {count}-byte field was expected at offset {at}"
            ),
            Self::ValuePlatform { value, maximum: _ } => write!(
                f,
                "payload value {value} does not fit this platform's index range"
            ),
            Self::Trailing { count } => write!(
                f,
                "payload holds {count} trailing byte(s) after the last node record"
            ),
            Self::MatrixRows { nodes, rows } => write!(
                f,
                "the payload holds {nodes} node(s) but the matrix holds {rows} row(s)"
            ),
        }
    }
}

impl std::error::Error for PayloadRefusal {}
