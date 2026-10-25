// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original grammar operands, kept independently of their human presentation.

use core::{fmt, num::TryFromIntError, str::Utf8Error};
use purrdf_core::ir::pack::bits::PackBitsError;

/// Native Compact metadata refusal, retaining original operands and causes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompactReason {
    /// A Compact STOP header also names a nonzero field delta.
    StopDelta {
        /// Original complete header byte.
        header: u8,
    },
    /// A Compact field id is outside its signed 16-bit space.
    FieldIdRange {
        /// Original decoded integer.
        value: i64,
        /// Original narrowing refusal.
        cause: TryFromIntError,
    },
    /// A Compact field delta overflows its preceding field id.
    FieldDelta {
        /// Previous field id.
        previous: i16,
        /// Original delta.
        delta: i16,
    },
    /// A Compact field id is not positive.
    NonpositiveFieldId {
        /// Original field id.
        id: i16,
    },
    /// A Compact integer is outside its signed 32-bit space.
    IntegerRange {
        /// Original decoded integer.
        value: i64,
        /// Original narrowing refusal.
        cause: TryFromIntError,
    },
    /// A Compact Boolean field has neither Boolean type discriminator.
    BooleanType {
        /// Original field id.
        id: i16,
        /// Original type discriminator.
        field_type: u8,
    },
    /// A Compact string is not UTF-8.
    Utf8 {
        /// Original UTF-8 refusal, retaining both exact byte coordinates.
        cause: Utf8Error,
    },
    /// A Compact collection names a different element type.
    ElementType {
        /// Original element type.
        actual: u8,
        /// Required element type.
        expected: u8,
    },
    /// STOP cannot occur where a Compact value is required.
    StopValue,
    /// STOP cannot be a Compact collection's element type.
    StopElement,
    /// A Compact collection length does not fit the host address space.
    CollectionLength {
        /// Original unsigned length.
        value: u64,
        /// Original narrowing refusal.
        cause: TryFromIntError,
    },
    /// A Compact binary length does not fit the host address space.
    BinaryLength {
        /// Original unsigned length.
        value: u64,
        /// Original narrowing refusal.
        cause: TryFromIntError,
    },
    /// The shared varint reader refused an original encoded integer.
    Varint {
        /// Byte position at which decoding began.
        offset: usize,
        /// Original shared byte-codec refusal.
        cause: PackBitsError,
    },
    /// A Compact byte range cannot be represented.
    ByteRange {
        /// Original start position.
        offset: usize,
        /// Original requested length.
        length: usize,
    },
    /// A Compact field has a different type from the required one.
    FieldType {
        /// Original field id.
        id: i16,
        /// Original type discriminator.
        actual: u8,
        /// Required type discriminator.
        expected: u8,
    },
}

impl purrdf_hash::error::StructuredError for CompactReason {
    purrdf_hash::error_visit! {
        StopDelta { header } => "columnar-stop-delta" {
            "header" => Unsigned(u128::from(*header)),
        },
        FieldIdRange { value, cause } => "columnar-field-id-range" {
            "value" => Signed(i128::from(*value)), "cause" => Cause(cause),
        },
        FieldDelta { previous, delta } => "columnar-field-delta" {
            "previous" => Signed(i128::from(*previous)), "delta" => Signed(i128::from(*delta)),
        },
        NonpositiveFieldId { id } => "columnar-nonpositive-field-id" {
            "id" => Signed(i128::from(*id)),
        },
        IntegerRange { value, cause } => "columnar-integer-range" {
            "value" => Signed(i128::from(*value)), "cause" => Cause(cause),
        },
        BooleanType { id, field_type } => "columnar-boolean-type" {
            "id" => Signed(i128::from(*id)), "field_type" => Unsigned(u128::from(*field_type)),
        },
        Utf8 { cause } => "columnar-utf8" { "cause" => Cause(cause) },
        ElementType { actual, expected } => "columnar-element-type" {
            "actual" => Unsigned(u128::from(*actual)), "expected" => Unsigned(u128::from(*expected)),
        },
        StopValue => "columnar-stop-value" {},
        StopElement => "columnar-stop-element" {},
        CollectionLength { value, cause } => "columnar-collection-length" {
            "value" => Unsigned(u128::from(*value)), "cause" => Cause(cause),
        },
        BinaryLength { value, cause } => "columnar-binary-length" {
            "value" => ByteCount(u128::from(*value)), "cause" => Cause(cause),
        },
        Varint { offset, cause } => "columnar-varint" {
            "offset" => ByteCount(*offset as u128), "cause" => Cause(cause),
        },
        ByteRange { offset, length } => "columnar-byte-range" {
            "offset" => ByteCount(*offset as u128), "length" => ByteCount(*length as u128),
        },
        FieldType { id, actual, expected } => "columnar-field-type" {
            "id" => Signed(i128::from(*id)), "actual" => Unsigned(u128::from(*actual)),
            "expected" => Unsigned(u128::from(*expected)),
        },
    }
}

impl fmt::Display for CompactReason {
    /// Render the established grammar message from retained native fields.
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StopDelta { .. } => output.write_str("STOP field carries a non-zero delta"),
            Self::FieldIdRange { .. } => output.write_str("value exceeds i16"),
            Self::FieldDelta { .. } => output.write_str("delta overflows i16"),
            Self::NonpositiveFieldId { .. } => output.write_str("field ids must be positive"),
            Self::IntegerRange { .. } => output.write_str("value exceeds i32"),
            Self::BooleanType { id, field_type } => {
                write!(output, "field {id} has type {field_type}")
            }
            Self::Utf8 { .. } => output.write_str("invalid UTF-8"),
            Self::ElementType { actual, .. } => write!(output, "element type is {actual}"),
            Self::StopValue => output.write_str("STOP is not a value type"),
            Self::StopElement => output.write_str("STOP is not a valid element type"),
            Self::CollectionLength { .. } => output.write_str("length exceeds usize"),
            Self::BinaryLength { .. } => output.write_str("value exceeds usize"),
            Self::Varint { cause, .. } => cause.fmt(output),
            Self::ByteRange { .. } => output.write_str("byte range overflows usize"),
            Self::FieldType {
                id,
                actual,
                expected,
            } => {
                write!(output, "field {id} has type {actual}, expected {expected}")
            }
        }
    }
}

purrdf_lex::error_source!(CompactReason => {
    Self::FieldIdRange { cause, .. } => cause,
    Self::IntegerRange { cause, .. } => cause,
    Self::Utf8 { cause } => cause,
    Self::CollectionLength { cause, .. } => cause,
    Self::BinaryLength { cause, .. } => cause,
    Self::Varint { cause, .. } => cause,
});
