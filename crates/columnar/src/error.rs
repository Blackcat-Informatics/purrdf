// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed failures from the narrow columnar codec.

use std::fmt;

/// A fail-closed Parquet encoding, decoding, or RDF reconstruction error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnarError {
    /// Input ended before a required byte range was available.
    Truncated {
        /// Structure being decoded.
        context: &'static str,
        /// Exclusive byte position or length required.
        needed: usize,
        /// Bytes available in the enclosing buffer.
        available: usize,
    },
    /// Bytes were present but violated the fixed schema or encoding contract.
    Malformed {
        /// Structure being decoded or constructed.
        context: &'static str,
        /// Stable diagnostic detail.
        detail: String,
    },
    /// A valid Parquet construct falls outside PurRDF's deliberately narrow profile.
    Unsupported {
        /// Unsupported field or construct.
        context: &'static str,
        /// Observed numeric discriminator.
        value: i64,
    },
    /// A count or byte length exceeds a representable or safety bound.
    LimitExceeded {
        /// Bounded structure.
        context: &'static str,
        /// Observed value.
        value: u64,
        /// Maximum accepted value.
        maximum: u64,
    },
}

impl ColumnarError {
    pub(crate) fn malformed(context: &'static str, detail: impl Into<String>) -> Self {
        Self::Malformed {
            context,
            detail: detail.into(),
        }
    }

    pub(crate) fn truncated(context: &'static str, needed: usize, available: usize) -> Self {
        Self::Truncated {
            context,
            needed,
            available,
        }
    }

    pub(crate) fn limit(context: &'static str, value: usize, maximum: usize) -> Self {
        Self::LimitExceeded {
            context,
            value: value as u64,
            maximum: maximum as u64,
        }
    }
}

impl fmt::Display for ColumnarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated {
                context,
                needed,
                available,
            } => write!(
                f,
                "columnar {context} is truncated: needs {needed} bytes, has {available}"
            ),
            Self::Malformed { context, detail } => {
                write!(f, "columnar {context} is malformed: {detail}")
            }
            Self::Unsupported { context, value } => {
                write!(f, "columnar {context} value {value} is unsupported")
            }
            Self::LimitExceeded {
                context,
                value,
                maximum,
            } => write!(
                f,
                "columnar {context} value {value} exceeds maximum {maximum}"
            ),
        }
    }
}

impl std::error::Error for ColumnarError {}
