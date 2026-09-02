// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

/// Which of the narrow, ENUMERATED S6-deferral residue an
/// [`EvalError::Unsupported`] belongs to — see that variant's docs for the full
/// list and why each entry is there. Absent (`None`, in
/// [`EvalError::Unsupported`]'s `kind` field / [`EvalError::diagnostic_code`])
/// for every OTHER unsupported construct — a genuine gap, not a scoped
/// deferral: `SERVICE`, `LATERAL`, a property function, a custom aggregate, an
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
    /// A manually constructed graph pattern's nesting exceeds the parser's
    /// safety bound.
    GraphPatternDepthExceeded,
}

impl UnsupportedKind {
    /// Every variant, for a caller that needs to test an arbitrary diagnostic
    /// code for membership (e.g. `purrdf_rdf::capture_support::is_deferred_construct`)
    /// without re-enumerating the closed set itself.
    pub const ALL: [Self; 4] = [
        Self::QuotedTripleTermVariable,
        Self::CustomFunction,
        Self::HeldInUnconfigured,
        Self::GraphPatternDepthExceeded,
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
            Self::GraphPatternDepthExceeded => "native-sparql-graph-pattern-depth-exceeded",
        }
    }
}

/// An error raised while evaluating a SPARQL query.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvalError {
    /// A query failed to parse in [`purrdf_sparql_algebra`]. Carries the rendered
    /// parse error.
    Parse(String),

    /// A well-formed construct this evaluator does not (or cannot) evaluate.
    ///
    /// This is the hard-fail boundary. `SERVICE` federation, `LATERAL`,
    /// property-function calls, and SPARQL `UPDATE` are all evaluated in-engine, so none
    /// of them surfaces here on its own account; what remains is a narrow, enumerated
    /// residue: a variable-bound quoted-triple-term component in a BGP or property-path
    /// pattern (structural triple-term matching is out of scope), an unresolved custom
    /// SPARQL function or aggregate IRI, `heldIn` called without a caller-supplied
    /// standpoint-predicate configuration, a manually constructed graph pattern
    /// whose nesting exceeds the parser's safety bound, and a query OR update
    /// declaring an unrecognized prologue `VERSION` (SPARQL 1.2 Query specification
    /// §4.4; [`purrdf_sparql_algebra::SparqlVersion::Other`]) — parsing is
    /// syntax-only for `VERSION` and accepts any string, so an unrecognized one is
    /// refused here, at evaluation admission, rather than at parse time, on both the
    /// query and the update evaluator (see `crate::eval::admit_version`, the one
    /// function both admission sites call). A `VERSION "1.2-basic"` request that uses
    /// an RDF 1.2 triple-term/reification construct outside that profile (SPARQL 1.2
    /// Query specification §4.3.1) is refused the same way, by the same chokepoint
    /// (see `crate::basic_profile`). The string names the unsupported
    /// construct. (Property paths are evaluated in-engine — S8 — and
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
        /// The closed S6-deferral classification, when this instance is one of
        /// the narrow enumerated residue [`UnsupportedKind`]'s docs list;
        /// `None` for a genuine gap. Set ONLY by `EvalError::unsupported_deferred`
        /// (a crate-private constructor, not part of this public field's own API).
        kind: Option<UnsupportedKind>,
    },

    /// An internal invariant was violated — e.g. a solution row whose width does
    /// not match its schema. This indicates a bug in the evaluator, not bad input
    /// (a frozen, validated dataset and a parsed algebra cannot legitimately cause
    /// it); it is surfaced rather than panicking so callers fail cleanly.
    Internal(String),

    /// A `SERVICE` federation step failed (transport error, undecodable remote
    /// response, or no remote source configured) and the `SERVICE` was **not**
    /// `SILENT`. Per the hard-fail doctrine a non-silent federation failure aborts
    /// the query rather than silently contributing no bindings; `SERVICE SILENT`
    /// instead swallows the failure to the join identity.
    Remote(String),

    /// A [`ServiceResolver`](crate::ServiceResolver)'s per-service policy withheld a
    /// capability, so the `SERVICE` step was refused before any endpoint was consulted.
    ///
    /// Structurally distinct from [`Self::Remote`] rather than folded into its string,
    /// because the two are classified oppositely and the difference has to survive being
    /// carried. An in-process resolver evaluates a forwarded `SERVICE` body *itself*, so a
    /// denial raised by a **nested** clause travels back out through that inner
    /// evaluation's error channel; flattened to a message it would be indistinguishable
    /// from an endpoint failure, and an enclosing `SERVICE SILENT` — entitled to swallow
    /// endpoint failures — would reduce it to the join identity. The surrounding join
    /// would become a no-op and the query would answer completely and wrongly, identically
    /// on every run. Keeping the [`ServiceDenial`](crate::ServiceDenial) whole is what lets
    /// `crate::remote::evaluate_in_memory` hand it back as
    /// [`RemoteError::Denied`](crate::RemoteError::Denied), which `SILENT` never swallows.
    ServiceDenied(crate::service::ServiceDenial),

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
    /// - a SHACL-AF SPARQL-based function (`sh:SPARQLFunction`: an arity mismatch, a
    ///   `sh:datatype`/`sh:nodeKind`/`sh:returnType` violation, or exceeding the
    ///   user-function recursion bound);
    /// - a native (host-Rust closure) function (an arity mismatch, the closure's own
    ///   returned `Err`, or a caught panic inside the closure);
    /// - a property function (`crate::property_fn`: an argument-vector arity mismatch,
    ///   the relation's own returned `Err`, or a caught panic inside `open`/`next`).
    ///
    /// Per the hard-fail doctrine a mis-invoked callee aborts the query rather than
    /// yielding a wrong or unbound value — or, for a relation, a short row stream
    /// offered as the complete one.
    Function(String),

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
        variable: String,
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
    /// `cdt:Map` value that crosses one of `purrdf-cdt`'s three resource bounds
    /// (`MAX_NESTING_DEPTH`, `MAX_ELEMENTS`, `MAX_LEXICAL_BYTES`). Carries the
    /// bound's own diagnostic.
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
}

impl EvalError {
    /// Construct an unclassified [`EvalError::Unsupported`] from any displayable
    /// construct name — a genuine gap, not one of the narrow S6-deferral residue.
    pub fn unsupported(what: impl Into<String>) -> Self {
        Self::Unsupported {
            what: what.into(),
            kind: None,
        }
    }

    /// Construct a CLASSIFIED [`EvalError::Unsupported`] — used ONLY by the four
    /// call sites producing the narrow, enumerated S6-deferral residue
    /// [`UnsupportedKind`]'s docs list. Every other unsupported construct stays
    /// [`EvalError::unsupported`].
    pub(crate) fn unsupported_deferred(kind: UnsupportedKind, what: impl Into<String>) -> Self {
        Self::Unsupported {
            what: what.into(),
            kind: Some(kind),
        }
    }

    /// The stable, machine-readable diagnostic code for this error's S6-deferral
    /// classification, if it has one — `None` for every other error, INCLUDING an
    /// unclassified [`EvalError::Unsupported`] (a genuine gap, not a scoped
    /// deferral). [`crate::engine`]'s `SparqlEngine` boundary reads this to set
    /// [`purrdf_core::RdfDiagnostic::code`] when reducing this typed error to a
    /// diagnostic; a caller further downstream that needs to tell "known S6
    /// deferral" from "real regression" reads that `RdfDiagnostic::code` field —
    /// never `Display` text.
    #[must_use]
    pub fn diagnostic_code(&self) -> Option<&'static str> {
        match self {
            Self::Unsupported { kind, .. } => kind.map(UnsupportedKind::code),
            Self::Parse(_)
            | Self::Internal(_)
            | Self::Remote(_)
            | Self::ServiceDenied(_)
            | Self::Data(_)
            | Self::Function(_)
            | Self::ExistsScopeCollision { .. }
            | Self::Config(_)
            | Self::CompositeBound(_) => None,
        }
    }

    /// Construct an [`EvalError::Internal`] from any displayable message.
    pub fn internal(what: impl Into<String>) -> Self {
        Self::Internal(what.into())
    }

    /// Construct an [`EvalError::Remote`] from any displayable message.
    pub fn remote(what: impl Into<String>) -> Self {
        Self::Remote(what.into())
    }

    /// Construct an [`EvalError::Data`] from any displayable message.
    pub fn data(what: impl Into<String>) -> Self {
        Self::Data(what.into())
    }

    /// Construct an [`EvalError::Function`] from any displayable message.
    pub fn function(what: impl Into<String>) -> Self {
        Self::Function(what.into())
    }

    /// Construct an [`EvalError::Config`] from any displayable message.
    pub fn config(what: impl Into<String>) -> Self {
        Self::Config(what.into())
    }

    /// Construct an [`EvalError::CompositeBound`] from a `purrdf-cdt` bound
    /// diagnostic.
    pub(crate) fn composite_bound(what: impl Into<String>) -> Self {
        Self::CompositeBound(what.into())
    }

    /// Construct an [`EvalError::ExistsScopeCollision`] naming the colliding
    /// variable (no leading `?`) and which construct introduced it (`"BIND
    /// target"` or `"VALUES variable"` — pass
    /// `crate::governor::soundness::RowCollisionIntro::as_str`'s result).
    pub(crate) fn exists_scope_collision(variable: impl Into<String>, intro: &'static str) -> Self {
        Self::ExistsScopeCollision {
            variable: variable.into(),
            intro,
        }
    }
}

impl core::fmt::Display for EvalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "SPARQL parse error: {msg}"),
            Self::Unsupported { what, .. } => {
                write!(f, "unsupported in sparql-eval (S6 scope): {what}")
            }
            Self::Internal(msg) => write!(f, "internal evaluator error: {msg}"),
            Self::Remote(msg) => write!(f, "SERVICE federation error: {msg}"),
            Self::ServiceDenied(denial) => {
                write!(f, "SERVICE federation denied: {denial}")
            }
            Self::Data(msg) => write!(f, "malformed RDF input: {msg}"),
            Self::ExistsScopeCollision { variable, intro } => write!(
                f,
                "{intro} ?{variable} inside EXISTS is already in scope on the row being \
                 filtered: the substitution semantics define no answer for a rebinding"
            ),
            Self::Function(msg) => write!(f, "host function error: {msg}"),
            Self::Config(msg) => write!(f, "invalid evaluation configuration: {msg}"),
            Self::CompositeBound(msg) => write!(
                f,
                "the composite value this query asked for exceeds a SEP-0009 resource bound: {msg}"
            ),
        }
    }
}

impl std::error::Error for EvalError {}

impl From<ParseError> for EvalError {
    fn from(err: ParseError) -> Self {
        Self::Parse(err.to_string())
    }
}

#[cfg(test)]
mod tests {
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
        assert!(e.to_string().contains("SERVICE"));
        assert!(e.to_string().contains("scope"));
    }

    /// An unclassified `Unsupported` (a genuine gap) carries no diagnostic
    /// code — the classifier at the `SparqlEngine` boundary must fall back to
    /// the generic per-callsite code for it, never mistake it for the S6
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

    /// Every other variant is likewise unclassified.
    #[test]
    fn non_unsupported_variants_have_no_diagnostic_code() {
        assert_eq!(EvalError::internal("x").diagnostic_code(), None);
        assert_eq!(EvalError::remote("x").diagnostic_code(), None);
        assert_eq!(EvalError::data("x").diagnostic_code(), None);
        assert_eq!(EvalError::function("x").diagnostic_code(), None);
        assert_eq!(EvalError::config("x").diagnostic_code(), None);
        assert_eq!(EvalError::Parse("x".to_owned()).diagnostic_code(), None);
    }
}
