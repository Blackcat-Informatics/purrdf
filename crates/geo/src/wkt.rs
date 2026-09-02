// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `geo:wktLiteral` lexical form: a recursive-descent reader that never
//! touches a float, and a writer whose bytes are a pure function of the geometry.
//!
//! # Why the reader is written against bytes rather than a number parser
//!
//! An ordinate reaches this module as text and leaves it as an exact [`Rat`].
//! Nothing in between rounds it, because nothing in between is a float: the number
//! scanner takes the maximal run of bytes that can occur in an XSD decimal or
//! double lexical form and hands that slice straight to [`Rat::parse_decimal`],
//! which builds a numerator and a denominator digit by digit. That scan is also
//! the whole of the `NaN`/`INF` refusal — `N`, `a`, `I` and `F` are not in the
//! run's alphabet, so those spellings can never reach the number parser at all,
//! and the refusal does not depend on what [`Rat::parse_decimal`] happens to think
//! of them. The consequence a caller can rely on: `1.5`, `1.50` and `15e-1`
//! produce the **identical** [`Geometry`], on every target, forever.
//!
//! # Where whitespace is significant, and where it is not
//!
//! OGC writes the dimension as its own token — `POINT Z (1 2 3)` — and
//! `POINTZ(1 2 3)` is not the same literal with a space removed, it is a literal
//! naming a keyword that does not exist. This module gets that distinction for
//! free rather than by a special case: the word reader takes a *maximal* run of
//! ASCII letters, so `POINTZ` is one word and never splits, while everywhere else
//! whitespace is skipped freely. Two word tokens must therefore be separated, and
//! nothing else must be — which is exactly the rule the grammar states, expressed
//! once instead of at every call site. `POINT Z(1 2 3)` (a tag abutting the paren)
//! is accepted for the same reason: a word and a delimiter need no separator, and
//! refusing it would reject a literal the grammar admits.
//!
//! # What the reader refuses, and what it deliberately does not
//!
//! It refuses what is not a geometry *lexically*: an unknown keyword, an ordinate
//! count that disagrees with the declared dimension, empty parentheses (which are
//! neither `EMPTY` nor a position list), unbalanced parentheses, text after the
//! geometry, an empty CRS prefix. Everything *structural* — a one-position line, a
//! ring that does not close, a collection member whose dimension differs — is left
//! to [`Geometry::new`], which is the single site those rules live at, so the
//! reader cannot drift away from the model's idea of a well-formed geometry.
//!
//! It does **not** refuse the forms that are merely unfashionable. `MULTIPOINT` is
//! accepted in both the bare (`MULTIPOINT(1 1,2 2)`) and the parenthesized
//! (`MULTIPOINT((1 1),(2 2))`) spelling because OGC 1.2.1 admits both and both are
//! in the wild; keywords, dimension tags and `EMPTY` are case-insensitive; leading
//! and trailing whitespace is skipped. Over-refusal is not the safe direction: a
//! literal wrongly rejected is a query that silently returns nothing, and it looks
//! exactly like correct strictness from the inside.
//!
//! # The writer picks one spelling and never varies it
//!
//! A geometry has many valid renderings, and a serializer that chose among them by
//! anything other than the geometry would break the byte determinism the rest of
//! PurRDF depends on. So the rendering is fixed: uppercase keywords, `,` with no
//! following space between positions and members, one space between ordinates, the
//! dimension tag spaced on both sides, `MULTIPOINT` always parenthesized. See
//! [`write_bare`] for the full table. There is no map iteration, no time, no RNG
//! and no float on the path, so the same geometry produces the same bytes on every
//! target and every run.

use crate::error::GeoError;
use crate::exact::Rat;
use crate::geom::{
    Coord, CoordDim, CoordSeq, Crs, Geometry, GeometryBody, GeometryKind, GeometryLiteral, Rings,
};

/// How deeply `GEOMETRYCOLLECTION` may nest before the reader refuses.
///
/// The reader is recursive and a `wktLiteral` is untrusted input, so without a cap
/// a literal consisting of nothing but ten thousand `GEOMETRYCOLLECTION(` tokens
/// would exhaust the stack — an abort, which is not a failure mode a query
/// evaluator can catch or report. The cap converts that into an ordinary
/// [`GeoError::Literal`].
///
/// Sixty-four counts *geometries*, not collections: a `POINT` inside 63 nested
/// collections is at depth 64 and is accepted. Real GeoSPARQL corpora nest two or
/// three levels, so the limit is roughly twenty times the deepest thing anyone
/// writes on purpose; it is a guard against a hostile literal, not a modelling
/// constraint.
const MAX_NESTING_DEPTH: usize = 64;

/// The seven keywords, in the order [`GeometryKind`] declares them.
///
/// Recognition reads [`GeometryKind::wkt_keyword`] rather than repeating the
/// spellings, so the reader and the writer cannot disagree about what a keyword is.
const ALL_KINDS: [GeometryKind; 7] = [
    GeometryKind::Point,
    GeometryKind::LineString,
    GeometryKind::Polygon,
    GeometryKind::MultiPoint,
    GeometryKind::MultiLineString,
    GeometryKind::MultiPolygon,
    GeometryKind::GeometryCollection,
];

/// The three dimensions that have a tag; [`CoordDim::Xy`] is the untagged one.
const TAGGED_DIMS: [CoordDim; 3] = [CoordDim::Xyz, CoordDim::Xym, CoordDim::Xyzm];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a `geo:wktLiteral` lexical form.
///
/// The optional leading `<IRI>` names the coordinate reference system the
/// ordinates are expressed in. When it is absent the literal is in
/// `default_crs` — supplied by the caller rather than defaulted here, because
/// PurRDF mints no vocabulary IRIs and there is no CRS this crate could
/// legitimately invent. When it is present it is carried verbatim, and
/// `default_crs` is not consulted.
///
/// # Errors
///
/// [`GeoError::Literal`] for every malformed input: an unknown or misspelled
/// keyword, a dimension tag fused to its keyword (`POINTZ`), an ordinate count
/// that disagrees with the declared dimension, a token that is not a number
/// (`NaN` and `INF` among them), empty or unbalanced parentheses, text after the
/// geometry, an empty CRS prefix, a nesting depth past 64, or a body
/// [`Geometry::new`] refuses structurally (a one-position line, an unclosed ring,
/// a collection member whose dimension differs from the collection's).
///
/// # What it accepts that a stricter reader might not
///
/// `MULTIPOINT` in both the bare and the parenthesized spelling; keywords,
/// dimension tags and `EMPTY` in any case; whitespace before the CRS prefix,
/// after the geometry, and between every pair of tokens; a dimension tag abutting
/// its opening parenthesis (`POINT Z(1 2 3)`); and a collection member that omits
/// the tag its collection declared, which inherits it. Each of those is a
/// conforming or widely-written literal, and refusing one would turn a working
/// query into one that silently returns nothing.
///
/// # Examples
///
/// ```
/// use purrdf_geo::{Crs, GeometryKind, wkt};
///
/// let default = Crs::new("http://example.org/crs/planar").expect("a non-empty IRI");
///
/// // A literal that omits the system is in the caller's default.
/// let literal = wkt::parse("POINT Z (1 2 3)", &default).expect("well formed");
/// assert_eq!(literal.crs(), &default);
/// assert_eq!(literal.geometry().kind(), GeometryKind::Point);
///
/// // One that names a system is in that system, verbatim.
/// let named = wkt::parse("<http://example.org/crs/other> POINT(1 2)", &default)
///     .expect("well formed");
/// assert_eq!(named.crs().as_str(), "http://example.org/crs/other");
///
/// // `POINTZ` fuses the tag onto the keyword and names no OGC geometry.
/// assert!(wkt::parse("POINTZ(1 2 3)", &default).is_err());
/// ```
pub fn parse(lexical: &str, default_crs: &Crs) -> Result<GeometryLiteral, GeoError> {
    if lexical.trim().is_empty() {
        // OGC 22-047r1 /req/geometry-extension/wkt-literal-empty: "An empty RDFS
        // Literal of type geo:wktLiteral shall be interpreted as an empty
        // Geometry." This is the exact mirror of the geoJSON rule in
        // `geojson::geometry_of`, and it is a REQUIREMENT rather than a
        // leniency: refusing here made one empty `geo:asWKT` object anywhere in a
        // dataset abort every `geof:` query that touched it, because a malformed
        // literal is an evaluation error rather than an unmatched row. The
        // shipped GeoSPARQL SHACL shape agrees — its `geo:wktLiteral` pattern
        // begins `^\s*$|...`, admitting the whitespace-only form.
        //
        // A collection rather than a `POINT EMPTY`, for the same reason the
        // geoJSON side gives: the empty geometry has no kind of its own, and a
        // collection is the one kind that carries no commitment about what it
        // would have held.
        return Ok(GeometryLiteral::new(
            default_crs.clone(),
            Geometry::empty(CoordDim::Xy, GeometryKind::GeometryCollection),
        ));
    }
    let mut cursor = Cursor::new(lexical);
    let crs = parse_crs_prefix(&mut cursor)?;
    let geometry = parse_geometry(&mut cursor, CoordDim::Xy, 1)?;
    cursor.skip_whitespace();
    if !cursor.is_at_end() {
        return Err(GeoError::literal(format!(
            "a wktLiteral holds exactly one geometry, but text follows it at byte {}: {}",
            cursor.pos,
            cursor.preview()
        )));
    }
    Ok(GeometryLiteral::new(
        crs.unwrap_or_else(|| default_crs.clone()),
        geometry,
    ))
}

/// Render a geometry literal back to a `geo:wktLiteral` lexical form, CRS prefix
/// included.
///
/// The prefix is emitted as `<IRI>` followed by exactly one space, then the
/// geometry text of [`write_bare`]. See that function for `coordinate_scale` and
/// for the byte-exact rendering table.
///
/// So a point in `http://example.org/crs/planar` renders as exactly
/// `<http://example.org/crs/planar> POINT(1 2)` — the prefix is written even when
/// the literal it came from omitted one, because the system is part of what the
/// geometry means and the reader's default is not recoverable from the text.
///
/// # Examples
///
/// ```
/// use purrdf_geo::{Crs, wkt};
///
/// let crs = Crs::new("http://example.org/crs/planar").expect("a non-empty IRI");
///
/// // Whatever spelling came in, one canonical rendering goes out.
/// let literal = wkt::parse("  linestring ( 0 0 , 1.50 1 )  ", &crs).expect("well formed");
/// assert_eq!(
///     wkt::write(&literal, 12),
///     "<http://example.org/crs/planar> LINESTRING(0 0,1.5 1)"
/// );
///
/// // `write_bare` is the same text without the system.
/// assert_eq!(
///     wkt::write_bare(literal.geometry(), 12),
///     "LINESTRING(0 0,1.5 1)"
/// );
/// ```
#[must_use]
pub fn write(literal: &GeometryLiteral, coordinate_scale: u32) -> String {
    let mut out = String::new();
    out.push('<');
    out.push_str(literal.crs().as_str());
    out.push_str("> ");
    write_geometry(&mut out, literal.geometry(), coordinate_scale);
    out
}

/// Render a geometry **without** the leading CRS prefix.
///
/// This is the form a `GEOMETRYCOLLECTION` member takes (members share the
/// collection's system and may not carry their own), and the form a caller wants
/// when the system travels beside the text rather than inside it.
///
/// # `coordinate_scale`
///
/// The maximum number of fraction digits an ordinate may be written with, passed
/// by the caller rather than fixed here for two reasons. First, an exact
/// coordinate can need more digits than any constant would allow: a literal is
/// free to carry sixty significant digits, and a scale chosen once in this module
/// would silently round it. Second, round-trip fidelity is the *caller's*
/// requirement — a store that must reproduce its input bytes needs a scale at
/// least as large as the widest ordinate it ingested, while a store rendering a
/// map tile wants six and does not care. A scale too small rounds; it never fails.
///
/// # The rendering, byte for byte
///
/// | geometry | text |
/// |---|---|
/// | point | `POINT(1 2)` |
/// | point, 3D | `POINT Z (1 2 3)` |
/// | empty point | `POINT EMPTY` |
/// | empty point, 3D | `POINT Z EMPTY` |
/// | line string | `LINESTRING(0 0,1 1)` |
/// | polygon | `POLYGON((0 0,1 0,0 1,0 0))` |
/// | multipoint | `MULTIPOINT((1 1),(2 2))` |
/// | multipoint with an empty member | `MULTIPOINT(EMPTY)` |
/// | multi line string | `MULTILINESTRING((0 0,1 1),EMPTY)` |
/// | multipolygon | `MULTIPOLYGON(((0 0,1 0,0 1,0 0)))` |
/// | collection | `GEOMETRYCOLLECTION(POINT(1 2),LINESTRING(0 0,1 1))` |
///
/// Keywords and tags are uppercase; positions and members are separated by `,`
/// with no space; ordinates by exactly one space; the dimension tag carries a
/// space on each side, and `EMPTY` is preceded by a space when there is no tag to
/// have supplied one.
///
/// `EMPTY` is written when the body has **no members**, never merely when the
/// geometry denotes the empty set: `MULTIPOINT(EMPTY,EMPTY)` is empty as a set but
/// is two members, and collapsing it to `MULTIPOINT EMPTY` would change the
/// geometry rather than render it.
#[must_use]
pub fn write_bare(geometry: &Geometry, coordinate_scale: u32) -> String {
    let mut out = String::new();
    write_geometry(&mut out, geometry, coordinate_scale);
    out
}

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

/// A byte position in the literal, with the token reads the grammar needs.
///
/// Every read advances only over ASCII bytes or stops at the first byte of a
/// multi-byte character, so `pos` is always a `char` boundary and slicing at it is
/// always sound.
#[derive(Clone, Debug)]
struct Cursor<'a> {
    /// The whole literal, never re-sliced, so byte offsets in diagnostics are
    /// offsets into what the caller passed.
    text: &'a str,
    /// The read position.
    pos: usize,
}

impl<'a> Cursor<'a> {
    /// A cursor at the start of `text`.
    const fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }

    /// The byte at the read position, if any.
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    /// Whether every byte has been read.
    const fn is_at_end(&self) -> bool {
        self.pos >= self.text.len()
    }

    /// Skip a run of ASCII space, tab, carriage return and line feed.
    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.pos += 1;
        }
    }

    /// Consume `byte` if it is next, reporting whether it was.
    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Skip whitespace and consume `byte`, or refuse.
    ///
    /// `what` names the production being read so the diagnostic says which
    /// bracket went missing rather than only that one did.
    fn expect(&mut self, byte: u8, what: &str) -> Result<(), GeoError> {
        self.skip_whitespace();
        if self.eat(byte) {
            return Ok(());
        }
        Err(GeoError::literal(format!(
            "expected {:?} in {what} at byte {}, found {}",
            char::from(byte),
            self.pos,
            self.preview()
        )))
    }

    /// The maximal run of ASCII letters at the read position, possibly empty.
    ///
    /// Maximal munch is what makes `POINTZ` a single unknown keyword rather than
    /// `POINT` followed by a `Z` tag, which is the whole of the "a dimension tag is
    /// its own token" rule.
    fn word(&mut self) -> &'a str {
        let start = self.pos;
        while self.peek().is_some_and(|byte| byte.is_ascii_alphabetic()) {
            self.pos += 1;
        }
        &self.text[start..self.pos]
    }

    /// Skip whitespace and consume `keyword` case-insensitively if it is the next
    /// word, restoring the position when it is not.
    fn eat_keyword(&mut self, keyword: &str) -> bool {
        self.skip_whitespace();
        let start = self.pos;
        if self.word().eq_ignore_ascii_case(keyword) {
            true
        } else {
            self.pos = start;
            false
        }
    }

    /// Read one ordinate as an exact rational.
    ///
    /// The scanned alphabet is exactly the bytes an XSD decimal or double lexical
    /// form is built from. `NaN` and `INF` are refused *here*, by not being
    /// spellable in that alphabet, rather than downstream — so the refusal holds
    /// whatever [`Rat::parse_decimal`] would have made of them, and a coordinate is
    /// never a non-number.
    fn number(&mut self) -> Result<Rat, GeoError> {
        let start = self.pos;
        while self.peek().is_some_and(is_number_byte) {
            self.pos += 1;
        }
        let text = &self.text[start..self.pos];
        if text.is_empty() {
            return Err(GeoError::literal(format!(
                "expected a number at byte {start}, found {}",
                self.preview()
            )));
        }
        Rat::parse_decimal(text).ok_or_else(|| {
            GeoError::literal(format!(
                "{text:?} at byte {start} is not an ordinate this reader can represent exactly; a \
                 WKT ordinate is an XSD decimal or double lexical form (so NaN and INF are not \
                 coordinates), with an exponent inside the exact parser's cap"
            ))
        })
    }

    /// A short, quoted rendering of the text at the read position, for
    /// diagnostics.
    fn preview(&self) -> String {
        let rest = &self.text[self.pos..];
        if rest.is_empty() {
            return "the end of the literal".to_owned();
        }
        let snippet: String = rest.chars().take(16).collect();
        format!("{snippet:?}")
    }
}

/// Whether `byte` can occur inside an XSD decimal or double lexical form.
const fn is_number_byte(byte: u8) -> bool {
    matches!(byte, b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E')
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// Read the optional `<IRI>` prefix, leaving the cursor at the geometry text.
///
/// `Ok(None)` means the literal named no system, which is the caller's default —
/// not a fabricated one.
fn parse_crs_prefix(cursor: &mut Cursor<'_>) -> Result<Option<Crs>, GeoError> {
    cursor.skip_whitespace();
    if !cursor.eat(b'<') {
        return Ok(None);
    }
    let start = cursor.pos;
    let Some(offset) = cursor.text[start..].find('>') else {
        return Err(GeoError::literal(format!(
            "the coordinate reference system prefix opened at byte {} is never closed; it is \
             written <IRI> before the geometry",
            start - 1
        )));
    };
    let iri = &cursor.text[start..start + offset];
    cursor.pos = start + offset + 1;
    if iri.is_empty() {
        return Err(GeoError::literal(
            "the coordinate reference system prefix is empty (<>); an empty IRI names no system, \
             so a literal that means \"the default system\" omits the prefix entirely rather than \
             writing an empty one",
        ));
    }
    // Cannot fail — the emptiness this rejects is the only thing `Crs::new`
    // rejects — but mapped rather than unwrapped so a future check there arrives
    // here as a malformed literal instead of a panic.
    Crs::new(iri)
        .map(Some)
        .map_err(|err| GeoError::literal(err.detail().to_owned()))
}

/// Read one tagged geometry: keyword, optional dimension tag, then `EMPTY` or a
/// coordinate body.
///
/// `inherited` is the dimension an untagged geometry takes. At the top level that
/// is [`CoordDim::Xy`]; inside a collection it is the collection's own dimension,
/// so `GEOMETRYCOLLECTION Z (POINT(1 2 3))` reads as the writer plainly meant it.
/// A member that writes a tag *disagreeing* with the collection is still refused —
/// by [`Geometry::new`], which owns that rule — because the two statements cannot
/// both be true.
fn parse_geometry(
    cursor: &mut Cursor<'_>,
    inherited: CoordDim,
    depth: usize,
) -> Result<Geometry, GeoError> {
    if depth > MAX_NESTING_DEPTH {
        return Err(too_deep(cursor.pos));
    }
    cursor.skip_whitespace();
    let keyword_at = cursor.pos;
    let keyword = cursor.word();
    let Some(kind) = keyword_kind(keyword) else {
        return Err(unknown_keyword(cursor, keyword_at, keyword));
    };

    let mut dim = inherited;
    cursor.skip_whitespace();
    let tag_at = cursor.pos;
    let tag = cursor.word();
    if !tag.is_empty() {
        if let Some(tagged) = tag_dim(tag) {
            dim = tagged;
        } else if tag.eq_ignore_ascii_case("EMPTY") {
            return Ok(Geometry::empty(inherited, kind));
        } else {
            return Err(bad_tag(kind, tag_at, tag));
        }
        // A tag may be followed by EMPTY or by the coordinate body, and by nothing
        // else; a second word here is a misspelling, not a second tag.
        cursor.skip_whitespace();
        let after_tag = cursor.pos;
        let follower = cursor.word();
        if !follower.is_empty() {
            if follower.eq_ignore_ascii_case("EMPTY") {
                return Ok(Geometry::empty(dim, kind));
            }
            return Err(bad_tag_follower(tag, after_tag, follower));
        }
    }

    let body = parse_body(cursor, kind, dim, depth)?;
    Geometry::new(dim, body)
}

/// The nesting refusal, built off the recursive frame.
///
/// Every diagnostic on the collection recursion path is built in a `#[cold]`
/// function like this one rather than inline. That is a *stack* decision, not a
/// stylistic one: at `opt-level = 0` a `format!` expansion's temporaries occupy
/// slots in the frame that holds them whether or not the branch is ever taken, and
/// this frame is multiplied by the nesting depth. See `parse_body` for the same
/// argument applied to the geometry bodies themselves.
#[cold]
#[inline(never)]
fn too_deep(at: usize) -> GeoError {
    GeoError::literal(format!(
        "geometries are nested more than {MAX_NESTING_DEPTH} deep at byte {at}; the reader is \
         recursive, so an unbounded nesting would exhaust the stack rather than fail"
    ))
}

/// The refusal for a word that is not one of the seven keywords.
#[cold]
#[inline(never)]
fn unknown_keyword(cursor: &Cursor<'_>, at: usize, word: &str) -> GeoError {
    GeoError::literal(format!(
        "expected a geometry keyword at byte {at}, found {}; WKT names seven (POINT, LINESTRING, \
         POLYGON, MULTIPOINT, MULTILINESTRING, MULTIPOLYGON, GEOMETRYCOLLECTION) and writes any \
         dimension tag as a separate token, so POINTZ is a keyword rather than POINT with a Z",
        preview_word(cursor, at, word)
    ))
}

/// The refusal for a word after the keyword that is neither a tag nor `EMPTY`.
#[cold]
#[inline(never)]
fn bad_tag(kind: GeometryKind, at: usize, word: &str) -> GeoError {
    GeoError::literal(format!(
        "expected a dimension tag (Z, M or ZM), EMPTY, or '(' after {} at byte {at}, found {word:?}",
        kind.wkt_keyword()
    ))
}

/// The refusal for a word after a dimension tag that is not `EMPTY`.
#[cold]
#[inline(never)]
fn bad_tag_follower(tag: &str, at: usize, word: &str) -> GeoError {
    GeoError::literal(format!(
        "expected EMPTY or '(' after the {tag:?} dimension tag at byte {at}, found {word:?}"
    ))
}

/// The kind a keyword names, case-insensitively.
fn keyword_kind(word: &str) -> Option<GeometryKind> {
    ALL_KINDS
        .into_iter()
        .find(|kind| word.eq_ignore_ascii_case(kind.wkt_keyword()))
}

/// The dimension a tag names, case-insensitively. [`CoordDim::Xy`] has no tag and
/// so is never returned.
fn tag_dim(word: &str) -> Option<CoordDim> {
    TAGGED_DIMS
        .into_iter()
        .find(|dim| word.eq_ignore_ascii_case(dim.wkt_tag()))
}

/// What to show for a keyword that did not match: the word itself when there was
/// one, and the offending text when the read produced nothing at all.
fn preview_word(cursor: &Cursor<'_>, at: usize, word: &str) -> String {
    if word.is_empty() {
        let mut probe = cursor.clone();
        probe.pos = at;
        probe.preview()
    } else {
        format!("{word:?}")
    }
}

/// Read the parenthesized body of `kind`, the cursor sitting before its `(`.
///
/// # Why this is a bare dispatcher and every arm is its own uninlined function
///
/// This function sits on the `GEOMETRYCOLLECTION` recursion, so its frame is
/// multiplied by the nesting depth — and a [`GeometryBody`] is not a small value.
/// A [`CoordSeq`] is a `SmallVec<[Coord; 4]>` and a [`Coord`] is four exact
/// rationals, each of which carries an inline limb buffer, so one `GeometryBody`
/// is on the order of a kilobyte and a half. At `opt-level = 0` there is no stack
/// slot reuse between match arms, so writing the arms inline with `?` would give
/// this frame **seven** such temporaries — about eleven kilobytes per nesting
/// level, which is what made a sixty-four-deep literal abort on a two-megabyte
/// thread stack instead of parsing.
///
/// Two properties fix that, and both are load-bearing rather than cosmetic:
///
/// * Every arm is a **tail call** with no `?`, so each arm's result is written
///   straight into this function's return place and there is one such temporary
///   rather than seven.
/// * Every arm's callee is `#[inline(never)]`, so the six leaf bodies' locals
///   cannot be hoisted into this frame. Their frames are large, but they are
///   *leaves* — they pop before the recursion continues — and only
///   `parse_collection_body` is on the recursive path.
///
/// The regression test is `nesting_is_capped_but_the_depth_just_below_the_cap_still_parses`,
/// which parses at the cap on an ordinary test thread.
fn parse_body(
    cursor: &mut Cursor<'_>,
    kind: GeometryKind,
    dim: CoordDim,
    depth: usize,
) -> Result<GeometryBody, GeoError> {
    match kind {
        GeometryKind::Point => parse_point_body(cursor, dim),
        GeometryKind::LineString => parse_line_string_body(cursor, dim),
        GeometryKind::Polygon => parse_polygon_body(cursor, dim),
        GeometryKind::MultiPoint => parse_multipoint_body(cursor, dim),
        GeometryKind::MultiLineString => parse_multi_line_string_body(cursor, dim),
        GeometryKind::MultiPolygon => parse_multipolygon_body(cursor, dim),
        GeometryKind::GeometryCollection => parse_collection_body(cursor, dim, depth),
    }
}

/// `POINT` — `( position )`.
#[inline(never)]
fn parse_point_body(cursor: &mut Cursor<'_>, dim: CoordDim) -> Result<GeometryBody, GeoError> {
    open_list(cursor, "a point")?;
    let coord = parse_coord(cursor, dim)?;
    cursor.expect(b')', "a point")?;
    Ok(GeometryBody::Point(Some(coord)))
}

/// `LINESTRING` — `( position, ... )`.
#[inline(never)]
fn parse_line_string_body(
    cursor: &mut Cursor<'_>,
    dim: CoordDim,
) -> Result<GeometryBody, GeoError> {
    Ok(GeometryBody::LineString(parse_coord_seq(
        cursor,
        dim,
        "a line string",
    )?))
}

/// `POLYGON` — `( ring, ... )`, a ring being a `<linestring text>`.
#[inline(never)]
fn parse_polygon_body(cursor: &mut Cursor<'_>, dim: CoordDim) -> Result<GeometryBody, GeoError> {
    Ok(GeometryBody::Polygon(parse_comma_list(
        cursor,
        "a polygon",
        |item| parse_line_text(item, dim),
    )?))
}

/// `MULTIPOINT` — `( member, ... )` in either admitted spelling.
#[inline(never)]
fn parse_multipoint_body(cursor: &mut Cursor<'_>, dim: CoordDim) -> Result<GeometryBody, GeoError> {
    Ok(GeometryBody::MultiPoint(parse_comma_list(
        cursor,
        "a multipoint",
        |item| parse_multipoint_member(item, dim),
    )?))
}

/// `MULTILINESTRING` — `( <linestring text>, ... )`.
#[inline(never)]
fn parse_multi_line_string_body(
    cursor: &mut Cursor<'_>,
    dim: CoordDim,
) -> Result<GeometryBody, GeoError> {
    Ok(GeometryBody::MultiLineString(parse_comma_list(
        cursor,
        "a multi line string",
        |item| parse_line_text(item, dim),
    )?))
}

/// `MULTIPOLYGON` — `( <polygon text>, ... )`.
#[inline(never)]
fn parse_multipolygon_body(
    cursor: &mut Cursor<'_>,
    dim: CoordDim,
) -> Result<GeometryBody, GeoError> {
    Ok(GeometryBody::MultiPolygon(parse_comma_list(
        cursor,
        "a multipolygon",
        |item| parse_polygon_text(item, dim),
    )?))
}

/// `GEOMETRYCOLLECTION` — `( geometry, ... )`, the one body that recurses.
#[inline(never)]
fn parse_collection_body(
    cursor: &mut Cursor<'_>,
    dim: CoordDim,
    depth: usize,
) -> Result<GeometryBody, GeoError> {
    Ok(GeometryBody::GeometryCollection(parse_comma_list(
        cursor,
        "a geometry collection",
        |item| parse_geometry(item, dim, depth + 1),
    )?))
}

/// Consume the `(` that opens a list and refuse the empty pair.
///
/// `()` is neither `EMPTY` nor a list of anything, and accepting it would require
/// inventing which of the two the writer meant. Refusing is the only reading that
/// does not guess.
fn open_list(cursor: &mut Cursor<'_>, what: &str) -> Result<(), GeoError> {
    cursor.expect(b'(', what)?;
    cursor.skip_whitespace();
    if cursor.peek() == Some(b')') {
        return Err(GeoError::literal(format!(
            "{what} has an empty pair of parentheses at byte {}; a geometry with no members is \
             written EMPTY, and () is neither that nor a list",
            cursor.pos
        )));
    }
    Ok(())
}

/// Read `( item, item, ... )` with at least one item.
fn parse_comma_list<'a, T>(
    cursor: &mut Cursor<'a>,
    what: &str,
    mut item: impl FnMut(&mut Cursor<'a>) -> Result<T, GeoError>,
) -> Result<Vec<T>, GeoError> {
    open_list(cursor, what)?;
    let mut items = Vec::new();
    loop {
        items.push(item(cursor)?);
        cursor.skip_whitespace();
        if cursor.eat(b',') {
            continue;
        }
        cursor.expect(b')', what)?;
        return Ok(items);
    }
}

/// Read `( position, position, ... )` into the inline-capacity sequence the model
/// stores.
///
/// Deliberately not `parse_comma_list` specialised to [`Coord`]: this is the
/// per-row hot path, and routing it through a `Vec` would copy every ring once
/// more on its way into the inline-capacity `SmallVec` behind [`CoordSeq`].
fn parse_coord_seq(
    cursor: &mut Cursor<'_>,
    dim: CoordDim,
    what: &str,
) -> Result<CoordSeq, GeoError> {
    open_list(cursor, what)?;
    let mut coords = CoordSeq::new();
    loop {
        coords.push(parse_coord(cursor, dim)?);
        cursor.skip_whitespace();
        if cursor.eat(b',') {
            continue;
        }
        cursor.expect(b')', what)?;
        return Ok(coords);
    }
}

/// Read a `<linestring text>`: `EMPTY`, or a parenthesized position list.
///
/// This one production is a polygon ring, a `MULTILINESTRING` member and a
/// `LINESTRING` body, exactly as the OGC grammar has it.
fn parse_line_text(cursor: &mut Cursor<'_>, dim: CoordDim) -> Result<CoordSeq, GeoError> {
    if cursor.eat_keyword("EMPTY") {
        return Ok(CoordSeq::new());
    }
    parse_coord_seq(cursor, dim, "a position list")
}

/// Read a `<polygon text>`: `EMPTY`, or a parenthesized list of rings.
fn parse_polygon_text(cursor: &mut Cursor<'_>, dim: CoordDim) -> Result<Rings, GeoError> {
    if cursor.eat_keyword("EMPTY") {
        return Ok(Rings::new());
    }
    parse_comma_list(cursor, "a polygon", |item| parse_line_text(item, dim))
}

/// Read one `MULTIPOINT` member in either spelling.
///
/// OGC 1.2.1 admits `MULTIPOINT(1 1,2 2)` and `MULTIPOINT((1 1),(2 2))`, and both
/// occur in real data. The two are distinguished by a single byte of lookahead, so
/// accepting both costs nothing and refusing either would reject a conforming
/// literal. The spellings are decided per member rather than once for the whole
/// list: a mixed list is not something OGC writes, but its meaning is unambiguous,
/// and there is no reading of it worth refusing.
fn parse_multipoint_member(
    cursor: &mut Cursor<'_>,
    dim: CoordDim,
) -> Result<Option<Coord>, GeoError> {
    if cursor.eat_keyword("EMPTY") {
        return Ok(None);
    }
    cursor.skip_whitespace();
    if cursor.peek() == Some(b'(') {
        open_list(cursor, "a multipoint member")?;
        let coord = parse_coord(cursor, dim)?;
        cursor.expect(b')', "a multipoint member")?;
        return Ok(Some(coord));
    }
    parse_coord(cursor, dim).map(Some)
}

/// Read one position: exactly as many ordinates as `dim` declares, no more.
///
/// The count is checked in both directions. Too few is caught by the read of the
/// missing ordinate; too many by looking at what follows, since a position is
/// always followed by `,` or `)` and never by another number.
fn parse_coord(cursor: &mut Cursor<'_>, dim: CoordDim) -> Result<Coord, GeoError> {
    let x = read_ordinate(cursor, dim, 1)?;
    let y = read_ordinate(cursor, dim, 2)?;
    let z = if dim.has_z() {
        Some(read_ordinate(cursor, dim, 3)?)
    } else {
        None
    };
    let m = if dim.has_m() {
        Some(read_ordinate(cursor, dim, dim.ordinates())?)
    } else {
        None
    };
    cursor.skip_whitespace();
    if cursor.peek().is_some_and(is_number_byte) {
        return Err(GeoError::literal(format!(
            "a {} position has {} ordinates, but another number follows the last one at byte {}; \
             the dimension is written once in the tag and governs every position",
            dim_label(dim),
            dim.ordinates(),
            cursor.pos
        )));
    }
    Ok(Coord::new(x, y, z, m))
}

/// Read the `index`-th ordinate of a position, naming the dimension when it is
/// missing so the diagnostic says *why* one more was expected.
fn read_ordinate(cursor: &mut Cursor<'_>, dim: CoordDim, index: usize) -> Result<Rat, GeoError> {
    cursor.skip_whitespace();
    cursor.number().map_err(|err| {
        GeoError::literal(format!(
            "{} — this is ordinate {index} of the {} a {} position carries",
            err.detail(),
            dim.ordinates(),
            dim_label(dim)
        ))
    })
}

/// The dimension's name as diagnostics spell it.
const fn dim_label(dim: CoordDim) -> &'static str {
    match dim {
        CoordDim::Xy => "XY",
        CoordDim::Xyz => "XYZ",
        CoordDim::Xym => "XYM",
        CoordDim::Xyzm => "XYZM",
    }
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// Append the tagged text of `geometry`.
fn write_geometry(out: &mut String, geometry: &Geometry, scale: u32) {
    out.push_str(geometry.kind().wkt_keyword());
    let tag = geometry.dim().wkt_tag();
    let empty = has_no_members(geometry.body());
    if tag.is_empty() {
        // Without a tag there is no separator yet, and `POINTEMPTY` would not
        // parse; `POINT(` needs none and canonical WKT does not write one.
        if empty {
            out.push(' ');
        }
    } else {
        out.push(' ');
        out.push_str(tag);
        out.push(' ');
    }
    if empty {
        out.push_str("EMPTY");
        return;
    }
    write_body(out, geometry.body(), scale);
}

/// Whether the body holds no members at all — the condition for writing `EMPTY`.
///
/// Structural, not semantic: `MULTIPOINT(EMPTY,EMPTY)` denotes the empty set but
/// holds two members, and writing it as `MULTIPOINT EMPTY` would round-trip to a
/// different geometry.
fn has_no_members(body: &GeometryBody) -> bool {
    match body {
        GeometryBody::Point(point) => point.is_none(),
        GeometryBody::LineString(coords) => coords.is_empty(),
        GeometryBody::Polygon(rings) => rings.is_empty(),
        GeometryBody::MultiPoint(points) => points.is_empty(),
        GeometryBody::MultiLineString(lines) => lines.is_empty(),
        GeometryBody::MultiPolygon(polygons) => polygons.is_empty(),
        GeometryBody::GeometryCollection(members) => members.is_empty(),
    }
}

/// Append the parenthesized part of a non-empty body.
fn write_body(out: &mut String, body: &GeometryBody, scale: u32) {
    match body {
        // The `None` arm is unreachable — `has_no_members` already routed the
        // empty point to `EMPTY` — but it writes `EMPTY` rather than `()` so that
        // no reachable-by-refactor path can emit bytes the reader would refuse.
        GeometryBody::Point(point) => match point {
            Some(coord) => {
                out.push('(');
                write_coord(out, coord, scale);
                out.push(')');
            }
            None => out.push_str("EMPTY"),
        },
        GeometryBody::LineString(coords) => write_coord_seq(out, coords, scale),
        GeometryBody::Polygon(rings) => write_rings(out, rings, scale),
        GeometryBody::MultiPoint(points) => {
            write_separated(out, points, |buf, point| match point {
                Some(coord) => {
                    buf.push('(');
                    write_coord(buf, coord, scale);
                    buf.push(')');
                }
                None => buf.push_str("EMPTY"),
            });
        }
        GeometryBody::MultiLineString(lines) => {
            write_separated(out, lines, |buf, line| write_line_text(buf, line, scale));
        }
        GeometryBody::MultiPolygon(polygons) => {
            write_separated(out, polygons, |buf, rings| {
                if rings.is_empty() {
                    buf.push_str("EMPTY");
                } else {
                    write_rings(buf, rings, scale);
                }
            });
        }
        GeometryBody::GeometryCollection(members) => {
            write_separated(out, members, |buf, member| {
                write_geometry(buf, member, scale);
            });
        }
    }
}

/// Append `(item,item,...)` — the one place the member separator is decided.
fn write_separated<T>(out: &mut String, items: &[T], mut each: impl FnMut(&mut String, &T)) {
    out.push('(');
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        each(out, item);
    }
    out.push(')');
}

/// Append a `<linestring text>`: `EMPTY` for a member with no positions.
fn write_line_text(out: &mut String, coords: &CoordSeq, scale: u32) {
    if coords.is_empty() {
        out.push_str("EMPTY");
    } else {
        write_coord_seq(out, coords, scale);
    }
}

/// Append `(position,position,...)`.
fn write_coord_seq(out: &mut String, coords: &[Coord], scale: u32) {
    write_separated(out, coords, |buf, coord| write_coord(buf, coord, scale));
}

/// Append `(ring,ring,...)`.
fn write_rings(out: &mut String, rings: &[CoordSeq], scale: u32) {
    write_separated(out, rings, |buf, ring| write_line_text(buf, ring, scale));
}

/// Append one position: its ordinates in `x y z m` order, one space apart.
fn write_coord(out: &mut String, coord: &Coord, scale: u32) {
    out.push_str(&coord.x().to_decimal_string(scale));
    out.push(' ');
    out.push_str(&coord.y().to_decimal_string(scale));
    if let Some(z) = coord.z() {
        out.push(' ');
        out.push_str(&z.to_decimal_string(scale));
    }
    if let Some(m) = coord.m() {
        out.push(' ');
        out.push_str(&m.to_decimal_string(scale));
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_NESTING_DEPTH, parse, write, write_bare};
    use crate::error::GeoError;
    use crate::exact::Rat;
    use crate::geom::{
        Coord, CoordDim, Crs, Geometry, GeometryBody, GeometryKind, GeometryLiteral,
    };

    /// A scale wide enough that no fixture in this module is rounded, so a
    /// round-trip failure means the codec disagreed rather than that the caller
    /// asked for too few digits.
    const SCALE: u32 = 40;

    fn crs() -> Crs {
        Crs::new("http://example.org/crs/planar").expect("a non-empty IRI")
    }

    fn read(text: &str) -> Result<GeometryLiteral, GeoError> {
        parse(text, &crs())
    }

    fn good(text: &str) -> GeometryLiteral {
        read(text).unwrap_or_else(|err| panic!("{text:?} must parse, but was refused: {err}"))
    }

    fn geometry_of(text: &str) -> Geometry {
        good(text).into_geometry()
    }

    fn refused(text: &str) -> GeoError {
        match read(text) {
            Ok(literal) => panic!(
                "{text:?} must be refused, but parsed as {}",
                write_bare(literal.geometry(), SCALE)
            ),
            Err(err) => err,
        }
    }

    /// Assert that `text` is refused as a malformed literal (never as a config or
    /// domain error, which would mean the blame landed on the wrong party).
    fn assert_refused(text: &str) {
        let err = refused(text);
        assert!(
            matches!(err, GeoError::Literal(_)),
            "{text:?} must be refused as a malformed LITERAL, but was refused as {err:?}"
        );
    }

    fn bare(text: &str) -> String {
        write_bare(good(text).geometry(), SCALE)
    }

    // -----------------------------------------------------------------------
    // Refusals, each paired with the neighbouring valid case
    // -----------------------------------------------------------------------

    /// An EMPTY lexical form is the empty geometry, not a refusal.
    ///
    /// OGC 22-047r1 `/req/geometry-extension/wkt-literal-empty`: "An empty RDFS
    /// Literal of type `geo:wktLiteral` shall be interpreted as an empty
    /// Geometry." Refusing it was an over-refusal with unusually wide blast
    /// radius, because a malformed literal is an *evaluation error*: one empty
    /// `geo:asWKT` object anywhere in a dataset aborted every `geof:` query that
    /// touched it, rather than contributing no rows.
    ///
    /// The neighbouring cases are pinned in both directions: whitespace-only
    /// forms are also empty (the shipped SHACL shape's pattern is `^\s*$|...`),
    /// while a non-empty lexical form that merely *fails* to name a geometry is
    /// still refused — accepting the empty form must not turn the parser lenient.
    #[test]
    fn an_empty_lexical_form_is_the_empty_geometry_and_a_malformed_one_still_is_not() {
        let crs = Crs::new("http://example.org/crs/planar").expect("a non-empty IRI");
        for empty in ["", " ", "   ", "\t", "\n", " \t\n "] {
            let literal = parse(empty, &crs).unwrap_or_else(|error| {
                panic!("the empty lexical form {empty:?} must parse, got {error}")
            });
            assert!(
                literal.geometry().is_empty(),
                "{empty:?} must denote an empty geometry"
            );
            assert_eq!(
                literal.geometry().kind(),
                GeometryKind::GeometryCollection,
                "{empty:?} must denote the kind-free empty geometry"
            );
            assert_eq!(
                literal.crs(),
                &crs,
                "an empty literal names no system, so it takes the caller's default"
            );
        }
        // The CONTROL: accepting the empty form must not make a merely broken
        // literal acceptable. Each of these is non-empty and still refused.
        for malformed in ["POIN(1 2)", "(1 2)", "<>", "POINT"] {
            assert!(
                parse(malformed, &crs).is_err(),
                "{malformed:?} is not empty and must still be refused"
            );
        }
    }

    /// An unknown keyword is refused, and the keyword one letter away from it in
    /// the same family still parses — the refusal is about the spelling, not about
    /// the shape of the literal.
    #[test]
    fn an_unknown_keyword_is_refused_but_every_real_one_parses() {
        for bad in [
            "CIRCULARSTRING(0 0,1 1,2 2)",
            "COMPOUNDCURVE(LINESTRING(0 0,1 1))",
            "TRIANGLE((0 0,1 0,0 1,0 0))",
            "LINE(0 0,1 1)",
            "POIN(1 2)",
            "(1 2)",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases: all seven keywords the grammar names.
        for text in [
            "POINT(1 2)",
            "LINESTRING(0 0,1 1)",
            "POLYGON((0 0,1 0,0 1,0 0))",
            "MULTIPOINT((1 1))",
            "MULTILINESTRING((0 0,1 1))",
            "MULTIPOLYGON(((0 0,1 0,0 1,0 0)))",
            "GEOMETRYCOLLECTION(POINT(1 2))",
        ] {
            let literal = good(text);
            assert!(
                !literal.geometry().is_empty(),
                "{text:?} must parse to a non-empty geometry"
            );
        }
    }

    /// `POINTZ(...)` fuses the tag onto the keyword and is not OGC; the spaced
    /// spelling one character away is, and so is a tag that abuts the paren, since
    /// a word and a delimiter need no separator.
    #[test]
    fn a_dimension_tag_must_be_its_own_token_but_need_not_be_spaced_from_the_paren() {
        for bad in [
            "POINTZ(1 2 3)",
            "POINTM(1 2 3)",
            "POINTZM(1 2 3 4)",
            "MULTIPOINTZ((1 2 3))",
            "LINESTRINGZ(0 0 0,1 1 1)",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases: the very same literals with the tag made
        // into its own token.
        assert_eq!(
            geometry_of("POINT Z (1 2 3)").dim(),
            CoordDim::Xyz,
            "the spaced tag is the OGC spelling and must parse"
        );
        assert_eq!(
            geometry_of("POINT Z(1 2 3)").dim(),
            CoordDim::Xyz,
            "a tag abutting the paren is still two tokens and must parse"
        );
        assert_eq!(
            geometry_of("POINT M (1 2 3)").dim(),
            CoordDim::Xym,
            "M is a measure, not an elevation"
        );
        assert_eq!(
            geometry_of("POINT ZM (1 2 3 4)").dim(),
            CoordDim::Xyzm,
            "ZM is one tag, not two"
        );
        assert_eq!(
            geometry_of("MULTIPOINT Z ((1 2 3))").dim(),
            CoordDim::Xyz,
            "the tag rule is the same at every keyword"
        );
    }

    /// The ordinate count must match the declared dimension in BOTH directions,
    /// and the correct count for each of the four dimensions still parses.
    #[test]
    fn the_ordinate_count_must_match_the_declared_dimension() {
        for bad in [
            "POINT Z (1 2)",
            "POINT ZM (1 2 3)",
            "POINT M (1 2)",
            "POINT(1 2 3)",
            "POINT Z (1 2 3 4)",
            "LINESTRING Z (0 0 0,1 1)",
            "POLYGON Z ((0 0 0,1 0 0,0 1 0,0 0))",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases: the same literals with the right count.
        for (text, dim, ordinates) in [
            ("POINT(1 2)", CoordDim::Xy, 2),
            ("POINT Z (1 2 3)", CoordDim::Xyz, 3),
            ("POINT M (1 2 3)", CoordDim::Xym, 3),
            ("POINT ZM (1 2 3 4)", CoordDim::Xyzm, 4),
        ] {
            let geometry = geometry_of(text);
            assert_eq!(geometry.dim(), dim, "{text:?} declares {dim:?}");
            assert_eq!(
                geometry.dim().ordinates(),
                ordinates,
                "{text:?} carries {ordinates} ordinates"
            );
        }
        assert!(
            read("LINESTRING Z (0 0 0,1 1 1)").is_ok(),
            "the neighbour of the refused 3D line, with the count fixed, must parse"
        );
    }

    /// A missing or extra bracket is refused; the balanced literal beside it is
    /// not.
    #[test]
    fn unbalanced_parentheses_are_refused_but_balanced_ones_are_not() {
        for bad in [
            "POINT(1 2",
            "POINT 1 2)",
            "POINT((1 2)",
            "POLYGON((0 0,1 0,0 1,0 0)",
            "POLYGON(0 0,1 0,0 1,0 0))",
            "MULTIPOLYGON(((0 0,1 0,0 1,0 0))",
            "GEOMETRYCOLLECTION(POINT(1 2)",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases.
        for text in [
            "POINT(1 2)",
            "POLYGON((0 0,1 0,0 1,0 0))",
            "MULTIPOLYGON(((0 0,1 0,0 1,0 0)))",
            "GEOMETRYCOLLECTION(POINT(1 2))",
        ] {
            assert!(read(text).is_ok(), "{text:?} is balanced and must parse");
        }
    }

    /// Text after the geometry is refused; whitespace after it is NOT — trailing
    /// whitespace is legal in a lexical form and refusing it would reject data
    /// nobody would think to look at.
    #[test]
    fn trailing_text_is_refused_but_trailing_whitespace_is_not() {
        for bad in [
            "POINT(1 2) X",
            "POINT(1 2) POINT(3 4)",
            "POINT(1 2),",
            "POINT(1 2))",
            "POINT(1 2) <http://example.org/crs/planar>",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases.
        for text in [
            "POINT(1 2)   ",
            "POINT(1 2)\n",
            "POINT(1 2)\t\r\n  ",
            "   POINT(1 2)   ",
        ] {
            assert_eq!(
                bare(text),
                "POINT(1 2)",
                "{text:?} differs from the canonical form only in insignificant whitespace"
            );
        }
    }

    /// An unclosed ring is refused (by the model's structural rule); the closed
    /// ring one position away is accepted.
    #[test]
    fn an_unclosed_ring_is_refused_but_a_closed_one_is_not() {
        for bad in [
            "POLYGON((0 0,1 0,0 1,2 2))",
            "POLYGON((0 0,1 0,0 0))",
            "MULTIPOLYGON(((0 0,1 0,0 1,9 9)))",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases: the same rings, closed.
        assert!(
            read("POLYGON((0 0,1 0,0 1,0 0))").is_ok(),
            "a closed triangle is the smallest ring"
        );
        assert!(
            read("MULTIPOLYGON(((0 0,1 0,0 1,0 0)))").is_ok(),
            "the rule is the same inside a multipolygon"
        );
        assert!(
            read("POLYGON((0 0,4 0,4 4,0 4,0 0),(1 1,2 1,2 2,1 1))").is_ok(),
            "interior rings are checked the same way and this one closes"
        );
        assert!(
            read("POLYGON Z ((0 0 0,1 0 1,0 1 2,0 0 9))").is_ok(),
            "a ring closes in the PLANE, so differing elevations at the endpoints are fine"
        );
    }

    /// `NaN` and `INF` are not coordinates; a very large finite exponent one
    /// character away is, and must not be swept up by the same refusal.
    #[test]
    fn nan_and_inf_are_refused_but_large_and_small_exponents_are_not() {
        for bad in [
            "POINT(NaN 1)",
            "POINT(1 NaN)",
            "POINT(INF 1)",
            "POINT(-INF 1)",
            "POINT(1 inf)",
            "POINT(1e 2)",
            "POINT(1 2e)",
            "POINT(. 2)",
            "POINT(- 2)",
            "POINT(1.2.3 2)",
            "POINT(1-2 3)",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases.
        for text in [
            "POINT(1e10 2)",
            "POINT(1E10 2)",
            "POINT(-2.5E-3 2)",
            "POINT(+1.5 2)",
            "POINT(.5 2)",
            "POINT(1e308 2)",
            "POINT(-1e-308 2)",
        ] {
            assert!(
                read(text).is_ok(),
                "{text:?} is a finite decimal and must parse"
            );
        }
        assert_eq!(
            geometry_of("POINT(1e10 2)")
                .coords()
                .next()
                .expect("one position")
                .x(),
            &Rat::parse_decimal("10000000000").expect("an exact decimal"),
            "an exponent is expanded exactly, not approximated"
        );
    }

    /// An empty CRS prefix names no system and is refused; a real one beside it is
    /// carried verbatim.
    #[test]
    fn an_empty_crs_prefix_is_refused_but_a_real_one_is_carried_verbatim() {
        for bad in ["<> POINT(1 2)", "<>POINT(1 2)", "   <> POINT(1 2)"] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases.
        for iri in [
            "http://example.org/crs/planar",
            "http://example.org/crs/other",
            "urn:example:crs:1",
            "http://example.org/crs/wíth-non-ascii",
        ] {
            let text = format!("<{iri}> POINT(1 2)");
            let literal = good(&text);
            assert_eq!(
                literal.crs().as_str(),
                iri,
                "the prefix IRI is carried byte for byte"
            );
        }
    }

    /// A prefix with no geometry after it is refused; the same prefix with a
    /// geometry is not.
    #[test]
    fn a_crs_prefix_needs_a_geometry_after_it() {
        for bad in [
            "<http://example.org/crs/planar>",
            "<http://example.org/crs/planar>   ",
            "<http://example.org/crs/planar",
            "<http://example.org/crs/planar> EMPTY",
            "<http://example.org/crs/planar> <http://example.org/crs/planar> POINT(1 2)",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases, including the spacing variants.
        for text in [
            "<http://example.org/crs/planar> POINT(1 2)",
            "<http://example.org/crs/planar>POINT(1 2)",
            "<http://example.org/crs/planar>\n\tPOINT(1 2)",
        ] {
            assert_eq!(
                bare(text),
                "POINT(1 2)",
                "{text:?} carries a prefix and a geometry"
            );
        }
    }

    /// A one-position line string is refused; two positions is the smallest curve
    /// and must parse.
    #[test]
    fn a_one_position_line_is_refused_but_two_positions_are_not() {
        for bad in [
            "LINESTRING(1 1)",
            "MULTILINESTRING((1 1))",
            "MULTILINESTRING((0 0,1 1),(2 2))",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases.
        assert!(
            read("LINESTRING(1 1,2 2)").is_ok(),
            "two positions is a curve"
        );
        assert!(
            read("MULTILINESTRING((1 1,2 2))").is_ok(),
            "the rule is the same inside a multi line string"
        );
        assert!(
            read("LINESTRING EMPTY").is_ok(),
            "zero positions is the empty line, which is not the refused case"
        );
        assert!(
            read("MULTILINESTRING(EMPTY)").is_ok(),
            "an empty member is a linestring text and is admitted by the grammar"
        );
    }

    /// `()` is neither `EMPTY` nor a member list, so it is refused at every level;
    /// the `EMPTY` keyword right beside it parses at every level.
    #[test]
    fn empty_parentheses_are_refused_but_the_empty_keyword_is_not() {
        for bad in [
            "MULTIPOINT()",
            "POINT()",
            "LINESTRING()",
            "POLYGON()",
            "POLYGON(())",
            "MULTILINESTRING()",
            "MULTIPOLYGON()",
            "GEOMETRYCOLLECTION()",
            "MULTIPOINT(  )",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases: EMPTY at every kind, tagged and untagged.
        for text in [
            "POINT EMPTY",
            "LINESTRING EMPTY",
            "POLYGON EMPTY",
            "MULTIPOINT EMPTY",
            "MULTILINESTRING EMPTY",
            "MULTIPOLYGON EMPTY",
            "GEOMETRYCOLLECTION EMPTY",
            "POINT Z EMPTY",
            "POINT M EMPTY",
            "POINT ZM EMPTY",
            "POLYGON ZM EMPTY",
        ] {
            let geometry = geometry_of(text);
            assert!(geometry.is_empty(), "{text:?} is the empty geometry");
            assert_eq!(geometry.coord_count(), 0, "{text:?} carries no positions");
        }
        // And EMPTY as a MEMBER, which is a different production from the whole
        // geometry being empty.
        for text in [
            "MULTIPOINT(EMPTY)",
            "MULTIPOINT(EMPTY,EMPTY)",
            "MULTILINESTRING(EMPTY)",
            "MULTIPOLYGON(EMPTY)",
            "GEOMETRYCOLLECTION(POINT EMPTY)",
        ] {
            assert!(
                read(text).is_ok(),
                "{text:?} writes EMPTY where the grammar admits a member"
            );
        }
    }

    /// Whitespace before the CRS prefix is legal and must not be refused; so is
    /// whitespace essentially anywhere else between tokens.
    #[test]
    fn whitespace_between_tokens_is_insignificant_everywhere() {
        for text in [
            "  <http://example.org/crs/planar> POINT(1 2)",
            "\n\t<http://example.org/crs/planar>POINT(1 2)",
            "POINT ( 1 2 )",
            "POINT(1\t2)",
            "LINESTRING ( 0 0 , 1 1 )",
            "POLYGON ( ( 0 0 , 1 0 , 0 1 , 0 0 ) )",
            "MULTIPOINT ( ( 1 1 ) , ( 2 2 ) )",
            "GEOMETRYCOLLECTION ( POINT ( 1 2 ) )",
            "POINT\nZ\n(1 2 3)",
        ] {
            assert!(
                read(text).is_ok(),
                "{text:?} differs from a canonical literal only in whitespace"
            );
        }
        assert_eq!(
            bare("LINESTRING ( 0 0 , 1 1 )"),
            "LINESTRING(0 0,1 1)",
            "whitespace is discarded, not carried into the rendering"
        );
        // The control: whitespace INSIDE a number is not insignificant.
        for bad in ["POINT(1 . 5 2)", "POINT(1 2 . 5)", "POINT(- 1 2)"] {
            assert_refused(bad);
        }
    }

    /// Keywords, dimension tags and `EMPTY` are case-insensitive on input, and the
    /// rendering is uppercase regardless.
    #[test]
    fn keywords_tags_and_empty_are_case_insensitive_on_input() {
        for (text, expected) in [
            ("point(1 2)", "POINT(1 2)"),
            ("Point(1 2)", "POINT(1 2)"),
            ("pOiNt(1 2)", "POINT(1 2)"),
            ("point z (1 2 3)", "POINT Z (1 2 3)"),
            ("POINT zm (1 2 3 4)", "POINT ZM (1 2 3 4)"),
            ("point empty", "POINT EMPTY"),
            ("MultiPoint Empty", "MULTIPOINT EMPTY"),
            ("multipoint(empty)", "MULTIPOINT(EMPTY)"),
            (
                "geometrycollection(linestring(0 0,1 1))",
                "GEOMETRYCOLLECTION(LINESTRING(0 0,1 1))",
            ),
        ] {
            assert_eq!(
                bare(text),
                expected,
                "{text:?} must parse and render as {expected:?}"
            );
        }
        // The control: case-insensitivity is not a licence to accept a keyword
        // that does not exist in any case.
        assert_refused("pointz(1 2 3)");
        assert_refused("circularstring(0 0,1 1,2 2)");
    }

    // -----------------------------------------------------------------------
    // MULTIPOINT's two spellings
    // -----------------------------------------------------------------------

    /// Both OGC-admitted `MULTIPOINT` spellings are accepted and mean the same
    /// geometry; the parenthesized one is what gets emitted.
    #[test]
    fn both_multipoint_spellings_are_accepted_and_the_parenthesized_one_is_emitted() {
        let bare_form = geometry_of("MULTIPOINT(1 1,2 2)");
        let paren_form = geometry_of("MULTIPOINT((1 1),(2 2))");
        assert_eq!(
            bare_form, paren_form,
            "the two spellings denote the identical geometry"
        );
        assert_eq!(
            write_bare(&bare_form, SCALE),
            "MULTIPOINT((1 1),(2 2))",
            "the parenthesized spelling is the one emitted"
        );
        // Both spellings carry the dimension tag the same way, and a mixed list is
        // unambiguous rather than something worth refusing.
        assert_eq!(
            bare("MULTIPOINT Z (1 1 1,2 2 2)"),
            "MULTIPOINT Z ((1 1 1),(2 2 2))",
            "the bare spelling works under a tag too"
        );
        assert_eq!(
            bare("MULTIPOINT(1 1,(2 2),EMPTY)"),
            "MULTIPOINT((1 1),(2 2),EMPTY)",
            "a mixed list has one reading and it is not worth a refusal"
        );
        // The control: a member with the wrong ordinate count is still refused in
        // either spelling.
        assert_refused("MULTIPOINT Z (1 1,2 2 2)");
        assert_refused("MULTIPOINT Z ((1 1),(2 2 2))");
    }

    // -----------------------------------------------------------------------
    // Exactness and determinism
    // -----------------------------------------------------------------------

    /// The proof no float was involved: the parsed ordinate is bit-identical to
    /// what the exact decimal reader produces from the same text, which `0.1` and
    /// `0.2` could not be if they had passed through an `f64`.
    #[test]
    fn coordinates_are_exactly_the_decimals_that_were_written() {
        let geometry = geometry_of("POINT(0.1 0.2)");
        let coord = geometry.coords().next().expect("one position");
        assert_eq!(
            coord.x(),
            &Rat::parse_decimal("0.1").expect("an exact decimal"),
            "0.1 is stored exactly; an f64 round trip would not be"
        );
        assert_eq!(
            coord.y(),
            &Rat::parse_decimal("0.2").expect("an exact decimal"),
            "0.2 is stored exactly"
        );
        // The control: the two ordinates are genuinely different values, so the
        // assertions above are not both passing against some shared default.
        assert_ne!(coord.x(), coord.y(), "0.1 and 0.2 are different numbers");
    }

    /// Determinism: three spellings of the same number produce the IDENTICAL
    /// geometry, which is what makes a rendering a pure function of the value
    /// rather than of the text it arrived in.
    #[test]
    fn three_spellings_of_one_number_produce_the_identical_geometry() {
        let spellings = [
            "POINT(1.5 0)",
            "POINT(1.50 0)",
            "POINT(15e-1 0)",
            "POINT(1.5000000000 0)",
            "POINT(0.15E1 0)",
            "POINT(+1.5 0)",
        ];
        let first = geometry_of(spellings[0]);
        for text in spellings {
            let geometry = geometry_of(text);
            assert_eq!(
                geometry, first,
                "{text:?} is the same number as {:?} and must be the same geometry",
                spellings[0]
            );
            assert_eq!(
                write_bare(&geometry, SCALE),
                "POINT(1.5 0)",
                "{text:?} renders canonically, not as it was written"
            );
        }
        // The control: a genuinely different number must NOT collapse into these.
        assert_ne!(
            geometry_of("POINT(1.51 0)"),
            first,
            "1.51 is not 1.5, so the equalities above are about the value"
        );
        // Signed zero has no meaning in an exact rational.
        assert_eq!(
            bare("POINT(-0 0.0)"),
            "POINT(0 0)",
            "zero has one exact representation"
        );
    }

    /// Over-refusal control: a sixty-significant-digit ordinate is a perfectly
    /// good exact decimal and must not be refused for being long, nor rounded on
    /// the way in.
    #[test]
    fn a_sixty_digit_coordinate_parses_exactly_and_is_not_refused() {
        // Sixty fraction digits, the last of them non-zero: a trailing zero is not
        // part of the value, and the renderer suppresses it, so ending on one
        // would have tested the fixture rather than the parse.
        let long = "0.123456789012345678901234567890123456789012345678901234567891";
        let text = format!("POINT({long} 1)");
        let geometry = geometry_of(&text);
        let coord = geometry.coords().next().expect("one position");
        assert_eq!(
            coord.x(),
            &Rat::parse_decimal(long).expect("an exact decimal"),
            "sixty digits survive the parse unrounded"
        );
        assert_eq!(
            write_bare(&geometry, 60),
            format!("POINT({long} 1)"),
            "and are rendered back in full when the caller asks for the scale"
        );
        // A scale too small ROUNDS; it does not fail. That is the documented
        // contract for `coordinate_scale`, and it is the caller's choice.
        let narrow = write_bare(&geometry, 2);
        assert!(
            narrow.starts_with("POINT(0.1"),
            "a narrow scale rounds rather than refusing: {narrow}"
        );
        // A long INTEGER part is equally fine, and its trailing zero IS part of
        // the value, so it survives the rendering.
        let huge = "123456789012345678901234567890123456789012345678901234567890";
        assert_eq!(
            bare(&format!("POINT({huge} 1)")),
            format!("POINT({huge} 1)"),
            "a sixty-digit integer ordinate is neither refused nor truncated"
        );
        // A trailing zero in the FRACTION carries no value, so it is not
        // reproduced — `0.50` and `0.5` are the same number and render the same.
        assert_eq!(
            bare("POINT(0.50 1)"),
            "POINT(0.5 1)",
            "a trailing fraction zero is not part of the value"
        );
    }

    // -----------------------------------------------------------------------
    // The CRS
    // -----------------------------------------------------------------------

    /// The default is used only when the literal omits a prefix, and a present
    /// prefix wins over it — this crate fabricates no system and overrides none.
    #[test]
    fn the_default_crs_is_used_only_when_the_literal_omits_one() {
        let default = crs();
        let other = Crs::new("http://example.org/crs/other").expect("a non-empty IRI");

        let omitted = parse("POINT(1 2)", &default).expect("well formed");
        assert_eq!(
            omitted.crs(),
            &default,
            "an omitted prefix means the caller's default"
        );

        let named =
            parse("<http://example.org/crs/other> POINT(1 2)", &default).expect("well formed");
        assert_eq!(
            named.crs(),
            &other,
            "a named prefix is used verbatim and the default is not consulted"
        );
        assert_ne!(
            named.crs(),
            &default,
            "the prefix genuinely overrode the default"
        );

        // The same literal under two different defaults yields two different
        // literals, which is what makes the parameter load-bearing.
        let under_other = parse("POINT(1 2)", &other).expect("well formed");
        assert_ne!(
            omitted, under_other,
            "the default is carried into the result, not discarded"
        );
        assert_eq!(
            omitted.geometry(),
            under_other.geometry(),
            "and it changes nothing about the geometry itself"
        );
    }

    /// A collection's members share the collection's system: they carry no prefix
    /// of their own, and one written there is refused.
    #[test]
    fn a_collection_member_may_not_carry_its_own_crs_prefix() {
        assert_refused("GEOMETRYCOLLECTION(<http://example.org/crs/other> POINT(1 2))");
        // The neighbouring VALID case: the prefix belongs on the literal.
        assert!(
            read("<http://example.org/crs/other> GEOMETRYCOLLECTION(POINT(1 2))").is_ok(),
            "the collection as a whole may carry a prefix"
        );
    }

    // -----------------------------------------------------------------------
    // GEOMETRYCOLLECTION
    // -----------------------------------------------------------------------

    /// A member whose explicit tag disagrees with the collection is refused; the
    /// members that agree with it — explicitly or by saying nothing — parse.
    #[test]
    fn a_collection_member_may_not_contradict_the_collections_dimension() {
        for bad in [
            "GEOMETRYCOLLECTION Z (POINT M (1 2 3))",
            "GEOMETRYCOLLECTION Z (POINT ZM (1 2 3 4))",
            "GEOMETRYCOLLECTION(POINT Z (1 2 3))",
            "GEOMETRYCOLLECTION Z (POINT Z (1 2 3),POINT M (4 5 6))",
        ] {
            assert_refused(bad);
        }
        // The neighbouring VALID cases: every member carrying the SAME tag as the
        // collection.
        assert_eq!(
            bare("GEOMETRYCOLLECTION Z (POINT Z (1 2 3),LINESTRING Z (0 0 0,1 1 1))"),
            "GEOMETRYCOLLECTION Z (POINT Z (1 2 3),LINESTRING Z (0 0 0,1 1 1))",
            "members that agree with the collection parse and round-trip"
        );
        // And a member that writes NO tag inherits the collection's, because
        // silence is not disagreement.
        assert_eq!(
            bare("GEOMETRYCOLLECTION Z (POINT(1 2 3))"),
            "GEOMETRYCOLLECTION Z (POINT Z (1 2 3))",
            "an untagged member inherits the collection's dimension"
        );
        assert_eq!(
            geometry_of("GEOMETRYCOLLECTION Z (POINT(1 2 3))").dim(),
            CoordDim::Xyz,
            "and the inherited dimension is the collection's"
        );
        // The control: inheritance does not excuse a wrong ordinate count.
        assert_refused("GEOMETRYCOLLECTION Z (POINT(1 2))");
        // An untagged collection of untagged members is unchanged by the rule.
        assert!(
            read("GEOMETRYCOLLECTION(POINT(1 2),POINT(3 4))").is_ok(),
            "the ordinary flat case still parses"
        );
    }

    /// Three levels of nesting is ordinary WKT and must parse; the members are
    /// walked in written order.
    #[test]
    fn a_three_level_collection_parses_and_keeps_its_order() {
        let text = "GEOMETRYCOLLECTION(GEOMETRYCOLLECTION(GEOMETRYCOLLECTION(POINT(1 2),\
                    POINT(3 4)),LINESTRING(0 0,1 1)),POINT(9 9))";
        let geometry = geometry_of(text);
        assert_eq!(
            geometry.kind(),
            GeometryKind::GeometryCollection,
            "the outermost geometry is the collection"
        );
        assert_eq!(
            geometry.coord_count(),
            5,
            "two points, two line positions and the outer point"
        );
        assert_eq!(
            write_bare(&geometry, SCALE),
            text,
            "and the nesting renders back exactly as written"
        );
    }

    /// Nesting is capped so a hostile literal cannot exhaust the stack — but the
    /// depth one below the cap must still parse, or the guard has become the bug
    /// it was meant to prevent.
    #[test]
    fn nesting_is_capped_but_the_depth_just_below_the_cap_still_parses() {
        fn nested(levels: usize) -> String {
            let wrappers = levels - 1;
            let mut text = String::with_capacity(wrappers * 20 + 16);
            for _ in 0..wrappers {
                text.push_str("GEOMETRYCOLLECTION(");
            }
            text.push_str("POINT(1 2)");
            for _ in 0..wrappers {
                text.push(')');
            }
            text
        }

        // The over-refusal control: 63 collections around a point is 64 geometries
        // deep, exactly the cap, and must be accepted.
        let at_the_cap = nested(MAX_NESTING_DEPTH);
        let parsed = good(&at_the_cap);
        let just_below = nested(MAX_NESTING_DEPTH - 1);
        assert!(
            read(&just_below).is_ok(),
            "depth {} must parse",
            MAX_NESTING_DEPTH - 1
        );

        // The cap has to bound EVERY recursion over a geometry, not just the
        // reader's. The writer recurses through the members and so does the
        // model's structural check inside `Geometry::new`, so a depth the reader
        // accepts but the writer cannot render would just move the abort one
        // function along. Rendering and re-reading at the cap exercises both.
        let rendered = write_bare(parsed.geometry(), SCALE);
        assert_eq!(
            rendered, at_the_cap,
            "the geometry at the cap must render back exactly as written"
        );
        assert_eq!(
            good(&rendered).geometry(),
            parsed.geometry(),
            "and must re-read as itself, which walks the structural check at full depth too"
        );
        // One deeper is refused, as a literal error rather than a stack overflow.
        assert_refused(&nested(MAX_NESTING_DEPTH + 1));
        assert_refused(&nested(MAX_NESTING_DEPTH + 200));
    }

    // -----------------------------------------------------------------------
    // Byte-exact serialization goldens
    // -----------------------------------------------------------------------

    /// The rendering is pinned byte for byte, for every kind: uppercase keywords,
    /// `,` with no following space, one space between ordinates, and no space
    /// before `(` unless a dimension tag supplied one.
    #[test]
    fn the_rendering_is_pinned_byte_for_byte_at_every_kind() {
        for (input, expected) in [
            ("POINT(1 2)", "POINT(1 2)"),
            ("POINT ( 1  2 )", "POINT(1 2)"),
            ("POINT(-83.4 42.25)", "POINT(-83.4 42.25)"),
            ("LINESTRING(0 0, 1 1, 2 2)", "LINESTRING(0 0,1 1,2 2)"),
            (
                "POLYGON((0 0, 4 0, 4 4, 0 4, 0 0), (1 1, 2 1, 2 2, 1 1))",
                "POLYGON((0 0,4 0,4 4,0 4,0 0),(1 1,2 1,2 2,1 1))",
            ),
            ("MULTIPOINT(1 1, 2 2)", "MULTIPOINT((1 1),(2 2))"),
            (
                "MULTILINESTRING((0 0, 1 1), EMPTY)",
                "MULTILINESTRING((0 0,1 1),EMPTY)",
            ),
            (
                "MULTIPOLYGON(((0 0, 1 0, 0 1, 0 0)), EMPTY)",
                "MULTIPOLYGON(((0 0,1 0,0 1,0 0)),EMPTY)",
            ),
            (
                "GEOMETRYCOLLECTION(POINT(1 2), LINESTRING(0 0, 1 1))",
                "GEOMETRYCOLLECTION(POINT(1 2),LINESTRING(0 0,1 1))",
            ),
        ] {
            assert_eq!(
                bare(input),
                expected,
                "{input:?} must render as exactly {expected:?}"
            );
        }
    }

    /// The dimension tag is spaced on both sides, and `EMPTY` gets the space a
    /// missing tag would otherwise have supplied.
    #[test]
    fn the_dimension_tag_and_empty_are_spaced_exactly() {
        for (input, expected) in [
            ("POINT Z (1 2 3)", "POINT Z (1 2 3)"),
            ("POINT Z(1 2 3)", "POINT Z (1 2 3)"),
            ("POINT M (1 2 3)", "POINT M (1 2 3)"),
            ("POINT ZM (1 2 3 4)", "POINT ZM (1 2 3 4)"),
            ("POINT EMPTY", "POINT EMPTY"),
            ("POINT Z EMPTY", "POINT Z EMPTY"),
            ("POINT ZM EMPTY", "POINT ZM EMPTY"),
            ("MULTIPOINT EMPTY", "MULTIPOINT EMPTY"),
            ("MULTIPOINT(EMPTY)", "MULTIPOINT(EMPTY)"),
            ("MULTIPOINT(EMPTY, EMPTY)", "MULTIPOINT(EMPTY,EMPTY)"),
            ("MULTIPOLYGON(EMPTY)", "MULTIPOLYGON(EMPTY)"),
            ("MULTILINESTRING(EMPTY)", "MULTILINESTRING(EMPTY)"),
            (
                "GEOMETRYCOLLECTION ZM (POINT ZM (1 2 3 4))",
                "GEOMETRYCOLLECTION ZM (POINT ZM (1 2 3 4))",
            ),
            (
                "LINESTRING ZM (0 0 0 0,1 1 1 1)",
                "LINESTRING ZM (0 0 0 0,1 1 1 1)",
            ),
        ] {
            assert_eq!(
                bare(input),
                expected,
                "{input:?} must render as exactly {expected:?}"
            );
        }
    }

    /// `MULTIPOINT(EMPTY,EMPTY)` denotes the empty set but holds two members;
    /// collapsing it to `MULTIPOINT EMPTY` would change the geometry, so the
    /// writer is structural rather than semantic.
    #[test]
    fn an_empty_set_with_members_is_not_written_as_the_memberless_empty() {
        let with_members = geometry_of("MULTIPOINT(EMPTY,EMPTY)");
        let memberless = geometry_of("MULTIPOINT EMPTY");
        assert!(
            with_members.is_empty() && memberless.is_empty(),
            "both denote the empty set"
        );
        assert_ne!(
            with_members, memberless,
            "but they are different geometries and must stay so"
        );
        assert_eq!(
            write_bare(&with_members, SCALE),
            "MULTIPOINT(EMPTY,EMPTY)",
            "the members are preserved in the rendering"
        );
        assert_eq!(
            write_bare(&memberless, SCALE),
            "MULTIPOINT EMPTY",
            "and the memberless form stays memberless"
        );
    }

    /// `write` prepends the system; `write_bare` does not, and the two differ by
    /// exactly that prefix.
    #[test]
    fn write_prepends_the_system_and_write_bare_does_not() {
        let literal = good("POINT(1 2)");
        assert_eq!(
            write(&literal, SCALE),
            "<http://example.org/crs/planar> POINT(1 2)",
            "the prefix is <IRI> and exactly one space"
        );
        assert_eq!(
            write_bare(literal.geometry(), SCALE),
            "POINT(1 2)",
            "the bare form omits the prefix entirely"
        );
        assert_eq!(
            write(&literal, SCALE),
            format!(
                "<{}> {}",
                literal.crs().as_str(),
                write_bare(literal.geometry(), SCALE)
            ),
            "the two differ by exactly the prefix"
        );
    }

    /// The rendering is a pure function of the geometry: the same input rendered
    /// repeatedly, and the same geometry reached by different spellings, produce
    /// identical bytes.
    #[test]
    fn the_rendering_is_a_pure_function_of_the_geometry() {
        let geometry = geometry_of("POLYGON((0 0,4 0,4 4,0 4,0 0),(1 1,2 1,2 2,1 1))");
        let first = write_bare(&geometry, SCALE);
        for _ in 0..8 {
            assert_eq!(
                write_bare(&geometry, SCALE),
                first,
                "the writer has no state, no map iteration and no clock"
            );
        }
        assert_eq!(
            write_bare(
                &geometry_of(
                    "polygon ( ( 0 0 , 4 0 , 4 4 , 0 4 , 0 0 ) , ( 1 1 , 2 1 , 2 2 , 1 1 ) )"
                ),
                SCALE
            ),
            first,
            "a differently-spelled but equal geometry renders identically"
        );
    }

    // -----------------------------------------------------------------------
    // The round-trip property
    // -----------------------------------------------------------------------

    /// Every kind at every dimension, empty and non-empty: `parse(write(parse(x)))`
    /// must equal `parse(x)`. This is the property that makes the codec a codec
    /// rather than two functions that happen to be in the same file.
    #[test]
    fn every_fixture_survives_a_write_then_parse_round_trip() {
        const FIXTURES: [&str; 34] = [
            // Point, every dimension, empty and not.
            "POINT(1 2)",
            "POINT Z (1 2 3)",
            "POINT M (1 2 3)",
            "POINT ZM (1 2 3 4)",
            "POINT EMPTY",
            "POINT Z EMPTY",
            "POINT M EMPTY",
            "POINT ZM EMPTY",
            // LineString.
            "LINESTRING(0 0,1 1,2 0)",
            "LINESTRING Z (0 0 0,1 1 1)",
            "LINESTRING M (0 0 0,1 1 1)",
            "LINESTRING ZM (0 0 0 0,1 1 1 1)",
            "LINESTRING EMPTY",
            "LINESTRING ZM EMPTY",
            // Polygon.
            "POLYGON((0 0,4 0,4 4,0 4,0 0))",
            "POLYGON((0 0,4 0,4 4,0 4,0 0),(1 1,2 1,2 2,1 1))",
            "POLYGON Z ((0 0 0,4 0 0,4 4 0,0 0 0))",
            "POLYGON ZM ((0 0 0 0,4 0 0 1,4 4 0 2,0 0 0 3))",
            "POLYGON EMPTY",
            "POLYGON M EMPTY",
            // MultiPoint, both spellings and the empty member.
            "MULTIPOINT((1 1),(2 2))",
            "MULTIPOINT(1 1,2 2)",
            "MULTIPOINT Z ((1 1 1),(2 2 2))",
            "MULTIPOINT(EMPTY,(1 1))",
            "MULTIPOINT EMPTY",
            // MultiLineString.
            "MULTILINESTRING((0 0,1 1),(2 2,3 3))",
            "MULTILINESTRING M ((0 0 0,1 1 1))",
            "MULTILINESTRING((0 0,1 1),EMPTY)",
            "MULTILINESTRING EMPTY",
            // MultiPolygon.
            "MULTIPOLYGON(((0 0,1 0,0 1,0 0)),((2 2,3 2,2 3,2 2)))",
            "MULTIPOLYGON ZM (((0 0 0 0,1 0 0 0,0 1 0 0,0 0 0 0)))",
            "MULTIPOLYGON(((0 0,1 0,0 1,0 0)),EMPTY)",
            "MULTIPOLYGON EMPTY",
            // Collections, flat and nested.
            "GEOMETRYCOLLECTION(POINT(1 2),LINESTRING(0 0,1 1),POLYGON((0 0,1 0,0 1,0 0)))",
        ];

        for text in FIXTURES {
            let once = good(text);
            let rendered = write(&once, SCALE);
            let twice = parse(&rendered, &crs()).unwrap_or_else(|err| {
                panic!("{text:?} rendered as {rendered:?}, which failed to re-parse: {err}")
            });
            assert_eq!(
                once, twice,
                "{text:?} must survive a write/parse round trip (rendered as {rendered:?})"
            );
            // And the rendering is a fixed point: writing the re-parsed literal
            // gives the same bytes, so the codec has settled after one pass.
            assert_eq!(
                write(&twice, SCALE),
                rendered,
                "{text:?} must reach its canonical rendering in one pass"
            );
        }

        // The nested and mixed-dimension cases, which do not fit the flat table.
        for text in [
            "GEOMETRYCOLLECTION EMPTY",
            "GEOMETRYCOLLECTION Z (POINT Z (1 2 3))",
            "GEOMETRYCOLLECTION ZM EMPTY",
            "GEOMETRYCOLLECTION(GEOMETRYCOLLECTION(POINT(1 2)))",
            "GEOMETRYCOLLECTION(MULTIPOINT(EMPTY),POINT EMPTY)",
            "<http://example.org/crs/other> POINT(1 2)",
            "<http://example.org/crs/other> GEOMETRYCOLLECTION Z (POINT Z (1 2 3))",
        ] {
            let once = good(text);
            let rendered = write(&once, SCALE);
            let twice = parse(&rendered, &crs()).unwrap_or_else(|err| {
                panic!("{text:?} rendered as {rendered:?}, which failed to re-parse: {err}")
            });
            assert_eq!(
                once, twice,
                "{text:?} must survive a write/parse round trip (rendered as {rendered:?})"
            );
        }
    }

    /// The round trip is exact for awkward numbers too — the ones a float codec
    /// would visibly lose.
    #[test]
    fn the_round_trip_is_exact_for_numbers_a_float_would_lose() {
        for text in [
            "POINT(0.1 0.2)",
            "POINT(0.3 0.7)",
            "POINT(-83.4 42.35)",
            "POINT(1e10 -1e10)",
            "POINT(0.0000000001 1000000000)",
            "POINT(123456789.123456789 0.000000000123456789)",
        ] {
            let once = good(text);
            let rendered = write(&once, SCALE);
            let twice = parse(&rendered, &crs()).expect("the rendering must re-parse");
            assert_eq!(
                once, twice,
                "{text:?} must round-trip exactly (rendered as {rendered:?})"
            );
            let original = once.geometry().coords().next().expect("one position");
            let restored = twice.geometry().coords().next().expect("one position");
            assert_eq!(
                original.x(),
                restored.x(),
                "{text:?} keeps its x ordinate bit for bit"
            );
        }
    }

    // -----------------------------------------------------------------------
    // The model boundary
    // -----------------------------------------------------------------------

    /// A geometry the reader never produces still renders sensibly, because the
    /// writer is total over the model rather than over what the reader emits.
    #[test]
    fn the_writer_is_total_over_the_model() {
        let point = Geometry::new(
            CoordDim::Xy,
            GeometryBody::Point(Some(Coord::xy(Rat::from_i64(7), Rat::from_i64(8)))),
        )
        .expect("a well-formed point");
        assert_eq!(
            write_bare(&point, SCALE),
            "POINT(7 8)",
            "a hand-built geometry renders like a parsed one"
        );
        for kind in [
            GeometryKind::Point,
            GeometryKind::LineString,
            GeometryKind::Polygon,
            GeometryKind::MultiPoint,
            GeometryKind::MultiLineString,
            GeometryKind::MultiPolygon,
            GeometryKind::GeometryCollection,
        ] {
            for dim in [CoordDim::Xy, CoordDim::Xyz, CoordDim::Xym, CoordDim::Xyzm] {
                let empty = Geometry::empty(dim, kind);
                let rendered = write_bare(&empty, SCALE);
                let reread = geometry_of(&rendered);
                assert_eq!(
                    reread, empty,
                    "the empty {kind:?} at {dim:?} rendered as {rendered:?} and must re-read as \
                     itself"
                );
            }
        }
    }

    /// The reader refuses structurally-impossible bodies by deferring to the
    /// model, and does not invent a repair for them.
    #[test]
    fn structural_refusals_come_from_the_model_and_are_not_repaired() {
        // A polygon ring written EMPTY is grammatically a `<linestring text>`, but
        // the model has no empty ring, so it is refused rather than silently
        // dropped — dropping it would turn a literal into a different geometry.
        assert_refused("POLYGON(EMPTY)");
        assert_refused("POLYGON((0 0,1 0,0 1,0 0),EMPTY)");
        // The neighbouring VALID cases: the empty polygon, and the empty member of
        // a multipolygon, both of which the model CAN hold.
        assert!(
            read("POLYGON EMPTY").is_ok(),
            "the memberless empty polygon is representable"
        );
        assert!(
            read("MULTIPOLYGON(EMPTY)").is_ok(),
            "an empty polygon member is representable"
        );
        assert!(
            read("MULTILINESTRING(EMPTY)").is_ok(),
            "an empty linestring member is representable"
        );
        // A ring with too few positions is the model's rule, reported as a literal
        // error rather than as a panic or a silent fix.
        assert_refused("POLYGON((0 0,1 0,0 0))");
    }

    /// Diagnostics name the offending construct, so a refusal is actionable rather
    /// than merely a refusal.
    #[test]
    fn diagnostics_name_what_went_wrong() {
        for (text, needle) in [
            ("POINTZ(1 2 3)", "POINTZ"),
            ("CIRCULARSTRING(0 0,1 1)", "CIRCULARSTRING"),
            ("POINT(NaN 1)", "NaN"),
            ("MULTIPOINT()", "EMPTY"),
            ("<> POINT(1 2)", "empty"),
            ("POINT(1 2) X", "byte"),
        ] {
            let err = refused(text);
            assert!(
                err.detail().contains(needle),
                "the diagnostic for {text:?} must mention {needle:?}, but was {}",
                err.detail()
            );
        }
    }
}
