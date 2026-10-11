// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact physical strategies over the existing logical algebra.
//!
//! A provider is consulted after rewriting. Accepting a strategy is a promise
//! to preserve the ordered bag, bindings, and retained search/evidence owners.
//! Returning `None` declines that promise. An accepted strategy's operational
//! failure is propagated; execution never retries it through a different plan.

pub(crate) mod barrier;
pub(crate) mod bucket;
pub(crate) mod dictionary;
pub(crate) mod runtime;
pub mod spans;
pub(crate) mod window;
pub use runtime::ShapeReport;

use crate::property_fn::PropertyFunctionRegistry;
use crate::solution::VarSchema;
use crate::{AdmittedVec, EvalError, WorkspaceAllocation, WorkspaceCapability, WorkspaceTerm};
use purrdf_core::AccessCost;
use purrdf_sparql_algebra::{GraphPattern, TriplePattern, Variable};
use std::sync::Arc;

/// Comparable work estimates, never storage admission or a governor fee.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShapeEstimate {
    /// Estimated input/output access work.
    pub access: Option<AccessCost>,
}

/// The rewritten node and its actual execution configuration.
#[derive(Clone, Copy)]
pub struct ShapeRequest<'a> {
    /// The original logical subtree. Offers must not retain this borrow.
    pub pattern: &'a GraphPattern,
    /// Exact original output column order.
    pub schema: &'a VarSchema,
    /// Caller-owned producers; spelling an IRI alone confers no capability.
    pub relations: &'a PropertyFunctionRegistry,
    /// True when the current invocation reads a named graph.
    pub named_graph: bool,
    /// The invocation has an explicit evidence source. Its original expression
    /// callbacks may retain an undecided row even when ordinary EBV is false.
    pub has_evidence_source: bool,
    /// The actual view's landed access-cost law and active graph scope.
    pub costs: &'a dyn ShapeCosts,
}
purrdf_hash::debug_non_exhaustive!([ 'a ] ShapeRequest<'a> { named_graph, has_evidence_source });

/// Native planner access information, without exposing or remapping view IDs.
pub trait ShapeCosts {
    /// Visit the original host-provided cost of each admitted graph slice.
    /// These values rank work; they certify neither memory nor exact COUNT.
    /// # Errors
    /// Preserves original term lookup, source and native storage failures.
    fn visit(
        &self,
        pattern: &TriplePattern,
        visit: &mut dyn FnMut(AccessCost),
    ) -> Result<(), EvalError>;
}

/// A host's exact physical knowledge, registered independently of parser meaning.
pub trait ShapeProvider: Send + Sync {
    /// Offer a strategy or decline without changing the logical result.
    ///
    /// # Errors
    /// Preserve allocation, source, and contained host failures.
    fn offer(
        &self,
        request: ShapeRequest<'_>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<ShapeOffer>, EvalError>;
}

/// A compiled immutable strategy. It cannot manufacture dataset or evidence IDs.
pub trait ShapeOperator: Send + Sync {
    /// Execute through engine-owned native input and publication operations.
    /// A withheld input must cause the strategy to return without publishing rows.
    ///
    /// # Errors
    /// Preserve the first operational failure; no literal fallback follows it.
    fn execute(&self, runtime: &mut dyn ShapeRuntime) -> Result<(), EvalError>;
}

/// An invocation-local native input. Its rows remain owned by the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeInput {
    slot: usize,
    length: usize,
}
impl ShapeInput {
    /// Number of rows in this complete input.
    pub const fn len(self) -> usize {
        self.length
    }
    /// Whether the input is empty.
    pub const fn is_empty(self) -> bool {
        self.length == 0
    }
    /// A reference to an existing retained row, never a detached cell buffer.
    pub const fn row(self, ordinal: usize) -> Option<ShapeRow> {
        if ordinal < self.length {
            Some(ShapeRow {
                input: self.slot,
                ordinal,
            })
        } else {
            None
        }
    }
}

/// An opaque reference to a row of this runtime invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeRow {
    input: usize,
    ordinal: usize,
}

/// An admitted computed binding; the original producer lease accompanies it.
#[derive(Debug)]
pub struct ShapeBinding {
    /// Destination variable in the original logical schema.
    pub variable: Variable,
    /// Native value with its original before-allocation admission.
    pub value: WorkspaceTerm,
}

/// The object-safe host execution boundary. The engine owns all native IDs,
/// rows, dictionaries and whole search handles for the lifetime of the call.
pub trait ShapeRuntime {
    /// Original admission account shared with this query and its retained output.
    fn workspace(&self) -> &WorkspaceCapability;
    /// Evaluate an original subtree, indexed by classified child ordinals.
    /// An empty path evaluates the offered root literally, without selecting it again.
    /// `None` means the original lift withheld this input; no output may follow.
    fn input(&mut self, path: &[usize]) -> Result<Option<ShapeInput>, EvalError>;
    /// Borrow an input's original schema.
    fn schema(&self, input: ShapeInput) -> Result<&VarSchema, EvalError>;
    /// Copy a bound cell through the existing before-copy native term home.
    fn binding(
        &self,
        row: ShapeRow,
        variable: &Variable,
    ) -> Result<Option<WorkspaceTerm>, EvalError>;
    /// Publish compatible original rows in the offered root's original schema.
    /// Shared cells use native identity; row evidence is conjoined by its original home.
    /// Returns false for an ordinary incompatible binding.
    fn emit(&mut self, rows: &[ShapeRow]) -> Result<bool, EvalError>;
    /// The same publication with native computed bindings. Grants are transferred,
    /// not replaced with a measurement of the finished payload.
    fn emit_with(
        &mut self,
        rows: &[ShapeRow],
        bindings: AdmittedVec<ShapeBinding>,
    ) -> Result<bool, EvalError>;
    /// Count source visits, transitions and emitted rows separately from estimates.
    /// False is the original governor stop; no further output may be emitted.
    fn record_work(&mut self, work: ShapeWork) -> Result<bool, EvalError>;
}

/// Actual physical work, independent of the planner's estimates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShapeWork {
    /// Once-read source items.
    pub source_visits: u64,
    /// State transitions or candidate comparisons.
    pub transitions: u64,
    /// Output rows (including required duplicate derivations).
    pub output_rows: u64,
}

/// An immutable strategy and the admission of its actual control allocation.
pub struct ShapeOffer {
    pub(crate) operator: OperatorOwner,
    pub(crate) provider: Option<&'static str>,
    /// Stable caller-supplied operator identity (not a vocabulary default).
    pub name: &'static str,
    /// Comparable work estimate, if the provider can state one honestly.
    pub estimate: ShapeEstimate,
}
pub(crate) enum OperatorOwner {
    Caller(Arc<dyn ShapeOperator>),
    Native {
        value: Box<dyn ShapeOperator>,
        _allocation: WorkspaceAllocation,
    },
    Builtin(Builtin),
}
pub(crate) enum Builtin {
    Dictionary,
    ExpandedDictionary(dictionary::ExpandedDictionary),
    Bucket,
    Window(window::WindowSpec),
}
impl ShapeOffer {
    pub(crate) fn builtin(name: &'static str, kind: Builtin) -> Self {
        Self {
            operator: OperatorOwner::Builtin(kind),
            provider: Some("native"),
            name,
            estimate: ShapeEstimate::default(),
        }
    }
    /// Offer a prebuilt caller-owned immutable strategy.
    pub fn from_shared(
        name: &'static str,
        operator: Arc<dyn ShapeOperator>,
        estimate: ShapeEstimate,
    ) -> Self {
        Self {
            operator: OperatorOwner::Caller(operator),
            provider: None,
            name,
            estimate,
        }
    }
    /// Admit the exact box before construction. Any captured native buffers must
    /// already carry their own original grants (for example `AdmittedVec`).
    ///
    /// # Errors
    /// Checked layout overflow, storage refusal, or physical allocator failure.
    pub fn new_admitted<T: ShapeOperator + 'static>(
        name: &'static str,
        operator: T,
        estimate: ShapeEstimate,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let allocation = workspace.charge(
            u64::try_from(size_of::<T>()).map_err(|_| EvalError::WorkspaceBoundOverflow)?,
        )?;
        let value =
            purrdf_core::small::try_boxed(operator).map_err(|_| EvalError::AllocationFailed {
                construct: "physical shape operator",
            })?;
        Ok(Self {
            operator: OperatorOwner::Native {
                value,
                _allocation: allocation,
            },
            provider: None,
            name,
            estimate,
        })
    }
    pub(crate) fn execute(&self, runtime: &mut dyn ShapeRuntime) -> Result<(), EvalError> {
        match &self.operator {
            OperatorOwner::Caller(value) => value.execute(runtime),
            OperatorOwner::Native { value, .. } => value.execute(runtime),
            OperatorOwner::Builtin(_) => {
                unreachable!("native shapes use the generic engine adapter")
            }
        }
    }
}
impl core::fmt::Debug for ShapeOffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ShapeOffer")
            .field("name", &self.name)
            .field("estimate", &self.estimate)
            .finish_non_exhaustive()
    }
}

/// Caller-owned provider registrations. Registration order is the deterministic
/// tie breaker; the first exact offer is used. Built-ins are tried afterwards.
#[derive(Clone)]
pub struct ShapeRegistry {
    pub(crate) providers: Vec<(&'static str, Arc<dyn ShapeProvider>)>,
    id: crate::registry_id::RegistryId,
}
impl ShapeRegistry {
    /// Canonical empty host registry; general built-ins remain available.
    pub const EMPTY: Self = Self {
        providers: Vec::new(),
        id: crate::registry_id::RegistryId::EMPTY,
    };
    /// A caller-owned mutable registration table.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            id: crate::registry_id::RegistryId::fresh(),
        }
    }
    /// Register a caller-owned immutable provider under a stable report name.
    pub fn register(&mut self, name: &'static str, provider: Arc<dyn ShapeProvider>) {
        self.providers.push((name, provider));
        self.id = crate::registry_id::RegistryId::fresh();
    }
    /// Instance identity; cloning preserves it. It is separate from parser meaning.
    pub const fn id(&self) -> crate::registry_id::RegistryId {
        self.id
    }
}
purrdf_hash::default_from_new!(ShapeRegistry);
impl core::fmt::Debug for ShapeRegistry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ShapeRegistry")
            .field("id", &self.id)
            .field("providers", &self.providers.len())
            .finish()
    }
}

pub(crate) use runtime::eval_shape;
