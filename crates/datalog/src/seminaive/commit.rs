// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Sorted publication retains the first unadmitted winner and its original sources.

use super::{
    BTreeSet, BudgetResource, Candidate, Derivation, DerivationMode, DictionaryError,
    DictionaryTerm, EvalError, FixpointState, HeadKey, HeadTerm, RoundBuffer, ShapeFact,
    StorageError, TermDictionary, TermId, TermIdentity,
};
use crate::store::{Arity, StoreShape};
use purrdf_lex::allocation::{Admission, Memory, Resident};

type Winner<'r, I, const N: usize> = (HeadKey<'r, I, N>, Candidate<I, N>, Option<ShapeFact<N>>);

#[derive(Debug)]
struct Preparation<'r, I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    original: std::collections::btree_map::IntoIter<HeadKey<'r, I, N>, Candidate<I, N>>,
    pending: Option<(HeadKey<'r, I, N>, Candidate<I, N>)>,
    winners: Vec<Winner<'r, I, N>>,
    sort: Option<purrdf_lex::walk::HeapSortFrame>,
}

pub(super) struct PreparationSlot<'r, I: TermIdentity, const N: usize>(Vec<Preparation<'r, I, N>>)
where
    Arity<N>: StoreShape<N>;

/// One round's original lexical publication order, with no store self-borrow.
#[derive(Debug)]
pub(super) struct Frame<'r, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    winners: std::vec::IntoIter<Winner<'r, I, N>>,
    /// Deferred array storage belongs to the supplemental owner; ordinary
    /// producer arrays retain their existing resident owner.
    winners_bytes: usize,
    pending: Option<Winner<'r, I, N>>,
    sources: Option<Vec<ShapeFact<N>>>,
    native_proof: Option<Candidate<I, N>>,
    generates_terms: bool,
    generating_rules: BTreeSet<usize>,
    complete: bool,
    winner_work_admitted: bool,
    preparation: Vec<Preparation<'r, I, N>>,
}

impl<'r, I: TermIdentity, const N: usize> Frame<'r, I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Project recorded head facts and order native keys once, before publication.
    pub(super) fn new<D: TermDictionary<Id = I>>(
        round: RoundBuffer<'r, I, N>,
        state: &FixpointState<D, N>,
    ) -> Result<Self, EvalError<D::Error>> {
        let mut winners = round
            .entries
            .into_iter()
            .map(|(key, candidate)| {
                let fact = Self::project_fact(&key, state)?;
                Ok((key, candidate, fact))
            })
            .collect::<Result<Vec<_>, D::Error>>()
            .map_err(EvalError::Source)?;
        purrdf_lex::walk::try_sort_unstable_by(&mut winners, |(left, _, _), (right, _, _)| {
            state.rel.interner().compare_rows(
                &left.terms().map(HeadTerm::as_term),
                &right.terms().map(HeadTerm::as_term),
            )
        })
        .map_err(EvalError::Source)?;
        Ok(Self {
            winners: winners.into_iter(),
            winners_bytes: 0,
            pending: None,
            sources: None,
            native_proof: None,
            generates_terms: round.generates_terms,
            generating_rules: round.generating_rules,
            complete: false,
            winner_work_admitted: false,
            preparation: Vec::new(),
        })
    }

    fn project_fact<D: TermDictionary<Id = I>>(
        key: &HeadKey<'r, I, N>,
        state: &FixpointState<D, N>,
    ) -> Result<Option<ShapeFact<N>>, D::Error> {
        if state.derivation_mode == DerivationMode::Record {
            Ok(Some(key.fact(&state.rel)?))
        } else {
            Ok(None)
        }
    }

    /// Own the original winners before any publication preparation can suspend.
    pub(super) fn reserve_deferred<A: Admission + ?Sized>(
        memory: &mut Memory<'_, A>,
        retired: &mut usize,
    ) -> Result<PreparationSlot<'r, I, N>, StorageError> {
        let mut slot = Vec::new();
        crate::allocation::reserve_array_in(&mut slot, memory, retired, |slot, memory| {
            memory.reserve(slot, 1)
        })?;
        Ok(PreparationSlot(slot))
    }

    pub(super) fn deferred(
        round: RoundBuffer<'r, I, N>,
        mut slot: PreparationSlot<'r, I, N>,
    ) -> Self {
        slot.0.push(Preparation {
            original: round.entries.into_iter(),
            pending: None,
            winners: Vec::new(),
            sort: None,
        });
        Self {
            winners: Vec::new().into_iter(),
            winners_bytes: 0,
            pending: None,
            sources: None,
            native_proof: None,
            generates_terms: round.generates_terms,
            generating_rules: round.generating_rules,
            complete: false,
            winner_work_admitted: false,
            preparation: slot.0,
        }
    }

    /// Retain each original winner and comparison across caller admission cuts.
    pub(super) fn prepare<D: TermDictionary<Id = I>, A: Admission + ?Sized>(
        &mut self,
        state: &FixpointState<D, N>,
        memory: &mut Memory<'_, A>,
        retired: &mut usize,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, EvalError<D::Error>> {
        let Some(frame) = self.preparation.first_mut() else {
            return Ok(true);
        };
        loop {
            if frame.sort.is_some() {
                break;
            }
            if frame.pending.is_none() {
                frame.pending = frame.original.next();
            }
            let Some((key, _)) = frame.pending.as_ref() else {
                frame.sort = Some(purrdf_lex::walk::HeapSortFrame::new(frame.winners.len()));
                break;
            };
            crate::allocation::reserve_array_in(
                &mut frame.winners,
                memory,
                retired,
                |values, memory| memory.reserve_for_push(values),
            )
            .map_err(EvalError::Storage)?;
            if !allow(1) {
                return Ok(false);
            }
            // Preserve the original candidate until every fallible projection succeeds.
            let fact = Self::project_fact(key, state).map_err(EvalError::Source)?;
            let (key, candidate) = frame.pending.take().expect("prepared original winner");
            frame.winners.push((key, candidate, fact));
        }
        if !frame
            .sort
            .as_mut()
            .expect("retained publication sorter")
            .resume(
                &mut frame.winners,
                |(a, _, _), (b, _, _)| {
                    state.rel.interner().compare_rows(
                        &a.terms().map(HeadTerm::as_term),
                        &b.terms().map(HeadTerm::as_term),
                    )
                },
                || allow(1),
            )
            .map_err(EvalError::Source)?
        {
            return Ok(false);
        }
        self.winners_bytes = frame
            .winners
            .capacity()
            .checked_mul(size_of::<Winner<'r, I, N>>())
            .expect("the original admitted winner array has a valid layout");
        self.winners = std::mem::take(&mut frame.winners).into_iter();
        self.preparation.clear();
        Ok(true)
    }

    pub(super) fn is_complete(&self) -> bool {
        self.complete
    }

    /// Destroy completed deferred payloads before returning their original charge.
    /// A caller retains this amount if its original admission refuses the shrink.
    pub(super) fn retire(self) -> usize {
        assert!(
            self.complete && self.native_proof.is_none() && self.preparation.is_empty(),
            "only completed publication retires its original arrays"
        );
        let bytes = self
            .preparation
            .capacity()
            .checked_mul(size_of::<Preparation<'r, I, N>>())
            .and_then(|bytes| bytes.checked_add(self.winners_bytes))
            .expect("the original publication arrays fit their admitted grant");
        drop(self);
        bytes
    }

    /// A refused fact stays pending; already published rows are never visited again.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        state: &mut FixpointState<D, N>,
    ) -> Result<(), EvalError<D::Error>> {
        let before = state.native_bytes;
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, before);
        let result = self.advance_until(state, &mut memory, |_| true);
        state.account_native_memory(before, memory.admitted_bytes());
        assert!(result?, "unlimited publication completes");
        Ok(())
    }

    pub(super) fn advance_until<D: TermDictionary<Id = I>, A: Admission + ?Sized>(
        &mut self,
        state: &mut FixpointState<D, N>,
        memory: &mut Memory<'_, A>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, EvalError<D::Error>> {
        assert!(
            self.preparation.is_empty(),
            "publication follows complete preparation"
        );
        if let Some(native) = state.native.as_mut() {
            native.flush_retired(memory).map_err(EvalError::Storage)?;
        }
        state
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        while !self.complete {
            state
                .rel
                .flush_metadata_retirement_in(memory)
                .map_err(EvalError::Storage)?;
            if self.pending.is_none() {
                self.pending = self.winners.next();
            }
            let Some((key, candidate, _)) = &self.pending else {
                if self.generates_terms {
                    state.term_generating_rounds = state.term_generating_rounds.saturating_add(1);
                    state.generating_rules = std::mem::take(&mut self.generating_rules);
                }
                self.complete = true;
                break;
            };
            if !self.winner_work_admitted {
                if !allow(candidate.sources.len().saturating_add(1)) {
                    return Ok(false);
                }
                self.winner_work_admitted = true;
            }
            let existing = if state.native.is_some() {
                super::present_row(key, &state.rel).map_err(EvalError::Source)?
            } else {
                None
            };
            if let Some(row) = existing {
                let native = state
                    .native
                    .as_ref()
                    .expect("present-head publication is maintained");
                let improves = match native.nodes.get(&row) {
                    Some(old) => candidate
                        .preferred_over(&old.proof, &state.rel)
                        .map_err(EvalError::Source)?,
                    None => false, // An asserted row has the preferred height zero.
                };
                if !improves {
                    self.pending = None;
                    self.winner_work_admitted = false;
                    self.sources = None;
                    if let Some(proof) = self.native_proof.take() {
                        let native = state
                            .native
                            .as_mut()
                            .expect("an admitted native proof owner");
                        native.retire_proof(proof);
                        native.flush_retired(memory).map_err(EvalError::Storage)?;
                    }
                    continue;
                }
            }
            let known = key.terms().map(HeadTerm::interned);
            let restored = state.native.as_ref().and_then(|native| {
                if known.iter().all(Option::is_some) {
                    let terms = known.map(|id| id.expect("all original positions are known"));
                    native
                        .withdrawn
                        .get(&terms)
                        .map(|original| (original.id(), terms))
                } else {
                    None
                }
            });
            if existing.is_none() {
                state
                    .report()
                    .admit_fact()
                    .map_err(|report| EvalError::BudgetExhausted {
                        resource: BudgetResource::StoredFacts,
                        report,
                    })?;
            }
            if self.sources.is_none() && state.derivation_mode == DerivationMode::Record {
                self.sources = Some(
                    candidate
                        .source_facts(&state.rel)
                        .map_err(EvalError::Source)?,
                );
            }
            // Publication must not leave a native row without its proof height
            // or requested recorded witness if a companion allocation refuses.
            // Reserve first; a refusal retains this original pending winner.
            let native_depth = state
                .native
                .as_ref()
                .is_some_and(super::maintained::Supports::uses_native_heights);
            if !native_depth {
                state
                    .depth
                    .try_reserve(1)
                    .map_err(|_| EvalError::Storage(StorageError::AllocationFailed))?;
            }
            if state.derivation_mode == DerivationMode::Record {
                state
                    .derivations
                    .try_reserve(1)
                    .map_err(|_| EvalError::Storage(StorageError::AllocationFailed))?;
            }
            if let Some(native) = state.native.as_mut() {
                native
                    .reserve(candidate, memory)
                    .map_err(EvalError::Storage)?;
                if self.native_proof.is_none() {
                    self.native_proof =
                        Some(candidate.copy_in(memory).map_err(EvalError::Storage)?);
                }
            }
            let terms = key.terms().map(|term| match term {
                HeadTerm::Interned(id) => DictionaryTerm::Id(*id),
                HeadTerm::Fresh(surface) => DictionaryTerm::Surface(surface.as_ref()),
            });
            let row = if let Some(row) = existing {
                row
            } else if let Some((row, ids)) = restored {
                let admitted = state
                    .rel
                    .admit_restore_in(
                        state
                            .native
                            .as_ref()
                            .expect("restoration has a native owner")
                            .withdrawn
                            .get(&ids)
                            .expect("the original withdrawal token remains owned"),
                        memory,
                        &mut allow,
                    )
                    .map_err(|error| match error {
                        DictionaryError::Source(error) => EvalError::Source(error),
                        DictionaryError::Storage(error) => EvalError::Storage(error),
                    })?;
                if !admitted {
                    return Ok(false);
                }
                let original = state
                    .native
                    .as_mut()
                    .expect("restoration has a native owner")
                    .withdrawn
                    .remove(&ids)
                    .expect("the admitted original token is moved exactly once");
                state.rel.restore_row_in(original, memory);
                row
            } else {
                let inserted = state
                    .rel
                    .insert_terms_until_in(terms, memory, &mut allow)
                    .map_err(|error| match error {
                        DictionaryError::Source(error) => EvalError::Source(error),
                        DictionaryError::Storage(error) => EvalError::Storage(error),
                    })?;
                let core::ops::ControlFlow::Continue(inserted) = inserted else {
                    return Ok(false);
                };
                inserted.expect("an absent winner inserts a new row").2
            };
            let (_, candidate, fact) = self
                .pending
                .take()
                .expect("the original winner was inserted");
            self.winner_work_admitted = false;
            if !native_depth {
                if row.resident_index() == state.depth.len() {
                    state.depth.push(candidate.proof_height);
                } else {
                    state.depth[row.resident_index()] = candidate.proof_height;
                }
            }
            state.committed_facts = state.committed_facts.saturating_add(1);
            if let Some(sources) = self.sources.take() {
                state.derivations.push(Derivation {
                    fact: fact.expect("recorded winners project their facts before publication"),
                    rule: candidate.rule,
                    head: candidate.head,
                    sources,
                    proof_height: candidate.proof_height,
                });
            }
            if let Some(native) = state.native.as_mut() {
                let (_, terms) = state
                    .rel
                    .rows_from(row.index())
                    .next()
                    .expect("the native row was just published");
                let proof = self
                    .native_proof
                    .take()
                    .expect("the native proof was admitted before publication");
                native.record(row, terms, proof, existing.is_none());
                native.flush_retired(memory).map_err(EvalError::Storage)?;
            }
            state
                .rel
                .flush_metadata_retirement_in(memory)
                .map_err(EvalError::Storage)?;
        }
        state
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        Ok(true)
    }
}
