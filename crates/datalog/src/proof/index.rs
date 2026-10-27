// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Requested proof extraction over the original immutable derivation inventory.

use super::{
    Arity, Evaluation, ProofArena, ProofError, ProofId, ShapeFact, StoreShape, TermInterner,
};
use crate::seminaive::Derivation;
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, Resident, StorageError};
use purrdf_lex::walk::{WorkList, try_sort_unstable_by};

/// Actual index and traversal operations, independent of elapsed time.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProofIndexWork {
    /// Derivation entries indexed during preparation or visited during extraction.
    pub derivations: u64,
    /// Authored source slots visited during extraction.
    pub source_slots: u64,
    /// Fact/order comparisons in index preparation or lookup.
    pub index_comparisons: u64,
}

impl ProofIndexWork {
    fn increment(value: &mut u64) -> Result<(), StorageError> {
        *value = value.checked_add(1).ok_or(StorageError::SizeOverflow)?;
        Ok(())
    }
}

#[derive(Debug)]
struct Entry {
    derivation: usize,
    rank: usize,
}

/// Reusable lookup over recorded derivations; preparation constructs no proof nodes.
///
/// The evaluation stays immutably borrowed. Each extraction owns only the requested
/// proof DAG and uses the same predecessor order as [`super::EvaluationProofs`].
/// Arena construction retains [`ProofArena`]'s existing resident allocation contract;
/// supplied memory admits the index and traversal scratch, not the returned arena.
#[derive(Debug)]
pub struct EvaluationProofIndex<'a, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    derivations: &'a [Derivation<N>],
    entries: Vec<Entry>,
    preparation: ProofIndexWork,
}

struct Frame {
    entry: usize,
    premises: Vec<ProofId>,
}

impl<'a, const N: usize> EvaluationProofIndex<'a, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Index one recorded evaluation without constructing its proofs.
    ///
    /// # Errors
    /// Returns disabled recording or the original native allocation refusal.
    pub fn of(evaluation: &'a Evaluation<TermInterner, N>) -> Result<Self, ProofError<N>> {
        Self::of_with_memory(evaluation, &mut Memory::new(&mut Resident))
    }

    /// Index through the caller's original memory. On success its grant remains
    /// with this index until [`Self::release_with_memory`] destroys the entries.
    /// Index release must use that same original memory owner.
    ///
    /// # Errors
    /// Returns disabled recording, original storage refusal or counter overflow.
    /// Failed construction drops its locals before refunding their grant.
    pub fn of_with_memory<A: Admission + ?Sized>(
        evaluation: &'a Evaluation<TermInterner, N>,
        memory: &mut Memory<'_, A>,
    ) -> Result<Self, ProofError<N>> {
        Self::from_derivations(
            evaluation
                .derivations()
                .ok_or(ProofError::DerivationsNotRecorded)?,
            memory,
        )
    }

    fn from_derivations<A: Admission + ?Sized>(
        derivations: &'a [Derivation<N>],
        memory: &mut Memory<'_, A>,
    ) -> Result<Self, ProofError<N>> {
        memory
            .scope(|memory| {
                let mut preparation = ProofIndexWork {
                    derivations: u64::try_from(derivations.len())
                        .map_err(|_| StorageError::SizeOverflow)?,
                    ..ProofIndexWork::default()
                };
                let mut entries =
                    memory.collect((0..derivations.len()).map(|derivation| Entry {
                        derivation,
                        rank: 0,
                    }))?;
                // The ordinal reproduces the eager stable sort's tie order without
                // allocating a second sorting buffer.
                try_sort_unstable_by(&mut entries, |left, right| {
                    ProofIndexWork::increment(&mut preparation.index_comparisons)?;
                    let a = &derivations[left.derivation];
                    let b = &derivations[right.derivation];
                    Ok::<_, StorageError>((a.proof_height(), a.fact(), left.derivation).cmp(&(
                        b.proof_height(),
                        b.fact(),
                        right.derivation,
                    )))
                })?;
                for (rank, entry) in entries.iter_mut().enumerate() {
                    entry.rank = rank;
                }
                try_sort_unstable_by(&mut entries, |left, right| {
                    ProofIndexWork::increment(&mut preparation.index_comparisons)?;
                    Ok::<_, StorageError>(
                        (derivations[left.derivation].fact(), left.rank)
                            .cmp(&(derivations[right.derivation].fact(), right.rank)),
                    )
                })?;
                Ok(Self {
                    derivations,
                    entries,
                    preparation,
                })
            })
            .map_err(ProofError::Storage)
    }

    /// The actual preparation inventory and ordering work.
    #[must_use]
    pub const fn preparation_work(&self) -> ProofIndexWork {
        self.preparation
    }

    /// Destroy index entries before returning their grant to the same memory
    /// supplied to [`Self::of_with_memory`]. Resident indexes need only normal drop.
    ///
    /// # Errors
    /// Preserves the original memory owner's release refusal.
    pub fn release_with_memory<A: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        memory.release_vec(self.entries)
    }

    /// Construct exactly one requested recorded proof DAG. An absent recorded
    /// target returns `None`; its caller decides whether it was independently given.
    /// The original checker must still validate every returned proof.
    ///
    /// # Errors
    /// Returns original arena/scratch allocation failure or work-counter overflow.
    pub fn extract(
        &self,
        goal: &ShapeFact<N>,
    ) -> Result<Option<(ProofArena<N>, ProofId)>, ProofError<N>> {
        self.extract_counted(goal, &mut ProofIndexWork::default())
    }

    /// Extract while reporting actual operations. Resets `work` before the query;
    /// a refusal leaves the completed counts available to the caller.
    ///
    /// # Errors
    /// Preserves the same original failures as [`Self::extract`].
    pub fn extract_counted(
        &self,
        goal: &ShapeFact<N>,
        work: &mut ProofIndexWork,
    ) -> Result<Option<(ProofArena<N>, ProofId)>, ProofError<N>> {
        self.extract_with_memory(goal, &mut Memory::new(&mut Resident), work)
    }

    /// Admit traversal scratch through the supplied original memory. Scratch is
    /// destroyed and released on success and failure; returned arena storage keeps
    /// its existing resident contract. Resets `work` before the query.
    ///
    /// # Errors
    /// Preserves native allocation, admission, release and counter-overflow errors.
    pub fn extract_with_memory<A: Admission + ?Sized>(
        &self,
        goal: &ShapeFact<N>,
        memory: &mut Memory<'_, A>,
        work: &mut ProofIndexWork,
    ) -> Result<Option<(ProofArena<N>, ProofId)>, ProofError<N>> {
        *work = ProofIndexWork::default();
        memory
            .scope(|memory| {
                let Some(target) = self.predecessor(goal, usize::MAX, work)? else {
                    return Ok(None);
                };
                let mut pending = WorkList::<Frame, 16>::new();
                let mut built = OrderedTable::<usize, ProofId>::default();
                let mut arena = ProofArena::default();
                pending.try_push_admitted(
                    Frame {
                        entry: target,
                        premises: Vec::new(),
                    },
                    memory,
                )?;
                ProofIndexWork::increment(&mut work.derivations)?;
                while let Some(mut frame) = pending.pop() {
                    let entry = &self.entries[frame.entry];
                    let derivation = &self.derivations[entry.derivation];
                    if let Some(source) = derivation.sources().get(frame.premises.len()) {
                        let predecessor = self.predecessor(source, entry.rank, work)?;
                        let premise = if let Some(index) = predecessor {
                            if let Some(root) = built.get(&index) {
                                *root
                            } else {
                                pending.try_push_admitted(frame, memory)?;
                                pending.try_push_admitted(
                                    Frame {
                                        entry: index,
                                        premises: Vec::new(),
                                    },
                                    memory,
                                )?;
                                ProofIndexWork::increment(&mut work.derivations)?;
                                continue;
                            }
                        } else {
                            arena.axiom(source.clone())?
                        };
                        ProofIndexWork::increment(&mut work.source_slots)?;
                        memory.push(&mut frame.premises, premise)?;
                        pending.try_push_admitted(frame, memory)?;
                    } else {
                        let root = arena.by_rule_head(
                            derivation.fact().clone(),
                            derivation.rule(),
                            derivation.head(),
                            &frame.premises,
                        )?;
                        memory.release_vec(frame.premises)?;
                        built.insert(frame.entry, root, memory)?;
                    }
                }
                // The target is the first pending frame. A frame leaves the work
                // list permanently only after publishing its entry in `built`,
                // so an empty work list establishes that the target is present.
                let root = *built
                    .get(&target)
                    .expect("INVARIANT: the completed target frame published its proof");
                built.release(memory)?;
                pending.release_admitted(memory)?;
                Ok(Some((arena, root)))
            })
            .map_err(ProofError::Storage)
    }

    // Last fact match preceding this exact eager rank. A later derivation must
    // not rescue a missing/incorrect antecedent that the eager checker rejects.
    fn predecessor(
        &self,
        goal: &ShapeFact<N>,
        before: usize,
        work: &mut ProofIndexWork,
    ) -> Result<Option<usize>, StorageError> {
        let mut low = 0;
        let mut high = self.entries.len();
        while low < high {
            let mid = low + (high - low) / 2;
            let entry = &self.entries[mid];
            ProofIndexWork::increment(&mut work.index_comparisons)?;
            if (self.derivations[entry.derivation].fact(), entry.rank) < (goal, before) {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        let Some(index) = low.checked_sub(1) else {
            return Ok(None);
        };
        ProofIndexWork::increment(&mut work.index_comparisons)?;
        Ok((self.derivations[self.entries[index].derivation].fact() == goal).then_some(index))
    }
}
