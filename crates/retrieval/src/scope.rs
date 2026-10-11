// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact binding of admitted request constraints to their original evaluators.

use purrdf_sparql_eval::{
    AdmittedVec, CandidateConstraint, EvalError, NativeCandidateScope, NativeDiagnostic,
    NativeDiagnosticKind, WorkspaceCapability,
};

use crate::{CompiledRead, RequestTerm};

/// An original projected solution and its inseparable native declaration.
/// Score/rank columns keep the values the producer published, including the
/// separate possible-candidate positions of undecided rows.
#[derive(Debug)]
pub struct ScopedRow {
    /// Original projected cells, in the receipt's variable order.
    pub cells: AdmittedVec<Option<purrdf_sparql_eval::WorkspaceTerm>>,
    /// The retained original query dictionary, record and membership bit.
    pub evidence: purrdf_sparql_eval::RowEvidence,
}

/// The original native rows retained as their actual stream is pulled.
/// A live on-demand read exposes its current prefix until it is settled.
#[derive(Clone, Debug)]
pub struct ScopedRows {
    state: purrdf_sparql_eval::SharedWorkspace<ScopedRowsState>,
    workspace: WorkspaceCapability,
}

#[derive(Debug)]
struct ScopedRowsState {
    rows: std::sync::Mutex<ScopedRowsData>,
    search: std::sync::Mutex<purrdf_sparql_eval::RowEvidence>,
    search_writer: std::sync::Mutex<()>,
}

#[derive(Debug)]
struct ScopedRowsData {
    rows: AdmittedVec<ScopedRow>,
    complete: bool,
}

impl ScopedRows {
    pub(crate) fn new(
        rows: AdmittedVec<ScopedRow>,
        complete: bool,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        Ok(Self {
            state: purrdf_sparql_eval::SharedWorkspace::new_admitted(
                ScopedRowsState {
                    rows: std::sync::Mutex::new(ScopedRowsData { rows, complete }),
                    search: std::sync::Mutex::new(purrdf_sparql_eval::RowEvidence::default()),
                    search_writer: std::sync::Mutex::new(()),
                },
                workspace,
            )?,
            workspace: workspace.clone(),
        })
    }

    fn poisoned(&self) -> EvalError {
        NativeDiagnostic::error(
            NativeDiagnosticKind::Internal,
            "poisoned original scoped row owner",
            &self.workspace,
        )
    }

    /// Inspect the original current prefix without copying cells or evidence.
    ///
    /// # Errors
    /// Refuses a poisoned native row owner.
    pub fn with_rows<R>(&self, read: impl FnOnce(&[ScopedRow]) -> R) -> Result<R, EvalError> {
        let state = self.state.rows.lock().map_err(|_| self.poisoned())?;
        Ok(read(&state.rows))
    }

    /// Whether the original cursor has been fully drained and settled.
    ///
    /// # Errors
    /// Refuses a poisoned native row owner.
    pub fn complete(&self) -> Result<bool, EvalError> {
        let state = self.state.rows.lock().map_err(|_| self.poisoned())?;
        Ok(state.complete)
    }

    pub(crate) fn push(&self, row: ScopedRow) -> Result<(), EvalError> {
        let mut state = self.state.rows.lock().map_err(|_| self.poisoned())?;
        state.rows.push(row)
    }

    pub(crate) fn finish(&self) -> Result<(), EvalError> {
        let mut state = self.state.rows.lock().map_err(|_| self.poisoned())?;
        state.complete = true;
        drop(state);
        Ok(())
    }

    pub(crate) fn retain_search(
        &self,
        search: &purrdf_sparql_eval::RowEvidence,
    ) -> Result<(), EvalError> {
        let writer = self
            .state
            .search_writer
            .lock()
            .map_err(|_| self.poisoned())?;
        let previous = {
            let state = self.state.search.lock().map_err(|_| self.poisoned())?;
            state.clone()
        };
        // A reader may inspect this value from inside a query-record callback.
        // Keep writer serialization, but never hold the value lock while the
        // original dictionary is borrowed by combination.
        let updated = previous.combine(
            search,
            purrdf_core::evidence::EvidenceCombination::Conjunction,
        )?;
        let mut state = self.state.search.lock().map_err(|_| self.poisoned())?;
        *state = updated;
        drop(state);
        drop(writer);
        Ok(())
    }

    pub(crate) fn search(&self) -> ScopedSearch {
        ScopedSearch { rows: self.clone() }
    }
}

/// Whole-search evidence sharing the original live row receipt owner.
/// Late native declarations remain visible after an on-demand stream settles.
#[derive(Clone, Debug)]
pub struct ScopedSearch {
    rows: ScopedRows,
}

impl ScopedSearch {
    /// Retain the current original handle, including closure, reasons and owner.
    ///
    /// # Errors
    /// Refuses a poisoned original search owner.
    pub fn retained(&self) -> Result<purrdf_sparql_eval::RowEvidence, EvalError> {
        let state = self
            .rows
            .state
            .search
            .lock()
            .map_err(|_| self.rows.poisoned())?;
        let search = state.clone();
        drop(state);
        Ok(search)
    }
    /// Borrow the current original record without reconstructing its declaration.
    ///
    /// # Errors
    /// Preserves poisoned-owner and original query evidence failures.
    pub fn with_record<R>(
        &self,
        read: impl FnOnce(&purrdf_core::evidence::EvidenceRecord) -> R,
    ) -> Result<R, EvalError> {
        self.retained()?.with_record(read)
    }
}

/// Scoped receipt for one actual native unit execution, retaining its live rows.
#[derive(Debug)]
pub struct ScopedStratum {
    /// Original stratum IRI text, retained under native physical admission.
    pub stratum: purrdf_lex::allocation::SharedText,
    /// Original projected variable names.
    pub variables: AdmittedVec<purrdf_lex::allocation::SharedText>,
    /// Original result order, with established and unknown rows both retained.
    pub rows: ScopedRows,
    /// Executed exclusion-query receipts, each with the candidate as its sole
    /// cell and the query's original existential evidence. This remains a live
    /// history while the ranked stream can accept further point lookups.
    pub exclusions: ScopedRows,
    /// Whole-search evidence retained even when the unit emits no solution.
    pub search: ScopedSearch,
    /// Independent exact counts for every original caller constraint.
    pub constraints: AdmittedVec<purrdf_core::query_scope::ConstraintCounts>,
}

impl ScopedStratum {
    pub(crate) fn opened(
        stratum: &str,
        cursor: &purrdf_sparql_eval::CallCursor,
        scope: &NativeCandidateScope<'_>,
        exclusions: ScopedRows,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let mut variables = AdmittedVec::with_capacity(cursor.variables().len(), workspace)?;
        for variable in cursor.variables() {
            variables.push(workspace.authored_text(variable)?)?;
        }
        let rows = ScopedRows::new(AdmittedVec::new(workspace), false, workspace)?;
        rows.retain_search(cursor.search_evidence())?;
        Ok(Self {
            stratum: workspace.authored_text(stratum)?,
            variables,
            search: rows.search(),
            rows,
            exclusions,
            constraints: retain_counts(scope, workspace)?,
        })
    }
}

pub(crate) fn retain_counts(
    scope: &NativeCandidateScope<'_>,
    workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<purrdf_core::query_scope::ConstraintCounts>, EvalError> {
    scope.with_counts(|counts| {
        let mut retained = AdmittedVec::with_capacity(counts.len(), workspace)?;
        for count in counts {
            retained.push(*count)?;
        }
        Ok(retained)
    })?
}

/// Ranked output plus the original scoped receipts. Only established rows enter
/// the ranked output; undecided rows remain inspectable with their own evidence.
#[derive(Debug)]
pub struct ScopedExecution<R> {
    /// Original compiled bundle output, ready for its existing consumer.
    pub result: R,
    /// Actual unit receipts in compiled unit order, each naming its stratum.
    pub scopes: AdmittedVec<ScopedStratum>,
    /// The original ordered fused partitions, present after scoped search.
    /// Unfused execution retains its per-stratum receipts instead.
    pub fused: Option<ScopedFusedAnswer>,
}

/// A certified original fused row and its inseparable native reason owner.
#[derive(Clone, Debug)]
pub struct ScopedFusedRow {
    /// Original fusion payload: scores, contributions and ranks are unchanged.
    pub row: crate::FusedRow,
    /// Original combined dependencies, including explicit undecided membership.
    pub evidence: purrdf_sparql_eval::RowEvidence,
    original_identity: purrdf_hash::hex::Digest32,
}

impl ScopedFusedRow {
    /// Content identity of the original scored row and its declared evidence.
    ///
    #[must_use]
    pub const fn identity(&self) -> purrdf_hash::hex::Digest32 {
        self.original_identity
    }

    pub(crate) fn new(
        row: crate::FusedRow,
        evidence: purrdf_sparql_eval::RowEvidence,
    ) -> Result<Self, EvalError> {
        use purrdf_hash::{Domain, blake3::RecordHasher, frame::frame_le_into, hex::Digest32};
        let mut hash = RecordHasher::new();
        frame_le_into(
            &mut hash,
            Domain::new(b"purrdf-retrieval/scoped-fused-row/v1").as_bytes(),
        );
        frame_le_into(&mut hash, row.entity.as_str().as_bytes());
        frame_le_into(&mut hash, &row.score.into_raw().to_le_bytes());
        frame_le_into(&mut hash, &(row.contributions.len() as u64).to_le_bytes());
        for (stratum, rank, contribution) in &*row.contributions {
            frame_le_into(&mut hash, stratum.as_str().as_bytes());
            frame_le_into(&mut hash, &rank.to_le_bytes());
            frame_le_into(&mut hash, &contribution.into_raw().to_le_bytes());
        }
        frame_le_into(&mut hash, &[u8::from(evidence.is_undecided())]);
        evidence.with_record(|record| {
            frame_le_into(&mut hash, record.identity().as_bytes());
        })?;
        Ok(Self {
            row,
            evidence,
            original_identity: Digest32::new(*hash.finalize().as_bytes()),
        })
    }
}

/// Original composition order over established and undecided fused rows.
/// Every payload and reason remains admitted while an acceptance iterator borrows it.
#[derive(Clone, Debug)]
pub struct ScopedFusedAnswer {
    /// Both partitions in the original engine's certified total order. Known
    /// rows past the requested established top-k are not published.
    pub rows: purrdf_sparql_eval::SharedWorkspace<AdmittedVec<ScopedFusedRow>>,
    pub(crate) owner: purrdf_sparql_eval::QueryEvidence,
}

impl ScopedFusedAnswer {
    /// Merge explicit accepted reasons into the retained answer. This allocates
    /// nothing and never invokes a producer, fusion, ranking or constraint.
    ///
    /// # Errors
    /// Refuses noncanonical/unknown reasons and original native owner failures.
    pub fn with_accepted_rows<R>(
        &self,
        accepted: &[purrdf_core::evidence::ReasonId],
        visit: impl FnOnce(purrdf_sparql_eval::AcceptedPayloadRows<'_, ScopedFusedRow>) -> R,
    ) -> Result<R, EvalError> {
        self.owner
            .with_accepted_payloads(&self.rows, |row| &row.evidence, accepted, visit)
    }
}

/// One complete caller constraint and the evaluator that decides it. The term
/// is compared by its complete value, including field, datatype and vector bits;
/// a label or field name alone cannot substitute for the admitted request.
#[derive(Debug)]
pub struct ConstraintBinding<'a> {
    /// The original complete value compiled into the request's plan identity.
    pub request: &'a RequestTerm,
    /// The caller's original data/precision/containment evaluator. Its canonical
    /// identity names this complete constraint; it supplies no ranking weight.
    pub evaluator: &'a dyn CandidateConstraint,
}

/// Borrowed bindings for the exact admitted hard-constraint set. Vocabulary and
/// source interpretation stay with these original caller evaluators; missing
/// data is declared unknown, and an operational source failure is a hard error.
#[derive(Debug)]
pub struct ConstraintBindings<'a> {
    bindings: &'a [ConstraintBinding<'a>],
}

impl<'a> ConstraintBindings<'a> {
    /// Retain the caller's original binding view without cloning request values.
    #[must_use]
    pub const fn new(bindings: &'a [ConstraintBinding<'a>]) -> Self {
        Self { bindings }
    }

    /// Bind every admitted constraint exactly once before candidate generation.
    /// Neither a missing constraint nor an extra binding can narrow or expand
    /// the plan under its original content identity. The returned scope owns
    /// admitted counters and the original query evidence dictionary.
    ///
    /// # Errors
    /// Refuses mismatched/duplicate bindings and preserves native admission
    /// errors. No constraint callback runs during this structural binding.
    pub fn bind(
        &self,
        compiled: &impl CompiledRead,
        workspace: &WorkspaceCapability,
    ) -> Result<NativeCandidateScope<'a>, EvalError> {
        let requests = compiled.constraints();
        if self.bindings.len() != requests.len() {
            return Err(NativeDiagnostic::error(
                NativeDiagnosticKind::Config,
                "candidate evaluators do not cover the complete admitted constraint set",
                workspace,
            ));
        }
        let mut evaluators = AdmittedVec::with_capacity(requests.len(), workspace)?;
        for request in requests {
            let mut matching = self
                .bindings
                .iter()
                .filter(|binding| binding.request == request);
            let Some(binding) = matching.next() else {
                return Err(NativeDiagnostic::error(
                    NativeDiagnosticKind::Config,
                    "an admitted hard constraint has no original candidate evaluator",
                    workspace,
                ));
            };
            if matching.next().is_some() {
                return Err(NativeDiagnostic::error(
                    NativeDiagnosticKind::Config,
                    "an admitted hard constraint has duplicate candidate evaluators",
                    workspace,
                ));
            }
            evaluators.push(binding.evaluator)?;
        }
        evaluators
            .as_mut_slice()
            .sort_unstable_by_key(|evaluator| evaluator.identity());
        NativeCandidateScope::new(&evaluators, workspace)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::sync::{Arc, Barrier, mpsc};

    use super::ScopedRows;

    #[test]
    fn live_search_callback_and_original_writer_do_not_invert_dictionary_locks() {
        let workspace = purrdf_sparql_eval::WorkspaceCapability::resident();
        let owner = purrdf_sparql_eval::QueryEvidence::new(&workspace).unwrap();
        let original = owner
            .with_completeness(
                &purrdf_sparql_eval::RowEvidence::default(),
                purrdf_core::evidence::SearchCompleteness::Undetermined,
                None,
            )
            .unwrap();
        let rows = ScopedRows::new(
            purrdf_sparql_eval::AdmittedVec::new(&workspace),
            false,
            &workspace,
        )
        .unwrap();
        rows.retain_search(&original).unwrap();
        let search = rows.search();
        let phases = Arc::new(Barrier::new(2));
        let (finished, completion) = mpsc::channel();
        let reader_phases = Arc::clone(&phases);
        let reader_finished = finished.clone();
        let reader = std::thread::spawn(move || {
            for _ in 0..64 {
                search
                    .with_record(|record| {
                        assert_eq!(
                            record.completeness(),
                            purrdf_core::evidence::SearchCompleteness::Undetermined,
                        );
                        // The dictionary is held before the writer starts.
                        reader_phases.wait();
                        std::thread::yield_now();
                        assert!(!search.retained().unwrap().is_default());
                    })
                    .unwrap();
                reader_phases.wait();
            }
            reader_finished.send(()).unwrap();
        });
        let writer = std::thread::spawn(move || {
            for _ in 0..64 {
                phases.wait();
                rows.retain_search(&original).unwrap();
                phases.wait();
            }
            finished.send(()).unwrap();
        });
        for _ in 0..2 {
            completion
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("original search inspection and publication must make progress");
        }
        reader.join().unwrap();
        writer.join().unwrap();
    }
}
