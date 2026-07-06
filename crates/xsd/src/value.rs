// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
use crate::numeric::Decimal;
use crate::temporal;

/// A parsed XSD value (value space). Variants are added per datatype family across
/// the foundation tasks; numeric first.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum XsdValue {
    /// `xsd:integer` and all derived integer datatypes — `i128`-bounded.
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
    /// `xsd:decimal` — exact fixed-point (`i128` mantissa + scale).
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
            Self::Binary { datatype, .. } => *datatype,
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
        | D::NegativeInteger => crate::numeric::parse_integer_typed(lexical, datatype)
            .map(|value| XsdValue::Integer { value, datatype }),
        D::Decimal => crate::numeric::parse_decimal(lexical).map(XsdValue::Decimal),
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
        }
    }
}

impl std::error::Error for XsdError {}
