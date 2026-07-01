// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        Some(match iri {
            XSD_INTEGER => Self::Integer,
            XSD_LONG => Self::Long,
            XSD_INT => Self::Int,
            XSD_SHORT => Self::Short,
            XSD_BYTE => Self::Byte,
            XSD_UNSIGNED_LONG => Self::UnsignedLong,
            XSD_UNSIGNED_INT => Self::UnsignedInt,
            XSD_UNSIGNED_SHORT => Self::UnsignedShort,
            XSD_UNSIGNED_BYTE => Self::UnsignedByte,
            XSD_NON_NEGATIVE_INTEGER => Self::NonNegativeInteger,
            XSD_POSITIVE_INTEGER => Self::PositiveInteger,
            XSD_NON_POSITIVE_INTEGER => Self::NonPositiveInteger,
            XSD_NEGATIVE_INTEGER => Self::NegativeInteger,
            XSD_DECIMAL => Self::Decimal,
            XSD_FLOAT => Self::Float,
            XSD_DOUBLE => Self::Double,
            XSD_BOOLEAN => Self::Boolean,
            XSD_STRING => Self::String,
            XSD_DATE => Self::Date,
            XSD_TIME => Self::Time,
            XSD_DATE_TIME => Self::DateTime,
            XSD_DURATION => Self::Duration,
            XSD_DAY_TIME_DURATION => Self::DayTimeDuration,
            XSD_YEAR_MONTH_DURATION => Self::YearMonthDuration,
            XSD_G_YEAR => Self::GYear,
            XSD_G_MONTH => Self::GMonth,
            XSD_G_DAY => Self::GDay,
            XSD_G_YEAR_MONTH => Self::GYearMonth,
            XSD_G_MONTH_DAY => Self::GMonthDay,
            XSD_HEX_BINARY => Self::HexBinary,
            XSD_BASE64_BINARY => Self::Base64Binary,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn iri_round_trips_for_every_datatype() {
        for dt in [
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
        ] {
            assert_eq!(XsdDatatype::from_iri(dt.iri()), Some(dt));
            assert!(dt.iri().starts_with(XSD_NS));
        }
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
