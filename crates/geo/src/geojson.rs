// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `geo:geoJSONLiteral` codec: RFC 7946 Geometry objects in, exact geometry
//! out, and back again.
//!
//! # What a `geo:geoJSONLiteral` is, exactly
//!
//! GeoSPARQL 1.1 Clause 10.8.3, **Requirement 25**: "All `geo:geoJSONLiteral`
//! instances shall consist of the **Geometry objects** as defined in [RFC 7946]."
//! Geometry *objects* — the seven of them, `Point`, `MultiPoint`, `LineString`,
//! `MultiLineString`, `Polygon`, `MultiPolygon` and `GeometryCollection`. A
//! `Feature` or a `FeatureCollection` is a perfectly good GeoJSON document and is
//! **not** a `geo:geoJSONLiteral`; [`parse`] refuses one by name rather than
//! reaching inside for its `geometry` member, because a literal that silently
//! became its own sub-object would make `geof:geometryType` answer a question
//! about a value the dataset never stated.
//!
//! **Requirement 26** fixes GeoJSON to WGS 84 longitude/latitude: a GeoJSON
//! literal carries no coordinate reference system of its own. PurRDF mints no
//! vocabulary IRIs, so that system is a **parameter** of [`parse`] and of
//! [`write()`] rather than a constant compiled into this crate; the caller's
//! `GeoVocab` supplies the IRI, and there is no fabricated default to fall back
//! on.
//!
//! **Requirement 27**: "An empty RDFS Literal of type `geo:geoJSONLiteral` shall
//! be interpreted as an empty Geometry." So the empty string — and, per the
//! pattern `^\s*$|^\s*({)(.*)(})\s*$` that the shipped GeoSPARQL SHACL shape
//! uses, a whitespace-only lexical form — parses. It becomes an **empty
//! `GeometryCollection`**: a collection with no members is the only one of the
//! seven kinds that denotes the empty set without also asserting a kind the
//! literal never named. `POINT EMPTY` would have `geof:geometryType` report
//! `Point` for a literal that said nothing at all.
//!
//! # Exactness
//!
//! Coordinates are read through [`crate::json`], which keeps every number as its
//! source lexeme, and decided by [`crate::exact::Rat::parse_decimal`], which is
//! integer arithmetic end to end. No coordinate passes through an `f64` on the
//! way in, so `1.5`, `1.50` and `15e-1` produce the identical [`Geometry`] and a
//! forty-significant-digit ordinate survives intact.
//!
//! # What RFC 7946 forbids, and what it does not
//!
//! * A position is two or three numbers — longitude, latitude, and an optional
//!   altitude. §3.1.1 requires "two or more elements" and then says
//!   "Implementations SHOULD NOT extend positions beyond three elements", noting
//!   that some historically carried a fourth as a linear referencing measure and
//!   that "the interpretation and meaning of additional elements is beyond the
//!   scope of this specification, and additional elements MAY be ignored by
//!   parsers".
//!
//!   So a four-element position is *discouraged* rather than forbidden, and the
//!   RFC leaves this reader a genuine choice. This crate **refuses** it, and the
//!   reason is the alternative rather than the spec: the RFC assigns the fourth
//!   element no meaning, and GeoJSON has no measure ordinate for it to become, so
//!   "ignoring" it would mean silently discarding a number the author wrote on
//!   purpose and answering as though it had never been there. A refusal that
//!   names the element is the only outcome that cannot be mistaken for having
//!   honoured it. This is why a parsed geometry is always [`CoordDim::Xy`] or
//!   [`CoordDim::Xyz`], and why [`write_bare`] likewise **refuses** rather than
//!   dropping an `M` it cannot write.
//! * Every position in one geometry must have the same number of elements, which
//!   is what makes the single [`CoordDim`] of the geometry model well defined.
//!   A mixture is refused.
//! * An empty `coordinates` array is the empty geometry of that type, for every
//!   type: `{"type":"Point","coordinates":[]}` is `POINT EMPTY`.
//! * Foreign members are **ignored, not refused**. §6.1 explicitly allows a
//!   Geometry object to carry members the specification does not define, and
//!   `bbox` is defined but carries no geometry. Refusing them would reject
//!   conforming GeoJSON — the over-refusal that mirrors a silent drop — so
//!   anything that is not `type`, `coordinates` or `geometries` is passed over.
//!   The three members that *do* decide the geometry are refused when repeated,
//!   because two different `coordinates` arrays make the literal ambiguous and
//!   picking one by position would be a silent choice.

use crate::error::GeoError;
use crate::exact::Rat;
use crate::geom::{
    Coord, CoordDim, CoordSeq, Crs, Geometry, GeometryBody, GeometryKind, GeometryLiteral, Rings,
};
use crate::json::{self, JsonValue};

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// Parse a `geo:geoJSONLiteral` lexical form.
///
/// `crs` is the coordinate reference system RFC 7946 fixes GeoJSON to (GeoSPARQL
/// 1.1 Requirement 26). It is a parameter because PurRDF mints no vocabulary
/// IRIs: the caller's `GeoVocab` names the system, and this crate has no default
/// to invent.
///
/// An empty or whitespace-only lexical form is the empty geometry
/// (Requirement 27), represented as an empty `GeometryCollection`.
///
/// # Errors
///
/// [`GeoError::Literal`] when the lexical form is not an RFC 7946 Geometry
/// object: malformed JSON, a `Feature` or `FeatureCollection`, an unknown
/// `type`, a missing/null/repeated `coordinates` or `geometries` member, a
/// position that is not two or three numbers, positions of mixed length within
/// one geometry, or a body the geometry model refuses structurally (a
/// one-position line, a ring shorter than four positions or one that does not
/// close).
pub fn parse(lexical: &str, crs: &Crs) -> Result<GeometryLiteral, GeoError> {
    Ok(GeometryLiteral::new(crs.clone(), geometry_of(lexical)?))
}

/// The geometry a lexical form denotes, before a coordinate reference system is
/// attached to it.
fn geometry_of(lexical: &str) -> Result<Geometry, GeoError> {
    if lexical.trim().is_empty() {
        // Requirement 27. See the module docs for why the empty geometry is a
        // collection rather than a `POINT EMPTY`.
        return Ok(Geometry::empty(
            CoordDim::Xy,
            GeometryKind::GeometryCollection,
        ));
    }
    let value = json::parse(lexical)?;
    let pending = read_object(&value)?;
    let dim = pending.dim.unwrap_or(CoordDim::Xy);
    pending.into_geometry(dim).map_err(|error| {
        GeoError::literal(format!(
            "this geo:geoJSONLiteral is well-formed JSON but does not denote a geometry: {}",
            error.detail()
        ))
    })
}

/// A geometry read out of the JSON tree but not yet given a dimension.
///
/// The dimension of a geometry is decided by the positions inside it, and a
/// geometry with no positions decides nothing — `{"type":"Point","coordinates":[]}`
/// is as much an `XYZ` point as an `XY` one. A `GeometryCollection` therefore
/// cannot be built member by member: an empty member has to adopt whatever
/// dimension its *siblings* fix, or a collection of one empty point and one 3D
/// point would be refused for a disagreement that does not exist. So the tree is
/// read first, the dimension is unified across it, and only then is it
/// materialized.
struct Pending {
    /// `Some` once a position has fixed the dimension; `None` while the geometry
    /// has no positions at all.
    dim: Option<CoordDim>,
    shape: PendingShape,
}

enum PendingShape {
    /// Any of the six non-collection kinds, already complete. Boxed because a
    /// `GeometryBody` inlines a `SmallVec` of exact coordinates and is two
    /// orders of magnitude larger than the collection variant beside it.
    Body(Box<GeometryBody>),
    /// A `GeometryCollection`'s members, still awaiting the shared dimension.
    Collection(Vec<Pending>),
}

impl Pending {
    fn into_geometry(self, dim: CoordDim) -> Result<Geometry, GeoError> {
        match self.shape {
            PendingShape::Body(body) => Geometry::new(dim, *body),
            PendingShape::Collection(members) => {
                let members = members
                    .into_iter()
                    .map(|member| member.into_geometry(dim))
                    .collect::<Result<Vec<_>, GeoError>>()?;
                Geometry::new(dim, GeometryBody::GeometryCollection(members))
            }
        }
    }
}

/// The seven type names, for the message an unknown one produces.
const GEOMETRY_TYPES: &str = "`Point`, `MultiPoint`, `LineString`, `MultiLineString`, `Polygon`, `MultiPolygon` or \
     `GeometryCollection`";

fn read_object(value: &JsonValue) -> Result<Pending, GeoError> {
    if !matches!(value, JsonValue::Object(_)) {
        return Err(GeoError::literal(format!(
            "a geo:geoJSONLiteral is an RFC 7946 Geometry object, but this is {}",
            value.kind_name()
        )));
    }
    let type_name = read_type(value)?;
    match type_name {
        "Feature" | "FeatureCollection" => Err(GeoError::literal(format!(
            "a geo:geoJSONLiteral is a GeoJSON Geometry object and `{type_name}` is not one of \
             them: GeoSPARQL 1.1 Requirement 25 admits only {GEOMETRY_TYPES}. Write the geometry \
             itself as the literal rather than the {type_name} that wraps it"
        ))),
        "Point" | "MultiPoint" | "LineString" | "MultiLineString" | "Polygon" | "MultiPolygon" => {
            reject_foreign_shape_member(value, "geometries", type_name, "coordinates")?;
            let coordinates = decisive_member(value, "coordinates", type_name)?;
            let mut dim = None;
            let body = read_coordinates(type_name, coordinates, &mut dim)?;
            Ok(Pending {
                dim,
                shape: PendingShape::Body(Box::new(body)),
            })
        }
        "GeometryCollection" => {
            reject_foreign_shape_member(value, "coordinates", type_name, "geometries")?;
            let geometries = decisive_member(value, "geometries", type_name)?;
            let items = expect_array(
                geometries,
                "the `geometries` of a GeoJSON GeometryCollection",
            )?;
            let mut members = Vec::with_capacity(items.len());
            for item in items {
                members.push(read_object(item)?);
            }
            let dim = unified_dim(&members)?;
            Ok(Pending {
                dim,
                shape: PendingShape::Collection(members),
            })
        }
        other => Err(GeoError::literal(format!(
            "`{other}` is not a GeoJSON geometry type; RFC 7946 defines {GEOMETRY_TYPES}"
        ))),
    }
}

/// The `type` member, which every Geometry object has exactly one of and which is
/// always a string.
fn read_type(object: &JsonValue) -> Result<&str, GeoError> {
    match object.count("type") {
        0 => Err(GeoError::literal(
            "a GeoJSON Geometry object has a `type` member naming its geometry type; this object \
             has none",
        )),
        1 => match object.get("type") {
            Some(JsonValue::String(name)) => Ok(name.as_str()),
            Some(other) => Err(GeoError::literal(format!(
                "the `type` member of a GeoJSON Geometry object is a string, but this one is {}",
                other.kind_name()
            ))),
            // Unreachable: `count` just said there is one.
            None => Err(GeoError::literal("a GeoJSON Geometry object has no `type`")),
        },
        repeats => Err(GeoError::literal(format!(
            "this GeoJSON object has {repeats} `type` members, so it names no single geometry \
             type; RFC 8259 permits the repetition but resolving it by position would be a \
             silent choice"
        ))),
    }
}

/// The single member that carries a geometry's shape (`coordinates`, or
/// `geometries` for a collection).
///
/// Absent, `null` and repeated are all refusals: a geometry with no coordinates
/// is not a geometry, RFC 7946 gives `null` no meaning here (an empty geometry is
/// written `[]`), and two `coordinates` arrays denote two different geometries.
fn decisive_member<'a>(
    object: &'a JsonValue,
    name: &str,
    type_name: &str,
) -> Result<&'a JsonValue, GeoError> {
    match object.count(name) {
        0 => Err(GeoError::literal(format!(
            "a GeoJSON {type_name} has a `{name}` member; this one has none, and RFC 7946 defines \
             the geometry entirely by it"
        ))),
        1 => match object.get(name) {
            Some(JsonValue::Null) => Err(GeoError::literal(format!(
                "the `{name}` member of this GeoJSON {type_name} is null; RFC 7946 gives null no \
                 meaning there, and an empty geometry is written `\"{name}\":[]`"
            ))),
            Some(member) => Ok(member),
            None => Err(GeoError::literal(format!(
                "a GeoJSON {type_name} has no `{name}` member"
            ))),
        },
        repeats => Err(GeoError::literal(format!(
            "this GeoJSON {type_name} has {repeats} `{name}` members, which denote different \
             geometries; the literal is ambiguous and is refused rather than resolved by position"
        ))),
    }
}

/// Refuse the shape member that belongs to the *other* family of types.
///
/// `geometries` on a `Point`, or `coordinates` on a `GeometryCollection`, is a
/// contradiction rather than a foreign member: both names are defined by RFC 7946
/// and each belongs to exactly one family, so an object carrying both states two
/// incompatible things about itself. Genuinely foreign members (`bbox`, `title`,
/// anything else) are ignored, which is the neighbouring case the tests prove.
fn reject_foreign_shape_member(
    object: &JsonValue,
    wrong: &str,
    type_name: &str,
    right: &str,
) -> Result<(), GeoError> {
    if object.count(wrong) == 0 {
        return Ok(());
    }
    Err(GeoError::literal(format!(
        "this GeoJSON {type_name} carries a `{wrong}` member, which RFC 7946 defines for the \
         other family of geometry types; a {type_name} states its shape in `{right}`, and an \
         object claiming both is a contradiction rather than a Geometry object with a foreign \
         member"
    )))
}

/// The one dimension every member of a collection shares, or `None` when no
/// member has any position at all.
fn unified_dim(members: &[Pending]) -> Result<Option<CoordDim>, GeoError> {
    let mut unified: Option<CoordDim> = None;
    for member in members {
        let Some(member_dim) = member.dim else {
            continue;
        };
        match unified {
            None => unified = Some(member_dim),
            Some(fixed) if fixed == member_dim => {}
            Some(fixed) => {
                return Err(GeoError::literal(format!(
                    "this GeoJSON GeometryCollection mixes {}-element and {}-element positions; \
                     the geometry model carries one coordinate dimension for a whole geometry, so \
                     a collection whose members disagree about it has no dimension to report to \
                     `geof:coordinateDimension`",
                    fixed.ordinates(),
                    member_dim.ordinates()
                )));
            }
        }
    }
    Ok(unified)
}

fn read_coordinates(
    type_name: &str,
    coordinates: &JsonValue,
    dim: &mut Option<CoordDim>,
) -> Result<GeometryBody, GeoError> {
    match type_name {
        "Point" => {
            let items = expect_array(coordinates, "the `coordinates` of a GeoJSON Point")?;
            if items.is_empty() {
                // RFC 7946 has no empty position, but an empty `coordinates`
                // array is how an empty geometry of each type is written.
                Ok(GeometryBody::Point(None))
            } else {
                Ok(GeometryBody::Point(Some(read_position(coordinates, dim)?)))
            }
        }
        "MultiPoint" => {
            let items = expect_array(coordinates, "the `coordinates` of a GeoJSON MultiPoint")?;
            let mut points = Vec::with_capacity(items.len());
            for item in items {
                // Every member of a GeoJSON MultiPoint is a real position:
                // there is no way to write an empty member, so none is `None`.
                points.push(Some(read_position(item, dim)?));
            }
            Ok(GeometryBody::MultiPoint(points))
        }
        "LineString" => Ok(GeometryBody::LineString(read_positions(
            coordinates,
            "the `coordinates` of a GeoJSON LineString",
            dim,
        )?)),
        "MultiLineString" => {
            let items = expect_array(
                coordinates,
                "the `coordinates` of a GeoJSON MultiLineString",
            )?;
            let mut lines = Vec::with_capacity(items.len());
            for item in items {
                lines.push(read_positions(
                    item,
                    "a member LineString of a GeoJSON MultiLineString",
                    dim,
                )?);
            }
            Ok(GeometryBody::MultiLineString(lines))
        }
        "Polygon" => Ok(GeometryBody::Polygon(read_rings(
            coordinates,
            "the `coordinates` of a GeoJSON Polygon",
            dim,
        )?)),
        "MultiPolygon" => {
            let items = expect_array(coordinates, "the `coordinates` of a GeoJSON MultiPolygon")?;
            let mut polygons = Vec::with_capacity(items.len());
            for item in items {
                polygons.push(read_rings(
                    item,
                    "a member Polygon of a GeoJSON MultiPolygon",
                    dim,
                )?);
            }
            Ok(GeometryBody::MultiPolygon(polygons))
        }
        // `read_object` has already narrowed the type name to this set.
        other => Err(GeoError::literal(format!(
            "`{other}` is not a GeoJSON geometry type with coordinates"
        ))),
    }
}

fn read_rings(
    value: &JsonValue,
    what: &str,
    dim: &mut Option<CoordDim>,
) -> Result<Rings, GeoError> {
    let items = expect_array(value, what)?;
    let mut rings = Rings::with_capacity(items.len());
    for item in items {
        // The "at least four positions" and "last repeats the first" checks are
        // `Geometry::new`'s; duplicating them here would be a second place for
        // them to drift.
        rings.push(read_positions(item, "a GeoJSON linear ring", dim)?);
    }
    Ok(rings)
}

fn read_positions(
    value: &JsonValue,
    what: &str,
    dim: &mut Option<CoordDim>,
) -> Result<CoordSeq, GeoError> {
    let items = expect_array(value, what)?;
    let mut coords = CoordSeq::with_capacity(items.len());
    for item in items {
        coords.push(read_position(item, dim)?);
    }
    Ok(coords)
}

/// One RFC 7946 position: `[longitude, latitude]` or `[longitude, latitude,
/// altitude]`, and nothing else.
fn read_position(value: &JsonValue, dim: &mut Option<CoordDim>) -> Result<Coord, GeoError> {
    let items = expect_array(value, "a GeoJSON position")?;
    let here = match items.len() {
        2 => CoordDim::Xy,
        3 => CoordDim::Xyz,
        few @ (0 | 1) => {
            return Err(GeoError::literal(format!(
                "a GeoJSON position is an array of two or three numbers (longitude, latitude and \
                 an optional altitude); this one has {few}"
            )));
        }
        many => {
            return Err(GeoError::literal(format!(
                "a GeoJSON position has at most three numbers; RFC 7946 §3.1.1 says \
                 \"Implementations SHOULD NOT extend positions beyond three elements\", so a \
                 position of {many} elements is refused rather than silently truncated. The RFC \
                 gives the extra elements no meaning and GeoJSON has no measure ordinate for them \
                 to become, so ignoring them would discard a number without saying so; an \
                 extension that used a fourth element would need its own datatype"
            )));
        }
    };
    match *dim {
        None => *dim = Some(here),
        Some(fixed) if fixed == here => {}
        Some(fixed) => {
            return Err(GeoError::literal(format!(
                "this GeoJSON geometry mixes {}-element and {}-element positions; the geometry \
                 model carries one coordinate dimension for a whole geometry, so a geometry whose \
                 positions disagree about it has no dimension to report to \
                 `geof:coordinateDimension`. RFC 7946 does not itself forbid the mixture — this \
                 is purrdf-geo's model refusing rather than silently dropping or inventing an \
                 altitude",
                fixed.ordinates(),
                here.ordinates()
            )));
        }
    }
    let x = read_ordinate(&items[0])?;
    let y = read_ordinate(&items[1])?;
    let z = if here.has_z() {
        Some(read_ordinate(&items[2])?)
    } else {
        None
    };
    Ok(Coord::new(x, y, z, None))
}

/// One ordinate, decided exactly from the JSON number's own text.
fn read_ordinate(value: &JsonValue) -> Result<Rat, GeoError> {
    let JsonValue::Number(lexeme) = value else {
        return Err(GeoError::literal(format!(
            "an ordinate of a GeoJSON position is a number, but this one is {}",
            value.kind_name()
        )));
    };
    // `Rat::parse_decimal` reads the digits, never an `f64`; it refuses only an
    // exponent so large that the power of ten could not be built.
    Rat::parse_decimal(lexeme).ok_or_else(|| {
        GeoError::literal(format!(
            "the ordinate `{lexeme}` is a valid JSON number but its exponent is past the exact \
             decimal reader's cap, so it names no value this crate can hold"
        ))
    })
}

fn expect_array<'a>(value: &'a JsonValue, what: &str) -> Result<&'a [JsonValue], GeoError> {
    match value {
        JsonValue::Array(items) => Ok(items.as_slice()),
        other => Err(GeoError::literal(format!(
            "{what} is a JSON array in RFC 7946, but this is {}",
            other.kind_name()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Render a geometry as a `geo:geoJSONLiteral` lexical form.
///
/// `coordinate_scale` is the greatest number of fraction digits an ordinate is
/// written with; see [`write_bare`].
///
/// # Errors
///
/// [`GeoError::Domain`] when the literal's coordinate reference system differs
/// from `required_crs`, because RFC 7946 admits exactly one system (GeoSPARQL 1.1
/// Requirement 26) and this crate reprojects nothing — relabelling the ordinates
/// would state something about the world that is not true. Also everything
/// [`write_bare`] refuses.
pub fn write(
    literal: &GeometryLiteral,
    required_crs: &Crs,
    coordinate_scale: u32,
) -> Result<String, GeoError> {
    if literal.crs() != required_crs {
        return Err(GeoError::domain(format!(
            "this geometry is expressed in <{}> but a geo:geoJSONLiteral is fixed to <{}>; RFC \
             7946 admits exactly one coordinate reference system and purrdf-geo reprojects \
             nothing, so it refuses rather than relabelling the ordinates",
            literal.crs(),
            required_crs
        )));
    }
    write_bare(literal.geometry(), coordinate_scale)
}

/// Render a geometry as a `geo:geoJSONLiteral` lexical form without checking a
/// coordinate reference system.
///
/// The output is compact — no insignificant whitespace — and byte-deterministic:
/// the members are always `"type"` then `"coordinates"` (or `"geometries"` for a
/// collection), in that order, and every position is written `[x,y]` or
/// `[x,y,z]`. `coordinate_scale` caps the fraction digits of an ordinate, which
/// is rounded half to even and stripped of trailing zeros, so `1.50` is written
/// `1.5`; that rounding is the only lossy step in this module and the caller
/// chooses its size.
///
/// # Errors
///
/// [`GeoError::Domain`] when the geometry cannot be written as GeoJSON at all:
///
/// * its [`CoordDim`] has a measure ordinate. RFC 7946 §3.1.1 defines a position
///   as longitude, latitude and an optional altitude — there is no fourth slot —
///   so writing an `XYM` or `XYZM` geometry would mean **dropping** the measure.
///   Silently discarding an ordinate is precisely the failure this crate exists
///   to prevent, so it is a refusal.
/// * it is a `MultiPoint` with an empty member. `MULTIPOINT(EMPTY)` is a
///   well-formed geometry that GeoJSON has no syntax for: `[]` in the member
///   position is not a position, and writing it as `MULTIPOINT EMPTY` would lose
///   a member. `MULTIPOINT EMPTY` itself — no members at all — writes fine.
pub fn write_bare(geometry: &Geometry, coordinate_scale: u32) -> Result<String, GeoError> {
    if geometry.dim().has_m() {
        return Err(GeoError::domain(format!(
            "this geometry is {} and GeoJSON has no measure ordinate: RFC 7946 §3.1.1 defines a \
             position as longitude, latitude and an optional altitude, so writing it would drop \
             the measure. purrdf-geo refuses rather than silently discarding an ordinate; write \
             it as a geo:wktLiteral instead",
            dim_name(geometry.dim())
        )));
    }
    Ok(json::write(&geometry_value(geometry, coordinate_scale)?))
}

fn dim_name(dim: CoordDim) -> &'static str {
    match dim {
        CoordDim::Xy => "XY",
        CoordDim::Xyz => "XYZ",
        CoordDim::Xym => "XYM",
        CoordDim::Xyzm => "XYZM",
    }
}

fn geometry_value(geometry: &Geometry, scale: u32) -> Result<JsonValue, GeoError> {
    let type_member = (
        "type".to_owned(),
        JsonValue::String(geometry.kind().geojson_type().to_owned()),
    );
    let coordinates = match geometry.body() {
        GeometryBody::Point(point) => point.as_ref().map_or_else(
            || JsonValue::Array(Vec::new()),
            |c| position_value(c, scale),
        ),
        GeometryBody::MultiPoint(points) => {
            let mut items = Vec::with_capacity(points.len());
            for point in points {
                let Some(coord) = point.as_ref() else {
                    return Err(GeoError::domain(
                        "this MULTIPOINT holds an empty member point and GeoJSON has no syntax \
                         for one: RFC 7946 has no empty position, and writing the member as `[]` \
                         or omitting it would change how many members the geometry has. \
                         purrdf-geo refuses rather than dropping a member; a MULTIPOINT with no \
                         members at all writes as `[]` and is unaffected",
                    ));
                };
                items.push(position_value(coord, scale));
            }
            JsonValue::Array(items)
        }
        GeometryBody::LineString(coords) => sequence_value(coords, scale),
        GeometryBody::MultiLineString(lines) => JsonValue::Array(
            lines
                .iter()
                .map(|line| sequence_value(line, scale))
                .collect(),
        ),
        GeometryBody::Polygon(rings) => rings_value(rings, scale),
        GeometryBody::MultiPolygon(polygons) => JsonValue::Array(
            polygons
                .iter()
                .map(|polygon| rings_value(polygon, scale))
                .collect(),
        ),
        GeometryBody::GeometryCollection(members) => {
            let mut items = Vec::with_capacity(members.len());
            for member in members {
                items.push(geometry_value(member, scale)?);
            }
            return Ok(JsonValue::Object(vec![
                type_member,
                ("geometries".to_owned(), JsonValue::Array(items)),
            ]));
        }
    };
    Ok(JsonValue::Object(vec![
        type_member,
        ("coordinates".to_owned(), coordinates),
    ]))
}

fn rings_value(rings: &Rings, scale: u32) -> JsonValue {
    JsonValue::Array(
        rings
            .iter()
            .map(|ring| sequence_value(ring, scale))
            .collect(),
    )
}

fn sequence_value(coords: &CoordSeq, scale: u32) -> JsonValue {
    JsonValue::Array(
        coords
            .iter()
            .map(|coord| position_value(coord, scale))
            .collect(),
    )
}

fn position_value(coord: &Coord, scale: u32) -> JsonValue {
    let mut ordinates = Vec::with_capacity(3);
    ordinates.push(JsonValue::Number(coord.x().to_decimal_string(scale)));
    ordinates.push(JsonValue::Number(coord.y().to_decimal_string(scale)));
    if let Some(z) = coord.z() {
        ordinates.push(JsonValue::Number(z.to_decimal_string(scale)));
    }
    // A measure is unreachable here: `write_bare` refuses an M dimension before
    // any position is written, and `Geometry::new` guarantees every coordinate
    // carries the geometry's dimension.
    JsonValue::Array(ordinates)
}

#[cfg(test)]
mod tests {
    use super::{parse, write, write_bare};
    use crate::error::GeoError;
    use crate::exact::Rat;
    use crate::geom::{
        Coord, CoordDim, Crs, Geometry, GeometryBody, GeometryKind, GeometryLiteral,
    };

    /// The scale the round-trip and golden tests write with. Twelve fraction
    /// digits is far more than any fixture here needs, so the goldens turn on the
    /// serializer's shape rather than on its rounding.
    const SCALE: u32 = 12;

    fn crs() -> Crs {
        Crs::new("http://example.org/crs/OGC/1.3/CRS84").expect("a non-empty IRI")
    }

    fn read(text: &str) -> Result<Geometry, GeoError> {
        parse(text, &crs()).map(GeometryLiteral::into_geometry)
    }

    fn parsed(text: &str) -> Geometry {
        match read(text) {
            Ok(value) => value,
            Err(error) => panic!("{text} must parse, but: {error}"),
        }
    }

    fn refusal(text: &str) -> String {
        match read(text) {
            Err(error) => error.detail().to_owned(),
            Ok(value) => panic!("{text} must be refused, but parsed as {value:?}"),
        }
    }

    fn rendered(text: &str) -> String {
        write_bare(&parsed(text), SCALE).expect("a GeoJSON geometry has no measure ordinate")
    }

    fn rat(text: &str) -> Rat {
        Rat::parse_decimal(text).expect("an exact decimal")
    }

    // ---- Requirement 27: the empty literal --------------------------------

    /// GeoSPARQL 1.1 Requirement 27: "An empty RDFS Literal of type
    /// `geo:geoJSONLiteral` shall be interpreted as an empty Geometry." The
    /// shipped SHACL shape's pattern `^\s*$|^\s*({)(.*)(})\s*$` makes a
    /// whitespace-only form empty too.
    #[test]
    fn an_empty_or_whitespace_only_literal_is_the_empty_geometry() {
        for text in ["", " ", "\t", "\n", "  \r\n\t "] {
            let empty = parsed(text);
            assert!(empty.is_empty(), "{text:?} must denote the empty geometry");
            assert_eq!(
                empty.kind(),
                GeometryKind::GeometryCollection,
                "the empty literal names no kind, so the empty set is a collection with no \
                 members rather than a POINT EMPTY that would make geof:geometryType report a \
                 type the literal never wrote"
            );
            assert_eq!(empty.dim(), CoordDim::Xy, "and the planar dimension");
            assert_eq!(empty.coord_count(), 0, "with no positions");
        }
        // The neighbouring NON-empty case, so the assertions above turn on
        // emptiness rather than on everything reporting empty.
        assert!(
            !parsed(r#"{"type":"Point","coordinates":[1,2]}"#).is_empty(),
            "a real geometry is not empty"
        );
    }

    #[test]
    fn the_empty_literal_and_an_empty_collection_denote_the_same_geometry() {
        assert_eq!(
            parsed(""),
            parsed(r#"{"type":"GeometryCollection","geometries":[]}"#),
            "Requirement 27's empty geometry is exactly the empty collection"
        );
    }

    // ---- Requirement 25: Geometry objects only ----------------------------

    /// Requirement 25 admits the seven Geometry objects and nothing else, so a
    /// `Feature` is refused by name — and the very same coordinates written as a
    /// bare `Point` still parse.
    #[test]
    fn a_feature_is_refused_by_name_but_the_bare_point_inside_it_parses() {
        let feature = r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[1,2]},"properties":null}"#;
        let message = refusal(feature);
        assert!(
            message.contains("Feature"),
            "the refusal names the type it refused: {message}"
        );
        assert!(
            message.contains("Requirement 25"),
            "and cites the requirement: {message}"
        );
        let collection = r#"{"type":"FeatureCollection","features":[]}"#;
        assert!(
            refusal(collection).contains("FeatureCollection"),
            "a FeatureCollection is refused by name too"
        );

        // The neighbouring VALID case: the same coordinates, as a Geometry object.
        let point = parsed(r#"{"type":"Point","coordinates":[1,2]}"#);
        assert_eq!(point.kind(), GeometryKind::Point, "the bare Point parses");
        assert_eq!(point.coord_count(), 1, "and carries the same position");
    }

    #[test]
    fn all_seven_geometry_types_parse() {
        for (text, kind) in [
            (
                r#"{"type":"Point","coordinates":[1,2]}"#,
                GeometryKind::Point,
            ),
            (
                r#"{"type":"MultiPoint","coordinates":[[1,2]]}"#,
                GeometryKind::MultiPoint,
            ),
            (
                r#"{"type":"LineString","coordinates":[[0,0],[1,1]]}"#,
                GeometryKind::LineString,
            ),
            (
                r#"{"type":"MultiLineString","coordinates":[[[0,0],[1,1]]]}"#,
                GeometryKind::MultiLineString,
            ),
            (
                r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}"#,
                GeometryKind::Polygon,
            ),
            (
                r#"{"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]]]}"#,
                GeometryKind::MultiPolygon,
            ),
            (
                r#"{"type":"GeometryCollection","geometries":[]}"#,
                GeometryKind::GeometryCollection,
            ),
        ] {
            assert_eq!(parsed(text).kind(), kind, "{text} is a {kind:?}");
        }
    }

    // ---- exactness --------------------------------------------------------

    /// The proof that no coordinate touched a float: `0.1` and `0.2` have no
    /// exact `f64`, so a reader that went through one could not produce the exact
    /// rationals this asserts.
    #[test]
    fn coordinates_are_exactly_the_decimals_that_were_written() {
        let geometry = parsed(r#"{"type":"Point","coordinates":[0.1,0.2]}"#);
        let GeometryBody::Point(Some(coord)) = geometry.body() else {
            panic!("a Point with a position");
        };
        assert_eq!(*coord.x(), rat("0.1"), "x is exactly one tenth");
        assert_eq!(*coord.y(), rat("0.2"), "y is exactly one fifth");
        assert_eq!(
            *coord.x(),
            Rat::new(
                crate::exact::Int::from_i64(1),
                crate::exact::Int::from_i64(10)
            )
            .expect("a non-zero denominator"),
            "and is the rational 1/10, not the nearest double to it"
        );
    }

    /// Three spellings of the same number must produce the identical geometry.
    /// A float path would agree here by accident; an exact path agrees by
    /// construction, and the third spelling has an exponent no textual
    /// comparison would equate.
    #[test]
    fn different_spellings_of_one_number_produce_the_identical_geometry() {
        let a = parsed(r#"{"type":"Point","coordinates":[1.5,0]}"#);
        let b = parsed(r#"{"type":"Point","coordinates":[1.50,0]}"#);
        let c = parsed(r#"{"type":"Point","coordinates":[15e-1,0]}"#);
        let d = parsed(r#"{"type":"Point","coordinates":[0.15E1,-0]}"#);
        assert_eq!(a, b, "1.5 and 1.50 are one value");
        assert_eq!(a, c, "1.5 and 15e-1 are one value");
        assert_eq!(a, d, "1.5 and 0.15E1 are one value, and -0 is 0");
        // The neighbouring case that must NOT be equal, so the assertions above
        // are not merely reporting that everything compares equal.
        assert_ne!(
            a,
            parsed(r#"{"type":"Point","coordinates":[1.51,0]}"#),
            "a different number is a different geometry"
        );
    }

    /// A forty-significant-digit ordinate is exact and must NOT be refused: no
    /// float could hold it, which is the whole point.
    #[test]
    fn a_forty_significant_digit_coordinate_parses_exactly() {
        let digits = "1.234567890123456789012345678901234567890";
        let text = format!(r#"{{"type":"Point","coordinates":[{digits},2]}}"#);
        let geometry = parsed(&text);
        let GeometryBody::Point(Some(coord)) = geometry.body() else {
            panic!("a Point with a position");
        };
        assert_eq!(
            *coord.x(),
            rat(digits),
            "every one of the forty digits survives"
        );
        assert_ne!(
            *coord.x(),
            rat("1.2345678901234568"),
            "and it is not the seventeen-digit value an f64 would have kept"
        );
        // A huge integer ordinate is equally acceptable.
        assert!(
            read(r#"{"type":"Point","coordinates":[123456789012345678901234567890,1]}"#).is_ok(),
            "an ordinate beyond i64 is a number, not an error"
        );
    }

    #[test]
    fn a_wildly_out_of_range_exponent_is_refused_but_a_large_one_is_not() {
        assert!(
            read(r#"{"type":"Point","coordinates":[1e999999999,1]}"#).is_err(),
            "an exponent past the exact reader's cap names no holdable value"
        );
        // The neighbouring VALID case: a large but representable exponent.
        assert!(
            read(r#"{"type":"Point","coordinates":[1e308,1]}"#).is_ok(),
            "1e308 is an ordinary number to an exact reader"
        );
        assert!(
            read(r#"{"type":"Point","coordinates":[1e-400,1]}"#).is_ok(),
            "and so is one an f64 would flush to zero"
        );
    }

    // ---- foreign members --------------------------------------------------

    /// RFC 7946 §6.1 allows a Geometry object to carry members the specification
    /// does not define, and `bbox` is defined but carries no geometry. Refusing
    /// them would reject conforming GeoJSON, which is the over-refusal that
    /// mirrors a silent drop.
    #[test]
    fn foreign_and_bbox_members_are_ignored_not_refused() {
        let plain = parsed(r#"{"type":"Point","coordinates":[1,2]}"#);
        for text in [
            r#"{"type":"Point","coordinates":[1,2],"bbox":[1,2,1,2],"title":"x"}"#,
            r#"{"bbox":[1,2,1,2],"type":"Point","coordinates":[1,2]}"#,
            r#"{"type":"Point","coordinates":[1,2],"crs":{"type":"name"},"id":7,"x":null}"#,
            r#"{"type":"Point","coordinates":[1,2],"bbox":[1,2,1,2],"bbox":[3,4,3,4]}"#,
        ] {
            assert_eq!(
                parsed(text),
                plain,
                "{text} must parse to the same geometry as the plain Point"
            );
        }
    }

    #[test]
    fn foreign_members_are_ignored_on_a_collection_too() {
        assert_eq!(
            parsed(
                r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2],"note":"a"}],"bbox":[1,2,1,2]}"#
            ),
            parsed(
                r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2]}]}"#
            ),
            "a foreign member changes nothing at any level"
        );
    }

    // ---- every refusal, with its valid neighbour --------------------------

    /// Each row is a refusal this codec makes and the neighbouring input, a
    /// character or two away, that must still parse. The second half of each row
    /// is the point: a validator that also rejected the neighbour would be as
    /// broken as one that accepted the first.
    #[test]
    fn every_refusal_has_a_neighbouring_valid_form_that_still_parses() {
        for (bad, good, why) in [
            (
                r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[1,2]}}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "a Feature is not a Geometry object (Requirement 25)",
            ),
            (
                r#"{"type":"FeatureCollection","features":[]}"#,
                r#"{"type":"GeometryCollection","geometries":[]}"#,
                "nor is a FeatureCollection",
            ),
            (
                r#"{"type":"Point","coordinates":[1,2,3,4]}"#,
                r#"{"type":"Point","coordinates":[1,2,3]}"#,
                "a position has at most three elements",
            ),
            (
                r#"{"type":"Point","coordinates":[1]}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "and at least two",
            ),
            (
                r#"{"type":"LineString","coordinates":[[0,0],[1,1,1]]}"#,
                r#"{"type":"LineString","coordinates":[[0,0,0],[1,1,1]]}"#,
                "positions in one geometry may not mix lengths",
            ),
            (
                r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2]},{"type":"Point","coordinates":[1,2,3]}]}"#,
                r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2,3]},{"type":"Point","coordinates":[1,2,3]}]}"#,
                "nor may a collection's members",
            ),
            (
                r#"{"type":"Point"}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "`coordinates` is required",
            ),
            (
                r#"{"type":"Point","coordinates":null}"#,
                r#"{"type":"Point","coordinates":[]}"#,
                "null is not a geometry; an empty one is written `[]`",
            ),
            (
                r#"{"type":"Point","coordinates":[1,2],"coordinates":[3,4]}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "a repeated `coordinates` is ambiguous",
            ),
            (
                r#"{"type":"Circle","coordinates":[1,2]}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "an unknown type names no RFC 7946 geometry",
            ),
            (
                r#"{"type":"point","coordinates":[1,2]}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "and the type names are case-sensitive",
            ),
            (
                r#"{"coordinates":[1,2]}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "`type` is required",
            ),
            (
                r#"{"type":123,"coordinates":[1,2]}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "`type` is a string",
            ),
            (
                r#"{"type":"Point","geometries":[]}"#,
                r#"{"type":"GeometryCollection","geometries":[]}"#,
                "`geometries` belongs to a GeometryCollection",
            ),
            (
                r#"{"type":"GeometryCollection","coordinates":[1,2],"geometries":[]}"#,
                r#"{"type":"GeometryCollection","geometries":[]}"#,
                "and `coordinates` does not",
            ),
            (
                r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[0,0]]]}"#,
                r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}"#,
                "a ring needs at least four positions",
            ),
            (
                r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[2,2]]]}"#,
                r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}"#,
                "and must close",
            ),
            (
                r#"{"type":"LineString","coordinates":[[0,0]]}"#,
                r#"{"type":"LineString","coordinates":[[0,0],[1,1]]}"#,
                "a line has no positions or at least two",
            ),
            (
                r#"{"type":"Point","coordinates":[1,2]} trailing"#,
                r#"{"type":"Point","coordinates":[1,2]}   "#,
                "content after the value is not JSON; trailing whitespace is",
            ),
            (
                r#"{"type":"Point","coordinates":[1,2],}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "a trailing comma is not JSON",
            ),
            (
                r#"{"type":"Point","coordinates":["1",2]}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "an ordinate is a number, not a string holding one",
            ),
            (
                r#"{"type":"Point","coordinates":[null,2]}"#,
                r#"{"type":"Point","coordinates":[0,2]}"#,
                "nor is it null",
            ),
            (
                r#"{"type":"MultiPoint","coordinates":[[]]}"#,
                r#"{"type":"MultiPoint","coordinates":[]}"#,
                "a member of a MultiPoint is a position; `[]` is the empty MultiPoint",
            ),
            (
                r#"{"type":"Point","coordinates":{"x":1,"y":2}}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "`coordinates` is an array",
            ),
            (
                r#"[{"type":"Point","coordinates":[1,2]}]"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
                "a literal is one Geometry object, not an array of them",
            ),
        ] {
            assert!(read(bad).is_err(), "{why}: {bad} must be refused");
            assert!(
                read(good).is_ok(),
                "{why}: the neighbouring valid form {good} must still parse"
            );
        }
    }

    #[test]
    fn deeper_nesting_than_the_json_reader_allows_is_refused_but_ordinary_nesting_is_not() {
        fn nested(levels: usize) -> String {
            let mut text = String::new();
            for _ in 0..levels {
                text.push_str(r#"{"type":"GeometryCollection","geometries":["#);
            }
            text.push_str(r#"{"type":"Point","coordinates":[1,2]}"#);
            for _ in 0..levels {
                text.push_str("]}");
            }
            text
        }
        // Each level of collection is two JSON containers (the object and its
        // `geometries` array), so the reader's 128-container cap bites at 64.
        assert!(
            read(&nested(1)).is_ok(),
            "RFC 7946 discourages but does not forbid nesting, so one level parses"
        );
        assert!(read(&nested(8)).is_ok(), "and so do eight");
        assert!(
            read(&nested(62)).is_ok(),
            "62 levels is 124 containers, below the cap: it must NOT be refused"
        );
        assert!(
            read(&nested(500)).is_err(),
            "a hostile depth is refused rather than recursed into"
        );
    }

    #[test]
    fn refusal_messages_say_what_is_wrong() {
        assert!(
            refusal(r#"{"type":"Point","coordinates":[1,2,3,4]}"#).contains("at most three"),
            "the four-element position names the limit"
        );
        assert!(
            refusal(r#"{"type":"LineString","coordinates":[[0,0],[1,1,1]]}"#).contains("mixes"),
            "the mixed-length refusal says so"
        );
        assert!(
            refusal(r#"{"type":"Point","coordinates":null}"#).contains("null"),
            "the null refusal names null"
        );
        assert!(
            refusal(r#"{"type":"Point","geometries":[]}"#).contains("geometries"),
            "the misplaced member is named"
        );
        assert!(
            refusal(r#"{"type":"Circle","coordinates":[1,2]}"#).contains("Circle"),
            "the unknown type is quoted back"
        );
        assert!(
            refusal(r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[0,0]]]}"#)
                .contains("four positions"),
            "the short ring names the minimum"
        );
    }

    #[test]
    fn every_refusal_is_a_literal_error_never_a_panic_or_a_default() {
        for text in [
            "not json",
            "{",
            r#"{"type":"Point","coordinates":[1,2]"#,
            r#"{"type":"Feature"}"#,
            "42",
            "null",
            "true",
            r#""a string""#,
        ] {
            assert!(
                matches!(read(text), Err(GeoError::Literal(_))),
                "{text} must be a Literal refusal"
            );
        }
    }

    // ---- empty geometries of every kind -----------------------------------

    #[test]
    fn an_empty_coordinates_array_is_the_empty_geometry_of_that_kind() {
        for (text, kind) in [
            (r#"{"type":"Point","coordinates":[]}"#, GeometryKind::Point),
            (
                r#"{"type":"MultiPoint","coordinates":[]}"#,
                GeometryKind::MultiPoint,
            ),
            (
                r#"{"type":"LineString","coordinates":[]}"#,
                GeometryKind::LineString,
            ),
            (
                r#"{"type":"MultiLineString","coordinates":[]}"#,
                GeometryKind::MultiLineString,
            ),
            (
                r#"{"type":"Polygon","coordinates":[]}"#,
                GeometryKind::Polygon,
            ),
            (
                r#"{"type":"MultiPolygon","coordinates":[]}"#,
                GeometryKind::MultiPolygon,
            ),
            (
                r#"{"type":"GeometryCollection","geometries":[]}"#,
                GeometryKind::GeometryCollection,
            ),
        ] {
            let geometry = parsed(text);
            assert_eq!(geometry.kind(), kind, "{text} keeps its kind");
            assert!(geometry.is_empty(), "{text} is empty");
            assert_eq!(
                geometry,
                Geometry::empty(CoordDim::Xy, kind),
                "{text} is exactly the empty geometry of its kind"
            );
        }
    }

    // ---- dimension --------------------------------------------------------

    #[test]
    fn a_two_element_position_is_xy_and_a_three_element_one_is_xyz() {
        assert_eq!(
            parsed(r#"{"type":"Point","coordinates":[1,2]}"#).dim(),
            CoordDim::Xy,
            "two elements is planar"
        );
        assert_eq!(
            parsed(r#"{"type":"Point","coordinates":[1,2,3]}"#).dim(),
            CoordDim::Xyz,
            "three elements adds altitude"
        );
        let geometry = parsed(r#"{"type":"Point","coordinates":[1,2,3]}"#);
        let GeometryBody::Point(Some(coord)) = geometry.body() else {
            panic!("a Point with a position");
        };
        assert_eq!(coord.z(), Some(&rat("3")), "the altitude is the third slot");
        assert_eq!(
            coord.m(),
            None,
            "GeoJSON has no measure ordinate, so one is never invented"
        );
    }

    /// An empty member has no positions and therefore fixes no dimension; it must
    /// adopt its siblings' rather than forcing the collection to XY. Refusing
    /// this would be an over-refusal of conforming GeoJSON.
    #[test]
    fn an_empty_member_adopts_the_collections_dimension() {
        let geometry = parsed(
            r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[]},{"type":"Point","coordinates":[1,2,3]}]}"#,
        );
        assert_eq!(
            geometry.dim(),
            CoordDim::Xyz,
            "the 3D sibling fixes the dimension and the empty member adopts it"
        );
        let all_empty = parsed(
            r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[]},{"type":"LineString","coordinates":[]}]}"#,
        );
        assert_eq!(
            all_empty.dim(),
            CoordDim::Xy,
            "with nothing to fix it, the planar dimension is the default"
        );
    }

    #[test]
    fn a_nested_collection_shares_the_outer_dimension() {
        let geometry = parsed(
            r#"{"type":"GeometryCollection","geometries":[{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2,3]}]},{"type":"Point","coordinates":[]}]}"#,
        );
        assert_eq!(
            geometry.dim(),
            CoordDim::Xyz,
            "a dimension fixed two levels down governs the whole tree"
        );
        assert_eq!(geometry.coord_count(), 1, "and the position survives");
    }

    // ---- writing ----------------------------------------------------------

    #[test]
    fn serialization_is_byte_exact_for_every_geometry_type() {
        for (input, golden) in [
            (
                r#"{"type":"Point","coordinates":[1,2]}"#,
                r#"{"type":"Point","coordinates":[1,2]}"#,
            ),
            (
                r#"{"type": "Point", "coordinates": [ 1 , 2 , 3 ] }"#,
                r#"{"type":"Point","coordinates":[1,2,3]}"#,
            ),
            (
                r#"{"type":"Point","coordinates":[]}"#,
                r#"{"type":"Point","coordinates":[]}"#,
            ),
            (
                r#"{"coordinates":[-83.40,42.280],"type":"Point","bbox":[0,0,0,0]}"#,
                r#"{"type":"Point","coordinates":[-83.4,42.28]}"#,
            ),
            (
                r#"{"type":"Point","coordinates":[15e-1,-0]}"#,
                r#"{"type":"Point","coordinates":[1.5,0]}"#,
            ),
            (
                r#"{"type":"MultiPoint","coordinates":[[1,2],[3,4]]}"#,
                r#"{"type":"MultiPoint","coordinates":[[1,2],[3,4]]}"#,
            ),
            (
                r#"{"type":"MultiPoint","coordinates":[]}"#,
                r#"{"type":"MultiPoint","coordinates":[]}"#,
            ),
            (
                r#"{"type":"LineString","coordinates":[[0,0],[1,1]]}"#,
                r#"{"type":"LineString","coordinates":[[0,0],[1,1]]}"#,
            ),
            (
                r#"{"type":"MultiLineString","coordinates":[[[0,0],[1,1]],[[2,2],[3,3]]]}"#,
                r#"{"type":"MultiLineString","coordinates":[[[0,0],[1,1]],[[2,2],[3,3]]]}"#,
            ),
            (
                r#"{"type":"Polygon","coordinates":[[[0,0],[4,0],[4,4],[0,0]],[[1,1],[2,1],[2,2],[1,1]]]}"#,
                r#"{"type":"Polygon","coordinates":[[[0,0],[4,0],[4,4],[0,0]],[[1,1],[2,1],[2,2],[1,1]]]}"#,
            ),
            (
                r#"{"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]]]}"#,
                r#"{"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]]]}"#,
            ),
            (
                r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2]},{"type":"LineString","coordinates":[[0,0],[1,1]]}]}"#,
                r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2]},{"type":"LineString","coordinates":[[0,0],[1,1]]}]}"#,
            ),
            ("", r#"{"type":"GeometryCollection","geometries":[]}"#),
        ] {
            assert_eq!(
                rendered(input),
                golden,
                "{input} must serialize to exactly {golden}"
            );
        }
    }

    #[test]
    fn the_member_order_is_always_type_then_the_shape_member() {
        let text = rendered(r#"{"bbox":[0,0,0,0],"coordinates":[1,2],"type":"Point"}"#);
        assert!(
            text.starts_with(r#"{"type":"Point","coordinates":"#),
            "member order is the serializer's, not the input's: {text}"
        );
        let collection = rendered(r#"{"geometries":[],"type":"GeometryCollection"}"#);
        assert_eq!(
            collection, r#"{"type":"GeometryCollection","geometries":[]}"#,
            "a collection writes `geometries` in the same slot"
        );
    }

    #[test]
    fn the_coordinate_scale_caps_the_fraction_digits() {
        let geometry = parsed(r#"{"type":"Point","coordinates":[1.23456789,2]}"#);
        assert_eq!(
            write_bare(&geometry, 3).expect("no measure"),
            r#"{"type":"Point","coordinates":[1.235,2]}"#,
            "three fraction digits, rounded half to even"
        );
        assert_eq!(
            write_bare(&geometry, 0).expect("no measure"),
            r#"{"type":"Point","coordinates":[1,2]}"#,
            "a scale of zero writes integers"
        );
        assert_eq!(
            write_bare(&geometry, 20).expect("no measure"),
            r#"{"type":"Point","coordinates":[1.23456789,2]}"#,
            "trailing zeros are never padded on"
        );
    }

    // ---- the measure ordinate ---------------------------------------------

    fn coord(x: i64, y: i64, z: Option<i64>, m: Option<i64>) -> Coord {
        Coord::new(
            Rat::from_i64(x),
            Rat::from_i64(y),
            z.map(Rat::from_i64),
            m.map(Rat::from_i64),
        )
    }

    /// GeoJSON has no measure ordinate, so writing an XYM or XYZM geometry would
    /// mean dropping data. That is a refusal, not a silent truncation — and the
    /// neighbouring XYZ geometry writes perfectly well.
    #[test]
    fn a_measure_ordinate_is_refused_on_write_but_an_altitude_is_not() {
        for dim in [CoordDim::Xym, CoordDim::Xyzm] {
            let empty = Geometry::empty(dim, GeometryKind::Point);
            let error = write_bare(&empty, SCALE)
                .expect_err("GeoJSON cannot express a measure, even on an empty geometry");
            assert!(
                matches!(error, GeoError::Domain(_)),
                "the refusal is a domain error: {error:?}"
            );
            assert!(
                error.detail().contains("measure"),
                "and it names the ordinate it will not drop: {error}"
            );
        }
        let measured = Geometry::new(
            CoordDim::Xym,
            GeometryBody::Point(Some(coord(1, 2, None, Some(3)))),
        )
        .expect("a well-formed XYM point");
        assert!(
            write_bare(&measured, SCALE).is_err(),
            "a populated XYM geometry is refused too"
        );

        // The neighbouring VALID case: the same shape with an altitude.
        let elevated = Geometry::new(
            CoordDim::Xyz,
            GeometryBody::Point(Some(coord(1, 2, Some(3), None))),
        )
        .expect("a well-formed XYZ point");
        assert_eq!(
            write_bare(&elevated, SCALE).expect("XYZ has no measure"),
            r#"{"type":"Point","coordinates":[1,2,3]}"#,
            "XYZ writes fine"
        );
        assert!(
            write_bare(&Geometry::empty(CoordDim::Xyz, GeometryKind::Point), SCALE).is_ok(),
            "and so does an empty XYZ geometry"
        );
    }

    /// `MULTIPOINT(EMPTY)` is a well-formed geometry GeoJSON has no syntax for,
    /// so it is refused rather than written as a geometry with fewer members.
    /// `MULTIPOINT EMPTY` — no members at all — is the valid neighbour.
    #[test]
    fn an_empty_multipoint_member_is_refused_on_write_but_no_members_is_not() {
        let with_empty_member = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPoint(vec![Some(coord(1, 2, None, None)), None]),
        )
        .expect("a well-formed MULTIPOINT(1 2, EMPTY)");
        let error = write_bare(&with_empty_member, SCALE)
            .expect_err("GeoJSON has no empty position to write the member as");
        assert!(
            matches!(error, GeoError::Domain(_)),
            "the refusal is a domain error: {error:?}"
        );

        // The neighbouring VALID cases.
        assert_eq!(
            write_bare(
                &Geometry::empty(CoordDim::Xy, GeometryKind::MultiPoint),
                SCALE
            )
            .expect("no members is writable"),
            r#"{"type":"MultiPoint","coordinates":[]}"#,
            "MULTIPOINT EMPTY writes as an empty array"
        );
        let populated = Geometry::new(
            CoordDim::Xy,
            GeometryBody::MultiPoint(vec![Some(coord(1, 2, None, None))]),
        )
        .expect("a well-formed MULTIPOINT(1 2)");
        assert_eq!(
            write_bare(&populated, SCALE).expect("every member is a position"),
            r#"{"type":"MultiPoint","coordinates":[[1,2]]}"#,
            "and a populated one writes its members"
        );
    }

    // ---- the coordinate reference system ----------------------------------

    #[test]
    fn writing_refuses_a_foreign_crs_but_accepts_the_required_one() {
        let literal = parse(r#"{"type":"Point","coordinates":[1,2]}"#, &crs()).expect("a Point");
        let other = Crs::new("http://example.org/crs/EPSG/0/3857").expect("a non-empty IRI");
        let error = write(&literal, &other, SCALE)
            .expect_err("RFC 7946 admits exactly one coordinate reference system");
        assert!(
            matches!(error, GeoError::Domain(_)),
            "a foreign system is a domain error: {error:?}"
        );
        assert!(
            error.detail().contains("example.org/crs/EPSG/0/3857"),
            "and the message names both systems: {error}"
        );

        // The neighbouring VALID case: the system the literal is actually in.
        assert_eq!(
            write(&literal, &crs(), SCALE).expect("the systems agree"),
            r#"{"type":"Point","coordinates":[1,2]}"#,
            "the matching system writes"
        );
    }

    #[test]
    fn parse_attaches_the_caller_supplied_crs_verbatim() {
        let literal = parse(r#"{"type":"Point","coordinates":[1,2]}"#, &crs()).expect("a Point");
        assert_eq!(
            literal.crs().as_str(),
            "http://example.org/crs/OGC/1.3/CRS84",
            "the system is the caller's, because purrdf-geo mints no vocabulary IRIs"
        );
        let empty = parse("", &crs()).expect("Requirement 27's empty geometry");
        assert_eq!(
            empty.crs(),
            &crs(),
            "including on the empty literal, which has no JSON to read it from"
        );
    }

    // ---- round trip -------------------------------------------------------

    /// `parse(write(parse(x)))` is `parse(x)` for every geometry type, in two and
    /// three dimensions, empty and populated. A codec whose two halves disagreed
    /// would fail here even when each half looked right on its own.
    #[test]
    fn parse_write_parse_is_a_fixed_point() {
        for text in [
            "",
            r#"{"type":"Point","coordinates":[1,2]}"#,
            r#"{"type":"Point","coordinates":[1,2,3]}"#,
            r#"{"type":"Point","coordinates":[-83.4,42.28]}"#,
            r#"{"type":"Point","coordinates":[]}"#,
            r#"{"type":"MultiPoint","coordinates":[[1,2],[3,4],[5,6]]}"#,
            r#"{"type":"MultiPoint","coordinates":[[1,2,3],[4,5,6]]}"#,
            r#"{"type":"MultiPoint","coordinates":[]}"#,
            r#"{"type":"LineString","coordinates":[[0,0],[1,1]]}"#,
            r#"{"type":"LineString","coordinates":[[0,0,0],[1,1,1],[2,2,2]]}"#,
            r#"{"type":"LineString","coordinates":[]}"#,
            r#"{"type":"MultiLineString","coordinates":[[[0,0],[1,1]],[[2,2],[3,3]]]}"#,
            r#"{"type":"MultiLineString","coordinates":[[[0,0,0],[1,1,1]]]}"#,
            r#"{"type":"MultiLineString","coordinates":[]}"#,
            r#"{"type":"MultiLineString","coordinates":[[]]}"#,
            r#"{"type":"Polygon","coordinates":[[[0,0],[4,0],[4,4],[0,0]]]}"#,
            r#"{"type":"Polygon","coordinates":[[[0,0],[4,0],[4,4],[0,4],[0,0]],[[1,1],[2,1],[2,2],[1,1]]]}"#,
            r#"{"type":"Polygon","coordinates":[[[0,0,1],[4,0,1],[4,4,1],[0,0,1]]]}"#,
            r#"{"type":"Polygon","coordinates":[]}"#,
            r#"{"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]],[[[5,5],[6,5],[6,6],[5,5]]]]}"#,
            r#"{"type":"MultiPolygon","coordinates":[[[[0,0,9],[1,0,9],[1,1,9],[0,0,9]]]]}"#,
            r#"{"type":"MultiPolygon","coordinates":[]}"#,
            r#"{"type":"MultiPolygon","coordinates":[[]]}"#,
            r#"{"type":"GeometryCollection","geometries":[]}"#,
            r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2]}]}"#,
            r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2,3]},{"type":"LineString","coordinates":[[0,0,0],[1,1,1]]}]}"#,
            r#"{"type":"GeometryCollection","geometries":[{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[7,8]}]},{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}]}"#,
        ] {
            let once = parsed(text);
            let written = write_bare(&once, SCALE).expect("no fixture carries a measure");
            let twice = parsed(&written);
            assert_eq!(
                once, twice,
                "parse(write(parse(x))) must equal parse(x) for {text} (wrote {written})"
            );
            let again =
                write_bare(&twice, SCALE).expect("the second geometry writes like the first");
            assert_eq!(
                written, again,
                "and serialization is a fixed point too, for {text}"
            );
        }
    }
}
