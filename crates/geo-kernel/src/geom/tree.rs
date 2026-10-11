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

use super::{Coord, CoordDim, Geometry, GeometryBody};
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use purrdf_lex::walk::{Tok, WorkList, dismantle_vec_tree, write_debug};

/// Visit non-collection nodes in written order through the original tree walk.
/// The work list is admitted before growth and destroyed before any refund.
pub(crate) fn visit_leaves_with_memory<'a, S: Admission + ?Sized, B>(
    root: &'a Geometry,
    memory: &mut Memory<'_, S>,
    mut visit: impl FnMut(&'a Geometry) -> core::ops::ControlFlow<B>,
) -> Result<core::ops::ControlFlow<B>, StorageError> {
    visit_leaves_with_memory_using(root, memory, |node, _| visit(node))
}

/// Continue the same native walk while a visitor grows its original metadata.
pub(crate) fn visit_leaves_with_memory_using<'a, S: Admission + ?Sized, B>(
    root: &'a Geometry,
    memory: &mut Memory<'_, S>,
    mut visit: impl FnMut(&'a Geometry, &mut Memory<'_, S>) -> core::ops::ControlFlow<B>,
) -> Result<core::ops::ControlFlow<B>, StorageError> {
    memory.try_scope(|memory| {
        let mut leaves = Leaves::new_with_memory(root, memory)?;
        while let Some(node) = leaves.next_with_memory(memory)? {
            if let core::ops::ControlFlow::Break(answer) = visit(node, memory) {
                memory.release_vec(leaves.pending)?;
                return Ok(core::ops::ControlFlow::Break(answer));
            }
        }
        memory.release_vec(leaves.pending)?;
        Ok(core::ops::ControlFlow::Continue(()))
    })
}

/// Resume the same leaf iterator between callbacks. The callback can use the
/// numerical arena after the iterator's original storage borrow has ended.
pub(crate) fn visit_leaves_with_progress<'a, 'observer>(
    root: &'a Geometry,
    progress: &mut crate::context::WorkProgress<'observer>,
    visit: impl FnMut(
        &'a Geometry,
        &mut crate::context::WorkProgress<'observer>,
    ) -> Result<(), crate::GeoError>,
) -> Result<(), crate::GeoError> {
    let leaves = progress
        .with_storage(0, |memory| {
            memory.try_scope(|memory| Leaves::new_with_memory(root, memory))
        })
        .map_err(crate::GeoError::Storage)?;
    visit_with_progress(leaves, progress, visit)
}

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
        dismantle_vec_tree(
            mem::take(members),
            |node| match &mut node.body {
                GeometryBody::GeometryCollection(children) => Some(mem::take(children)),
                _ => None,
            },
            |children| Self {
                dim: CoordDim::Xy,
                body: GeometryBody::GeometryCollection(children),
            },
        );
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

pub(crate) enum FoldError<E> {
    Visitor(E),
    Storage(StorageError),
}

impl FoldError<crate::GeoError> {
    pub(crate) fn into_geo_error(self) -> crate::GeoError {
        match self {
            Self::Visitor(error) => error,
            Self::Storage(error) => crate::GeoError::Storage(error),
        }
    }
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
    let mut resident = Resident;
    match try_fold_with_memory(
        root,
        &mut Memory::new(&mut resident),
        |node, _| leaf(node),
        |node, members, _| collection(node, members),
    ) {
        Ok(answer) => Ok(answer),
        Err(FoldError::Visitor(error)) => Err(error),
        Err(FoldError::Storage(error)) => panic!("resident geometry fold allocation: {error}"),
    }
}

/// The same bottom-up fold with original before-birth storage. A collection
/// callback receives its original admitted member array: it either retains
/// that array in its answer, or destroys it with `Memory::release_vec` before
/// refund. Visitor failure destroys all partial answers before scope cleanup.
pub(crate) fn try_fold_with_memory<'a, T, E, S: Admission + ?Sized>(
    root: &'a Geometry,
    memory: &mut Memory<'_, S>,
    mut leaf: impl FnMut(&'a Geometry, &mut Memory<'_, S>) -> Result<T, E>,
    mut collection: impl FnMut(&'a Geometry, Vec<T>, &mut Memory<'_, S>) -> Result<T, E>,
) -> Result<T, FoldError<E>> {
    memory.try_scope(|memory| {
        let mut steps: Vec<Step<'a>> = Vec::new();
        let mut answers: Vec<T> = Vec::new();
        memory
            .push(&mut steps, Step::Enter(root))
            .map_err(FoldError::Storage)?;
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(node) => match &node.body {
                    GeometryBody::GeometryCollection(members) => {
                        memory
                            .push(&mut steps, Step::Exit(node))
                            .map_err(FoldError::Storage)?;
                        for member in members.iter().rev() {
                            memory
                                .push(&mut steps, Step::Enter(member))
                                .map_err(FoldError::Storage)?;
                        }
                    }
                    _ => {
                        memory
                            .reserve_for_push(&mut answers)
                            .map_err(FoldError::Storage)?;
                        answers.push(leaf(node, memory).map_err(FoldError::Visitor)?);
                    }
                },
                Step::Exit(node) => {
                    let GeometryBody::GeometryCollection(members) = &node.body else {
                        unreachable!("only a collection is exited")
                    };
                    let first = answers
                        .len()
                        .checked_sub(members.len())
                        .expect("every scheduled member has an answer");
                    let mut of_members = Vec::new();
                    memory
                        .reserve(&mut of_members, members.len())
                        .map_err(FoldError::Storage)?;
                    of_members.extend(answers.drain(first..));
                    memory
                        .reserve_for_push(&mut answers)
                        .map_err(FoldError::Storage)?;
                    answers.push(collection(node, of_members, memory).map_err(FoldError::Visitor)?);
                }
            }
        }
        let answer = answers.pop().expect("the root's answer is assembled last");
        memory.release_vec(steps).map_err(FoldError::Storage)?;
        memory.release_vec(answers).map_err(FoldError::Storage)?;
        Ok(answer)
    })
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
        let mut resident = Resident;
        self.copy_observed(&mut Memory::new(&mut resident), None)
            .expect("resident geometry copy allocation")
    }
}

impl Geometry {
    /// Destroy a native carrier's original arrays before refunding them. Scalar
    /// ordinates retain their independent numerical owners and release those
    /// owners through their own Drop. This counts only destruction layouts;
    /// it never admits a resident carrier after construction.
    ///
    /// # Errors
    /// Preserves checked layout, traversal and original refund refusal.
    pub fn release_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let bytes = try_fold_with_memory(
            &self,
            memory,
            |node, _| leaf_array_bytes(&node.body),
            |node, children, memory| {
                let GeometryBody::GeometryCollection(members) = &node.body else {
                    unreachable!("only a collection is assembled")
                };
                let bytes = children
                    .iter()
                    .try_fold(array_bytes::<Self>(members.capacity())?, |total, bytes| {
                        total.checked_add(*bytes).ok_or(StorageError::SizeOverflow)
                    })?;
                memory.release_vec(children)?;
                Ok(bytes)
            },
        )
        .map_err(|error| match error {
            FoldError::Visitor(error) | FoldError::Storage(error) => error,
        })?;
        drop(self);
        memory.release_bytes(bytes)
    }

    /// Copy the same complete geometry through its original admitted array
    /// factories and reusable numeric destinations. All collection dimensions
    /// and written member/position orders are preserved.
    ///
    /// # Errors
    /// Preserves exact layout, allocator, admission and numeric capacity errors.
    pub fn copy_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
        scratch: &purrdf_xsd::integer::LimbScratch,
    ) -> Result<Self, crate::GeoError> {
        self.copy_observed(memory, Some(scratch))
    }

    fn copy_observed<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
        scratch: Option<&purrdf_xsd::integer::LimbScratch>,
    ) -> Result<Self, crate::GeoError> {
        self.copy_with_memory_using(memory, &mut |value| match scratch {
            Some(scratch) => value
                .copy_in(scratch)
                .map_err(crate::GeoError::NumericalScratch),
            None => Ok(value.clone()),
        })
    }

    /// The original tree clone with scalar destinations selected by its caller.
    pub(crate) fn copy_with_memory_using<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
        copy: &mut impl FnMut(&Coord) -> Result<Coord, crate::GeoError>,
    ) -> Result<Self, crate::GeoError> {
        let mut leaf = |node: &Self, memory: &mut Memory<'_, S>| {
            Ok(Self {
                dim: node.dim,
                body: copy_leaf(&node.body, memory, copy)?,
            })
        };
        if !has_members(self) {
            return memory.try_scope(|memory| leaf(self, memory));
        }
        try_fold_with_memory(self, memory, leaf, |node, copies, _| {
            Ok(Self {
                dim: node.dim,
                body: GeometryBody::GeometryCollection(copies),
            })
        })
        .map_err(FoldError::into_geo_error)
    }
}

fn array_bytes<T>(capacity: usize) -> Result<usize, StorageError> {
    core::alloc::Layout::array::<T>(capacity)
        .map(|layout| layout.size())
        .map_err(|_| StorageError::SizeOverflow)
}

fn leaf_array_bytes(body: &GeometryBody) -> Result<usize, StorageError> {
    let sequence = |values: &Vec<Coord>| array_bytes::<Coord>(values.capacity());
    let rings = |values: &Vec<super::CoordSeq>| {
        values.iter().try_fold(
            array_bytes::<super::CoordSeq>(values.capacity())?,
            |total, values| {
                total
                    .checked_add(sequence(values)?)
                    .ok_or(StorageError::SizeOverflow)
            },
        )
    };
    match body {
        GeometryBody::Point(_) => Ok(0),
        GeometryBody::LineString(values) => sequence(values),
        GeometryBody::Polygon(values) | GeometryBody::MultiLineString(values) => rings(values),
        GeometryBody::MultiPoint(values) => array_bytes::<Option<Coord>>(values.capacity()),
        GeometryBody::MultiPolygon(values) => values.iter().try_fold(
            array_bytes::<Vec<super::CoordSeq>>(values.capacity())?,
            |total, values| {
                total
                    .checked_add(rings(values)?)
                    .ok_or(StorageError::SizeOverflow)
            },
        ),
        GeometryBody::GeometryCollection(values) => {
            debug_assert_eq!(values.as_slice(), []);
            array_bytes::<Geometry>(values.capacity())
        }
    }
}

fn copy_leaf<S: Admission + ?Sized>(
    body: &GeometryBody,
    memory: &mut Memory<'_, S>,
    copy: &mut impl FnMut(&Coord) -> Result<Coord, crate::GeoError>,
) -> Result<GeometryBody, crate::GeoError> {
    Ok(match body {
        GeometryBody::Point(point) => {
            GeometryBody::Point(point.as_ref().map(&mut *copy).transpose()?)
        }
        GeometryBody::LineString(values) => {
            GeometryBody::LineString(copy_sequence_with_memory(values, memory, copy)?)
        }
        GeometryBody::Polygon(values) => {
            GeometryBody::Polygon(copy_rings_with_memory(values, memory, copy)?)
        }
        GeometryBody::MultiPoint(values) => {
            GeometryBody::MultiPoint(memory.try_map(values, |value, _| {
                value.as_ref().map(&mut *copy).transpose()
            })?)
        }
        GeometryBody::MultiLineString(values) => {
            GeometryBody::MultiLineString(copy_rings_with_memory(values, memory, copy)?)
        }
        GeometryBody::MultiPolygon(values) => {
            GeometryBody::MultiPolygon(memory.try_map(values, |value, memory| {
                copy_rings_with_memory(value, memory, copy)
            })?)
        }
        GeometryBody::GeometryCollection(members) => {
            debug_assert_eq!(members.as_slice(), []);
            GeometryBody::GeometryCollection(Vec::new())
        }
    })
}

pub(crate) fn copy_sequence_with_memory<S: Admission + ?Sized>(
    values: &[Coord],
    memory: &mut Memory<'_, S>,
    copy: &mut impl FnMut(&Coord) -> Result<Coord, crate::GeoError>,
) -> Result<super::CoordSeq, crate::GeoError> {
    memory.try_map(values, |value, _| copy(value))
}

pub(crate) fn copy_rings_with_memory<S: Admission + ?Sized>(
    values: &[super::CoordSeq],
    memory: &mut Memory<'_, S>,
    copy: &mut impl FnMut(&Coord) -> Result<Coord, crate::GeoError>,
) -> Result<Vec<super::CoordSeq>, crate::GeoError> {
    memory.try_map(values, |value, memory| {
        copy_sequence_with_memory(value, memory, copy)
    })
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
    /// Original leaf traversal shared by every geometry visitor.
    leaves: Leaves<'a>,
    /// The positions of the geometry being visited, when it holds positions itself.
    leaf: Option<LeafCoords<'a>>,
}

/// Non-collection geometries in written order. This is the one pending-node
/// body used by leaf visitors and coordinate iterators alike.
struct Leaves<'a> {
    pending: Vec<&'a Geometry>,
}

impl<'a> Leaves<'a> {
    fn new_with_memory<S: Admission + ?Sized>(
        root: &'a Geometry,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let mut pending = Vec::new();
        memory.push(&mut pending, root)?;
        Ok(Self { pending })
    }

    fn next_with_memory<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<&'a Geometry>, StorageError> {
        while let Some(geometry) = self.pending.pop() {
            if let GeometryBody::GeometryCollection(members) = &geometry.body {
                for member in members.iter().rev() {
                    memory.push(&mut self.pending, member)?;
                }
            } else {
                return Ok(Some(geometry));
            }
        }
        Ok(None)
    }
}

/// The finite set of borrowed leaf iterators. They require no boxed iterator
/// or ownership control; only the original pending geometry array can grow.
pub(crate) enum LeafCoords<'a> {
    Point(core::option::Iter<'a, Coord>),
    Sequence(core::slice::Iter<'a, Coord>),
    Rings(core::iter::Flatten<core::slice::Iter<'a, super::CoordSeq>>),
    Points(core::iter::Flatten<core::slice::Iter<'a, Option<Coord>>>),
    Polygons(core::iter::Flatten<core::iter::Flatten<core::slice::Iter<'a, super::Rings>>>),
}

impl<'a> LeafCoords<'a> {
    pub(crate) fn of(geometry: &'a Geometry) -> Self {
        match &geometry.body {
            GeometryBody::GeometryCollection(_) => unreachable!("only leaves hold positions"),
            GeometryBody::Point(point) => Self::Point(point.iter()),
            GeometryBody::LineString(coords) => Self::Sequence(coords.iter()),
            GeometryBody::Polygon(rings) | GeometryBody::MultiLineString(rings) => {
                Self::Rings(rings.iter().flatten())
            }
            GeometryBody::MultiPoint(points) => Self::Points(points.iter().flatten()),
            GeometryBody::MultiPolygon(polygons) => {
                Self::Polygons(polygons.iter().flatten().flatten())
            }
        }
    }
}

impl<'a> Iterator for LeafCoords<'a> {
    type Item = &'a Coord;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Point(iter) => iter.next(),
            Self::Sequence(iter) => iter.next(),
            Self::Rings(iter) => iter.next(),
            Self::Points(iter) => iter.next(),
            Self::Polygons(iter) => iter.next(),
        }
    }
}

impl<'a> Coords<'a> {
    pub(super) fn new(root: &'a Geometry) -> Self {
        let mut resident = Resident;
        Self::new_with_memory(root, &mut Memory::new(&mut resident))
            .expect("resident coordinate walk allocation")
    }

    fn new_with_memory<S: Admission + ?Sized>(
        root: &'a Geometry,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        Ok(Self {
            leaves: Leaves::new_with_memory(root, memory)?,
            leaf: None,
        })
    }

    fn next_with_memory<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<&'a Coord>, StorageError> {
        loop {
            if let Some(coord) = self.leaf.as_mut().and_then(Iterator::next) {
                return Ok(Some(coord));
            }
            self.leaf = None;
            let Some(geometry) = self.leaves.next_with_memory(memory)? else {
                return Ok(None);
            };
            self.leaf = Some(LeafCoords::of(geometry));
        }
    }
}

impl<'a> Iterator for Coords<'a> {
    type Item = &'a Coord;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = core::alloc::Layout::array::<&Geometry>(self.leaves.pending.capacity())
            .expect("original coordinate work-list layout")
            .size();
        let mut resident = Resident;
        self.next_with_memory(&mut Memory::resume(&mut resident, bytes))
            .expect("resident coordinate walk allocation")
    }
}

/// Continue the original native iterator between work callbacks, ending its
/// storage borrow before each callback can poll or use the numerical arena.
pub(crate) fn visit_coords_with_progress<'a, 'observer>(
    root: &'a Geometry,
    progress: &mut crate::context::WorkProgress<'observer>,
    visit: impl FnMut(
        &'a Coord,
        &mut crate::context::WorkProgress<'observer>,
    ) -> Result<(), crate::GeoError>,
) -> Result<(), crate::GeoError> {
    let coords = progress
        .with_storage(0, |memory| {
            memory.try_scope(|memory| Coords::new_with_memory(root, memory))
        })
        .map_err(crate::GeoError::Storage)?;
    visit_with_progress(coords, progress, visit)
}

/// The two native iterators share one account continuation and cleanup law.
trait ProgressWalk<'a>: Sized {
    type Item;
    fn pending(&self) -> &Vec<&'a Geometry>;
    fn into_pending(self) -> Vec<&'a Geometry>;
    fn next_admitted<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<Self::Item>, StorageError>;
}

impl<'a> ProgressWalk<'a> for Leaves<'a> {
    type Item = &'a Geometry;
    fn pending(&self) -> &Vec<&'a Geometry> {
        &self.pending
    }
    fn into_pending(self) -> Vec<&'a Geometry> {
        self.pending
    }
    fn next_admitted<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<Self::Item>, StorageError> {
        self.next_with_memory(memory)
    }
}

impl<'a> ProgressWalk<'a> for Coords<'a> {
    type Item = &'a Coord;
    fn pending(&self) -> &Vec<&'a Geometry> {
        &self.leaves.pending
    }
    fn into_pending(self) -> Vec<&'a Geometry> {
        self.leaves.pending
    }
    fn next_admitted<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<Self::Item>, StorageError> {
        self.next_with_memory(memory)
    }
}

fn visit_with_progress<'a, 'observer, W: ProgressWalk<'a>>(
    mut walk: W,
    progress: &mut crate::context::WorkProgress<'observer>,
    mut visit: impl FnMut(
        W::Item,
        &mut crate::context::WorkProgress<'observer>,
    ) -> Result<(), crate::GeoError>,
) -> Result<(), crate::GeoError> {
    let result = (|| {
        loop {
            let bytes = core::alloc::Layout::array::<&Geometry>(walk.pending().capacity())
                .map_err(|_| crate::GeoError::Storage(StorageError::SizeOverflow))?
                .size();
            let next = progress
                .with_storage(bytes, |memory| walk.next_admitted(memory))
                .map_err(crate::GeoError::Storage)?;
            let Some(coordinate) = next else {
                return Ok(());
            };
            visit(coordinate, progress)?;
        }
    })();
    let bytes = core::alloc::Layout::array::<&Geometry>(walk.pending().capacity())
        .map_err(|_| crate::GeoError::Storage(StorageError::SizeOverflow))?
        .size();
    let released = progress
        .with_storage(bytes, |memory| memory.release_vec(walk.into_pending()))
        .map_err(crate::GeoError::Storage);
    progress.settle_result(result, released)
}

/// The native coordinate iterator body, with its transient work list enclosed
/// by the original caller account. Early break and failure both destroy the
/// work list before returning its actual array layout.
pub(crate) fn visit_coords_with_memory<'a, S: Admission + ?Sized, B>(
    root: &'a Geometry,
    memory: &mut Memory<'_, S>,
    mut visit: impl FnMut(&'a Coord) -> core::ops::ControlFlow<B>,
) -> Result<core::ops::ControlFlow<B>, StorageError> {
    visit_coords_with_memory_using(root, memory, |coordinate, _| visit(coordinate))
}

/// Let the original coordinate visitor grow its own admitted metadata while
/// the iterator has ended its transient borrow of the same account.
pub(crate) fn visit_coords_with_memory_using<'a, S: Admission + ?Sized, B>(
    root: &'a Geometry,
    memory: &mut Memory<'_, S>,
    mut visit: impl FnMut(&'a Coord, &mut Memory<'_, S>) -> core::ops::ControlFlow<B>,
) -> Result<core::ops::ControlFlow<B>, StorageError> {
    memory.try_scope(|memory| {
        let mut coords = Coords::new_with_memory(root, memory)?;
        while let Some(coord) = coords.next_with_memory(memory)? {
            if let core::ops::ControlFlow::Break(answer) = visit(coord, memory) {
                memory.release_vec(coords.leaves.pending)?;
                return Ok(core::ops::ControlFlow::Break(answer));
            }
        }
        memory.release_vec(coords.leaves.pending)?;
        Ok(core::ops::ControlFlow::Continue(()))
    })
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

        purrdf_stack::on_stack(arbitrary::SMALL_STACK, move || {
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
        })
        .expect("the thread starts");
    }
}
