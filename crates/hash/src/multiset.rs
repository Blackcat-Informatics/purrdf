// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reversible accounting for multisets of 256-bit member digests.
//!
//! Members add in Z/(2^256), with an independent exact 128-bit population.
//! Order does not matter; repeated members contribute repeatedly. The sum is an
//! accounting value for drift and population checks, not an identity: a party
//! choosing members can build two different multisets with one sum and
//! population, and it is no membership or authentication proof. A caller must
//! establish membership before subtraction and obtain the observed source state
//! independently of a proposed delta. An identity over a row set is
//! [`sorted_fingerprint`].

use crate::{Domain, blake3, frame::frame_le_with};

/// A refused population change. The original accumulator remains unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultisetError {
    /// The combined population exceeds the exact counter's width.
    Overflow {
        /// Population before the operation.
        before: u128,
        /// Population being added.
        added: u128,
    },
    /// More members were removed than the accumulator contains.
    Underflow {
        /// Population before the operation.
        before: u128,
        /// Population being removed.
        removed: u128,
    },
}

impl crate::error::StructuredError for MultisetError {
    crate::error_visit! {
        Overflow { before, added } => "hash-multiset-overflow" {
            "before" => Unsigned(*before), "added" => Unsigned(*added),
        },
        Underflow { before, removed } => "hash-multiset-underflow" {
            "before" => Unsigned(*before), "removed" => Unsigned(*removed),
        },
    }
}

impl core::fmt::Display for MultisetError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Overflow { before, added } => {
                write!(out, "multiset population overflow: {before} plus {added}")
            }
            Self::Underflow { before, removed } => {
                write!(
                    out,
                    "multiset population underflow: {before} minus {removed}"
                )
            }
        }
    }
}
impl std::error::Error for MultisetError {}

/// A little-endian sum of member digests and its exact population.
///
/// Only the population can overflow: the sum deliberately wraps modulo 2^256.
/// No operation allocates, and every operation has constant work in this width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Multiset256 {
    sum: [u8; 32],
    population: u128,
}

impl Multiset256 {
    /// Adopt an encoded mathematical state. Protocol readers must separately
    /// validate the population and sum against their original source records.
    #[must_use]
    pub const fn from_parts(sum: [u8; 32], population: u128) -> Self {
        Self { sum, population }
    }

    /// The exact member population.
    #[must_use]
    pub const fn population(&self) -> u128 {
        self.population
    }

    /// The raw sum, as 32 little-endian bytes, for a specified wire encoding.
    #[must_use]
    pub const fn sum(&self) -> &[u8; 32] {
        &self.sum
    }

    /// Add one already-hashed member, preserving multiplicity.
    ///
    /// # Errors
    /// Returns [`MultisetError::Overflow`] before changing either field.
    pub fn insert(&mut self, member: [u8; 32]) -> Result<(), MultisetError> {
        self.combine(Self::from_parts(member, 1))
    }

    /// Remove one member whose membership the caller established independently.
    /// A nonempty population alone does not establish that membership.
    ///
    /// # Errors
    /// Returns [`MultisetError::Underflow`] before changing either field.
    pub fn remove(&mut self, member: [u8; 32]) -> Result<(), MultisetError> {
        self.subtract(Self::from_parts(member, 1))
    }

    /// Combine two independently observed multisets without visiting their members.
    ///
    /// # Errors
    /// Returns [`MultisetError::Overflow`] before changing either field.
    pub fn combine(&mut self, other: Self) -> Result<(), MultisetError> {
        let population =
            self.population
                .checked_add(other.population)
                .ok_or(MultisetError::Overflow {
                    before: self.population,
                    added: other.population,
                })?;
        let mut carry = 0_u16;
        for (sum, byte) in self.sum.iter_mut().zip(other.sum) {
            let total = u16::from(*sum) + u16::from(byte) + carry;
            *sum = total as u8;
            carry = total >> 8;
        }
        self.population = population;
        Ok(())
    }

    /// Subtract a multiset whose containment the caller established independently.
    ///
    /// # Errors
    /// Returns [`MultisetError::Underflow`] before changing either field.
    pub fn subtract(&mut self, other: Self) -> Result<(), MultisetError> {
        let population =
            self.population
                .checked_sub(other.population)
                .ok_or(MultisetError::Underflow {
                    before: self.population,
                    removed: other.population,
                })?;
        let mut borrow = 0_u16;
        for (sum, byte) in self.sum.iter_mut().zip(other.sum) {
            let subtrahend = u16::from(byte) + borrow;
            let difference = u16::from(*sum).wrapping_sub(subtrahend);
            borrow = u16::from(u16::from(*sum) < subtrahend);
            *sum = difference as u8;
        }
        self.population = population;
        Ok(())
    }

    /// BLAKE3 of the framed domain, 16-byte little-endian population, then
    /// the 32-byte little-endian sum. Every family supplies its explicit domain.
    ///
    /// This is additive accounting over the population and the wrapping sum,
    /// not the identity of a row set: a party choosing the members can build
    /// two different multisets with one check. [`sorted_fingerprint`] is the
    /// row-set identity.
    #[must_use]
    pub fn accounting_check(&self, domain: Domain) -> [u8; 32] {
        let mut digest = blake3::Hasher::new();
        frame_le_with(domain.as_bytes(), &mut |part| {
            digest.update(part);
        });
        digest.update(&self.population.to_le_bytes());
        digest.update(&self.sum);
        *digest.finalize().as_bytes()
    }
}

/// BLAKE3 of the framed domain, every member digest in ascending order, then
/// the exact member count as 16 little-endian bytes.
///
/// This is the identity of a multiset of rows: the hash commits to the sorted
/// sequence itself, so two different multisets share a fingerprint only through
/// a BLAKE3 collision. The additive [`Multiset256`] sum is no such identity: a
/// party choosing the members can construct two different multisets with equal
/// sums and populations. Every row family supplies its own domain. The caller
/// supplies the digests already sorted, where it visits every row; a debug
/// build refuses an unsorted sequence.
///
/// # Panics
/// Panics, in a debug build only, when `members` is not in ascending order.
#[must_use]
pub fn sorted_fingerprint(domain: Domain, members: impl IntoIterator<Item = [u8; 32]>) -> [u8; 32] {
    let mut digest = blake3::Hasher::new();
    frame_le_with(domain.as_bytes(), &mut |part| {
        digest.update(part);
    });
    let mut population = 0_u128;
    let mut previous: Option<[u8; 32]> = None;
    for member in members {
        debug_assert!(
            previous.is_none_or(|before| before <= member),
            "sorted_fingerprint members must be in ascending order"
        );
        previous = Some(member);
        digest.update(&member);
        population += 1;
    }
    digest.update(&population.to_le_bytes());
    *digest.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carry_and_borrow_wrap_the_full_256_bit_sum() {
        let mut value = Multiset256::from_parts([u8::MAX; 32], 1);
        let mut one = [0; 32];
        one[0] = 1;
        value.insert(one).unwrap();
        assert_eq!(value.sum(), &[0; 32]);
        assert_eq!(value.population(), 2);
        value.remove(one).unwrap();
        assert_eq!(value, Multiset256::from_parts([u8::MAX; 32], 1));
        let mut high = [0; 32];
        high[31] = 128;
        let mut wrapped = Multiset256::default();
        wrapped.insert(high).unwrap();
        wrapped.insert(high).unwrap();
        assert_eq!(wrapped, Multiset256::from_parts([0; 32], 2));
    }

    #[test]
    fn refused_population_changes_preserve_the_original_state() {
        let mut full = Multiset256::from_parts([7; 32], u128::MAX);
        let before = full;
        assert_eq!(
            full.insert([1; 32]),
            Err(MultisetError::Overflow {
                before: u128::MAX,
                added: 1,
            })
        );
        assert_eq!(full, before);
        let mut empty = Multiset256::default();
        assert_eq!(
            empty.remove([3; 32]),
            Err(MultisetError::Underflow {
                before: 0,
                removed: 1,
            })
        );
        assert_eq!(empty, Multiset256::default());
    }

    #[test]
    fn combining_and_retracting_keeps_duplicate_members() {
        let mut left = Multiset256::default();
        left.insert([17; 32]).unwrap();
        left.insert([17; 32]).unwrap();
        let duplicate = left;
        let mut right = Multiset256::default();
        right.insert([23; 32]).unwrap();
        let single = right;
        left.combine(right).unwrap();
        right.combine(duplicate).unwrap();
        assert_eq!(left, right);
        left.subtract(single).unwrap();
        assert_eq!(left, duplicate);
        left.subtract(duplicate).unwrap();
        assert_eq!(left, Multiset256::default());
    }

    /// The row-set identity law: two multisets with equal additive sums and
    /// populations get different fingerprints, as do a member and its repeat
    /// and one multiset under two domains; the same multiset agrees.
    #[test]
    fn sorted_fingerprints_separate_additive_collisions_multiplicity_and_domains() {
        const ROWS: Domain = Domain::new(b"purrdf-hash/test/rows");
        const OTHER: Domain = Domain::new(b"purrdf-hash/test/other");
        let member = |seed: u8| *blake3::hash(&[seed]).as_bytes();
        let (a, b, c) = (member(1), member(2), member(3));
        let mut d = Multiset256::default();
        d.insert(a).unwrap();
        d.insert(b).unwrap();
        d.remove(c).unwrap();
        let d = *d.sum();
        let sorted = |mut members: Vec<[u8; 32]>| {
            members.sort_unstable();
            members
        };
        let (left, right) = (sorted(vec![a, b]), sorted(vec![c, d]));
        let additive = |members: &[[u8; 32]]| {
            let mut sum = Multiset256::default();
            for &member in members {
                sum.insert(member).unwrap();
            }
            sum
        };
        assert_eq!(additive(&left), additive(&right));
        let fingerprint =
            |domain, members: &[[u8; 32]]| sorted_fingerprint(domain, members.iter().copied());
        assert_ne!(fingerprint(ROWS, &left), fingerprint(ROWS, &right));
        assert_eq!(
            fingerprint(ROWS, &left),
            fingerprint(ROWS, &sorted(vec![b, a]))
        );
        assert_ne!(fingerprint(ROWS, &[a]), fingerprint(ROWS, &[a, a]));
        assert_ne!(fingerprint(ROWS, &left), fingerprint(OTHER, &left));
    }
}
