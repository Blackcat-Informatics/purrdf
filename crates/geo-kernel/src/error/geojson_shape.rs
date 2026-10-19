// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original GeoJSON shape decisions and coordinate operands.

use crate::CoordDim;
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_lex::json::Kind;

/// Native GeoJSON shape refusal. Borrowed tokens are copied only inside admission.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GeoJsonShapeError<T = String> {
    /// A type member was not a string.
    TypeNotString {
        /// Original JSON kind.
        found: Option<Kind>,
    },
    /// The object has no type member.
    MissingType,
    /// The object has multiple type members.
    RepeatedType {
        /// Original member count.
        count: usize,
    },
    /// A Feature wrapper is not a Geometry object.
    Feature {
        /// Original decoded type name.
        type_name: T,
    },
    /// The type name is unknown.
    UnknownType {
        /// Original decoded type name.
        type_name: T,
    },
    /// A geometry carries the other geometry family's shape member.
    ForeignShapeMember {
        /// Original member.
        member: &'static str,
        /// Original type name.
        type_name: T,
        /// Required shape member.
        expected: &'static str,
    },
    /// A shape member is null.
    NullMember {
        /// Original member.
        member: &'static str,
        /// Original type name.
        type_name: T,
    },
    /// A required shape member is missing or repeated.
    MemberCount {
        /// Original member.
        member: &'static str,
        /// Original type name.
        type_name: T,
        /// Original count.
        count: usize,
    },
    /// A geometry value is not an object.
    ExpectedObject {
        /// Original JSON kind.
        found: Option<Kind>,
    },
    /// A position or shape member is not an array.
    ExpectedArray {
        /// Original production.
        production: &'static str,
        /// Original JSON kind.
        found: Option<Kind>,
    },
    /// This type does not carry coordinates.
    CoordinateType {
        /// Original type name.
        type_name: T,
    },
    /// A position does not contain two or three elements.
    PositionLength {
        /// Original element count.
        count: usize,
    },
    /// Positions disagree on coordinate dimension.
    MixedDimensions {
        /// First fixed dimension.
        first: CoordDim,
        /// Original conflicting dimension.
        second: CoordDim,
    },
    /// A position ordinate is not a number.
    ExpectedNumber {
        /// Original one-based ordinate index.
        index: usize,
        /// Original JSON kind.
        found: Option<Kind>,
    },
    /// An exact ordinate's exponent cannot be represented.
    InvalidNumber {
        /// Original one-based ordinate index.
        index: usize,
        /// Original number lexeme.
        lexical: T,
    },
    /// Collection members disagree on coordinate dimension.
    CollectionDimensions {
        /// First fixed dimension.
        first: CoordDim,
        /// Original conflicting dimension.
        second: CoordDim,
    },
}

impl purrdf_hash::error::StructuredError for GeoJsonShapeError {
    purrdf_hash::error_visit! {
        TypeNotString { found } => "geojson-type-not-string" { "found" => Value(found.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |kind| purrdf_hash::error::ErrorValue::Record(kind))) },
        MissingType => "geojson-missing-type" {},
        RepeatedType { count } => "geojson-repeated-type" { "count" => Unsigned(*count as u128) },
        Feature { type_name } => "geojson-feature-wrapper" { "type" => Text(type_name) },
        UnknownType { type_name } => "geojson-unknown-type" { "type" => Text(type_name) },
        ForeignShapeMember { member, type_name, expected } => "geojson-foreign-shape-member" {
            "member" => Code(member), "type" => Text(type_name), "expected" => Code(expected)
        },
        NullMember { member, type_name } => "geojson-null-member" { "member" => Code(member), "type" => Text(type_name) },
        MemberCount { member, type_name, count } => "geojson-member-count" { "member" => Code(member), "type" => Text(type_name), "count" => Unsigned(*count as u128) },
        ExpectedObject { found } => "geojson-expected-object" { "found" => Value(found.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |kind| purrdf_hash::error::ErrorValue::Record(kind))) },
        ExpectedArray { production, found } => "geojson-expected-array" { "production" => Code(production), "found" => Value(found.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |kind| purrdf_hash::error::ErrorValue::Record(kind))) },
        CoordinateType { type_name } => "geojson-coordinate-type" { "type" => Text(type_name) },
        PositionLength { count } => "geojson-position-length" { "count" => Unsigned(*count as u128) },
        MixedDimensions { first, second } => "geojson-mixed-dimensions" { "first" => Record(first), "second" => Record(second) },
        ExpectedNumber { index, found } => "geojson-expected-number" { "index" => Unsigned(*index as u128), "found" => Value(found.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |kind| purrdf_hash::error::ErrorValue::Record(kind))) },
        InvalidNumber { index, lexical } => "geojson-invalid-number" { "index" => Unsigned(*index as u128), "lexical" => Text(lexical) },
        CollectionDimensions { first, second } => "geojson-collection-dimensions" { "first" => Record(first), "second" => Record(second) },
    }
}

impl GeoJsonShapeError<&str> {
    fn into_owned<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<GeoJsonShapeError, StorageError> {
        Ok(match self {
            Self::TypeNotString { found } => GeoJsonShapeError::TypeNotString { found },
            Self::MissingType => GeoJsonShapeError::MissingType,
            Self::RepeatedType { count } => GeoJsonShapeError::RepeatedType { count },
            Self::Feature { type_name } => GeoJsonShapeError::Feature {
                type_name: memory.string(type_name)?,
            },
            Self::UnknownType { type_name } => GeoJsonShapeError::UnknownType {
                type_name: memory.string(type_name)?,
            },
            Self::ForeignShapeMember {
                member,
                type_name,
                expected,
            } => GeoJsonShapeError::ForeignShapeMember {
                member,
                type_name: memory.string(type_name)?,
                expected,
            },
            Self::NullMember { member, type_name } => GeoJsonShapeError::NullMember {
                member,
                type_name: memory.string(type_name)?,
            },
            Self::MemberCount {
                member,
                type_name,
                count,
            } => GeoJsonShapeError::MemberCount {
                member,
                type_name: memory.string(type_name)?,
                count,
            },
            Self::ExpectedObject { found } => GeoJsonShapeError::ExpectedObject { found },
            Self::ExpectedArray { production, found } => {
                GeoJsonShapeError::ExpectedArray { production, found }
            }
            Self::CoordinateType { type_name } => GeoJsonShapeError::CoordinateType {
                type_name: memory.string(type_name)?,
            },
            Self::PositionLength { count } => GeoJsonShapeError::PositionLength { count },
            Self::MixedDimensions { first, second } => {
                GeoJsonShapeError::MixedDimensions { first, second }
            }
            Self::ExpectedNumber { index, found } => {
                GeoJsonShapeError::ExpectedNumber { index, found }
            }
            Self::InvalidNumber { index, lexical } => GeoJsonShapeError::InvalidNumber {
                index,
                lexical: memory.string(lexical)?,
            },
            Self::CollectionDimensions { first, second } => {
                GeoJsonShapeError::CollectionDimensions { first, second }
            }
        })
    }

    pub(crate) fn capture(
        self,
        storage: Option<&crate::carrier::CarrierStorage<'_>>,
    ) -> super::GeoError {
        crate::carrier::with_storage(storage, |memory| {
            memory.try_scope(|memory| {
                let cause = self.into_owned(memory)?;
                memory.boxed(cause).map(super::GeoError::GeoJsonShape)
            })
        })
        .unwrap_or_else(super::GeoError::Storage)
    }
}

impl GeoJsonShapeError {
    pub(crate) fn owned_bytes(&self) -> Result<usize, StorageError> {
        let text = match self {
            Self::Feature { type_name }
            | Self::UnknownType { type_name }
            | Self::ForeignShapeMember {
                member: _,
                type_name,
                expected: _,
            }
            | Self::NullMember {
                member: _,
                type_name,
            }
            | Self::MemberCount {
                member: _,
                type_name,
                count: _,
            }
            | Self::CoordinateType { type_name } => type_name.capacity(),
            Self::InvalidNumber { index: _, lexical } => lexical.capacity(),
            Self::TypeNotString { found: _ }
            | Self::MissingType
            | Self::RepeatedType { count: _ }
            | Self::ExpectedObject { found: _ }
            | Self::ExpectedArray {
                production: _,
                found: _,
            }
            | Self::PositionLength { count: _ }
            | Self::MixedDimensions {
                first: _,
                second: _,
            }
            | Self::ExpectedNumber { index: _, found: _ }
            | Self::CollectionDimensions {
                first: _,
                second: _,
            } => 0,
        };
        core::alloc::Layout::new::<Self>()
            .size()
            .checked_add(text)
            .ok_or(StorageError::SizeOverflow)
    }
}

impl core::fmt::Display for GeoJsonShapeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use crate::geojson::{GEOMETRY_TYPES, kind_name};
        match self {
            Self::TypeNotString { found } => write!(f, "the `type` member of a GeoJSON Geometry object is a string, but this one is {}", kind_name(*found)),
            Self::MissingType => f.write_str("a GeoJSON Geometry object has a `type` member naming its geometry type; this object has none"),
            Self::RepeatedType { count } => write!(f, "this GeoJSON object has {count} `type` members, so it names no single geometry type; RFC 8259 permits the repetition but resolving it by position would be a silent choice"),
            Self::Feature { type_name } => write!(f, "a geo:geoJSONLiteral is a GeoJSON Geometry object and `{type_name}` is not one of them: GeoSPARQL 1.1 Requirement 25 admits only {GEOMETRY_TYPES}. Write the geometry itself as the literal rather than the {type_name} that wraps it"),
            Self::UnknownType { type_name } => write!(f, "`{type_name}` is not a GeoJSON geometry type; RFC 7946 defines {GEOMETRY_TYPES}"),
            Self::ForeignShapeMember { member, type_name, expected } => write!(f, "this GeoJSON {type_name} carries a `{member}` member, which RFC 7946 defines for the other family of geometry types; a {type_name} states its shape in `{expected}`, and an object claiming both is a contradiction rather than a Geometry object with a foreign member"),
            Self::NullMember { member, type_name } => write!(f, "the `{member}` member of this GeoJSON {type_name} is null; RFC 7946 gives null no meaning there, and an empty geometry is written `\"{member}\":[]`"),
            Self::MemberCount { member, type_name, count: 0 } => write!(f, "a GeoJSON {type_name} has a `{member}` member; this one has none, and RFC 7946 defines the geometry entirely by it"),
            Self::MemberCount { member, type_name, count } => write!(f, "this GeoJSON {type_name} has {count} `{member}` members, which denote different geometries; the literal is ambiguous and is refused rather than resolved by position"),
            Self::ExpectedObject { found } => write!(f, "a geo:geoJSONLiteral is an RFC 7946 Geometry object, but this is {}", kind_name(*found)),
            Self::ExpectedArray { production, found } => write!(f, "{production} is a JSON array in RFC 7946, but this is {}", kind_name(*found)),
            Self::CoordinateType { type_name } => write!(f, "`{type_name}` is not a GeoJSON geometry type with coordinates"),
            Self::PositionLength { count: count @ (0 | 1) } => write!(f, "a GeoJSON position is an array of two or three numbers (longitude, latitude and an optional altitude); this one has {count}"),
            Self::PositionLength { count } => write!(f, "a GeoJSON position has at most three numbers; RFC 7946 §3.1.1 says \"Implementations SHOULD NOT extend positions beyond three elements\", so a position of {count} elements is refused rather than silently truncated. The RFC gives the extra elements no meaning and GeoJSON has no measure ordinate for them to become, so ignoring them would discard a number without saying so; an extension that used a fourth element would need its own datatype"),
            Self::MixedDimensions { first, second } => write!(f, "this GeoJSON geometry mixes {}-element and {}-element positions; the geometry model carries one coordinate dimension for a whole geometry, so a geometry whose positions disagree about it has no dimension to report to `geof:coordinateDimension`. RFC 7946 does not itself forbid the mixture — this is purrdf-geo's model refusing rather than silently dropping or inventing an altitude", first.ordinates(), second.ordinates()),
            Self::ExpectedNumber { index: _, found } => write!(f, "an ordinate of a GeoJSON position is a number, but this one is {}", kind_name(*found)),
            Self::InvalidNumber { index: _, lexical } => write!(f, "the ordinate `{lexical}` is a valid JSON number but its exponent is past the exact decimal reader's cap, so it names no value this crate can hold"),
            Self::CollectionDimensions { first, second } => write!(f, "this GeoJSON GeometryCollection mixes {}-element and {}-element positions; the geometry model carries one coordinate dimension for a whole geometry, so a collection whose members disagree about it has no dimension to report to `geof:coordinateDimension`", first.ordinates(), second.ordinates()),
        }
    }
}
impl std::error::Error for GeoJsonShapeError {}

#[cfg(test)]
mod tests {
    use super::GeoJsonShapeError;
    use crate::GeoError;
    use purrdf_hash::error::ErrorValue;
    use purrdf_lex::allocation::{Resident, StorageError};
    use purrdf_testkit::error_capture::Capture;

    #[test]
    fn admitted_shape_copies_only_original_tokens_and_refunds_failed_construction() {
        let token = "é未知";
        let mut resident = Resident;
        let storage = crate::carrier::CarrierStorage::resume(&mut resident, 0);
        let error = GeoJsonShapeError::UnknownType { type_name: token }.capture(Some(&storage));
        let GeoError::GeoJsonShape(cause) = &error else {
            panic!("native shape cause")
        };
        assert_eq!(
            &**cause,
            &GeoJsonShapeError::UnknownType {
                type_name: token.to_owned()
            }
        );
        assert_eq!(storage.admitted_bytes(), cause.owned_bytes().unwrap());
        drop(error);

        let mut resident = Resident;
        let parent = usize::MAX - token.len();
        let storage = crate::carrier::CarrierStorage::resume(&mut resident, parent);
        let error = GeoJsonShapeError::UnknownType { type_name: token }.capture(Some(&storage));
        assert_eq!(error, GeoError::Storage(StorageError::SizeOverflow));
        assert_eq!(storage.admitted_bytes(), parent);
    }

    #[test]
    fn original_coordinate_lexeme_and_full_width_indices_remain_native() {
        let lexical = "1e99999999999999999999999999999".to_owned();
        let error = crate::geojson::parse(
            &format!(r#"{{"type":"Point","coordinates":[0,{lexical}]}}"#),
            &crate::Crs::new("http://example.org/crs").unwrap(),
        )
        .unwrap_err();
        let GeoError::GeoJsonShape(cause) = &error else {
            panic!("native coordinate cause")
        };
        assert_eq!(
            &**cause,
            &GeoJsonShapeError::InvalidNumber {
                index: 2,
                lexical: lexical.clone()
            }
        );
        drop(lexical);
        assert_eq!(
            Capture::read(&error).child("cause").code,
            "geojson-invalid-number"
        );

        let error = GeoJsonShapeError::InvalidNumber {
            index: usize::MAX,
            lexical: "1e999",
        }
        .capture(None);
        let fields = Capture::read(&error).child("cause");
        assert!(
            matches!(fields.field("index"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
        );
        assert!(matches!(fields.field("lexical"), ErrorValue::Text("1e999")));
    }
}
