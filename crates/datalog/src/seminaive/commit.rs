// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Sorted publication retains the first unadmitted winner and its original sources.

use super::{
    BTreeSet, BudgetResource, Candidate, Derivation, DerivationMode, DictionaryError,
    DictionaryTerm, EvalError, FixpointState, HeadKey, HeadTerm, RoundBuffer, ShapeFact,
    TermDictionary, TermId, TermIdentity,
};
use crate::store::{Arity, StoreShape};

type Winner<'r, I, const N: usize> = (HeadKey<'r, I, N>, Candidate<I, N>, Option<ShapeFact<N>>);

/// One round's original lexical publication order, with no store self-borrow.
#[derive(Debug)]
pub(super) struct Frame<'r, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    winners: std::vec::IntoIter<Winner<'r, I, N>>,
    pending: Option<Winner<'r, I, N>>,
    sources: Option<Vec<ShapeFact<N>>>,
    generates_terms: bool,
    generating_rules: BTreeSet<usize>,
    complete: bool,
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
                let fact = if state.derivation_mode == DerivationMode::Record {
                    Some(key.fact(&state.rel)?)
                } else {
                    None
                };
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
            pending: None,
            sources: None,
            generates_terms: round.generates_terms,
            generating_rules: round.generating_rules,
            complete: false,
        })
    }

    pub(super) fn is_complete(&self) -> bool {
        self.complete
    }

    /// A refused fact stays pending; already published rows are never visited again.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        state: &mut FixpointState<D, N>,
    ) -> Result<(), EvalError<D::Error>> {
        state
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        while !self.complete {
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
            state
                .report()
                .admit_fact()
                .map_err(|report| EvalError::BudgetExhausted {
                    resource: BudgetResource::StoredFacts,
                    report,
                })?;
            if self.sources.is_none() && state.derivation_mode == DerivationMode::Record {
                self.sources = Some(
                    candidate
                        .source_facts(&state.rel)
                        .map_err(EvalError::Source)?,
                );
            }
            let terms = key.terms().map(|term| match term {
                HeadTerm::Interned(id) => DictionaryTerm::Id(*id),
                HeadTerm::Fresh(surface) => DictionaryTerm::Surface(surface.as_ref()),
            });
            let inserted = state.rel.insert_terms(terms).map_err(|error| match error {
                DictionaryError::Source(error) => EvalError::Source(error),
                DictionaryError::Storage(error) => EvalError::Storage(error),
            })?;
            let (_, _, row) = inserted.expect(
                "a round winner is absent from the store by construction, so it inserts a new row",
            );
            debug_assert_eq!(
                row.resident_index(),
                state.depth.len(),
                "depth tracks store rows"
            );
            let (_, candidate, fact) = self
                .pending
                .take()
                .expect("the original winner was inserted");
            state.depth.push(candidate.proof_height);
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
        }
        state.rel.interner().check_read().map_err(EvalError::Source)
    }
}
