// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native in-memory relation refusals, retained before presentation.

use super::PfArity;
use crate::workspace::{WorkspaceCapability, WorkspaceTerm};
use crate::{EvalError, NativeDiagnosticKind, RetainedNativeFailure};
use purrdf_core::collections::RdfListError;

#[derive(Debug)]
pub(super) enum Refusal {
    ArityOverflow {
        arity: PfArity,
    },
    ArityCapacity {
        arity: PfArity,
        width: usize,
        limit: usize,
    },
    ConfigRow {
        index: usize,
        actual: usize,
        arity: PfArity,
    },
    MissingHead {
        head: WorkspaceTerm,
    },
    TableList {
        cause: RdfListError,
    },
    RowList {
        index: usize,
        cause: RdfListError,
    },
    GraphRow {
        index: usize,
        actual: usize,
        arity: PfArity,
    },
    Invocation {
        expected: PfArity,
        actual: PfArity,
    },
}

pub(super) fn width(arity: PfArity, workspace: &WorkspaceCapability) -> Result<usize, EvalError> {
    let width = arity.subject.checked_add(arity.object).ok_or_else(|| {
        RetainedNativeFailure::error(
            NativeDiagnosticKind::Config,
            Refusal::ArityOverflow { arity },
            workspace,
        )
    })?;
    let limit = purrdf_core::binding_pattern::BindingPattern::MAX_ARITY;
    if width > limit {
        return Err(RetainedNativeFailure::error(
            NativeDiagnosticKind::Config,
            Refusal::ArityCapacity {
                arity,
                width,
                limit,
            },
            workspace,
        ));
    }
    Ok(width)
}

impl purrdf_hash::error::StructuredError for Refusal {
    purrdf_hash::error_visit! {
        ArityOverflow { arity } => "property-memory-arity-overflow" {
            "arity" => Record(arity),
            "requested" => Unsigned(arity.subject as u128 + arity.object as u128),
            "limit" => Unsigned(usize::MAX as u128),
        },
        ArityCapacity { arity, width, limit } => "property-memory-arity-capacity" {
            "arity" => Record(arity), "requested" => Unsigned(*width as u128),
            "limit" => Unsigned(*limit as u128),
        },
        ConfigRow { index, actual, arity } => "property-memory-config-row" {
            "index" => Unsigned(*index as u128), "actual" => Unsigned(*actual as u128),
            "arity" => Record(arity), "required" => Unsigned(arity.subject as u128 + arity.object as u128),
        },
        MissingHead { head } => "property-memory-missing-head" { "head" => Term(&**head) },
        TableList { cause } => "property-memory-table-list" { "cause" => Cause(cause) },
        RowList { index, cause } => "property-memory-row-list" {
            "index" => Unsigned(*index as u128), "cause" => Cause(cause),
        },
        GraphRow { index, actual, arity } => "property-memory-graph-row" {
            "index" => Unsigned(*index as u128), "actual" => Unsigned(*actual as u128),
            "arity" => Record(arity), "required" => Unsigned(arity.subject as u128 + arity.object as u128),
        },
        Invocation { expected, actual } => "property-memory-invocation" {
            "expected" => Record(expected), "actual" => Record(actual),
        },
    }
}

impl core::fmt::Display for Refusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ArityOverflow { arity } => write!(
                f,
                "in-memory property-function arity ({arity}) exceeds the host usize width"
            ),
            Self::ArityCapacity {
                arity,
                width,
                limit,
            } => write!(
                f,
                "in-memory property-function arity ({arity}) requires {width} positions; binding patterns support at most {limit}"
            ),
            Self::ConfigRow {
                index,
                actual,
                arity,
            } => write!(
                f,
                "in-memory property-function row {index} has {actual} value(s); the declared arity ({arity}) requires {}",
                arity.total()
            ),
            Self::MissingHead { head } => write!(
                f,
                "property-function table head {:?} is not present in the dataset",
                **head
            ),
            Self::TableList { cause } => {
                write!(f, "property-function table is not an rdf:List: {cause}")
            }
            Self::RowList { index, cause } => write!(
                f,
                "property-function table row {index} is not an rdf:List: {cause}"
            ),
            Self::GraphRow {
                index,
                actual,
                arity,
            } => write!(
                f,
                "property-function table row {index} has {actual} value(s); the declared arity ({arity}) requires {}",
                arity.total()
            ),
            Self::Invocation { expected, actual } => write!(
                f,
                "in-memory property function expects {expected} argument(s), got {actual}"
            ),
        }
    }
}

impl std::error::Error for Refusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TableList { cause } | Self::RowList { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
