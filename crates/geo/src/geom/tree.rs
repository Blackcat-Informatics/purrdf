// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The geometry tree's `Drop`, `Clone`, `PartialEq` and `Debug`, and the walk over
//! every position, each over an explicit heap work list rather than recursion.
//!
//! A [`Geometry`] owns other geometries through
//! [`GeometryBody::GeometryCollection`], and a `wktLiteral` or `geoJSONLiteral` is
//! untrusted input, so a collection nests as deep as its author writes it and the
//! readers have no depth at which to refuse it. The glue `#[derive]` writes for a
//! recursive type recurses once per level, and in Rust a stack overflow is an
//! `abort` no caller can catch, so every walk over the whole tree is a loop over a
//! heap stack:
//!
//! * **`Drop`** moves a collection's members onto a work list and dismantles the list
//!   in a loop, taking every popped member's own members before it goes, so no drop
//!   it runs recurses.
//! * **`Clone`** builds the copy bottom-up: a two-phase walk enters each geometry
//!   and, on the way back out, assembles a collection's copy from its members'
//!   finished copies. A geometry that is not a collection is copied outright.
//! * **`PartialEq`** compares two trees pair by pair off one work list.
//! * **`Debug`** prints the *script* `#[derive(Debug)]` prints — struct and variant
//!   names, field names, leaf values — token by token, in both the plain (`{:?}`)
//!   and the pretty (`{:#?}`) form, indenting the pretty form the way the standard
//!   library's builders do, so the bytes are the derive's exactly.
//! * **[`Coords`]** yields every position in written order off a work list of
//!   pending geometries.
//!
//! The six non-collection bodies own no geometry, so [`GeometryBody`]'s derived
//! impls reach a nested geometry only through the `GeometryCollection` variant, where
//! they call the impls here within one frame.

use core::fmt::{self, Write as _};
use core::mem;

use super::{Coord, CoordDim, Geometry, GeometryBody};

/// Whether `geometry` owns other geometries.
fn has_members(geometry: &Geometry) -> bool {
    matches!(&geometry.body, GeometryBody::GeometryCollection(members) if !members.is_empty())
}

// ── Drop ─────────────────────────────────────────────────────────────────────────

impl Drop for Geometry {
    fn drop(&mut self) {
        let GeometryBody::GeometryCollection(members) = &mut self.body else {
            return;
        };
        if members.is_empty() {
            return;
        }
        let mut work = mem::take(members);
        while let Some(mut member) = work.pop() {
            if let GeometryBody::GeometryCollection(inner) = &mut member.body {
                work.append(inner);
            }
            // `member` drops here owning no geometry, so the drop it runs is this
            // one's early return.
        }
    }
}

// ── Clone ────────────────────────────────────────────────────────────────────────

/// One step of the bottom-up copy.
enum Step<'a> {
    /// Visit a geometry: copy one that owns no geometry outright, or schedule a
    /// collection's members before it.
    Enter(&'a Geometry),
    /// Every member of the collection has been copied; assemble the collection's copy.
    Exit(&'a Geometry),
}

/// A copy of the tree under `root`, built bottom-up over a work list.
fn clone_tree(root: &Geometry) -> Geometry {
    let mut steps: Vec<Step<'_>> = vec![Step::Enter(root)];
    let mut copies: Vec<Geometry> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(node) => match &node.body {
                GeometryBody::GeometryCollection(members) if !members.is_empty() => {
                    // Members are pushed last-first so they pop, and so their copies
                    // land, in written order.
                    steps.push(Step::Exit(node));
                    steps.extend(members.iter().rev().map(Step::Enter));
                }
                body => copies.push(Geometry {
                    dim: node.dim,
                    body: body.clone(),
                }),
            },
            Step::Exit(node) => {
                let GeometryBody::GeometryCollection(members) = &node.body else {
                    unreachable!("only a collection is exited")
                };
                let first = copies.len() - members.len();
                let copied: Vec<Geometry> = copies.drain(first..).collect();
                copies.push(Geometry {
                    dim: node.dim,
                    body: GeometryBody::GeometryCollection(copied),
                });
            }
        }
    }
    copies
        .pop()
        .expect("the root's copy is the last one assembled")
}

impl Clone for Geometry {
    fn clone(&self) -> Self {
        if has_members(self) {
            clone_tree(self)
        } else {
            Self {
                dim: self.dim,
                body: self.body.clone(),
            }
        }
    }
}

// ── PartialEq ────────────────────────────────────────────────────────────────────

impl PartialEq for Geometry {
    fn eq(&self, other: &Self) -> bool {
        let mut work: Vec<(&Self, &Self)> = vec![(self, other)];
        while let Some((a, b)) = work.pop() {
            if a.dim != b.dim {
                return false;
            }
            match (&a.body, &b.body) {
                (GeometryBody::GeometryCollection(x), GeometryBody::GeometryCollection(y)) => {
                    if x.len() != y.len() {
                        return false;
                    }
                    work.extend(x.iter().zip(y.iter()));
                }
                (GeometryBody::GeometryCollection(_), _)
                | (_, GeometryBody::GeometryCollection(_)) => return false,
                (x, y) => {
                    if x != y {
                        return false;
                    }
                }
            }
        }
        true
    }
}

// ── Coordinates ──────────────────────────────────────────────────────────────────

/// Every position of a geometry in written order, off a work list of the geometries
/// still to visit.
pub(super) struct Coords<'a> {
    /// Geometries not yet visited, the next one on top.
    pending: Vec<&'a Geometry>,
    /// The positions of the geometry being visited, when it holds positions itself.
    leaf: Option<Box<dyn Iterator<Item = &'a Coord> + 'a>>,
}

impl<'a> Coords<'a> {
    pub(super) fn new(root: &'a Geometry) -> Self {
        Self {
            pending: vec![root],
            leaf: None,
        }
    }
}

impl<'a> Iterator for Coords<'a> {
    type Item = &'a Coord;

    fn next(&mut self) -> Option<&'a Coord> {
        loop {
            if let Some(coord) = self.leaf.as_mut().and_then(Iterator::next) {
                return Some(coord);
            }
            self.leaf = None;
            let geometry = self.pending.pop()?;
            self.leaf = Some(match &geometry.body {
                GeometryBody::GeometryCollection(members) => {
                    // Members are pushed last-first so the first is visited first.
                    self.pending.extend(members.iter().rev());
                    continue;
                }
                GeometryBody::Point(point) => Box::new(point.iter()),
                GeometryBody::LineString(coords) => Box::new(coords.iter()),
                GeometryBody::Polygon(rings) => Box::new(rings.iter().flat_map(|r| r.iter())),
                GeometryBody::MultiPoint(points) => Box::new(points.iter().flatten()),
                GeometryBody::MultiLineString(lines) => {
                    Box::new(lines.iter().flat_map(|l| l.iter()))
                }
                GeometryBody::MultiPolygon(polygons) => Box::new(
                    polygons
                        .iter()
                        .flat_map(|p| p.iter().flat_map(|r| r.iter())),
                ),
            });
        }
    }
}

// ── Debug ────────────────────────────────────────────────────────────────────────

/// One token of a geometry's script.
#[derive(Clone, Copy)]
enum Tok<'a> {
    /// A struct opens: its name.
    Struct(&'static str),
    /// A field of the open struct: its name. Its value follows.
    Field(&'static str),
    /// The open struct closes.
    EndStruct,
    /// A tuple-like variant opens: its name.
    Tuple(&'static str),
    /// The open tuple closes.
    EndTuple,
    /// A list opens.
    List,
    /// The open list closes.
    EndList,
    /// The dimension, a leaf.
    Dim(CoordDim),
    /// A body that owns no geometry, a leaf.
    Body(&'a GeometryBody),
    /// A member geometry, read as its own script.
    Node(&'a Geometry),
}

/// Append the script `#[derive(Debug)]` prints for `node` to `out`, each member as a
/// [`Tok::Node`].
fn script<'a>(node: &'a Geometry, out: &mut Vec<Tok<'a>>) {
    out.extend([
        Tok::Struct("Geometry"),
        Tok::Field("dim"),
        Tok::Dim(node.dim),
        Tok::Field("body"),
    ]);
    match &node.body {
        GeometryBody::GeometryCollection(members) => {
            out.extend([Tok::Tuple("GeometryCollection"), Tok::List]);
            out.extend(members.iter().map(Tok::Node));
            out.extend([Tok::EndList, Tok::EndTuple]);
        }
        body => out.push(Tok::Body(body)),
    }
    out.push(Tok::EndStruct);
}

/// Replace the `Node` token just popped from `stack` by its script, so the script's
/// first token is popped next.
fn expand<'a>(node: &'a Geometry, stack: &mut Vec<Tok<'a>>) {
    let before = stack.len();
    script(node, stack);
    stack[before..].reverse();
}

/// Writes into a formatter, indenting every line after the first by four spaces per
/// open pretty-printed container — what nesting the standard library's `PadAdapter`
/// once per level produces.
struct Pad<'f, 'g> {
    f: &'f mut fmt::Formatter<'g>,
    pretty: bool,
    depth: usize,
    line_start: bool,
}

impl fmt::Write for Pad<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for piece in s.split_inclusive('\n') {
            if self.line_start {
                for _ in 0..self.depth {
                    self.f.write_str("    ")?;
                }
            }
            self.line_start = piece.ends_with('\n');
            self.f.write_str(piece)?;
        }
        Ok(())
    }
}

/// A container open in [`geometry_debug`]: what it is and how many entries it holds
/// so far.
struct Open {
    kind: OpenKind,
    entries: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenKind {
    Struct,
    Tuple,
    List,
}

impl Pad<'_, '_> {
    /// A value starts inside the innermost open container: write what separates it
    /// from the entry before it.
    fn begin(&mut self, open: &mut [Open]) -> fmt::Result {
        let Some(container) = open.last_mut() else {
            return Ok(());
        };
        match container.kind {
            // The field token already wrote the separator and the name.
            OpenKind::Struct => return Ok(()),
            OpenKind::Tuple => {
                if self.pretty {
                    if container.entries == 0 {
                        self.write_str("(\n")?;
                        self.depth += 1;
                    }
                } else {
                    self.write_str(if container.entries == 0 { "(" } else { ", " })?;
                }
            }
            OpenKind::List => {
                if self.pretty {
                    if container.entries == 0 {
                        self.write_str("\n")?;
                        self.depth += 1;
                    }
                } else if container.entries > 0 {
                    self.write_str(", ")?;
                }
            }
        }
        container.entries += 1;
        Ok(())
    }

    /// A value inside an open container has ended.
    fn end(&mut self, open: &[Open]) -> fmt::Result {
        if self.pretty && !open.is_empty() {
            self.write_str(",\n")?;
        }
        Ok(())
    }

    /// A leaf, written as the derive writes it in this form.
    fn leaf<T: fmt::Debug>(&mut self, value: &T) -> fmt::Result {
        if self.pretty {
            write!(self, "{value:#?}")
        } else {
            write!(self, "{value:?}")
        }
    }
}

/// `{:?}` / `{:#?}` of `geometry` exactly as `#[derive(Debug)]` writes them, over a
/// work list.
fn geometry_debug(geometry: &Geometry, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let pretty = f.alternate();
    let mut out = Pad {
        f,
        pretty,
        depth: 0,
        line_start: false,
    };
    let mut stack: Vec<Tok<'_>> = vec![Tok::Node(geometry)];
    let mut open: Vec<Open> = Vec::new();
    while let Some(tok) = stack.pop() {
        match tok {
            Tok::Node(node) => expand(node, &mut stack),
            Tok::Struct(name) | Tok::Tuple(name) => {
                out.begin(&mut open)?;
                out.write_str(name)?;
                open.push(Open {
                    kind: if matches!(tok, Tok::Struct(_)) {
                        OpenKind::Struct
                    } else {
                        OpenKind::Tuple
                    },
                    entries: 0,
                });
            }
            Tok::List => {
                out.begin(&mut open)?;
                out.write_str("[")?;
                open.push(Open {
                    kind: OpenKind::List,
                    entries: 0,
                });
            }
            Tok::Field(name) => {
                let container = open
                    .last_mut()
                    .expect("a field is written inside its struct");
                if pretty {
                    if container.entries == 0 {
                        out.write_str(" {\n")?;
                        out.depth += 1;
                    }
                } else {
                    out.write_str(if container.entries == 0 { " { " } else { ", " })?;
                }
                container.entries += 1;
                out.write_str(name)?;
                out.write_str(": ")?;
            }
            Tok::EndStruct | Tok::EndTuple | Tok::EndList => {
                let container = open.pop().expect("a container closes after it opens");
                match container.kind {
                    OpenKind::Struct if container.entries > 0 => {
                        if pretty {
                            out.depth -= 1;
                            out.write_str("}")?;
                        } else {
                            out.write_str(" }")?;
                        }
                    }
                    OpenKind::Struct => {}
                    OpenKind::Tuple if container.entries > 0 => {
                        if pretty {
                            out.depth -= 1;
                        }
                        out.write_str(")")?;
                    }
                    OpenKind::Tuple => {}
                    OpenKind::List => {
                        if pretty && container.entries > 0 {
                            out.depth -= 1;
                        }
                        out.write_str("]")?;
                    }
                }
                out.end(&open)?;
            }
            Tok::Dim(dim) => {
                out.begin(&mut open)?;
                out.leaf(&dim)?;
                out.end(&open)?;
            }
            Tok::Body(body) => {
                out.begin(&mut open)?;
                out.leaf(body)?;
                out.end(&open)?;
            }
        }
    }
    Ok(())
}

impl fmt::Debug for Geometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        geometry_debug(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        Coord, CoordDim, CoordSeq, Crs, Geometry, GeometryBody, GeometryKind, Rings,
    };

    /// A type-for-type twin of the geometry tree with the compiler's own `Debug`, so
    /// the derive itself is the oracle for what the iterative `Debug` must write.
    #[allow(
        dead_code,
        reason = "the twin's fields exist to be printed by its derived `Debug`, which dead-code \
                  analysis does not count as a read"
    )]
    mod derived {
        use super::{Coord, CoordDim, CoordSeq, Rings};

        #[derive(Debug)]
        pub(super) struct Geometry {
            pub(super) dim: CoordDim,
            pub(super) body: GeometryBody,
        }

        #[allow(
            clippy::large_enum_variant,
            reason = "the twin mirrors the model's own variants, whose Point variant is a \
                      Coord's size for the reason `geom::GeometryBody` gives"
        )]
        #[derive(Debug)]
        pub(super) enum GeometryBody {
            Point(Option<Coord>),
            LineString(CoordSeq),
            Polygon(Rings),
            MultiPoint(Vec<Option<Coord>>),
            MultiLineString(Vec<CoordSeq>),
            MultiPolygon(Vec<Rings>),
            GeometryCollection(Vec<Geometry>),
        }
    }

    /// The twin of `geometry`; recursive, on shallow fixtures only.
    fn twin(geometry: &Geometry) -> derived::Geometry {
        derived::Geometry {
            dim: geometry.dim(),
            body: match geometry.body() {
                GeometryBody::Point(p) => derived::GeometryBody::Point(p.clone()),
                GeometryBody::LineString(c) => derived::GeometryBody::LineString(c.clone()),
                GeometryBody::Polygon(r) => derived::GeometryBody::Polygon(r.clone()),
                GeometryBody::MultiPoint(p) => derived::GeometryBody::MultiPoint(p.clone()),
                GeometryBody::MultiLineString(l) => {
                    derived::GeometryBody::MultiLineString(l.clone())
                }
                GeometryBody::MultiPolygon(p) => derived::GeometryBody::MultiPolygon(p.clone()),
                GeometryBody::GeometryCollection(members) => {
                    derived::GeometryBody::GeometryCollection(members.iter().map(twin).collect())
                }
            },
        }
    }

    fn fixtures() -> Vec<Geometry> {
        let crs = Crs::new("http://example.org/crs/planar").expect("a non-empty IRI");
        [
            "POINT(1 2)",
            "POINT Z (1 2 3)",
            "POINT EMPTY",
            "GEOMETRYCOLLECTION EMPTY",
            "MULTIPOINT((1 1),EMPTY)",
            "MULTIPOLYGON(((0 0,1 0,0 1,0 0)),EMPTY)",
            "GEOMETRYCOLLECTION(POINT(1 2),LINESTRING(0 0,1 1),GEOMETRYCOLLECTION(POLYGON((0 0,1 \
             0,0 1,0 0)),GEOMETRYCOLLECTION EMPTY),MULTILINESTRING((0 0,1 1),EMPTY))",
        ]
        .into_iter()
        .map(|text| {
            crate::wkt::parse(text, &crs)
                .unwrap_or_else(|err| panic!("{text} parses: {err}"))
                .into_geometry()
        })
        .collect()
    }

    /// The iterative `Debug` writes byte for byte what the derive writes, in both
    /// forms, for every kind, empty and filled, flat and nested.
    #[test]
    fn the_iterative_debug_is_byte_identical_to_the_derive() {
        for geometry in fixtures() {
            let expected = twin(&geometry);
            assert_eq!(format!("{geometry:?}"), format!("{expected:?}"));
            assert_eq!(format!("{geometry:#?}"), format!("{expected:#?}"));
        }
    }

    /// A clone is equal to its original, prints identically and yields the same
    /// positions, for every shape; and equality tells the shapes apart.
    #[test]
    fn a_clone_is_the_same_geometry_and_the_shapes_are_distinct() {
        let all = fixtures();
        for (index, geometry) in all.iter().enumerate() {
            let copy = geometry.clone();
            assert_eq!(&copy, geometry);
            assert_eq!(format!("{copy:?}"), format!("{geometry:?}"));
            assert_eq!(copy.coord_count(), geometry.coord_count());
            assert!(
                copy.coords().zip(geometry.coords()).all(|(a, b)| a == b),
                "positions are yielded in the same order"
            );
            for (other_index, other) in all.iter().enumerate() {
                assert_eq!(geometry == other, index == other_index);
            }
        }
    }

    /// A hundred thousand nested collections, built one level at a time through
    /// `Geometry::new`, are cloned, compared, printed, counted and dropped on a
    /// 128 KiB stack.
    ///
    /// The `Debug` length is a formula whose constant is pinned against the derive's
    /// own spelling at depths one and two: the empty collection prints as
    /// `Geometry { dim: Xy, body: GeometryCollection([]) }` (50 bytes), and each
    /// further level wraps the one below in `Geometry { dim: Xy, body:
    /// GeometryCollection([` and `]) }` (50 bytes).
    #[test]
    fn a_hundred_thousand_deep_collection_clones_compares_prints_and_drops() {
        let depth = 100_000usize;
        const ONE: &str = "Geometry { dim: Xy, body: GeometryCollection([]) }";
        const TWO: &str = "Geometry { dim: Xy, body: GeometryCollection([Geometry { dim: Xy, body: \
                           GeometryCollection([]) }]) }";
        let nest = |inner: Geometry| {
            Geometry::new(CoordDim::Xy, GeometryBody::GeometryCollection(vec![inner]))
                .expect("a collection of one collection is well formed")
        };
        let empty = || Geometry::empty(CoordDim::Xy, GeometryKind::GeometryCollection);
        assert_eq!(format!("{:?}", empty()), ONE);
        assert_eq!(format!("{:?}", nest(empty())), TWO);
        let per_level = TWO.len() - ONE.len();
        assert_eq!((ONE.len(), per_level), (50, 50));
        let expected_debug_len = ONE.len() + per_level * (depth - 1);

        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(move || {
                let mut geometry = empty();
                for _ in 1..depth {
                    geometry = nest(geometry);
                }
                let copy = geometry.clone();
                assert_eq!(copy, geometry);
                assert!(geometry.is_empty());
                assert_eq!(geometry.coord_count(), 0);
                assert_eq!(format!("{geometry:?}").len(), expected_debug_len);
                // A point at the bottom makes the tree non-empty and its one
                // position reachable through every level.
                let crs = Crs::new("http://example.org/crs/planar").expect("a non-empty IRI");
                let mut pointed = crate::wkt::parse("POINT(1 2)", &crs)
                    .expect("a point")
                    .into_geometry();
                for _ in 1..depth {
                    pointed = nest(pointed);
                }
                assert!(!pointed.is_empty());
                assert_eq!(pointed.coord_count(), 1);
                assert_ne!(pointed, geometry);
                drop(pointed);
                drop(copy);
                drop(geometry);
            })
            .expect("the thread starts")
            .join()
            .expect("the walks did not abort");
    }
}
