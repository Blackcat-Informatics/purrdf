// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The typed failures of the exact tower, each carrying the XPath F&O 3.1 error
//! code a query surface reports for it.

use std::fmt;

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

/// The bounded representation a narrowing conversion was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BoundedTarget {
    /// A Rust `i128` — the machine-word `xsd:integer` value ([`crate::XsdValue::Integer`]).
    I128,
    /// A Rust `i64`.
    I64,
    /// The bounded [`crate::numeric::Decimal`]: an `i128` mantissa with at most
    /// eighteen fractional digits.
    BoundedDecimal,
    /// The bounded [`crate::rational::Rational`]: an `i128` numerator and
    /// denominator.
    BoundedRational,
}

impl BoundedTarget {
    /// The representation's name, for messages.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::I128 => "i128",
            Self::I64 => "i64",
            Self::BoundedDecimal => "the bounded xsd:decimal (i128 mantissa, scale <= 18)",
            Self::BoundedRational => "the bounded owl:rational (i128 numerator and denominator)",
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
    /// integer target is `err:FOCA0003`, the bounded decimal `err:FOCA0001`, the
    /// bounded rational `err:FOAR0002`. Narrowing never wraps, saturates or rounds.
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
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidLexical { .. } => "FORG0001",
            Self::DivisionByZero => "FOAR0001",
            Self::NotFinite => "FOCA0002",
            Self::OutOfRange { target, .. } => match target {
                BoundedTarget::I128 | BoundedTarget::I64 => "FOCA0003",
                BoundedTarget::BoundedDecimal => "FOCA0001",
                BoundedTarget::BoundedRational => "FOAR0002",
            },
            Self::NonTerminating | Self::ScaleOverflow => "FOAR0002",
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
