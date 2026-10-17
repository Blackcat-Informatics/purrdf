// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native space selection and complete row-binding refusals.

use crate::{EvalError, NativeDiagnosticKind, RetainedNativeFailure, WorkspaceCapability};
use purrdf_core::{
    DistanceMetric, FamilyId, IndexId, MatrixId, ProjectionId, TargetId, TargetSetId, TermValue,
    VectorSpaceId,
};

/// Validated projection of the original immutable, admitted numeric owner.
#[derive(Debug)]
pub(super) struct CountValue(crate::parsed_value::ParsedValue);

impl CountValue {
    pub(super) fn new(value: crate::parsed_value::ParsedValue) -> Option<Self> {
        matches!(&*value, purrdf_xsd::XsdValue::Integer { .. }).then_some(Self(value))
    }

    pub(super) fn value(&self) -> &purrdf_xsd::exact::Integer {
        // Only `new` can construct this projection, and ParsedValue exposes no
        // mutation of its retained value. This does not reparse or clone it.
        let purrdf_xsd::XsdValue::Integer { value, .. } = &*self.0 else {
            unreachable!("a validated immutable integer owner changed its value kind")
        };
        value
    }
}

impl purrdf_hash::error::StructuredError for CountValue {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        let Self(owner) = self;
        visitor.variant("knn-integer-count");
        visitor.value(
            Some("value"),
            purrdf_hash::error::ErrorValue::Record(self.value()),
        );
        visitor.value(
            Some("datatype"),
            purrdf_hash::error::ErrorValue::Iri(owner.datatype().iri()),
        );
    }
}

impl core::fmt::Display for CountValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.value(), f)
    }
}

#[derive(Debug)]
pub(super) enum InvocationRefusal {
    Arity {
        relation: purrdf_lex::allocation::SharedText,
        expected: crate::PfArity,
        supplied: crate::PfArity,
    },
    FreeQuery,
    FreeCount,
    InvalidCount {
        actual: crate::WorkspaceTerm,
    },
    NegativeCount {
        count: CountValue,
    },
    CountLimit {
        count: CountValue,
        maximum: u64,
    },
    CountIndex {
        count: CountValue,
    },
    MissingRankedRow {
        row: usize,
    },
    DistanceRange {
        query_row: usize,
        row: usize,
        metric: DistanceMetric,
    },
    RowsShape {
        values: usize,
        norms: usize,
        rows: usize,
        components: usize,
    },
}

impl InvocationRefusal {
    pub(super) fn error(self, workspace: &WorkspaceCapability) -> EvalError {
        let kind = match &self {
            Self::Arity { .. }
            | Self::FreeQuery
            | Self::FreeCount
            | Self::InvalidCount { .. }
            | Self::NegativeCount { .. }
            | Self::CountLimit { .. }
            | Self::CountIndex { .. } => NativeDiagnosticKind::Function,
            Self::MissingRankedRow { .. } | Self::DistanceRange { .. } => {
                NativeDiagnosticKind::Data
            }
            Self::RowsShape { .. } => NativeDiagnosticKind::Internal,
        };
        RetainedNativeFailure::error(kind, self, workspace)
    }
}

impl purrdf_hash::error::StructuredError for InvocationRefusal {
    purrdf_hash::error_visit! {
        Arity { relation, expected, supplied } => "knn-invocation-arity" {
            "relation" => Text(relation.as_str()), "expected" => Record(expected), "supplied" => Record(supplied),
        },
        FreeQuery => "knn-free-query" {
            "position" => Unsigned(super::KNN_QUERY as u128), "search_mode" => Code(super::KNN_MODE),
            "membership_mode" => Code(super::KNN_MEMBERSHIP_MODE),
        },
        FreeCount => "knn-free-count" {
            "position" => Unsigned(super::KNN_COUNT as u128), "neighbour_position" => Unsigned(super::KNN_NEIGHBOUR as u128),
            "membership_mode" => Code(super::KNN_MEMBERSHIP_MODE),
        },
        InvalidCount { actual } => "knn-invalid-count" { "actual" => Term(&**actual), "position" => Unsigned(super::KNN_COUNT as u128) },
        NegativeCount { count } => "knn-negative-count" { "count" => Record(count), "position" => Unsigned(super::KNN_COUNT as u128) },
        CountLimit { count, maximum } => "knn-count-limit" { "count" => Record(count), "maximum" => Unsigned(u128::from(*maximum)) },
        CountIndex { count } => "knn-count-index" { "count" => Record(count), "maximum" => Unsigned(usize::MAX as u128) },
        MissingRankedRow { row } => "knn-missing-ranked-row" { "row" => Unsigned(*row as u128) },
        DistanceRange { query_row, row, metric } => "knn-distance-range" {
            "query_row" => Unsigned(*query_row as u128), "row" => Unsigned(*row as u128), "metric" => Record(metric),
        },
        RowsShape { values, norms, rows, components } => "knn-rows-shape" {
            "values" => Unsigned(*values as u128), "norms" => Unsigned(*norms as u128),
            "rows" => Unsigned(*rows as u128), "components" => Unsigned(*components as u128),
        },
    }
}

impl std::error::Error for InvocationRefusal {}

impl core::fmt::Display for InvocationRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use super::{KNN_COUNT, KNN_MEMBERSHIP_MODE, KNN_MODE, KNN_NEIGHBOUR, KNN_QUERY};
        match self {
            Self::Arity {
                relation,
                expected,
                supplied,
            } => write!(
                f,
                "{relation} expects {expected} argument(s), got {supplied}"
            ),
            Self::FreeQuery => write!(
                f,
                "the query term at position {KNN_QUERY} is free; this relation retrieves the \
                 neighbours of a seed and cannot enumerate seeds, which is why both of its \
                 declared modes — `{KNN_MODE}` and `{KNN_MEMBERSHIP_MODE}` — demand it"
            ),
            Self::FreeCount => write!(
                f,
                "the neighbour count at position {KNN_COUNT} is free and so is the \
                     neighbour at position {KNN_NEIGHBOUR}; how many neighbours to retrieve \
                     is a question this relation is asked, not one it answers, so the only \
                     call it serves without a count is the membership lookup \
                     `{KNN_MEMBERSHIP_MODE}`, which names the one term it is about"
            ),
            Self::InvalidCount { .. } => write!(
                f,
                "the neighbour count at position {KNN_COUNT} is not an integer literal; \
             there is no number of neighbours that names"
            ),
            Self::NegativeCount { count } => write!(
                f,
                "the neighbour count at position {KNN_COUNT} is {count}; a search cannot return a \
             negative number of neighbours"
            ),
            Self::CountLimit { count, maximum } => write!(
                f,
                "the invocation asks for {count} neighbour(s), and the configured guard admits at \
             most {maximum}; returning the {maximum} nearest instead would be a short answer \
             reported as a complete one, so the request is refused rather than clamped"
            ),
            Self::CountIndex { count } => write!(
                f,
                "the neighbour count {count} does not fit this platform's index range"
            ),
            Self::MissingRankedRow { row } => write!(
                f,
                "the ranking named row {row}, which the space does not hold"
            ),
            Self::DistanceRange {
                query_row,
                row,
                metric,
            } => write!(
                f,
                "the distance from row {query_row} to row {row} left the finite range \
                 under {metric:?}; the artifact's magnitudes cannot be ranked under this metric"
            ),
            Self::RowsShape {
                values,
                norms,
                rows,
                components,
            } => write!(
                f,
                "the space's {values} value(s) and {norms} norm(s) do not describe {rows} row(s) of {components} \
                     component(s)"
            ),
        }
    }
}

#[derive(Debug)]
pub(super) enum ConfigurationRefusal {
    MissingSpace {
        space: VectorSpaceId,
    },
    MissingFamily {
        space: VectorSpaceId,
        family: FamilyId,
    },
    ExtensionMetric {
        space: Option<VectorSpaceId>,
        metric: DistanceMetric,
    },
    MissingSet {
        set: TargetSetId,
    },
    MissingMatrix {
        set: TargetSetId,
        space: VectorSpaceId,
    },
    DuplicateTerm {
        term: TermValue,
    },
    UnknownTarget {
        target: TargetId,
    },
    DuplicateBinding {
        target: TargetId,
        row: usize,
    },
    MissingBinding {
        row: usize,
        target: Option<TargetId>,
        row_count: usize,
    },
    GuardMismatch {
        index: IndexId,
        set: TargetSetId,
        space: VectorSpaceId,
        bound_matrix: MatrixId,
        bound_projection: ProjectionId,
        matrix: MatrixId,
        projection: ProjectionId,
    },
}

impl ConfigurationRefusal {
    pub(super) fn error(self) -> EvalError {
        let kind = match &self {
            Self::MissingFamily { .. } | Self::GuardMismatch { .. } => NativeDiagnosticKind::Data,
            Self::MissingSpace { .. }
            | Self::ExtensionMetric { .. }
            | Self::MissingSet { .. }
            | Self::MissingMatrix { .. }
            | Self::DuplicateTerm { .. }
            | Self::UnknownTarget { .. }
            | Self::DuplicateBinding { .. }
            | Self::MissingBinding { .. } => NativeDiagnosticKind::Config,
        };
        RetainedNativeFailure::error(kind, self, &WorkspaceCapability::resident())
    }
}

impl purrdf_hash::error::StructuredError for ConfigurationRefusal {
    purrdf_hash::error_visit! {
        MissingSpace { space } => "knn-missing-space" { "space" => Record(space) },
        MissingFamily { space, family } => "knn-missing-family" {
            "space" => Record(space), "family" => Record(family),
        },
        ExtensionMetric { space, metric } => "knn-extension-metric" {
            "space" => Value(space.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |id| purrdf_hash::error::ErrorValue::Record(id))),
            "metric" => Record(metric),
        },
        MissingSet { set } => "knn-missing-target-set" { "set" => Record(set) },
        MissingMatrix { set, space } => "knn-missing-matrix" {
            "set" => Record(set), "space" => Record(space),
        },
        DuplicateTerm { term } => "knn-duplicate-term" { "term" => Term(term) },
        UnknownTarget { target } => "knn-unknown-target" { "target" => Record(target) },
        DuplicateBinding { target, row } => "knn-duplicate-binding" {
            "target" => Record(target), "row" => Unsigned(*row as u128),
        },
        MissingBinding { row, target, row_count } => "knn-missing-binding" {
            "row" => Unsigned(*row as u128), "row_count" => Unsigned(*row_count as u128),
            "target" => Value(target.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |id| purrdf_hash::error::ErrorValue::Record(id))),
        },
        GuardMismatch { index, set, space, bound_matrix, bound_projection, matrix, projection } => "knn-index-guard-mismatch" {
            "index" => Record(index), "set" => Record(set), "space" => Record(space),
            "bound_matrix" => Record(bound_matrix), "bound_projection" => Record(bound_projection),
            "matrix" => Record(matrix), "projection" => Record(projection),
        },
    }
}

impl std::error::Error for ConfigurationRefusal {}

impl core::fmt::Display for ConfigurationRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingSpace { space } => {
                write!(f, "the artifact declares no vector space {space}")
            }
            Self::MissingFamily { space, family } => write!(
                f,
                "vector space {space} names family {family} , which the artifact does not hold"
            ),
            Self::ExtensionMetric { space, metric } => {
                if let Some(space) = space {
                    write!(
                        f,
                        "vector space {space} declares the caller-defined distance metric \
                        {metric:?}, whose parameters are opaque bytes this engine cannot evaluate; \
                        only the three built-in PURREMB metrics can be ranked here"
                    )
                } else {
                    write!(
                        f,
                        "the caller-defined distance metric {metric:?} carries parameters as opaque \
                        bytes this engine cannot evaluate; only the three built-in PURREMB metrics \
                        can be ranked here"
                    )
                }
            }
            Self::MissingSet { set } => write!(f, "the artifact declares no target set {set}"),
            Self::MissingMatrix { set, space } => write!(
                f,
                "the artifact holds no matrix joining target set {set} to vector space {space}"
            ),
            Self::DuplicateTerm { term } => write!(
                f,
                "the term {term:?} is bound to two different rows; a query seed names a term, so \
                 a term claimed by two rows makes the seed ambiguous"
            ),
            Self::UnknownTarget { target } => write!(
                f,
                "the binding for target {target} names a target this space's target set does not hold"
            ),
            Self::DuplicateBinding { target, row } => write!(
                f,
                "target {target} (row {row}) is bound twice; a row stands for exactly one RDF term"
            ),
            Self::MissingBinding {
                row,
                target,
                row_count,
            } => {
                write!(f, "row {row} (target ")?;
                if let Some(target) = target {
                    core::fmt::Display::fmt(target, f)?;
                } else {
                    f.write_str("<unknown>")?;
                }
                write!(
                    f,
                    ") has no bound RDF term; a space with an unnamed row \
                     would search {row_count} candidates and be able to report only some of \
                     them, so the top-k it returned would silently be the top-k of a subset"
                )
            }
            Self::GuardMismatch {
                index,
                set,
                space,
                bound_matrix,
                bound_projection,
                matrix,
                projection,
            } => write!(
                f,
                "derived index {index} names target set {set} and vector space {space} but binds \
                 matrix {bound_matrix} / projection {bound_projection}, where this space scans \
                 matrix {matrix} / projection {projection}; the guard is stale or substituted"
            ),
        }
    }
}
