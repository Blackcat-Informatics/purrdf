// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original certificate fields and native numeric decoding failures.

use purrdf_hash::error::ErrorValue;
use purrdf_lex::allocation::{CapacitySpace, StorageError};

/// A DL certificate that cannot describe the native run it claims to describe.
#[derive(Debug)]
#[non_exhaustive]
pub enum CertificateError {
    /// The first line is absent or names another grammar.
    Banner {
        /// Original first line.
        offered: Option<String>,
    },
    /// A completeness name is unknown.
    Completeness(String),
    /// An original field occurs more than once.
    Duplicate(&'static str),
    /// A required field was absent.
    Missing(&'static str),
    /// A schema-preparation obstruction has an unknown kind.
    SchemaObstruction(String),
    /// A schema-preparation obstruction carries extra arguments.
    SchemaArguments(String),
    /// A required obstruction counter is absent.
    MissingSchemaCounter {
        /// The semantic counter being read.
        field: &'static str,
    },
    /// An obstruction counter failed its native integer decoder.
    SchemaCounter {
        /// The semantic counter being read.
        field: &'static str,
        /// Original supplied spelling.
        offered: String,
        /// Original integer refusal.
        cause: core::num::ParseIntError,
    },
    /// A capacity space is absent or unknown.
    CapacitySpace {
        /// Original supplied name, absent when the field was missing.
        offered: Option<String>,
    },
    /// A required capacity count is absent.
    MissingCapacityCounter {
        /// The original field being read.
        field: &'static str,
    },
    /// A capacity count failed its native integer decoder.
    CapacityCounter {
        /// The original field being read.
        field: &'static str,
        /// Original supplied spelling.
        offered: String,
        /// Original integer refusal.
        cause: core::num::ParseIntError,
    },
    /// Counts do not constitute a capacity refusal, or trailing fields were present.
    CapacityRefusal {
        /// Original native capacity space.
        space: CapacitySpace,
        /// Original attempted count.
        attempted: u128,
        /// Original stated limit.
        limit: u128,
        /// Original arguments after the expected counts.
        extra: Vec<String>,
    },
    /// A storage-refusal name is unknown.
    Storage(String),
    /// A boundary names no construct in this build.
    Construct(String),
    /// An original run counter failed its native integer decoder.
    Counter {
        /// Original field name.
        field: &'static str,
        /// Original supplied spelling.
        offered: String,
        /// Original integer refusal.
        cause: core::num::ParseIntError,
    },
    /// A complete certificate also claims an obstruction.
    ObstructedComplete {
        /// Original native obstruction.
        obstruction: purrdf_entail::reasoner::SchemaObstruction,
    },
    /// Storage completeness and the original physical-cause field disagree.
    StorageCompleteness {
        /// Whether the original completeness declared storage refusal.
        storage_refused: bool,
        /// Original decoded physical cause, when present.
        cause: Option<StorageError>,
    },
}

impl core::fmt::Display for CertificateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Banner { .. } => write!(
                f,
                "a DL certificate opens with {:?}, and this one does not",
                super::DL_CERTIFICATE_BANNER
            ),
            Self::Completeness(offered) => write!(f, "unknown DL completeness \"{offered}\""),
            Self::Duplicate("storage-refusal") => {
                f.write_str("the certificate states storage-refusal twice")
            }
            Self::Duplicate(field) => write!(f, "the certificate states `{field}` twice"),
            Self::Missing("completeness") => f.write_str("the certificate states no completeness"),
            Self::Missing(field) => write!(f, "the certificate states no `{field}`"),
            Self::SchemaObstruction(_) => f.write_str("unknown schema preparation obstruction"),
            Self::SchemaArguments(_) => {
                f.write_str("extra schema preparation obstruction arguments")
            }
            Self::MissingSchemaCounter { .. } => {
                f.write_str("missing schema preparation obstruction argument")
            }
            Self::SchemaCounter { .. } => {
                f.write_str("invalid schema preparation obstruction counter")
            }
            Self::CapacitySpace { .. } => f.write_str("unknown DL capacity space"),
            Self::MissingCapacityCounter { field: "attempted" }
            | Self::CapacityCounter {
                field: "attempted", ..
            } => f.write_str("invalid DL capacity attempted count"),
            Self::MissingCapacityCounter { .. } | Self::CapacityCounter { .. } => {
                f.write_str("invalid DL capacity limit")
            }
            Self::CapacityRefusal { .. } => f.write_str("invalid DL capacity refusal"),
            Self::Storage(offered) => write!(f, "unknown DL storage refusal \"{offered}\""),
            Self::Construct(offered) => {
                write!(f, "\"{offered}\" names no construct this build knows")
            }
            Self::Counter { field, cause, .. } => {
                write!(f, "the certificate's `{field}` line: {cause}")
            }
            Self::ObstructedComplete { .. } => {
                f.write_str("a schema preparation obstruction requires incomplete certification")
            }
            Self::StorageCompleteness { .. } => {
                f.write_str("storage-refused completeness requires exactly one physical cause")
            }
        }
    }
}

impl core::error::Error for CertificateError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::SchemaCounter { cause, .. }
            | Self::CapacityCounter { cause, .. }
            | Self::Counter { cause, .. } => Some(cause),
            Self::Banner { .. }
            | Self::Completeness(_)
            | Self::Duplicate(_)
            | Self::Missing(_)
            | Self::SchemaObstruction(_)
            | Self::SchemaArguments(_)
            | Self::MissingSchemaCounter { .. }
            | Self::CapacitySpace { .. }
            | Self::MissingCapacityCounter { .. }
            | Self::CapacityRefusal { .. }
            | Self::Storage(_)
            | Self::Construct(_)
            | Self::ObstructedComplete { .. }
            | Self::StorageCompleteness { .. } => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for CertificateError {
    purrdf_hash::error_visit! {
        Banner { offered } => "reasoning-certificate-banner" {
            "expected" => Code(super::DL_CERTIFICATE_BANNER),
            "offered" => Value(offered.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
        },
        Completeness(offered) => "reasoning-certificate-completeness" { "offered" => Text(offered) },
        Duplicate(field) => "reasoning-certificate-duplicate" { "field" => Code(field) },
        Missing(field) => "reasoning-certificate-missing" { "field" => Code(field) },
        SchemaObstruction(offered) => "reasoning-certificate-obstruction" { "offered" => Text(offered) },
        SchemaArguments(offered) => "reasoning-certificate-schema-arguments" { "offered" => Text(offered) },
        MissingSchemaCounter { field } => "reasoning-certificate-missing-schema-counter" { "field" => Code(field) },
        SchemaCounter { field, offered, cause } => "reasoning-certificate-schema-counter" {
            "field" => Code(field), "offered" => Text(offered), "cause" => Cause(cause),
        },
        CapacitySpace { offered } => "reasoning-certificate-capacity-space" {
            "offered" => Value(offered.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
        },
        MissingCapacityCounter { field } => "reasoning-certificate-missing-capacity-counter" { "field" => Code(field) },
        CapacityCounter { field, offered, cause } => "reasoning-certificate-capacity-counter" {
            "field" => Code(field), "offered" => Text(offered), "cause" => Cause(cause),
        },
        CapacityRefusal { space, attempted, limit, extra } => "reasoning-certificate-capacity-refusal" {
            "space" => Code(space.as_str()), "attempted" => Unsigned(*attempted), "limit" => Unsigned(*limit),
            "extra" => List(extra.iter().map(|value| ErrorValue::Text(value))),
        },
        Storage(offered) => "reasoning-certificate-storage" { "offered" => Text(offered) },
        Construct(offered) => "reasoning-certificate-construct" { "offered" => Text(offered) },
        Counter { field, offered, cause } => "reasoning-certificate-counter" {
            "field" => Code(field), "offered" => Text(offered), "cause" => Cause(cause),
        },
        ObstructedComplete { obstruction } => "reasoning-certificate-obstructed-complete" { "obstruction" => Record(obstruction) },
        StorageCompleteness { storage_refused, cause } => "reasoning-certificate-storage-completeness" {
            "storage_refused" => Boolean(*storage_refused),
            "cause" => Value(cause.as_ref().map_or(ErrorValue::Absent, |cause| ErrorValue::Record(cause))),
        },
    }
}
