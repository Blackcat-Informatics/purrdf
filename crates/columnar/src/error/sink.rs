// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original source, codec and destination failures for incremental exports.

use core::fmt;

use crate::{ColumnarError, ColumnarWriteError, Table};

/// A failed export can leave earlier complete files and an incomplete current
/// file in the caller's sinks. No later callback is invoked after a refusal.
#[derive(Debug)]
#[non_exhaustive]
pub enum ColumnarSinkError<S, E> {
    /// Original checked source-session or term-access failure.
    SourceRead(S),
    /// Original native projection, configuration or encoding failure.
    Codec(ColumnarError),
    /// Original destination failure; no formatting or cloning is required.
    Sink {
        /// File receiving the failed chunk.
        table: Table,
        /// Bytes in this file accepted by earlier successful callbacks.
        /// The failed callback may additionally have written a partial chunk.
        successful_bytes: u64,
        /// Length of the chunk offered to the failed callback.
        attempted_bytes: usize,
        /// The unchanged caller failure.
        cause: E,
    },
}

purrdf_lex::variant_from!(impl<S, E> ColumnarSinkError<S, E> { Codec(ColumnarError) });

impl<S, E> From<ColumnarWriteError<S>> for ColumnarSinkError<S, E> {
    fn from(error: ColumnarWriteError<S>) -> Self {
        match error {
            ColumnarWriteError::Codec(cause) => Self::Codec(cause),
            ColumnarWriteError::SourceRead(cause) => Self::SourceRead(cause),
        }
    }
}

impl<S> From<ColumnarSinkError<S, core::convert::Infallible>> for ColumnarWriteError<S> {
    fn from(error: ColumnarSinkError<S, core::convert::Infallible>) -> Self {
        match error {
            ColumnarSinkError::Codec(cause) => Self::Codec(cause),
            ColumnarSinkError::SourceRead(cause) => Self::SourceRead(cause),
            ColumnarSinkError::Sink { cause, .. } => match cause {},
        }
    }
}

impl<S: fmt::Display, E: fmt::Display> fmt::Display for ColumnarSinkError<S, E> {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceRead(cause) => write!(output, "columnar source read failed: {cause}"),
            Self::Codec(cause) => cause.fmt(output),
            Self::Sink {
                table,
                successful_bytes,
                cause,
                ..
            } => write!(
                output,
                "columnar {} sink failed after {successful_bytes} acknowledged bytes: {cause}",
                table.file_name()
            ),
        }
    }
}

impl<S: std::error::Error + 'static, E: std::error::Error + 'static> std::error::Error
    for ColumnarSinkError<S, E>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SourceRead(cause) => Some(cause),
            Self::Codec(cause) => Some(cause),
            Self::Sink { cause, .. } => Some(cause),
        }
    }
}

impl<S: purrdf_hash::error::StructuredError, E: purrdf_hash::error::StructuredError>
    purrdf_hash::error::StructuredError for ColumnarSinkError<S, E>
{
    purrdf_hash::error_visit! {
        SourceRead(cause) => "columnar-source-read" { "cause" => Cause(cause) },
        Codec(cause) => "columnar-write-codec" { "cause" => Cause(cause) },
        Sink { table, successful_bytes, attempted_bytes, cause } => "columnar-sink" {
            "table" => Code(table.name()),
            "successful_bytes" => ByteCount(u128::from(*successful_bytes)),
            "attempted_bytes" => ByteCount(*attempted_bytes as u128),
            "cause" => Cause(cause),
        },
    }
}
