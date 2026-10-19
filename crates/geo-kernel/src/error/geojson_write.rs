// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original operands refused by the GeoJSON writer.

use purrdf_lex::allocation::{Admission, Memory, StorageError};

/// A geometry cannot be represented by the GeoJSON carrier.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GeoJsonWriteError<T = String> {
    /// Its original reference differs from the required reference.
    Reference {
        /// Original source reference identifier.
        actual: T,
        /// Required carrier reference identifier.
        required: T,
    },
    /// The carrier has no slot for the original measure ordinate.
    Measure {
        /// Original coordinate dimension.
        dimension: crate::CoordDim,
    },
    /// A MultiPoint contains an empty member that cannot be omitted.
    EmptyMember {
        /// Original zero-based member index.
        index: usize,
        /// Original member count.
        count: usize,
    },
}

impl purrdf_hash::error::StructuredError for GeoJsonWriteError {
    purrdf_hash::error_visit! {
        Reference { actual, required } => "geojson-write-reference" { "actual" => Iri(actual), "required" => Iri(required) },
        Measure { dimension } => "geojson-write-measure" { "dimension" => Record(dimension) },
        EmptyMember { index, count } => "geojson-write-empty-member" { "index" => Unsigned(*index as u128), "count" => Unsigned(*count as u128) },
    }
}

impl GeoJsonWriteError<&str> {
    pub(crate) fn refusal(
        self,
        writer: &mut crate::carrier::writer::CarrierWriter<'_, '_>,
    ) -> super::GeoError {
        writer.with_progress(|progress| progress.retained_error(|memory| self.capture(memory)))
    }

    pub(crate) fn capture<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<super::GeoError, StorageError> {
        memory.try_scope(|memory| {
            let cause = match self {
                Self::Reference { actual, required } => GeoJsonWriteError::Reference {
                    actual: memory.string(actual)?,
                    required: memory.string(required)?,
                },
                Self::Measure { dimension } => GeoJsonWriteError::Measure { dimension },
                Self::EmptyMember { index, count } => {
                    GeoJsonWriteError::EmptyMember { index, count }
                }
            };
            memory.boxed(cause).map(super::GeoError::GeoJsonWrite)
        })
    }
}

impl GeoJsonWriteError {
    pub(crate) fn owned_bytes(&self) -> Result<usize, StorageError> {
        let text = match self {
            Self::Reference { actual, required } => actual
                .capacity()
                .checked_add(required.capacity())
                .ok_or(StorageError::SizeOverflow)?,
            Self::Measure { dimension: _ } | Self::EmptyMember { index: _, count: _ } => 0,
        };
        core::alloc::Layout::new::<Self>()
            .size()
            .checked_add(text)
            .ok_or(StorageError::SizeOverflow)
    }
}

impl core::fmt::Display for GeoJsonWriteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Reference { actual, required } => write!(f, "this geometry is expressed in <{actual}> but a geo:geoJSONLiteral is fixed to <{required}>; RFC 7946 admits exactly one coordinate reference system and purrdf-geo reprojects nothing, so it refuses rather than relabelling the ordinates"),
            Self::Measure { dimension } => write!(f, "this geometry is {} and GeoJSON has no measure ordinate: RFC 7946 §3.1.1 defines a position as longitude, latitude and an optional altitude, so writing it would drop the measure. purrdf-geo refuses rather than silently discarding an ordinate; write it as a geo:wktLiteral instead", dimension.name()),
            Self::EmptyMember { index: _, count: _ } => f.write_str("this MULTIPOINT holds an empty member point and GeoJSON has no syntax for one: RFC 7946 has no empty position, and writing the member as `[]` or omitting it would change how many members the geometry has. purrdf-geo refuses rather than dropping a member; a MULTIPOINT with no members at all writes as `[]` and is unaffected"),
        }
    }
}
impl std::error::Error for GeoJsonWriteError {}
