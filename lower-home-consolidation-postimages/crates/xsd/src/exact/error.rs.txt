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
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidLexical { .. } => ErrorCode::Forg0001,
            Self::DivisionByZero => ErrorCode::Foar0001,
            Self::NotFinite => ErrorCode::Foca0002,
            Self::OutOfRange { target, .. } => match target {
                BoundedTarget::I128 | BoundedTarget::I64 => ErrorCode::Foca0003,
                BoundedTarget::BoundedDecimal => ErrorCode::Foca0001,
                BoundedTarget::BoundedRational => ErrorCode::Foar0002,
            },
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

/// Native parse classification separate from physical destination refusal.
/// It borrows no owned lexical diagnostic, so failure reporting can remain
/// allocation-free after an operational workspace refuses storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactParseError {
    /// Invalid lexical text; a resident public parser may attach its text.
    InvalidLexical {
        /// Value-space lexical kind.
        kind: ExactKind,
        /// Stable lexical reason.
        reason: &'static str,
    },
    /// Original decimal lexical scale exceeds its exact representation.
    ScaleOverflow,
    /// Native physical destination could not be allocated.
    Storage(crate::bigint::LimbScratchError),
}

impl ExactParseError {
    /// Value-space F&O code; physical storage refusal has none.
    #[must_use]
    pub const fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::InvalidLexical { .. } => Some(ErrorCode::Forg0001),
            Self::ScaleOverflow => Some(ErrorCode::Foar0002),
            Self::Storage(_) => None,
        }
    }

    pub(crate) fn into_value_error(
        self,
        lexical: &str,
    ) -> Result<ExactError, crate::bigint::LimbScratchError> {
        match self {
            Self::InvalidLexical { kind, reason } => Ok(ExactError::invalid(lexical, kind, reason)),
            Self::ScaleOverflow => Ok(ExactError::ScaleOverflow),
            Self::Storage(error) => Err(error),
        }
    }
}

impl fmt::Display for ExactParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLexical { kind, reason } => {
                write!(formatter, "invalid {} lexical: {reason}", kind.name())
            }
            Self::ScaleOverflow => formatter.write_str("decimal scale exceeds u32::MAX digits"),
            Self::Storage(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for ExactParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}

/// A fallible native operation's semantic error or physical destination refusal.
/// Physical errors do not acquire an F&O code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactOperationError {
    /// Existing exact value-space/domain failure.
    Value(ExactError),
    /// Checked native destination/allocation refusal.
    Storage(crate::bigint::LimbScratchError),
}

impl ExactOperationError {
    /// Existing F&O code, absent for physical storage failure.
    #[must_use]
    pub const fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::Value(value) => Some(value.code()),
            Self::Storage(_) => None,
        }
    }
    pub(crate) fn into_value_error(self) -> Result<ExactError, crate::bigint::LimbScratchError> {
        match self {
            Self::Value(value) => Ok(value),
            Self::Storage(error) => Err(error),
        }
    }
}
purrdf_lex::variant_from!(ExactOperationError {
    Value(ExactError),
    Storage(crate::bigint::LimbScratchError),
});
impl fmt::Display for ExactOperationError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(value) => fmt::Display::fmt(value, out),
            Self::Storage(error) => fmt::Display::fmt(error, out),
        }
    }
}
impl std::error::Error for ExactOperationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Value(value) => Some(value),
            Self::Storage(error) => Some(error),
        }
    }
}

// Each exact datatype keeps its own lexical reader. Both public entry protocols
// select the same original destination policy and preserve their error channels.
macro_rules! lexical_entry {
    (
        $type:ty;
        try_from_lexical { $(#[$native:meta])* }
        from_str { $(#[$resident:meta])* }
    ) => {
        impl $type {
            $(#[$native])*
            pub fn try_from_lexical(lexical: &str) -> Result<Self, $crate::exact::ExactParseError> {
                Self::parse_with_storage(lexical, &$crate::bigint::scratch::Fallible)
            }
        }
        impl ::std::str::FromStr for $type {
            type Err = $crate::exact::ExactError;
            $(#[$resident])*
            fn from_str(lexical: &str) -> Result<Self, $crate::exact::ExactError> {
                Self::parse_with_storage(lexical, &$crate::bigint::scratch::Unbounded)
                    .map_err(|error| error.into_value_error(lexical)
                        .expect("unbounded integer storage"))
            }
        }
    };
}
pub(crate) use lexical_entry;
