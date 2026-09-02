// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The DE-9IM intersection matrix, and the pattern language every topological
//! relation in GeoSPARQL is defined by.
//!
//! GeoSPARQL does not define `sfWithin`, `ehCovers` or `rcc8tpp` as algorithms.
//! It defines each of them as a **pattern over the dimensionally extended
//! nine-intersection matrix** — a 3×3 table whose entry `(r, c)` is the topological
//! dimension of the intersection of one of `a`'s three point-sets (interior,
//! boundary, exterior) with one of `b`'s. So there is exactly one thing to compute
//! ([`crate::topology::relate`]) and twenty-four things to look up, and the three
//! relation families cannot drift apart from one another because they read the
//! same matrix.
//!
//! Keeping the matrix as the single source is not merely tidy. A per-relation
//! implementation of twenty-four predicates is twenty-four chances to answer
//! `false` for the wrong reason, and a `false` from a topological predicate is
//! indistinguishable from an honest one — the exact silent-wrong-answer channel
//! this crate is built to keep closed.
//!
//! # The dimension values
//!
//! An entry is one of four values: the two sets do not meet ([`Dim::Empty`],
//! written `F`), or they meet in a set of dimension 0, 1 or 2 (a set of points, a
//! set of curves, a set of areas). [`Dim`] is ordered by that dimension with
//! `Empty` below `Zero`, which is what makes "the dimension of a union is the
//! maximum of the dimensions" a plain [`Ord::max`] and lets a matrix be
//! accumulated one contribution at a time.
//!
//! # The pattern language
//!
//! A pattern is nine characters, one per entry, read row-major from
//! `(interior, interior)`:
//!
//! | char | matches |
//! |---|---|
//! | `F` | only [`Dim::Empty`] |
//! | `T` | any of `0`, `1`, `2` — that is, anything but empty |
//! | `0` `1` `2` | exactly that dimension |
//! | `*` | anything at all |
//!
//! Several GeoSPARQL relations are the **union** of two or three patterns
//! (`sfIntersects` is four), and a few are dimension-dependent (`sfOverlaps` and
//! `sfCrosses` read a different pattern when both arguments are curves). Both are
//! expressed here as data — [`Pattern`] and a slice of them — rather than as
//! branching code, so the spec table and the implementation are the same object.

use core::fmt;

use crate::error::GeoError;

/// The topological dimension of an intersection, or its absence.
///
/// Ordered by dimension with [`Self::Empty`] below [`Self::Zero`], so that
/// "the dimension of a union is the larger of the dimensions" is [`Ord::max`]
/// and a matrix can be accumulated contribution by contribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Dim {
    /// The two sets do not meet. Written `F` in a pattern and `-1` in the
    /// literature.
    #[default]
    Empty,
    /// They meet in a set of isolated points.
    Zero,
    /// They meet in a set of curves.
    One,
    /// They meet in a set of areas.
    Two,
}

impl Dim {
    /// The character this value is written as in a matrix rendering: `F`, `0`,
    /// `1` or `2`.
    #[must_use]
    pub const fn as_char(self) -> char {
        match self {
            Self::Empty => 'F',
            Self::Zero => '0',
            Self::One => '1',
            Self::Two => '2',
        }
    }

    /// The dimension of a non-empty set of this many dimensions.
    #[must_use]
    pub const fn from_dimension(dimension: u8) -> Option<Self> {
        match dimension {
            0 => Some(Self::Zero),
            1 => Some(Self::One),
            2 => Some(Self::Two),
            _ => None,
        }
    }

    /// Whether the two sets meet at all.
    #[must_use]
    pub const fn is_present(self) -> bool {
        !matches!(self, Self::Empty)
    }
}

impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "F",
            Self::Zero => "0",
            Self::One => "1",
            Self::Two => "2",
        })
    }
}

/// Which of a geometry's three point-sets a matrix row or column names.
///
/// The discriminants are the row-major indices of [`IntersectionMatrix`], so
/// `matrix.get(Set::Interior, Set::Exterior)` and pattern position 2 are the same
/// cell by construction rather than by a lookup that could be written wrongly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Set {
    /// The interior: the geometry minus its boundary.
    Interior = 0,
    /// The boundary, as OGC Simple Features defines it per geometry kind.
    Boundary = 1,
    /// The exterior: everything in the plane that is not the geometry.
    Exterior = 2,
}

impl Set {
    /// The three sets, in matrix order.
    pub const ALL: [Self; 3] = [Self::Interior, Self::Boundary, Self::Exterior];
}

/// The nine intersection dimensions of an ordered pair of geometries.
///
/// Rows are `a`'s three sets, columns are `b`'s, both in [`Set::ALL`] order.
/// Default is the all-empty matrix, which is the correct starting point for an
/// accumulation: every entry begins as "these sets have not been shown to meet"
/// and is raised by [`Self::raise`] as evidence arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct IntersectionMatrix {
    cells: [Dim; 9],
}

impl IntersectionMatrix {
    /// The all-empty matrix.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cells: [Dim::Empty; 9],
        }
    }

    /// The dimension of `a`'s `row` set intersected with `b`'s `column` set.
    #[must_use]
    pub const fn get(&self, row: Set, column: Set) -> Dim {
        self.cells[(row as usize) * 3 + (column as usize)]
    }

    /// Set an entry outright.
    pub const fn set(&mut self, row: Set, column: Set, dim: Dim) {
        self.cells[(row as usize) * 3 + (column as usize)] = dim;
    }

    /// Raise an entry to `dim` if `dim` is larger, leaving it alone otherwise.
    ///
    /// This is the only mutator the topology computation uses on the eight
    /// non-exterior entries. Contributions arrive one labelled node or edge at a
    /// time and each one witnesses a *lower bound* on the dimension of an
    /// intersection; taking the maximum is exactly what "the dimension of a union
    /// of witnesses" means, and it makes the accumulation independent of the order
    /// the witnesses are visited in — which is what keeps the answer a pure
    /// function of the two geometries rather than of the traversal.
    pub fn raise(&mut self, row: Set, column: Set, dim: Dim) {
        let cell = &mut self.cells[(row as usize) * 3 + (column as usize)];
        if dim > *cell {
            *cell = dim;
        }
    }

    /// The nine entries in row-major order.
    #[must_use]
    pub const fn cells(&self) -> &[Dim; 9] {
        &self.cells
    }

    /// Build a matrix from a nine-character rendering (`F`, `0`, `1`, `2`).
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] if `text` is not exactly nine of those characters.
    /// This is a test and diagnostic constructor; a pattern with `T` or `*` in it
    /// is a [`Pattern`], not a matrix, and is refused here by name.
    pub fn parse(text: &str) -> Result<Self, GeoError> {
        let mut cells = [Dim::Empty; 9];
        let mut count = 0usize;
        for (index, ch) in text.chars().enumerate() {
            if index >= 9 {
                count = index + 1;
                break;
            }
            cells[index] = match ch {
                'F' => Dim::Empty,
                '0' => Dim::Zero,
                '1' => Dim::One,
                '2' => Dim::Two,
                other => {
                    return Err(GeoError::config(format!(
                        "{other:?} is not an intersection-matrix entry; a matrix is written with \
                         F, 0, 1 and 2 only ({other:?} looks like a PATTERN character, and a \
                         pattern is matched against a matrix rather than being one)"
                    )));
                }
            };
            count = index + 1;
        }
        if count == 9 {
            Ok(Self { cells })
        } else {
            Err(GeoError::config(format!(
                "an intersection matrix has exactly nine entries; got {count} in {text:?}"
            )))
        }
    }

    /// Whether this matrix satisfies `pattern`.
    #[must_use]
    pub fn matches(&self, pattern: &Pattern) -> bool {
        self.cells
            .iter()
            .zip(pattern.slots.iter())
            .all(|(&dim, &slot)| slot.accepts(dim))
    }

    /// Whether this matrix satisfies **any** of `patterns`.
    ///
    /// Several GeoSPARQL relations are a union of patterns; this is the shape
    /// that reads them, so a relation's whole definition stays one data item.
    #[must_use]
    pub fn matches_any(&self, patterns: &[Pattern]) -> bool {
        patterns.iter().any(|pattern| self.matches(pattern))
    }
}

impl fmt::Display for IntersectionMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for cell in &self.cells {
            write!(f, "{cell}")?;
        }
        Ok(())
    }
}

/// One position of a [`Pattern`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    /// `F` — the entry must be [`Dim::Empty`].
    Empty,
    /// `T` — the entry must be anything but [`Dim::Empty`].
    Present,
    /// `0`, `1` or `2` — the entry must be exactly this dimension.
    Exactly(Dim),
    /// `*` — the entry is unconstrained.
    Any,
}

impl Slot {
    /// Whether `dim` satisfies this position.
    #[must_use]
    pub fn accepts(self, dim: Dim) -> bool {
        match self {
            Self::Empty => dim == Dim::Empty,
            Self::Present => dim.is_present(),
            Self::Exactly(expected) => dim == expected,
            Self::Any => true,
        }
    }

    /// The character this slot is written as.
    #[must_use]
    pub const fn as_char(self) -> char {
        match self {
            Self::Empty => 'F',
            Self::Present => 'T',
            Self::Exactly(dim) => dim.as_char(),
            Self::Any => '*',
        }
    }
}

/// A nine-character DE-9IM pattern.
///
/// Built at compile time from the specification's own strings by
/// [`Pattern::new`], which is `const` precisely so that a mistyped pattern in the
/// relation tables is a **compile error** rather than a predicate that quietly
/// answers the wrong question at run time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pattern {
    slots: [Slot; 9],
}

impl Pattern {
    /// The pattern written as `text`.
    ///
    /// # Panics
    ///
    /// Panics if `text` is not exactly nine bytes drawn from `F`, `T`, `0`, `1`,
    /// `2`, `*`. This is a `const fn` evaluated in the relation tables' constant
    /// initializers, so the panic is a build failure at the definition site, not a
    /// run-time surprise at a call site.
    #[must_use]
    pub const fn new(text: &str) -> Self {
        let bytes = text.as_bytes();
        assert!(
            bytes.len() == 9,
            "a DE-9IM pattern has exactly nine positions"
        );
        let mut slots = [Slot::Any; 9];
        let mut index = 0;
        while index < 9 {
            slots[index] = match bytes[index] {
                b'F' => Slot::Empty,
                b'T' => Slot::Present,
                b'0' => Slot::Exactly(Dim::Zero),
                b'1' => Slot::Exactly(Dim::One),
                b'2' => Slot::Exactly(Dim::Two),
                b'*' => Slot::Any,
                _ => panic!("a DE-9IM pattern is written with F, T, 0, 1, 2 and * only"),
            };
            index += 1;
        }
        Self { slots }
    }

    /// The nine slots in row-major order.
    #[must_use]
    pub const fn slots(&self) -> &[Slot; 9] {
        &self.slots
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for slot in &self.slots {
            f.write_fmt(format_args!("{}", slot.as_char()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Dim, IntersectionMatrix, Pattern, Set, Slot};
    use crate::error::GeoError;

    #[test]
    fn dim_orders_by_dimension_with_empty_at_the_bottom() {
        assert!(Dim::Empty < Dim::Zero);
        assert!(Dim::Zero < Dim::One);
        assert!(Dim::One < Dim::Two);
        assert_eq!(Dim::Empty.max(Dim::Two), Dim::Two);
        assert!(!Dim::Empty.is_present());
        for dim in [Dim::Zero, Dim::One, Dim::Two] {
            assert!(dim.is_present());
        }
        assert_eq!(Dim::from_dimension(1), Some(Dim::One));
        assert_eq!(Dim::from_dimension(3), None);
    }

    /// The row/column enum and the pattern position must name the same cell.
    #[test]
    fn set_discriminants_are_the_row_major_indices() {
        let mut matrix = IntersectionMatrix::new();
        matrix.set(Set::Interior, Set::Exterior, Dim::Two);
        assert_eq!(matrix.to_string(), "FF2FFFFFF");
        matrix.set(Set::Exterior, Set::Interior, Dim::One);
        assert_eq!(matrix.to_string(), "FF2FFF1FF");
        assert_eq!(matrix.get(Set::Interior, Set::Exterior), Dim::Two);
        assert_eq!(matrix.get(Set::Exterior, Set::Interior), Dim::One);
    }

    /// `raise` is a maximum, so the accumulation cannot depend on visit order.
    #[test]
    fn raise_takes_the_maximum_and_is_order_independent() {
        let contributions = [Dim::Zero, Dim::Two, Dim::One, Dim::Empty];
        let mut forward = IntersectionMatrix::new();
        for dim in contributions {
            forward.raise(Set::Interior, Set::Interior, dim);
        }
        let mut backward = IntersectionMatrix::new();
        for dim in contributions.iter().rev() {
            backward.raise(Set::Interior, Set::Interior, *dim);
        }
        assert_eq!(forward, backward);
        assert_eq!(forward.get(Set::Interior, Set::Interior), Dim::Two);
        // The control: `set` is NOT a maximum, so a last-writer-wins mutator
        // would have produced a different answer for the same contributions.
        let mut overwritten = IntersectionMatrix::new();
        for dim in contributions {
            overwritten.set(Set::Interior, Set::Interior, dim);
        }
        assert_eq!(overwritten.get(Set::Interior, Set::Interior), Dim::Empty);
    }

    #[test]
    fn a_matrix_round_trips_through_its_rendering() {
        for text in ["FFFFFFFFF", "212101212", "0FFF0FFF0"] {
            let matrix = IntersectionMatrix::parse(text).expect("nine entries");
            assert_eq!(matrix.to_string(), text);
        }
    }

    /// A pattern character is refused by name in the matrix constructor, and a
    /// nine-entry matrix right beside it is still accepted.
    #[test]
    fn matrix_parse_refuses_pattern_characters_but_accepts_real_entries() {
        for bad in ["T********", "*********", "TFFFTFFFT"] {
            assert!(
                matches!(IntersectionMatrix::parse(bad), Err(GeoError::Config(_))),
                "{bad:?} is a pattern, not a matrix"
            );
        }
        for bad in ["FFFFFFFF", "FFFFFFFFFF", ""] {
            assert!(
                matches!(IntersectionMatrix::parse(bad), Err(GeoError::Config(_))),
                "{bad:?} does not have nine entries"
            );
        }
        // The neighbouring VALID cases.
        assert!(IntersectionMatrix::parse("0FFFFFFFF").is_ok());
        assert!(IntersectionMatrix::parse("212101212").is_ok());
    }

    #[test]
    fn every_slot_accepts_exactly_what_it_says() {
        assert!(Slot::Empty.accepts(Dim::Empty));
        assert!(!Slot::Empty.accepts(Dim::Zero));
        assert!(!Slot::Present.accepts(Dim::Empty));
        for dim in [Dim::Zero, Dim::One, Dim::Two] {
            assert!(Slot::Present.accepts(dim));
        }
        assert!(Slot::Exactly(Dim::One).accepts(Dim::One));
        assert!(!Slot::Exactly(Dim::One).accepts(Dim::Two));
        for dim in [Dim::Empty, Dim::Zero, Dim::One, Dim::Two] {
            assert!(Slot::Any.accepts(dim));
        }
    }

    #[test]
    fn a_pattern_round_trips_through_its_rendering() {
        for text in ["T*F**F***", "FF*FF****", "1*T***T**", "*********"] {
            assert_eq!(Pattern::new(text).to_string(), text);
        }
    }

    /// The worked example from the DE-9IM literature: a point strictly inside a
    /// polygon matches `sfWithin`'s `T*F**F***` and not `sfContains`'s
    /// `T*****FF*`.
    #[test]
    fn matching_is_positional_and_the_reverse_pattern_does_not_also_match() {
        let point_in_polygon = IntersectionMatrix::parse("0FFFFF212").expect("nine entries");
        assert!(
            point_in_polygon.matches(&Pattern::new("T*F**F***")),
            "sfWithin"
        );
        assert!(
            !point_in_polygon.matches(&Pattern::new("T*****FF*")),
            "sfContains must NOT also match — the matrix is ordered"
        );
        assert!(
            !point_in_polygon.matches(&Pattern::new("FF*FF****")),
            "sfDisjoint must not match a point inside its polygon"
        );
    }

    #[test]
    fn matches_any_is_the_union_of_its_patterns() {
        // `sfIntersects` is the union of four patterns; a matrix that satisfies
        // only the third must still be accepted.
        let intersects = [
            Pattern::new("T********"),
            Pattern::new("*T*******"),
            Pattern::new("***T*****"),
            Pattern::new("****T****"),
        ];
        let only_the_third = IntersectionMatrix::parse("FFF0FFFFF").expect("nine entries");
        assert!(!only_the_third.matches(&intersects[0]));
        assert!(only_the_third.matches_any(&intersects));

        let nothing_meets = IntersectionMatrix::parse("FFFFFFFFF").expect("nine entries");
        assert!(
            !nothing_meets.matches_any(&intersects),
            "the union must still reject a matrix no member accepts"
        );
    }
}
