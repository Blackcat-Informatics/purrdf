// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The typed failures of the exact tower, each carrying the XPath F&O 3.1 error
//! code a query surface reports for it.

use std::fmt;

use crate::value::ErrorCode;

/// Which exact value space a lexical form, a conversion or a refusal concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExactKind {
    /// `xsd:integer`, unbounded ([`crate::exact::Integer`]).
    Integer,
    /// `xsd:decimal`, unbounded ([`crate::exact::Decimal`]).
    Decimal,
    /// `owl:rational`, unbounded ([`crate::exact::Rational`]).
    Rational,
}

impl ExactKind {
    /// The datatype's prefixed name, for messages.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Integer => "xsd:integer",
            Self::Decimal => "xsd:decimal",
            Self::Rational => "owl:rational",
        }
    }
}

/// The machine-word representation a narrowing conversion was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BoundedTarget {
    /// A Rust `i128`.
    I128,
    /// A Rust `i64`.
    I64,
}

impl BoundedTarget {
    /// The representation's name, for messages.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::I128 => "i128",
            Self::I64 => "i64",
        }
    }
}

/// A refusal from the exact tower. Every variant is a typed error, never a
/// rounded or wrapped stand-in for the value that could not be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExactError {
    /// The text is not in the value space's lexical space (`err:FORG0001`).
    InvalidLexical {
        /// The lexical space the text was read against.
        kind: ExactKind,
        /// The offending text.
        lexical: String,
        /// A short, stable explanation.
        reason: &'static str,
    },
    /// An exact division, integer division, modulus or rational construction by
    /// zero (`err:FOAR0001`).
    DivisionByZero,
    /// `NaN` or an infinity has no exact value (`err:FOCA0002`, the F&O cast rule
    /// for a float or double source).
    NotFinite,
    /// The exact value does not fit the bounded representation asked for: an
    /// integer target is `err:FOCA0003`. Narrowing never wraps, saturates or rounds.
    OutOfRange {
        /// The representation that cannot hold the value.
        target: BoundedTarget,
        /// A short, stable explanation of which bound was exceeded.
        reason: &'static str,
    },
    /// [`crate::exact::DivisionPolicy::Exact`] was asked for a quotient with no
    /// finite decimal expansion, such as `1 / 3` (`err:FOAR0002`: the exact result
    /// is not representable as a decimal under the caller's policy).
    NonTerminating,
    /// A decimal scale or exponent beyond `u32::MAX` digits (`err:FOAR0002`). No
    /// such value fits in memory; the refusal keeps the scale arithmetic exact.
    ScaleOverflow,
}

impl ExactError {
    /// The XPath F&O 3.1 error code a query surface reports for this refusal.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidLexical { .. } => ErrorCode::Forg0001,
            Self::DivisionByZero => ErrorCode::Foar0001,
            Self::NotFinite => ErrorCode::Foca0002,
            Self::OutOfRange { .. } => ErrorCode::Foca0003,
            Self::NonTerminating | Self::ScaleOverflow => ErrorCode::Foar0002,
        }
    }

    /// [`Self::OutOfRange`] for `target`.
    pub(crate) const fn out_of_range(target: BoundedTarget, reason: &'static str) -> Self {
        Self::OutOfRange { target, reason }
    }
}

purrdf_lex::constructors! {
    impl ExactError {
        /// [`Self::InvalidLexical`] for `lexical` read as `kind`.
        pub(crate) fn invalid(lexical, kind: ExactKind, reason: &'static str) -> Self::InvalidLexical { .. };
    }
}

impl fmt::Display for ExactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLexical {
                kind,
                lexical,
                reason,
            } => write!(
                f,
                "invalid lexical form {lexical:?} for {}: {reason}",
                kind.name()
            ),
            Self::DivisionByZero => f.write_str("exact division by zero"),
            Self::NotFinite => f.write_str("NaN or an infinity has no exact value"),
            Self::OutOfRange { target, reason } => {
                write!(f, "value does not fit {}: {reason}", target.name())
            }
            Self::NonTerminating => {
                f.write_str("the exact quotient has no finite decimal expansion")
            }
            Self::ScaleOverflow => f.write_str("decimal scale exceeds u32::MAX digits"),
        }
    }
}

impl std::error::Error for ExactError {}
