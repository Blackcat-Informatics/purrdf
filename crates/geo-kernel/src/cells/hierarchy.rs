// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Validated sentinel keys, hierarchy operations and external-store intervals.

use crate::error::GeoError;

use super::{GridProfileId, MAX_LEVEL, validate_level};

/// A validated native cube key paired with its complete grid profile identity.
///
/// Numeric and big-endian key order agree. Disjoint subtrees occur in Hilbert
/// leaf order; an ancestor's sentinel is the midpoint of its leaf interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellId {
    profile: GridProfileId,
    key: u64,
}

impl CellId {
    /// Validate a stored sentinel key and attach its declared grid profile.
    ///
    /// # Errors
    ///
    /// Refuses faces six/seven, zero, missing sentinels and odd sentinel bits.
    pub fn from_key(profile: GridProfileId, key: u64) -> Result<Self, GeoError> {
        let face = key >> 61;
        let trailing = key.trailing_zeros();
        if face > 5 || trailing > 60 || trailing & 1 == 1 {
            return Err(GeoError::InvalidCellId(key));
        }
        Ok(Self { profile, key })
    }

    /// Construct a face root.
    ///
    /// # Errors
    ///
    /// Refuses a face outside zero through five.
    pub fn root(profile: GridProfileId, face: u8) -> Result<Self, GeoError> {
        Self::from_path(profile, face, 0, 0)
    }

    pub(super) fn from_path(
        profile: GridProfileId,
        face: u8,
        path: u64,
        level: u8,
    ) -> Result<Self, GeoError> {
        validate_level(level)?;
        if face > 5 || path >= 1_u64 << (2 * level) {
            return Err(GeoError::InvalidCellId((u64::from(face) << 61) | path));
        }
        let key = (u64::from(face) << 61) | (path << (61 - 2 * level)) | (1 << (60 - 2 * level));
        Ok(Self { profile, key })
    }

    /// The complete assignment/reference identity.
    #[must_use]
    pub const fn profile(self) -> GridProfileId {
        self.profile
    }

    /// The unsigned, hierarchy-ordered external-store key.
    #[must_use]
    pub const fn key(self) -> u64 {
        self.key
    }

    /// Face ordinal in the frozen +X,+Y,+Z,−X,−Y,−Z order.
    #[must_use]
    pub const fn face(self) -> u8 {
        (self.key >> 61) as u8
    }

    /// Level zero through thirty, recovered from the validated sentinel.
    #[must_use]
    pub const fn level(self) -> u8 {
        ((60 - self.key.trailing_zeros()) / 2) as u8
    }

    /// The key encoded for lexicographically ordered external stores.
    #[must_use]
    pub const fn to_be_bytes(self) -> [u8; 8] {
        self.key.to_be_bytes()
    }

    /// Select an ancestor, including this cell at its own level.
    ///
    /// # Errors
    ///
    /// Refuses unsupported levels and levels deeper than this cell.
    pub fn ancestor(self, level: u8) -> Result<Self, GeoError> {
        validate_level(level)?;
        if level > self.level() {
            return Err(GeoError::InvalidAncestorLevel {
                cell: self.level(),
                ancestor: level,
            });
        }
        Ok(self.at_level(level))
    }

    /// The immediate parent.
    ///
    /// # Errors
    ///
    /// A face root has no parent.
    pub fn parent(self) -> Result<Self, GeoError> {
        if self.level() == 0 {
            Err(GeoError::RootHasNoParent)
        } else {
            Ok(self.at_level(self.level() - 1))
        }
    }

    /// Visit immediate parent through face root, with no allocation.
    #[must_use]
    pub fn ancestors(self) -> Ancestors {
        Ancestors {
            next: if self.level() == 0 {
                None
            } else {
                Some(self.at_level(self.level() - 1))
            },
        }
    }

    /// Four immediate children in Hilbert order, in a fixed-size buffer.
    ///
    /// # Errors
    ///
    /// A level-thirty leaf has no children.
    pub fn children(self) -> Result<[Self; 4], GeoError> {
        if self.level() == MAX_LEVEL {
            return Err(GeoError::LeafHasNoChildren);
        }
        let quarter = self.sentinel() / 4;
        let keys = [
            self.key - 3 * quarter,
            self.key - quarter,
            self.key + quarter,
            self.key + 3 * quarter,
        ];
        Ok(keys.map(|key| Self {
            profile: self.profile,
            key,
        }))
    }

    /// The least level-thirty descendant key, inclusive.
    #[must_use]
    pub const fn range_min(self) -> u64 {
        self.key - self.sentinel() + 1
    }

    /// The greatest level-thirty descendant key, inclusive.
    #[must_use]
    pub const fn range_max(self) -> u64 {
        self.key + self.sentinel() - 1
    }

    /// An inclusive, strided range at an explicit external-store resolution.
    ///
    /// # Errors
    ///
    /// Refuses unsupported resolutions and a stored level shallower than the cell.
    pub fn descendant_range(self, stored_level: u8) -> Result<CellRange, GeoError> {
        validate_level(stored_level)?;
        if stored_level < self.level() {
            return Err(GeoError::InvalidStoredLevel {
                cell: self.level(),
                stored: stored_level,
            });
        }
        let s = self.sentinel();
        let t = 1_u64 << (60 - 2 * stored_level);
        Ok(CellRange {
            profile: self.profile,
            min: self.key - s + t,
            max: self.key + s - t,
            stride: 2 * t,
            stored_level,
            count: 1_u64 << (2 * (stored_level - self.level())),
        })
    }

    const fn sentinel(self) -> u64 {
        1_u64 << self.key.trailing_zeros()
    }

    const fn at_level(self, level: u8) -> Self {
        let s = 1_u64 << (60 - 2 * level);
        Self {
            profile: self.profile,
            key: (self.key & !(2 * s - 1)) | s,
        }
    }
}

/// Inclusive keys of a subtree at one stored resolution.
///
/// Compression does not change [`count`](Self::count): it is the logical cell
/// count, not the number of records used to represent the interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRange {
    profile: GridProfileId,
    min: u64,
    max: u64,
    stride: u64,
    stored_level: u8,
    count: u64,
}

impl CellRange {
    /// The bucket's grid profile identity.
    #[must_use]
    pub const fn profile(self) -> GridProfileId {
        self.profile
    }

    /// Inclusive least key.
    #[must_use]
    pub const fn min(self) -> u64 {
        self.min
    }

    /// Inclusive greatest key.
    #[must_use]
    pub const fn max(self) -> u64 {
        self.max
    }

    /// Difference between consecutive keys at this stored level.
    #[must_use]
    pub const fn stride(self) -> u64 {
        self.stride
    }

    /// Resolution of every represented cell.
    #[must_use]
    pub const fn stored_level(self) -> u8 {
        self.stored_level
    }

    /// Logical number of represented cells, including compressed intervals.
    #[must_use]
    pub const fn count(self) -> u64 {
        self.count
    }

    /// Membership at this range's exact resolution.
    ///
    /// # Errors
    ///
    /// Refuses comparison across grid profile identities.
    pub fn contains(self, cell: CellId) -> Result<bool, GeoError> {
        if cell.profile != self.profile {
            return Err(GeoError::GridProfileMismatch);
        }
        Ok(cell.level() == self.stored_level
            && cell.key >= self.min
            && cell.key <= self.max
            && (cell.key - self.min).is_multiple_of(self.stride))
    }
}

/// Allocation-free traversal from immediate parent through root.
#[derive(Clone, Debug)]
pub struct Ancestors {
    next: Option<CellId>,
}

impl Iterator for Ancestors {
    type Item = CellId;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.next?;
        self.next = if result.level() == 0 {
            None
        } else {
            Some(result.at_level(result.level() - 1))
        };
        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.next.map_or(0, |cell| usize::from(cell.level()) + 1);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Ancestors {}
impl core::iter::FusedIterator for Ancestors {}
