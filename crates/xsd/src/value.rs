// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The [`XsdValue`] value type and the [`XsdError`] parse-failure type.
//!
//! `XsdValue` is a **value-space** representation: parsing maps a lexical form into
//! the abstract value it denotes. It is deliberately NOT a term-identity key —
//! parsing discards the lexical form, so `"1"^^xsd:integer` and `"01"^^xsd:integer`
//! both become [`XsdValue::Integer`] with `{ value: 1, datatype: Integer }` even though
//! they are DISTINCT RDF terms (`sameTerm` is false). RDF term identity (`sameTerm`)
//! is the IR's `(lexical, datatype, language)` tuple, NOT this type. Consequently
//! `XsdValue` intentionally implements neither `PartialEq`/`Eq`/`Hash` (which would
//! falsely read as term identity) nor `PartialOrd`/`Ord` (value ordering is the
//! partial `value_cmp` free fn). It implements only `Clone`/`Debug`, so a consumer
//! can cache `HashMap<TermId, XsdValue>` keyed by the IR's `TermId`.

use crate::datatype::XsdDatatype;
use crate::exact;
use crate::numeric::Decimal;
use crate::temporal;

/// A parsed XSD value (value space). Variants are added per datatype family across
/// the foundation tasks; numeric first.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum XsdValue {
    /// `xsd:integer` and all derived integer datatypes, for a value inside
    /// `i128`. A value outside `i128` is [`Self::BigInteger`].
    ///
    /// The `datatype` field carries the exact XSD derived type (e.g. `xsd:byte`,
    /// `xsd:unsignedLong`) so that `value_cmp` can distinguish types for cross-type
    /// equality while `value_cmp` still compares by value across integer subtypes
    /// (xsd:int 5 == xsd:long 5 per the SPARQL promotion rules).
    Integer {
        /// The parsed integer value.
        value: i128,
        /// The exact XSD datatype (Integer, Long, Byte, UnsignedLong, etc.).
        datatype: XsdDatatype,
    },
    /// `xsd:decimal` — exact fixed-point (`i128` mantissa + scale ≤ 18). A value
    /// that needs more digits is [`Self::BigDecimal`].
    Decimal(Decimal),
    /// `xsd:float` — IEEE single-precision.
    Float(f32),
    /// `xsd:double` — IEEE double-precision.
    Double(f64),
    /// `xsd:boolean`.
    Boolean(bool),
    /// `xsd:string` — the value space is the lexical space (no normalization).
    String(String),
    /// `xsd:dateTime`.
    DateTime(temporal::DateTime),
    /// `xsd:date`.
    Date(temporal::Date),
    /// `xsd:time`.
    Time(temporal::Time),
    /// `xsd:duration` and its `dayTimeDuration`/`yearMonthDuration` subtypes.
    Duration(temporal::Duration),
    /// `xsd:gYear`, `xsd:gMonth`, `xsd:gDay`, `xsd:gYearMonth`, `xsd:gMonthDay`.
    Gregorian(temporal::Gregorian),
    /// `xsd:hexBinary` and `xsd:base64Binary` — a byte sequence.
    ///
    /// The `datatype` field distinguishes the two value spaces: even though the
    /// underlying representation is bytes in both cases, `hexBinary` and
    /// `base64Binary` are DIFFERENT value spaces and their values are INCOMPARABLE.
    Binary {
        /// The decoded byte sequence.
        bytes: Vec<u8>,
        /// Must be [`XsdDatatype::HexBinary`] or [`XsdDatatype::Base64Binary`].
        datatype: XsdDatatype,
    },
    /// `xsd:integer` or an unbounded derived integer datatype (`nonNegativeInteger`
    /// and its relatives) whose value lies outside `i128`, held exactly.
    ///
    /// [`parse`] and every arithmetic result produce this variant only for a value
    /// [`Self::Integer`] cannot hold; [`XsdValue::from_exact_integer`] applies the
    /// same split. Every operation of this crate accepts either variant for any
    /// value, so a value built here by hand inside `i128` still computes and
    /// compares exactly.
    BigInteger {
        /// The exact value.
        value: exact::Integer,
        /// The exact XSD datatype.
        datatype: XsdDatatype,
    },
    /// `xsd:decimal` whose canonical form needs more than eighteen fractional
    /// digits or a coefficient outside `i128`, held exactly.
    ///
    /// [`parse`] and every arithmetic result produce this variant only for a value
    /// [`Self::Decimal`] cannot hold; [`XsdValue::from_exact_decimal`] applies the
    /// same split.
    BigDecimal(exact::Decimal),
}

impl XsdValue {
    /// The XSD datatype this value belongs to.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::{XsdDatatype, parse};
    ///
    /// // Derived integer values keep their exact datatype.
    /// let v = parse("5", XsdDatatype::Byte)?;
    /// assert_eq!(v.datatype(), XsdDatatype::Byte);
    /// # Ok::<(), purrdf_xsd::XsdError>(())
    /// ```
    #[must_use]
    pub fn datatype(&self) -> XsdDatatype {
        match self {
            Self::Integer { datatype, .. } => *datatype,
            Self::Decimal(_) => XsdDatatype::Decimal,
            Self::Float(_) => XsdDatatype::Float,
            Self::Double(_) => XsdDatatype::Double,
            Self::Boolean(_) => XsdDatatype::Boolean,
            Self::String(_) => XsdDatatype::String,
            Self::DateTime(_) => XsdDatatype::DateTime,
            Self::Date(_) => XsdDatatype::Date,
            Self::Time(_) => XsdDatatype::Time,
            Self::Duration(d) => d.datatype(),
            Self::Gregorian(g) => g.datatype(),
            Self::Binary { datatype, .. } | Self::BigInteger { datatype, .. } => *datatype,
            Self::BigDecimal(_) => XsdDatatype::Decimal,
        }
    }

    /// Whether the value is in the SPARQL numeric tower (SPARQL 1.1 §17.1,
    /// "numeric"): an integer of any integer-family datatype, a decimal, a float
    /// or a double. Booleans, strings, temporal and binary values are not.
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Integer { .. }
                | Self::Decimal(_)
                | Self::Float(_)
                | Self::Double(_)
                | Self::BigInteger { .. }
                | Self::BigDecimal(_)
        )
    }

    /// Whether the value is on the exact branch of the numeric tower: an integer
    /// of any integer-family datatype or a decimal, of any size.
    #[must_use]
    pub const fn is_exact_numeric(&self) -> bool {
        matches!(
            self,
            Self::Integer { .. } | Self::Decimal(_) | Self::BigInteger { .. } | Self::BigDecimal(_)
        )
    }

    /// The value `value` of integer `datatype`: [`Self::Integer`] inside `i128`,
    /// [`Self::BigInteger`] outside it.
    #[must_use]
    pub fn from_exact_integer(value: exact::Integer, datatype: XsdDatatype) -> Self {
        match value.as_i128() {
            Some(value) => Self::Integer { value, datatype },
            None => Self::BigInteger { value, datatype },
        }
    }

    /// The decimal `value`: [`Self::Decimal`] when the bounded decimal holds it,
    /// [`Self::BigDecimal`] otherwise.
    #[must_use]
    pub fn from_exact_decimal(value: exact::Decimal) -> Self {
        match value.to_bounded() {
            Ok(bounded) => Self::Decimal(bounded),
            Err(_) => Self::BigDecimal(value),
        }
    }

    /// The exact integer an integer-family value holds, of any size; `None` for
    /// every other variant.
    #[must_use]
    pub fn to_exact_integer(&self) -> Option<exact::Integer> {
        match self {
            Self::Integer { value, .. } => Some(exact::Integer::from_i128(*value)),
            Self::BigInteger { value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// The exact decimal value an integer-family or decimal value holds, of any
    /// size; `None` for every other variant (a float or double converts only
    /// explicitly, through [`exact::Decimal::from_f64`]).
    #[must_use]
    pub fn to_exact_decimal(&self) -> Option<exact::Decimal> {
        match self {
            Self::Integer { value, .. } => Some(exact::Decimal::from_integer(
                exact::Integer::from_i128(*value),
            )),
            Self::BigInteger { value, .. } => Some(exact::Decimal::from_integer(value.clone())),
            Self::Decimal(decimal) => Some(exact::Decimal::from_bounded(decimal)),
            Self::BigDecimal(decimal) => Some(decimal.clone()),
            _ => None,
        }
    }

    /// The canonical lexical form of this value (XSD canonical mapping).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::{XsdDatatype, parse};
    ///
    /// assert_eq!(parse("+007", XsdDatatype::Integer)?.canonical_lexical(), "7");
    /// assert_eq!(parse("1", XsdDatatype::Boolean)?.canonical_lexical(), "true");
    /// # Ok::<(), purrdf_xsd::XsdError>(())
    /// ```
    #[must_use]
    pub fn canonical_lexical(&self) -> String {
        match self {
            Self::Integer { value, .. } => value.to_string(),
            Self::Decimal(d) => d.canonical_lexical(),
            Self::BigInteger { value, .. } => value.canonical_lexical(),
            Self::BigDecimal(d) => d.canonical_lexical(),
            Self::Float(f) => crate::numeric::canonical_float(*f),
            Self::Double(d) => crate::numeric::canonical_double(*d),
            Self::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
            Self::String(s) => s.clone(),
            Self::DateTime(v) => v.canonical_lexical(),
            Self::Date(v) => v.canonical_lexical(),
            Self::Time(v) => v.canonical_lexical(),
            Self::Duration(v) => v.canonical_lexical(),
            Self::Gregorian(v) => v.canonical_lexical(),
            Self::Binary { bytes, datatype } => {
                // Only two binary datatypes exist; the constructor in `parse` guarantees
                // the variant carries HexBinary or Base64Binary. Use a total two-arm form.
                if *datatype == XsdDatatype::Base64Binary {
                    crate::binary::canonical_base64(bytes)
                } else {
                    crate::binary::canonical_hex(bytes)
                }
            }
        }
    }
}

/// Parse a lexical form into the XSD value space for a known [`XsdDatatype`].
///
/// Hard-fails on malformed input. This is the interning entry point: a consumer
/// parses once and caches the result keyed by the IR's `TermId` (the cache lives in
/// the consumer; this crate stays decoupled from `purrdf-core`).
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, XsdValue, parse};
///
/// // Distinct lexical forms may denote ONE value (term identity is the IR's job).
/// let v = parse("042", XsdDatatype::Integer)?;
/// assert!(matches!(v, XsdValue::Integer { value: 42, .. }));
///
/// let b = parse("1", XsdDatatype::Boolean)?;
/// assert!(matches!(b, XsdValue::Boolean(true)));
///
/// // Malformed lexicals hard-fail; derived integers are range-checked.
/// assert!(parse("4.2", XsdDatatype::Integer).is_err());
/// assert!(parse("300", XsdDatatype::Byte).is_err());
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn parse(lexical: &str, datatype: XsdDatatype) -> Result<XsdValue, XsdError> {
    use XsdDatatype as D;
    match datatype {
        // All integer-family datatypes share one parse path with range checking.
        D::Integer
        | D::Long
        | D::Int
        | D::Short
        | D::Byte
        | D::UnsignedLong
        | D::UnsignedInt
        | D::UnsignedShort
        | D::UnsignedByte
        | D::NonNegativeInteger
        | D::PositiveInteger
        | D::NonPositiveInteger
        | D::NegativeInteger => match crate::numeric::parse_integer_typed(lexical, datatype) {
            Ok(value) => Ok(XsdValue::Integer { value, datatype }),
            Err(error @ XsdError::OutOfRange { .. }) => parse_big_integer(lexical, datatype, error),
            Err(error) => Err(error),
        },
        D::Decimal => match crate::numeric::parse_decimal(lexical) {
            Ok(decimal) => Ok(XsdValue::Decimal(decimal)),
            Err(XsdError::OutOfRange { .. }) => lexical
                .parse::<exact::Decimal>()
                .map(XsdValue::from_exact_decimal)
                .map_err(XsdError::Exact),
            Err(error) => Err(error),
        },
        D::Float => crate::numeric::parse_float(lexical).map(XsdValue::Float),
        D::Double => crate::numeric::parse_double(lexical).map(XsdValue::Double),
        D::Boolean => crate::simple::parse_boolean(lexical).map(XsdValue::Boolean),
        D::String => Ok(XsdValue::String(lexical.to_string())),
        D::DateTime => temporal::parse_datetime(lexical).map(XsdValue::DateTime),
        D::Date => temporal::parse_date(lexical).map(XsdValue::Date),
        D::Time => temporal::parse_time(lexical).map(XsdValue::Time),
        D::Duration | D::DayTimeDuration | D::YearMonthDuration => {
            temporal::parse_duration(datatype, lexical).map(XsdValue::Duration)
        }
        D::GYear | D::GMonth | D::GDay | D::GYearMonth | D::GMonthDay => {
            temporal::parse_gregorian(datatype, lexical).map(XsdValue::Gregorian)
        }
        D::HexBinary => {
            crate::binary::parse_hex(lexical).map(|bytes| XsdValue::Binary { bytes, datatype })
        }
        D::Base64Binary => {
            crate::binary::parse_base64(lexical).map(|bytes| XsdValue::Binary { bytes, datatype })
        }
    }
}

/// The integer-family value of a lexical form the `i128` parse refused with
/// `bounded_error`: the exact value when it lies outside `i128` and inside
/// `datatype`'s value space, the original refusal otherwise (a value inside `i128`
/// was refused for its derived range, and every bounded derived datatype lies
/// inside `i128`).
fn parse_big_integer(
    lexical: &str,
    datatype: XsdDatatype,
    bounded_error: XsdError,
) -> Result<XsdValue, XsdError> {
    let Ok(value) = lexical.parse::<exact::Integer>() else {
        return Err(bounded_error);
    };
    if value.as_i128().is_some() {
        return Err(bounded_error);
    }
    if datatype.admits_integer(&value) {
        Ok(XsdValue::BigInteger { value, datatype })
    } else {
        Err(bounded_error)
    }
}

/// As [`parse`], but applies the XSD-1.0 lexical restriction for `Float`/`Double`
/// (rejects `+INF`). All other datatypes behave exactly as [`parse`].
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdDatatype, parse, parse_xsd10};
///
/// // XSD 1.1 (the default) accepts the `+INF` spelling; XSD 1.0 rejects it.
/// assert!(parse("+INF", XsdDatatype::Double).is_ok());
/// assert!(parse_xsd10("+INF", XsdDatatype::Double).is_err());
///
/// // The unsigned spelling is valid in both versions.
/// assert!(parse_xsd10("INF", XsdDatatype::Double).is_ok());
/// ```
pub fn parse_xsd10(s: &str, dt: XsdDatatype) -> Result<XsdValue, XsdError> {
    match dt {
        XsdDatatype::Float => crate::numeric::parse_float_xsd10(s).map(XsdValue::Float),
        XsdDatatype::Double => crate::numeric::parse_double_xsd10(s).map(XsdValue::Double),
        _ => parse(s, dt),
    }
}

/// Parse a lexical form by datatype IRI.
///
/// Returns `Ok(None)` when `datatype_iri` is **not** an XSD value-space datatype —
/// the caller then treats the literal as a plain (opaque) term. `Err` means the IRI
/// *is* an XSD value-space datatype but the lexical form is invalid. This cleanly
/// separates "unknown datatype" from "malformed lexical".
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::{XsdValue, parse_by_iri};
///
/// // A known XSD datatype IRI parses into the value space.
/// let v = parse_by_iri("true", "http://www.w3.org/2001/XMLSchema#boolean")?;
/// assert!(matches!(v, Some(XsdValue::Boolean(true))));
///
/// // An unknown datatype IRI is `Ok(None)` — an opaque literal, not an error.
/// let opaque = parse_by_iri("anything", "http://example.org/customType")?;
/// assert!(opaque.is_none());
///
/// // A known datatype with a malformed lexical IS an error.
/// assert!(parse_by_iri("maybe", "http://www.w3.org/2001/XMLSchema#boolean").is_err());
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
pub fn parse_by_iri(lexical: &str, datatype_iri: &str) -> Result<Option<XsdValue>, XsdError> {
    match XsdDatatype::from_iri(datatype_iri) {
        Some(dt) => parse(lexical, dt).map(Some),
        None => Ok(None),
    }
}

/// A failure to map a lexical form into the XSD value space. Malformed input is a
/// hard error (never a silent default), per the project's no-optionality rule.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum XsdError {
    /// The lexical form is not valid for the target datatype.
    InvalidLexical {
        /// The datatype the lexical was being parsed as.
        datatype: XsdDatatype,
        /// The offending lexical form.
        lexical: String,
        /// A short, stable explanation.
        reason: &'static str,
    },
    /// The lexical form is well-formed but exceeds this crate's representable range
    /// (e.g. an integer beyond `i128`, a derived integer out of its subtype bounds,
    /// or a decimal beyond `i128` mantissa). This is a deliberate hard-fail rather
    /// than saturation: values outside the `i128` / scale-≤18 domain are rejected,
    /// never silently truncated.
    OutOfRange {
        /// The datatype the lexical was being parsed as.
        datatype: XsdDatatype,
        /// The offending lexical form.
        lexical: String,
        /// A short, stable explanation of which bound was exceeded.
        reason: &'static str,
    },
    /// Division by zero for an exact numeric type (integer or decimal). Per
    /// SPARQL §17.4 / XPath `op:numeric-divide`, dividing an `xsd:integer` or
    /// `xsd:decimal` by zero is a hard type error. Float and double division by zero
    /// follows IEEE 754 (yields ±INF or NaN) and is NOT an error.
    DivisionByZero {
        /// The datatype of the dividend (the numerator operand).
        datatype: XsdDatatype,
    },
    /// An arithmetic or unary operation was applied to a non-numeric value (e.g.
    /// `numeric_unary_minus` on a boolean). SPARQL treats this as a type error.
    TypeMismatch {
        /// A short description of what was expected vs. what was received.
        reason: &'static str,
    },
    /// The result of an operation is genuinely **under-determined by the data** —
    /// distinct from [`Self::TypeMismatch`] (the operand *types* are fine; a
    /// static check cannot reject the expression) and from [`Self::OutOfRange`]
    /// (the value exists but this crate cannot represent it). Examples: instant
    /// subtraction where exactly one operand carries a timezone (XSD's partial
    /// order has no defined answer for the mix), or dividing two value-
    /// incommensurable `xsd:duration`s (a months-shaped dividend against a
    /// seconds-shaped divisor). The crate already treats indeterminacy as
    /// first-class elsewhere (`value_cmp`'s `None`, `range::TemporalBounds::Indeterminate`);
    /// this variant gives it the same status inside `Result`.
    Indeterminate {
        /// A short, stable explanation of what is indeterminate and why.
        reason: &'static str,
    },
    /// An operation of the exact tower refused, with the typed reason and its
    /// XPath F&O error code ([`exact::ExactError::code`]) — for example a
    /// quotient with no finite decimal expansion under
    /// [`exact::DivisionPolicy::Exact`].
    Exact(exact::ExactError),
}

impl XsdError {
    /// [`XsdError::InvalidLexical`] for `lexical` read as `datatype`: the one
    /// constructor every lexical-space check reports its refusal through.
    pub(crate) fn invalid(datatype: XsdDatatype, lexical: &str, reason: &'static str) -> Self {
        Self::InvalidLexical {
            datatype,
            lexical: lexical.to_owned(),
            reason,
        }
    }
}

impl std::fmt::Display for XsdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLexical {
                datatype,
                lexical,
                reason,
            } => write!(
                f,
                "invalid lexical form {lexical:?} for <{}>: {reason}",
                datatype.iri()
            ),
            Self::OutOfRange {
                datatype,
                lexical,
                reason,
            } => write!(
                f,
                "lexical form {lexical:?} is out of representable range for <{}>: {reason}",
                datatype.iri()
            ),
            Self::DivisionByZero { datatype } => {
                write!(f, "division by zero for <{}>", datatype.iri())
            }
            Self::TypeMismatch { reason } => write!(f, "type mismatch: {reason}"),
            Self::Indeterminate { reason } => write!(f, "indeterminate: {reason}"),
            Self::Exact(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for XsdError {}
