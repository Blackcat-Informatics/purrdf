// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Pointwise information, with unspecified claims outside the standpoint order.

use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::{Information, StandpointError, StandpointOrder};

/// The context of a polarity witness, independent of its scenario frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StandpointKey {
    /// An unspecified claim; never an implicit member of the declared order.
    Unscoped,
    /// A caller-declared standpoint, identified by canonical content.
    Declared(Digest32),
}

/// A canonical finite override map over a uniform information value.
///
/// Combining maps applies FOUR pointwise. An ordinary supported row has no
/// heap storage. A scoped claim uses `Neither` as its fallback and explicit
/// entries for the selected standpoints which inherit it. Unspecified claims
/// use an explicit `Unscoped` entry and do not acquire declared memberships.
/// The enclosing native owner retains the original admission until destruction.
#[derive(Debug, PartialEq, Eq)]
pub struct InformationMap {
    fallback: Information,
    entries: Vec<(StandpointKey, Information)>,
}

impl Default for InformationMap {
    fn default() -> Self {
        Self::uniform(Information::Supported)
    }
}

impl InformationMap {
    /// One information value everywhere, without a per-standpoint allocation.
    #[must_use]
    pub const fn uniform(fallback: Information) -> Self {
        Self {
            fallback,
            entries: Vec::new(),
        }
    }

    /// The value at every context without an explicit override.
    #[must_use]
    pub const fn fallback(&self) -> Information {
        self.fallback
    }

    /// Borrow canonical overrides, sorted by context with no fallback entries.
    #[must_use]
    pub fn entries(&self) -> &[(StandpointKey, Information)] {
        &self.entries
    }

    /// Information at one context; no standpoint inheritance is inferred here.
    #[must_use]
    pub fn get(&self, standpoint: StandpointKey) -> Information {
        self.entries
            .binary_search_by_key(&standpoint, |entry| entry.0)
            .map_or(self.fallback, |index| self.entries[index].1)
    }

    /// Extend explicit witnesses to the standpoints which sharpen their source.
    /// Unscoped witnesses stay outside the declared order. This operation does
    /// not change the uniform fallback or invent an undeclared standpoint.
    ///
    /// # Errors
    /// Refuses undeclared witnesses and original native traversal/storage errors.
    pub fn inherited<S: Admission + ?Sized>(
        &self,
        order: &StandpointOrder<Digest32>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StandpointError<Digest32>> {
        for &(key, _) in &self.entries {
            if let StandpointKey::Declared(key) = key {
                order.sharpens(key, key, memory)?;
            }
        }
        memory.try_scope(|memory| {
            let values = order.inherit_values(
                |key| {
                    self.entries
                        .binary_search_by_key(&StandpointKey::Declared(key), |entry| entry.0)
                        .map_or(Information::Neither, |index| self.entries[index].1)
                },
                Information::information_join,
                memory,
            )?;
            let mut entries = Vec::new();
            if let Ok(index) = self
                .entries
                .binary_search_by_key(&StandpointKey::Unscoped, |entry| entry.0)
            {
                memory.push(&mut entries, self.entries[index])?;
            }
            for (&selected, &inherited) in order.declared().iter().zip(&values) {
                let value = self
                    .get(StandpointKey::Declared(selected))
                    .information_join(inherited);
                if value != self.fallback {
                    memory.push(&mut entries, (StandpointKey::Declared(selected), value))?;
                }
            }
            memory.release_vec(values)?;
            Ok(Self {
                fallback: self.fallback,
                entries,
            })
        })
    }

    /// Canonicalize explicit witnesses under their original native admission.
    ///
    /// Repeated entries combine in information order, independently of input
    /// ordering. Overrides equal to the fallback are removed after coalescing.
    ///
    /// # Errors
    /// Returns physical layout, admission or allocation refusal.
    pub fn from_entries<S: Admission + ?Sized>(
        fallback: Information,
        entries: &[(StandpointKey, Information)],
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        if entries.is_empty() {
            return Ok(Self::uniform(fallback));
        }
        memory.scope(|memory| {
            let mut entries = memory.collect(entries.iter().copied())?;
            entries.sort_unstable_by_key(|entry| entry.0);
            let mut written = 0;
            for read in 0..entries.len() {
                let entry = entries[read];
                if written != 0 && entries[written - 1].0 == entry.0 {
                    entries[written - 1].1 = entries[written - 1].1.information_join(entry.1);
                } else {
                    entries[written] = entry;
                    written += 1;
                }
            }
            entries.truncate(written);
            entries.retain(|entry| entry.1 != fallback);
            if entries.is_empty() {
                memory.release_vec(entries)?;
                Ok(Self::uniform(fallback))
            } else {
                Ok(Self { fallback, entries })
            }
        })
    }

    /// Conjunctive premises use truth-order meet at each context.
    ///
    /// # Errors
    /// Returns native storage refusal or physical cardinality overflow.
    pub fn truth_meet<S: Admission + ?Sized>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        self.combine(other, Information::truth_meet, memory)
    }

    /// Coalesced alternative witnesses use information-order join per context.
    ///
    /// # Errors
    /// Returns native storage refusal or physical cardinality overflow.
    pub fn information_join<S: Admission + ?Sized>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        self.combine(other, Information::information_join, memory)
    }

    fn combine<S: Admission + ?Sized>(
        &self,
        other: &Self,
        law: fn(Information, Information) -> Information,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let fallback = law(self.fallback, other.fallback);
        if self.entries.is_empty() && other.entries.is_empty() {
            return Ok(Self::uniform(fallback));
        }
        memory.scope(|memory| {
            let mut entries = Vec::new();
            let mut left = self.entries.iter().peekable();
            let mut right = other.entries.iter().peekable();
            while left.peek().is_some() || right.peek().is_some() {
                let key = match (left.peek(), right.peek()) {
                    (Some(a), Some(b)) => a.0.min(b.0),
                    (Some(a), None) => a.0,
                    (None, Some(b)) => b.0,
                    (None, None) => break,
                };
                let a = if left.peek().is_some_and(|entry| entry.0 == key) {
                    left.next().map_or(self.fallback, |entry| entry.1)
                } else {
                    self.fallback
                };
                let b = if right.peek().is_some_and(|entry| entry.0 == key) {
                    right.next().map_or(other.fallback, |entry| entry.1)
                } else {
                    other.fallback
                };
                let value = law(a, b);
                if value != fallback {
                    memory.push(&mut entries, (key, value))?;
                }
            }
            Ok(Self { fallback, entries })
        })
    }

    /// Swap support and opposition pointwise, preserving context membership.
    ///
    /// # Errors
    /// Returns native storage refusal.
    pub fn negate<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let fallback = self.fallback.negate();
        memory.scope(|memory| {
            let entries = memory.collect(
                self.entries
                    .iter()
                    .map(|&(key, value)| (key, value.negate())),
            )?;
            Ok(Self { fallback, entries })
        })
    }

    /// Destroy the original override array before refunding its native grant.
    ///
    /// # Errors
    /// Returns the caller's typed refund refusal marker.
    pub fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_vec(self.entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Resident;

    #[test]
    fn inherited_diamond_matches_reachability_without_propagating_uniform_fallback() {
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        let keys = [1, 2, 3, 4, 5].map(|byte| Digest32::new([byte; 32]));
        // Numeric identity order deliberately differs from dependency order.
        let order = StandpointOrder::new(
            &keys,
            &[
                (keys[0], keys[2]),
                (keys[0], keys[3]),
                (keys[2], keys[1]),
                (keys[3], keys[1]),
            ],
            None,
            &mut memory,
        )
        .unwrap();
        for fallback in [
            Information::Neither,
            Information::Supported,
            Information::Opposed,
            Information::Both,
        ] {
            let source = InformationMap::from_entries(
                fallback,
                &[
                    (StandpointKey::Declared(keys[1]), Information::Opposed),
                    (StandpointKey::Declared(keys[2]), Information::Supported),
                    (StandpointKey::Unscoped, Information::Opposed),
                ],
                &mut memory,
            )
            .unwrap();
            let inherited = source.inherited(&order, &mut memory).unwrap();
            for &selected in &keys {
                let mut expected = source.get(StandpointKey::Declared(selected));
                for &(key, witness) in source.entries() {
                    if let StandpointKey::Declared(broader) = key
                        && order.sharpens(selected, broader, &mut memory).unwrap()
                    {
                        expected = expected.information_join(witness);
                    }
                }
                assert_eq!(inherited.get(StandpointKey::Declared(selected)), expected);
            }
            assert_eq!(
                inherited.get(StandpointKey::Unscoped),
                source.get(StandpointKey::Unscoped)
            );
            assert_eq!(inherited.get(StandpointKey::Declared(keys[4])), fallback);
            source.release(&mut memory).unwrap();
            inherited.release(&mut memory).unwrap();
        }
        order.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn context_polarities_combine_without_unscoped_inheritance() {
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        let declared = StandpointKey::Declared(Digest32::new([7; 32]));
        let left = InformationMap::from_entries(
            Information::Neither,
            &[
                (declared, Information::Supported),
                (StandpointKey::Unscoped, Information::Opposed),
            ],
            &mut memory,
        )
        .unwrap();
        let right = InformationMap::from_entries(
            Information::Neither,
            &[(declared, Information::Opposed)],
            &mut memory,
        )
        .unwrap();
        let joined = left.information_join(&right, &mut memory).unwrap();
        let met = left.truth_meet(&right, &mut memory).unwrap();
        assert_eq!(joined.get(declared), Information::Both);
        assert_eq!(met.get(declared), Information::Opposed);
        assert_eq!(right.get(StandpointKey::Unscoped), Information::Neither);
        assert_eq!(joined.get(StandpointKey::Unscoped), Information::Opposed);
        let negated = joined.negate(&mut memory).unwrap();
        assert_eq!(negated.get(declared), Information::Both);
        assert_eq!(negated.get(StandpointKey::Unscoped), Information::Supported);
        for map in [left, right, joined, met, negated] {
            map.release(&mut memory).unwrap();
        }
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn canonical_witness_coalescing_is_order_independent() {
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        let key = StandpointKey::Unscoped;
        let first = InformationMap::from_entries(
            Information::Neither,
            &[(key, Information::Supported), (key, Information::Opposed)],
            &mut memory,
        )
        .unwrap();
        let second = InformationMap::from_entries(
            Information::Neither,
            &[(key, Information::Opposed), (key, Information::Supported)],
            &mut memory,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.get(key), Information::Both);
        first.release(&mut memory).unwrap();
        second.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn uniform_laws_work_with_zero_admission() {
        struct Refuse;
        impl Admission for Refuse {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes == 0 {
                    Ok(())
                } else {
                    Err(StorageError::AdmissionFailed)
                }
            }
        }
        let mut admission = Refuse;
        let mut memory = Memory::new(&mut admission);
        let ordinary = InformationMap::default();
        let opposed = InformationMap::uniform(Information::Opposed);
        assert_eq!(
            ordinary
                .truth_meet(&opposed, &mut memory)
                .unwrap()
                .fallback(),
            Information::Opposed
        );
        assert_eq!(
            ordinary
                .information_join(&opposed, &mut memory)
                .unwrap()
                .fallback(),
            Information::Both
        );
        assert_eq!(
            ordinary.negate(&mut memory).unwrap().fallback(),
            Information::Opposed
        );
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
