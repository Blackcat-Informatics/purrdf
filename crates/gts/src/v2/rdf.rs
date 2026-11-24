// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use std::collections::TryReserveError;

mod dictionary;
mod lines;
mod terms;
mod values;

pub use dictionary::{DefinitionFindings, RowDisposition, RowFindings, SegmentDictionary};
pub use lines::{Line, RdfFold};
pub use terms::{LiteralDirection, LiteralValue, Term, TermId};
pub use values::{ValueName, ValueStore};
pub(super) use values::{unit_digest, unit_hasher};

/// A resource refusal or inconsistent input supplied to the RDF fold.
#[derive(Debug)]
#[non_exhaustive]
pub enum FoldError {
    /// The allocator refused fold storage.
    Resource(TryReserveError),
    /// A reservation does not extend this dictionary.
    Reservation {
        /// The first unreserved ID.
        expected: u128,
        /// The declared first ID.
        start: u128,
        /// The declared exclusive end.
        end: u128,
    },
    /// The supplied definitions do not match the admitted term runs.
    Definitions,
    /// A supplied typed value has an inadmissible timestamp.
    Time(purrdf_xsd::rfc3339::FormatError),
    /// A rebuilt value does not match the declaration's digest and length.
    ValueClaim {
        /// The name claimed by the declaration.
        expected: ValueName,
        /// The name computed over the rebuilt bytes.
        actual: ValueName,
    },
    /// A term handle does not belong to the supplied fold.
    TermHandle(TermId),
    /// A term is incompatible with its RDF position.
    PositionConstraint(TermId),
    /// Two annotation rows of one frame are written alike.
    RepeatedAnnotation,
}

impl fmt::Display for FoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resource(error) => {
                write!(f, "could not reserve GTS v2 RDF fold storage ({error})")
            }
            Self::Reservation {
                expected,
                start,
                end,
            } => {
                write!(
                    f,
                    "GTS v2 term reservation {start}..{end} does not extend the dictionary at {expected}"
                )
            }
            Self::Definitions => {
                f.write_str("GTS v2 definitions do not match their admitted term runs")
            }
            Self::Time(error) => write!(f, "could not format GTS v2 typed timestamp ({error})"),
            Self::ValueClaim { expected, actual } => {
                write!(
                    f,
                    "GTS v2 value claim {expected} differs from rebuilt value {actual}"
                )
            }
            Self::TermHandle(id) => write!(f, "GTS v2 fold has no term with handle {}", id.index()),
            Self::RepeatedAnnotation => f.write_str("GTS v2 annotation frame repeats a row"),
            Self::PositionConstraint(id) => {
                write!(
                    f,
                    "GTS v2 term handle {} violates its position constraint",
                    id.index()
                )
            }
        }
    }
}

impl std::error::Error for FoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::Time(error) => Some(error),
            _ => None,
        }
    }
}

purrdf_lex::variant_from!(FoldError { Resource(TryReserveError) });

impl purrdf_hash::error::StructuredError for FoldError {
    purrdf_hash::error_visit! {
        Resource(cause) => "gts-v2-fold-resource" { "cause" => Cause(cause) },
        Reservation { expected, start, end } => "gts-v2-fold-reservation" {
            "expected" => Unsigned(*expected), "start" => Unsigned(*start), "end" => Unsigned(*end),
        },
        Definitions => "gts-v2-fold-definitions" {},
        Time(cause) => "gts-v2-fold-time" { "cause" => Cause(cause) },
        ValueClaim { expected, actual } => "gts-v2-fold-value-claim" {
            "expected" => Record(expected), "actual" => Record(actual),
        },
        TermHandle(id) => "gts-v2-fold-term-handle" { "index" => Unsigned(id.index() as u128) },
        PositionConstraint(id) => "gts-v2-fold-position" { "index" => Unsigned(id.index() as u128) },
        RepeatedAnnotation => "gts-v2-repeated-annotation" {},
    }
}
