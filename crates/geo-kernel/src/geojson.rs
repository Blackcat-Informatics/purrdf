// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! literal carries no coordinate reference system of its own. The low-level
//! [`parse`] and [`write()`] carrier APIs accept the reference as a parameter.
//! The immutable standard vocabulary supplies official CRS84, whose geographic
//! profile is WGS84 longitude followed by latitude. Standard query and host
//! paths retain that interpretation and require an actual registered operation
//! chain before writing a different source as GeoJSON.
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
//! Coordinates are read from the events of the workspace's JSON reader
//! ([`purrdf_lex::json::Reader`], under [`crate::json`]'s reading policy), which
//! hands every number over as its source lexeme, and decided by
//! [`crate::exact::Rat::parse_decimal`], which is integer arithmetic end to end. No coordinate passes through an `f64` on the
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

use crate::carrier::writer::CarrierWriter as Writer;
use crate::error::GeoError;
use crate::exact::Rat;
use crate::geom::{
    Coord, CoordDim, CoordSeq, Crs, Geometry, GeometryBody, GeometryKind, GeometryLiteral, Rings,
};
use crate::json;
#[cfg(test)]
use crate::json::JsonValue;
use core::mem;
#[cfg(test)]
use purrdf_lex::json::Number;
use purrdf_lex::json::{Event, Kind, Reader as JsonReader};
use std::{cell::RefCell, rc::Rc};

struct Reader<'a, 'observer> {
    inner: JsonReader<'a>,
    admission: Option<Rc<RefCell<crate::carrier::ParseAdmission<'observer>>>>,
}
impl<'a> std::ops::Deref for Reader<'a, '_> {
    type Target = JsonReader<'a>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl std::ops::DerefMut for Reader<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
use std::borrow::Cow;
use std::ops::Range;

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
/// "Whitespace" is RFC 8259 §2's insignificant whitespace,
///
/// > `ws = *( %x20 / %x09 / %x0A / %x0D )`
///
/// — the same four code points [`crate::json`]'s scanner skips between tokens,
/// and not the twenty-six of [`char::is_whitespace`]. The two must be the same
/// set or this function contradicts the parser it delegates to: U+00A0 is not
/// `ws`, so `"\u{A0}{\"type\":\"Point\",\"coordinates\":[1,2]}"` has always been
/// malformed JSON here, and a wider test would have made the same scalar mean
/// "empty geometry" when it stood alone. See [`crate::wkt::parse`] for the full
/// argument; this is the GeoJSON half of the identical decision.
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

pub(crate) fn parse_admitted(
    lexical: &str,
    crs: &Crs,
    admission: Option<&Rc<RefCell<crate::carrier::ParseAdmission<'_>>>>,
) -> Result<GeometryLiteral, GeoError> {
    Ok(GeometryLiteral::new(
        crs.clone(),
        geometry_of_admitted(lexical, admission)?,
    ))
}

/// A streaming step of the reader: the outer error is a JSON refusal, which
/// outranks every shape refusal and ends the read at once; the inner one is a
/// shape refusal the step recorded after reading its whole value.
type Step<T> = Result<Result<T, GeoError>, purrdf_lex::json::Error>;

/// The geometry a lexical form denotes, before a coordinate reference system is
/// attached to it.
///
/// The literal is read from the JSON reader's events in one pass: positions go
/// straight into the exact geometry model, and no JSON tree is built. It refuses
/// exactly what a reader that built the tree first and then judged it would
/// refuse: malformed JSON anywhere, trailing content included, outranks every
/// shape refusal, and among shape refusals the first one a depth-first walk of
/// the tree meets wins — an object's own members before its collection's
/// members, the members in written order. So each object is read to its close
/// before it is judged, and a shape refusal found inside it waits there.
fn geometry_of(lexical: &str) -> Result<Geometry, GeoError> {
    geometry_of_admitted(lexical, None)
}
fn geometry_of_admitted(
    lexical: &str,
    admission: Option<&Rc<RefCell<crate::carrier::ParseAdmission<'_>>>>,
) -> Result<Geometry, GeoError> {
    // RFC 8259 §2 `ws = *( %x20 / %x09 / %x0A / %x0D )`, byte-tested and so
    // exact over UTF-8 (no member is above 0x7F, and no byte of a multi-byte
    // sequence is below 0x80). `purrdf_iri::terminals::is_ws` is the workspace's
    // one transcription of that four-member set; JSON and the Turtle/SPARQL
    // grammars enumerate it independently and arrive at the same four.
    if crate::carrier::whitespace_only(lexical, admission)? {
        // Requirement 27. See the module docs for why the empty geometry is a
        // collection rather than a `POINT EMPTY`.
        return Ok(Geometry::empty(
            CoordDim::Xy,
            GeometryKind::GeometryCollection,
        ));
    }
    let mut progress = admission.map(|admission| crate::carrier::JsonProgress(admission.as_ref()));
    let inner = if let Some(progress) = progress.as_mut() {
        JsonReader::new_observed(lexical, json::LIMITS, progress)
    } else {
        JsonReader::new(lexical, json::LIMITS)
    };
    let mut reader = Reader {
        inner,
        admission: admission.cloned(),
    };
    let pending = read_literal(&mut reader)
        .and_then(|read| reader.finish().map(|()| read))
        .map_err(|error| json::refusal(lexical, error))??;
    let dim = pending.dim.unwrap_or(CoordDim::Xy);
    pending.into_geometry(dim).map_err(|error| {
        GeoError::literal(format!(
            "this geo:geoJSONLiteral is well-formed JSON but does not denote a geometry: {}",
            error.detail()
        ))
    })
}

/// A Geometry object open in the reader: the members that decide it, as far as
/// they have been read. Every other member is checked and passed over.
struct OpenObject<'a> {
    /// How many `type` members, and the first.
    types: usize,
    kind: Option<TypeMember<'a>>,
    /// How many `coordinates` members, and the first.
    coordinates: usize,
    coords: Option<Coordinates>,
    /// How many `geometries` members, and the first.
    geometries: usize,
    members: Option<Members>,
    /// Whether the reader is inside the first `geometries` array.
    in_members: bool,
}

/// A `type` member: the name it holds, or the kind of value it is instead.
enum TypeMember<'a> {
    Name(Cow<'a, str>),
    Other(&'static str),
}

/// The first `coordinates` member of an object.
enum Coordinates {
    /// `null`.
    Null,
    /// Read into the body its object's `type`, already seen, names.
    Read(Result<(Box<GeometryBody>, Option<CoordDim>), GeoError>),
    /// Checked where it stood, its object's `type` not yet known: its span,
    /// read once the object has closed.
    Unread(Range<usize>),
}

/// The first `geometries` member of an object.
enum Members {
    /// `null`.
    Null,
    /// Not an array: the kind of value it is.
    Other(&'static str),
    /// An array, whose elements are read as Geometry objects whatever the
    /// object's `type` turns out to be (only a `GeometryCollection` has
    /// members, and any other type refuses a `geometries` member): the
    /// dimension each member fixed, in written order, and the first member's
    /// refusal.
    Read {
        dims: Vec<Option<CoordDim>>,
        refusal: Option<GeoError>,
    },
}

impl OpenObject<'_> {
    const fn new() -> Self {
        Self {
            types: 0,
            kind: None,
            coordinates: 0,
            coords: None,
            geometries: 0,
            members: None,
            in_members: false,
        }
    }

    /// The object, closed: the dimension its positions fixed, with its node
    /// pushed onto `nodes`, or its refusal. `text` is the document, which an
    /// [`Coordinates::Unread`] span indexes.
    fn judge(
        self,
        reader: &Reader<'_, '_>,
        nodes: &mut Vec<PendingNode>,
    ) -> Step<Option<CoordDim>> {
        let text = reader.text();
        let type_name = match (self.types, self.kind) {
            (1, Some(TypeMember::Name(name))) => name,
            (1, Some(TypeMember::Other(kind))) => {
                return Ok(Err(GeoError::literal(format!(
                    "the `type` member of a GeoJSON Geometry object is a string, but this one is \
                     {kind}"
                ))));
            }
            (0, _) => {
                return Ok(Err(GeoError::literal(
                    "a GeoJSON Geometry object has a `type` member naming its geometry type; this \
                     object has none",
                )));
            }
            (repeats, _) => {
                return Ok(Err(GeoError::literal(format!(
                    "this GeoJSON object has {repeats} `type` members, so it names no single \
                     geometry type; RFC 8259 permits the repetition but resolving it by position \
                     would be a silent choice"
                ))));
            }
        };
        let type_name = &*type_name;
        match type_name {
            "Feature" | "FeatureCollection" => Ok(Err(GeoError::literal(format!(
                "a geo:geoJSONLiteral is a GeoJSON Geometry object and `{type_name}` is not one of \
                 them: GeoSPARQL 1.1 Requirement 25 admits only {GEOMETRY_TYPES}. Write the \
                 geometry itself as the literal rather than the {type_name} that wraps it"
            )))),
            "Point" | "MultiPoint" | "LineString" | "MultiLineString" | "Polygon"
            | "MultiPolygon" => {
                if self.geometries > 0 {
                    return Ok(Err(foreign_shape_member(
                        "geometries",
                        type_name,
                        "coordinates",
                    )));
                }
                let read = match (self.coordinates, self.coords) {
                    (1, Some(Coordinates::Null)) => Err(null_member("coordinates", type_name)),
                    (1, Some(Coordinates::Read(read))) => read,
                    (1, Some(Coordinates::Unread(span))) => {
                        let mut progress = reader
                            .admission
                            .as_ref()
                            .map(|admission| crate::carrier::JsonProgress(admission.as_ref()));
                        let inner = if let Some(progress) = progress.as_mut() {
                            JsonReader::new_observed(&text[span], json::LIMITS, progress)
                        } else {
                            JsonReader::new(&text[span], json::LIMITS)
                        };
                        let mut deferred = Reader {
                            inner,
                            admission: reader.admission.clone(),
                        };
                        read_coordinates(&mut deferred, type_name)?
                    }
                    (count, _) => Err(member_count("coordinates", type_name, count)),
                };
                Ok(read.map(|(body, dim)| {
                    nodes.push(PendingNode::Body(body));
                    dim
                }))
            }
            "GeometryCollection" => {
                if self.coordinates > 0 {
                    return Ok(Err(foreign_shape_member(
                        "coordinates",
                        type_name,
                        "geometries",
                    )));
                }
                Ok(match (self.geometries, self.members) {
                    (1, Some(Members::Null)) => Err(null_member("geometries", type_name)),
                    (1, Some(Members::Other(kind))) => Err(not_an_array(
                        "the `geometries` of a GeoJSON GeometryCollection",
                        kind,
                    )),
                    (
                        1,
                        Some(Members::Read {
                            refusal: Some(refusal),
                            ..
                        }),
                    ) => Err(refusal),
                    (
                        1,
                        Some(Members::Read {
                            dims,
                            refusal: None,
                        }),
                    ) => unified_dim(&dims)
                        .inspect(|_| nodes.push(PendingNode::Collection(dims.len()))),
                    (count, _) => Err(member_count("geometries", type_name, count)),
                })
            }
            other => Ok(Err(GeoError::literal(format!(
                "`{other}` is not a GeoJSON geometry type; RFC 7946 defines {GEOMETRY_TYPES}"
            )))),
        }
    }
}

/// Read the literal's one Geometry object, with every collection it nests.
///
/// Iterative: the open objects live on a heap stack, and the loop alternates
/// between reading the innermost object's members, reading its collection's
/// next member, and handing a closed object to the collection that holds it.
/// Nesting therefore costs heap and never stack, so a literal is read however
/// deep it nests. A member of a collection that has already been refused is
/// checked and not read.
fn read_literal(reader: &mut Reader<'_, '_>) -> Step<Pending> {
    let mut nodes: Vec<PendingNode> = Vec::new();
    let mut open: Vec<OpenObject<'_>> = Vec::new();
    let mut closed = open_object(reader, &mut open)?;
    loop {
        if let Some(outcome) = closed.take() {
            let Some(parent) = open.last_mut() else {
                return Ok(outcome.map(|dim| Pending { dim, nodes }));
            };
            let Some(Members::Read { dims, refusal }) = &mut parent.members else {
                unreachable!("a closed object is the literal or a collection's member");
            };
            match outcome {
                Ok(dim) => dims.push(dim),
                Err(error) => *refusal = Some(error),
            }
        }
        let top = open.last_mut().expect("an object is open");
        if top.in_members {
            if !reader.next_item()? {
                top.in_members = false;
            } else if matches!(
                top.members,
                Some(Members::Read {
                    refusal: Some(_),
                    ..
                })
            ) {
                reader.check_value()?;
            } else {
                closed = open_object(reader, &mut open)?;
            }
            continue;
        }
        let Some(key) = reader.next_key()? else {
            let object = open.pop().expect("an object is open");
            closed = Some(object.judge(reader, &mut nodes)?);
            continue;
        };
        match &*key.decode()? {
            "type" => {
                top.types += 1;
                if top.types == 1 {
                    top.kind = Some(read_type_member(reader)?);
                } else {
                    reader.check_value()?;
                }
            }
            "coordinates" => {
                top.coordinates += 1;
                if top.coordinates == 1 {
                    top.coords = Some(coordinates_member(reader, top.kind.as_ref())?);
                } else {
                    reader.check_value()?;
                }
            }
            "geometries" => {
                top.geometries += 1;
                if top.geometries > 1 {
                    reader.check_value()?;
                    continue;
                }
                top.members = Some(match reader.peek_kind() {
                    Some(Kind::Null) => {
                        reader.next_event()?;
                        Members::Null
                    }
                    Some(Kind::Array) => {
                        reader.next_event()?;
                        top.in_members = true;
                        Members::Read {
                            dims: Vec::new(),
                            refusal: None,
                        }
                    }
                    kind => {
                        reader.check_value()?;
                        Members::Other(kind_name(kind))
                    }
                });
            }
            _ => {
                reader.check_value()?;
            }
        }
    }
}

/// A Geometry object at the cursor: opened onto `open` (`None`), or, when the
/// value is not an object, checked and refused.
fn open_object<'a>(
    reader: &mut Reader<'a, '_>,
    open: &mut Vec<OpenObject<'a>>,
) -> Result<Option<Result<Option<CoordDim>, GeoError>>, purrdf_lex::json::Error> {
    let kind = reader.peek_kind();
    if kind == Some(Kind::Object) {
        reader.next_event()?;
        open.push(OpenObject::new());
        return Ok(None);
    }
    reader.check_value()?;
    Ok(Some(Err(GeoError::literal(format!(
        "a geo:geoJSONLiteral is an RFC 7946 Geometry object, but this is {}",
        kind_name(kind)
    )))))
}

/// A `type` member's value, the cursor on it.
fn read_type_member<'a>(
    reader: &mut Reader<'a, '_>,
) -> Result<TypeMember<'a>, purrdf_lex::json::Error> {
    let kind = reader.peek_kind();
    if kind == Some(Kind::String) {
        let Event::String(name) = reader.next_event()? else {
            unreachable!("a string begins at the cursor");
        };
        return Ok(TypeMember::Name(name.decode()?));
    }
    reader.check_value()?;
    Ok(TypeMember::Other(kind_name(kind)))
}

/// A `coordinates` member's value, the cursor on it: read into its geometry's
/// body when the object's first `type` has named a type with coordinates, and
/// checked for later otherwise.
fn coordinates_member(
    reader: &mut Reader<'_, '_>,
    kind: Option<&TypeMember<'_>>,
) -> Result<Coordinates, purrdf_lex::json::Error> {
    if reader.peek_kind() == Some(Kind::Null) {
        reader.next_event()?;
        return Ok(Coordinates::Null);
    }
    match kind {
        Some(TypeMember::Name(name)) if has_coordinates(name) => {
            Ok(Coordinates::Read(read_coordinates(reader, name)?))
        }
        _ => Ok(Coordinates::Unread(reader.check_value()?)),
    }
}

/// Whether `type_name` is one of the six types that carry `coordinates`.
fn has_coordinates(type_name: &str) -> bool {
    matches!(
        type_name,
        "Point" | "MultiPoint" | "LineString" | "MultiLineString" | "Polygon" | "MultiPolygon"
    )
}

/// The name of a value's kind, for diagnostics, from its first byte.
const fn kind_name(kind: Option<Kind>) -> &'static str {
    match kind {
        Some(Kind::Null) => "null",
        Some(Kind::True | Kind::False) => "a boolean",
        Some(Kind::Number) => "a number",
        Some(Kind::String) => "a string",
        Some(Kind::Array) => "an array",
        Some(Kind::Object) => "an object",
        // The reader refuses such a byte before the name could be reported.
        None => "not a JSON value",
    }
}

/// `geometries` on a coordinate type, or `coordinates` on a collection.
///
/// Both names are defined by RFC 7946 and each belongs to exactly one family,
/// so an object carrying the other family's member states two incompatible
/// things about itself: a contradiction rather than a foreign member.
/// Genuinely foreign members (`bbox`, `title`, anything else) are ignored,
/// which is the neighbouring case the tests prove.
fn foreign_shape_member(wrong: &str, type_name: &str, right: &str) -> GeoError {
    GeoError::literal(format!(
        "this GeoJSON {type_name} carries a `{wrong}` member, which RFC 7946 defines for the \
         other family of geometry types; a {type_name} states its shape in `{right}`, and an \
         object claiming both is a contradiction rather than a Geometry object with a foreign \
         member"
    ))
}

/// A shape member (`coordinates`, or `geometries` for a collection) that is
/// `null`: RFC 7946 gives `null` no meaning there, and an empty geometry is
/// written `[]`.
fn null_member(name: &str, type_name: &str) -> GeoError {
    GeoError::literal(format!(
        "the `{name}` member of this GeoJSON {type_name} is null; RFC 7946 gives null no meaning \
         there, and an empty geometry is written `\"{name}\":[]`"
    ))
}

/// A shape member absent (`count` is `0`) or repeated: a geometry with no
/// coordinates is not a geometry, and two `coordinates` arrays denote two
/// different geometries.
fn member_count(name: &str, type_name: &str, count: usize) -> GeoError {
    if count == 0 {
        return GeoError::literal(format!(
            "a GeoJSON {type_name} has a `{name}` member; this one has none, and RFC 7946 defines \
             the geometry entirely by it"
        ));
    }
    GeoError::literal(format!(
        "this GeoJSON {type_name} has {count} `{name}` members, which denote different \
         geometries; the literal is ambiguous and is refused rather than resolved by position"
    ))
}

/// `what` is not an array, but a value of kind `kind`.
fn not_an_array(what: &str, kind: &str) -> GeoError {
    GeoError::literal(format!(
        "{what} is a JSON array in RFC 7946, but this is {kind}"
    ))
}

/// Open the array at the cursor; or, when the value is not one, check it and
/// refuse it as `what`.
fn open_array(reader: &mut Reader<'_, '_>, what: &str) -> Step<()> {
    let kind = reader.peek_kind();
    if kind == Some(Kind::Array) {
        reader.next_event()?;
        return Ok(Ok(()));
    }
    reader.check_value()?;
    Ok(Err(not_an_array(what, kind_name(kind))))
}

/// Read the elements of the open array to its close, each with `element`,
/// into `items`; after the first refusal the rest are checked and not read.
fn array_items<'a, T>(
    reader: &mut Reader<'a, '_>,
    items: &mut Vec<T>,
    mut element: impl FnMut(&mut Reader<'a, '_>) -> Step<T>,
) -> Step<()> {
    let mut refusal = None;
    while reader.next_item()? {
        if refusal.is_some() {
            reader.check_value()?;
            continue;
        }
        match element(reader)? {
            Ok(item) => items.push(item),
            Err(error) => refusal = Some(error),
        }
    }
    Ok(refusal.map_or(Ok(()), Err))
}

/// The `coordinates` value of a `type_name` geometry, the cursor on it: the
/// body and the dimension its positions fixed, `None` when it has none.
fn read_coordinates(
    reader: &mut Reader<'_, '_>,
    type_name: &str,
) -> Step<(Box<GeometryBody>, Option<CoordDim>)> {
    let mut dim = None;
    let body = match type_name {
        "Point" => point(reader, &mut dim)?,
        "MultiPoint" => {
            let mut points = Vec::new();
            nested(
                reader,
                "the `coordinates` of a GeoJSON MultiPoint",
                &mut points,
                |reader| {
                    // Every member of a GeoJSON MultiPoint is a real position:
                    // there is no way to write an empty member, so none is `None`.
                    Ok(position(reader, &mut dim)?.map(Some))
                },
            )?
            .map(|()| GeometryBody::MultiPoint(points))
        }
        "LineString" => positions(
            reader,
            "the `coordinates` of a GeoJSON LineString",
            &mut dim,
        )?
        .map(GeometryBody::LineString),
        "MultiLineString" => {
            let mut lines = Vec::new();
            nested(
                reader,
                "the `coordinates` of a GeoJSON MultiLineString",
                &mut lines,
                |reader| {
                    positions(
                        reader,
                        "a member LineString of a GeoJSON MultiLineString",
                        &mut dim,
                    )
                },
            )?
            .map(|()| GeometryBody::MultiLineString(lines))
        }
        "Polygon" => rings(reader, "the `coordinates` of a GeoJSON Polygon", &mut dim)?
            .map(GeometryBody::Polygon),
        "MultiPolygon" => {
            let mut polygons = Vec::new();
            nested(
                reader,
                "the `coordinates` of a GeoJSON MultiPolygon",
                &mut polygons,
                |reader| {
                    rings(
                        reader,
                        "a member Polygon of a GeoJSON MultiPolygon",
                        &mut dim,
                    )
                },
            )?
            .map(|()| GeometryBody::MultiPolygon(polygons))
        }
        // Only the six coordinate types reach here.
        other => {
            reader.check_value()?;
            Err(GeoError::literal(format!(
                "`{other}` is not a GeoJSON geometry type with coordinates"
            )))
        }
    };
    Ok(body.map(|body| (Box::new(body), dim)))
}

/// An array at the cursor, refused as `what` when it is not one, whose
/// elements `element` reads into `items`.
fn nested<'a, T>(
    reader: &mut Reader<'a, '_>,
    what: &str,
    items: &mut Vec<T>,
    element: impl FnMut(&mut Reader<'a, '_>) -> Step<T>,
) -> Step<()> {
    if let Err(refusal) = open_array(reader, what)? {
        return Ok(Err(refusal));
    }
    array_items(reader, items, element)
}

/// A Point's `coordinates`: `[]` is the empty point; otherwise the array is
/// its position.
fn point(reader: &mut Reader<'_, '_>, dim: &mut Option<CoordDim>) -> Step<GeometryBody> {
    if let Err(refusal) = open_array(reader, "the `coordinates` of a GeoJSON Point")? {
        return Ok(Err(refusal));
    }
    // RFC 7946 has no empty position, but an empty `coordinates` array is how
    // an empty geometry of each type is written.
    if !reader.next_item()? {
        return Ok(Ok(GeometryBody::Point(None)));
    }
    Ok(position_items(reader, dim, true)?.map(|coord| GeometryBody::Point(Some(coord))))
}

/// A linear ring sequence at the cursor, refused as `what` when it is not an
/// array. The "at least four positions" and "last repeats the first" checks
/// are `Geometry::new`'s; duplicating them here would be a second place for
/// them to drift.
fn rings(reader: &mut Reader<'_, '_>, what: &str, dim: &mut Option<CoordDim>) -> Step<Rings> {
    let mut rings = Rings::new();
    Ok(nested(reader, what, &mut rings, |reader| {
        positions(reader, "a GeoJSON linear ring", dim)
    })?
    .map(|()| rings))
}

/// A position sequence at the cursor, refused as `what` when it is not an
/// array.
fn positions(
    reader: &mut Reader<'_, '_>,
    what: &str,
    dim: &mut Option<CoordDim>,
) -> Step<CoordSeq> {
    let mut coords = CoordSeq::new();
    Ok(nested(reader, what, &mut coords, |reader| position(reader, dim))?.map(|()| coords))
}

/// One RFC 7946 position at the cursor: `[longitude, latitude]` or
/// `[longitude, latitude, altitude]`, and nothing else.
fn position(reader: &mut Reader<'_, '_>, dim: &mut Option<CoordDim>) -> Step<Coord> {
    if let Err(refusal) = open_array(reader, "a GeoJSON position")? {
        return Ok(Err(refusal));
    }
    position_items(reader, dim, false)
}

/// The elements of an open position array, read to its close (`first` when
/// the reader already stands on the first element): each ordinate straight
/// from its number's lexeme into the exact model. The length is judged before
/// any ordinate, and the dimension before any ordinate too, as the elements'
/// count decides both.
fn position_items(
    reader: &mut Reader<'_, '_>,
    dim: &mut Option<CoordDim>,
    first: bool,
) -> Step<Coord> {
    /// An element among the first three: a number's lexeme, or the kind of
    /// value it is instead.
    #[derive(Clone, Copy)]
    enum Ordinate<'t> {
        Number(&'t str),
        Other(&'static str),
    }
    let mut ordinates = [Ordinate::Other("null"); 3];
    let mut count = 0_usize;
    let mut first = first;
    while mem::take(&mut first) || reader.next_item()? {
        if let Some(slot) = ordinates.get_mut(count) {
            let kind = reader.peek_kind();
            *slot = if kind == Some(Kind::Number) {
                let Event::Number { lexeme, .. } = reader.next_event()? else {
                    unreachable!("a number begins at the cursor");
                };
                Ordinate::Number(lexeme)
            } else {
                reader.check_value()?;
                Ordinate::Other(kind_name(kind))
            };
        } else {
            reader.check_value()?;
        }
        count += 1;
    }
    let here = match count {
        2 => CoordDim::Xy,
        3 => CoordDim::Xyz,
        few @ (0 | 1) => {
            return Ok(Err(GeoError::literal(format!(
                "a GeoJSON position is an array of two or three numbers (longitude, latitude and \
                 an optional altitude); this one has {few}"
            ))));
        }
        many => {
            return Ok(Err(GeoError::literal(format!(
                "a GeoJSON position has at most three numbers; RFC 7946 §3.1.1 says \
                 \"Implementations SHOULD NOT extend positions beyond three elements\", so a \
                 position of {many} elements is refused rather than silently truncated. The RFC \
                 gives the extra elements no meaning and GeoJSON has no measure ordinate for them \
                 to become, so ignoring them would discard a number without saying so; an \
                 extension that used a fourth element would need its own datatype"
            ))));
        }
    };
    match *dim {
        None => *dim = Some(here),
        Some(fixed) if fixed == here => {}
        Some(fixed) => {
            return Ok(Err(GeoError::literal(format!(
                "this GeoJSON geometry mixes {}-element and {}-element positions; the geometry \
                 model carries one coordinate dimension for a whole geometry, so a geometry whose \
                 positions disagree about it has no dimension to report to \
                 `geof:coordinateDimension`. RFC 7946 does not itself forbid the mixture — this \
                 is purrdf-geo's model refusing rather than silently dropping or inventing an \
                 altitude",
                fixed.ordinates(),
                here.ordinates()
            ))));
        }
    }
    if let Some(admission) = &reader.admission
        && let Err(error) = admission.borrow_mut().coordinate()
    {
        return Ok(Err(error));
    }
    let ordinate = |ordinate: Ordinate<'_>| match ordinate {
        Ordinate::Number(lexeme) => read_ordinate(lexeme, reader.admission.as_ref()),
        Ordinate::Other(kind) => Err(GeoError::literal(format!(
            "an ordinate of a GeoJSON position is a number, but this one is {kind}"
        ))),
    };
    let coord = (|| {
        let x = ordinate(ordinates[0])?;
        let y = ordinate(ordinates[1])?;
        let z = if here.has_z() {
            Some(ordinate(ordinates[2])?)
        } else {
            None
        };
        Ok(Coord::new(x, y, z, None))
    })();
    Ok(coord)
}

/// One ordinate, decided exactly from the JSON number's own text.
fn read_ordinate(
    lexeme: &str,
    admission: Option<&Rc<RefCell<crate::carrier::ParseAdmission<'_>>>>,
) -> Result<Rat, GeoError> {
    // `Rat::parse_decimal` reads the digits, never an `f64`; it refuses only an
    // exponent so large that the power of ten could not be built.
    let value = if let Some(admission) = admission {
        admission.borrow_mut().decimal(lexeme)?
    } else {
        Rat::parse_decimal(lexeme)
    };
    value.ok_or_else(|| {
        GeoError::literal(format!(
            "the ordinate `{lexeme}` is a valid JSON number but its exponent is past the exact \
             decimal reader's cap, so it names no value this crate can hold"
        ))
    })
}

/// A geometry read out of the literal but not yet given a dimension.
///
/// The dimension of a geometry is decided by the positions inside it, and a
/// geometry with no positions decides nothing — `{"type":"Point","coordinates":[]}`
/// is as much an `XYZ` point as an `XY` one. A `GeometryCollection` therefore
/// cannot be built member by member: an empty member has to adopt whatever
/// dimension its *siblings* fix, or a collection of one empty point and one 3D
/// point would be refused for a disagreement that does not exist. So the geometry
/// tree is read first, the dimension is unified across it, and only then is it
/// materialized.
///
/// The tree is kept flat, as its nodes in post-order — every member before the
/// collection that holds it — which is the order the geometries are built in, so
/// neither holding it nor dropping it nor materializing it walks a level per stack
/// frame.
struct Pending {
    /// `Some` once a position has fixed the dimension; `None` while the geometry
    /// has no positions at all.
    dim: Option<CoordDim>,
    /// The nodes in post-order; the last is the root.
    nodes: Vec<PendingNode>,
}

enum PendingNode {
    /// Any of the six non-collection kinds, already complete. Boxed because a
    /// `GeometryBody` inlines a `SmallVec` of exact coordinates and is two
    /// orders of magnitude larger than the collection variant beside it.
    Body(Box<GeometryBody>),
    /// A `GeometryCollection` of this many members, which are the subtrees just
    /// before it.
    Collection(usize),
}

impl Pending {
    /// The geometry, every node given `dim`, built bottom-up in one pass over the
    /// post-order: a body becomes a geometry, and a collection takes the last
    /// `count` geometries built as its members. The first refusal is the one the
    /// tree's first offending node raises, in written order.
    fn into_geometry(self, dim: CoordDim) -> Result<Geometry, GeoError> {
        let mut built: Vec<Geometry> = Vec::new();
        for node in self.nodes {
            let geometry = match node {
                PendingNode::Body(body) => Geometry::new(dim, *body)?,
                PendingNode::Collection(count) => {
                    let first = built
                        .len()
                        .checked_sub(count)
                        .expect("a collection's members are built before it");
                    let members: Vec<Geometry> = built.drain(first..).collect();
                    Geometry::new(dim, GeometryBody::GeometryCollection(members))?
                }
            };
            built.push(geometry);
        }
        Ok(built.pop().expect("the root is the last geometry built"))
    }
}

/// The seven type names, for the message an unknown one produces.
const GEOMETRY_TYPES: &str = "`Point`, `MultiPoint`, `LineString`, `MultiLineString`, `Polygon`, `MultiPolygon` or \
     `GeometryCollection`";

/// The one dimension every member of a collection shares, given the dimension each
/// member fixed in written order, or `None` when no member has any position at all.
fn unified_dim(dims: &[Option<CoordDim>]) -> Result<Option<CoordDim>, GeoError> {
    let mut unified: Option<CoordDim> = None;
    for member in dims {
        let Some(member_dim) = *member else {
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
    check_reference(literal, required_crs)?;
    write_bare(literal.geometry(), coordinate_scale)
}

fn check_reference(literal: &GeometryLiteral, required_crs: &Crs) -> Result<(), GeoError> {
    if literal.crs() != required_crs {
        return Err(GeoError::domain(format!(
            "this geometry is expressed in <{}> but a geo:geoJSONLiteral is fixed to <{}>; RFC \
             7946 admits exactly one coordinate reference system and purrdf-geo reprojects \
             nothing, so it refuses rather than relabelling the ordinates",
            literal.crs(),
            required_crs
        )));
    }
    Ok(())
}

/// Render the same GeoJSON bytes under the worker's remaining arithmetic and
/// output-storage admission. The caller retains the admitted completed text.
/// # Errors
/// Preserves carrier-domain errors and refuses incomplete work or output storage.
pub fn write_in_context(
    literal: &GeometryLiteral,
    required_crs: &Crs,
    coordinate_scale: u32,
    context: &mut crate::MetricContext,
) -> Result<String, GeoError> {
    write_admitted(
        literal,
        required_crs,
        coordinate_scale,
        context,
        &mut crate::context::WorkProgress::new(None),
    )
}

/// Render the original GeoJSON carrier with bounded governor/cancellation polls.
/// # Errors
/// Adds observer refusal to [`write_in_context`].
pub fn write_in_context_metered(
    literal: &GeometryLiteral,
    required_crs: &Crs,
    coordinate_scale: u32,
    context: &mut crate::MetricContext,
    observer: &mut dyn crate::MetricWorkObserver,
) -> Result<String, GeoError> {
    write_admitted(
        literal,
        required_crs,
        coordinate_scale,
        context,
        &mut crate::context::WorkProgress::new(Some(observer)),
    )
}

pub(crate) fn write_admitted(
    literal: &GeometryLiteral,
    required_crs: &Crs,
    coordinate_scale: u32,
    context: &mut crate::MetricContext,
    progress: &mut crate::context::WorkProgress<'_>,
) -> Result<String, GeoError> {
    check_reference(literal, required_crs)?;
    let mut writer = Writer::admitted(context, progress);
    let result = write_with(literal.geometry(), coordinate_scale, &mut writer);
    writer.finish(result.is_ok())?;
    result
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
    write_with(geometry, coordinate_scale, &mut Writer::plain())
}

fn write_with(
    geometry: &Geometry,
    coordinate_scale: u32,
    writer: &mut Writer<'_, '_>,
) -> Result<String, GeoError> {
    if geometry.dim().has_m() {
        return Err(GeoError::domain(format!(
            "this geometry is {} and GeoJSON has no measure ordinate: RFC 7946 §3.1.1 defines a \
             position as longitude, latitude and an optional altitude, so writing it would drop \
             the measure. purrdf-geo refuses rather than silently discarding an ordinate; write \
             it as a geo:wktLiteral instead",
            geometry.dim().name()
        )));
    }
    let mut out = String::new();
    writer.tree(&mut out, geometry, |out, member, writer| {
        writer.text(out, "{\"type\":\"")?;
        // Every geometry kind has a fixed ASCII RFC 7946 name.
        writer.text(out, member.kind().geojson_type())?;
        if let GeometryBody::GeometryCollection(members) = member.body() {
            writer.text(out, "\",\"geometries\":[")?;
            if members.is_empty() {
                writer.text(out, "]}")?;
                Ok(None)
            } else {
                Ok(Some("]}"))
            }
        } else {
            writer.text(out, "\",\"coordinates\":")?;
            write_coordinates(out, member.body(), coordinate_scale, writer)?;
            writer.text(out, "}")?;
            Ok(None)
        }
    })?;
    Ok(out)
}

fn empty_multipoint_error() -> GeoError {
    GeoError::domain(
        "this MULTIPOINT holds an empty member point and GeoJSON has no syntax \
         for one: RFC 7946 has no empty position, and writing the member as `[]` \
         or omitting it would change how many members the geometry has. \
         purrdf-geo refuses rather than dropping a member; a MULTIPOINT with no \
         members at all writes as `[]` and is unaffected",
    )
}

fn write_coordinates(
    out: &mut String,
    body: &GeometryBody,
    scale: u32,
    writer: &mut Writer<'_, '_>,
) -> Result<(), GeoError> {
    match body {
        GeometryBody::Point(point) => match point {
            Some(point) => write_position(out, point, scale, writer),
            None => writer.text(out, "[]"),
        },
        GeometryBody::MultiPoint(points) => {
            writer.sequence(out, points, "[", "]", |out, point, writer| {
                write_position(
                    out,
                    point.as_ref().ok_or_else(empty_multipoint_error)?,
                    scale,
                    writer,
                )
            })
        }
        GeometryBody::LineString(points) => write_positions(out, points, scale, writer),
        GeometryBody::MultiLineString(lines) | GeometryBody::Polygon(lines) => {
            write_lines(out, lines, scale, writer)
        }
        GeometryBody::MultiPolygon(polygons) => {
            writer.sequence(out, polygons, "[", "]", |out, rings, writer| {
                write_lines(out, rings, scale, writer)
            })
        }
        GeometryBody::GeometryCollection(_) => {
            unreachable!("collections use the carrier writer's original work list")
        }
    }
}

fn write_lines(
    out: &mut String,
    lines: &[CoordSeq],
    scale: u32,
    writer: &mut Writer<'_, '_>,
) -> Result<(), GeoError> {
    writer.sequence(out, lines, "[", "]", |out, points, writer| {
        write_positions(out, points, scale, writer)
    })
}

fn write_positions(
    out: &mut String,
    points: &[Coord],
    scale: u32,
    writer: &mut Writer<'_, '_>,
) -> Result<(), GeoError> {
    writer.sequence(out, points, "[", "]", |out, point, writer| {
        write_position(out, point, scale, writer)
    })
}

fn write_position(
    out: &mut String,
    point: &Coord,
    scale: u32,
    writer: &mut Writer<'_, '_>,
) -> Result<(), GeoError> {
    writer.text(out, "[")?;
    writer.ordinate(out, point.x(), scale)?;
    writer.text(out, ",")?;
    writer.ordinate(out, point.y(), scale)?;
    if let Some(z) = point.z() {
        writer.text(out, ",")?;
        writer.ordinate(out, z, scale)?;
    }
    writer.text(out, "]")
}

/// Compare streaming output with the independent value-tree syntax oracle.
#[cfg(test)]
fn geometry_value(geometry: &Geometry, scale: u32) -> Result<JsonValue, GeoError> {
    json::parse(&write_bare(geometry, scale)?)
}

/// The `"type"` member of a geometry's object.
#[cfg(test)]
fn type_member(geometry: &Geometry) -> (String, JsonValue) {
    (
        "type".to_owned(),
        JsonValue::String(geometry.kind().geojson_type().to_owned()),
    )
}

/// The object for a collection whose members' objects are `members`, in written
/// order.
#[cfg(test)]
fn collection_value(geometry: &Geometry, members: Vec<JsonValue>) -> JsonValue {
    JsonValue::object([
        type_member(geometry),
        ("geometries".to_owned(), JsonValue::Array(members)),
    ])
}

/// The object for a geometry that holds positions rather than geometries.
#[cfg(test)]
fn body_value(geometry: &Geometry, scale: u32) -> Result<JsonValue, GeoError> {
    let coordinates = match geometry.body() {
        GeometryBody::Point(point) => point.as_ref().map_or_else(
            || JsonValue::Array(Vec::new()),
            |c| position_value(c, scale),
        ),
        GeometryBody::MultiPoint(points) => {
            let mut items = Vec::with_capacity(points.len());
            for point in points {
                let Some(coord) = point.as_ref() else {
                    return Err(empty_multipoint_error());
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
        GeometryBody::GeometryCollection(_) => {
            unreachable!("a collection's object is assembled from its members' objects")
        }
    };
    Ok(JsonValue::object([
        type_member(geometry),
        ("coordinates".to_owned(), coordinates),
    ]))
}

#[cfg(test)]
fn rings_value(rings: &Rings, scale: u32) -> JsonValue {
    JsonValue::Array(
        rings
            .iter()
            .map(|ring| sequence_value(ring, scale))
            .collect(),
    )
}

#[cfg(test)]
fn sequence_value(coords: &CoordSeq, scale: u32) -> JsonValue {
    JsonValue::Array(
        coords
            .iter()
            .map(|coord| position_value(coord, scale))
            .collect(),
    )
}

/// One ordinate as a JSON number: its exact decimal rendering at `scale`, which
/// is always `-? digits [ "." digits ]` with no leading zero before another digit.
#[cfg(test)]
fn ordinate_value(ordinate: &Rat, scale: u32) -> JsonValue {
    JsonValue::Number(
        Number::from_lexeme(ordinate.to_decimal_string(scale))
            .expect("a decimal rendering is an RFC 8259 number"),
    )
}

#[cfg(test)]
fn position_value(coord: &Coord, scale: u32) -> JsonValue {
    let mut ordinates = Vec::with_capacity(3);
    ordinates.push(ordinate_value(coord.x(), scale));
    ordinates.push(ordinate_value(coord.y(), scale));
    if let Some(z) = coord.z() {
        ordinates.push(ordinate_value(z, scale));
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

    /// "Whitespace-only" is RFC 8259 §2 `ws = *( %x20 / %x09 / %x0A / %x0D )`,
    /// the four code points [`crate::json`]'s scanner skips — not the
    /// twenty-six of [`char::is_whitespace`].
    ///
    /// The two have to be the same set or this crate contradicts itself: the
    /// JSON scanner never skipped U+00A0, so a literal with one in front of an
    /// object has always been malformed, and the `str::trim` this function once
    /// used called the same scalar "an empty geometry" when it stood alone.
    /// Pinned in both directions — the refusal vectors below, and every run of
    /// the four still parsing.
    #[test]
    fn only_rfc_8259_ws_makes_a_geojson_literal_empty() {
        // Refusal vectors: Unicode whitespace RFC 8259 does not name.
        for outside_ws in [
            "\u{a0}",   // NO-BREAK SPACE
            "\u{2028}", // LINE SEPARATOR
            "\u{3000}", // IDEOGRAPHIC SPACE
            "\u{c}",    // FORM FEED — JSON's `ws` does not name it
            " \u{a0} ", // mixed with the real thing
        ] {
            let _ = refusal(outside_ws);
        }
        // The consistency this buys: the same scalar ahead of a real geometry
        // was always refused by the JSON scanner, and still is.
        let _ = refusal("\u{a0}{\"type\":\"Point\",\"coordinates\":[1,2]}");
        // The VALID neighbours: every run of the four is still empty…
        for empty in ["", " ", "\t", "\r", "\n", "\r\n", "\t \r\n\t "] {
            assert!(
                parsed(empty).is_empty(),
                "{empty:?} must still denote the empty geometry"
            );
        }
        // …and a real geometry padded with them still parses, so narrowing the
        // empty test refused nothing the scanner accepted.
        assert!(
            !parsed(" \t\r\n{\"type\":\"Point\",\"coordinates\":[1,2]}\n ").is_empty(),
            "a padded geometry is still a geometry"
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

/// The reader's and the writer's walks over nested collections, against recursive
/// references on generated literals and a hundred thousand levels deep.
#[cfg(test)]
mod nesting_tests {
    use super::{
        GEOMETRY_TYPES, body_value, collection_value, geometry_of, geometry_value, unified_dim,
        write_bare,
    };
    use crate::error::GeoError;
    use crate::exact::Rat;
    use crate::geom::arbitrary::{self, Lcg};
    use crate::geom::{Coord, CoordDim, CoordSeq, Geometry, GeometryBody, GeometryKind, Rings};
    use crate::json::{self, JsonValue};

    const SCALE: u32 = 12;

    // ---- the tree reader the reference reads with -------------------------
    //
    // The reader over a built JSON tree that the streaming reader replaced:
    // every Geometry object judged from its whole member list, the order of
    // its refusals stated by the order of the checks. The streaming reader
    // must meet the same refusal first on every input, which is what the
    // generated-literal comparison below holds it to.

    /// What one Geometry object is: a complete body, or a collection whose members are
    /// still to be read.
    enum Node<'a> {
        Body {
            dim: Option<CoordDim>,
            body: Box<GeometryBody>,
        },
        Collection(&'a [JsonValue]),
    }

    /// Read one Geometry object without descending into a collection's members.
    fn read_node(value: &JsonValue) -> Result<Node<'_>, GeoError> {
        if !matches!(value, JsonValue::Object(_)) {
            return Err(GeoError::literal(format!(
                "a geo:geoJSONLiteral is an RFC 7946 Geometry object, but this is {}",
                json::kind_name(value)
            )));
        }
        let type_name = read_type(value)?;
        match type_name {
            "Feature" | "FeatureCollection" => Err(GeoError::literal(format!(
                "a geo:geoJSONLiteral is a GeoJSON Geometry object and `{type_name}` is not one of \
                 them: GeoSPARQL 1.1 Requirement 25 admits only {GEOMETRY_TYPES}. Write the geometry \
                 itself as the literal rather than the {type_name} that wraps it"
            ))),
            "Point" | "MultiPoint" | "LineString" | "MultiLineString" | "Polygon"
            | "MultiPolygon" => {
                reject_foreign_shape_member(value, "geometries", type_name, "coordinates")?;
                let coordinates = decisive_member(value, "coordinates", type_name)?;
                let mut dim = None;
                let body = read_coordinates(type_name, coordinates, &mut dim)?;
                Ok(Node::Body {
                    dim,
                    body: Box::new(body),
                })
            }
            "GeometryCollection" => {
                reject_foreign_shape_member(value, "coordinates", type_name, "geometries")?;
                let geometries = decisive_member(value, "geometries", type_name)?;
                let items = expect_array(
                    geometries,
                    "the `geometries` of a GeoJSON GeometryCollection",
                )?;
                Ok(Node::Collection(items))
            }
            other => Err(GeoError::literal(format!(
                "`{other}` is not a GeoJSON geometry type; RFC 7946 defines {GEOMETRY_TYPES}"
            ))),
        }
    }

    /// The `type` member, which every Geometry object has exactly one of and which is
    /// always a string.
    fn read_type(object: &JsonValue) -> Result<&str, GeoError> {
        match json::count(object, "type") {
            0 => Err(GeoError::literal(
                "a GeoJSON Geometry object has a `type` member naming its geometry type; this object \
                 has none",
            )),
            1 => match object.get("type") {
                Some(JsonValue::String(name)) => Ok(name.as_str()),
                Some(other) => Err(GeoError::literal(format!(
                    "the `type` member of a GeoJSON Geometry object is a string, but this one is {}",
                    json::kind_name(other)
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
        match json::count(object, name) {
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
        if json::count(object, wrong) == 0 {
            return Ok(());
        }
        Err(GeoError::literal(format!(
            "this GeoJSON {type_name} carries a `{wrong}` member, which RFC 7946 defines for the \
             other family of geometry types; a {type_name} states its shape in `{right}`, and an \
             object claiming both is a contradiction rather than a Geometry object with a foreign \
             member"
        )))
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
                let items =
                    expect_array(coordinates, "the `coordinates` of a GeoJSON MultiPolygon")?;
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
        let JsonValue::Number(number) = value else {
            return Err(GeoError::literal(format!(
                "an ordinate of a GeoJSON position is a number, but this one is {}",
                json::kind_name(value)
            )));
        };
        // `Rat::parse_decimal` reads the digits, never an `f64`; it refuses only an
        // exponent so large that the power of ten could not be built.
        let lexeme = number.lexeme();
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
                json::kind_name(other)
            ))),
        }
    }

    // ---- the recursive references -----------------------------------------

    /// The tree-shaped intermediate the recursive reference reads into: each node
    /// with the dimension its positions fixed.
    enum Reference {
        Body(Option<CoordDim>, Box<GeometryBody>),
        Collection(Option<CoordDim>, Vec<Self>),
    }

    impl Reference {
        fn dim(&self) -> Option<CoordDim> {
            match self {
                Self::Body(dim, _) | Self::Collection(dim, _) => *dim,
            }
        }
    }

    /// The recursive reading: one object through `read_node`, its members through
    /// itself, its dimension unified when its last member is read.
    fn reference_read(value: &JsonValue) -> Result<Reference, GeoError> {
        match read_node(value)? {
            Node::Body { dim, body } => Ok(Reference::Body(dim, body)),
            Node::Collection(items) => {
                let mut members = Vec::with_capacity(items.len());
                for item in items {
                    members.push(reference_read(item)?);
                }
                let dims: Vec<Option<CoordDim>> = members.iter().map(Reference::dim).collect();
                let dim = unified_dim(&dims)?;
                Ok(Reference::Collection(dim, members))
            }
        }
    }

    /// The recursive materialization, every node given `dim`.
    fn reference_materialize(pending: Reference, dim: CoordDim) -> Result<Geometry, GeoError> {
        match pending {
            Reference::Body(_, body) => Geometry::new(dim, *body),
            Reference::Collection(_, members) => {
                let members = members
                    .into_iter()
                    .map(|member| reference_materialize(member, dim))
                    .collect::<Result<Vec<_>, GeoError>>()?;
                Geometry::new(dim, GeometryBody::GeometryCollection(members))
            }
        }
    }

    /// `geometry_of` with the recursive walks in place of the work-list ones.
    fn reference_geometry_of(lexical: &str) -> Result<Geometry, GeoError> {
        if lexical.bytes().all(purrdf_iri::terminals::is_ws) {
            return Ok(Geometry::empty(
                CoordDim::Xy,
                GeometryKind::GeometryCollection,
            ));
        }
        let value = json::parse(lexical)?;
        let pending = reference_read(&value)?;
        let dim = pending.dim().unwrap_or(CoordDim::Xy);
        reference_materialize(pending, dim).map_err(|error| {
            GeoError::literal(format!(
                "this geo:geoJSONLiteral is well-formed JSON but does not denote a geometry: {}",
                error.detail()
            ))
        })
    }

    /// The recursive writer: a collection's object from its members' objects.
    fn reference_geometry_value(geometry: &Geometry, scale: u32) -> Result<JsonValue, GeoError> {
        match geometry.body() {
            GeometryBody::GeometryCollection(members) => {
                let mut items = Vec::with_capacity(members.len());
                for member in members {
                    items.push(reference_geometry_value(member, scale)?);
                }
                Ok(collection_value(geometry, items))
            }
            _ => body_value(geometry, scale),
        }
    }

    // ---- the literal generator --------------------------------------------

    /// Objects both readers must refuse, and one (the 3D point) they must accept
    /// alone and refuse beside a 2D sibling.
    const FAULTS: [&str; 11] = [
        r#"{"type":"Circle","coordinates":[1,2]}"#,
        r#"{"type":"Point","coordinates":null}"#,
        r#"{"type":"Feature","geometry":null}"#,
        r#"{"type":"Point","coordinates":[1,2,3,4]}"#,
        r#"{"type":"GeometryCollection","coordinates":[]}"#,
        r#"{"type":"Point","geometries":[]}"#,
        r#"{"type":"Point","coordinates":[1,2],"coordinates":[3,4]}"#,
        r#"{"type":"LineString","coordinates":[[0,0]]}"#,
        r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[0,0]]]}"#,
        "[1,2]",
        r#"{"type":"Point","coordinates":[1,2,3]}"#,
    ];

    /// One run of RFC 8259 whitespace, one time in three.
    fn space(rng: &mut Lcg, out: &mut String) {
        if rng.chance(3) {
            out.push_str([" ", "\t", "\n", "\r\n"][rng.below(4) as usize]);
        }
    }

    /// An ordinate in one of three spellings of the same value.
    fn ordinate(rng: &mut Lcg, value: &Rat) -> String {
        let plain = value.to_decimal_string(0);
        match rng.below(3) {
            0 => plain,
            1 => format!("{plain}.0"),
            _ => format!("{plain}e0"),
        }
    }

    fn position(rng: &mut Lcg, coord: &Coord) -> String {
        let mut out = format!("[{},{}", ordinate(rng, coord.x()), ordinate(rng, coord.y()));
        if let Some(z) = coord.z() {
            out.push(',');
            out.push_str(&ordinate(rng, z));
        }
        out.push(']');
        out
    }

    fn positions(rng: &mut Lcg, coords: &CoordSeq) -> String {
        let items: Vec<String> = coords.iter().map(|coord| position(rng, coord)).collect();
        format!("[{}]", items.join(","))
    }

    fn rings(rng: &mut Lcg, rings: &[CoordSeq]) -> String {
        let items: Vec<String> = rings.iter().map(|ring| positions(rng, ring)).collect();
        format!("[{}]", items.join(","))
    }

    /// The `coordinates` of a geometry that holds positions. A `MULTIPOINT` member
    /// with no position, which GeoJSON cannot write, is written `[]`, which both
    /// readers must refuse identically.
    fn coordinates(rng: &mut Lcg, body: &GeometryBody) -> String {
        match body {
            GeometryBody::Point(None) => "[]".to_owned(),
            GeometryBody::Point(Some(coord)) => position(rng, coord),
            GeometryBody::LineString(coords) => positions(rng, coords),
            GeometryBody::Polygon(shape) => rings(rng, shape),
            GeometryBody::MultiPoint(points) => {
                let items: Vec<String> = points
                    .iter()
                    .map(|point| {
                        point
                            .as_ref()
                            .map_or_else(|| "[]".to_owned(), |c| position(rng, c))
                    })
                    .collect();
                format!("[{}]", items.join(","))
            }
            GeometryBody::MultiLineString(lines) => {
                let items: Vec<String> = lines.iter().map(|line| positions(rng, line)).collect();
                format!("[{}]", items.join(","))
            }
            GeometryBody::MultiPolygon(polygons) => {
                let items: Vec<String> = polygons.iter().map(|shape| rings(rng, shape)).collect();
                format!("[{}]", items.join(","))
            }
            GeometryBody::GeometryCollection(_) => {
                unreachable!("a collection's members are written by `literal`")
            }
        }
    }

    /// `geometry` as a GeoJSON literal: members in a random order, whitespace
    /// sprinkled, a foreign member one time in three, and one time in eight the
    /// object replaced by one of [`FAULTS`].
    fn literal(rng: &mut Lcg, geometry: &Geometry, out: &mut String) {
        if rng.chance(8) {
            out.push_str(FAULTS[rng.below(11) as usize]);
            return;
        }
        let type_member = format!("\"type\":\"{}\"", geometry.kind().geojson_type());
        let shape_member = match geometry.body() {
            GeometryBody::GeometryCollection(members) => {
                let mut text = String::from("\"geometries\":[");
                for (index, member) in members.iter().enumerate() {
                    if index > 0 {
                        text.push(',');
                    }
                    literal(rng, member, &mut text);
                }
                text.push(']');
                text
            }
            body => format!("\"coordinates\":{}", coordinates(rng, body)),
        };
        let mut members = vec![type_member, shape_member];
        if rng.chance(3) {
            members.push(
                ["\"bbox\":[0,0,1,1]", "\"note\":\"x\"", "\"id\":7"][rng.below(3) as usize]
                    .to_owned(),
            );
        }
        if rng.chance(2) {
            members.swap(0, 1);
        }
        if members.len() == 3 && rng.chance(2) {
            members.swap(1, 2);
        }
        out.push('{');
        space(rng, out);
        for (index, member) in members.iter().enumerate() {
            if index > 0 {
                space(rng, out);
                out.push(',');
                space(rng, out);
            }
            out.push_str(member);
        }
        space(rng, out);
        out.push('}');
    }

    /// How many collections deep a chain of first members goes.
    fn levels(geometry: &Geometry) -> usize {
        let mut depth = 0;
        let mut node = geometry;
        while let GeometryBody::GeometryCollection(members) = node.body() {
            depth += 1;
            match members.first() {
                Some(member) => node = member,
                None => break,
            }
        }
        depth
    }

    // ---- reading ------------------------------------------------------------

    /// Over generated literals — every kind, nested, foreign members, whitespace,
    /// and faults at any level — the work-list reader and the recursive reference
    /// return the same geometry or the same refusal.
    #[test]
    fn the_reader_agrees_with_the_recursive_reference_on_generated_literals() {
        let mut rng = Lcg::new(0x5eed_4001);
        for round in 0..400 {
            let dim = if round % 2 == 0 {
                CoordDim::Xy
            } else {
                CoordDim::Xyz
            };
            let tree = arbitrary::geometry(&mut rng, dim, 4);
            let mut text = String::new();
            literal(&mut rng, &tree, &mut text);
            assert_eq!(
                geometry_of(&text),
                reference_geometry_of(&text),
                "round {round}: the two readers disagree on {text}"
            );
        }
    }

    /// Over generated geometries the work-list writer and the recursive reference
    /// build the same object or raise the same refusal, and a written geometry
    /// reads back to itself.
    #[test]
    fn the_writer_agrees_with_the_recursive_reference_on_generated_geometries() {
        let mut rng = Lcg::new(0x5eed_4002);
        for round in 0..400 {
            let dim = if round % 2 == 0 {
                CoordDim::Xy
            } else {
                CoordDim::Xyz
            };
            let tree = arbitrary::geometry(&mut rng, dim, 4);
            let iterative = geometry_value(&tree, SCALE);
            let recursive = reference_geometry_value(&tree, SCALE);
            assert_eq!(iterative, recursive, "round {round}: {tree:?}");
            if let Ok(value) = recursive {
                let written = write_bare(&tree, SCALE).expect("the object was built");
                assert_eq!(written, json::write(&value), "round {round}");
                // A tree with no position fixes no dimension and reads back planar,
                // so only a tree that is planar or holds a position reads back to
                // itself.
                if tree.dim() == CoordDim::Xy || tree.coord_count() > 0 {
                    assert_eq!(
                        geometry_of(&written),
                        Ok(tree),
                        "round {round}: the written literal reads back: {written}"
                    );
                }
            }
        }
    }

    /// A hundred thousand nested collections are read and written back byte for
    /// byte on a 128 KiB stack, a dimension fixed at the bottom governs the top,
    /// and a refusal at the bottom is the refusal the same object raises alone.
    ///
    /// The expected text is a closed form pinned at depths one and two by literal
    /// strings: each level wraps the one below in
    /// `{"type":"GeometryCollection","geometries":[` and `]}`.
    #[test]
    fn a_hundred_thousand_deep_collection_reads_and_writes() {
        const OPEN: &str = r#"{"type":"GeometryCollection","geometries":["#;
        const CLOSE: &str = "]}";
        const POINT: &str = r#"{"type":"Point","coordinates":[1,2]}"#;
        const ONE: &str =
            r#"{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2]}]}"#;
        const TWO: &str = r#"{"type":"GeometryCollection","geometries":[{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1,2]}]}]}"#;
        assert_eq!(format!("{OPEN}{POINT}{CLOSE}"), ONE);
        assert_eq!(format!("{OPEN}{OPEN}{POINT}{CLOSE}{CLOSE}"), TWO);
        for (text, depth) in [(POINT, 0), (ONE, 1), (TWO, 2)] {
            let geometry = geometry_of(text).expect("a shallow literal reads");
            assert_eq!(levels(&geometry), depth);
            assert_eq!(geometry.coord_count(), 1);
            assert_eq!(write_bare(&geometry, SCALE).expect("no measure"), text);
        }

        let depth = arbitrary::DEEP;
        let wrapped = move |inner: &str| {
            let mut text =
                String::with_capacity(OPEN.len() * depth + inner.len() + CLOSE.len() * depth);
            for _ in 0..depth {
                text.push_str(OPEN);
            }
            text.push_str(inner);
            for _ in 0..depth {
                text.push_str(CLOSE);
            }
            text
        };
        purrdf_stack::on_stack(arbitrary::SMALL_STACK, move || {
            let text = wrapped(POINT);
            assert_eq!(
                text.len(),
                OPEN.len() * depth + POINT.len() + CLOSE.len() * depth
            );
            let geometry = geometry_of(&text).expect("a hundred thousand nested collections read");
            assert_eq!(geometry.kind(), GeometryKind::GeometryCollection);
            assert_eq!(geometry.dim(), CoordDim::Xy);
            assert_eq!(levels(&geometry), depth);
            assert_eq!(geometry.coord_count(), 1);
            assert!(!geometry.is_empty());
            assert_eq!(
                write_bare(&geometry, SCALE).expect("no measure"),
                text,
                "the literal writes back byte for byte"
            );
            drop(geometry);

            // The dimension a position fixes at the bottom governs the whole tree.
            let elevated = geometry_of(&wrapped(r#"{"type":"Point","coordinates":[1,2,3]}"#))
                .expect("a 3D point at the bottom reads");
            assert_eq!(elevated.dim(), CoordDim::Xyz);
            drop(elevated);

            // Nothing at the bottom is the empty geometry through every level.
            let hollow = geometry_of(&wrapped(r#"{"type":"GeometryCollection","geometries":[]}"#))
                .expect("an empty collection at the bottom reads");
            assert!(hollow.is_empty());
            assert_eq!(levels(&hollow), depth + 1);
            drop(hollow);

            // A refusal at the bottom is the refusal the object raises alone.
            let circle = r#"{"type":"Circle","coordinates":[1,2]}"#;
            assert_eq!(geometry_of(&wrapped(circle)), geometry_of(circle));
            let mixed =
                r#"{"type":"Point","coordinates":[1,2]},{"type":"Point","coordinates":[1,2,3]}"#;
            assert_eq!(
                geometry_of(&wrapped(mixed)),
                geometry_of(&format!("{OPEN}{mixed}{CLOSE}"))
            );
        })
        .expect("the thread starts");
    }
}
