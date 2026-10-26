// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The error every shapes-graph entry point returns.
//!
//! Most of what can go wrong while turning a shapes document into a verdict is a message for
//! a human — a syntax error, an unsupported construct, a SPARQL body that does not parse —
//! and is carried as one ([`ShapesError::Invalid`]). An incomplete `owl:imports` closure is
//! different: a caller can act on it programmatically, by supplying the
//! documents it names, and it must read the same on every host. So it is its own variant,
//! carrying the typed [`ShapesImportError`], and a caller branches on the variant rather
//! than on message text. A shapes graph that uses the SHACL JavaScript Extensions is the
//! other: SHACL-JS is a 2017 Working Group Note, not SHACL 1.2, and this engine has no
//! JavaScript engine, so a shape that reaches SHACL-JS is refused typed
//! ([`ShapesError::ShaclJs`]), naming the extension, the node and the term, rather than
//! reading like a malformed graph. SHACL-JS that no shape reaches is declared vocabulary
//! and loads.
//!
//! Materialization can refuse a logical capacity or physical allocation. Its original
//! [`ShapesError::Storage`] cause survives query, expression and rule execution; it is
//! not a malformed-shape verdict.
//!
//! Two refusals of SHACL-SPARQL and SHACL-AF declarations are typed as well, because they
//! answer to two different sentences of the specifications and a caller must be able to
//! tell them apart:
//!
//! * [`ShapesError::IllFormed`] — a declaration violates a SYNTAX RULE. SHACL 1.2 Core,
//!   "Handling of Ill-formed Shapes Graphs": "If the shapes graph contains ill-formed
//!   nodes, then the result of the validation process is undefined. A SHACL processor
//!   SHOULD produce a failure in this case." The sentence has no reachability qualifier,
//!   so the load fails whether or not any shape reaches the declaration, and the
//!   refusal lists EVERY such violation in the graph, each naming its declaration and
//!   syntax rule ([`IllFormedShapesGraph`]).
//! * [`ShapesError::Prebinding`] — a query that is EXECUTED with pre-bound variables
//!   violates a pre-binding restriction. SHACL 1.2 SPARQL Extensions, Appendix A:
//!   "SHACL-SPARQL processors MUST report a failure when it is operating on a shapes
//!   graph that contains SHACL-SPARQL queries (via sh:ask, sh:construct and sh:select)
//!   that are executed with pre-bound variables and violate any of these MUST
//!   restrictions." The failure is scoped to queries that execute, so it is raised where
//!   the engine determines that a query executes ([`PrebindingViolation`]); the same
//!   query nowhere executed is reported by `lint` instead (its `unexecuted` section).

use std::fmt;

use crate::imports::ShapesImportError;

/// Why a shapes-graph entry point refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapesError {
    /// A recursive shapes catalog requires an explicit recursion semantics.
    Recursion(RecursionRefusal),
    /// The shapes model could not be prepared within the native stack margin.
    Preparation(PreparationRefusal),
    /// Logical capacity or physical storage refused an operation.
    Storage(purrdf_lex::allocation::StorageError),
    /// The shapes graph's `owl:imports` closure is not in hand, or the import table the
    /// caller supplied cannot be used. See [`crate::imports`].
    Imports(ShapesImportError),
    /// A shape of the shapes graph reaches a construct of the SHACL JavaScript
    /// Extensions (SHACL-JS). See [`ShaclJsRefusal`].
    ShaclJs(ShaclJsRefusal),
    /// A SHACL-SPARQL or SHACL-AF declaration of the shapes graph, or a node's
    /// `sh:message` values (`message-datatype`), violates a syntax rule, whether or not
    /// any shape reaches it. See [`IllFormedShapesGraph`].
    IllFormed(IllFormedShapesGraph),
    /// A query the engine executes with pre-bound variables violates a pre-binding
    /// restriction. See [`PrebindingViolation`].
    Prebinding(PrebindingViolation),
    /// A shape declares a SHACL-AF custom target (`sh:target`) this engine cannot
    /// compute. See [`UnsupportedTargetRefusal`].
    UnsupportedTarget(UnsupportedTargetRefusal),
    /// A `sh:SPARQLTarget`'s `sh:ask` and its `sh:select` answer two ways for one node a
    /// candidate check asked about. See [`SparqlTargetDisagreement`].
    SparqlTargetDisagreement(SparqlTargetDisagreement),
    /// The processor itself failed or was stopped, as opposed to a node expression or
    /// query failing to evaluate: a governor trip, a stop signal, a recursion bound, a
    /// query-engine failure that must propagate. See [`ProcessorFailure`].
    Processor(ProcessorFailure),
    /// Anything else: a document that does not parse, an unsupported or malformed SHACL
    /// construct, a failure during evaluation. The engine's own diagnostic.
    Invalid(String),
}

impl ShapesError {
    /// Add evaluation context to a textual diagnostic while retaining typed refusals.
    pub(crate) fn context(self, prefix: &str) -> Self {
        match self {
            Self::Invalid(message) => Self::Invalid(format!("{prefix}{message}")),
            error => error,
        }
    }

    /// The import refusal, when this is one.
    #[must_use]
    pub const fn as_imports(&self) -> Option<&ShapesImportError> {
        match self {
            Self::Imports(error) => Some(error),
            Self::ShaclJs(_)
            | Self::Recursion(_)
            | Self::Preparation(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Storage(_)
            | Self::Processor(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The SHACL-JS refusal, when this is one.
    #[must_use]
    pub const fn as_shacl_js(&self) -> Option<&ShaclJsRefusal> {
        match self {
            Self::ShaclJs(refusal) => Some(refusal),
            Self::Imports(_)
            | Self::Recursion(_)
            | Self::Preparation(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Storage(_)
            | Self::Processor(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The syntax-rule refusal, when this is one.
    #[must_use]
    pub const fn as_ill_formed(&self) -> Option<&IllFormedShapesGraph> {
        match self {
            Self::IllFormed(refusal) => Some(refusal),
            Self::Imports(_)
            | Self::Recursion(_)
            | Self::Preparation(_)
            | Self::ShaclJs(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Storage(_)
            | Self::Processor(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The unsupported-custom-target refusal, when this is one.
    #[must_use]
    pub const fn as_unsupported_target(&self) -> Option<&UnsupportedTargetRefusal> {
        match self {
            Self::UnsupportedTarget(refusal) => Some(refusal),
            Self::Imports(_)
            | Self::Recursion(_)
            | Self::Preparation(_)
            | Self::ShaclJs(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Storage(_)
            | Self::Processor(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The `sh:ask` / `sh:select` disagreement refusal, when this is one.
    #[must_use]
    pub const fn as_sparql_target_disagreement(&self) -> Option<&SparqlTargetDisagreement> {
        match self {
            Self::SparqlTargetDisagreement(refusal) => Some(refusal),
            Self::Imports(_)
            | Self::Recursion(_)
            | Self::Preparation(_)
            | Self::ShaclJs(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::Storage(_)
            | Self::Processor(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The pre-binding refusal, when this is one.
    #[must_use]
    pub const fn as_prebinding(&self) -> Option<&PrebindingViolation> {
        match self {
            Self::Prebinding(violation) => Some(violation),
            Self::Imports(_)
            | Self::Recursion(_)
            | Self::Preparation(_)
            | Self::ShaclJs(_)
            | Self::IllFormed(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Storage(_)
            | Self::Processor(_)
            | Self::Invalid(_) => None,
        }
    }
}

/// Who a failure belongs to, for a caller that must decide whether the failure is the
/// answer to ONE evaluation or the end of the whole run. See [`ShapesError::failure_class`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureClass {
    /// A node expression, function body or query evaluated to a failure: an arity or
    /// list-argument cardinality mismatch, a type error, a non-boolean where a boolean is
    /// required, a body that produced the wrong number of nodes. SHACL 1.2 SPARQL
    /// Extensions, "Evaluation of Custom SPARQL Functions": "if the evaluation produces an
    /// evaluation failure, the result of the SPARQL function call is an error".
    Evaluation,
    /// The processor failed or was stopped: storage refusal, governor trip, stop signal,
    /// recursion bound, a query-engine failure that must propagate. No verdict, and no
    /// SPARQL error value, can be computed from such a run, so it ends the validation.
    Processor,
    /// A typed refusal of the shapes graph itself (imports, SHACL-JS, ill-formed
    /// declaration, pre-binding restriction, unsupported target, target disagreement).
    /// Not an evaluation outcome of any one expression, so it too ends the validation.
    Refusal,
}

impl ShapesError {
    /// Whether this is an evaluation failure, a processor failure or a refusal of the
    /// shapes graph. The match is exhaustive, so a new variant must choose a class.
    ///
    /// [`Self::Invalid`] holds the evaluation failures, and what else a `String` error
    /// converts into: arity, list-argument cardinality, an unresolvable shape argument, a
    /// wrong node count, a malformed value, and query text a SHACL body supplied that the
    /// engine refuses with a non-propagating diagnostic code. It must NOT hold a failure
    /// of the processor: storage refusal, governor trip, stop signal, recursion bound,
    /// internal defect and propagating query-engine failure are [`Self::Storage`] or
    /// [`Self::Processor`].
    #[must_use]
    pub const fn failure_class(&self) -> FailureClass {
        match self {
            Self::Invalid(_) => FailureClass::Evaluation,
            Self::Storage(_) | Self::Processor(_) | Self::Preparation(_) => FailureClass::Processor,
            Self::Imports(_)
            | Self::Recursion(_)
            | Self::ShaclJs(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_) => FailureClass::Refusal,
        }
    }

    /// An internal defect: an invariant of the processor was violated (a lowering that
    /// does not match its expression, a function whose body was never installed). Not an
    /// outcome of evaluating any input, so it is a [`ProcessorFailure::Internal`].
    pub(crate) fn internal(what: impl Into<String>) -> Self {
        Self::Processor(ProcessorFailure::Internal { what: what.into() })
    }

    /// The processor failure, when this is one.
    #[must_use]
    pub const fn as_processor(&self) -> Option<&ProcessorFailure> {
        match self {
            Self::Processor(failure) => Some(failure),
            Self::Imports(_)
            | Self::Preparation(_)
            | Self::Recursion(_)
            | Self::ShaclJs(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Storage(_)
            | Self::Invalid(_) => None,
        }
    }
}

/// A failure of the processor rather than of an evaluation.
///
/// Each variant is a way the run cannot continue whatever the expression under
/// evaluation means, so none of them may be folded into a SPARQL error value or an
/// unbound variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessorFailure {
    /// Evaluating a node expression would enter the physical stack margin.
    ExpressionStackMargin,
    /// Calling an original custom function would enter the physical stack margin.
    CallStackMargin {
        /// Original function IRI.
        iri: String,
    },
    /// A shape re-entry would enter the physical stack margin.
    ReentryStackMargin {
        /// Original shape identity in RDF term spelling.
        shape: String,
    },
    /// Validating an original shape would enter the physical stack margin.
    ValidationStackMargin {
        /// Original native shape identity.
        shape: Box<crate::term::Term>,
    },
    /// A SPARQL query exhausted an execution governor (fuel, rows, cells, scratch, UDF
    /// depth, remote requests) or was refused at admission.
    Governor(purrdf_core::governor::TrippedGovernor),
    /// The host's stop signal fired between two evaluations.
    Stopped(purrdf_core::governor::StopCause),
    /// A relation the query invoked declared its index was not whole.
    IncompleteRelation {
        /// Each relation IRI with the reason it gave.
        relations: Vec<String>,
    },
    /// A custom function's re-entry depth bound was reached.
    CallDepth {
        /// The function IRI whose call crossed the bound.
        iri: String,
        /// The bound.
        limit: u32,
    },
    /// A node-expression tree nested past the structural bound.
    ExpressionDepth {
        /// The depth that was refused.
        depth: u32,
        /// The bound.
        limit: u32,
    },
    /// Nested filter/exists validation crossed the processor's recursion bound.
    ValidationDepth {
        /// The shape at the refused re-entry.
        shape: String,
        /// The depth that was refused.
        depth: u32,
        /// The bound.
        limit: u32,
    },
    /// A `(shape, focus)` pair re-entered itself.
    ReentryCycle {
        /// The shape, as the engine renders its id.
        shape: String,
        /// The focus node, as the engine renders a term.
        focus: String,
    },
    /// An invariant of the processor itself was violated: a validation-plan lowering that
    /// does not describe the expression beside it, or a custom function whose body was never
    /// installed. A defect in the engine, never in the data or the shapes graph, so it is
    /// never an evaluation failure.
    Internal {
        /// The engine's diagnostic.
        what: String,
    },
    /// The dated query law refused a query. The refusal is also recorded on the request's
    /// law runtime, which the dated entry re-reports as the typed cause; the call that met it
    /// has no SPARQL-error reading, because the query was never admitted to run.
    Admission {
        /// The rendered refusal.
        message: String,
    },
    /// The query engine failed with a diagnostic whose stable code requires
    /// propagation (execution, storage, resource and invariant failures; see
    /// `EvalError::diagnostic_requires_propagation`).
    Query {
        /// The diagnostic's stable code.
        code: String,
        /// The rendered diagnostic.
        message: String,
    },
}

impl fmt::Display for ProcessorFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpressionStackMargin => {
                f.write_str("node expression evaluation exhausted its stack margin")
            }
            Self::CallStackMargin { iri } => write!(
                f,
                "custom node-expression function <{iri}> exhausted its stack margin"
            ),
            Self::ReentryStackMargin { shape } => write!(
                f,
                "SHACL node-expression re-entry exhausted its stack margin at shape {shape}"
            ),
            Self::ValidationStackMargin { shape } => write!(
                f,
                "SHACL validation exhausted its stack margin at shape {shape}"
            ),
            Self::Governor(tripped) => write!(
                f,
                "validation budget exhausted: {tripped}; a conformance verdict cannot be \
                 computed from a truncated solution bag"
            ),
            Self::Stopped(cause) => write!(
                f,
                "validation stopped between evaluations ({cause:?}); a conformance verdict \
                 cannot be computed from a validation that did not finish"
            ),
            Self::IncompleteRelation { relations } => write!(
                f,
                "a relation this query invoked declares its index was not whole ({}); a \
                 conformance verdict cannot be computed over an index that was not whole",
                relations.join("; ")
            ),
            Self::CallDepth { iri, limit } => write!(
                f,
                "custom node-expression function <{iri}> re-entry depth exceeded ({limit}): \
                 the call chain is recursive and has been refused rather than allowed to \
                 exhaust the native stack"
            ),
            Self::ExpressionDepth { depth, limit } => write!(
                f,
                "node expression nesting depth exceeded ({depth} > {limit}): the expression \
                 tree is nested past the structural limit"
            ),
            Self::ValidationDepth {
                shape,
                depth,
                limit,
            } => write!(
                f,
                "SHACL validation recursion depth exceeded ({depth} > {limit}) at shape \
                 {shape}: a cyclic sh:filterShape / sh:exists reference"
            ),
            Self::ReentryCycle { shape, focus } => write!(
                f,
                "recursive node expression detected: shape {shape} re-entered for focus {focus}"
            ),
            Self::Internal { what } => f.write_str(what),
            Self::Admission { message } => f.write_str(message),
            Self::Query { message, .. } => f.write_str(message),
        }
    }
}

impl std::error::Error for ProcessorFailure {}

impl fmt::Display for ShapesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preparation(refusal) => refusal.fmt(f),
            Self::Recursion(refusal) => refusal.fmt(f),
            Self::Storage(error) => error.fmt(f),
            Self::Imports(error) => error.fmt(f),
            Self::ShaclJs(refusal) => refusal.fmt(f),
            Self::IllFormed(refusal) => refusal.fmt(f),
            Self::Prebinding(violation) => violation.fmt(f),
            Self::UnsupportedTarget(refusal) => refusal.fmt(f),
            Self::SparqlTargetDisagreement(refusal) => refusal.fmt(f),
            Self::Processor(failure) => failure.fmt(f),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ShapesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preparation(refusal) => Some(refusal),
            Self::Recursion(refusal) => Some(refusal),
            Self::Storage(error) => Some(error),
            Self::Imports(error) => Some(error),
            Self::ShaclJs(refusal) => Some(refusal),
            Self::IllFormed(refusal) => Some(refusal),
            Self::Prebinding(violation) => Some(violation),
            Self::UnsupportedTarget(refusal) => Some(refusal),
            Self::SparqlTargetDisagreement(refusal) => Some(refusal),
            Self::Processor(failure) => Some(failure),
            Self::Invalid(_) => None,
        }
    }
}

/// A preparation refusal, distinct from malformed shapes and nonconformance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparationRefusal {
    /// A selected recursive execution lacks a proved original dependency scope.
    UnqualifiedRecursion(crate::recursion::ClosedRecursionRefusal),
    /// The original native workspace admission or allocation refusal.
    Storage(purrdf_lex::allocation::StorageError),
    /// A custom call has no declaration in its retained model domain.
    UnknownFunction(crate::term::NamedNode),
    /// A function IRI has distinct declarations in one model or preparation.
    DuplicateFunction(crate::term::NamedNode),
    /// A retained declaration has no installed body.
    MissingFunctionBody(crate::term::NamedNode),
    /// A computed shape reference has no retained model domain.
    UnboundModel,
    /// A computed shape reference outlived its retained definition table.
    ExpiredIndex,
    /// A computed shape reference names another model's definition table.
    ForeignIndex,
    /// A recursive lowering or model-codec frame would enter the stack margin.
    StackMargin,
    /// Two catalog definitions declare the same identity.
    DuplicateDefinition(std::sync::Arc<crate::term::Term>),
    /// A definition owns a catalog instead of being a bare catalog member.
    OwningDefinition(std::sync::Arc<crate::term::Term>),
    /// A reference was used outside an owning catalog root.
    UnboundReference,
    /// A catalog reference names no definition in that catalog.
    UnknownReference(std::sync::Arc<crate::term::Term>),
    /// A property edge names a catalog definition that is not a property shape.
    ExpectedProperty(std::sync::Arc<crate::term::Term>),
    /// A model's definition cell was installed more than once.
    InitializedIndex,
}

impl PreparationRefusal {
    /// Stop a model-inspection walk at its first refusal or physical stack bound.
    pub(crate) fn admit_walk(refusal: &mut Option<Self>) -> bool {
        if refusal.is_some() {
            return false;
        }
        if purrdf_stack::is_low() {
            *refusal = Some(Self::StackMargin);
            return false;
        }
        true
    }
}

impl fmt::Display for PreparationRefusal {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnqualifiedRecursion(refusal) => refusal.fmt(out),
            Self::Storage(error) => error.fmt(out),
            Self::UnknownFunction(iri) => write!(out, "custom function {iri} has no declaration in the retained model; keep the exported model owner with its expression"),
            Self::DuplicateFunction(iri) => write!(out, "custom function {iri} has distinct declarations; retain one original declaration per function IRI"),
            Self::MissingFunctionBody(iri) => write!(out, "custom function {iri} has no installed body; link its declaration before preparation"),
            Self::UnboundModel => out.write_str("computed shape reference has no retained model; keep its exported owning root"),
            Self::ExpiredIndex => out.write_str("computed shape reference outlived its definition table; keep its exported owning root"),
            Self::ForeignIndex => out.write_str("computed shape reference names a different model's definition table"),
            Self::StackMargin => out.write_str("SHACL preparation exhausted its stack margin; flatten the model or prepare it on a larger stack"),
            Self::DuplicateDefinition(id) => write!(out, "duplicate shape definition {id}; provide one definition per RDF identity"),
            Self::OwningDefinition(id) => write!(out, "catalog definition {id} already owns a catalog; provide bare definitions"),
            Self::UnboundReference => out.write_str("a shape identity reference has no catalog; obtain an owning root through ModelOwner::root"),
            Self::UnknownReference(id) => write!(out, "shape reference {id} has no catalog definition; provide its definition"),
            Self::ExpectedProperty(id) => write!(out, "property reference {id} does not name a property-shape definition; provide a shape with its own sh:path"),
            Self::InitializedIndex => out.write_str("model definitions were already installed; link the original model exactly once"),
        }
    }
}

purrdf_hash::error_sources!(PreparationRefusal, Storage, UnqualifiedRecursion);

purrdf_lex::variant_from!(ShapesError { Preparation(PreparationRefusal) });

/// How a shape reference affects the containing constraint's truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecursionPolarity {
    /// Growing the referenced conformance relation preserves satisfaction.
    Positive,
    /// Growing the referenced conformance relation can remove satisfaction.
    Negative,
    /// The dependency can have either sign, or its truth transfer is unproved.
    Both,
    /// A where-target dependency, separate from the conformance formula.
    Target,
}

impl RecursionPolarity {
    pub(crate) const fn inverted(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
            other => other,
        }
    }
}

/// One reference inside a recursive strongly connected component.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecursionDependency {
    /// Source shape, in native RDF term spelling.
    pub source: String,
    /// Referenced shape, in native RDF term spelling.
    pub target: String,
    /// Original shape/query parameter IRI, or an explicit native operation.
    pub parameter: String,
    /// The reference's role in conformance or target selection.
    pub polarity: RecursionPolarity,
}

/// Why an original recursive component cannot use the requested semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecursionRefusalReason {
    /// The request supplied no fixed-point selection.
    MissingSelection,
    /// A negative or nonmonotone dependency lies inside a recursive component.
    NonStratified {
        /// The explicitly selected law, retained in the refusal.
        semantics: crate::recursion::RecursionSemantics,
    },
    /// The run cannot admit another original focus/definition pair.
    PairLimit {
        /// Maximum pairs admitted by the request.
        limit: u64,
        /// Pairs already admitted when the next pair was refused.
        consumed: u64,
    },
    /// The run cannot start another evaluation round.
    IterationLimit {
        /// Maximum rounds admitted by the request.
        limit: u64,
        /// Rounds already consumed before this refusal.
        consumed: u64,
    },
}

/// An exact original dependency, indexed into a refusal's scoped definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopedRecursionDependency {
    /// Original source definition index, without lexical identity folding.
    pub source: usize,
    /// Original target definition index, without lexical identity folding.
    pub target: usize,
    /// Original parameter or native operation.
    pub parameter: &'static str,
    /// Original conformance/target polarity.
    pub polarity: RecursionPolarity,
}

#[derive(Debug)]
pub(crate) struct RecursionComponent {
    pub(crate) identities: Vec<(usize, usize)>,
    pub(crate) members: Vec<String>,
    pub(crate) definitions: Vec<crate::recursion::RecursionDefinition>,
    pub(crate) edges: Vec<RecursionDependency>,
    pub(crate) scoped_edges: Vec<ScopedRecursionDependency>,
}

impl PartialEq for RecursionComponent {
    fn eq(&self, other: &Self) -> bool {
        self.members == other.members
            && self.edges == other.edges
            && self.scoped_edges == other.scoped_edges
            && self.identities == other.identities
            && self.definitions.len() == other.definitions.len()
            && self
                .definitions
                .iter()
                .zip(&other.definitions)
                .all(|(a, b)| a.definition() == b.definition() && a.kind() == b.kind())
    }
}
impl Eq for RecursionComponent {}

/// A recursive component or an original-pair/round limit refusal.
/// Component payloads are immutable; cloning retains their original allocation.
#[derive(Clone)]
pub struct RecursionRefusal {
    /// The actionable distinction between absent selection, invalid strata and work.
    pub reason: RecursionRefusalReason,
    component: Option<purrdf_lex::allocation::Shared<RecursionComponent>>,
}
impl RecursionRefusal {
    pub(crate) fn work(reason: RecursionRefusalReason) -> Self {
        Self {
            reason,
            component: None,
        }
    }
    pub(crate) fn component(
        reason: RecursionRefusalReason,
        component: RecursionComponent,
    ) -> Result<Self, purrdf_lex::allocation::StorageError> {
        Ok(Self {
            reason,
            component: Some(
                purrdf_lex::allocation::Shared::try_new(component)
                    .map_err(|_| purrdf_lex::allocation::StorageError::AllocationFailed)?,
            ),
        })
    }
    /// Sorted distinct RDF spellings for presentation. Scoped identities are in
    /// [`Self::definitions`], including distinct occurrences with equal names.
    #[must_use]
    pub fn members(&self) -> &[String] {
        self.component.as_ref().map_or(&[], |value| &value.members)
    }
    /// Every original scoped definition, retained after its caller is dropped.
    #[must_use]
    pub fn definitions(&self) -> &[crate::recursion::RecursionDefinition] {
        self.component
            .as_ref()
            .map_or(&[], |value| &value.definitions)
    }
    /// Sorted distinct rendered edges for presentation; [`Self::scoped_edges`]
    /// preserves their exact original endpoints before lexical deduplication.
    #[must_use]
    pub fn edges(&self) -> &[RecursionDependency] {
        self.component.as_ref().map_or(&[], |value| &value.edges)
    }
    /// Every original internal edge, indexed into [`Self::definitions`].
    #[must_use]
    pub fn scoped_edges(&self) -> &[ScopedRecursionDependency] {
        self.component
            .as_ref()
            .map_or(&[], |value| &value.scoped_edges)
    }
}
impl fmt::Debug for RecursionRefusal {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("RecursionRefusal")
            .field("reason", &self.reason)
            .field("component", &self.component)
            .finish()
    }
}
impl PartialEq for RecursionRefusal {
    fn eq(&self, other: &Self) -> bool {
        self.reason == other.reason && self.component.as_deref() == other.component.as_deref()
    }
}
impl Eq for RecursionRefusal {}
impl fmt::Display for RecursionRefusal {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.reason {
            RecursionRefusalReason::MissingSelection => out.write_str(
                "cyclic SHACL shape reference requires explicit recursion semantics: ",
            )?,
            RecursionRefusalReason::NonStratified { semantics } => write!(
                out,
                "{semantics:?} requires stratified SHACL recursion; this component contains negative or nonmonotone conformance dependencies: "
            )?,
            RecursionRefusalReason::PairLimit { limit, consumed } => {
                return write!(
                    out,
                    "recursive SHACL pair limit {limit} exhausted after {consumed} pairs; increase the request's max_pairs"
                );
            }
            RecursionRefusalReason::IterationLimit { limit, consumed } => {
                return write!(
                    out,
                    "recursive SHACL iteration limit {limit} exhausted after {consumed} rounds; increase the request's max_iterations"
                );
            }
        }
        for (index, member) in self.members().iter().enumerate() {
            if index != 0 {
                out.write_str(", ")?;
            }
            out.write_str(member)?;
        }
        Ok(())
    }
}

impl std::error::Error for RecursionRefusal {}

purrdf_lex::variant_from!(ShapesError {
    Storage(purrdf_lex::allocation::StorageError),
    Processor(ProcessorFailure),
});

impl From<ShapesError> for purrdf_datalog::guard::CallbackError {
    fn from(error: ShapesError) -> Self {
        match error {
            ShapesError::Storage(error) => Self::Storage(error),
            error => Self::Evaluation(error.to_string()),
        }
    }
}

impl From<purrdf_core::RdfDiagnostic> for ShapesError {
    fn from(error: purrdf_core::RdfDiagnostic) -> Self {
        error
            .storage_error()
            .map_or_else(|| Self::Invalid(error.to_string()), Self::Storage)
    }
}

impl From<purrdf_iri::IriReadError> for ShapesError {
    fn from(error: purrdf_iri::IriReadError) -> Self {
        match error {
            purrdf_iri::IriReadError::Storage(error) => Self::Storage(error),
            error @ purrdf_iri::IriReadError::Lexical(_) => Self::Invalid(error.to_string()),
        }
    }
}

impl From<purrdf_core::ir::import::DatasetImportError<std::convert::Infallible>> for ShapesError {
    fn from(error: purrdf_core::ir::import::DatasetImportError<std::convert::Infallible>) -> Self {
        Self::Storage(error.into())
    }
}

/// A shapes graph refused because it uses the SHACL JavaScript Extensions.
///
/// SHACL-JS (`sh:JSConstraint`, `sh:js`, `sh:JSValidator`, `sh:JSRule`, …) is the 2017
/// Working Group Note "SHACL JavaScript Extensions", not part of SHACL 1.2, and this
/// engine has no JavaScript engine to evaluate it. A constraint it cannot evaluate is
/// refused rather than validated as if it were absent. The equivalent SHACL-SPARQL
/// constraint (`sh:sparql` with a `sh:select` query) loads.
///
/// Only a construct a shape REACHES is refused here: SHACL-JS on the shape itself
/// (`sh:js`, a SHACL-JS type), a `sh:JSTarget` or a target whose type is a
/// `sh:JSTargetType`, a `sh:JSRule` among its rules, and a call to a `sh:JSFunction` from
/// a node expression or from reachable SPARQL. A library that only DECLARES such
/// constructs — `sh:JSLibrary`s, `sh:JSFunction`s nothing calls — loads, and those
/// declarations are inert. A `sh:JSValidator` attached to a component is different: as a
/// value of `sh:validator`, `sh:nodeValidator` or `sh:propertyValidator` it violates the
/// attachment's class rule (SHACL 1.2 SPARQL Extensions: "The values of sh:validator must
/// be ASK-based validators", likewise SELECT-based for the scoped attachments), so it is
/// refused as [`ShapesError::IllFormed`] whether or not a shape reaches it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaclJsRefusal {
    node: String,
    term: String,
    message: String,
}

impl ShaclJsRefusal {
    pub(crate) const fn new(node: String, term: String, message: String) -> Self {
        Self {
            node,
            term,
            message,
        }
    }

    /// The node that uses the term, as the engine renders a term.
    #[must_use]
    pub fn node(&self) -> &str {
        &self.node
    }

    /// The SHACL-JS term's IRI.
    #[must_use]
    pub fn term(&self) -> &str {
        &self.term
    }

    /// The engine's full diagnostic, which names the SHACL JavaScript Extensions.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ShaclJsRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ShaclJsRefusal {}

/// A shape refused because it declares a custom target this engine cannot compute.
///
/// SHACL Advanced Features, "Custom Targets": "The behavior of a SHACL engine that is
/// unable to handle a given custom target is left undefined. [...] Engines that are aware
/// of this property and cannot handle a given custom target SHOULD at least report a
/// warning." PurRDF computes a `sh:SPARQLTarget` and an instance of a declared
/// `sh:SPARQLTargetType`; any other value of `sh:target` — a node of another target type,
/// or of none — would validate the shape against focus nodes it never computed. A warning
/// beside a report would still be a report about fewer focus nodes than the shape names,
/// so PurRDF refuses the shapes graph instead, which is stronger than the warning the
/// specification asks for. (A SHACL-JS target is refused as [`ShapesError::ShaclJs`].)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedTargetRefusal {
    shape: String,
    target: String,
    message: String,
}

impl UnsupportedTargetRefusal {
    pub(crate) const fn new(shape: String, target: String, message: String) -> Self {
        Self {
            shape,
            target,
            message,
        }
    }

    /// The shape that declares the target, as the engine renders a term.
    #[must_use]
    pub fn shape(&self) -> &str {
        &self.shape
    }

    /// The `sh:target` value, as the engine renders a term.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// The engine's full diagnostic.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for UnsupportedTargetRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for UnsupportedTargetRefusal {}

/// A `sh:SPARQLTarget` whose `sh:ask` and `sh:select` disagree about a node.
///
/// SHACL Advanced Features §3.1 gives a SPARQL-based target two queries. The SELECT
/// defines the target: its `this` bindings are the target nodes. The optional ASK answers
/// the same question for one node: "A SHACL engine can then determine whether a given
/// shape applies to a given node by executing the ASK query with the variable this
/// pre-bound to the node. If the ASK query evaluates to true then the node is in the target
/// of the shape." Both are the author's statement of ONE target, so a candidate check — which
/// decides a given node by the ASK rather than enumerating the SELECT — confirms the ASK's
/// answer against the SELECT's result set for the same data graph. When they disagree the
/// target has no single meaning, and a verdict computed from either answer would differ
/// from a whole-graph validation of the same node; the check is refused instead, naming
/// the shape, the target (by its two queries), the node and both answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlTargetDisagreement {
    shape: String,
    ask: String,
    select: String,
    node: String,
    asked: bool,
}

impl SparqlTargetDisagreement {
    pub(crate) const fn new(
        shape: String,
        ask: String,
        select: String,
        node: String,
        asked: bool,
    ) -> Self {
        Self {
            shape,
            ask,
            select,
            node,
            asked,
        }
    }

    /// The shape that declares the target, as the engine renders a term.
    #[must_use]
    pub fn shape(&self) -> &str {
        &self.shape
    }

    /// The target's `sh:ask` query text — with the PREFIX header the target's
    /// `sh:prefixes` supply — which identifies the target among the shape's `sh:target`s.
    #[must_use]
    pub fn ask(&self) -> &str {
        &self.ask
    }

    /// The target's `sh:select` query text, with the same PREFIX header.
    #[must_use]
    pub fn select(&self) -> &str {
        &self.select
    }

    /// The node the two queries disagree about, as the engine renders a term.
    #[must_use]
    pub fn node(&self) -> &str {
        &self.node
    }

    /// The ASK's answer with `$this` pre-bound to [`Self::node`]; the SELECT's is the
    /// opposite.
    #[must_use]
    pub const fn asked(&self) -> bool {
        self.asked
    }
}

impl fmt::Display for SparqlTargetDisagreement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "sparql-target-disagreement: a sh:SPARQLTarget of shape {shape} answers two ways \
             for {node}: its sh:ask, with $this pre-bound to that node, is {asked}, and its \
             sh:select {selects} return it. SHACL Advanced Features section 3.1 defines the \
             target by the SELECT and lets the ASK decide a given node (\"If the ASK query \
             evaluates to true then the node is in the target of the shape\"), so the two \
             state one target and must agree; with them disagreeing, checking the node would \
             not give the verdict a whole-graph validation gives it. Make the sh:ask true \
             exactly for the nodes the sh:select returns.\n  sh:ask: {ask}\n  sh:select: \
             {select}",
            shape = self.shape,
            node = self.node,
            asked = self.asked,
            selects = if self.asked { "does not" } else { "does" },
            ask = self.ask.replace('\n', " "),
            select = self.select.replace('\n', " "),
        )
    }
}

impl std::error::Error for SparqlTargetDisagreement {}

/// A syntax-rule violation as a declaration's parser reports it, before the declaration
/// is named: the id of the rule, when a numbered rule states it, and the diagnostic.
pub(crate) type RuleViolation = (Option<&'static str>, String);

/// One violation of a syntax rule by one SHACL-SPARQL or SHACL-AF declaration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IllFormedDeclaration {
    declaration: String,
    rule: Option<&'static str>,
    message: String,
}

impl IllFormedDeclaration {
    pub(crate) fn new(
        declaration: impl Into<String>,
        rule: Option<&'static str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            declaration: declaration.into(),
            rule,
            message: message.into(),
        }
    }

    /// The declaration, as the engine renders it: e.g. `validator _:b0 of the
    /// constraint component <http://example.org/ns#C>, via
    /// <http://www.w3.org/ns/shacl#nodeValidator>`, or `the sh:SPARQLFunction
    /// <http://example.org/ns#f>`.
    #[must_use]
    pub fn declaration(&self) -> &str {
        &self.declaration
    }

    /// The id of the syntax rule the declaration violates, as the specification's
    /// "Summary of Syntax Rules" spells it — e.g. `nodeValidator-class` (SHACL 1.2
    /// SPARQL Extensions), `parameter-name-not-in`, `SPARQLFunction-query` (SHACL
    /// Advanced Features). `None` for a refusal no numbered rule states: a term the
    /// engine would otherwise silently ignore, or a declaration that would be
    /// evaluated ambiguously.
    #[must_use]
    pub const fn rule(&self) -> Option<&'static str> {
        self.rule
    }

    /// The engine's diagnostic for this one violation.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for IllFormedDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.declaration, self.message)?;
        if let Some(rule) = self.rule {
            write!(f, " [syntax rule {rule}]")?;
        }
        Ok(())
    }
}

/// A shapes graph refused because SHACL-SPARQL or SHACL-AF declarations in it — or the
/// `sh:message` values of a node (SHACL 1.2 Core `message-datatype`) — violate syntax
/// rules, with every violation the graph contains, in a deterministic order.
///
/// SHACL 1.2 Core, "Handling of Ill-formed Shapes Graphs": "If the shapes graph contains
/// ill-formed nodes, then the result of the validation process is undefined. A SHACL
/// processor SHOULD produce a failure in this case." PurRDF treats that SHOULD as a MUST,
/// and the sentence has no reachability qualifier: a declaration no shape reaches is
/// refused exactly like one a shape uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IllFormedShapesGraph {
    violations: Vec<IllFormedDeclaration>,
}

impl IllFormedShapesGraph {
    /// `violations`, sorted and de-duplicated; `None` when there are none.
    pub(crate) fn from_violations(mut violations: Vec<IllFormedDeclaration>) -> Option<Self> {
        violations.sort();
        violations.dedup();
        (!violations.is_empty()).then_some(Self { violations })
    }

    /// Every violation, sorted by declaration, rule and message.
    #[must_use]
    pub fn violations(&self) -> &[IllFormedDeclaration] {
        &self.violations
    }
}

impl fmt::Display for IllFormedShapesGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the shapes graph is ill-formed: {} declaration{} violate{} a syntax rule, and SHACL \
             1.2 Core says \"A SHACL processor SHOULD produce a failure in this case\"",
            self.violations.len(),
            if self.violations.len() == 1 { "" } else { "s" },
            if self.violations.len() == 1 { "s" } else { "" },
        )?;
        for violation in &self.violations {
            write!(f, "\n  {violation}")?;
        }
        Ok(())
    }
}

impl std::error::Error for IllFormedShapesGraph {}

/// A query executed with pre-bound variables that violates a pre-binding restriction of
/// SHACL 1.2 SPARQL Extensions, Appendix A — a `MINUS`, a `VALUES`, an `AS ?var` for a
/// potentially pre-bound variable, among the restrictions PurRDF enforces.
///
/// As [`ShapesError::Prebinding`], the refusal of a query the engine executes: the
/// selected validator of a custom constraint component a shape uses, or a
/// `sh:SPARQLFunction` a node expression or reachable SPARQL text calls. The same value
/// describes a violating query nothing executes in the `unexecuted` section of
/// [`crate::lint`], where it is a finding rather than a refusal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrebindingViolation {
    declaration: String,
    message: String,
}

impl PrebindingViolation {
    pub(crate) fn new(declaration: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            declaration: declaration.into(),
            message: message.into(),
        }
    }

    /// The declaration whose query violates the restriction, as the engine renders it.
    #[must_use]
    pub fn declaration(&self) -> &str {
        &self.declaration
    }

    /// The restriction violated, naming the offending construct.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PrebindingViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: its query violates a pre-binding restriction [syntax rule \
             pre-binding-limitations]: {}",
            self.declaration, self.message
        )
    }
}

impl std::error::Error for PrebindingViolation {}

impl From<ShapesImportError> for ShapesError {
    fn from(error: ShapesImportError) -> Self {
        Self::Imports(error)
    }
}

impl From<String> for ShapesError {
    fn from(message: String) -> Self {
        Self::Invalid(message)
    }
}

impl From<&str> for ShapesError {
    fn from(message: &str) -> Self {
        Self::Invalid(message.to_owned())
    }
}
