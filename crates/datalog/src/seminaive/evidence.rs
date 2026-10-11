// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native evidence capture at the original rule-emission and round-commit boundaries.

use purrdf_core::evidence::{
    EvidenceCombination, EvidenceId, EvidenceWorkspaceError, ReasonId, SearchCompleteness,
};
use purrdf_lex::allocation::Admission;

use super::{Cow, EvalError, HeadKey, HeadTerm, RelationStore, SourceRow, factors::Frontier};
use crate::cursor::LendingIterator;
use crate::evidence::DerivationEvidence;

/// Caller-declared semantic scope for negative body predicates.
///
/// Closure is never inferred from a completed execution alone. The supplied reason
/// is the caller's canonical description of a missing or incomplete scope.
#[derive(Clone, Copy, Debug)]
pub struct NegationScope {
    /// Completeness of the source relation in this scope.
    pub completeness: SearchCompleteness,
    /// Whether absence in this scope has a declared closed-world interpretation.
    pub closure_declared: bool,
    /// Original reason identifying the incomplete or undeclared scope.
    pub missing_scope_reason: ReasonId,
}

pub(super) trait RuleEvidence {
    fn source(&mut self, row: crate::id::RowId) -> Result<EvidenceId, EvalError>;
    fn combine(
        &mut self,
        left: EvidenceId,
        right: EvidenceId,
        mode: EvidenceCombination,
    ) -> Result<EvidenceId, EvalError>;
    fn negative(&mut self, witnesses: Option<EvidenceId>) -> Result<(EvidenceId, bool), EvalError>;
    fn conjunction(&mut self, sources: &[SourceRow]) -> Result<EvidenceId, EvalError>;
    fn product(&mut self, factors: &[&Frontier<'_>]) -> Result<EvidenceId, EvalError>;
    fn emit(
        &mut self,
        key: &HeadKey<'_>,
        witness: EvidenceId,
        uses_absence: bool,
    ) -> Result<(), EvalError>;
    fn commit(&mut self, store: &RelationStore) -> Result<bool, EvalError>;
}

/// Pending head owners precede the aligned row table and its original grant.
pub(super) struct Capture<S: Admission> {
    pending: Vec<(HeadKey<'static>, EvidenceId)>,
    pub(super) rows: DerivationEvidence<S>,
    scope: NegationScope,
}

impl<S: Admission> Capture<S> {
    pub(super) fn new(rows: DerivationEvidence<S>, scope: NegationScope) -> Self {
        Self {
            pending: Vec::new(),
            rows,
            scope,
        }
    }

    fn negative_native(
        &mut self,
        witnesses: Option<EvidenceId>,
    ) -> Result<(EvidenceId, bool), EvidenceWorkspaceError> {
        use purrdf_core::evidence::Information;
        let scope = self.scope;
        let id = match witnesses {
            None if scope.completeness.admits_absence(scope.closure_declared) => {
                EvidenceId::DEFAULT
            }
            None => self.rows.workspace_mut().intern(|memory| {
                purrdf_core::evidence::EvidenceRecord::from_parts(
                    &purrdf_core::evidence::InformationMap::uniform(Information::Neither),
                    scope.completeness.worse(SearchCompleteness::Undetermined),
                    &purrdf_core::evidence::Frame::actual(),
                    &[scope.missing_scope_reason],
                    &[],
                    memory,
                )
            })?,
            Some(id) => self.rows.workspace_mut().transform(id, |record, memory| {
                if scope.completeness.admits_absence(scope.closure_declared)
                    && record.completeness() == SearchCompleteness::Complete
                {
                    record.negated(memory)
                } else {
                    record.with_completeness(
                        record
                            .completeness()
                            .worse(scope.completeness)
                            .worse(SearchCompleteness::Undetermined),
                        Some(scope.missing_scope_reason),
                        memory,
                    )
                }
            })?,
        };
        let record = self.rows.workspace().get(id)?;
        let blocked = record.completeness() == SearchCompleteness::Complete
            && record.information().fallback() == Information::Opposed
            && record
                .information()
                .entries()
                .iter()
                .all(|(_, value)| *value == Information::Opposed);
        Ok((id, blocked))
    }

    fn emit_native(
        &mut self,
        key: &HeadKey<'_>,
        mut witness: EvidenceId,
        uses_absence: bool,
    ) -> Result<(), EvidenceWorkspaceError> {
        if uses_absence
            && !self
                .scope
                .completeness
                .admits_absence(self.scope.closure_declared)
        {
            let scope = self.scope;
            witness = self
                .rows
                .workspace_mut()
                .transform(witness, |record, memory| {
                    record.with_completeness(
                        record
                            .completeness()
                            .worse(scope.completeness)
                            .worse(SearchCompleteness::Undetermined),
                        Some(scope.missing_scope_reason),
                        memory,
                    )
                })?;
        }
        if let Some(position) = self.pending.iter().position(|(stored, _)| stored == key) {
            let previous = self.pending[position].1;
            self.pending[position].1 = self.rows.workspace_mut().combine(
                previous,
                witness,
                EvidenceCombination::Alternatives,
            )?;
            return Ok(());
        }
        let pending = &mut self.pending;
        self.rows.workspace_mut().native_operation(|_, memory| {
            // Persistent capacity is reserved outside the child's rollback scope.
            memory.reserve_for_push(pending)?;
            let owned = memory.try_scope(|memory| {
                let mut copy =
                    |term: &HeadTerm<'_>| -> Result<HeadTerm<'static>, EvidenceWorkspaceError> {
                        Ok(match term {
                            HeadTerm::Interned(id) => HeadTerm::Interned(*id),
                            HeadTerm::Fresh(surface) => {
                                HeadTerm::Fresh(Cow::Owned(memory.string(surface)?))
                            }
                        })
                    };
                Ok::<_, EvidenceWorkspaceError>(HeadKey {
                    subject: copy(&key.subject)?,
                    predicate: copy(&key.predicate)?,
                    object: copy(&key.object)?,
                    graph: copy(&key.graph)?,
                })
            })?;
            pending.push((owned, witness));
            Ok(())
        })
    }

    fn product_native(
        &mut self,
        factors: &[&Frontier<'_>],
    ) -> Result<EvidenceId, EvidenceWorkspaceError> {
        let mut groups = Vec::new();
        for factor in factors {
            let mut ids = Vec::new();
            for witness in factor.witnesses() {
                let mut id = self
                    .rows
                    .conjunction(witness.sources.iter().map(|source| source.row))?;
                if let Some(negative) = witness.negative_evidence {
                    id = self.rows.workspace_mut().combine(
                        id,
                        negative,
                        EvidenceCombination::Conjunction,
                    )?;
                }
                self.rows.workspace_mut().native_operation(|_, memory| {
                    memory.reserve_for_push(&mut ids)?;
                    ids.push(id);
                    Ok(())
                })?;
            }
            self.rows.workspace_mut().native_operation(|_, memory| {
                memory.reserve_for_push(&mut groups)?;
                groups.push(ids);
                Ok(())
            })?;
        }
        let references = self
            .rows
            .workspace_mut()
            .native_operation(|_, memory| Ok(memory.collect(groups.iter().map(Vec::as_slice))?))?;
        let result = self.rows.workspace_mut().factorized(&references)?.ok_or(
            EvidenceWorkspaceError::Column(
                purrdf_core::evidence::EvidenceColumnError::RowOutOfBounds,
            ),
        )?;
        self.rows.workspace_mut().native_operation(|_, memory| {
            memory.release_vec(references)?;
            Ok(())
        })?;
        self.rows.workspace_mut().native_operation(|_, memory| {
            for ids in &mut groups {
                memory.release_vec(core::mem::take(ids))?;
            }
            memory.release_vec(groups)?;
            Ok(())
        })?;
        Ok(result)
    }

    fn commit_native(&mut self, store: &RelationStore) -> Result<bool, EvidenceWorkspaceError> {
        let original_rows = self.rows.row_count();
        for (row, _) in store.rows_from(original_rows) {
            let witness = self
                .pending
                .iter()
                .find_map(|(key, id)| (stored_row(key, store) == Some(row)).then_some(*id))
                .ok_or(EvidenceWorkspaceError::Column(
                    purrdf_core::evidence::EvidenceColumnError::RowOutOfBounds,
                ))?;
            self.rows.push_derived(row, witness)?;
        }
        let mut changed = false;
        for (key, id) in &self.pending {
            let row = stored_row(key, store).ok_or(EvidenceWorkspaceError::Column(
                purrdf_core::evidence::EvidenceColumnError::RowOutOfBounds,
            ))?;
            if row.index() < original_rows {
                changed |= self.rows.coalesce(row, *id)?;
            }
        }
        let pending = core::mem::take(&mut self.pending);
        self.rows.workspace_mut().native_operation(|_, memory| {
            let mut pending = pending;
            while let Some((key, _)) = pending.pop() {
                for term in [key.subject, key.predicate, key.object, key.graph] {
                    if let HeadTerm::Fresh(Cow::Owned(surface)) = term {
                        memory.release_string(surface)?;
                    }
                }
            }
            memory.release_vec(pending)?;
            Ok(())
        })?;
        Ok(changed)
    }
}

fn stored_row(key: &HeadKey<'_>, store: &RelationStore) -> Option<crate::id::RowId> {
    let term_id = |term: &HeadTerm<'_>| match term {
        HeadTerm::Interned(id) => Some(*id),
        HeadTerm::Fresh(surface) => store.term_id(surface),
    };
    let (subject, predicate, object, graph) = (
        term_id(&key.subject)?,
        term_id(&key.predicate)?,
        term_id(&key.object)?,
        term_id(&key.graph)?,
    );
    store
        .partition(predicate, graph)?
        .select(crate::store::Bound::Both(subject, object))
        .next()
        .map(|(_, _, row)| row)
}

impl<S: Admission> RuleEvidence for Capture<S> {
    fn source(&mut self, row: crate::id::RowId) -> Result<EvidenceId, EvalError> {
        self.rows
            .rows()
            .get(row.index())
            .map_err(|error| EvalError::Evidence(EvidenceWorkspaceError::Column(error)))
    }

    fn combine(
        &mut self,
        left: EvidenceId,
        right: EvidenceId,
        mode: EvidenceCombination,
    ) -> Result<EvidenceId, EvalError> {
        self.rows
            .workspace_mut()
            .combine(left, right, mode)
            .map_err(EvalError::Evidence)
    }

    fn negative(&mut self, witnesses: Option<EvidenceId>) -> Result<(EvidenceId, bool), EvalError> {
        self.negative_native(witnesses).map_err(EvalError::Evidence)
    }
    fn conjunction(&mut self, sources: &[SourceRow]) -> Result<EvidenceId, EvalError> {
        self.rows
            .conjunction(sources.iter().map(|source| source.row))
            .map_err(EvalError::Evidence)
    }

    fn product(&mut self, factors: &[&Frontier<'_>]) -> Result<EvidenceId, EvalError> {
        self.product_native(factors).map_err(EvalError::Evidence)
    }

    fn emit(
        &mut self,
        key: &HeadKey<'_>,
        witness: EvidenceId,
        uses_absence: bool,
    ) -> Result<(), EvalError> {
        self.emit_native(key, witness, uses_absence)
            .map_err(EvalError::Evidence)
    }

    fn commit(&mut self, store: &RelationStore) -> Result<bool, EvalError> {
        self.commit_native(store).map_err(EvalError::Evidence)
    }
}
