// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original encoded acquisition failures and their source coordinates.

use super::{Extent, RdfDiagnostic, StorageError, StreamError};

/// Original encoded input failure; compatibility diagnostics remain unchanged.
#[derive(Debug)]
pub enum EncodedReadError<C, O> {
    /// Original strict codec/control/output error.
    Codec(StreamError<C, O>),
    /// Original progress/cancellation failure outside scalar conversion.
    Control(C),
    /// Original memory admission or allocator failure.
    Storage(StorageError),
    /// Original reader error before another source scalar is consumed.
    Read {
        /// Unchanged I/O error and its original native source chain.
        cause: std::io::Error,
        /// Original byte offset of the next read window.
        offset: u64,
    },
    /// A reader violated its slice count contract.
    ReadCount {
        /// Returned count.
        actual: usize,
        /// Supplied slice width.
        maximum: usize,
    },
    /// Explicit conversion workspace or line ceiling, before allocation.
    Limit {
        /// Bounded resource.
        resource: &'static str,
        /// Required bytes.
        required: u64,
        /// Inclusive ceiling.
        limit: u64,
    },
    /// Unchanged native parser diagnostic, with original source contribution.
    Parse {
        /// Original diagnostic, with its native location/presentation intact.
        diagnostic: RdfDiagnostic,
        /// Exact original scalar extent, or an exact end boundary, when located.
        source: Option<Extent>,
    },
}

purrdf_lex::variant_from!(impl<C,O> EncodedReadError<C,O> { Storage(StorageError) });

impl<C: core::fmt::Display, O: core::fmt::Display> core::fmt::Display for EncodedReadError<C, O> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Codec(cause) => cause.fmt(f),
            Self::Control(cause) => cause.fmt(f),
            Self::Storage(cause) => cause.fmt(f),
            Self::Read { cause, offset } => {
                write!(f, "encoded source read at byte {offset}: {cause}")
            }
            Self::ReadCount { actual, maximum } => write!(
                f,
                "reader returned {actual} bytes for a {maximum}-byte window"
            ),
            Self::Limit {
                resource,
                required,
                limit,
            } => write!(f, "{resource} needs {required} bytes, limit {limit}"),
            Self::Parse { diagnostic, .. } => diagnostic.fmt(f),
        }
    }
}
impl<C: std::error::Error + 'static, O: std::error::Error + 'static> std::error::Error
    for EncodedReadError<C, O>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec(cause) => Some(cause),
            Self::Control(cause) => Some(cause),
            Self::Storage(cause) => Some(cause),
            Self::Read { cause, .. } => Some(cause),
            Self::Parse { diagnostic, .. } => Some(diagnostic),
            _ => None,
        }
    }
}
impl<C: purrdf_hash::error::StructuredError, O: purrdf_hash::error::StructuredError>
    purrdf_hash::error::StructuredError for EncodedReadError<C, O>
{
    purrdf_hash::error_visit! {
        Codec(cause)=>"encoded-read-codec" {"cause"=>Record(cause)},
        Control(cause)=>"encoded-read-control" {"cause"=>Record(cause)},
        Storage(cause)=>"encoded-read-storage" {"cause"=>Cause(cause)},
        Read {cause,offset}=>"encoded-read-io" {"cause"=>Cause(cause),"offset"=>ByteCount(u128::from(*offset))},
        ReadCount {actual,maximum}=>"encoded-read-count" {"actual"=>Unsigned(*actual as u128),"maximum"=>Unsigned(*maximum as u128)},
        Limit {resource,required,limit}=>"encoded-read-limit" {"resource"=>Code(resource),"required"=>ByteCount(u128::from(*required)),"limit"=>ByteCount(u128::from(*limit))},
        Parse {diagnostic,source}=>"encoded-read-parse" {
            "cause"=>Cause(diagnostic),
            "start"=>Value(source.map_or(purrdf_hash::error::ErrorValue::Absent,|range|purrdf_hash::error::ErrorValue::ByteCount(u128::from(range.start)))),
            "end"=>Value(source.map_or(purrdf_hash::error::ErrorValue::Absent,|range|purrdf_hash::error::ErrorValue::ByteCount(u128::from(range.end))))
        },
    }
}
