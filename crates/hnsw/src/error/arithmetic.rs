// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original operands of matrix and index-footprint arithmetic refusals.

/// The exact calculation that could not represent its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArithmeticRefusal {
    /// The platform-sized matrix element count.
    MatrixElements {
        /// Original row count.
        rows: usize,
        /// Original dimension count.
        dimensions: usize,
    },
    /// The platform-sized vector storage size.
    MatrixBytes {
        /// Admitted element count.
        elements: usize,
        /// Storage width charged for each element.
        element_bytes: usize,
    },
    /// A row count cannot be represented by the portable footprint law.
    RowsConversion {
        /// Original row count.
        rows: usize,
        /// Original conversion failure.
        cause: core::num::TryFromIntError,
    },
    /// A dimension count cannot be represented by the portable footprint law.
    DimensionsConversion {
        /// Original dimension count.
        dimensions: usize,
        /// Original conversion failure.
        cause: core::num::TryFromIntError,
    },
    /// The portable vector element count.
    VectorElements {
        /// Admitted row count.
        rows: u64,
        /// Admitted dimension count.
        dimensions: u64,
    },
    /// The portable vector storage size.
    VectorBytes {
        /// Admitted element count.
        elements: u64,
        /// Storage width charged for each element.
        element_bytes: u64,
    },
    /// The portable norm storage size.
    NormBytes {
        /// Admitted row count.
        rows: u64,
        /// Storage width charged for each norm.
        norm_bytes: u64,
    },
    /// The portable node-header storage size.
    NodeBytes {
        /// Admitted row count.
        rows: u64,
        /// Storage charged for both headers of each node.
        node_bytes: u64,
    },
    /// The branching factor cannot be represented by the footprint law.
    BranchingConversion {
        /// Original branching factor.
        branching: usize,
        /// Original conversion failure.
        cause: core::num::TryFromIntError,
    },
    /// A layer's degree cannot be represented by the footprint law.
    DegreeConversion {
        /// Original layer index.
        layer: u64,
        /// Original degree bound.
        degree: usize,
        /// Original conversion failure.
        cause: core::num::TryFromIntError,
    },
    /// One layer's edge count.
    LayerEdges {
        /// Original layer index.
        layer: u64,
        /// Expected population at this layer.
        population: u64,
        /// Admitted degree bound.
        degree: u64,
    },
    /// Accumulation of the layer edge counts.
    AccumulatedEdges {
        /// Original layer index.
        layer: u64,
        /// Edge count before this layer.
        accumulated: u64,
        /// This layer's admitted edge count.
        next: u64,
    },
    /// The portable adjacency storage size.
    EdgeBytes {
        /// Admitted edge count.
        edges: u64,
        /// Storage width charged for each edge.
        edge_bytes: u64,
    },
    /// Vector and norm storage accumulation.
    VectorAndNormBytes {
        /// Admitted vector storage.
        vectors: u64,
        /// Admitted norm storage.
        norms: u64,
    },
    /// Addition of node-header storage.
    WithNodeBytes {
        /// Admitted vector and norm storage.
        accumulated: u64,
        /// Admitted node-header storage.
        nodes: u64,
    },
    /// Addition of adjacency storage.
    WithEdgeBytes {
        /// Admitted vector, norm and node-header storage.
        accumulated: u64,
        /// Admitted adjacency storage.
        edges: u64,
    },
}

impl purrdf_hash::error::StructuredError for ArithmeticRefusal {
    purrdf_hash::error_visit! {
        MatrixElements { rows, dimensions } => "hnsw-arithmetic-matrix-elements" {
            "operation" => Code("multiply"), "rows" => Unsigned(*rows as u128),
            "dimensions" => Unsigned(*dimensions as u128), "maximum" => Unsigned(usize::MAX as u128),
        },
        MatrixBytes { elements, element_bytes } => "hnsw-arithmetic-matrix-bytes" {
            "operation" => Code("multiply"), "elements" => Unsigned(*elements as u128),
            "element_bytes" => ByteCount(*element_bytes as u128), "maximum" => ByteCount(usize::MAX as u128),
        },
        RowsConversion { rows, cause } => "hnsw-arithmetic-rows-conversion" {
            "operation" => Code("convert-u64"), "rows" => Unsigned(*rows as u128),
            "maximum" => Unsigned(u128::from(u64::MAX)), "cause" => Cause(cause),
        },
        DimensionsConversion { dimensions, cause } => "hnsw-arithmetic-dimensions-conversion" {
            "operation" => Code("convert-u64"), "dimensions" => Unsigned(*dimensions as u128),
            "maximum" => Unsigned(u128::from(u64::MAX)), "cause" => Cause(cause),
        },
        VectorElements { rows, dimensions } => "hnsw-arithmetic-vector-elements" {
            "operation" => Code("multiply"), "rows" => Unsigned(u128::from(*rows)),
            "dimensions" => Unsigned(u128::from(*dimensions)), "maximum" => Unsigned(u128::from(u64::MAX)),
        },
        VectorBytes { elements, element_bytes } => "hnsw-arithmetic-vector-bytes" {
            "operation" => Code("multiply"), "elements" => Unsigned(u128::from(*elements)),
            "element_bytes" => ByteCount(u128::from(*element_bytes)), "maximum" => ByteCount(u128::from(u64::MAX)),
        },
        NormBytes { rows, norm_bytes } => "hnsw-arithmetic-norm-bytes" {
            "operation" => Code("multiply"), "rows" => Unsigned(u128::from(*rows)),
            "norm_bytes" => ByteCount(u128::from(*norm_bytes)), "maximum" => ByteCount(u128::from(u64::MAX)),
        },
        NodeBytes { rows, node_bytes } => "hnsw-arithmetic-node-bytes" {
            "operation" => Code("multiply"), "rows" => Unsigned(u128::from(*rows)),
            "node_bytes" => ByteCount(u128::from(*node_bytes)), "maximum" => ByteCount(u128::from(u64::MAX)),
        },
        BranchingConversion { branching, cause } => "hnsw-arithmetic-branching-conversion" {
            "operation" => Code("convert-u64"), "branching" => Unsigned(*branching as u128),
            "maximum" => Unsigned(u128::from(u64::MAX)), "cause" => Cause(cause),
        },
        DegreeConversion { layer, degree, cause } => "hnsw-arithmetic-degree-conversion" {
            "operation" => Code("convert-u64"), "layer" => Unsigned(u128::from(*layer)),
            "degree" => Unsigned(*degree as u128), "maximum" => Unsigned(u128::from(u64::MAX)), "cause" => Cause(cause),
        },
        LayerEdges { layer, population, degree } => "hnsw-arithmetic-layer-edges" {
            "operation" => Code("multiply"), "layer" => Unsigned(u128::from(*layer)),
            "population" => Unsigned(u128::from(*population)), "degree" => Unsigned(u128::from(*degree)),
            "maximum" => Unsigned(u128::from(u64::MAX)),
        },
        AccumulatedEdges { layer, accumulated, next } => "hnsw-arithmetic-accumulated-edges" {
            "operation" => Code("add"), "layer" => Unsigned(u128::from(*layer)),
            "accumulated" => Unsigned(u128::from(*accumulated)), "next" => Unsigned(u128::from(*next)),
            "maximum" => Unsigned(u128::from(u64::MAX)),
        },
        EdgeBytes { edges, edge_bytes } => "hnsw-arithmetic-edge-bytes" {
            "operation" => Code("multiply"), "edges" => Unsigned(u128::from(*edges)),
            "edge_bytes" => ByteCount(u128::from(*edge_bytes)), "maximum" => ByteCount(u128::from(u64::MAX)),
        },
        VectorAndNormBytes { vectors, norms } => "hnsw-arithmetic-vector-norm-sum" {
            "operation" => Code("add"), "vectors" => ByteCount(u128::from(*vectors)),
            "norms" => ByteCount(u128::from(*norms)), "maximum" => ByteCount(u128::from(u64::MAX)),
        },
        WithNodeBytes { accumulated, nodes } => "hnsw-arithmetic-node-sum" {
            "operation" => Code("add"), "accumulated" => ByteCount(u128::from(*accumulated)),
            "nodes" => ByteCount(u128::from(*nodes)), "maximum" => ByteCount(u128::from(u64::MAX)),
        },
        WithEdgeBytes { accumulated, edges } => "hnsw-arithmetic-edge-sum" {
            "operation" => Code("add"), "accumulated" => ByteCount(u128::from(*accumulated)),
            "edges" => ByteCount(u128::from(*edges)), "maximum" => ByteCount(u128::from(u64::MAX)),
        },
    }
}

impl core::fmt::Display for ArithmeticRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("index arithmetic overflowed")
    }
}

impl core::error::Error for ArithmeticRefusal {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::RowsConversion { cause, .. }
            | Self::DimensionsConversion { cause, .. }
            | Self::BranchingConversion { cause, .. }
            | Self::DegreeConversion { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
