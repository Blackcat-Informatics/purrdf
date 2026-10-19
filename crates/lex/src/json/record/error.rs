// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native record diagnostic construction and borrowed typed inspection.

use super::{DecodeError, Error, OneOf, RecordAllocation, Unexpected, Value, fmt};
use crate::allocation::{Admission, Memory, Resident, StorageError};

/// Borrowed native inputs to a record diagnostic, before any owned copy.
/// The original record account admits both retained fields and human presentation.
#[derive(Clone, Copy)]
pub enum DecodeReason<'a> {
    /// A genuine caller-supplied message, rather than a native shape failure.
    Message(&'a dyn fmt::Display),
    /// A value has a different JSON kind from the required kind.
    InvalidType {
        /// The original offered value.
        found: &'a Value,
        /// The required kind's description.
        expected: &'a str,
    },
    /// A value is outside the required value domain.
    InvalidValue {
        /// The original offered value.
        found: &'a Value,
        /// The required value domain's description.
        expected: &'a str,
    },
    /// A string names no admitted variant.
    UnknownVariant {
        /// The offered variant name.
        variant: &'a str,
        /// The complete ordered set of admitted names.
        expected: &'a [&'a str],
    },
    /// An object member names no declared field.
    UnknownField {
        /// The offered member name.
        field: &'a str,
        /// The complete ordered set of declared names.
        expected: &'a [&'a str],
    },
    /// A required member is absent.
    MissingField(&'a str),
    /// A declared member occurs more than once.
    DuplicateField(&'a str),
    /// The original JSON document reader refused the input.
    Json(Error),
}
purrdf_hash::debug_non_exhaustive!(DecodeReason<'_> {});

impl fmt::Display for DecodeReason<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => message.fmt(formatter),
            Self::InvalidType { found, expected } => write!(
                formatter,
                "invalid type: {}, expected {expected}",
                Unexpected(found)
            ),
            Self::InvalidValue { found, expected } => write!(
                formatter,
                "invalid value: {}, expected {expected}",
                Unexpected(found)
            ),
            Self::UnknownVariant { variant, expected } => write!(
                formatter,
                "unknown variant `{variant}`, {}",
                OneOf(expected, "variants")
            ),
            Self::UnknownField { field, expected } => write!(
                formatter,
                "unknown field `{field}`, {}",
                OneOf(expected, "fields")
            ),
            Self::MissingField(field) => write!(formatter, "missing field `{field}`"),
            Self::DuplicateField(field) => write!(formatter, "duplicate field `{field}`"),
            Self::Json(error) => error.fmt(formatter),
        }
    }
}

/// Original scalar data or container kind; never a clone of an input subtree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Found {
    Null,
    Boolean(bool),
    Integer(String),
    Number(String),
    Text(String),
    Array { length: usize },
    Object { length: usize },
}

impl Found {
    fn owned_bytes(&self) -> usize {
        match self {
            Self::Integer(text) | Self::Number(text) | Self::Text(text) => text.capacity(),
            Self::Null | Self::Boolean(_) | Self::Array { .. } | Self::Object { .. } => 0,
        }
    }
    fn capture<S: Admission + ?Sized>(
        value: &Value,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        Ok(match value {
            Value::Null => Self::Null,
            Value::Bool(value) => Self::Boolean(*value),
            Value::Number(number) if number.is_integer() => {
                Self::Integer(memory.string(number.lexeme())?)
            }
            Value::Number(number) => Self::Number(memory.string(number.lexeme())?),
            Value::String(text) => Self::Text(memory.string(text)?),
            Value::Array(items) => Self::Array {
                length: items.len(),
            },
            Value::Object(object) => Self::Object {
                length: object.len(),
            },
        })
    }
}

impl purrdf_hash::error::StructuredError for Found {
    purrdf_hash::error_visit! {
        Null => "json-record-found-null" {},
        Boolean(value) => "json-record-found-boolean" { "value" => Boolean(*value) },
        Integer(lexical) => "json-record-found-integer" { "lexical" => Text(lexical) },
        Number(lexical) => "json-record-found-number" { "lexical" => Text(lexical) },
        Text(value) => "json-record-found-string" { "value" => Text(value) },
        Array { length } => "json-record-found-array" { "length" => Unsigned(*length as u128) },
        Object { length } => "json-record-found-object" { "length" => Unsigned(*length as u128) },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Failure {
    Message,
    InvalidType {
        found: Found,
        expected: String,
    },
    InvalidValue {
        found: Found,
        expected: String,
    },
    UnknownVariant {
        variant: String,
        expected: Vec<String>,
    },
    UnknownField {
        field: String,
        expected: Vec<String>,
    },
    MissingField(String),
    DuplicateField(String),
    Json(Error),
    Storage(StorageError),
}

impl Failure {
    pub(super) const fn storage_error(&self) -> Option<StorageError> {
        match self {
            Self::Storage(storage) => Some(*storage),
            Self::Json(cause) => match cause.kind() {
                crate::json::ErrorKind::Storage(storage)
                | crate::json::ErrorKind::Escape(
                    crate::json_escape::JsonEscapeErrorKind::Storage(storage),
                ) => Some(storage),
                _ => None,
            },
            Self::Message
            | Self::InvalidType { .. }
            | Self::InvalidValue { .. }
            | Self::UnknownVariant { .. }
            | Self::UnknownField { .. }
            | Self::MissingField(_)
            | Self::DuplicateField(_) => None,
        }
    }

    fn owned_bytes(&self) -> Result<usize, StorageError> {
        match self {
            Self::InvalidType { found, expected } | Self::InvalidValue { found, expected } => found
                .owned_bytes()
                .checked_add(expected.capacity())
                .ok_or(StorageError::SizeOverflow),
            Self::UnknownVariant {
                variant: field,
                expected,
            }
            | Self::UnknownField { field, expected } => {
                let buffer = core::alloc::Layout::array::<String>(expected.capacity())
                    .map_err(|_| StorageError::SizeOverflow)?
                    .size();
                expected.iter().try_fold(
                    field
                        .capacity()
                        .checked_add(buffer)
                        .ok_or(StorageError::SizeOverflow)?,
                    |bytes, text| {
                        bytes
                            .checked_add(text.capacity())
                            .ok_or(StorageError::SizeOverflow)
                    },
                )
            }
            Self::MissingField(field) | Self::DuplicateField(field) => Ok(field.capacity()),
            Self::Message | Self::Json(_) | Self::Storage(_) => Ok(0),
        }
    }
}

impl purrdf_hash::error::StructuredError for Failure {
    purrdf_hash::error_visit! {
        Message => "json-record-custom" {},
        InvalidType { found, expected } => "json-record-invalid-type" {
            "found" => Record(found), "expected" => Text(expected),
        },
        InvalidValue { found, expected } => "json-record-invalid-value" {
            "found" => Record(found), "expected" => Text(expected),
        },
        UnknownVariant { variant, expected } => "json-record-unknown-variant" {
            "variant" => Text(variant), "expected" => List(expected.iter().map(|name| purrdf_hash::error::ErrorValue::Text(name))),
        },
        UnknownField { field, expected } => "json-record-unknown-field" {
            "field" => Text(field), "expected" => List(expected.iter().map(|name| purrdf_hash::error::ErrorValue::Text(name))),
        },
        MissingField(field) => "json-record-missing-field" { "field" => Text(field) },
        DuplicateField(field) => "json-record-duplicate-field" { "field" => Text(field) },
        Json(cause) => "json-record-json" { "cause" => Cause(cause) },
        Storage(cause) => "json-record-storage" { "cause" => Cause(cause) },
    }
}

impl DecodeError {
    /// Destroy this diagnostic, then refund its originally admitted buffers.
    /// The diagnostic must belong to this same memory account.
    ///
    /// # Errors
    /// Returns checked layout overflow or the original account's shrink refusal.
    pub fn release_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let fields = self.failure.owned_bytes()?;
        let bytes = self
            .pointer
            .capacity()
            .checked_add(self.message.capacity())
            .and_then(|bytes| bytes.checked_add(fields))
            .ok_or(StorageError::SizeOverflow)?;
        drop(self);
        memory.release_bytes(bytes)
    }
    pub(super) fn from_reason_native(
        reason: DecodeReason<'_>,
        allocation: Option<&dyn RecordAllocation>,
    ) -> Self {
        let result = match allocation {
            Some(allocation) => allocation.diagnostic(reason),
            None => Self::capture_with_memory(reason, &mut Memory::new(&mut Resident)),
        };
        result.unwrap_or_else(Self::from_storage)
    }

    /// Retain native reason fields and their presentation under one original account.
    /// Partial buffers die before their grant is refunded on failure. A JSON
    /// storage cause stays inline with its original byte offset and needs no copy.
    ///
    /// # Errors
    /// Returns the first physical admission, allocation or presentation failure.
    pub fn capture_with_memory<S: Admission + ?Sized>(
        reason: DecodeReason<'_>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        if let DecodeReason::Json(cause) = reason {
            let failure = Failure::Json(cause);
            if failure.storage_error().is_some() {
                return Ok(Self {
                    pointer: String::new(),
                    message: String::new(),
                    failure,
                });
            }
        }
        memory.scope(|memory| {
            // Keep the original rendering order and admit its actual growth.
            // A later typed-field refusal drops this text before refunding it.
            let message = memory.format(&reason)?;
            let failure = match reason {
                DecodeReason::Message(_) => Failure::Message,
                DecodeReason::InvalidType { found, expected } => Failure::InvalidType {
                    found: Found::capture(found, memory)?,
                    expected: memory.string(expected)?,
                },
                DecodeReason::InvalidValue { found, expected } => Failure::InvalidValue {
                    found: Found::capture(found, memory)?,
                    expected: memory.string(expected)?,
                },
                DecodeReason::UnknownVariant { variant, expected } => Failure::UnknownVariant {
                    variant: memory.string(variant)?,
                    expected: memory.try_map(expected, |name, memory| memory.string(name))?,
                },
                DecodeReason::UnknownField { field, expected } => Failure::UnknownField {
                    field: memory.string(field)?,
                    expected: memory.try_map(expected, |name, memory| memory.string(name))?,
                },
                DecodeReason::MissingField(field) => Failure::MissingField(memory.string(field)?),
                DecodeReason::DuplicateField(field) => {
                    Failure::DuplicateField(memory.string(field)?)
                }
                DecodeReason::Json(cause) => Failure::Json(cause),
            };
            Ok(Self {
                pointer: String::new(),
                message,
                failure,
            })
        })
    }
}

impl purrdf_hash::error::StructuredError for DecodeError {
    purrdf_hash::error_visit! {
        @record self { pointer, message, failure } => "json-record-decode" {
            "pointer" => Text(&self.pointer), "cause" => Cause(&self.failure),
            // Native presentation is a cache, not a second semantic cause.
            "external_message" => Value(if matches!(self.failure, Failure::Message) {
                purrdf_hash::error::ErrorValue::Message(&self.message)
            } else { purrdf_hash::error::ErrorValue::Absent }),
        }
    }
}
