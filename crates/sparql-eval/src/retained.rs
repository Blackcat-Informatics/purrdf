// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable operational publication with allocation and admission in one owner.

use purrdf_core::small::Shared;

use purrdf_core::{RdfDataset, SolutionRow, SparqlResult, TermValue};

use crate::EvalError;
use crate::QueryExplanation;

fn add(total: &mut u64, bytes: usize) -> Result<(), EvalError> {
    *total = total
        .checked_add(u64::try_from(bytes).map_err(|_| EvalError::WorkspaceBoundOverflow)?)
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TermLayout {
    pub(crate) bytes: u64,
    pub(crate) nodes: usize,
}

pub(crate) fn term_layout(
    value: &TermValue,
    workspace: &crate::WorkspaceCapability,
) -> Result<TermLayout, EvalError> {
    let mut bytes = 0;
    let mut nodes = 0usize;
    let mut admission = None;
    let mut pending: purrdf_core::SmallVec<[&TermValue; 8]> = purrdf_core::smallvec![value];
    while let Some(value) = pending.pop() {
        nodes = nodes
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        match value {
            TermValue::Iri(iri) => add(&mut bytes, iri.capacity())?,
            TermValue::Blank { label, .. } => add(&mut bytes, label.capacity())?,
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                ..
            } => {
                add(&mut bytes, lexical_form.capacity())?;
                add(&mut bytes, datatype.capacity())?;
                add(&mut bytes, language.as_ref().map_or(0, String::capacity))?;
            }
            TermValue::Triple { s, p, o } => {
                add(&mut bytes, 3 * size_of::<TermValue>())?;
                let required = pending
                    .len()
                    .checked_add(3)
                    .ok_or(EvalError::WorkspaceBoundOverflow)?;
                if required > pending.capacity() {
                    let capacity = required
                        .checked_next_power_of_two()
                        .ok_or(EvalError::WorkspaceBoundOverflow)?;
                    let charge = capacity
                        .checked_mul(size_of::<&TermValue>())
                        .ok_or(EvalError::WorkspaceBoundOverflow)?;
                    let next = workspace.charge(
                        u64::try_from(charge).map_err(|_| EvalError::WorkspaceBoundOverflow)?,
                    )?;
                    pending
                        .try_reserve_exact(capacity - pending.len())
                        .map_err(|_| EvalError::AllocationFailed {
                            construct: "owned term layout walk",
                        })?;
                    admission = Some(next);
                }
                pending.extend([&**o, &**p, &**s]);
            }
        }
    }
    drop(pending);
    drop(admission);
    Ok(TermLayout { bytes, nodes })
}

pub(crate) fn result_bytes(
    result: &SparqlResult,
    workspace: &crate::WorkspaceCapability,
) -> Result<u64, EvalError> {
    let mut bytes = 0;
    match result {
        SparqlResult::Solutions {
            variables,
            rows,
            aux,
        } => {
            add(
                &mut bytes,
                variables
                    .capacity()
                    .checked_mul(size_of::<String>())
                    .ok_or(EvalError::WorkspaceBoundOverflow)?,
            )?;
            for variable in variables {
                add(&mut bytes, variable.capacity())?;
            }
            add(
                &mut bytes,
                rows.capacity()
                    .checked_mul(size_of::<SolutionRow>())
                    .ok_or(EvalError::WorkspaceBoundOverflow)?,
            )?;
            for row in rows {
                add(
                    &mut bytes,
                    row.capacity()
                        .checked_mul(size_of::<Option<TermValue>>())
                        .ok_or(EvalError::WorkspaceBoundOverflow)?,
                )?;
                for value in row.iter().flatten() {
                    bytes = bytes
                        .checked_add(term_layout(value, workspace)?.bytes)
                        .ok_or(EvalError::WorkspaceBoundOverflow)?;
                }
            }
            if !aux.retains_admission() {
                add(
                    &mut bytes,
                    aux.query_retained_bytes()
                        .ok_or(EvalError::WorkspaceBoundOverflow)?,
                )?;
            }
        }
        SparqlResult::Graph(graph) => {
            if !graph.retains_admission() {
                add(
                    &mut bytes,
                    graph
                        .query_retained_bytes()
                        .ok_or(EvalError::WorkspaceBoundOverflow)?,
                )?;
            }
        }
        SparqlResult::Boolean(_) => {}
    }
    Ok(bytes)
}

pub(crate) fn warm_result(result: &SparqlResult) -> Result<(), EvalError> {
    match result {
        SparqlResult::Solutions { aux, .. } => aux.warm_query_indexes(),
        SparqlResult::Graph(graph) => graph.warm_query_indexes(),
        SparqlResult::Boolean(_) => Ok(()),
    }
    .map_err(|_| EvalError::AllocationFailed {
        construct: "retained graph query index",
    })
}

pub(crate) trait RetainedLease: std::fmt::Debug + Send + Sync {}
impl<T: std::fmt::Debug + Send + Sync> RetainedLease for T {}

#[derive(Debug)]
struct Payload<T> {
    // Declaration order is deliberate: allocations die before their admission.
    value: T,
    _lease: Box<dyn RetainedLease>,
    _control: crate::WorkspaceAllocation,
}

fn payload<T, L: RetainedLease + 'static>(
    value: T,
    lease: L,
    workspace: &crate::WorkspaceCapability,
) -> Result<Shared<Payload<T>>, EvalError> {
    let bytes = Shared::<Payload<T>>::allocation_layout()
        .size()
        .checked_add(size_of::<L>())
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    let control =
        workspace.charge(u64::try_from(bytes).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
    let lease =
        purrdf_core::small::try_boxed_one(lease).map_err(|_| EvalError::AllocationFailed {
            construct: "retained lease owner",
        })?;
    Shared::try_new(Payload {
        value,
        _lease: lease,
        _control: control,
    })
    .map_err(|_| EvalError::AllocationFailed {
        construct: "retained payload owner",
    })
}

/// An operational receipt sharing the account that admits its published output.
/// Receipt cloning shares allocation; explicit copies through a borrow belong
/// to the caller, just like explicit copies of borrowed solution terms.
#[derive(Debug)]
pub struct RetainedEvidence<E> {
    storage: EvidenceStorage<E>,
}

#[derive(Debug)]
enum EvidenceStorage<E> {
    // A backend receipt already carries its own provider ownership. Keeping it
    // inline lets the first admission refusal be reported without allocating.
    Backend(E),
}

impl<E: Clone> Clone for RetainedEvidence<E> {
    fn clone(&self) -> Self {
        Self {
            storage: match &self.storage {
                EvidenceStorage::Backend(value) => EvidenceStorage::Backend(value.clone()),
            },
        }
    }
}

impl<E> RetainedEvidence<E> {
    /// Keep an already provider-owned receipt without a new engine allocation.
    pub(crate) const fn backend(evidence: E) -> Self {
        Self {
            storage: EvidenceStorage::Backend(evidence),
        }
    }
}

impl<E> std::ops::Deref for RetainedEvidence<E> {
    type Target = E;

    fn deref(&self) -> &E {
        match &self.storage {
            EvidenceStorage::Backend(value) => value,
        }
    }
}

/// Complete or certified partial query output whose allocation remains admitted
/// until its last shared owner dies. Cloning shares payload, never deep-copies it.
#[derive(Debug, Clone)]
pub struct RetainedSparqlResult {
    payload: Shared<Payload<SparqlResult>>,
}

impl RetainedSparqlResult {
    pub(crate) fn new(
        result: SparqlResult,
        lease: impl RetainedLease + 'static,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        Ok(Self {
            payload: payload(result, lease, workspace)?,
        })
    }

    /// Query form in the shared results vocabulary.
    #[must_use]
    pub fn query_form(&self) -> &'static str {
        self.payload.value.query_form()
    }

    /// Borrow the immutable projected names and rows. Explicit caller copies are
    /// caller-owned; the engine's shared output stays under this owner's lease.
    #[must_use]
    pub fn solutions(&self) -> Option<(&[String], &[SolutionRow])> {
        self.payload.value.solutions()
    }

    /// Borrow a CONSTRUCT/DESCRIBE graph without exposing its lease-free Arc.
    #[must_use]
    pub fn graph(&self) -> Option<&RdfDataset> {
        match &self.payload.value {
            SparqlResult::Graph(graph) => Some(graph),
            SparqlResult::Solutions { .. } | SparqlResult::Boolean(_) => None,
        }
    }

    /// Borrow the auxiliary dataset produced by list-building expressions.
    #[must_use]
    pub fn auxiliary_graph(&self) -> Option<&RdfDataset> {
        match &self.payload.value {
            SparqlResult::Solutions { aux, .. } => Some(aux),
            SparqlResult::Graph(_) | SparqlResult::Boolean(_) => None,
        }
    }

    /// The ASK answer, when this is an ASK result.
    #[must_use]
    pub fn boolean(&self) -> Option<bool> {
        match self.payload.value {
            SparqlResult::Boolean(answer) => Some(answer),
            SparqlResult::Solutions { .. } | SparqlResult::Graph(_) => None,
        }
    }

    /// Extract a solution carrier that retains the same admitted allocation.
    ///
    /// # Errors
    /// Returns the original owner for another query form.
    pub fn into_solutions(self) -> Result<RetainedSolutions, Self> {
        if self.solutions().is_some() {
            Ok(RetainedSolutions { result: self })
        } else {
            Err(self)
        }
    }
}

/// An immutable frozen graph whose shared original account survives every
/// shallow clone and extraction. Borrowed datasets expose no raw Arc escape.
#[derive(Debug, Clone)]
pub struct RetainedGraph {
    result: RetainedSparqlResult,
}

impl RetainedGraph {
    pub(crate) fn from_result(result: RetainedSparqlResult) -> Self {
        assert!(
            result.graph().is_some(),
            "graph carrier requires a graph result"
        );
        Self { result }
    }

    /// Borrow the immutable dataset while its original admission remains live.
    #[must_use]
    pub fn as_dataset(&self) -> &RdfDataset {
        self.result
            .graph()
            .expect("graph extraction preserves its query form")
    }

    /// Share the frozen dataset with its original physical admission owner.
    /// This shallow clone allocates no graph or shared control.
    #[must_use]
    pub fn dataset_handle(&self) -> purrdf_core::DatasetHandle {
        match &self.result.payload.value {
            SparqlResult::Graph(graph) => graph.clone(),
            SparqlResult::Solutions { .. } | SparqlResult::Boolean(_) => {
                unreachable!("retained graph preserves its query form")
            }
        }
    }

    /// Move the same original owner into the common result vocabulary.
    #[must_use]
    pub fn into_result(self) -> RetainedSparqlResult {
        self.result
    }
}

impl std::ops::Deref for RetainedGraph {
    type Target = RdfDataset;
    fn deref(&self) -> &Self::Target {
        self.as_dataset()
    }
}

/// Ownership-carrying extraction of a SELECT output.
#[derive(Debug, Clone)]
pub struct RetainedSolutions {
    result: RetainedSparqlResult,
}

/// Certified governed rows with the same retained ownership law as completion.
#[derive(Debug, Clone)]
pub struct RetainedPartialSparqlResult {
    result: RetainedSparqlResult,
    positional_prefix: bool,
}

impl RetainedPartialSparqlResult {
    pub(crate) const fn new(result: RetainedSparqlResult, positional_prefix: bool) -> Self {
        Self {
            result,
            positional_prefix,
        }
    }

    /// Borrow the admitted incomplete output, preserving its certificate.
    #[must_use]
    pub const fn result(&self) -> &RetainedSparqlResult {
        &self.result
    }

    /// Extract the owner without releasing the allocation's admission.
    #[must_use]
    pub fn into_result(self) -> RetainedSparqlResult {
        self.result
    }

    /// Whether the certified rows are the true answer's first rows, in order.
    #[must_use]
    pub const fn is_positional_prefix(&self) -> bool {
        self.positional_prefix
    }
}

impl RetainedSolutions {
    /// The immutable projection and solution bag.
    #[must_use]
    pub fn as_parts(&self) -> (&[String], &[SolutionRow]) {
        self.result
            .solutions()
            .expect("SELECT extraction preserves its query form")
    }
}

/// A measured explanation and its independently owned retained admission.
#[derive(Debug, Clone)]
pub struct RetainedQueryExplanation {
    payload: Shared<Payload<QueryExplanation>>,
}

impl RetainedQueryExplanation {
    pub(crate) fn new(
        explanation: QueryExplanation,
        lease: impl RetainedLease + 'static,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        Ok(Self {
            payload: payload(explanation, lease, workspace)?,
        })
    }
}

impl std::ops::Deref for RetainedQueryExplanation {
    type Target = QueryExplanation;

    fn deref(&self) -> &Self::Target {
        &self.payload.value
    }
}

#[derive(Debug)]
struct DiagnosticPayload {
    // The physical payload dies before its original grant.
    value: purrdf_core::RdfDiagnostic,
    _allocation: crate::WorkspaceAllocation,
}

/// An immutable query diagnostic retaining its original allocation account.
/// Cloning shares payload; borrowing preserves code, location and presentation.
#[derive(Debug, Clone)]
pub struct RetainedDiagnostic {
    payload: Shared<DiagnosticPayload>,
}

impl RetainedDiagnostic {
    /// Transfer a diagnostic whose complete heap layout is already covered by
    /// allocation. The grant must also cover the concrete Shared control layout
    /// before invoking this allocation-first publication.
    pub(crate) fn from_admitted(
        value: purrdf_core::RdfDiagnostic,
        allocation: crate::WorkspaceAllocation,
    ) -> Result<Self, EvalError> {
        let original = DiagnosticPayload {
            value,
            _allocation: allocation,
        };
        let payload =
            Shared::try_new_with(|| original).map_err(|_| EvalError::AllocationFailed {
                construct: "retained diagnostic owner",
            })?;
        Ok(Self { payload })
    }

    /// Render borrowed native error fields through one original admission.
    /// The diagnostic text and code are allocated only after their concrete
    /// layouts are admitted; the control header is added before publication.
    pub(crate) fn render(
        code: &str,
        message: &(impl std::fmt::Display + ?Sized),
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let value = {
            let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
            purrdf_core::RdfDiagnostic::try_error_with_memory(code, message, &mut memory)
        }
        .map_err(|error| frame.storage_error(error, "retained diagnostic text"))?;
        let total = frame
            .admitted_bytes()
            .checked_add(Self::control_bytes())
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        frame.resize_live(total)?;
        let allocation = frame
            .into_allocation()
            .expect("nonzero diagnostic control layout has its original grant");
        Self::from_admitted(value, allocation)
    }

    /// The resident public boundary keeps an already caller-owned diagnostic.
    /// Operational bounded paths must supply its original admitted producer owner.
    pub(crate) fn resident(value: purrdf_core::RdfDiagnostic) -> Result<Self, EvalError> {
        let allocation = crate::WorkspaceCapability::default().charge(0)?;
        Self::from_admitted(value, allocation)
    }

    pub(crate) fn control_bytes() -> usize {
        Shared::<DiagnosticPayload>::allocation_layout().size()
    }

    /// Borrow the immutable diagnostic; explicit caller copies use caller storage.
    #[must_use]
    pub fn diagnostic(&self) -> &purrdf_core::RdfDiagnostic {
        &self.payload.value
    }
}

impl std::ops::Deref for RetainedDiagnostic {
    type Target = purrdf_core::RdfDiagnostic;
    fn deref(&self) -> &Self::Target {
        self.diagnostic()
    }
}

impl std::fmt::Display for RetainedDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.diagnostic(), f)
    }
}

impl std::error::Error for RetainedDiagnostic {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.diagnostic())
    }
}

impl PartialEq for RetainedDiagnostic {
    fn eq(&self, other: &Self) -> bool {
        self.diagnostic() == other.diagnostic()
    }
}
impl Eq for RetainedDiagnostic {}
