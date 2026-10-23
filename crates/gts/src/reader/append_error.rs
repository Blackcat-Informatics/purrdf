// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native append admission failures, before a writer adopts the segment.

use core::fmt;
use purrdf_lex::cbor::Value;

/// A refusal to recover an existing segment's authoring state.
#[derive(Debug)]
#[non_exhaustive]
pub enum AppendError {
    /// An incomplete trailing item starts at this byte offset.
    Torn {
        /// Byte offset of the incomplete trailing item.
        offset: usize,
        /// Original item decoder refusal.
        cause: purrdf_lex::cbor::DecodeError,
    },
    /// No segment header was found.
    MissingHeader,
    /// A header or the final item is not a header-compatible map.
    Header(crate::wire::HeaderError),
    /// Original malformed catalog key and entry.
    CatalogEntry {
        /// Original catalog identifier.
        id: Value,
        /// Original rejected catalog value.
        entry: Value,
    },
    /// A declared compression level is not an integer.
    CatalogLevel {
        /// Original catalog identifier.
        id: i128,
        /// Original rejected compression level.
        value: Value,
    },
    /// A declared compression level does not fit the authoring type.
    LevelRange {
        /// Original catalog identifier.
        id: i128,
        /// Original rejected value, before narrowing.
        actual: i128,
    },
    /// A catalog ID does not fit the authoring type.
    IdRange {
        /// Original rejected value, before narrowing.
        actual: i128,
    },
    /// Original malformed dictionary name and data.
    DictionaryEntry {
        /// Original dictionary name.
        name: Value,
        /// Original dictionary payload.
        bytes: Value,
    },
    /// A catalog entry names a dictionary absent from the header.
    MissingDictionary {
        /// Original catalog identifier.
        id: i64,
        /// Original dictionary name.
        name: String,
    },
    /// The final item has no byte-string ID; retain its original field, if any.
    MissingHead {
        /// Original rejected value, before narrowing.
        actual: Option<Value>,
    },
}

impl fmt::Display for AppendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Torn { offset, .. } => write!(
                f,
                "cannot append to a file with a torn append at byte {offset}"
            ),
            Self::MissingHeader => {
                f.write_str("cannot append: the file carries no GTS segment header")
            }
            Self::Header(cause) => write!(f, "cannot append: {cause}"),
            Self::CatalogEntry { .. } => f.write_str("cannot append: a catalog entry is malformed"),
            Self::CatalogLevel { .. } => f.write_str("cannot append: a catalog level is malformed"),
            Self::LevelRange { .. } => {
                f.write_str("cannot append: a catalog level is out of range")
            }
            Self::IdRange { .. } => f.write_str("cannot append: a catalog id is out of range"),
            Self::DictionaryEntry { .. } => {
                f.write_str("cannot append: a header \"dct\" entry is malformed")
            }
            Self::MissingDictionary { id, name } => write!(
                f,
                "cannot append: catalog id {id} names dictionary {name:?}, absent from the header \"dct\" map"
            ),
            Self::MissingHead { .. } => {
                f.write_str("cannot append: the file's last item carries no \"id\"")
            }
        }
    }
}

purrdf_lex::error_source!(AppendError => {
    Self::Header(cause) => cause,
    Self::Torn { cause, .. } => cause,
});

impl purrdf_hash::error::StructuredError for AppendError {
    purrdf_hash::error_visit! {
        Torn { offset, cause } => "gts-append-torn" { "offset" => ByteCount(*offset as u128), "cause" => Cause(cause) },
        MissingHeader => "gts-append-missing-header" {},
        Header(cause) => "gts-append-header" { "cause" => Cause(cause) },
        CatalogEntry { id, entry } => "gts-append-catalog-entry" { "id" => Record(id), "entry" => Record(entry) },
        CatalogLevel { id, value } => "gts-append-catalog-level" { "id" => Signed(*id), "value" => Record(value) },
        LevelRange { id, actual } => "gts-append-level-range" {
            "id" => Signed(*id), "actual" => Signed(*actual),
            "minimum" => Signed(i128::from(i32::MIN)), "maximum" => Signed(i128::from(i32::MAX))
        },
        IdRange { actual } => "gts-append-id-range" {
            "actual" => Signed(*actual), "minimum" => Signed(i128::from(i64::MIN)), "maximum" => Signed(i128::from(i64::MAX))
        },
        DictionaryEntry { name, bytes } => "gts-append-dictionary-entry" { "name" => Record(name), "bytes" => Record(bytes) },
        MissingDictionary { id, name } => "gts-append-missing-dictionary" { "id" => Signed(i128::from(*id)), "name" => Text(name) },
        MissingHead { actual } => "gts-append-missing-head" {
            "actual" => Value(actual.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Record(value)))
        },
    }
}
