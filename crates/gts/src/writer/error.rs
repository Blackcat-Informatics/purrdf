// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native writer option and catalog refusals.

use core::fmt;
use std::collections::BTreeSet;

/// Invalid frame, catalog or writer configuration, before authoring any bytes.
#[derive(Debug)]
#[non_exhaustive]
pub enum FrameError {
    /// A codec name occurs more than once.
    DuplicateCodecName {
        /// Original codec or dictionary name.
        name: String,
    },
    /// A codec ID occurs more than once.
    DuplicateCodecId {
        /// Original catalog identifier.
        id: i64,
    },
    /// No ID follows the largest base catalog ID.
    DictionaryIdStart {
        /// Largest ID in the base catalog.
        maximum: i64,
    },
    /// Dictionary count times codec count overflows the entry count.
    DictionaryEntryCount {
        /// Number of dictionary-capable codecs.
        codecs: usize,
        /// Number of pinned dictionaries.
        dictionaries: usize,
    },
    /// Assigning the remaining dictionary entries would overflow their IDs.
    DictionaryIdRange {
        /// Last assigned catalog ID.
        last: i64,
        /// Number of entries still requiring an ID.
        remaining: usize,
    },
    /// Only the streamable layout claim is supported.
    Layout {
        /// Original caller layout claim.
        actual: Option<String>,
    },
    /// Dictionary names must be nonempty.
    EmptyDictionaryName,
    /// A dictionary name occurs more than once.
    DuplicateDictionaryName {
        /// Original codec or dictionary name.
        name: String,
    },
    /// A catalog-level zstd level needs a zstd codec.
    CatalogLevel {
        /// Requested compression level.
        level: i32,
    },
    /// An adopted catalog contains repeated IDs.
    AppendDuplicateId {
        /// Original catalog identifier.
        id: i64,
    },
    /// An adopted catalog contains repeated codec/dictionary keys.
    AppendDuplicateEntry {
        /// Original codec or dictionary name.
        name: String,
        /// Original optional dictionary name.
        dictionary: Option<String>,
    },
    /// The adopted catalog declares more than one compression level.
    AppendLevels {
        /// Distinct declared levels in numeric order.
        levels: BTreeSet<i32>,
    },
    /// Payload and raw sources cannot both be supplied.
    ConflictingSources,
    /// Transforms or encryption require a data source.
    MissingSource {
        /// Number of requested transforms.
        transforms: usize,
        /// Whether encryption was requested.
        encrypted: bool,
    },
    /// A frame's zstd level needs a zstd transform.
    FrameLevel {
        /// Requested compression level.
        level: i32,
    },
    /// A frame's compression level must agree with the catalog.
    LevelMismatch {
        /// Requested frame compression level.
        requested: i32,
        /// Compression level declared by the catalog.
        declared: i32,
    },
    /// The named dictionary is not pinned by this pack.
    UnknownDictionary {
        /// Original codec or dictionary name.
        name: String,
    },
    /// A dictionary needs a zstd transform.
    DictionaryTransform {
        /// Original codec or dictionary name.
        name: String,
    },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCodecName { name } => write!(
                f,
                "duplicate codec name {name:?} in the writer catalog (§5 \"cat\" is a map)"
            ),
            Self::DuplicateCodecId { id } => write!(
                f,
                "duplicate codec id {id} in the writer catalog (§5 \"cat\" is a map)"
            ),
            Self::DictionaryIdStart { .. } => {
                f.write_str("writer catalog ids leave no range for dictionary-bound entries")
            }
            Self::DictionaryEntryCount { .. } => {
                f.write_str("writer catalog entry count overflows while assigning dictionaries")
            }
            Self::DictionaryIdRange { .. } => {
                f.write_str("writer catalog ids overflow while assigning dictionary-bound entries")
            }
            Self::Layout { actual } => write!(f, "unsupported layout claim {actual:?} (§3.3)"),
            Self::EmptyDictionaryName => {
                f.write_str("an in-band dictionary name must be non-empty (§5 \"dct\")")
            }
            Self::DuplicateDictionaryName { name } => write!(
                f,
                "duplicate in-band dictionary name {name:?} (§5 \"dct\" is a map)"
            ),
            Self::CatalogLevel { .. } => {
                f.write_str("zstd_level requires a zstd or zstd-rsyncable catalog entry")
            }
            Self::AppendDuplicateId { id } => {
                write!(f, "cannot append: catalog id {id} appears more than once")
            }
            Self::AppendDuplicateEntry { name, dictionary } => write!(
                f,
                "cannot append: catalog entry {:?} appears more than once",
                (name, dictionary)
            ),
            Self::AppendLevels { levels } => write!(
                f,
                "cannot append: the segment catalog declares conflicting zstd levels {levels:?} (§8.5 \"level\")"
            ),
            Self::ConflictingSources => f.write_str("payload and raw are mutually exclusive"),
            Self::MissingSource { .. } => {
                f.write_str("transform/encrypt requires a payload or raw source")
            }
            Self::FrameLevel { .. } => {
                f.write_str("zstd_level requires a zstd or zstd-rsyncable transform")
            }
            Self::LevelMismatch {
                requested,
                declared,
            } => write!(
                f,
                "frame zstd_level {requested} contradicts the pack's declared level {declared} (§8.5 catalog \"level\")"
            ),
            Self::UnknownDictionary { name } => write!(
                f,
                "frame names in-band dictionary {name:?}, which this pack does not pin (§5 header \"dct\")"
            ),
            Self::DictionaryTransform { .. } => {
                f.write_str("a frame dictionary requires a zstd or zstd-rsyncable transform")
            }
        }
    }
}

impl std::error::Error for FrameError {}

impl purrdf_hash::error::StructuredError for FrameError {
    purrdf_hash::error_visit! {
        DuplicateCodecName { name } => "gts-writer-duplicate-codec-name" { "name" => Text(name) },
        DuplicateCodecId { id } => "gts-writer-duplicate-codec-id" { "id" => Signed(i128::from(*id)) },
        DictionaryIdStart { maximum } => "gts-writer-dictionary-id-start" { "maximum" => Signed(i128::from(*maximum)) },
        DictionaryEntryCount { codecs, dictionaries } => "gts-writer-dictionary-entry-count" {
            "codecs" => Unsigned(*codecs as u128), "dictionaries" => Unsigned(*dictionaries as u128)
        },
        DictionaryIdRange { last, remaining } => "gts-writer-dictionary-id-range" {
            "last" => Signed(i128::from(*last)), "remaining" => Unsigned(*remaining as u128)
        },
        Layout { actual } => "gts-writer-layout" {
            "actual" => Value(actual.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Text))
        },
        EmptyDictionaryName => "gts-writer-empty-dictionary-name" {},
        DuplicateDictionaryName { name } => "gts-writer-duplicate-dictionary-name" { "name" => Text(name) },
        CatalogLevel { level } => "gts-writer-catalog-level" { "level" => Signed(i128::from(*level)) },
        AppendDuplicateId { id } => "gts-writer-append-duplicate-id" { "id" => Signed(i128::from(*id)) },
        AppendDuplicateEntry { name, dictionary } => "gts-writer-append-duplicate-entry" {
            "name" => Text(name),
            "dictionary" => Value(dictionary.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Text))
        },
        AppendLevels { levels } => "gts-writer-append-levels" {
            "levels" => List(levels.iter().map(|level| purrdf_hash::error::ErrorValue::Signed(i128::from(*level))))
        },
        ConflictingSources => "gts-writer-conflicting-sources" {},
        MissingSource { transforms, encrypted } => "gts-writer-missing-source" { "transforms" => Unsigned(*transforms as u128), "encrypted" => Boolean(*encrypted) },
        FrameLevel { level } => "gts-writer-frame-level" { "level" => Signed(i128::from(*level)) },
        LevelMismatch { requested, declared } => "gts-writer-level-mismatch" { "requested" => Signed(i128::from(*requested)), "declared" => Signed(i128::from(*declared)) },
        UnknownDictionary { name } => "gts-writer-unknown-dictionary" { "name" => Text(name) },
        DictionaryTransform { name } => "gts-writer-dictionary-transform" { "name" => Text(name) },
    }
}
