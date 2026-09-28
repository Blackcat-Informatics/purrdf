// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The XSD datatype vocabulary this crate's value space covers.
//!
//! The IRI string constants are **value-identical** to the ones used elsewhere in
//! the workspace (e.g. `XSD_STRING` in `purrdf-core`'s `ir/term.rs`). They are
//! copied here deliberately: `purrdf-xsd` is a leaf crate and does not (yet) share a
//! symbol with `purrdf-core` (whose copies are `pub(crate)` and which does not
//! depend on this crate). The crate tests pin the exact strings so the copies
//! cannot silently drift; de-duplicating into a single source is a later slice.

/// The XML Schema datatype namespace.
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// `xsd:integer` — arbitrary-magnitude (this crate: `i128`-bounded) signed integer.
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// `xsd:long` — 64-bit signed integer (`-9223372036854775808..9223372036854775807`).
pub const XSD_LONG: &str = "http://www.w3.org/2001/XMLSchema#long";
/// `xsd:int` — 32-bit signed integer (`-2147483648..2147483647`).
pub const XSD_INT: &str = "http://www.w3.org/2001/XMLSchema#int";
/// `xsd:short` — 16-bit signed integer (`-32768..32767`).
pub const XSD_SHORT: &str = "http://www.w3.org/2001/XMLSchema#short";
/// `xsd:byte` — 8-bit signed integer (`-128..127`).
pub const XSD_BYTE: &str = "http://www.w3.org/2001/XMLSchema#byte";
/// `xsd:unsignedLong` — 64-bit unsigned integer (`0..18446744073709551615`).
pub const XSD_UNSIGNED_LONG: &str = "http://www.w3.org/2001/XMLSchema#unsignedLong";
/// `xsd:unsignedInt` — 32-bit unsigned integer (`0..4294967295`).
pub const XSD_UNSIGNED_INT: &str = "http://www.w3.org/2001/XMLSchema#unsignedInt";
/// `xsd:unsignedShort` — 16-bit unsigned integer (`0..65535`).
pub const XSD_UNSIGNED_SHORT: &str = "http://www.w3.org/2001/XMLSchema#unsignedShort";
/// `xsd:unsignedByte` — 8-bit unsigned integer (`0..255`).
pub const XSD_UNSIGNED_BYTE: &str = "http://www.w3.org/2001/XMLSchema#unsignedByte";
/// `xsd:nonNegativeInteger` — integer `>= 0`.
pub const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
/// `xsd:positiveInteger` — integer `> 0` (i.e. `>= 1`).
pub const XSD_POSITIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#positiveInteger";
/// `xsd:nonPositiveInteger` — integer `<= 0`.
pub const XSD_NON_POSITIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonPositiveInteger";
/// `xsd:negativeInteger` — integer `< 0` (i.e. `<= -1`).
pub const XSD_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#negativeInteger";
/// `xsd:decimal` — exact decimal (this crate: `i128` mantissa, fixed scale).
pub const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
/// `xsd:float` — IEEE single-precision.
pub const XSD_FLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";
/// `xsd:double` — IEEE double-precision.
pub const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
/// `xsd:boolean`.
pub const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
/// `xsd:string`.
pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// `xsd:date`.
pub const XSD_DATE: &str = "http://www.w3.org/2001/XMLSchema#date";
/// `xsd:time`.
pub const XSD_TIME: &str = "http://www.w3.org/2001/XMLSchema#time";
/// `xsd:dateTime`.
pub const XSD_DATE_TIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
/// `xsd:duration` — the general duration (months + seconds; partial order).
pub const XSD_DURATION: &str = "http://www.w3.org/2001/XMLSchema#duration";
/// `xsd:dayTimeDuration` — totally-ordered duration subtype (seconds only).
pub const XSD_DAY_TIME_DURATION: &str = "http://www.w3.org/2001/XMLSchema#dayTimeDuration";
/// `xsd:yearMonthDuration` — totally-ordered duration subtype (months only).
pub const XSD_YEAR_MONTH_DURATION: &str = "http://www.w3.org/2001/XMLSchema#yearMonthDuration";
/// `xsd:gYear` — a Gregorian year (e.g. `2024`).
pub const XSD_G_YEAR: &str = "http://www.w3.org/2001/XMLSchema#gYear";
/// `xsd:gMonth` — a Gregorian month (e.g. `--05`).
pub const XSD_G_MONTH: &str = "http://www.w3.org/2001/XMLSchema#gMonth";
/// `xsd:gDay` — a Gregorian day of the month (e.g. `---15`).
pub const XSD_G_DAY: &str = "http://www.w3.org/2001/XMLSchema#gDay";
/// `xsd:gYearMonth` — a Gregorian year + month (e.g. `2024-05`).
pub const XSD_G_YEAR_MONTH: &str = "http://www.w3.org/2001/XMLSchema#gYearMonth";
/// `xsd:gMonthDay` — a Gregorian month + day (e.g. `--02-29`).
pub const XSD_G_MONTH_DAY: &str = "http://www.w3.org/2001/XMLSchema#gMonthDay";
/// `xsd:hexBinary` — a sequence of hex-encoded bytes.
pub const XSD_HEX_BINARY: &str = "http://www.w3.org/2001/XMLSchema#hexBinary";
/// `xsd:base64Binary` — a Base64-encoded byte sequence.
pub const XSD_BASE64_BINARY: &str = "http://www.w3.org/2001/XMLSchema#base64Binary";

// ── Datatypes this crate names but does not model as a value space ─────────────────
//
// Each of these is a real XSD 1.1 datatype whose value space is a string (or, for
// `dateTimeStamp`, a `dateTime` with a required timezone) and which other crates in
// the workspace recognise by IRI — SHACL datatype constraints, JSON Schema
// mappings, OWL 2 datatype maps, the `IRI()`/`STR()` cast family. They are spelled
// here ONCE so those crates share one byte-exact constant instead of retyping it.
// None of them is an [`XsdDatatype`] variant: `parse_by_iri` still answers
// `Ok(None)` for them, and that three-valued honesty is deliberate.

/// `xsd:anyURI` — a URI reference; its value space is its lexical space.
pub const XSD_ANY_URI: &str = "http://www.w3.org/2001/XMLSchema#anyURI";
/// `xsd:normalizedString` — `xsd:string` under the `whiteSpace = replace` facet.
pub const XSD_NORMALIZED_STRING: &str = "http://www.w3.org/2001/XMLSchema#normalizedString";
/// `xsd:token` — `xsd:normalizedString` under the `whiteSpace = collapse` facet.
pub const XSD_TOKEN: &str = "http://www.w3.org/2001/XMLSchema#token";
/// `xsd:language` — a BCP 47 language tag, derived from `xsd:token`.
pub const XSD_LANGUAGE: &str = "http://www.w3.org/2001/XMLSchema#language";
/// `xsd:Name` — an XML `Name`, derived from `xsd:token`.
pub const XSD_NAME: &str = "http://www.w3.org/2001/XMLSchema#Name";
/// `xsd:NCName` — an XML non-colonized name, derived from `xsd:Name`.
pub const XSD_NCNAME: &str = "http://www.w3.org/2001/XMLSchema#NCName";
/// `xsd:NMTOKEN` — an XML `Nmtoken`, derived from `xsd:token`.
pub const XSD_NMTOKEN: &str = "http://www.w3.org/2001/XMLSchema#NMTOKEN";
/// `xsd:dateTimeStamp` — an `xsd:dateTime` whose timezone is required (XSD 1.1).
pub const XSD_DATE_TIME_STAMP: &str = "http://www.w3.org/2001/XMLSchema#dateTimeStamp";
/// `xsd:anySimpleType` — the root of the simple-type hierarchy.
pub const XSD_ANY_SIMPLE_TYPE: &str = "http://www.w3.org/2001/XMLSchema#anySimpleType";
/// `xsd:anyAtomicType` — the base of every atomic datatype (XSD 1.1).
pub const XSD_ANY_ATOMIC_TYPE: &str = "http://www.w3.org/2001/XMLSchema#anyAtomicType";
/// `xsd:NOTATION` — an XML notation reference.
pub const XSD_NOTATION: &str = "http://www.w3.org/2001/XMLSchema#NOTATION";
/// `xsd:QName` — an XML qualified name.
pub const XSD_QNAME: &str = "http://www.w3.org/2001/XMLSchema#QName";
/// `xsd:ID` — an XML identifier, derived from `xsd:NCName`.
pub const XSD_ID: &str = "http://www.w3.org/2001/XMLSchema#ID";
/// `xsd:IDREF` — an XML identifier reference, derived from `xsd:NCName`.
pub const XSD_IDREF: &str = "http://www.w3.org/2001/XMLSchema#IDREF";
/// `xsd:ENTITY` — an XML unparsed-entity reference, derived from `xsd:NCName`.
pub const XSD_ENTITY: &str = "http://www.w3.org/2001/XMLSchema#ENTITY";

// ── Constraining facets (XSD 1.1 Part 2 §4.3) ──────────────────────────────────────
//
// The facet IRIs SHACL (`sh:datatype` restrictions), OWL 2 datatype restrictions
// and JSON Schema mappings name. They are predicates, not datatypes, and have no
// [`XsdDatatype`] variant.

/// `xsd:minInclusive` — the inclusive lower bound facet (§4.3.10).
pub const XSD_MIN_INCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#minInclusive";
/// `xsd:maxInclusive` — the inclusive upper bound facet (§4.3.7).
pub const XSD_MAX_INCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#maxInclusive";
/// `xsd:minExclusive` — the exclusive lower bound facet (§4.3.9).
pub const XSD_MIN_EXCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#minExclusive";
/// `xsd:maxExclusive` — the exclusive upper bound facet (§4.3.8).
pub const XSD_MAX_EXCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#maxExclusive";
/// `xsd:length` — the exact length facet (§4.3.1).
pub const XSD_LENGTH: &str = "http://www.w3.org/2001/XMLSchema#length";
/// `xsd:minLength` — the minimum length facet (§4.3.2).
pub const XSD_MIN_LENGTH: &str = "http://www.w3.org/2001/XMLSchema#minLength";
/// `xsd:maxLength` — the maximum length facet (§4.3.3).
pub const XSD_MAX_LENGTH: &str = "http://www.w3.org/2001/XMLSchema#maxLength";
/// `xsd:pattern` — the regular-expression facet (§4.3.4).
pub const XSD_PATTERN: &str = "http://www.w3.org/2001/XMLSchema#pattern";
/// `xsd:totalDigits` — the total-digits facet on decimals (§4.3.11).
pub const XSD_TOTAL_DIGITS: &str = "http://www.w3.org/2001/XMLSchema#totalDigits";
/// `xsd:fractionDigits` — the fraction-digits facet on decimals (§4.3.12).
pub const XSD_FRACTION_DIGITS: &str = "http://www.w3.org/2001/XMLSchema#fractionDigits";
/// `xsd:whiteSpace` — the whitespace-normalization facet (§4.3.6).
pub const XSD_WHITE_SPACE: &str = "http://www.w3.org/2001/XMLSchema#whiteSpace";

/// The XSD datatypes whose **value space** `purrdf-xsd` models.
///
/// This is a closed set by design: XSD does not grow at runtime, so dispatch over
/// this enum is closed-but-correct (no runtime registry). A datatype IRI outside
/// this set is "not an XSD value-space type" — the caller treats such a literal as
/// a plain term (see `parse_by_iri` returning `Ok(None)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum XsdDatatype {
    /// `xsd:integer`.
    Integer,
    /// `xsd:long` — derived integer, 64-bit signed.
    Long,
    /// `xsd:int` — derived integer, 32-bit signed.
    Int,
    /// `xsd:short` — derived integer, 16-bit signed.
    Short,
    /// `xsd:byte` — derived integer, 8-bit signed.
    Byte,
    /// `xsd:unsignedLong` — derived integer, 64-bit unsigned.
    UnsignedLong,
    /// `xsd:unsignedInt` — derived integer, 32-bit unsigned.
    UnsignedInt,
    /// `xsd:unsignedShort` — derived integer, 16-bit unsigned.
    UnsignedShort,
    /// `xsd:unsignedByte` — derived integer, 8-bit unsigned.
    UnsignedByte,
    /// `xsd:nonNegativeInteger` — integer >= 0.
    NonNegativeInteger,
    /// `xsd:positiveInteger` — integer >= 1.
    PositiveInteger,
    /// `xsd:nonPositiveInteger` — integer <= 0.
    NonPositiveInteger,
    /// `xsd:negativeInteger` — integer <= -1.
    NegativeInteger,
    /// `xsd:decimal`.
    Decimal,
    /// `xsd:float`.
    Float,
    /// `xsd:double`.
    Double,
    /// `xsd:boolean`.
    Boolean,
    /// `xsd:string`.
    String,
    /// `xsd:date`.
    Date,
    /// `xsd:time`.
    Time,
    /// `xsd:dateTime`.
    DateTime,
    /// `xsd:duration`.
    Duration,
    /// `xsd:dayTimeDuration`.
    DayTimeDuration,
    /// `xsd:yearMonthDuration`.
    YearMonthDuration,
    /// `xsd:gYear` — Gregorian year.
    GYear,
    /// `xsd:gMonth` — Gregorian month.
    GMonth,
    /// `xsd:gDay` — Gregorian day of month.
    GDay,
    /// `xsd:gYearMonth` — Gregorian year and month.
    GYearMonth,
    /// `xsd:gMonthDay` — Gregorian month and day.
    GMonthDay,
    /// `xsd:hexBinary` — a sequence of bytes encoded as hexadecimal digits.
    HexBinary,
    /// `xsd:base64Binary` — a sequence of bytes encoded as Base64.
    Base64Binary,
}

impl XsdDatatype {
    /// Resolve a datatype IRI to its [`XsdDatatype`], or `None` when the IRI is not
    /// one of the XSD value-space datatypes this crate models.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::XsdDatatype;
    ///
    /// assert_eq!(
    ///     XsdDatatype::from_iri("http://www.w3.org/2001/XMLSchema#integer"),
    ///     Some(XsdDatatype::Integer)
    /// );
    /// // Round-trips with `iri()`.
    /// assert_eq!(
    ///     XsdDatatype::from_iri(XsdDatatype::DateTime.iri()),
    ///     Some(XsdDatatype::DateTime)
    /// );
    /// // A non-XSD datatype IRI is simply not in the value space.
    /// assert_eq!(XsdDatatype::from_iri("http://example.org/customType"), None);
    /// ```
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        // One shared-prefix compare then a match on the short local name: every
        // `XSD_*` constant is literally `XSD_NS ++ local`, so this is the same
        // predicate as 31 full-IRI compares without re-scanning the 33-byte
        // namespace per arm (a non-XSD IRI now fails on the first compare).
        let local = iri.strip_prefix(XSD_NS)?;
        Some(match local {
            "integer" => Self::Integer,
            "long" => Self::Long,
            "int" => Self::Int,
            "short" => Self::Short,
            "byte" => Self::Byte,
            "unsignedLong" => Self::UnsignedLong,
            "unsignedInt" => Self::UnsignedInt,
            "unsignedShort" => Self::UnsignedShort,
            "unsignedByte" => Self::UnsignedByte,
            "nonNegativeInteger" => Self::NonNegativeInteger,
            "positiveInteger" => Self::PositiveInteger,
            "nonPositiveInteger" => Self::NonPositiveInteger,
            "negativeInteger" => Self::NegativeInteger,
            "decimal" => Self::Decimal,
            "float" => Self::Float,
            "double" => Self::Double,
            "boolean" => Self::Boolean,
            "string" => Self::String,
            "date" => Self::Date,
            "time" => Self::Time,
            "dateTime" => Self::DateTime,
            "duration" => Self::Duration,
            "dayTimeDuration" => Self::DayTimeDuration,
            "yearMonthDuration" => Self::YearMonthDuration,
            "gYear" => Self::GYear,
            "gMonth" => Self::GMonth,
            "gDay" => Self::GDay,
            "gYearMonth" => Self::GYearMonth,
            "gMonthDay" => Self::GMonthDay,
            "hexBinary" => Self::HexBinary,
            "base64Binary" => Self::Base64Binary,
            _ => return None,
        })
    }

    /// The canonical datatype IRI for this value-space datatype.
    #[must_use]
    pub const fn iri(self) -> &'static str {
        match self {
            Self::Integer => XSD_INTEGER,
            Self::Long => XSD_LONG,
            Self::Int => XSD_INT,
            Self::Short => XSD_SHORT,
            Self::Byte => XSD_BYTE,
            Self::UnsignedLong => XSD_UNSIGNED_LONG,
            Self::UnsignedInt => XSD_UNSIGNED_INT,
            Self::UnsignedShort => XSD_UNSIGNED_SHORT,
            Self::UnsignedByte => XSD_UNSIGNED_BYTE,
            Self::NonNegativeInteger => XSD_NON_NEGATIVE_INTEGER,
            Self::PositiveInteger => XSD_POSITIVE_INTEGER,
            Self::NonPositiveInteger => XSD_NON_POSITIVE_INTEGER,
            Self::NegativeInteger => XSD_NEGATIVE_INTEGER,
            Self::Decimal => XSD_DECIMAL,
            Self::Float => XSD_FLOAT,
            Self::Double => XSD_DOUBLE,
            Self::Boolean => XSD_BOOLEAN,
            Self::String => XSD_STRING,
            Self::Date => XSD_DATE,
            Self::Time => XSD_TIME,
            Self::DateTime => XSD_DATE_TIME,
            Self::Duration => XSD_DURATION,
            Self::DayTimeDuration => XSD_DAY_TIME_DURATION,
            Self::YearMonthDuration => XSD_YEAR_MONTH_DURATION,
            Self::GYear => XSD_G_YEAR,
            Self::GMonth => XSD_G_MONTH,
            Self::GDay => XSD_G_DAY,
            Self::GYearMonth => XSD_G_YEAR_MONTH,
            Self::GMonthDay => XSD_G_MONTH_DAY,
            Self::HexBinary => XSD_HEX_BINARY,
            Self::Base64Binary => XSD_BASE64_BINARY,
        }
    }

    /// The inclusive `(min, max)` integer bounds for this datatype, or `None` if it is
    /// not an integer-family datatype.
    ///
    /// The returned bounds are the XSD-specified INCLUSIVE constraints. Parsing an
    /// integer-family literal that falls outside these bounds is a hard
    /// [`crate::value::XsdError::OutOfRange`] failure.
    #[must_use]
    pub const fn integer_range(self) -> Option<(i128, i128)> {
        Some(match self {
            Self::Integer => (i128::MIN, i128::MAX),
            Self::Long => (i64::MIN as i128, i64::MAX as i128),
            Self::Int => (i32::MIN as i128, i32::MAX as i128),
            Self::Short => (i16::MIN as i128, i16::MAX as i128),
            Self::Byte => (i8::MIN as i128, i8::MAX as i128),
            Self::UnsignedLong => (0, u64::MAX as i128),
            Self::UnsignedInt => (0, u32::MAX as i128),
            Self::UnsignedShort => (0, u16::MAX as i128),
            Self::UnsignedByte => (0, u8::MAX as i128),
            Self::NonNegativeInteger => (0, i128::MAX),
            Self::PositiveInteger => (1, i128::MAX),
            Self::NonPositiveInteger => (i128::MIN, 0),
            Self::NegativeInteger => (i128::MIN, -1),
            _ => return None,
        })
    }

    /// Whether this datatype is `xsd:integer` or one of the twelve integer datatypes
    /// derived from it (`long` … `unsignedByte`, `nonNegativeInteger` …
    /// `negativeInteger`).
    ///
    /// Derived from [`Self::integer_range`] — the one table that says which datatypes
    /// are integer-shaped — so the two can never disagree.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::XsdDatatype;
    ///
    /// assert!(XsdDatatype::UnsignedByte.is_integer_family());
    /// assert!(!XsdDatatype::Decimal.is_integer_family());
    /// ```
    #[must_use]
    pub const fn is_integer_family(self) -> bool {
        self.integer_range().is_some()
    }

    /// Whether this datatype sits in the SPARQL numeric tower: the integer family
    /// ([`Self::is_integer_family`]) plus `xsd:decimal`, `xsd:float` and
    /// `xsd:double`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::XsdDatatype;
    ///
    /// assert!(XsdDatatype::Float.is_numeric());
    /// assert!(XsdDatatype::NegativeInteger.is_numeric());
    /// assert!(!XsdDatatype::Boolean.is_numeric());
    /// ```
    #[must_use]
    pub const fn is_numeric(self) -> bool {
        self.is_integer_family() || matches!(self, Self::Decimal | Self::Float | Self::Double)
    }

    /// Whether `iri` names an integer-family datatype ([`Self::is_integer_family`]).
    /// `false` for any IRI outside this crate's value space, including an XSD IRI it
    /// does not model.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::XsdDatatype;
    ///
    /// assert!(XsdDatatype::is_integer_family_iri("http://www.w3.org/2001/XMLSchema#long"));
    /// assert!(!XsdDatatype::is_integer_family_iri("http://www.w3.org/2001/XMLSchema#decimal"));
    /// assert!(!XsdDatatype::is_integer_family_iri("http://example.org/integer"));
    /// ```
    #[must_use]
    pub fn is_integer_family_iri(iri: &str) -> bool {
        Self::from_iri(iri).is_some_and(Self::is_integer_family)
    }

    /// Whether `iri` names a datatype in the numeric tower ([`Self::is_numeric`]).
    /// `false` for any IRI outside this crate's value space.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_xsd::XsdDatatype;
    ///
    /// assert!(XsdDatatype::is_numeric_iri("http://www.w3.org/2001/XMLSchema#double"));
    /// assert!(!XsdDatatype::is_numeric_iri("http://www.w3.org/2001/XMLSchema#string"));
    /// ```
    #[must_use]
    pub fn is_numeric_iri(iri: &str) -> bool {
        Self::from_iri(iri).is_some_and(Self::is_numeric)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant, once. A new variant that is not added here fails
    /// `every_variant_is_in_the_roster`, whose exhaustive `match` the compiler
    /// refuses to build without an arm for it.
    const ALL: [XsdDatatype; 31] = [
        XsdDatatype::Integer,
        XsdDatatype::Long,
        XsdDatatype::Int,
        XsdDatatype::Short,
        XsdDatatype::Byte,
        XsdDatatype::UnsignedLong,
        XsdDatatype::UnsignedInt,
        XsdDatatype::UnsignedShort,
        XsdDatatype::UnsignedByte,
        XsdDatatype::NonNegativeInteger,
        XsdDatatype::PositiveInteger,
        XsdDatatype::NonPositiveInteger,
        XsdDatatype::NegativeInteger,
        XsdDatatype::Decimal,
        XsdDatatype::Float,
        XsdDatatype::Double,
        XsdDatatype::Boolean,
        XsdDatatype::String,
        XsdDatatype::Date,
        XsdDatatype::Time,
        XsdDatatype::DateTime,
        XsdDatatype::Duration,
        XsdDatatype::DayTimeDuration,
        XsdDatatype::YearMonthDuration,
        XsdDatatype::GYear,
        XsdDatatype::GMonth,
        XsdDatatype::GDay,
        XsdDatatype::GYearMonth,
        XsdDatatype::GMonthDay,
        XsdDatatype::HexBinary,
        XsdDatatype::Base64Binary,
    ];

    /// The expected answers, one arm per variant: `(is_integer_family, is_numeric)`.
    /// Spelled as a `match` rather than a list so a new variant cannot be forgotten.
    const fn expected_predicates(dt: XsdDatatype) -> (bool, bool) {
        match dt {
            XsdDatatype::Integer
            | XsdDatatype::Long
            | XsdDatatype::Int
            | XsdDatatype::Short
            | XsdDatatype::Byte
            | XsdDatatype::UnsignedLong
            | XsdDatatype::UnsignedInt
            | XsdDatatype::UnsignedShort
            | XsdDatatype::UnsignedByte
            | XsdDatatype::NonNegativeInteger
            | XsdDatatype::PositiveInteger
            | XsdDatatype::NonPositiveInteger
            | XsdDatatype::NegativeInteger => (true, true),
            XsdDatatype::Decimal | XsdDatatype::Float | XsdDatatype::Double => (false, true),
            XsdDatatype::Boolean
            | XsdDatatype::String
            | XsdDatatype::Date
            | XsdDatatype::Time
            | XsdDatatype::DateTime
            | XsdDatatype::Duration
            | XsdDatatype::DayTimeDuration
            | XsdDatatype::YearMonthDuration
            | XsdDatatype::GYear
            | XsdDatatype::GMonth
            | XsdDatatype::GDay
            | XsdDatatype::GYearMonth
            | XsdDatatype::GMonthDay
            | XsdDatatype::HexBinary
            | XsdDatatype::Base64Binary => (false, false),
        }
    }

    #[test]
    fn every_variant_is_in_the_roster() {
        for dt in ALL {
            // The match is exhaustive over the enum; reaching it for every roster
            // entry and finding no duplicates pins the roster to the enum.
            let _ = expected_predicates(dt);
            assert_eq!(ALL.iter().filter(|&&other| other == dt).count(), 1);
        }
    }

    #[test]
    fn iri_round_trips_for_every_datatype() {
        for dt in ALL {
            assert_eq!(XsdDatatype::from_iri(dt.iri()), Some(dt));
            assert!(dt.iri().starts_with(XSD_NS));
        }
    }

    #[test]
    fn integer_family_and_numeric_predicates_agree_with_the_table() {
        let mut integer_count = 0;
        let mut numeric_count = 0;
        for dt in ALL {
            let (integer, numeric) = expected_predicates(dt);
            assert_eq!(dt.is_integer_family(), integer, "{dt:?}");
            assert_eq!(dt.is_numeric(), numeric, "{dt:?}");
            assert_eq!(
                dt.is_integer_family(),
                dt.integer_range().is_some(),
                "{dt:?}: derived from the range table"
            );
            assert_eq!(XsdDatatype::is_integer_family_iri(dt.iri()), integer);
            assert_eq!(XsdDatatype::is_numeric_iri(dt.iri()), numeric);
            integer_count += usize::from(integer);
            numeric_count += usize::from(numeric);
        }
        assert_eq!(integer_count, 13, "integer plus its twelve derived types");
        assert_eq!(
            numeric_count, 16,
            "the integer family plus decimal, float, double"
        );
    }

    #[test]
    fn iri_predicates_refuse_every_iri_outside_the_value_space() {
        for iri in [
            "http://example.org/integer",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
            "http://www.w3.org/2002/07/owl#rational",
            // XSD IRIs this crate names but does not model.
            XSD_ANY_URI,
            XSD_TOKEN,
            XSD_DATE_TIME_STAMP,
            XSD_MIN_INCLUSIVE,
            "",
            // A near miss on the local name.
            "http://www.w3.org/2001/XMLSchema#Integer",
            "http://www.w3.org/2001/XMLSchema#integer ",
        ] {
            assert!(!XsdDatatype::is_integer_family_iri(iri), "{iri:?}");
            assert!(!XsdDatatype::is_numeric_iri(iri), "{iri:?}");
        }
        // The neighbouring valid spellings still answer `true`.
        assert!(XsdDatatype::is_integer_family_iri(XSD_INTEGER));
        assert!(XsdDatatype::is_numeric_iri(XSD_DECIMAL));
    }

    /// The datatypes this crate names without modelling, and the constraining
    /// facets, pinned byte-for-byte: other crates share these constants in place of
    /// their own copies, so the strings must be exactly the W3C spellings (note the
    /// case of `Name`, `NCName`, `NMTOKEN`, `NOTATION`, `QName`, `ID`, `IDREF`,
    /// `ENTITY`).
    #[test]
    fn unmodelled_datatype_and_facet_constants_are_byte_exact() {
        for (constant, local) in [
            (XSD_ANY_URI, "anyURI"),
            (XSD_NORMALIZED_STRING, "normalizedString"),
            (XSD_TOKEN, "token"),
            (XSD_LANGUAGE, "language"),
            (XSD_NAME, "Name"),
            (XSD_NCNAME, "NCName"),
            (XSD_NMTOKEN, "NMTOKEN"),
            (XSD_DATE_TIME_STAMP, "dateTimeStamp"),
            (XSD_ANY_SIMPLE_TYPE, "anySimpleType"),
            (XSD_ANY_ATOMIC_TYPE, "anyAtomicType"),
            (XSD_NOTATION, "NOTATION"),
            (XSD_QNAME, "QName"),
            (XSD_ID, "ID"),
            (XSD_IDREF, "IDREF"),
            (XSD_ENTITY, "ENTITY"),
            (XSD_YEAR_MONTH_DURATION, "yearMonthDuration"),
            (XSD_DAY_TIME_DURATION, "dayTimeDuration"),
            (XSD_MIN_INCLUSIVE, "minInclusive"),
            (XSD_MAX_INCLUSIVE, "maxInclusive"),
            (XSD_MIN_EXCLUSIVE, "minExclusive"),
            (XSD_MAX_EXCLUSIVE, "maxExclusive"),
            (XSD_LENGTH, "length"),
            (XSD_MIN_LENGTH, "minLength"),
            (XSD_MAX_LENGTH, "maxLength"),
            (XSD_PATTERN, "pattern"),
            (XSD_TOTAL_DIGITS, "totalDigits"),
            (XSD_FRACTION_DIGITS, "fractionDigits"),
            (XSD_WHITE_SPACE, "whiteSpace"),
        ] {
            assert_eq!(constant, format!("{XSD_NS}{local}"));
        }
        // None of the unmodelled datatypes or facets is a value-space datatype: the
        // shared entry point keeps answering "recognised as unsupported" for them.
        for iri in [
            XSD_ANY_URI,
            XSD_NORMALIZED_STRING,
            XSD_TOKEN,
            XSD_LANGUAGE,
            XSD_NAME,
            XSD_NCNAME,
            XSD_NMTOKEN,
            XSD_DATE_TIME_STAMP,
            XSD_ANY_SIMPLE_TYPE,
            XSD_ANY_ATOMIC_TYPE,
            XSD_NOTATION,
            XSD_QNAME,
            XSD_ID,
            XSD_IDREF,
            XSD_ENTITY,
            XSD_MIN_INCLUSIVE,
            XSD_MAX_INCLUSIVE,
            XSD_MIN_EXCLUSIVE,
            XSD_MAX_EXCLUSIVE,
            XSD_LENGTH,
            XSD_MIN_LENGTH,
            XSD_MAX_LENGTH,
            XSD_PATTERN,
            XSD_TOTAL_DIGITS,
            XSD_FRACTION_DIGITS,
            XSD_WHITE_SPACE,
        ] {
            assert_eq!(XsdDatatype::from_iri(iri), None, "{iri}");
        }
        // The two duration subtypes, by contrast, ARE modelled.
        assert_eq!(
            XsdDatatype::from_iri(XSD_YEAR_MONTH_DURATION),
            Some(XsdDatatype::YearMonthDuration)
        );
        assert_eq!(
            XsdDatatype::from_iri(XSD_DAY_TIME_DURATION),
            Some(XsdDatatype::DayTimeDuration)
        );
    }

    #[test]
    fn non_xsd_iri_is_none() {
        assert_eq!(
            XsdDatatype::from_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"),
            None
        );
        assert_eq!(XsdDatatype::from_iri("https://example.org/custom"), None);
    }

    /// Pins the exact IRI strings byte-for-byte (the value-equality guard described
    /// in the module docs — these must match `purrdf-core`'s `pub(crate)` copies).
    #[test]
    fn iri_constants_are_byte_exact() {
        assert_eq!(XSD_STRING, "http://www.w3.org/2001/XMLSchema#string");
        assert_eq!(XSD_INTEGER, "http://www.w3.org/2001/XMLSchema#integer");
        assert_eq!(XSD_LONG, "http://www.w3.org/2001/XMLSchema#long");
        assert_eq!(XSD_INT, "http://www.w3.org/2001/XMLSchema#int");
        assert_eq!(XSD_SHORT, "http://www.w3.org/2001/XMLSchema#short");
        assert_eq!(XSD_BYTE, "http://www.w3.org/2001/XMLSchema#byte");
        assert_eq!(
            XSD_UNSIGNED_LONG,
            "http://www.w3.org/2001/XMLSchema#unsignedLong"
        );
        assert_eq!(
            XSD_UNSIGNED_INT,
            "http://www.w3.org/2001/XMLSchema#unsignedInt"
        );
        assert_eq!(
            XSD_UNSIGNED_SHORT,
            "http://www.w3.org/2001/XMLSchema#unsignedShort"
        );
        assert_eq!(
            XSD_UNSIGNED_BYTE,
            "http://www.w3.org/2001/XMLSchema#unsignedByte"
        );
        assert_eq!(
            XSD_NON_NEGATIVE_INTEGER,
            "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
        );
        assert_eq!(
            XSD_POSITIVE_INTEGER,
            "http://www.w3.org/2001/XMLSchema#positiveInteger"
        );
        assert_eq!(
            XSD_NON_POSITIVE_INTEGER,
            "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
        );
        assert_eq!(
            XSD_NEGATIVE_INTEGER,
            "http://www.w3.org/2001/XMLSchema#negativeInteger"
        );
        assert_eq!(XSD_DECIMAL, "http://www.w3.org/2001/XMLSchema#decimal");
        assert_eq!(XSD_BOOLEAN, "http://www.w3.org/2001/XMLSchema#boolean");
        assert_eq!(XSD_DOUBLE, "http://www.w3.org/2001/XMLSchema#double");
        assert_eq!(XSD_DATE_TIME, "http://www.w3.org/2001/XMLSchema#dateTime");
        // Gregorian family pin
        assert_eq!(XSD_G_YEAR, "http://www.w3.org/2001/XMLSchema#gYear");
        assert_eq!(XSD_G_MONTH, "http://www.w3.org/2001/XMLSchema#gMonth");
        assert_eq!(XSD_G_DAY, "http://www.w3.org/2001/XMLSchema#gDay");
        assert_eq!(
            XSD_G_YEAR_MONTH,
            "http://www.w3.org/2001/XMLSchema#gYearMonth"
        );
        assert_eq!(
            XSD_G_MONTH_DAY,
            "http://www.w3.org/2001/XMLSchema#gMonthDay"
        );
        assert_eq!(XSD_HEX_BINARY, "http://www.w3.org/2001/XMLSchema#hexBinary");
        assert_eq!(
            XSD_BASE64_BINARY,
            "http://www.w3.org/2001/XMLSchema#base64Binary"
        );
    }

    #[test]
    fn integer_range_table() {
        assert_eq!(
            XsdDatatype::Integer.integer_range(),
            Some((i128::MIN, i128::MAX))
        );
        assert_eq!(
            XsdDatatype::Long.integer_range(),
            Some((i128::from(i64::MIN), i128::from(i64::MAX)))
        );
        assert_eq!(
            XsdDatatype::Int.integer_range(),
            Some((i128::from(i32::MIN), i128::from(i32::MAX)))
        );
        assert_eq!(
            XsdDatatype::Short.integer_range(),
            Some((i128::from(i16::MIN), i128::from(i16::MAX)))
        );
        assert_eq!(
            XsdDatatype::Byte.integer_range(),
            Some((i128::from(i8::MIN), i128::from(i8::MAX)))
        );
        assert_eq!(
            XsdDatatype::UnsignedLong.integer_range(),
            Some((0, i128::from(u64::MAX)))
        );
        assert_eq!(
            XsdDatatype::UnsignedInt.integer_range(),
            Some((0, i128::from(u32::MAX)))
        );
        assert_eq!(
            XsdDatatype::UnsignedShort.integer_range(),
            Some((0, i128::from(u16::MAX)))
        );
        assert_eq!(
            XsdDatatype::UnsignedByte.integer_range(),
            Some((0, i128::from(u8::MAX)))
        );
        assert_eq!(
            XsdDatatype::NonNegativeInteger.integer_range(),
            Some((0, i128::MAX))
        );
        assert_eq!(
            XsdDatatype::PositiveInteger.integer_range(),
            Some((1, i128::MAX))
        );
        assert_eq!(
            XsdDatatype::NonPositiveInteger.integer_range(),
            Some((i128::MIN, 0))
        );
        assert_eq!(
            XsdDatatype::NegativeInteger.integer_range(),
            Some((i128::MIN, -1))
        );
        // Non-integer datatypes have no range.
        assert_eq!(XsdDatatype::Decimal.integer_range(), None);
        assert_eq!(XsdDatatype::Double.integer_range(), None);
        assert_eq!(XsdDatatype::Boolean.integer_range(), None);
        assert_eq!(XsdDatatype::String.integer_range(), None);
        // Gregorian types have no integer range.
        assert_eq!(XsdDatatype::GYear.integer_range(), None);
        assert_eq!(XsdDatatype::GMonth.integer_range(), None);
        assert_eq!(XsdDatatype::GDay.integer_range(), None);
        assert_eq!(XsdDatatype::GYearMonth.integer_range(), None);
        assert_eq!(XsdDatatype::GMonthDay.integer_range(), None);
    }
}
