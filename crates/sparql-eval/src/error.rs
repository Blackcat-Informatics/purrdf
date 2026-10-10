// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The evaluator's typed error channel.
//!
//! Per the project `no-optionality` / hard-fail doctrine, every condition that is
//! not a valid in-scope result is a typed error — there is no lenient mode and no
//! silent degradation. An unsupported algebra node or an unimplemented builtin is
//! [`EvalError::Unsupported`], not a best-effort answer.
//!
//! # This channel is disjoint from the governor channel, and outranks it
//!
//! A governed execution can end in a **truncated** solution sequence
//! ([`GovernedOutcome::BudgetExhausted`](crate::GovernedOutcome)), which is not a
//! contradiction of the paragraph above and is deliberately not an [`EvalError`]. The
//! doctrine bans answering a question *wrongly*; a governor answers a different, honestly
//! labelled question — "what had been established when the ceiling was reached" — and it
//! can only do so because the certificate travelling with those rows says which bound they
//! are. A partial sequence that arrived here instead would be exactly the silent
//! degradation the doctrine forbids, because an [`EvalError`] carries no such certificate
//! and a caller reducing one to "the query failed" would discard rows it could have used.
//!
//! The two channels therefore never merge, and where they meet the precedence is fixed: an
//! [`EvalError`] outranks **every** governor. Reporting an exhausted budget for a query
//! that could not have been answered at all would hand a caller a partial answer to a
//! question that has none — which is the same falsehood the hard-fail rule exists to
//! prevent, merely wearing a receipt.

use purrdf_sparql_algebra::ParseError;

/// Which of the narrow, ENUMERATED classified-unsupported residue an
/// [`EvalError::Unsupported`] belongs to — see that variant's docs for the full
/// list and why each entry is there. Absent (`None`, in
/// [`EvalError::Unsupported`]'s `kind` field / [`EvalError::diagnostic_code`])
/// for every OTHER unsupported construct — a genuine gap, not a classified
/// construct: `SERVICE`, `LATERAL`, a property function, a custom aggregate, an
/// unrecognized `VERSION`, and a Basic-profile triple-term refusal are all
/// evaluated (or refused) in-engine and never carry a kind.
///
/// Mirrors `crate::property_fn_plan::PlanSeam`'s shape: a small, closed
/// classification with a stable [`Self::code`] a caller reads instead of
/// scraping [`EvalError`]'s free-form `Display` text — which is prose with no
/// classification contract and is free to change wording at any time. The
/// golden-capture harness (`purrdf_rdf::capture_support::is_deferred_construct`)
/// is the in-repo caller: it keys off [`purrdf_core::RdfDiagnostic::code`],
/// which `crate::engine`'s `SparqlEngine` boundary sets from
/// [`EvalError::diagnostic_code`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnsupportedKind {
    /// A variable occupies a quoted-triple-term component in a BGP or
    /// property-path pattern (structural triple-term matching is out of
    /// scope).
    QuotedTripleTermVariable,
    /// A SPARQL function or aggregate IRI resolved to no registered custom
    /// function, native function, or XSD constructor.
    CustomFunction,
    /// `heldIn` was called with no caller-supplied standpoint-predicate
    /// configuration.
    HeldInUnconfigured,
}

impl UnsupportedKind {
    /// Every variant, for a caller that needs to test an arbitrary diagnostic
    /// code for membership (e.g. `purrdf_rdf::capture_support::is_deferred_construct`)
    /// without re-enumerating the closed set itself.
    pub const ALL: [Self; 3] = [
        Self::QuotedTripleTermVariable,
        Self::CustomFunction,
        Self::HeldInUnconfigured,
    ];

    /// The stable, machine-readable diagnostic code this kind maps to at the
    /// `SparqlEngine` boundary (`crate::engine`, where this typed error is
    /// reduced to a [`purrdf_core::RdfDiagnostic`]).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::QuotedTripleTermVariable => "native-sparql-quoted-triple-term-variable",
            Self::CustomFunction => "native-sparql-custom-function",
            Self::HeldInUnconfigured => "native-sparql-heldin-unconfigured",
        }
    }
}

/// Existing request/error meanings with immutable, admission-carrying text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NativeDiagnosticKind {
    /// The relation received an invalid invocation.
    Function,
    /// The relation failed operationally while executing its invocation.
    FunctionOperational,
    /// The requested data violated the native producer's input contract.
    Data,
    /// The supplied native configuration was invalid.
    Config,
    /// A native evaluator invariant failed.
    Internal,
    /// A classified unsupported construct, preserving its stable diagnostic code.
    Unsupported(UnsupportedKind),
    /// An unclassified request refusal, preserving the original Unsupported code.
    UnclassifiedUnsupported,
    /// A SEP-0009 semantic construction bound, with its original diagnostic code.
    CompositeBound,
}

#[derive(Debug)]
struct NativeDiagnosticPayload {
    // Payload dies before its string+Shared-header admission is released.
    text: String,
    _allocation: crate::WorkspaceAllocation,
}

/// An engine-supplied diagnostic clones its immutable allocation and lease.
#[derive(Debug, Clone)]
pub struct NativeDiagnostic {
    kind: NativeDiagnosticKind,
    payload: purrdf_core::small::Shared<NativeDiagnosticPayload>,
}

impl NativeDiagnostic {
    /// Stable, allocation-free sizing followed by fallible admitted rendering.
    /// `message` must render borrowed fields without private heap allocation;
    /// a term Debug walker must acquire its own actual work-list grant first.
    /// # Errors
    /// Returns typed admission, allocator or inconsistent-rendering failure.
    pub fn render(
        kind: NativeDiagnosticKind,
        message: impl core::fmt::Display,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        let preserve_failure = |error| {
            if workspace.has_failed() {
                EvalError::WorkspaceStopped
            } else {
                error
            }
        };
        let length = crate::workspace::display_len(&message).map_err(preserve_failure)?;
        let control =
            purrdf_core::small::Shared::<NativeDiagnosticPayload>::allocation_layout().size();
        let bytes = length
            .checked_add(control)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let allocation = workspace
            .charge(u64::try_from(bytes).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
        let text = crate::workspace::format_exact(&message, length, "native diagnostic text")
            .map_err(preserve_failure)?;
        let payload = purrdf_core::small::Shared::try_new(NativeDiagnosticPayload {
            text,
            _allocation: allocation,
        })
        .map_err(|_| EvalError::AllocationFailed {
            construct: "native diagnostic owner",
        })?;
        Ok(Self { kind, payload })
    }

    /// Stable typed classification, independent of its English message.
    #[must_use]
    pub const fn kind(&self) -> NativeDiagnosticKind {
        self.kind
    }
    /// Borrow the immutable body while retaining its shared owner.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.payload.text
    }
    /// One native diagnostic adaptation for `ok_or_else` and `map_err` callers.
    /// A refused render returns its original static operational failure.
    #[must_use]
    pub fn error(
        kind: NativeDiagnosticKind,
        message: impl core::fmt::Display,
        workspace: &crate::WorkspaceCapability,
    ) -> EvalError {
        match Self::render(kind, message, workspace) {
            Ok(diagnostic) => EvalError::NativeDiagnostic(diagnostic),
            Err(error) => error,
        }
    }
    fn preserve_function_failure(mut self) -> Self {
        if self.kind == NativeDiagnosticKind::Function {
            self.kind = NativeDiagnosticKind::FunctionOperational;
        }
        self
    }
}

impl PartialEq for NativeDiagnostic {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.message() == other.message()
    }
}
impl Eq for NativeDiagnostic {}
impl core::hash::Hash for NativeDiagnostic {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(&self.kind, state);
        core::hash::Hash::hash(self.message(), state);
    }
}
impl core::fmt::Display for NativeDiagnostic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let prefix = match self.kind {
            NativeDiagnosticKind::Function | NativeDiagnosticKind::FunctionOperational => {
                "host function error"
            }
            NativeDiagnosticKind::Data => "malformed RDF input",
            NativeDiagnosticKind::Config => "invalid evaluation configuration",
            NativeDiagnosticKind::Internal => "internal evaluator error",
            NativeDiagnosticKind::Unsupported(_)
            | NativeDiagnosticKind::UnclassifiedUnsupported => "unsupported",
            NativeDiagnosticKind::CompositeBound => {
                "the composite value this query asked for exceeds a SEP-0009 resource bound"
            }
        };
        write!(f, "{prefix}: {}", self.message())
    }
}
impl std::error::Error for NativeDiagnostic {}

/// An error raised while evaluating a SPARQL query.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvalError {
    /// An immutable native diagnostic retaining its exact allocation grant.
    NativeDiagnostic(NativeDiagnostic),
    /// Original RDF diagnostic and immutable allocation owner.
    RetainedDiagnostic(crate::RetainedDiagnostic),
    /// Federation failure retaining its original producer payload and physical grant.
    RetainedServiceFailure(crate::remote::RetainedServiceFailure),
    /// A native formatter violated its certified stable output length.
    UnstableNativeDiagnostic,
    /// A query failed to parse in [`purrdf_sparql_algebra`]. Carries the rendered
    /// parse error.
    Parse(String),

    /// The backing dataset failed to read a term or index. No partial answer is
    /// valid; fallible engine entry points retain the view's typed root cause.
    SourceRead(String),
    /// Internal allocation-free stop after the shared account refuses growth.
    /// The account retains the exact typed backend cause for final publication.
    /// This signal is an operational failure, never an expression type error or
    /// an invocation that SERVICE SILENT may absorb.
    WorkspaceStopped,
    /// A selected native XPath compiler, matcher or replacement operation was
    /// refused. This carries the typed resource/allocation cause and aborts the
    /// query; it never becomes an unbound expression or a negative match.
    XPathRegex(purrdf_core::xsd_regex::xpath::Error),
    /// The asynchronous host has issued every logical exchange identifier.
    /// An execution refusal, never an invocation failure or an expression error.
    ExchangeIdExhausted,

    /// A bounded read has no certified price for this query construct.
    WorkspaceUnpriced(&'static str),

    /// The certified worst-case workspace bound exceeds the logical byte width.
    WorkspaceBoundOverflow,

    /// The host could not allocate storage for materialized query solutions.
    /// An operational failure, not an expression error or a governor trip: no
    /// partial answer is certified when its result storage cannot be grown.
    AllocationFailed {
        /// The result storage whose allocation failed.
        construct: &'static str,
    },

    /// A well-formed construct this evaluator does not (or cannot) evaluate.
    ///
    /// This is the hard-fail boundary. `SERVICE` federation, `LATERAL`,
    /// property-function calls, and SPARQL `UPDATE` are all evaluated in-engine, so none
    /// of them surfaces here on its own account; what remains is a narrow, enumerated
    /// residue: a variable-bound quoted-triple-term component in a BGP or property-path
    /// pattern (structural triple-term matching is out of scope), an unresolved custom
    /// SPARQL function or aggregate IRI, `heldIn` called without a caller-supplied
    /// standpoint-predicate configuration, and a query OR update
    /// declaring an unrecognized prologue `VERSION` (SPARQL 1.2 Query specification
    /// §4.4; [`purrdf_sparql_algebra::SparqlVersion::Other`]) — parsing is
    /// syntax-only for `VERSION` and accepts any string, so an unrecognized one is
    /// refused here, at evaluation admission, rather than at parse time, on both the
    /// query and the update evaluator (see `crate::eval::admit_version`, the one
    /// function both admission sites call). A `VERSION "1.2-basic"` request that uses
    /// an RDF 1.2 triple-term/reification construct outside that profile (SPARQL 1.2
    /// Query specification §4.3.1) is refused the same way, by the same chokepoint
    /// (see `crate::basic_profile`). The string names the unsupported
    /// construct. (Property paths are evaluated in-engine and
    /// `DESCRIBE` evaluates via the canonical Symmetric CBD, so neither is here
    /// either. A property-function call whose predicate IRI resolves to no registered
    /// relation, or whose access pattern no declared mode admits, is
    /// [`EvalError::Function`]: the construct is supported and the host's table is
    /// what does not answer it. One property-function shape DOES surface here: a call
    /// inside a `SERVICE` body is refused at the forwarding boundary
    /// (`crate::remote::eval_service`), because the body is serialized and sent as
    /// SPARQL text and a call serializes as an ordinary triple — forwarding it would
    /// match it against the remote endpoint's data instead of invoking the relation, with
    /// no symptom anywhere.)
    Unsupported {
        /// Human-readable detail naming the construct.
        what: String,
        /// The closed unsupported-construct classification, when this instance is one of
        /// the narrow enumerated residue [`UnsupportedKind`]'s docs list;
        /// `None` for a genuine gap. Set ONLY by `EvalError::unsupported_deferred`
        /// (a crate-private constructor, not part of this public field's own API).
        kind: Option<UnsupportedKind>,
    },

    /// What the request builds was refused by the dataset it builds into: a
    /// `CONSTRUCT` graph is a frozen dataset, and a template that instantiates a
    /// statement no dataset admits — a triple term nested past the dataset's limit,
    /// say — is refused by that dataset's own admission. Carries its diagnostic, whose
    /// code (`rdf-ir-triple-nesting-limit`, …) is the one the request's diagnostic
    /// carries too ([`Self::code`]), exactly as an `UPDATE` that writes the same
    /// statement is refused.
    Dataset(purrdf_core::RdfDiagnostic),

    /// An internal invariant was violated — e.g. a solution row whose width does
    /// not match its schema. This indicates a bug in the evaluator, not bad input
    /// (a frozen, validated dataset and a parsed algebra cannot legitimately cause
    /// it); it is surfaced rather than panicking so callers fail cleanly.
    Internal(String),

    /// A `SERVICE` federation step failed (transport error or undecodable remote
    /// response) and the `SERVICE` was **not** `SILENT`. Per the hard-fail doctrine a
    /// non-silent federation failure aborts the query rather than silently contributing
    /// no bindings; `SERVICE SILENT` instead answers the join identity and records the
    /// failure on the execution's evidence.
    Remote(String),

    /// A `SERVICE` had no source to send its request to: the engine was given no remote
    /// query source at all, or its source answered that nothing it holds reaches the
    /// endpoint ([`RemoteError::Unconfigured`](crate::RemoteError::Unconfigured)).
    ///
    /// Distinct from [`Self::Remote`] because the two are different facts with different
    /// owners: [`Self::Remote`] is an endpoint that was asked and failed, this is a host
    /// that was never given a way to ask — its own configuration, not the endpoint's
    /// fault. Like every failed invocation, `SERVICE SILENT` answers it with the join
    /// identity and records it. It renders exactly as [`Self::Remote`] does; the two
    /// differ in [`Self::code`].
    ServiceUnconfigured(String),

    /// A [`ServiceResolver`](crate::ServiceResolver)'s per-service policy withheld a
    /// capability, so the `SERVICE` step was refused before any endpoint was consulted.
    ///
    /// Structurally distinct from [`Self::Remote`] rather than folded into its string,
    /// because the difference has to survive being carried. An in-process resolver
    /// evaluates a forwarded `SERVICE` body *itself*, so a denial raised by a **nested**
    /// clause travels back out through that inner evaluation's error channel; keeping the
    /// [`ServiceDenial`](crate::ServiceDenial) whole is what lets
    /// `crate::remote::evaluate_in_memory` hand it back as
    /// [`RemoteError::Denied`](crate::RemoteError::Denied), so the enclosing clause reports
    /// it — or, under `SILENT`, records it — as the denial it was rather than as an
    /// undecodable response.
    ServiceDenied(crate::service::ServiceDenial),

    /// A [`ServiceResolver`](crate::ServiceResolver) refused a `SERVICE` request as its
    /// own host-policy decision, with **no catalog capability** disclosed as the cause —
    /// see [`crate::remote::RemoteError::HostDenied`], which this carries the fields of.
    ///
    /// Distinct from [`Self::ServiceDenied`] because the two are different facts and
    /// conflating them would report a capability the host never named: [`Self::ServiceDenied`]
    /// names a capability an installed [`ServiceCatalog`](crate::service::ServiceCatalog)
    /// withheld; this variant is what a host resolver's own policy (a rate limit, an
    /// allowlist the host keeps outside any catalog, …) refuses on its own, independent of
    /// any catalog. Structurally distinct for the same reason [`Self::ServiceDenied`] is —
    /// it must survive an in-process resolver's nested `SERVICE` body without decaying
    /// into endpoint-failure text.
    ServiceHostDenied {
        /// The service IRI that was refused.
        endpoint: String,
        /// The host's own denial message, verbatim.
        message: String,
    },

    /// The host answering a `SERVICE` request broke the resolver protocol: its handler
    /// threw or rejected, or answered with something that is not an answer (a value the
    /// protocol does not define, a failure kind it does not name). The invocation failed
    /// — see [`crate::remote::RemoteError::HostFault`], whose fields this carries — so
    /// `SILENT` silences it as it silences every failed invocation. Without `SILENT` it
    /// is reported under [`Self::HOST_FAULT_CODE`]; a protocol boundary answers that code
    /// without the host's words, which describe the host's own defect.
    ServiceHostFault {
        /// The service IRI whose invocation the host faulted on.
        endpoint: String,
        /// The host's fault, verbatim, for the host's own logs.
        message: String,
    },

    /// The dataset carries structurally malformed RDF that a builtin cannot
    /// interpret — e.g. a cyclic `rdf:List` (a cell reachable from itself) or a
    /// list cell missing its `rdf:first`/`rdf:rest` edge. Distinct from
    /// [`EvalError::Internal`] (an evaluator bug over valid data) and
    /// [`EvalError::Unsupported`] (a valid construct out of scope): this is bad
    /// *input*. Per the hard-fail doctrine it aborts the query loudly rather than
    /// looping forever or guessing an answer.
    Data(String),

    /// A call into caller-injected host code was invalid. Three kinds share this
    /// variant, because all three are "the host's callee could not be invoked as
    /// written":
    ///
    /// - a SHACL-AF SPARQL-based function (`sh:SPARQLFunction`: an arity mismatch or a
    ///   `sh:datatype`/`sh:nodeKind`/`sh:returnType` violation);
    /// - a native (host-Rust closure) function with an argument arity/type mismatch;
    /// - a property function with an argument-vector arity/access-mode mismatch.
    ///
    /// Per the hard-fail doctrine a mis-invoked callee aborts the query rather than
    /// yielding a wrong or unbound value — or, for a relation, a short row stream
    /// offered as the complete one.
    Function(String),

    /// A selected invocation law refused a SPARQL-bodied function. This is a
    /// hard query failure, with the caller's exact cause retained through workers.
    FunctionAdmission(crate::user_fn::UserFunctionRefusal),

    /// An invoked function or relation failed operationally: an opaque host error,
    /// a caught panic, a resource ceiling, or an invalid host protocol response.
    /// Distinct from [`Self::Function`]'s request refusal so consumers cannot
    /// discard an execution failure when another validation alternative conforms.
    FunctionOperational(String),

    /// An `EXISTS`/`NOT EXISTS` body contains a `BIND`/`(expr AS ?v)` target or
    /// a `VALUES` column that collides with a variable already bound on the
    /// row being filtered — SEP-0007 Part 3's no-rebinding rule, enforced at
    /// evaluation admission here for algebra that reaches this evaluator WITHOUT
    /// going through [`purrdf_sparql_algebra`]'s parser (which refuses the
    /// same shape at parse time): a SHACL-AF pre-binding, an
    /// entailment-chase rewrite, or any other caller of the public algebra
    /// API. The substitution theorem `crate::expr::exists`'s doc states
    /// requires the inner pattern never observably rebind an outer-row
    /// variable; a shape that does has NO DEFINED ANSWER, so both evaluation
    /// strategies (the memoized probe and the per-row definition) are refused
    /// rather than one of them silently answering based on whichever
    /// happened to run — see `crate::governor::soundness::exists_row_collision`,
    /// this variant's sole constructor's caller.
    ExistsScopeCollision {
        /// The colliding variable's name, WITHOUT a leading `?`.
        variable: purrdf_lex::allocation::SharedText,
        /// `"BIND target"` or `"VALUES variable"` — matches the parser's own
        /// `ScopeIntro` wording exactly, so the message reads identically
        /// whether the collision was caught at parse time or here.
        intro: &'static str,
    },

    /// A caller supplied an invalid evaluation-configuration parameter -- e.g. a
    /// deterministic blank-mint prefix (`EvalCtx::with_bnode_mint_prefix`) that is
    /// not a legal `BLANK_NODE_LABEL` prefix, or an in-memory property-function
    /// table (`crate::property_fn::MemoryRelation::new`) whose rows do not all
    /// match its declared arity. Distinct from [`EvalError::Data`], which is
    /// about the dataset being evaluated rather than the caller's evaluation
    /// configuration: per the hard-fail doctrine, an out-of-alphabet
    /// configuration parameter is rejected at the setter rather than left to
    /// surface later as a silently rewritten label at egress.
    Config(String),

    /// A SEP-0009 composite-datatype function was asked to mint a `cdt:List` /
    /// `cdt:Map` value that crosses one of `purrdf-cdt`'s two resource bounds
    /// (`MAX_ELEMENTS`, `MAX_LEXICAL_BYTES`). Carries the bound's own diagnostic.
    ///
    /// Its own variant, and a HARD failure rather than an expression error,
    /// because the two are observably different and only one of them is safe:
    /// `cdt:put(?m, ?k, ?m)` roughly doubles a map's size on every application, so
    /// a query of a couple of dozen lines can ask for a value no host can hold.
    /// Answering "unbound" would let that query quietly change a result set — a
    /// `FILTER(!BOUND(?x))` would then be satisfied *by the refusal* — instead of
    /// being refused, so the refusal is propagated all the way out. This is
    /// [`purrdf_cdt::CdtOutcome::Bound`] reaching the query boundary, and it is
    /// distinct from [`EvalError::Data`]: nothing is malformed, the value is
    /// simply too large to exist.
    CompositeBound(String),

    /// A relation declared that the index behind it was **not** whole, on an execution
    /// whose entry point has nowhere to carry that declaration.
    ///
    /// # Witnessed or fatal
    ///
    /// A relation that serves an invocation from a partial index returns fewer rows than
    /// the query asked about. There are exactly three things an engine can do with that,
    /// and two of them are wrong. Failing the query outright overstates what happened —
    /// nothing broke, the relation answered, it simply answered from less than all of its
    /// data. Returning the short bag unlabelled is worse: it is a complete answer to a
    /// question nobody asked, indistinguishable from the true complete answer, and is
    /// precisely the silent degradation the hard-fail doctrine exists to forbid (see this
    /// module's header).
    ///
    /// The third is to return the rows **with the declaration attached**, which is what
    /// [`RelationWitness`](crate::RelationWitness) on a governed receipt does: a short bag
    /// whose receipt says which relation was short, and why, is not short — it is
    /// labelled, and a caller can act on it. So the rule is not a caller flag and cannot
    /// be one; it follows from the entry point's own return type. An entry that carries a
    /// witness reports the incompleteness as evidence beside its rows. An entry that
    /// does not — the ungoverned query lane, and the UPDATE lane, whose outcome types
    /// have no slot for evidence about a relation — cannot label the bag, so it refuses
    /// rather than hand back an unreadable receipt.
    ///
    /// Carries the relation's registered IRI and its own verbatim reason, because
    /// "something was incomplete" is not actionable and "shard 3 is still rebuilding on
    /// `<iri>`" is.
    RelationIncomplete {
        /// The registered IRI of the relation that declared the incompleteness.
        iri: purrdf_lex::allocation::SharedText,
        /// The relation's own description of what was missing, verbatim.
        reason: purrdf_lex::allocation::SharedText,
    },

    /// The calling thread's floating-point environment is not the IEEE-754 one a
    /// distance arithmetic defines its results under: it flushes subnormals to zero or
    /// rounds other than to nearest, ties to even. The refusal carries what showed it —
    /// the control register where one is read, or the binary64 probe operation whose
    /// bits differed.
    ///
    /// Its own variant rather than [`Self::Data`], because nothing about the data is
    /// wrong: the same artifact ranks correctly on a thread with the default
    /// environment. Ranking under the flushed one would return different distances
    /// with nothing to say so, which is a silent divergence rather than an answer.
    FloatEnvironment(purrdf_core::distance::FloatEnvironmentError),

    /// The request nests deeper than the stack of the thread evaluating it can hold:
    /// `construct` was about to be evaluated with less than
    /// [`purrdf_stack::MARGIN_BYTES`] of stack left, or the plan (`"query algebra"`) is
    /// too tall for the evaluator's walks over it to fit the stack its evaluation starts
    /// on.
    ///
    /// Its own variant because nothing about the request is malformed and nothing about
    /// the data is wrong: the same request answers on a native thread spawned with a
    /// larger stack. Refusing is what stands between an
    /// admitted request and a crash — natively an aborted process, on wasm32 a trapped
    /// instance whose memory can no longer be trusted — so this is never a partial
    /// answer and never retried shallower: the request as written does not fit.
    StackExhausted {
        /// What was about to be evaluated — an algebra node, an expression, an `EXISTS`,
        /// a correlated evaluation, a property path, a template term.
        construct: &'static str,
    },

    /// On `wasm32`, the request nests deeper than the JavaScript engine's own call
    /// stack holds: the plan is past the host-stack bounds of its admission
    /// (`construct` is then `"query algebra"`), or its graph patterns nest deeper than
    /// the depth the host-stack budget admits (`construct` is then `"graph pattern"`).
    /// Never raised on another target.
    ///
    /// Its own variant rather than [`Self::StackExhausted`] because no stack a caller
    /// sizes answers it: the engine's call stack is about 984 KiB under V8 on the
    /// synchronous lane and on an asynchronous job's suspendable stack alike. The remedy
    /// is a request nested less deeply.
    HostStackExhausted {
        /// The construct the budget stopped at.
        construct: &'static str,
    },
}

purrdf_lex::constructors! {
    impl EvalError {
        /// Construct an [`EvalError::Internal`] from any displayable message.
        pub fn internal(what) -> Self::Internal;

        /// Construct an [`EvalError::Remote`] from any displayable message.
        pub fn remote(what) -> Self::Remote;

        /// Construct an [`EvalError::Data`] from any displayable message.
        pub fn data(what) -> Self::Data;

        /// Construct an [`EvalError::Function`] from any displayable message.
        pub fn function(what) -> Self::Function;

        /// Preserve a function's execution failure separately from a bad request.
        pub(crate) fn function_operational(what) -> Self::FunctionOperational;

        /// Construct an [`EvalError::Config`] from any displayable message.
        pub fn config(what) -> Self::Config;

        /// Construct an [`EvalError::CompositeBound`] from a `purrdf-cdt` bound
        /// diagnostic.
        pub(crate) fn composite_bound(what) -> Self::CompositeBound;


    }
}

impl EvalError {
    /// A caller-returned `Function` contains no typed request classification.
    /// Retain it as an execution failure while preserving every more specific
    /// typed cause (including XPath, source and governor-adjacent refusals).
    pub(crate) fn preserve_function_failure(self) -> Self {
        match self {
            Self::Function(message) => Self::FunctionOperational(message),
            Self::NativeDiagnostic(message) => {
                Self::NativeDiagnostic(message.preserve_function_failure())
            }
            other => other,
        }
    }

    /// Preserve an operational source failure separately from RDF/type errors.
    pub(crate) fn source_read(error: impl core::fmt::Display) -> Self {
        // The typed root error remains in the fallible engine receipt. This
        // secondary English rendering must fit the fixed report admission even
        // when a host supplies an arbitrarily long operation label.
        const LIMIT: usize = 2048;
        const MARKER: &str = "… [truncated]";
        struct Bounded(String);
        impl core::fmt::Write for Bounded {
            fn write_str(&mut self, text: &str) -> core::fmt::Result {
                let remaining = (LIMIT - MARKER.len()).saturating_sub(self.0.len());
                if text.len() > remaining {
                    self.0
                        .push_str(&text[..text.floor_char_boundary(remaining)]);
                    return Err(core::fmt::Error);
                }
                self.0.push_str(text);
                Ok(())
            }
        }
        let mut rendered = Bounded(String::with_capacity(LIMIT));
        if core::fmt::write(&mut rendered, format_args!("{error}")).is_err() {
            rendered.0.push_str(MARKER);
        }
        Self::SourceRead(rendered.0)
    }
    /// Construct an unclassified [`EvalError::Unsupported`] from any displayable
    /// construct name — a genuine gap, not one of the narrow classified residue.
    pub fn unsupported(what: impl Into<String>) -> Self {
        Self::Unsupported {
            what: what.into(),
            kind: None,
        }
    }

    /// Construct a CLASSIFIED [`EvalError::Unsupported`] — used ONLY by the
    /// call sites producing the narrow, enumerated classified residue
    /// [`UnsupportedKind`]'s docs list. Every other unsupported construct stays
    /// [`EvalError::unsupported`].
    pub(crate) fn unsupported_deferred(kind: UnsupportedKind, what: impl Into<String>) -> Self {
        Self::Unsupported {
            what: what.into(),
            kind: Some(kind),
        }
    }

    /// The stable diagnostic code for a classified unsupported construct or a
    /// distinct execution refusal. Request/data errors without a distinct code
    /// return `None`, as does an unclassified [`Self::Unsupported`]; [`Self::code`]
    /// also preserves dataset and service codes. The engine reads that code when
    /// reducing this typed error to [`purrdf_core::RdfDiagnostic`], so consumers
    /// can retain execution failures without inspecting `Display` text.
    #[must_use]
    pub fn diagnostic_code(&self) -> Option<&'static str> {
        match self {
            Self::RetainedDiagnostic(_) => None,
            Self::RetainedServiceFailure(error) => {
                if matches!(
                    error.remote_error().error(),
                    crate::remote::RemoteError::SourceRead(_)
                ) {
                    Some("native-sparql-source-read")
                } else {
                    None
                }
            }
            Self::NativeDiagnostic(message) => match message.kind() {
                NativeDiagnosticKind::Internal => Some(Self::INTERNAL_CODE),
                NativeDiagnosticKind::FunctionOperational => Some(Self::FUNCTION_OPERATIONAL_CODE),
                NativeDiagnosticKind::Unsupported(kind) => Some(kind.code()),
                NativeDiagnosticKind::CompositeBound => Some(Self::COMPOSITE_BOUND_CODE),
                NativeDiagnosticKind::Function
                | NativeDiagnosticKind::Data
                | NativeDiagnosticKind::Config
                | NativeDiagnosticKind::UnclassifiedUnsupported => None,
            },
            Self::UnstableNativeDiagnostic => Some("native-sparql-unstable-native-diagnostic"),
            Self::SourceRead(_) | Self::WorkspaceStopped => Some("native-sparql-source-read"),
            Self::XPathRegex(purrdf_core::xsd_regex::xpath::Error::Resource(refusal)) => {
                Some(refusal.resource.code())
            }
            Self::XPathRegex(purrdf_core::xsd_regex::xpath::Error::Allocation {
                resource, ..
            }) => Some(resource.code()),
            Self::XPathRegex(_) => Some("native-sparql-xpath-operational"),
            Self::ExchangeIdExhausted => Some("native-sparql-exchange-id-exhausted"),
            Self::WorkspaceUnpriced(_) => Some("native-sparql-workspace-unpriced"),
            Self::WorkspaceBoundOverflow => Some("native-sparql-workspace-bound-overflow"),
            Self::AllocationFailed { .. } => Some(Self::ALLOCATION_FAILED_CODE),
            Self::Internal(_) => Some(Self::INTERNAL_CODE),
            Self::CompositeBound(_) => Some(Self::COMPOSITE_BOUND_CODE),
            Self::FloatEnvironment(_) => Some(Self::FLOAT_ENVIRONMENT_CODE),
            Self::FunctionOperational(_) => Some(Self::FUNCTION_OPERATIONAL_CODE),
            Self::Unsupported { kind, .. } => kind.map(UnsupportedKind::code),
            Self::Parse(_)
            | Self::Dataset(_)
            | Self::Remote(_)
            | Self::ServiceUnconfigured(_)
            | Self::ServiceDenied(_)
            | Self::ServiceHostDenied { .. }
            | Self::ServiceHostFault { .. }
            | Self::Data(_)
            | Self::Function(_)
            | Self::FunctionAdmission(_)
            | Self::ExistsScopeCollision { .. }
            | Self::Config(_) => None,
            Self::RelationIncomplete { .. } => Some(Self::RELATION_INCOMPLETE_CODE),
            Self::StackExhausted { .. } => Some(Self::STACK_EXHAUSTED_CODE),
            Self::HostStackExhausted { .. } => Some(Self::HOST_STACK_EXHAUSTED_CODE),
        }
    }

    /// The machine-readable code this error carries to the `SparqlEngine` boundary:
    /// [`Self::diagnostic_code`]; for [`Self::Dataset`], the dataset's own diagnostic
    /// code; for an unclassified [`Self::Unsupported`], [`Self::UNSUPPORTED_CODE`], so a
    /// host can tell a request this engine refuses to evaluate (the request's to change)
    /// from an evaluation that failed; and for each `SERVICE` outcome its own code —
    /// [`Self::SERVICE_DENIED_CODE`], [`Self::SERVICE_HOST_DENIED_CODE`],
    /// [`Self::SERVICE_FAILED_CODE`], [`Self::SERVICE_UNCONFIGURED_CODE`] — so a host can
    /// tell a refusal to ask an endpoint from an endpoint that failed, and both from a
    /// host that had no way to ask, without reading message text.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::RetainedDiagnostic(message) => Some(&message.diagnostic().code),
            Self::RetainedServiceFailure(error) => Some(error.code()),
            Self::Dataset(diagnostic) => Some(&diagnostic.code),
            Self::Unsupported { kind: None, .. } => Some(Self::UNSUPPORTED_CODE),
            Self::NativeDiagnostic(message)
                if message.kind() == NativeDiagnosticKind::UnclassifiedUnsupported =>
            {
                Some(Self::UNSUPPORTED_CODE)
            }
            Self::ServiceDenied(_) => Some(Self::SERVICE_DENIED_CODE),
            Self::ServiceHostDenied { .. } => Some(Self::SERVICE_HOST_DENIED_CODE),
            Self::ServiceHostFault { .. } => Some(Self::HOST_FAULT_CODE),
            Self::Remote(_) => Some(Self::SERVICE_FAILED_CODE),
            Self::ServiceUnconfigured(_) => Some(Self::SERVICE_UNCONFIGURED_CODE),
            other => other.diagnostic_code(),
        }
    }

    /// Whether a query diagnostic must survive a consumer's otherwise successful
    /// alternative, such as SHACL's existential value check.
    ///
    /// Known request, data and configuration failures remain ordinary failures:
    /// the consumer's own rules decide whether another alternative supersedes
    /// them. Execution, storage, resource and invariant failures must propagate.
    /// Unknown codes also propagate, including diagnostics supplied by a dataset;
    /// a newly introduced refusal cannot silently become a successful answer.
    /// This classifies stable codes, never diagnostic message text.
    #[must_use]
    pub fn diagnostic_requires_propagation(code: &str) -> bool {
        !matches!(
            code,
            "native-sparql-query-parse"
                | "native-sparql-query-eval"
                | "native-sparql-query-explain"
                | "native-sparql-algebra"
                | "native-sparql-property-function"
                | "native-sparql-aggregate-function"
                | "native-sparql-execution-parameter"
                | "native-sparql-bnode-mint-prefix"
                | "native-sparql-subst-iri"
                | "native-sparql-subst-langtag"
                | "native-sparql-subst-literal-datatype"
                | "native-sparql-subst-triple-predicate"
                | Self::UNSUPPORTED_CODE
        ) && !UnsupportedKind::ALL.iter().any(|kind| kind.code() == code)
    }

    /// The stable, machine-readable code [`Self::ServiceDenied`] carries to the
    /// `SparqlEngine` boundary ([`Self::code`]): an installed service catalog withheld a
    /// capability, so no endpoint was asked.
    pub const SERVICE_DENIED_CODE: &'static str = "native-sparql-service-denied";

    /// The stable, machine-readable code [`Self::ServiceHostDenied`] carries to the
    /// `SparqlEngine` boundary ([`Self::code`]): the host's own resolver refused the
    /// request by its own policy, with no catalog capability named, so no endpoint was
    /// asked.
    pub const SERVICE_HOST_DENIED_CODE: &'static str = "native-sparql-service-host-denied";

    /// The stable, machine-readable code a host's own fault carries to the `SparqlEngine`
    /// boundary — [`Self::ServiceHostFault`] here, and every job-level host fault the wasm
    /// lane reports: the host's adapter, not the request and not an endpoint, failed.
    pub const HOST_FAULT_CODE: &'static str = "native-sparql-host-fault";

    /// The stable, machine-readable code [`Self::Remote`] carries to the `SparqlEngine`
    /// boundary ([`Self::code`]): the endpoint was asked and did not produce a decodable
    /// answer — a transport failure, an HTTP error status, an undecodable body.
    pub const SERVICE_FAILED_CODE: &'static str = "native-sparql-service-failed";

    /// The stable, machine-readable code [`Self::ServiceUnconfigured`] carries to the
    /// `SparqlEngine` boundary ([`Self::code`]): the engine had no source that reaches the
    /// endpoint, so nothing was asked.
    pub const SERVICE_UNCONFIGURED_CODE: &'static str = "native-sparql-service-unconfigured";

    /// The stable, machine-readable code an unclassified [`Self::Unsupported`] carries to
    /// the `SparqlEngine` boundary ([`Self::code`]): a well-formed request this engine
    /// refuses to evaluate as written — for example a `SERVICE ?e` no solution names an
    /// endpoint for. The classified residue keeps its own [`UnsupportedKind::code`].
    pub const UNSUPPORTED_CODE: &'static str = "native-sparql-unsupported";

    /// The stable diagnostic code for a failed solution-storage reservation.
    pub const ALLOCATION_FAILED_CODE: &'static str = "native-sparql-allocation-failed";

    /// The stable diagnostic code for an evaluator invariant violation.
    pub const INTERNAL_CODE: &'static str = "native-sparql-internal";

    /// The stable diagnostic code for a composite value exceeding its resource bound.
    pub const COMPOSITE_BOUND_CODE: &'static str = "native-sparql-composite-bound";

    /// The stable diagnostic code for an unsafe floating-point execution environment.
    pub const FLOAT_ENVIRONMENT_CODE: &'static str = "native-sparql-float-environment";

    /// The stable diagnostic code for a function's operational execution failure.
    pub const FUNCTION_OPERATIONAL_CODE: &'static str = "native-sparql-function-operational";

    /// The stable, machine-readable diagnostic code
    /// [`Self::RelationIncomplete`] maps to at the `SparqlEngine` boundary.
    ///
    /// Named as a constant rather than written inline at the one `match` arm because it
    /// is the string a CALLER matches on — a host that must tell "a relation was short"
    /// apart from every other evaluation failure reads
    /// [`purrdf_core::RdfDiagnostic::code`] and compares it against this, never against
    /// `Display` prose, which is free to be reworded at any time. Spelled in the same
    /// `native-sparql-…` family as [`UnsupportedKind::code`]'s entries, so the whole
    /// code space stays one vocabulary.
    pub const RELATION_INCOMPLETE_CODE: &'static str = "native-sparql-relation-incomplete";

    /// The stable, machine-readable diagnostic code [`Self::StackExhausted`] maps to at
    /// the `SparqlEngine` boundary — the string a host compares against to tell "this
    /// request nests deeper than this thread's stack" from every other evaluation
    /// failure, and to know that a larger stack (not a different request) answers it.
    pub const STACK_EXHAUSTED_CODE: &'static str = "native-sparql-evaluation-stack-exhausted";

    /// The stable, machine-readable diagnostic code [`Self::HostStackExhausted`] maps to
    /// at the `SparqlEngine` boundary — the string a host compares against to tell "this
    /// request nests deeper than the JavaScript engine's call stack holds" from a stack a
    /// native caller can size ([`Self::STACK_EXHAUSTED_CODE`]): no lane answers it.
    pub const HOST_STACK_EXHAUSTED_CODE: &'static str = "native-sparql-host-stack-exhausted";

    /// Construct an [`Self::RelationIncomplete`] naming the relation and quoting its
    /// own reason.
    pub(crate) fn relation_incomplete_admitted(
        iri: &str,
        reason: &str,
        workspace: &crate::WorkspaceCapability,
    ) -> Self {
        let result = (|| {
            let iri = workspace.authored_text(&iri)?;
            let reason = workspace.authored_text(&reason)?;
            Ok::<_, Self>(Self::RelationIncomplete { iri, reason })
        })();
        result.unwrap_or_else(|error| error)
    }
}

impl core::fmt::Display for EvalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RetainedDiagnostic(message) => core::fmt::Display::fmt(message.diagnostic(), f),
            Self::RetainedServiceFailure(error) => core::fmt::Display::fmt(error, f),
            Self::NativeDiagnostic(message) => core::fmt::Display::fmt(message, f),
            Self::UnstableNativeDiagnostic => {
                f.write_str("native diagnostic formatter violated its certified output length")
            }
            Self::Parse(msg) => write!(f, "SPARQL parse error: {msg}"),
            Self::SourceRead(msg) => write!(f, "dataset read failed: {msg}"),
            Self::WorkspaceStopped => {
                f.write_str("operational workspace admission stopped evaluation")
            }
            Self::XPathRegex(error) => write!(f, "XPath operation refused: {error}"),
            Self::ExchangeIdExhausted => {
                f.write_str("asynchronous exchange identifier space is exhausted")
            }
            Self::WorkspaceUnpriced(construct) => write!(
                f,
                "bounded query workspace has no certified price for {construct}"
            ),
            Self::WorkspaceBoundOverflow => {
                f.write_str("bounded query workspace price exceeds the logical byte width")
            }
            Self::AllocationFailed { construct } => {
                write!(f, "memory allocation failed for {construct}")
            }
            Self::Unsupported { what, .. } => {
                write!(f, "unsupported: {what}")
            }
            Self::Dataset(diagnostic) => write!(
                f,
                "the dataset this request builds refused it: {}",
                diagnostic.message
            ),
            Self::Internal(msg) => write!(f, "internal evaluator error: {msg}"),
            Self::Remote(msg) | Self::ServiceUnconfigured(msg) => {
                write!(f, "SERVICE federation error: {msg}")
            }
            Self::ServiceDenied(denial) => {
                write!(f, "SERVICE federation denied: {denial}")
            }
            Self::ServiceHostDenied { endpoint, message } => {
                write!(
                    f,
                    "SERVICE <{endpoint}>: the host denied the request: {message}"
                )
            }
            Self::ServiceHostFault { endpoint, message } => {
                write!(
                    f,
                    "SERVICE <{endpoint}>: the host faulted answering the request: {message}"
                )
            }
            Self::Data(msg) => write!(f, "malformed RDF input: {msg}"),
            Self::ExistsScopeCollision { variable, intro } => write!(
                f,
                "{intro} ?{} inside EXISTS is already in scope on the row being \
                 filtered: the substitution semantics define no answer for a rebinding",
                variable.as_str()
            ),
            Self::Function(msg) | Self::FunctionOperational(msg) => {
                write!(f, "host function error: {msg}")
            }
            Self::FunctionAdmission(error) => write!(f, "host function error: {error}"),
            Self::Config(msg) => write!(f, "invalid evaluation configuration: {msg}"),
            Self::CompositeBound(msg) => write!(
                f,
                "the composite value this query asked for exceeds a SEP-0009 resource bound: {msg}"
            ),
            Self::RelationIncomplete { iri, reason } => write!(
                f,
                "property function <{iri}> served this query from an index it declares was \
                 not whole ({reason}); this entry point carries no witness to label the \
                 shortfall with, so the query is refused rather than answered short"
            ),
            Self::FloatEnvironment(error) => write!(
                f,
                "the thread's floating-point environment cannot run the distance \
                 arithmetic: {error}"
            ),
            Self::StackExhausted { construct } => {
                write!(
                    f,
                    "evaluation stack exhausted: the request's nesting exceeds what this \
                     host's stack can evaluate ({construct} needs more stack than this \
                     thread has left above its {}-byte reserve)",
                    purrdf_stack::MARGIN_BYTES
                )?;
                // A wasm caller has no thread to spawn, and both lanes run on stacks the
                // module sizes: the message names no remedy there.
                if cfg!(target_arch = "wasm32") {
                    Ok(())
                } else {
                    f.write_str("; run it on a thread with a larger stack")
                }
            }
            Self::HostStackExhausted { construct } => write!(
                f,
                "host call stack budget exceeded: the request's {construct} nests deeper \
                 than the JavaScript engine's own call stack holds ({} bytes of it are \
                 budgeted for a request, {} nested graph patterns at most, the same on the \
                 synchronous and the asynchronous lane); nest the request less deeply",
                crate::stack::height::WASM_HOST_STACK_BUDGET,
                crate::stack::height::WASM_GRAPH_PATTERN_DEPTH
            ),
        }
    }
}

impl std::error::Error for EvalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::XPathRegex(error) => Some(error),
            Self::FunctionAdmission(error) => Some(error.cause()),
            _ => None,
        }
    }
}

impl From<ParseError> for EvalError {
    /// A parse failure is [`EvalError::Parse`].
    fn from(err: ParseError) -> Self {
        Self::Parse(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_read_rendering_is_bounded_and_preserves_utf8_and_short_messages() {
        let host_message = "漢字操作".repeat(20_000);
        let EvalError::SourceRead(rendered) = EvalError::source_read(&host_message) else {
            panic!("source refusal has its own variant")
        };
        assert!(rendered.len() <= 2048);
        assert!(rendered.ends_with("… [truncated]"));
        assert!(rendered.starts_with("漢字操作"));
        let EvalError::SourceRead(short) = EvalError::source_read("read refused") else {
            panic!("source refusal has its own variant")
        };
        assert_eq!(short, "read refused");
    }
    use super::*;

    #[test]
    fn parse_error_converts_and_renders() {
        let pe = ParseError::Unsupported("VALUES with mixed arity".to_owned());
        let ee: EvalError = pe.into();
        assert!(matches!(ee, EvalError::Parse(_)));
        assert!(ee.to_string().contains("parse error"));
    }

    #[test]
    fn unsupported_names_the_construct() {
        let e = EvalError::unsupported("SERVICE");
        // A plain user-facing prefix, and no development label.
        assert_eq!(e.to_string(), "unsupported: SERVICE");
        assert_eq!(e.code(), Some(EvalError::UNSUPPORTED_CODE));
    }

    /// An unclassified `Unsupported` (a genuine gap) carries no diagnostic
    /// code — the classifier at the `SparqlEngine` boundary must fall back to
    /// the generic per-callsite code for it, never mistake it for the classified
    /// residue.
    #[test]
    fn unclassified_unsupported_has_no_diagnostic_code() {
        let e = EvalError::unsupported("SERVICE");
        assert_eq!(e.diagnostic_code(), None);
    }

    /// `unsupported_deferred` is the only path that attaches a
    /// [`UnsupportedKind`], and its code round-trips through `diagnostic_code`
    /// unchanged — the exact seam `crate::engine::eval_diagnostic_code` reads.
    #[test]
    fn deferred_unsupported_carries_its_kind_code() {
        for kind in UnsupportedKind::ALL {
            let e = EvalError::unsupported_deferred(kind, "detail");
            assert_eq!(e.diagnostic_code(), Some(kind.code()));
            assert!(e.to_string().contains("detail"));
        }
    }

    /// Ordinary request/data failures retain the boundary's generic code.
    #[test]
    fn ordinary_request_variants_have_no_diagnostic_code() {
        assert_eq!(EvalError::remote("x").diagnostic_code(), None);
        assert_eq!(EvalError::data("x").diagnostic_code(), None);
        assert_eq!(EvalError::function("x").diagnostic_code(), None);
        assert_eq!(EvalError::config("x").diagnostic_code(), None);
        assert_eq!(EvalError::Parse("x".to_owned()).diagnostic_code(), None);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn execution_refusals_keep_distinct_codes_at_diagnostic_boundaries() {
        use purrdf_core::distance::{FloatEnvironmentError, FloatEnvironmentEvidence};

        let environment = FloatEnvironmentError::RoundingMode {
            evidence: FloatEnvironmentEvidence::Probe {
                operation: "frozen test evidence",
                expected: 1,
                observed: 2,
            },
        };
        for (error, expected) in [
            (EvalError::internal("row width"), EvalError::INTERNAL_CODE),
            (
                EvalError::composite_bound("element ceiling"),
                EvalError::COMPOSITE_BOUND_CODE,
            ),
            (
                EvalError::FloatEnvironment(environment),
                EvalError::FLOAT_ENVIRONMENT_CODE,
            ),
            (
                EvalError::function_operational("callee panicked"),
                EvalError::FUNCTION_OPERATIONAL_CODE,
            ),
        ] {
            assert_eq!(error.diagnostic_code(), Some(expected));
            assert_eq!(error.code(), Some(expected));
            for fallback in [
                "native-sparql-query-eval",
                "native-sparql-query-explain",
                "native-sparql-update-eval",
                "native-sparql-algebra",
            ] {
                assert_eq!(
                    crate::engine::eval_diagnostic_code(&error, fallback),
                    expected
                );
            }
            assert!(EvalError::diagnostic_requires_propagation(expected));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn query_diagnostic_propagation_preserves_execution_and_unknown_failures() {
        use purrdf_core::xsd_regex::xpath::Resource;

        for code in [
            "native-sparql-query-parse",
            "native-sparql-query-eval",
            "native-sparql-algebra",
            "native-sparql-property-function",
            "native-sparql-aggregate-function",
            "native-sparql-execution-parameter",
            "native-sparql-bnode-mint-prefix",
            "native-sparql-subst-iri",
            "native-sparql-subst-langtag",
            "native-sparql-subst-literal-datatype",
            "native-sparql-subst-triple-predicate",
            EvalError::UNSUPPORTED_CODE,
        ] {
            assert!(!EvalError::diagnostic_requires_propagation(code), "{code}");
        }
        for kind in UnsupportedKind::ALL {
            assert!(!EvalError::diagnostic_requires_propagation(kind.code()));
        }
        for code in [
            "native-sparql-source-read",
            "native-sparql-exchange-id-exhausted",
            "native-sparql-workspace-unpriced",
            "native-sparql-workspace-bound-overflow",
            "native-sparql-xpath-operational",
            EvalError::ALLOCATION_FAILED_CODE,
            EvalError::RELATION_INCOMPLETE_CODE,
            EvalError::STACK_EXHAUSTED_CODE,
            EvalError::HOST_STACK_EXHAUSTED_CODE,
            EvalError::INTERNAL_CODE,
            EvalError::COMPOSITE_BOUND_CODE,
            EvalError::FLOAT_ENVIRONMENT_CODE,
            EvalError::FUNCTION_OPERATIONAL_CODE,
            EvalError::SERVICE_DENIED_CODE,
            EvalError::SERVICE_HOST_DENIED_CODE,
            EvalError::HOST_FAULT_CODE,
            EvalError::SERVICE_FAILED_CODE,
            EvalError::SERVICE_UNCONFIGURED_CODE,
            "caller-dataset-refusal",
            "native-sparql-unrecognized-refusal",
            "",
        ] {
            assert!(EvalError::diagnostic_requires_propagation(code), "{code}");
        }
        for resource in [
            Resource::PatternBytes,
            Resource::CompileSteps,
            Resource::ProgramNodes,
            Resource::CompileSlots,
            Resource::MatchSteps,
            Resource::MatchStates,
            Resource::MatchSlots,
            Resource::OutputBytes,
        ] {
            assert!(EvalError::diagnostic_requires_propagation(resource.code()));
        }
    }

    /// A stack refusal carries its own code and names the construct it stopped at.
    #[test]
    fn stack_exhausted_carries_its_code_and_names_the_construct() {
        let e = EvalError::StackExhausted {
            construct: "FILTER EXISTS",
        };
        assert_eq!(
            e.diagnostic_code(),
            Some("native-sparql-evaluation-stack-exhausted")
        );
        assert!(e.to_string().contains("FILTER EXISTS"), "{e}");
        assert!(e.to_string().contains("evaluation stack exhausted"), "{e}");
        // Natively the remedy is a thread with a larger stack.
        assert!(
            e.to_string()
                .ends_with("; run it on a thread with a larger stack"),
            "{e}"
        );
    }

    /// The host-stack refusal has its own code, and its message names the budget and
    /// no larger stack as a remedy: no stack a caller sizes raises it.
    #[test]
    fn host_stack_exhausted_carries_its_own_code_and_names_no_larger_stack() {
        let e = EvalError::HostStackExhausted {
            construct: "graph pattern",
        };
        assert_eq!(
            e.diagnostic_code(),
            Some("native-sparql-host-stack-exhausted")
        );
        let text = e.to_string();
        assert!(text.contains("graph pattern"), "{text}");
        assert!(text.contains("655360 bytes"), "{text}");
        assert!(text.contains("284 nested graph patterns"), "{text}");
        assert!(
            text.contains("the same on the synchronous and the asynchronous lane)"),
            "{text}"
        );
        assert!(text.ends_with("nest the request less deeply"), "{text}");
        assert!(!text.contains("thread"), "{text}");
    }
}
