// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The geometry tree's `Drop`, `Clone`, `PartialEq` and `Debug`, the bottom-up fold
//! every derived answer over a tree is built with, and the walk over every position,
//! each over an explicit heap work list rather than recursion.
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
//! * **[`try_fold`]** and **[`fold`]** build an answer bottom-up: a two-phase walk
//!   enters each geometry and, on the way back out, assembles a collection's answer
//!   from its members' finished answers in written order. `Clone` is the fold whose
//!   leaf answer is a copy, and the GeoJSON writer, the boundary constructor and
//!   point location are folds too.
//! * **`PartialEq`** compares two trees pair by pair off one work list.
//! * **`Debug`** prints the *script* `#[derive(Debug)]` prints — struct and variant
//!   names, field names, leaf values — through [`purrdf_lex::walk::write_debug`],
//!   which writes it token by token in both the plain (`{:?}`) and the pretty (`{:#?}`) form, so
//!   the bytes are the derive's exactly.
//! * **[`Coords`]** yields every position in written order off a work list of
//!   pending geometries.
//!
//! The six non-collection bodies own no geometry, so [`GeometryBody`]'s derived
//! impls reach a nested geometry only through the `GeometryCollection` variant, where
//! they call the impls here within one frame.

use core::convert::Infallible;
use core::fmt;
use core::mem;

use super::{Coord, Geometry, GeometryBody};
use purrdf_lex::walk::{Tok, WorkList, write_debug};

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

// ── Bottom-up fold ───────────────────────────────────────────────────────────────

/// One step of the bottom-up walk.
enum Step<'a> {
    /// Visit a geometry: answer for one that owns no geometry outright, or schedule a
    /// collection's members before it.
    Enter(&'a Geometry),
    /// Every member of the collection has been answered; assemble the collection's
    /// answer.
    Exit(&'a Geometry),
}

/// An answer for the tree under `root`, built bottom-up over a work list.
///
/// `leaf` answers for a geometry that is not a collection; `collection` assembles a
/// collection's answer from its members' answers, in written order, and receives an
/// empty vector for a collection with no members. The walk stops at the first `Err`,
/// which is the error the recursive spelling would have raised: leaves are visited in
/// written order and a collection is assembled after its last member.
pub(crate) fn try_fold<'a, T, E>(
    root: &'a Geometry,
    mut leaf: impl FnMut(&'a Geometry) -> Result<T, E>,
    mut collection: impl FnMut(&'a Geometry, Vec<T>) -> Result<T, E>,
) -> Result<T, E> {
    let mut steps: Vec<Step<'a>> = vec![Step::Enter(root)];
    let mut answers: Vec<T> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(node) => match &node.body {
                GeometryBody::GeometryCollection(members) => {
                    // Members are pushed last-first so they pop, and so their
                    // answers land, in written order.
                    steps.push(Step::Exit(node));
                    steps.extend(members.iter().rev().map(Step::Enter));
                }
                _ => answers.push(leaf(node)?),
            },
            Step::Exit(node) => {
                let GeometryBody::GeometryCollection(members) = &node.body else {
                    unreachable!("only a collection is exited")
                };
                let first = answers.len() - members.len();
                let of_members: Vec<T> = answers.drain(first..).collect();
                answers.push(collection(node, of_members)?);
            }
        }
    }
    Ok(answers
        .pop()
        .expect("the root's answer is the last one assembled"))
}

/// [`try_fold`] for answers that cannot fail.
pub(crate) fn fold<'a, T>(
    root: &'a Geometry,
    mut leaf: impl FnMut(&'a Geometry) -> T,
    mut collection: impl FnMut(&'a Geometry, Vec<T>) -> T,
) -> T {
    let Ok(answer) = try_fold::<T, Infallible>(
        root,
        |node| Ok(leaf(node)),
        |node, of_members| Ok(collection(node, of_members)),
    );
    answer
}

// ── Clone ────────────────────────────────────────────────────────────────────────

impl Clone for Geometry {
    fn clone(&self) -> Self {
        if has_members(self) {
            fold(
                self,
                |node| Self {
                    dim: node.dim,
                    body: node.body.clone(),
                },
                |node, copies| Self {
                    dim: node.dim,
                    body: GeometryBody::GeometryCollection(copies),
                },
            )
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

/// Append the script `#[derive(Debug)]` prints for `node` to `out`, each member as a
/// [`Tok::Node`].
fn script<'a>(node: &'a Geometry, out: &mut WorkList<GeometryTok<'a>, 16>) {
    out.extend([
        Tok::Struct("Geometry"),
        Tok::Field("dim"),
        Tok::Leaf(&node.dim as &dyn fmt::Debug),
        Tok::Field("body"),
    ]);
    match &node.body {
        GeometryBody::GeometryCollection(members) => {
            out.extend([Tok::Tuple("GeometryCollection"), Tok::List(members.len())]);
            out.extend(members.iter().map(Tok::Node));
            out.extend([Tok::EndList, Tok::EndTuple]);
        }
        body => out.push(Tok::Leaf(body)),
    }
    out.push(Tok::EndStruct);
}

/// One token of a geometry's `Debug` script.
type GeometryTok<'a> = Tok<&'a Geometry, &'a dyn fmt::Debug>;

impl fmt::Debug for Geometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_debug(f, self, script)
    }
}

#[cfg(test)]
mod tests {
    use super::super::arbitrary::{self, Lcg};
    use super::super::{
        Coord, CoordDim, CoordSeq, Crs, Geometry, GeometryBody, GeometryKind, Rings,
    };
    use super::{fold, try_fold};

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

    /// The derive stays the oracle over generated trees of every shape: the plain and
    /// the pretty form agree byte for byte, and equality agrees with the derive's
    /// printed form, which is injective over these types.
    #[test]
    fn the_iterative_debug_and_equality_agree_with_the_derive_on_generated_trees() {
        let mut rng = Lcg::new(0x5eed_0001);
        let dims = [CoordDim::Xy, CoordDim::Xyz, CoordDim::Xym, CoordDim::Xyzm];
        let trees: Vec<Geometry> = (0..160)
            .map(|index| arbitrary::geometry(&mut rng, dims[index % 4], 3))
            .collect();
        let printed: Vec<String> = trees
            .iter()
            .map(|tree| format!("{:?}", twin(tree)))
            .collect();
        for (index, tree) in trees.iter().enumerate() {
            assert_eq!(format!("{tree:?}"), printed[index]);
            assert_eq!(format!("{tree:#?}"), format!("{:#?}", twin(tree)));
            let copy = tree.clone();
            assert_eq!(
                format!("{copy:?}"),
                printed[index],
                "a copy prints the same"
            );
            for (other_index, other) in trees.iter().enumerate() {
                assert_eq!(
                    tree == other,
                    printed[index] == printed[other_index],
                    "equality agrees with the derive's printed form"
                );
            }
        }
    }

    /// The recursive spelling of a fold, the reference the work-list fold is
    /// compared with on shallow generated trees.
    fn reference_fold<T>(
        node: &Geometry,
        leaf: &mut impl FnMut(&Geometry) -> T,
        collection: &mut impl FnMut(&Geometry, Vec<T>) -> T,
    ) -> T {
        match node.body() {
            GeometryBody::GeometryCollection(members) => {
                let answers = members
                    .iter()
                    .map(|member| reference_fold(member, leaf, collection))
                    .collect();
                collection(node, answers)
            }
            _ => leaf(node),
        }
    }

    /// The fold visits leaves in written order, assembles each collection from
    /// exactly its members' answers, and stops at the first error — the same script
    /// the recursive spelling produces.
    #[test]
    fn the_fold_visits_the_same_script_as_the_recursive_reference() {
        let mut rng = Lcg::new(0x5eed_0002);
        for round in 0..200 {
            let tree = arbitrary::geometry(&mut rng, CoordDim::Xy, 4);
            // A leaf's answer is the WKT it writes; a collection's is its members'
            // answers bracketed, so the answer is a rendering of the whole tree.
            let mut leaf = |node: &Geometry| crate::wkt::write_bare(node, 0);
            let mut collection =
                |_: &Geometry, answers: Vec<String>| format!("<{}>", answers.join("|"));
            let iterative = fold(&tree, &mut leaf, &mut collection);
            let recursive = reference_fold(&tree, &mut leaf, &mut collection);
            assert_eq!(iterative, recursive, "round {round}: {tree:?}");

            // Failing at the n-th leaf, for every n, stops at the same leaf; `fail_at`
            // zero never fails, since the first leaf is the first.
            let leaves = fold(&tree, |_| 1_usize, |_, counts| counts.iter().sum());
            let failing_leaf = |fail_at: usize| {
                let mut seen = 0_usize;
                move |node: &Geometry| {
                    seen += 1;
                    if seen == fail_at {
                        Err(format!("stopped at leaf {fail_at}: {node:?}"))
                    } else {
                        Ok(seen)
                    }
                }
            };
            for fail_at in 0..=leaves {
                let iterative = try_fold(&tree, failing_leaf(fail_at), |_, answers: Vec<usize>| {
                    Ok(answers.into_iter().max().unwrap_or(0))
                });
                let recursive = reference_try_fold(&tree, &mut failing_leaf(fail_at));
                assert_eq!(iterative, recursive, "round {round}, failing at {fail_at}");
            }
        }
    }

    /// The recursive reference for the failing fold: the maximum leaf ordinal, or the
    /// first leaf's refusal.
    fn reference_try_fold(
        node: &Geometry,
        leaf: &mut impl FnMut(&Geometry) -> Result<usize, String>,
    ) -> Result<usize, String> {
        match node.body() {
            GeometryBody::GeometryCollection(members) => {
                let mut best = 0;
                for member in members {
                    best = best.max(reference_try_fold(member, leaf)?);
                }
                Ok(best)
            }
            _ => leaf(node),
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
    /// `Geometry::new`, are cloned, compared, printed, counted, folded and dropped on
    /// a 128 KiB stack.
    ///
    /// The `Debug` length is a formula whose constant is pinned against the derive's
    /// own spelling at depths one and two: the empty collection prints as
    /// `Geometry { dim: Xy, body: GeometryCollection([]) }` (50 bytes), and each
    /// further level wraps the one below in `Geometry { dim: Xy, body:
    /// GeometryCollection([` and `]) }` (50 bytes).
    #[test]
    fn a_hundred_thousand_deep_collection_clones_compares_prints_and_drops() {
        let depth = arbitrary::DEEP;
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

        arbitrary::on_small_stack(move || {
            let mut geometry = empty();
            for _ in 1..depth {
                geometry = nest(geometry);
            }
            let copy = geometry.clone();
            assert_eq!(copy, geometry);
            assert!(geometry.is_empty());
            assert_eq!(geometry.coord_count(), 0);
            assert_eq!(format!("{geometry:?}").len(), expected_debug_len);
            // The fold counts the levels: every collection adds one to its single
            // member's count, and the innermost empty collection is one level.
            let levels = fold(
                &geometry,
                |_| 0_usize,
                |_, counts| 1 + counts.into_iter().max().unwrap_or(0),
            );
            assert_eq!(levels, depth);
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
        });
    }
}
