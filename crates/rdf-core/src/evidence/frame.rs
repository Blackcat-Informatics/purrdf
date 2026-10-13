// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical frame conjunctions and alternatives, independent of FOUR.

use purrdf_hash::{Domain, blake3::RecordHasher, frame::frame_le_into, hex::Digest32};
use purrdf_lex::allocation::{Admission, Memory, StorageError};

const FRAME_DOMAIN: Domain = Domain::new(b"purrdf-core/evidence-frame/v1");

/// One content-identified restriction on the world in which a witness holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FrameComponent {
    /// Membership in a caller-declared standpoint.
    Standpoint(Digest32),
    /// A counterfactual scenario, never an unrestricted actual premise.
    Counterfactual(Digest32),
    /// An imagined scenario, never an unrestricted actual premise.
    Imagined(Digest32),
    /// An explicitly assumed scenario, not an unrestricted actual premise.
    Assumed(Digest32),
    /// A caller-declared licensed falsehood, never actualized by a join.
    LicensedFalsehood(Digest32),
}

impl FrameComponent {
    pub(super) fn preimage(self) -> (u8, Digest32) {
        match self {
            Self::Standpoint(id) => (0, id),
            Self::Counterfactual(id) => (1, id),
            Self::Imagined(id) => (2, id),
            Self::Assumed(id) => (3, id),
            Self::LicensedFalsehood(id) => (4, id),
        }
    }
}

/// A normalized disjunction of conjunctive frame restrictions.
///
/// The unrestricted actual frame has no heap storage. Every scoped alternative
/// has at least one component, sorted and unique. Alternatives are sorted,
/// unique and absorption-normalized: A OR (A AND B) is A. This representation
/// preserves conjunction across alternatives without silently promoting an
/// imagined or counterfactual premise into the actual frame.
///
/// The original result owner retains its native admission through destruction.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Frame {
    alternatives: Vec<Vec<FrameComponent>>,
}

impl Frame {
    /// The allocation-free unrestricted actual frame.
    #[must_use]
    pub const fn actual() -> Self {
        Self {
            alternatives: Vec::new(),
        }
    }

    /// Whether an unrestricted actual witness establishes this frame.
    #[must_use]
    pub fn is_unrestricted_actual(&self) -> bool {
        self.alternatives.is_empty()
    }

    /// Borrow the original canonical alternatives; an empty slice means actual.
    #[must_use]
    pub fn alternatives(&self) -> &[Vec<FrameComponent>] {
        &self.alternatives
    }

    /// Construct one conjunctive frame under its original native admission.
    ///
    /// An empty conjunction is actual; input order and duplicates have no effect.
    ///
    /// # Errors
    /// Returns native admission, layout or allocator refusal.
    pub fn from_components<S: Admission + ?Sized>(
        components: &[FrameComponent],
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        if components.is_empty() {
            return Ok(Self::actual());
        }
        memory.scope(|memory| {
            let mut branch = memory.collect(components.iter().copied())?;
            branch.sort_unstable();
            branch.dedup();
            let mut alternatives = Vec::new();
            memory.push(&mut alternatives, branch)?;
            Ok(Self { alternatives })
        })
    }

    fn copy_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        memory.scope(|memory| {
            let mut alternatives = Vec::new();
            memory.reserve(&mut alternatives, self.alternatives.len())?;
            for branch in &self.alternatives {
                alternatives.push(memory.collect(branch.iter().copied())?);
            }
            Ok(Self { alternatives })
        })
    }

    /// Meet of premises: intersect standpoints and conjoin scenario restrictions.
    ///
    /// An actual premise contributes no restriction, so actual AND scenario
    /// remains scenario. The physical budget, never a semantic preset, bounds
    /// expansion of a conjunction over alternative witnesses.
    ///
    /// # Errors
    /// Returns physical cardinality overflow or native storage refusal.
    pub fn meet<S: Admission + ?Sized>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        if self.is_unrestricted_actual() {
            return other.copy_with_memory(memory);
        }
        if other.is_unrestricted_actual() {
            return self.copy_with_memory(memory);
        }
        memory.scope(|memory| {
            let count = self
                .alternatives
                .len()
                .checked_mul(other.alternatives.len())
                .ok_or(StorageError::SizeOverflow)?;
            let mut result = Self::actual();
            memory.reserve(&mut result.alternatives, count)?;
            for left in &self.alternatives {
                for right in &other.alternatives {
                    let mut branch = memory.collect(left.iter().chain(right).copied())?;
                    branch.sort_unstable();
                    branch.dedup();
                    result.alternatives.push(branch);
                }
            }
            result.normalize(memory)?;
            Ok(result)
        })
    }

    /// Alternative witnesses: an unrestricted actual witness establishes actual.
    ///
    /// # Errors
    /// Returns physical cardinality overflow or native storage refusal.
    pub fn union<S: Admission + ?Sized>(
        &self,
        other: &Self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        if self.is_unrestricted_actual() || other.is_unrestricted_actual() {
            return Ok(Self::actual());
        }
        memory.scope(|memory| {
            let count = self
                .alternatives
                .len()
                .checked_add(other.alternatives.len())
                .ok_or(StorageError::SizeOverflow)?;
            let mut result = Self::actual();
            memory.reserve(&mut result.alternatives, count)?;
            for branch in self.alternatives.iter().chain(&other.alternatives) {
                result
                    .alternatives
                    .push(memory.collect(branch.iter().copied())?);
            }
            result.normalize(memory)?;
            Ok(result)
        })
    }

    fn normalize<S: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.alternatives.sort_unstable();
        let mut index = 0;
        while index < self.alternatives.len() {
            let redundant = self.alternatives.iter().enumerate().any(|(other, branch)| {
                other != index
                    && (branch.len() < self.alternatives[index].len()
                        || (branch == &self.alternatives[index] && other < index))
                    && branch
                        .iter()
                        .all(|atom| self.alternatives[index].binary_search(atom).is_ok())
            });
            if redundant {
                memory.release_vec(self.alternatives.remove(index))?;
            } else {
                index += 1;
            }
        }
        Ok(())
    }

    /// Hash the canonical framed content, never dictionary ids or insertion order.
    #[must_use]
    pub fn identity(&self) -> Digest32 {
        Self::hash_alternatives(self.alternatives.iter().map(Vec::as_slice))
    }

    /// Identity of one original sorted conjunctive branch, without copying it.
    ///
    /// A lazy ledger uses precisely the identity of a frame consisting of this
    /// branch. The empty conjunction remains the unrestricted actual frame.
    #[must_use]
    fn conjunction_identity(components: &[FrameComponent]) -> Digest32 {
        Self::hash_alternatives((!components.is_empty()).then_some(components).into_iter())
    }

    /// Content identity of a canonical conjunctive alternative already owned here.
    #[must_use]
    pub fn alternative_identity(&self, index: usize) -> Option<Digest32> {
        self.alternatives
            .get(index)
            .map(|branch| Self::conjunction_identity(branch))
    }

    fn hash_alternatives<'a>(
        branches: impl ExactSizeIterator<Item = &'a [FrameComponent]>,
    ) -> Digest32 {
        let mut hash = RecordHasher::new();
        frame_le_into(&mut hash, FRAME_DOMAIN.as_bytes());
        frame_le_into(&mut hash, &(branches.len() as u64).to_le_bytes());
        for branch in branches {
            frame_le_into(&mut hash, &(branch.len() as u64).to_le_bytes());
            for atom in branch {
                let (kind, id) = atom.preimage();
                frame_le_into(&mut hash, &[kind]);
                frame_le_into(&mut hash, id.as_bytes());
            }
        }
        Digest32::new(*hash.finalize().as_bytes())
    }

    /// Destroy all original frame arrays before refunding their native layouts.
    ///
    /// # Errors
    /// Returns the original admission callback's refund refusal.
    pub fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let Self { mut alternatives } = self;
        for branch in &mut alternatives {
            memory.release_vec(core::mem::take(branch))?;
        }
        memory.release_vec(alternatives)
    }
}

#[cfg(test)]
mod tests {
    use super::{Frame, FrameComponent};
    use purrdf_hash::hex::Digest32;
    use purrdf_lex::allocation::{Memory, Resident};

    #[test]
    fn actual_is_conjunction_identity_but_alternative_actual_is_decisive() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let imagined = Frame::from_components(
            &[FrameComponent::Imagined(Digest32::new([1; 32]))],
            &mut memory,
        )
        .unwrap();
        let joined = Frame::actual().meet(&imagined, &mut memory).unwrap();
        assert_eq!(joined, imagined);
        assert!(!joined.is_unrestricted_actual());
        let alternatives = imagined.union(&Frame::actual(), &mut memory).unwrap();
        assert!(alternatives.is_unrestricted_actual());
        alternatives.release(&mut memory).unwrap();
        joined.release(&mut memory).unwrap();
        imagined.release(&mut memory).unwrap();
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn sorted_content_and_absorption_determine_identity() {
        let a = FrameComponent::Standpoint(Digest32::new([1; 32]));
        let b = FrameComponent::Counterfactual(Digest32::new([2; 32]));
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let left = Frame::from_components(&[a, b, a], &mut memory).unwrap();
        let right = Frame::from_components(&[b, a], &mut memory).unwrap();
        assert_eq!(left.identity(), right.identity());
        let broad = Frame::from_components(&[a], &mut memory).unwrap();
        let absorbed = broad.union(&left, &mut memory).unwrap();
        assert_eq!(absorbed.identity(), broad.identity());
        assert_ne!(left.identity(), broad.identity());
        for frame in [absorbed, broad, right, left] {
            frame.release(&mut memory).unwrap();
        }
        assert_eq!(memory.admitted_bytes(), 0);
    }

    #[test]
    fn conjunction_distributes_over_alternatives_without_actual_promotion() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut frames = Vec::new();
        for byte in 1..=3 {
            frames.push(
                Frame::from_components(
                    &[FrameComponent::Imagined(Digest32::new([byte; 32]))],
                    &mut memory,
                )
                .unwrap(),
            );
        }
        let alternatives = frames[0].union(&frames[1], &mut memory).unwrap();
        let distributed = alternatives.meet(&frames[2], &mut memory).unwrap();
        let first = frames[0].meet(&frames[2], &mut memory).unwrap();
        let second = frames[1].meet(&frames[2], &mut memory).unwrap();
        let expected = first.union(&second, &mut memory).unwrap();
        assert_eq!(distributed.identity(), expected.identity());
        assert!(!distributed.is_unrestricted_actual());
        for frame in [expected, second, first, distributed, alternatives] {
            frame.release(&mut memory).unwrap();
        }
        for frame in frames {
            frame.release(&mut memory).unwrap();
        }
        assert_eq!(memory.admitted_bytes(), 0);
    }
}
