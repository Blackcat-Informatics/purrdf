// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Demand queries over the original native signed transaction owner.

use super::{Arity, DemandError, GoalMatch, PreparedDemand, RelationStore, StoreShape};
use crate::dictionary::{DictionaryTerm, TermDictionary, TermIdentity};
use crate::guard::NoGuards;
use crate::maintenance::ChangeBatch;
use crate::seminaive::{
    BudgetReport, EvalError, EvalOptions, Maintained, MaintenanceError, ResumeProgress,
    SessionContext,
};
use crate::stop::StopSignal;
use purrdf_lex::allocation::{Admission, Resident};

/// A maintained demand query whose private transformed model is never exposed.
///
/// Borrows the prepared executable and preserves the original dictionary, signed
/// transaction, admission and opaque watermark through native pause/resume.
#[derive(Debug)]
pub struct MaintainedDemand<
    'p,
    M,
    S: Admission,
    D: TermDictionary,
    const N: usize = 3,
    A: Admission = Resident,
> where
    Arity<N>: StoreShape<N>,
{
    inner: Maintained<'p, M, S, D, N, A>,
    prepared: &'p PreparedDemand<N>,
}

/// Complete net goal-membership changes at the original host position.
///
/// The selected vector uses resident allocation. Original IDs retain their
/// meaning through the borrow of the maintained owner; private predicates are absent.
#[derive(Debug)]
pub struct DemandDelta<'a, M, I: TermIdentity, const N: usize = 3> {
    watermark: &'a M,
    changes: Vec<([I; N], i64)>,
}

/// Completed goal delta, preserving the original dictionary's failure type.
pub type DemandDeltaResult<'a, M, D, const N: usize = 3> = Result<
    Option<DemandDelta<'a, M, <D as TermDictionary>::Id, N>>,
    MaintenanceError<<D as TermDictionary>::Error>,
>;

impl<M, I: TermIdentity, const N: usize> DemandDelta<'_, M, I, N> {
    /// Original position acknowledged only after the native transaction completes.
    pub const fn watermark(&self) -> &M {
        self.watermark
    }

    /// Inserted (+1) and finally removed (-1) matching rows in native canonical order.
    pub fn changes(&self) -> &[([I; N], i64)] {
        &self.changes
    }
}

impl<const N: usize> PreparedDemand<N>
where
    Arity<N>: StoreShape<N>,
{
    /// Maintain this query with resident native metadata storage.
    /// Initialization is resumed through [`MaintainedDemand::resume_until`].
    ///
    /// # Errors
    /// Refuses incompatible original dictionaries before execution, preserving
    /// original dictionary and native initialization failures.
    pub fn maintain<M, S: Admission, D: TermDictionary>(
        &self,
        edb: RelationStore<D, N>,
        context: SessionContext,
        options: EvalOptions,
    ) -> Result<MaintainedDemand<'_, M, S, D, N>, DemandError<D::Error>> {
        self.maintain_in(edb, context, options, Resident)
    }

    /// Maintain this query under an original supplemental storage grant.
    /// The returned owner borrows this prepared artifact and owns the input store.
    ///
    /// # Errors
    /// Preserves the same pre-execution and initialization failures as [`Self::maintain`].
    pub fn maintain_in<M, S: Admission, D: TermDictionary, A: Admission>(
        &self,
        edb: RelationStore<D, N>,
        context: SessionContext,
        options: EvalOptions,
        admission: A,
    ) -> Result<MaintainedDemand<'_, M, S, D, N, A>, DemandError<D::Error>> {
        self.check_compatibility(edb.interner())?;
        let inner = Maintained::new_in(
            &self.executable,
            edb,
            &NoGuards,
            context,
            options,
            admission,
        )
        .map_err(DemandError::Evaluation)?
        .with_reserved_input_names(&self.names);
        Ok(MaintainedDemand {
            inner,
            prepared: self,
        })
    }
}

impl<M, S: Admission, D: TermDictionary, const N: usize, A: Admission>
    MaintainedDemand<'_, M, S, D, N, A>
where
    Arity<N>: StoreShape<N>,
{
    /// Queue an original transaction without acknowledging or scanning its rows.
    /// Reserved-name and multiplicity validation are native metered preparation.
    ///
    /// # Errors
    /// Preserves the native uninitialized, busy and failed-owner refusals.
    pub fn begin(
        &mut self,
        batch: ChangeBatch<M, S, D::Id, N>,
    ) -> Result<(), MaintenanceError<D::Error>> {
        self.inner.begin(batch)
    }

    /// Resume initialization or a transaction under the native cumulative allowance.
    /// Pauses retain the original input, storage grant and watermark. Complete
    /// answers remain withheld until the entire original transaction settles.
    ///
    /// # Errors
    /// Preserves native work, source, storage, stop, multiplicity and namespace refusals.
    pub fn resume_until(
        &mut self,
        options: EvalOptions,
        limit: u128,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ResumeProgress<D::Error>, MaintenanceError<D::Error>> {
        self.inner.resume_until(options, limit, stop)
    }

    /// Admit host terms in the original dictionary before constructing a transaction.
    /// Every position is checked before any term is interned; generated names are
    /// private to the transformation even when passed by their existing IDs.
    ///
    /// # Errors
    /// Preserves original native readiness, dictionary, storage and namespace failures.
    pub fn admit_terms(
        &mut self,
        terms: [DictionaryTerm<'_, D::Id>; N],
    ) -> Result<[D::Id; N], MaintenanceError<D::Error>> {
        self.inner.admit_terms(terms)
    }

    /// Borrow the original dictionary only at a complete readable snapshot.
    ///
    /// # Errors
    /// Preserves native readiness and original source failures.
    pub fn dictionary(&self) -> Result<&D, MaintenanceError<D::Error>> {
        self.inner.facts().map(RelationStore::interner)
    }

    /// Collect complete goal answers through the same selector as one-shot demand.
    /// The result uses resident allocation and retains the original dictionary IDs.
    ///
    /// # Errors
    /// Preserves native readiness and original lookup/read failures, even for no rows.
    pub fn answers(&self) -> Result<Vec<[D::Id; N]>, MaintenanceError<D::Error>> {
        super::answers(
            self.inner.facts()?,
            &self.prepared.goal,
            self.inner.demand_projection(),
        )
        .map_err(|error| MaintenanceError::Failed(EvalError::Source(error)))
    }

    /// Select the last completed transaction's exact goal-membership delta.
    /// Duplicate assertions and temporary proof repairs contribute no change.
    ///
    /// # Errors
    /// Preserves native readiness and original dictionary lookup/read failures.
    pub fn delta(&self) -> DemandDeltaResult<'_, M, D, N> {
        let Some(delta) = self.inner.model_delta()? else {
            return Ok(None);
        };
        let dictionary = self.dictionary()?;
        let matcher = GoalMatch::new(dictionary, &self.prepared.goal)
            .map_err(|error| MaintenanceError::Failed(EvalError::Source(error)))?;
        let changes = delta
            .changes()
            .iter()
            .copied()
            .filter(|(row, _)| {
                (!self.prepared.goal.predicate().is_var()
                    || self
                        .inner
                        .demand_projection()
                        .is_some_and(|projection| projection.original(row[1])))
                    && matcher
                        .as_ref()
                        .is_some_and(|matcher| matcher.matches(*row))
            })
            .collect();
        dictionary
            .check_read()
            .map_err(|error| MaintenanceError::Failed(EvalError::Source(error)))?;
        Ok(Some(DemandDelta {
            watermark: self
                .inner
                .applied_watermark()
                .expect("a completed native delta retains its original applied batch"),
            changes,
        }))
    }

    /// Last atomically acknowledged original host position.
    pub fn applied_watermark(&self) -> Option<&M> {
        self.inner.applied_watermark()
    }

    /// Cumulative admitted initialization work, independent of later transactions.
    pub const fn initialization_work(&self) -> u128 {
        self.inner.initialization_work()
    }

    /// Native preparation and mutation work for the current or last transaction.
    pub fn transaction_work(&self) -> u128 {
        self.inner.transaction_work()
    }

    /// Native execution budget, including pending transformed work and internal facts.
    pub fn budget(&self) -> BudgetReport {
        self.inner.budget()
    }
}
