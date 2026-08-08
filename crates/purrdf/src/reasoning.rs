// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entailment-aware SPARQL orchestration over the native PurRDF engines.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use purrdf_datalog::seminaive::BudgetReport;
use purrdf_entail::{
    Construct, EntailError, Materialization, QNode, QTriple, ReasoningReport, Regime, RuleSet,
    materialize_combined, materialize_combined_until,
};
use purrdf_rdf::{
    RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfQuad, RdfTerm, RdfTextDirection,
    SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::{
    AggregateExpression, BaseDirection, BlankNode, Expression, GraphPattern, GroundTerm, Literal,
    NamedNodePattern, PropertyFunctionCall, Query, TermPattern, TriplePattern, Variable,
};
use purrdf_sparql_eval::{
    BudgetExhausted, GovernedOutcome, NativeSparqlEngine, PreparedQuery, QueryGovernors,
    QueryOptions, StopCause, StopSignal, TrippedGovernor,
};

/// A reasoning session over one ontology — the OWL 2 Direct-Semantics services, held
/// open so that asking N questions costs one parse and one reverse mapping.
///
/// Re-exported here because this is the module a Rust caller looks in for reasoning.
/// Reachable before this existed only as `purrdf::validate::regime::ReasonerSession`,
/// which is a truthful path and a misleading one: the type is not about validation, and
/// the three other hosts (`purrdf.entail.Reasoner`, `new Reasoner(…)`,
/// `purrdf_reasoner_open`) all name it where the reasoning surface is.
///
/// Distinct from [`purrdf_entail::Reasoner`], which is the knowledge base itself and
/// answers in DL terms. This is the STRING boundary over it — the one every non-Rust
/// host calls — so a Rust caller gets byte-identical answers and certificates to what
/// Python, WASM and C see.
///
/// ```
/// use purrdf::reasoning::ReasonerSession;
///
/// let data = "<http://example.org/tom> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Cat> .\n";
/// let mut session = ReasonerSession::open(data, 0).expect("parses");
/// assert_eq!(session.consistency().expect("decides").answer(), "consistency true\n");
/// let hierarchy = session.classify().expect("decides"); // no second parse
/// assert!(hierarchy.certificate().starts_with("purrdf-dl-certificate 1\n"));
/// ```
pub use purrdf_validate::regime::ReasonerSession;

/// Entailment behavior applied before evaluating one SPARQL query.
///
/// Every W3C `sparql:entailmentRegime` this repository implements is here, because a
/// regime that is materializable everywhere else and unreachable from the query surface is
/// a capability the caller cannot use: `entailment/D` is a regime of the SPARQL 1.1
/// Entailment Regimes recommendation exactly as `entailment/RDFS` is, and
/// [`purrdf_entail::materialize`] serves it like any other rule table.
#[derive(Debug, Clone, Copy)]
pub enum QueryEntailment<'a> {
    /// Query asserted data directly.
    Simple,
    /// Materialize RDF entailment.
    Rdf,
    /// Materialize RDFS entailment.
    Rdfs,
    /// Materialize OWL 2 RL entailment.
    OwlRl,
    /// Materialize `entailment/D` — Simple entailment plus the five `dt-*` rules of
    /// OWL 2 Profiles §4.3 Table 8.
    D,
    /// Perform query-directed OWL Direct-Semantics augmentation.
    OwlDirect,
    /// Materialize the supplied RIF-Core rule set.
    Rif(&'a RuleSet),
}

/// Owned, host-neutral configuration for one entailment-aware SPARQL query.
///
/// Language bindings receive a regime spelling plus a string program rather than a
/// borrowed [`RuleSet`]. This type resolves those two values once through the shared
/// boundary vocabulary and then lends [`QueryEntailment`] to the native orchestrator.
/// Keeping the validation here prevents Python, WebAssembly, and C from acquiring three
/// subtly different readings of an empty RIF program or an unexpected program on RDFS.
#[derive(Debug)]
pub struct QueryEntailmentPlan {
    regime: Regime,
    rules: RuleSet,
}

impl QueryEntailmentPlan {
    /// Parse the exact cross-host regime spelling and its regime-owned program.
    ///
    /// `rif` requires a RIF-in-XML program and rejects imports because this in-memory
    /// boundary performs no I/O. Every other regime requires `program` to be empty.
    ///
    /// # Errors
    ///
    /// Returns the shared boundary diagnostic for an unknown regime, an invalid program,
    /// a missing RIF program, or a program supplied to a regime whose calculus is fixed.
    pub fn parse(regime: &str, program: &str) -> Result<Self, String> {
        let parsed = purrdf_validate::regime::parse_regime(regime)?;
        let rules = purrdf_validate::regime::regime_rule_set(parsed, regime, program)?;
        Ok(Self {
            regime: parsed,
            rules,
        })
    }

    /// Borrow this owned configuration in the native query orchestrator's form.
    #[must_use]
    pub const fn entailment(&self) -> QueryEntailment<'_> {
        match self.regime {
            Regime::Simple => QueryEntailment::Simple,
            Regime::Rdf => QueryEntailment::Rdf,
            Regime::Rdfs => QueryEntailment::Rdfs,
            Regime::OwlRl => QueryEntailment::OwlRl,
            Regime::D => QueryEntailment::D,
            Regime::OwlDirect => QueryEntailment::OwlDirect,
            Regime::Rif => QueryEntailment::Rif(&self.rules),
        }
    }

    /// The resolved native regime.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }
}

/// Failure from entailment-aware query preparation or evaluation.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReasoningError {
    /// SPARQL parsing or evaluation failed.
    Query(RdfDiagnostic),
    /// Entailment or rule materialization failed.
    Entailment(EntailError),
}

impl std::fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query(error) => write!(f, "SPARQL query failed: {error}"),
            Self::Entailment(error) => write!(f, "entailment failed: {error}"),
        }
    }
}

impl std::error::Error for ReasoningError {
    /// The wrapped cause — always present, because every variant is one.
    ///
    /// This type exists ONLY to say which of two subsystems failed; it adds no failure
    /// of its own. Returning `None` therefore hid the entire diagnostic behind a value
    /// whose whole content was the choice between two wrappers, and a caller walking
    /// `Error::source` reached the fork and then nothing.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Query(inner) => Some(inner),
            Self::Entailment(inner) => Some(inner),
        }
    }
}

impl From<RdfDiagnostic> for ReasoningError {
    fn from(value: RdfDiagnostic) -> Self {
        Self::Query(value)
    }
}

impl From<EntailError> for ReasoningError {
    fn from(value: EntailError) -> Self {
        Self::Entailment(value)
    }
}

/// Evaluate SPARQL under an explicit native entailment regime, and say what the reasoner
/// did.
///
/// Returns the query's answer AND the [`ReasoningReport`] of the run that produced the
/// dataset it was answered over.
///
/// # The certificate travels with the answer
///
/// A SPARQL result set carries no reasoning metadata, and this function used to take that
/// as permission to drop the report: the closure was computed, the evidence was bound to
/// `_`, and the caller received rows with no way to learn that "OWL Direct-Semantics
/// answers" had been computed over an ontology holding an `owl:propertyChainAxiom` the
/// reverse mapping could not read, or that most of their input sat in named graphs the lane
/// never opened. That is not a constraint of [`SparqlResult`]; it is a missing return
/// value, and a pair is the smallest honest fix. A caller who does not want it binds
/// `(result, _)`.
///
/// [`QueryEntailment::Simple`] asks no reasoner to run, so its report is the one
/// [`purrdf_entail::materialize`] returns for [`Regime::Simple`] — an identity closure has
/// no rule table, meets no boundary and consumes no ceiling — assembled directly rather
/// than by copying the dataset to obtain it. The equality is asserted in this module's
/// tests rather than asserted in prose.
///
/// # Errors
///
/// Returns [`ReasoningError::Query`] for SPARQL failures and
/// [`ReasoningError::Entailment`] for malformed or inconsistent knowledge bases.
pub fn query_with_entailment(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    request: SparqlRequest<'_>,
    entailment: QueryEntailment<'_>,
) -> Result<(SparqlResult, ReasoningReport), ReasoningError> {
    // Parse first so invalid queries fail before potentially expensive closure work.
    // OWL Direct also inspects this same cached plan, avoiding a second parse/cache lookup.
    let prepared_query = engine.prepare_query(request.query, request.base_iri)?;
    // Every lane hands back a `ReasoningReport` alongside the closure, and every one of
    // them is carried out of this function rather than dropped at this call site.
    // `collect_query_bgp` is bound outside the match because the OWL-Direct plan BORROWS
    // it; it is computed for that mode alone.
    let pattern = match entailment {
        QueryEntailment::OwlDirect => collect_query_bgp(&prepared_query.query),
        _ => Vec::new(),
    };
    // Populated only when the OWL-Direct lane answered through the COMBINED APPROACH
    // (`purrdf_entail::materialize_combined`) rather than the whole-vocabulary
    // augmentation: the set of blank terms its restricted chase minted as existential
    // witnesses. A witness is not a certain answer for a variable whose binding the caller
    // can OBSERVE — the regime draws its answers from the scoping graph, and a minted
    // witness is not in it — so `restrict_witness_bindings` below forbids exactly those
    // bindings, at the point the binding is made rather than after the fact.
    let mut combined_surrogates = None;
    // ONE call, seven modes. `purrdf_entail::materialize` is total over
    // `Materialization`, so this lane no longer splits into "the regimes that
    // materialize" and "the two that need their own entry point".
    let (prepared, report) = match entailment {
        QueryEntailment::Simple => (Arc::clone(dataset), simple_report()),
        QueryEntailment::Rdf => purrdf_entail::materialize(dataset, Materialization::Rdf)?,
        QueryEntailment::Rdfs => purrdf_entail::materialize(dataset, Materialization::Rdfs)?,
        QueryEntailment::OwlRl => purrdf_entail::materialize(dataset, Materialization::OwlRl)?,
        QueryEntailment::D => purrdf_entail::materialize(dataset, Materialization::D)?,
        QueryEntailment::OwlDirect => match materialize_combined(dataset, &pattern)? {
            // The ontology's TBox is in the certified Horn fragment: answer through the
            // combined approach (restricted-chase witnesses for the anonymous part,
            // filtered below) rather than the whole-vocabulary augmentation, which is
            // silently incomplete for a query's non-distinguished variable.
            Some(combined) => {
                combined_surrogates = Some(combined.surrogates);
                (combined.dataset, combined.report)
            }
            // Outside that fragment: the pre-existing augmentation, boundary and all — plus
            // the ONE boundary only this call site can raise. The augmentation's own report
            // is complete about the run IT made; what it cannot know is that the combined
            // approach was tried first and declined, which is exactly what
            // `Construct::NonHornTBox` says. Attaching it here is what gives that variant a
            // producer instead of leaving it a promise three prose sites made and no code
            // path kept.
            None => {
                let (closure, report) =
                    purrdf_entail::materialize(dataset, Materialization::OwlDirect(&pattern))?;
                (closure, report.with_boundary(Construct::NonHornTBox))
            }
        },
        QueryEntailment::Rif(ruleset) => {
            purrdf_entail::materialize(dataset, Materialization::Rif(ruleset))?
        }
    };
    // The combined approach's filtration, in the only two places a witness can escape: the
    // solution sequence (forbidden BEFORE evaluation, so the algebra above the restriction
    // sees the filtered sequence) and a constructed graph (scrubbed after, because a
    // `DESCRIBE` draws triples from the dataset rather than from a variable binding).
    let surrogates = combined_surrogates.unwrap_or_default();
    let restricted = (!surrogates.is_empty())
        .then(|| {
            PreparedQuery::rewritten(
                restrict_witness_bindings(&prepared_query.query, &surrogates),
                QueryOptions::EMPTY,
            )
        })
        .transpose()?;
    let plan = restricted.as_ref().unwrap_or(&prepared_query);
    let mut result = engine.query_prepared(&prepared, plan, request.substitutions)?;
    let _ = withhold_surrogate_triples(&mut result, &surrogates);
    Ok((result, report))
}

/// What a governed entailment-regime query produced.
///
/// # Why this is not `(GovernedOutcome, ReasoningReport)`
///
/// Because an entailment-regime query is TWO phases and only one of them has a partial
/// answer to give. Phase one materializes the regime's closure; phase two evaluates SPARQL
/// over that frozen closure. [`GovernedOutcome`] is exactly the right shape for phase two —
/// a complete result, or an exhausted budget carrying certified partial answers — and it is
/// the wrong shape for phase one, because a closure that was stopped mid-fixpoint is not a
/// smaller closure. There are no certified rows to carry, and there is no
/// [`ReasoningReport`] either: a report is the certificate of a run that produced a closure,
/// and this run produced none.
///
/// Folding that case into a `BudgetExhausted` with empty rows would state, in the only
/// vocabulary the caller has for reading it, that a query was evaluated and yielded nothing
/// — which is a claim about the DATA. Nothing was evaluated. So it gets its own arm, and the
/// arm structurally carries no rows and no report, in the same way
/// [`GovernedUpdateOutcome`](purrdf_sparql_eval::GovernedUpdateOutcome) structurally carries
/// no partial mutation.
#[derive(Debug)]
#[allow(
    clippy::large_enum_variant,
    reason = "the large arm is the ORDINARY one — a GovernedOutcome plus the reasoning \
              certificate, both of which the caller then reads — and the small arm is the \
              rare stop. Boxing the common payload would put an allocation on every \
              governed entailment query to shrink a value that is moved once, and it would \
              force a caller to deref through a Box to reach the outcome they asked for"
)]
#[non_exhaustive]
pub enum GovernedEntailment {
    /// The closure was computed and the query was evaluated over it.
    ///
    /// The `outcome` is phase two's, so every ceiling the caller named — fuel, answer cap,
    /// intermediate cells, scratch bytes, remote requests — was in force over the closure,
    /// and a trip here carries the partial answers the evaluation reached. The `report` is
    /// the certificate of the closure those answers were drawn from, and it travels on BOTH
    /// arms of the outcome: a truncated answer over an OWL 2 RL closure is unreadable
    /// without knowing what closed it, exactly as a complete one is.
    Answered {
        /// Phase two's outcome: complete, or stopped by a governor with certified partials.
        outcome: GovernedOutcome,
        /// The certificate of the reasoning run that produced the queried closure.
        report: ReasoningReport,
    },
    /// The caller's stop signal fired while the CLOSURE was still being computed.
    ///
    /// Nothing was evaluated, nothing is certified, and nothing is claimed — there is no
    /// field on this arm to read a row or a report out of, because there is none to read.
    /// Only a stop signal (a cancellation or a wall deadline) can produce it: the numeric
    /// ceilings are charged by the SPARQL evaluator and reach phase two alone, which is
    /// stated where [`query_with_entailment_governed`] documents what it governs.
    ClosureStopped {
        /// The stop signal that ended the run, in the shared governor vocabulary.
        tripped: TrippedGovernor,
    },
}

impl GovernedEntailment {
    /// Phase two's outcome, when the closure was computed at all.
    #[must_use]
    pub const fn outcome(&self) -> Option<&GovernedOutcome> {
        match self {
            Self::Answered { outcome, .. } => Some(outcome),
            Self::ClosureStopped { .. } => None,
        }
    }

    /// The reasoning certificate, when a closure was produced.
    #[must_use]
    pub const fn report(&self) -> Option<&ReasoningReport> {
        match self {
            Self::Answered { report, .. } => Some(report),
            Self::ClosureStopped { .. } => None,
        }
    }

    /// The governor that stopped this run, in either phase, or `None` if it completed.
    ///
    /// One accessor over both phases, so a caller deciding an exit code or a retry writes
    /// the decision once rather than per phase.
    #[must_use]
    pub const fn tripped(&self) -> Option<TrippedGovernor> {
        match self {
            Self::Answered { outcome, .. } => outcome.tripped(),
            Self::ClosureStopped { tripped } => Some(*tripped),
        }
    }

    /// Whether the closure was computed AND the query over it completed under every ceiling.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        match self {
            Self::Answered { outcome, .. } => outcome.is_complete(),
            Self::ClosureStopped { .. } => false,
        }
    }
}

/// A SPARQL execution's [`StopSignal`] seen through the reasoner's own stop trait.
///
/// The two traits are deliberately not one. `purrdf-datalog` has no dependency on
/// `purrdf-core` and must acquire none — it is the substrate every rule engine sits on — so
/// it declares the two-line yes/no question it needs, and `purrdf-sparql-eval` declares the
/// richer one its evaluator needs (a [`StopCause`], for the receipt). This adapter is the
/// one place they meet, and it is one method long.
///
/// The observed cause is REMEMBERED rather than re-polled. Both traits' contracts say a
/// signal latches, so re-polling would answer the same thing — but "the reason this run
/// stopped" is then a fact about a value the caller owns, and this way it is a fact about
/// what actually happened here.
#[derive(Debug)]
struct ClosureStop {
    /// The SPARQL execution's own signal, shared with the evaluation phase.
    signal: Arc<dyn StopSignal>,
    /// The cause observed the first time the signal fired, as a [`StopCause`] discriminant
    /// (`0` = never fired, `1` = cancelled, `2` = deadline).
    observed: AtomicU8,
}

impl ClosureStop {
    /// The cause this signal fired with, if it fired while the closure was being computed.
    fn cause(&self) -> Option<StopCause> {
        match self.observed.load(Ordering::Relaxed) {
            1 => Some(StopCause::Cancelled),
            2 => Some(StopCause::Deadline),
            _ => None,
        }
    }
}

impl purrdf_datalog::StopSignal for ClosureStop {
    fn stopped(&self) -> bool {
        let Some(cause) = self.signal.poll() else {
            return false;
        };
        self.observed.store(
            match cause {
                StopCause::Cancelled => 1,
                StopCause::Deadline => 2,
            },
            Ordering::Relaxed,
        );
        true
    }
}

/// Evaluate SPARQL under an explicit native entailment regime, under caller-supplied
/// execution governors, and say what the reasoner did.
///
/// The governed sibling of [`query_with_entailment`], which keeps its signature and its
/// behaviour exactly: an ungoverned entailment query is the same call it always was.
///
/// # What is governed, and by what
///
/// An entailment-regime query is two phases, and they are governed by different halves of
/// [`QueryGovernors`] for a reason that is about semantics rather than about effort.
///
/// **Phase two — the SPARQL evaluation over the materialized closure — is governed
/// completely.** It runs through [`NativeSparqlEngine::query_prepared_governed_view`], so
/// every ceiling the caller named is in force over the closure exactly as it is over any
/// other frozen dataset: fuel, the answer cap, intermediate cells, scratch bytes, remote
/// requests, and the stop signal. A trip there is a [`GovernedOutcome::BudgetExhausted`]
/// carrying certified partial answers, and it arrives on
/// [`GovernedEntailment::Answered`] beside the closure's [`ReasoningReport`].
///
/// **Phase one — materializing the closure — honours the STOP SIGNAL and nothing else.**
/// That is not an omission and it is not a smaller version of the ceilings; it is the only
/// thing that can be honoured there without changing what a closure IS. A numeric ceiling on
/// a reasoning run is a *charge schedule* — some tally of rounds, facts or steps, priced per
/// lane — and a caller-settable one would mean two callers materializing the same regime
/// over the same data get different closures, which is the semantic optionality
/// `purrdf-datalog` states its [fixed
/// budgets](purrdf_datalog#budgets-are-constants-not-knobs) to prevent. A stop signal has no
/// such property: it either lets the closure finish, in which case it is bit-for-bit the
/// closure [`query_with_entailment`] would have computed, or it ends the run with
/// [`GovernedEntailment::ClosureStopped`] and nothing at all. See
/// [`purrdf_entail::materialize_until`] for the boundaries each lane polls it at.
///
/// # The combined approach's witnesses cannot escape through a PARTIAL answer
///
/// The OWL-Direct lane's filtration is the same one [`query_with_entailment`] applies, and
/// it is applied at the same two points — but the governed path has a third place a witness
/// could reach a caller, and it is closed here rather than assumed shut:
///
/// * a solution sequence is restricted **before** evaluation — every leaf that binds a term
///   is wrapped in a `MINUS` against the witness list — so the algebra the governed evaluator
///   runs is already the restricted one. A partial answer is a prefix or a sub-bag of THAT evaluation's
///   rows, so no observable variable can bind a witness in a partial answer either — the
///   restriction is upstream of the truncation, not applied to its output.
/// * a constructed graph is scrubbed **after**, because a `DESCRIBE` reaches triples no
///   variable names — and the scrub runs over the partial answers as well as the complete
///   result, through
///   [`purrdf_sparql_eval::PartialAnswers::withholding_blank_nodes`]. That API performs
///   the removal itself rather than exposing mutable certified rows: it preserves a lower
///   bound, and conservatively withholds an upper bound altogether if a witness-bearing
///   item had to be removed.
///
/// # Errors
///
/// Returns [`ReasoningError::Query`] for SPARQL failures and
/// [`ReasoningError::Entailment`] for malformed or inconsistent knowledge bases. A tripped
/// governor is **not** an error in either phase: it is one of the two arms of
/// [`GovernedEntailment`].
pub fn query_with_entailment_governed(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    request: SparqlRequest<'_>,
    entailment: QueryEntailment<'_>,
    governors: &QueryGovernors,
) -> Result<GovernedEntailment, ReasoningError> {
    // Parse first, exactly as the ungoverned lane does: an invalid query is a failure rather
    // than a budget, and it must be one before any closure work is charged for.
    let prepared_query = engine.prepare_query(request.query, request.base_iri)?;
    let pattern = match entailment {
        QueryEntailment::OwlDirect => collect_query_bgp(&prepared_query.query),
        _ => Vec::new(),
    };
    // The execution's stop signal, wearing the reasoner's trait. Built once and shared by
    // both phases, so a deadline that has already expired when the closure finishes is the
    // SAME latched deadline the evaluator then observes — a query cannot outrun it by
    // crossing the phase boundary.
    let stop: Option<Arc<ClosureStop>> = governors.stop_signal().map(|signal| {
        Arc::new(ClosureStop {
            signal: Arc::clone(signal),
            observed: AtomicU8::new(0),
        })
    });
    let closure_stop: Option<Arc<dyn purrdf_datalog::StopSignal>> = stop
        .as_ref()
        .map(|stop| Arc::clone(stop) as Arc<dyn purrdf_datalog::StopSignal>);
    let closure_stop = closure_stop.as_ref();

    let mut combined_surrogates = None;
    let materialized = match entailment {
        QueryEntailment::Simple => Ok((Arc::clone(dataset), simple_report())),
        QueryEntailment::Rdf => {
            purrdf_entail::materialize_until(dataset, Materialization::Rdf, closure_stop)
        }
        QueryEntailment::Rdfs => {
            purrdf_entail::materialize_until(dataset, Materialization::Rdfs, closure_stop)
        }
        QueryEntailment::OwlRl => {
            purrdf_entail::materialize_until(dataset, Materialization::OwlRl, closure_stop)
        }
        QueryEntailment::D => {
            purrdf_entail::materialize_until(dataset, Materialization::D, closure_stop)
        }
        QueryEntailment::OwlDirect => {
            match materialize_combined_until(dataset, &pattern, closure_stop) {
                Ok(Some(combined)) => {
                    combined_surrogates = Some(combined.surrogates);
                    Ok((combined.dataset, combined.report))
                }
                Ok(None) => purrdf_entail::materialize_until(
                    dataset,
                    Materialization::OwlDirect(&pattern),
                    closure_stop,
                )
                .map(|(closure, report)| (closure, report.with_boundary(Construct::NonHornTBox))),
                Err(error) => Err(error),
            }
        }
        QueryEntailment::Rif(ruleset) => {
            purrdf_entail::materialize_until(dataset, Materialization::Rif(ruleset), closure_stop)
        }
    };
    let (prepared, report) = match materialized {
        Ok(pair) => pair,
        // The one refusal that is an OUTCOME rather than a failure. The cause is what the
        // adapter observed when it fired, so the receipt names the caller's own signal.
        Err(EntailError::Stopped) => {
            return Ok(GovernedEntailment::ClosureStopped {
                tripped: TrippedGovernor::Stopped {
                    cause: stop.as_ref().and_then(|stop| stop.cause()).unwrap_or(
                        // Unreachable through either shipped signal: `EntailError::Stopped`
                        // is produced only by the adapter above, which records the cause on
                        // the same call that returns `true`. A host signal that answered
                        // `Some` once and `None` afterwards would violate the latching
                        // contract both traits state; it is reported as a cancellation
                        // rather than invented as a deadline, because a deadline is a
                        // measurement and there would be none to report.
                        StopCause::Cancelled,
                    ),
                },
            });
        }
        Err(error) => return Err(error.into()),
    };

    // The combined approach's filtration, in the SAME two places and the same order the
    // ungoverned lane applies it: the restriction is in the algebra (so it is upstream of
    // any truncation), and the scrub is over the result (so it reaches a `DESCRIBE`'s
    // triples). See this function's documentation for why a partial answer needs both.
    let surrogates = combined_surrogates.unwrap_or_default();
    let restricted = (!surrogates.is_empty())
        .then(|| {
            PreparedQuery::rewritten(
                restrict_witness_bindings(&prepared_query.query, &surrogates),
                QueryOptions::EMPTY,
            )
        })
        .transpose()?;
    let plan = restricted.as_ref().unwrap_or(&prepared_query);
    // `QueryOptions::EMPTY`: this lane owns its plan (it rewrites the algebra to restrict
    // chase-minted witnesses) and exposes no registry seam of its own, so there is nothing
    // for a caller to have configured and nothing that could be silently dropped here. A
    // host that wants relations in scope names them at the engine's own governed entries.
    let outcome = engine.query_prepared_governed_view(
        &*prepared,
        plan,
        request.substitutions,
        QueryOptions::EMPTY,
        governors,
    )?;
    Ok(GovernedEntailment::Answered {
        outcome: withhold_surrogates_from_outcome(outcome, &surrogates),
        report,
    })
}

/// Scrub every chase-minted witness out of a governed outcome, complete or partial.
///
/// A no-op when the run minted no witness, which is every lane but the OWL-Direct combined
/// approach — so the ordinary governed query pays one `is_empty` for the guarantee.
fn withhold_surrogates_from_outcome(
    outcome: GovernedOutcome,
    surrogates: &BTreeSet<String>,
) -> GovernedOutcome {
    if surrogates.is_empty() {
        return outcome;
    }
    match outcome {
        GovernedOutcome::Complete {
            mut result,
            evidence,
            relations,
        } => {
            withhold_surrogate_triples(&mut result, surrogates);
            GovernedOutcome::Complete {
                result,
                evidence,
                relations,
            }
        }
        GovernedOutcome::BudgetExhausted(exhausted) => {
            GovernedOutcome::BudgetExhausted(BudgetExhausted {
                partial: exhausted
                    .partial
                    .withholding_blank_nodes(|label| label_is_surrogate(label, surrogates)),
                ..exhausted
            })
        }
    }
}

/// The query's OBSERVABLE variable names — the ones whose binding a caller can read off, or
/// compute a returned value from, in this query's answer.
///
/// # The reading, stated once
///
/// A chase-minted witness is a legitimate value for a variable the answer never exposes (that
/// is the whole point of the combined approach: `?y` in `SELECT ?x WHERE { ?x r ?y . ?y a B }`
/// is existential, and binding it to the witness is what makes `?x = a` findable). It is NOT
/// a legitimate value for a variable whose binding leaves the query, because a SPARQL
/// entailment regime draws its answers from the scoping graph and a minted witness is not in
/// it. "Observable" is that distinction, and it is decided per query form:
///
/// * `SELECT` — the projected variables. Plus, everywhere in the pattern, the variables an
///   `Extend` expression READS (a `BIND`/select-expression turns a binding into a returned
///   term), the `GROUP BY` key variables (the grouping decides how many rows come back), and
///   the variables an aggregate reads (`COUNT(?y)` turns `?y`'s multiplicity into a returned
///   number, which is why the aggregate must see the restricted sequence rather than the raw
///   one). A `COUNT(*)` reads no variable and still counts ROWS, so it makes every variable of
///   the grouped pattern observable — row multiplicity is a function of all of them.
/// * `CONSTRUCT` — the TEMPLATE's variables. Every one of them becomes a term of the emitted
///   graph.
/// * `DESCRIBE` — the target variables. The triples themselves are scrubbed separately (see
///   [`withhold_surrogate_triples`]), because a `DESCRIBE` reaches triples no variable names.
/// * `ASK` — none. An `ASK` returns a boolean and exposes no term, and the boolean is exactly
///   the entailment `KB ⊨ ∃x⃗. BGP` that the witness is evidence FOR: withholding it would
///   answer `false` to a question whose certain answer is `true`.
///
/// Two things are deliberately NOT observable. A `FILTER` reads a variable to decide a row's
/// fate without returning its value, and constraining an existential variable is what a
/// filter over a non-distinguished variable means. `ORDER BY` reads one to decide row ORDER;
/// the rows it orders are certain answers either way, and the witness labels are content
/// digests, so the order is deterministic rather than arbitrary. Neither puts a witness in
/// front of the caller.
fn observable_variables(query: &Query) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let pattern = query_pattern(query);
    match query {
        Query::Select { .. } => match find_projection(pattern) {
            Some(projected) => names.extend(projected),
            // A `SELECT` whose algebra carries no `Project` is not a shape the parser
            // produces; if one ever reaches here, every variable is observable, because the
            // conservative answer is the only one that cannot leak.
            None => collect_all_variables(pattern, &mut names),
        },
        Query::Construct { template, .. } => {
            for triple in template {
                collect_triple_pattern_variables(triple, &mut names);
            }
        }
        Query::Describe { targets, .. } => {
            for target in targets {
                if let NamedNodePattern::Variable(variable) = target {
                    names.insert(variable.as_str().to_owned());
                }
            }
        }
        Query::Ask { .. } => {}
    }
    collect_returned_value_variables(pattern, &mut names);
    names
}

/// The root graph pattern of any query form.
fn query_pattern(query: &Query) -> &GraphPattern {
    match query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => pattern,
    }
}

/// The variable list of the first [`GraphPattern::Project`] reached by peeling off solution
/// modifiers — `SELECT`'s own root pattern is exactly that, wrapped by
/// `Slice`/`OrderBy`/`Distinct`/`Reduced`/`Group` and the like. `None` if none is found
/// (there is no `SELECT` projection to read).
fn find_projection(pattern: &GraphPattern) -> Option<BTreeSet<String>> {
    match pattern {
        GraphPattern::Project { variables, .. } => {
            Some(variables.iter().map(|v| v.as_str().to_owned()).collect())
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => find_projection(inner),
        _ => None,
    }
}

/// Every variable an `Extend` expression, a `GROUP BY` key or an aggregate READS — the
/// variables whose bindings become returned VALUES rather than returned bindings.
///
/// `Expression::Exists`'s inner pattern is deliberately not descended into: an `EXISTS`
/// yields a boolean and no binding of its own escapes, so a witness inside one is invisible
/// for the same reason an `ASK`'s is.
fn collect_returned_value_variables(pattern: &GraphPattern, names: &mut BTreeSet<String>) {
    match pattern {
        GraphPattern::Extend {
            inner, expression, ..
        } => {
            collect_expression_variables(expression, names);
            collect_returned_value_variables(inner, names);
        }
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => {
            names.extend(variables.iter().map(|v| v.as_str().to_owned()));
            for (_, aggregate) in aggregates {
                match aggregate {
                    // `COUNT(*)` names no variable and returns row MULTIPLICITY, which every
                    // variable of the grouped pattern contributes to — so all of them are
                    // observable through it.
                    AggregateExpression::CountStar { .. } => {
                        collect_all_variables(inner, names);
                    }
                    AggregateExpression::FunctionCall { expression, .. } => {
                        collect_expression_variables(expression, names);
                    }
                }
            }
            collect_returned_value_variables(inner, names);
        }
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::LeftJoin { left, right, .. } => {
            collect_returned_value_variables(left, names);
            collect_returned_value_variables(right, names);
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => collect_returned_value_variables(inner, names),
        // A relation READS its argument variables and derives its output rows from what
        // they are bound to. The relation is caller code, so any function of an input
        // cell may appear in an output cell — a witness reaching an argument can surface
        // as a returned VALUE exactly the way an `Extend` expression's would. Reporting
        // both argument sides is therefore the honest answer as well as the conservative
        // one: which side is input and which is output is decided per relation at
        // evaluation time and is not visible in the algebra, and the output positions are
        // returned bindings outright.
        GraphPattern::PropertyFunction(call) => collect_call_variables(call, names),
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => {}
    }
}

/// Every variable of a property-function call's arguments — subject side, then object
/// side.
///
/// One set over both sides because the node's arguments simply ARE its variables: the
/// algebra does not say which side is bound on input and which is produced, and every one
/// of them is visible in the enclosing group graph pattern.
fn collect_call_variables(call: &PropertyFunctionCall, names: &mut BTreeSet<String>) {
    for term in call.subject_args.iter().chain(&call.object_args) {
        collect_term_pattern_variable(term, names);
    }
}

/// Every variable an expression reads, `EXISTS` bodies excepted (see
/// [`collect_returned_value_variables`]).
fn collect_expression_variables(expression: &Expression, names: &mut BTreeSet<String>) {
    match expression {
        Expression::Variable(variable) | Expression::Bound(variable) => {
            names.insert(variable.as_str().to_owned());
        }
        Expression::NamedNode(_) | Expression::Literal(_) | Expression::Exists(_) => {}
        Expression::Or(left, right)
        | Expression::And(left, right)
        | Expression::Equal(left, right)
        | Expression::SameTerm(left, right)
        | Expression::Greater(left, right)
        | Expression::GreaterOrEqual(left, right)
        | Expression::Less(left, right)
        | Expression::LessOrEqual(left, right)
        | Expression::Add(left, right)
        | Expression::Subtract(left, right)
        | Expression::Multiply(left, right)
        | Expression::Divide(left, right) => {
            collect_expression_variables(left, names);
            collect_expression_variables(right, names);
        }
        Expression::UnaryPlus(inner) | Expression::UnaryMinus(inner) | Expression::Not(inner) => {
            collect_expression_variables(inner, names);
        }
        Expression::In(inner, list) => {
            collect_expression_variables(inner, names);
            for item in list {
                collect_expression_variables(item, names);
            }
        }
        Expression::If(condition, then, otherwise) => {
            collect_expression_variables(condition, names);
            collect_expression_variables(then, names);
            collect_expression_variables(otherwise, names);
        }
        Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
            for item in list {
                collect_expression_variables(item, names);
            }
        }
    }
}

/// Every variable mentioned anywhere in `pattern` — the conservative answer, used where a
/// precise one is unavailable (`COUNT(*)`, or a `SELECT` with no projection to read).
fn collect_all_variables(pattern: &GraphPattern, names: &mut BTreeSet<String>) {
    match pattern {
        GraphPattern::Bgp { patterns } => {
            for triple in patterns {
                collect_triple_pattern_variables(triple, names);
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            collect_term_pattern_variable(subject, names);
            collect_term_pattern_variable(object, names);
        }
        GraphPattern::Values { variables, .. } | GraphPattern::Project { variables, .. } => {
            names.extend(variables.iter().map(|v| v.as_str().to_owned()));
            if let GraphPattern::Project { inner, .. } = pattern {
                collect_all_variables(inner, names);
            }
        }
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::LeftJoin { left, right, .. } => {
            collect_all_variables(left, names);
            collect_all_variables(right, names);
        }
        GraphPattern::Extend {
            inner, variable, ..
        } => {
            names.insert(variable.as_str().to_owned());
            collect_all_variables(inner, names);
        }
        GraphPattern::Graph { name, inner } => {
            if let NamedNodePattern::Variable(variable) = name {
                names.insert(variable.as_str().to_owned());
            }
            collect_all_variables(inner, names);
        }
        GraphPattern::Group {
            inner, variables, ..
        } => {
            names.extend(variables.iter().map(|v| v.as_str().to_owned()));
            collect_all_variables(inner, names);
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => collect_all_variables(inner, names),
        GraphPattern::PropertyFunction(call) => collect_call_variables(call, names),
    }
}

/// The variables of one triple pattern, in all three positions.
fn collect_triple_pattern_variables(triple: &TriplePattern, names: &mut BTreeSet<String>) {
    collect_term_pattern_variable(&triple.subject, names);
    if let NamedNodePattern::Variable(variable) = &triple.predicate {
        names.insert(variable.as_str().to_owned());
    }
    collect_term_pattern_variable(&triple.object, names);
}

/// `term`'s variable name, if it is one — descending into an RDF 1.2 quoted triple,
/// whose nested variables bind exactly the way a top-level one does and can therefore
/// carry a witness just as visibly.
fn collect_term_pattern_variable(term: &TermPattern, names: &mut BTreeSet<String>) {
    match term {
        TermPattern::Variable(variable) => {
            names.insert(variable.as_str().to_owned());
        }
        TermPattern::Triple(triple) => collect_triple_pattern_variables(triple, names),
        TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {}
    }
}

/// `query` rewritten so that NO observable variable can bind a chase-minted witness.
///
/// # Why the restriction is in the algebra and not in the result
///
/// This used to be a pass over the returned rows that dropped any row mentioning a witness
/// anywhere, and that reading lost correct answers outright. `SELECT ?x ?y WHERE { ?x a A .
/// OPTIONAL { ?x r ?y . ?y a B } }` returned ZERO rows over an ABox that literally asserts
/// `a a A`: the `OPTIONAL` matched a witness for `?y`, and dropping the row threw away the
/// left operand's own certain answer with it. It also could not touch an aggregate —
/// `SELECT (COUNT(?y) AS ?n)` had already counted the witnesses by the time the rows arrived —
/// and a `CONSTRUCT` template emitted the internal witness label verbatim.
///
/// Forbidding the BINDING instead of censoring the ROW fixes all three at once, because every
/// operator above the restriction then does its own job correctly and unaided: `OPTIONAL`
/// sees an empty right operand and left-joins `?y` UNBOUND (which is precisely SPARQL's
/// reading — the row survives, the variable is not in the solution's domain), `COUNT` sees the
/// restricted sequence and counts what is in it, and a `CONSTRUCT` template is never handed a
/// term it must not emit. No hand-rolled solution-sequence surgery is involved, and no
/// question about duplicate rows or bag cardinality has to be answered by this module,
/// because the sequence is the one SPARQL itself produces for the restricted pattern.
///
/// # The rewrite
///
/// Every leaf that BINDS a term — a `Bgp` and a `Path` — is wrapped in one `MINUS` per
/// observable variable it binds, against a one-column `VALUES` listing the witnesses. SPARQL's
/// `MINUS` removes a solution only when a right-hand solution is compatible with it AND
/// shares a bound variable, which is exactly "this variable is bound to one of these terms";
/// a row where the variable is unbound, or bound to anything else, survives untouched. An
/// inline `VALUES` in the query itself is left alone: its cells come from the query text, and
/// a witness label is this module's own digest-prefixed string that no parser produces.
///
/// `EXISTS` bodies are left alone for the reason [`observable_variables`] gives: nothing binds
/// out of one.
fn restrict_witness_bindings(query: &Query, surrogates: &BTreeSet<String>) -> Query {
    let observable = observable_variables(query);
    // The algebra's single blank slot carries the SCOPE-QUALIFIED rendering (the
    // evaluator decodes it back to `(label, scope)`), and a witness is minted at
    // the default scope, so the cell is the qualification of the raw label.
    let witnesses: Vec<Vec<Option<GroundTerm>>> = surrogates
        .iter()
        .map(|label| {
            let qualified = purrdf_rdf::BlankScope::DEFAULT.qualify_label(label);
            vec![Some(GroundTerm::BlankNode(BlankNode::new(
                qualified.into_owned(),
            )))]
        })
        .collect();
    let restrict = |pattern: &GraphPattern| restrict_pattern(pattern, &observable, &witnesses);
    match query {
        Query::Select {
            pattern,
            dataset,
            base_iri,
        } => Query::Select {
            pattern: restrict(pattern),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
        },
        Query::Construct {
            template,
            pattern,
            dataset,
            base_iri,
        } => Query::Construct {
            template: template.clone(),
            pattern: restrict(pattern),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
        },
        Query::Describe {
            pattern,
            targets,
            dataset,
            base_iri,
        } => Query::Describe {
            pattern: restrict(pattern),
            targets: targets.clone(),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
        },
        Query::Ask {
            pattern,
            dataset,
            base_iri,
        } => Query::Ask {
            pattern: restrict(pattern),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
        },
    }
}

/// [`restrict_witness_bindings`] over one graph pattern.
fn restrict_pattern(
    pattern: &GraphPattern,
    observable: &BTreeSet<String>,
    witnesses: &[Vec<Option<GroundTerm>>],
) -> GraphPattern {
    let recurse = |inner: &GraphPattern| Box::new(restrict_pattern(inner, observable, witnesses));
    match pattern {
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } => {
            let mut bound = BTreeSet::new();
            collect_all_variables(pattern, &mut bound);
            exclude_witnesses(pattern.clone(), bound.intersection(observable), witnesses)
        }
        // A call and an inline `VALUES` are the two leaves that bind terms WITHOUT reading
        // the entailed graph, so witness restriction over either of them is the identity.
        //
        // For a call the argument runs both ways, and both ways it holds. Nothing it emits
        // can be a witness: its rows come from the injected relation registry, and a
        // witness label is this module's own digest-prefixed string minted inside the
        // chase, which no registry ever saw. Nothing it READS can be one either, because
        // [`collect_returned_value_variables`] reports every argument variable of every
        // call as observable, so whichever `Bgp` or `Path` binds one has already been
        // wrapped against the witness `VALUES` by the arm above — the call is handed a
        // witness-free row by construction.
        //
        // Wrapping it the way a `Bgp` is wrapped would be wrong twice over besides. The
        // `MINUS` would land between the enclosing `Lateral` and the call, and
        // `Lateral(left, PropertyFunction)` is the shape the evaluator dispatches a call
        // on; and the arguments it would constrain include the call's INPUTS, which a
        // relation may require to be bound — restricting them there could turn a relation
        // that only offers a bound-input mode into an infeasible plan.
        GraphPattern::Values { .. } | GraphPattern::PropertyFunction(_) => pattern.clone(),
        GraphPattern::Join { left, right } => GraphPattern::Join {
            left: recurse(left),
            right: recurse(right),
        },
        GraphPattern::Union { left, right } => GraphPattern::Union {
            left: recurse(left),
            right: recurse(right),
        },
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: recurse(left),
            right: recurse(right),
        },
        GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
            left: recurse(left),
            right: recurse(right),
        },
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => GraphPattern::LeftJoin {
            left: recurse(left),
            right: recurse(right),
            expression: expression.clone(),
        },
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr: expr.clone(),
            inner: recurse(inner),
        },
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: name.clone(),
            inner: recurse(inner),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => GraphPattern::Extend {
            inner: recurse(inner),
            variable: variable.clone(),
            expression: expression.clone(),
        },
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => GraphPattern::Service {
            name: name.clone(),
            inner: recurse(inner),
            silent: *silent,
        },
        GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
            inner: recurse(inner),
            expression: expression.clone(),
        },
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: recurse(inner),
            variables: variables.clone(),
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: recurse(inner),
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: recurse(inner),
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: recurse(inner),
            start: *start,
            length: *length,
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => GraphPattern::Group {
            inner: recurse(inner),
            variables: variables.clone(),
            aggregates: aggregates.clone(),
        },
    }
}

/// `pattern` wrapped in one `MINUS` per named variable, each against the witness `VALUES`.
///
/// One `MINUS` per variable rather than one multi-column `VALUES` for all of them, because
/// `MINUS` requires EVERY shared column to be compatible: a two-column right operand would
/// only remove a row whose two variables were both bound to a witness, which is not the
/// condition being excluded.
fn exclude_witnesses<'a>(
    pattern: GraphPattern,
    variables: impl Iterator<Item = &'a String>,
    witnesses: &[Vec<Option<GroundTerm>>],
) -> GraphPattern {
    let mut restricted = pattern;
    for name in variables {
        restricted = GraphPattern::Minus {
            left: Box::new(restricted),
            right: Box::new(GraphPattern::Values {
                variables: vec![Variable::new(name.clone())],
                bindings: witnesses.to_vec(),
            }),
        };
    }
    restricted
}

/// Drop every triple of a `CONSTRUCT`/`DESCRIBE` result graph that MENTIONS a chase-minted
/// witness, in any position and at any depth inside a triple term.
///
/// A no-op for a solution sequence and an `ASK` boolean: the sequence is restricted before
/// evaluation ([`restrict_witness_bindings`]) and the boolean exposes no term.
///
/// A constructed triple ABOUT an anonymous witness asserts nothing the scoping graph licenses
/// — the witness names no element the ontology identifies, so a caller who reads the emitted
/// graph learns only this module's internal label — so dropping the triple is the whole of the
/// correct behaviour, not a truncation of it. For `CONSTRUCT` the restriction above already
/// makes this unreachable, since every template variable is observable; `DESCRIBE` is why it
/// exists, because a `DESCRIBE <iri>` reaches the dataset's triples directly and no variable
/// of that query names the witness those triples mention.
///
/// The graph is rebuilt only if a witness is actually present, so the ordinary case pays
/// nothing and the RDF 1.2 statement-layer overlay of an untouched result is carried through
/// by identity rather than by a copy.
fn withhold_surrogate_triples(result: &mut SparqlResult, surrogates: &BTreeSet<String>) -> bool {
    if surrogates.is_empty() {
        return false;
    }
    let SparqlResult::Graph(graph) = result else {
        return false;
    };
    let mentions = |term: &RdfTerm| term_mentions_surrogate(term, surrogates);
    let quad_offends = |quad: &RdfQuad| {
        mentions(&quad.subject)
            || mentions(&quad.object)
            || quad.graph_name.as_ref().is_some_and(&mentions)
    };
    let reifier_offends = |reifier: &purrdf_rdf::RdfReifier| {
        mentions(&reifier.reifier)
            || mentions(&reifier.statement.subject)
            || mentions(&reifier.statement.object)
            || reifier.graph.as_ref().is_some_and(&mentions)
    };
    let annotation_offends = |annotation: &purrdf_rdf::RdfAnnotation| {
        mentions(&annotation.reifier)
            || mentions(&annotation.object)
            || annotation.graph.as_ref().is_some_and(&mentions)
    };
    let offends = graph.owned_quads().any(|quad| quad_offends(&quad))
        || graph.owned_reifiers().any(|r| reifier_offends(&r))
        || graph.owned_annotations().any(|a| annotation_offends(&a));
    if !offends {
        return false;
    }
    let mut builder = RdfDatasetBuilder::new();
    for quad in graph.owned_quads() {
        if !quad_offends(&quad) {
            builder.push_owned_quad(&quad);
        }
    }
    for reifier in graph.owned_reifiers() {
        if !reifier_offends(&reifier) {
            builder.push_owned_reifier(&reifier);
        }
    }
    for annotation in graph.owned_annotations() {
        if !annotation_offends(&annotation) {
            builder.push_owned_annotation(&annotation);
        }
    }
    for name in graph.owned_named_graphs() {
        if !mentions(&name) {
            let id = builder.intern_owned_term(&name);
            builder.declare_named_graph(id);
        }
    }
    *graph = builder
        .freeze()
        .expect("a subset of an already-frozen dataset's quads is itself a valid dataset");
    true
}

/// Whether an owned-model blank label denotes a chase-minted witness.
///
/// The surrogate set holds the RAW labels `combined::witness_label` minted (a
/// digest under a reserved prefix, which contains `.` separators), while an owned
/// [`RdfTerm::BlankNode`] carries the SCOPE-QUALIFIED rendering of whatever label
/// the dataset holds. Decoding the rendering is the exact inverse of that
/// qualification, so the comparison is against the label that was actually
/// minted — and a witness label carried at a non-default scope, whose rendering
/// IS an envelope, still compares equal to the raw label it was minted as.
fn label_is_surrogate(label: &str, surrogates: &BTreeSet<String>) -> bool {
    let (raw, _scope) = purrdf_rdf::BlankScope::unqualify_label(label);
    surrogates.contains(raw.as_ref())
}

/// Whether `term` IS a chase-minted witness, or quotes one at any depth.
fn term_mentions_surrogate(term: &RdfTerm, surrogates: &BTreeSet<String>) -> bool {
    match term {
        RdfTerm::BlankNode(label) => label_is_surrogate(label, surrogates),
        RdfTerm::Iri(_) | RdfTerm::Literal(_) => false,
        RdfTerm::Triple(triple) => {
            term_mentions_surrogate(&triple.subject, surrogates)
                || term_mentions_surrogate(&triple.object, surrogates)
        }
    }
}

/// The report for the identity closure — what `materialize(ds, Materialization::Simple)` returns.
///
/// Assembled rather than obtained by calling it, because that call COPIES the dataset to
/// produce a closure this lane already has as an `Arc`. Every field is a property of the
/// regime and not of the data: `Simple` has no rule table (so nothing can be missing), it
/// copies every quad of every graph faithfully (so it meets no boundary), and it evaluates
/// no program (so it consumes none of the three ceilings), and it invents no term (so it
/// has no termination obligation to discharge). The contract hash is derived inside
/// [`ReasoningReport::new`] from the regime itself.
fn simple_report() -> ReasoningReport {
    ReasoningReport::new(
        Regime::Simple,
        Vec::new(),
        Vec::new(),
        BudgetReport::new(0, 0, 0),
        None,
        0,
        None,
    )
}

fn collect_query_bgp(query: &Query) -> Vec<QTriple> {
    let pattern = match query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => pattern,
    };
    let mut triples = Vec::new();
    collect_bgp(pattern, &mut triples);
    triples
}

fn collect_bgp(pattern: &GraphPattern, output: &mut Vec<QTriple>) {
    match pattern {
        GraphPattern::Bgp { patterns } => output.extend(patterns.iter().filter_map(|pattern| {
            Some(QTriple {
                s: term_to_qnode(&pattern.subject)?,
                p: named_node_pattern_to_qnode(&pattern.predicate),
                o: term_to_qnode(&pattern.object)?,
            })
        })),
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::LeftJoin { left, right, .. } => {
            collect_bgp(left, output);
            collect_bgp(right, output);
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => collect_bgp(inner, output),
        // Leaves that hold no triple pattern. A property-function call matches no triple
        // in any graph — its rows come from the relation registry — so it contributes
        // nothing to the pattern that drives OWL Direct's query-directed augmentation,
        // exactly as a `Path` or an inline `VALUES` contributes nothing.
        GraphPattern::Path { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::PropertyFunction(_) => {}
    }
}

fn term_to_qnode(term: &TermPattern) -> Option<QNode> {
    Some(match term {
        TermPattern::Variable(variable) => QNode::Var(variable.as_str().to_owned()),
        TermPattern::NamedNode(node) => QNode::Term(TermValue::iri(node.as_str())),
        TermPattern::BlankNode(node) => QNode::Term(TermValue::blank(node.as_str())),
        TermPattern::Literal(literal) => QNode::Term(literal_to_term_value(literal)),
        TermPattern::Triple(_) => return None,
    })
}

fn named_node_pattern_to_qnode(pattern: &NamedNodePattern) -> QNode {
    match pattern {
        NamedNodePattern::NamedNode(node) => QNode::Term(TermValue::iri(node.as_str())),
        NamedNodePattern::Variable(variable) => QNode::Var(variable.as_str().to_owned()),
    }
}

fn literal_to_term_value(literal: &Literal) -> TermValue {
    match literal.language() {
        Some(language) => TermValue::Literal {
            lexical_form: literal.value().to_owned(),
            datatype: literal.datatype().as_str().to_owned(),
            language: Some(language.to_ascii_lowercase()),
            direction: literal.direction().map(|direction| match direction {
                BaseDirection::Ltr => RdfTextDirection::Ltr,
                BaseDirection::Rtl => RdfTextDirection::Rtl,
            }),
        },
        None => TermValue::typed_literal(literal.value(), literal.datatype().as_str()),
    }
}

#[cfg(test)]
mod tests {
    use purrdf_entail::{Atom, RifTerm, Rule, RuleSet};
    use purrdf_rdf::{BlankScope, RdfDatasetBuilder, TermValue};

    use super::*;

    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const RDFS_SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

    /// A caller can walk from the wrapper to the failure it wraps.
    ///
    /// `ReasoningError` adds nothing of its own — it says only which subsystem failed —
    /// so before it carried a `source` the standard chain reached that fork and stopped,
    /// and the diagnostic underneath was reachable only by matching the concrete enum.
    /// That is the situation `Error::source` exists to remove, and this asserts it is
    /// gone rather than trusting the impl to be there.
    ///
    /// The inner error is compared by its RENDERED text, not by identity: the claim a
    /// caller depends on is that walking the chain reaches the message describing the
    /// real failure, which is what a `{:#}` printer or a `source()` loop will show.
    #[test]
    fn the_error_chain_reaches_the_wrapped_failure() {
        use std::error::Error as _;

        let inner = EntailError::UnsupportedRegime(Regime::Rif);
        let rendered = inner.to_string();
        let wrapped = ReasoningError::Entailment(inner);

        let source = wrapped
            .source()
            .expect("the entailment wrapper must expose the failure it wraps");
        assert_eq!(
            source.to_string(),
            rendered,
            "walking the chain must reach the wrapped diagnostic itself, not a \
             re-description of it"
        );

        // And the chain terminates rather than cycling: this variant of `EntailError`
        // names a regime and wraps nothing further.
        assert!(
            source.source().is_none(),
            "an error that wraps nothing must end the chain"
        );
    }

    /// A property-function call reaches every algebra walker this module has, and each
    /// one treats it as what it is: a leaf that BINDS its argument variables and READS
    /// no graph.
    ///
    /// The three decisions asserted together, because they depend on each other. The
    /// call's argument variables are observable, so whatever `Bgp` binds one is
    /// witness-restricted; the call itself is therefore already handed witness-free
    /// rows and needs no restriction of its own, which is what lets the restriction be
    /// the identity over it — and that identity is also what preserves the
    /// `Lateral(left, call)` shape the evaluator dispatches a call on.
    #[test]
    fn a_property_function_call_binds_its_arguments_and_reads_no_graph() {
        use purrdf_sparql_algebra::{ParserOptions, SparqlParser};

        let options = ParserOptions {
            extension_fn_namespaces: Vec::new(),
            property_fn_namespaces: vec!["https://example.org/rel/".to_owned()],
            property_fn_iris: Vec::new(),
        };
        let query = SparqlParser::new()
            .parse_query_with(
                "PREFIX rel: <https://example.org/rel/>\n\
                 SELECT ?team WHERE {\n\
                   ?person <https://example.org/name> ?name .\n\
                   ?person rel:memberOf ?team\n\
                 }",
                &options,
            )
            .expect("the query parses under the configured namespace");

        // Both argument variables are observable. `?team` is projected; `?person` is an
        // input the relation reads and may derive an output cell from, which is the same
        // exposure an `Extend` expression's operand has. `?name` is neither.
        let observable = observable_variables(&query);
        assert!(observable.contains("person"), "{observable:?}");
        assert!(observable.contains("team"), "{observable:?}");
        assert!(!observable.contains("name"), "{observable:?}");

        // The call scaffolds nothing for the query-directed OWL-Direct augmentation:
        // only the one triple actually written in the query is there.
        let bgp = collect_query_bgp(&query);
        assert_eq!(bgp.len(), 1, "{bgp:?}");

        // Witness restriction wraps the data leaf and leaves the call alone.
        let surrogates: BTreeSet<String> = std::iter::once("chase.witness.0".to_owned()).collect();
        let restricted = restrict_witness_bindings(&query, &surrogates);
        let Query::Select { pattern, .. } = &restricted else {
            panic!("a SELECT restricts to a SELECT");
        };
        let GraphPattern::Project { inner, .. } = pattern else {
            panic!("a SELECT's algebra root is a Project, got {pattern:?}");
        };
        let GraphPattern::Lateral { left, right } = &**inner else {
            panic!("the call's Lateral must survive the rewrite, got {inner:?}");
        };
        assert!(
            matches!(&**left, GraphPattern::Minus { .. }),
            "the data leaf binding an observable variable is excluded from the \
             witnesses, got {left:?}"
        );
        let GraphPattern::PropertyFunction(call) = &**right else {
            panic!(
                "the Lateral's right operand must still be the bare call — anything \
                 between them is a shape the evaluator does not dispatch on, got {right:?}"
            );
        };
        assert_eq!(call.iri, "https://example.org/rel/memberOf");
    }

    fn hierarchy() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let cat = builder.intern_iri("https://example.org/Cat");
        let animal = builder.intern_iri("https://example.org/Animal");
        let lillith = builder.intern_iri("https://example.org/lillith");
        let rdf_type = builder.intern_iri(RDF_TYPE);
        let subclass = builder.intern_iri(RDFS_SUBCLASS);
        builder.push_quad(cat, subclass, animal, None);
        builder.push_quad(lillith, rdf_type, cat, None);
        builder.freeze().unwrap()
    }

    #[test]
    fn host_query_plan_uses_the_shared_regime_and_program_contract() {
        let rdfs = QueryEntailmentPlan::parse("rdfs", "").expect("fixed regime plan");
        assert!(matches!(rdfs.entailment(), QueryEntailment::Rdfs));
        assert_eq!(rdfs.regime(), Regime::Rdfs);

        let wrong_program = QueryEntailmentPlan::parse("rdfs", "not ignored")
            .expect_err("a fixed calculus cannot silently discard caller rules");
        assert!(
            wrong_program.contains("takes no rule document"),
            "{wrong_program}"
        );

        let missing_rif = QueryEntailmentPlan::parse("rif", "")
            .expect_err("RIF has no caller-independent rule table");
        assert!(missing_rif.contains("rule document"), "{missing_rif}");

        let unknown =
            QueryEntailmentPlan::parse("RDFS", "").expect_err("the cross-host spelling is exact");
        assert!(
            unknown.contains("owl-direct") && unknown.contains("rif"),
            "{unknown}"
        );
    }

    /// Run the fixture ASK under `mode`, returning both halves of the answer.
    fn ask_reported(mode: QueryEntailment<'_>) -> (SparqlResult, ReasoningReport) {
        let query = "ASK { <https://example.org/lillith> a <https://example.org/Animal> }";
        query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            mode,
        )
        .unwrap()
    }

    fn ask(mode: QueryEntailment<'_>) -> SparqlResult {
        ask_reported(mode).0
    }

    #[test]
    fn rdfs_query_sees_derived_type() {
        assert!(matches!(
            ask(QueryEntailment::Rdfs),
            SparqlResult::Boolean(true)
        ));
    }

    #[test]
    fn owl_rl_query_sees_derived_type() {
        assert!(matches!(
            ask(QueryEntailment::OwlRl),
            SparqlResult::Boolean(true)
        ));
    }

    #[test]
    fn owl_direct_query_uses_the_query_bgp() {
        assert!(matches!(
            ask(QueryEntailment::OwlDirect),
            SparqlResult::Boolean(true)
        ));
    }

    /// `entailment/D` IS SELECTABLE, and it is the regime it says it is.
    ///
    /// It is a W3C SPARQL entailment regime and `materialize` serves it like any other rule
    /// table; a query surface that could not name it was withholding a capability the
    /// library has.
    #[test]
    fn the_d_regime_is_reachable_from_the_query_surface() {
        let query = "ASK { <http://www.w3.org/2001/XMLSchema#integer> a \
                     <http://www.w3.org/2000/01/rdf-schema#Datatype> }";
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::D,
        )
        .unwrap();
        // `dt-type1` is premise-free, so every supported datatype is typed in every closure.
        assert!(matches!(result, SparqlResult::Boolean(true)));
        assert_eq!(report.regime(), Regime::D);
        // Simple entailment does NOT derive it, so the answer is the regime's and not the
        // data's.
        assert!(matches!(
            query_with_entailment(
                &NativeSparqlEngine::new(),
                &hierarchy(),
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryEntailment::Simple,
            )
            .unwrap()
            .0,
            SparqlResult::Boolean(false)
        ));
    }

    /// EVERY MODE CARRIES ITS CERTIFICATE OUT, and each names its own regime.
    #[test]
    fn every_mode_returns_the_report_of_the_run_it_made() {
        let rules = RuleSet::new();
        for (mode, regime) in [
            (QueryEntailment::Simple, Regime::Simple),
            (QueryEntailment::Rdf, Regime::Rdf),
            (QueryEntailment::Rdfs, Regime::Rdfs),
            (QueryEntailment::OwlRl, Regime::OwlRl),
            (QueryEntailment::D, Regime::D),
            (QueryEntailment::OwlDirect, Regime::OwlDirect),
            (QueryEntailment::Rif(&rules), Regime::Rif),
        ] {
            let (_, report) = ask_reported(mode);
            assert_eq!(report.regime(), regime, "{regime:?}");
        }
    }

    /// The `Simple` report is EXACTLY the one `materialize` returns for that regime —
    /// assembled without paying for the copy, and checked rather than claimed.
    #[test]
    fn the_simple_report_equals_the_materialized_one() {
        let dataset = hierarchy();
        let (_, from_materialize) =
            purrdf_entail::materialize(&dataset, Materialization::Simple).expect("simple");
        let (_, from_query) = ask_reported(QueryEntailment::Simple);
        assert_eq!(format!("{from_query:?}"), format!("{from_materialize:?}"));
    }

    #[test]
    fn rdf_query_types_predicates_as_properties() {
        let query = format!(
            "ASK {{ <{RDFS_SUBCLASS}> a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> }}"
        );
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::Rdf,
        )
        .unwrap();
        assert!(matches!(result, SparqlResult::Boolean(true)));
        assert_eq!(report.regime(), Regime::Rdf);
    }

    #[test]
    fn simple_query_does_not_invent_closure() {
        assert!(matches!(
            ask(QueryEntailment::Simple),
            SparqlResult::Boolean(false)
        ));
    }

    #[test]
    fn rif_query_sees_rule_derived_fact() {
        let mut rules = RuleSet::new();
        rules.push_rule(Rule {
            body: vec![Atom {
                s: RifTerm::Var("subject".to_owned()),
                p: RifTerm::Const(TermValue::iri(RDF_TYPE)),
                o: RifTerm::Const(TermValue::iri("https://example.org/Cat")),
            }],
            head: vec![Atom {
                s: RifTerm::Var("subject".to_owned()),
                p: RifTerm::Const(TermValue::iri(RDF_TYPE)),
                o: RifTerm::Const(TermValue::iri("https://example.org/Animal")),
            }],
        });
        assert!(matches!(
            ask(QueryEntailment::Rif(&rules)),
            SparqlResult::Boolean(true)
        ));
    }

    // ── The combined approach: a non-distinguished variable, answered correctly ────────

    const COMBINED_NS: &str = "https://example.org/combined#";
    const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
    const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
    const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
    const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";

    /// `A ⊑ ∃r.B`, `a : A` — the classic shape a query-independent, whole-vocabulary
    /// augmentation cannot answer correctly for a non-distinguished variable, because no
    /// NAMED individual need be `r`-related to anything: the axiom only entails that SOME
    /// element is.
    fn some_values_from_ontology() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE);
        let class = b.intern_iri(OWL_CLASS);
        let subclass_of = b.intern_iri(RDFS_SUBCLASS);
        let a = b.intern_iri(&format!("{COMBINED_NS}A"));
        let big_b = b.intern_iri(&format!("{COMBINED_NS}B"));
        let r = b.intern_iri(&format!("{COMBINED_NS}r"));
        let little_a = b.intern_iri(&format!("{COMBINED_NS}a"));
        let restriction = b.intern_blank("restriction", BlankScope::DEFAULT);
        let restriction_class = b.intern_iri(OWL_RESTRICTION);
        let on_property = b.intern_iri(OWL_ON_PROPERTY);
        let some_values_from = b.intern_iri(OWL_SOME_VALUES_FROM);
        b.push_quad(a, ty, class, None);
        b.push_quad(big_b, ty, class, None);
        b.push_quad(restriction, ty, restriction_class, None);
        b.push_quad(restriction, on_property, r, None);
        b.push_quad(restriction, some_values_from, big_b, None);
        b.push_quad(a, subclass_of, restriction, None);
        b.push_quad(little_a, ty, a, None);
        b.freeze().expect("freeze")
    }

    /// HALF ONE: `a` IS a certain answer of `SELECT ?x WHERE { ?x r ?y . ?y a B }`, even
    /// though no triple — asserted or in the whole-vocabulary augmentation — ever states
    /// that any named individual is `r`-related to anything. Only the combined approach's
    /// restricted-chase witness makes the match possible.
    #[test]
    fn the_combined_approach_finds_the_certain_answer_a_whole_vocabulary_augmentation_misses() {
        let query = format!("SELECT ?x WHERE {{ ?x <{COMBINED_NS}r> ?y . ?y a <{COMBINED_NS}B> }}");
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &some_values_from_ontology(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::OwlDirect,
        )
        .unwrap();
        assert_eq!(report.regime(), Regime::OwlDirect);
        let SparqlResult::Solutions {
            variables, rows, ..
        } = result
        else {
            panic!("expected a solution sequence");
        };
        let x = variables
            .iter()
            .position(|v| v == "x")
            .expect("?x is projected");
        let bindings: Vec<&TermValue> = rows
            .iter()
            .map(|row| row[x].as_ref().expect("?x is bound"))
            .collect();
        assert_eq!(
            bindings,
            vec![&TermValue::iri(format!("{COMBINED_NS}a"))],
            "a is a certain answer: every model has SOME r-successor of a typed B"
        );
    }

    /// HALF TWO: no chase-minted Skolem surrogate ever leaks as a binding for a
    /// DISTINGUISHED variable. `?y` is now the projected variable, and the only "value"
    /// `?y` could take is the witness the restricted chase invented for the existential —
    /// which is not a certain answer (the axiom does not name which element it is), so the
    /// solution set must be EMPTY rather than surfacing the internal witness.
    #[test]
    fn no_chase_witness_leaks_as_a_binding_for_a_distinguished_variable() {
        let query = format!(
            "SELECT ?y WHERE {{ <{COMBINED_NS}a> <{COMBINED_NS}r> ?y . ?y a <{COMBINED_NS}B> }}"
        );
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &some_values_from_ontology(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::OwlDirect,
        )
        .unwrap();
        assert_eq!(report.regime(), Regime::OwlDirect);
        let SparqlResult::Solutions { rows, .. } = result else {
            panic!("expected a solution sequence");
        };
        assert!(
            rows.is_empty(),
            "a chase-minted witness must never bind the distinguished ?y: {rows:?}"
        );
    }

    /// The wiring itself: an ontology outside the combined approach's Horn fragment (here,
    /// `owl:equivalentClass`) still answers through the pre-existing whole-vocabulary
    /// augmentation, unchanged.
    #[test]
    fn an_ontology_outside_the_horn_fragment_still_uses_the_whole_vocabulary_augmentation() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE);
        let class = b.intern_iri(OWL_CLASS);
        let equiv = b.intern_iri("http://www.w3.org/2002/07/owl#equivalentClass");
        let a = b.intern_iri(&format!("{COMBINED_NS}A"));
        let big_b = b.intern_iri(&format!("{COMBINED_NS}B"));
        let little_a = b.intern_iri(&format!("{COMBINED_NS}a"));
        b.push_quad(a, ty, class, None);
        b.push_quad(big_b, ty, class, None);
        b.push_quad(a, equiv, big_b, None);
        b.push_quad(little_a, ty, a, None);
        let dataset = b.freeze().expect("freeze");

        let query = format!("ASK {{ <{COMBINED_NS}a> a <{COMBINED_NS}B> }}");
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::OwlDirect,
        )
        .unwrap();
        assert_eq!(report.regime(), Regime::OwlDirect);
        assert!(matches!(result, SparqlResult::Boolean(true)));
    }

    // ── Filtration: the witness never reaches the caller, and no answer is lost ────────

    const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
    const RDFS_SUBPROPERTY: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";

    /// The `some_values_from_ontology` plus ASSERTED data a witness has nothing to do with:
    /// `c : B` and `a s c`. Without it every query in the corpus below would answer nothing
    /// under both lanes, and a superset property over two empty sets proves nothing.
    fn combined_corpus_ontology() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE);
        let class = b.intern_iri(OWL_CLASS);
        let subclass_of = b.intern_iri(RDFS_SUBCLASS);
        let a = b.intern_iri(&format!("{COMBINED_NS}A"));
        let big_b = b.intern_iri(&format!("{COMBINED_NS}B"));
        let r = b.intern_iri(&format!("{COMBINED_NS}r"));
        let s = b.intern_iri(&format!("{COMBINED_NS}s"));
        let little_a = b.intern_iri(&format!("{COMBINED_NS}a"));
        let little_c = b.intern_iri(&format!("{COMBINED_NS}c"));
        let restriction = b.intern_blank("restriction", BlankScope::DEFAULT);
        let restriction_class = b.intern_iri(OWL_RESTRICTION);
        let on_property = b.intern_iri(OWL_ON_PROPERTY);
        let some_values_from = b.intern_iri(OWL_SOME_VALUES_FROM);
        b.push_quad(a, ty, class, None);
        b.push_quad(big_b, ty, class, None);
        b.push_quad(restriction, ty, restriction_class, None);
        b.push_quad(restriction, on_property, r, None);
        b.push_quad(restriction, some_values_from, big_b, None);
        b.push_quad(a, subclass_of, restriction, None);
        b.push_quad(little_a, ty, a, None);
        b.push_quad(little_c, ty, big_b, None);
        b.push_quad(little_a, s, little_c, None);
        b.freeze().expect("freeze")
    }

    /// Answer `query` over `ds` through the production surface, under `OWL-Direct`.
    fn owl_direct(ds: &Arc<RdfDataset>, query: &str) -> (SparqlResult, ReasoningReport) {
        query_with_entailment(
            &NativeSparqlEngine::new(),
            ds,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::OwlDirect,
        )
        .expect("owl-direct answers")
    }

    /// Answer `query` over the WHOLE-VOCABULARY augmentation alone — the lane the combined
    /// approach replaced, evaluated exactly as this module's fallback arm evaluates it.
    fn augmentation_only(ds: &Arc<RdfDataset>, query: &str) -> SparqlResult {
        let engine = NativeSparqlEngine::new();
        let prepared = engine.prepare_query(query, None).expect("parse");
        let pattern = collect_query_bgp(&prepared.query);
        let (closure, _) =
            purrdf_entail::materialize(ds, Materialization::OwlDirect(&pattern)).expect("augment");
        engine
            .query_prepared(&closure, &prepared, &[])
            .expect("evaluate")
    }

    /// A solution sequence as a SET of `(variable, term)` rows, for comparing two lanes'
    /// answers without depending on row order or bag cardinality.
    fn rows_of(result: &SparqlResult) -> BTreeSet<Vec<String>> {
        rendered_rows(result, false)
    }

    /// [`rows_of`] with every BLANK-node cell rendered as its kind alone.
    ///
    /// A blank-node label is scoped to the result set it appears in — SPARQL guarantees
    /// nothing about it matching the queried graph's label, and this kernel deliberately
    /// re-scopes and re-qualifies a blank's label on every dataset merge hop so that two
    /// same-labelled blanks from different sources stay distinct. The combined lane merges one
    /// hop further than the augmentation-only lane (it adds the chase's witnesses to the
    /// augmentation's output), so the same input blank node comes back under two different
    /// labels. Comparing labels across the two lanes would therefore be asserting something
    /// neither lane promises. What the superset property is about is which SOLUTIONS come
    /// back, so a blank cell compares as a blank cell. The witness assertion in the same test
    /// reads the RAW labels, which is the one place a label does carry meaning — the combined
    /// approach mints those itself.
    fn shapes_of(result: &SparqlResult) -> BTreeSet<Vec<String>> {
        rendered_rows(result, true)
    }

    fn rendered_rows(result: &SparqlResult, anonymize_blanks: bool) -> BTreeSet<Vec<String>> {
        let SparqlResult::Solutions {
            variables, rows, ..
        } = result
        else {
            panic!("expected a solution sequence");
        };
        rows.iter()
            .map(|row| {
                variables
                    .iter()
                    .zip(row.iter())
                    .map(|(name, cell)| match cell {
                        Some(TermValue::Blank { .. }) if anonymize_blanks => {
                            format!("{name}=blank")
                        }
                        _ => format!("{name}={cell:?}"),
                    })
                    .collect()
            })
            .collect()
    }

    /// THE ROW SURVIVES, AND `?y` IS UNBOUND.
    ///
    /// `ex:a a ex:A` is ASSERTED data, so `ex:a` is an answer of the left operand under any
    /// reading whatsoever. The previous filter dropped the whole row because the `OPTIONAL`
    /// had matched a chase witness for `?y`, and returned zero rows over an ontology that
    /// states the answer outright. Restricting the BINDING instead lets SPARQL's own
    /// left-join do what it is for: the right operand is empty, so the row comes back with
    /// `?y` outside the solution's domain.
    #[test]
    fn an_optional_that_only_a_witness_could_satisfy_leaves_the_row_with_the_variable_unbound() {
        let query = format!(
            "SELECT ?x ?y WHERE {{ ?x a <{COMBINED_NS}A> . \
             OPTIONAL {{ ?x <{COMBINED_NS}r> ?y . ?y a <{COMBINED_NS}B> }} }}"
        );
        let ds = some_values_from_ontology();
        let (result, _) = owl_direct(&ds, &query);
        let SparqlResult::Solutions {
            variables, rows, ..
        } = &result
        else {
            panic!("expected a solution sequence");
        };
        assert_eq!(variables, &["x".to_owned(), "y".to_owned()]);
        assert_eq!(rows.len(), 1, "the row must survive: {rows:?}");
        let x = rows[0][0].as_ref().expect("?x is bound");
        assert_eq!(x, &TermValue::iri(format!("{COMBINED_NS}a")));
        assert!(
            rows[0][1].is_none(),
            "?y must be UNBOUND, not a witness and not a dropped row: {:?}",
            rows[0][1]
        );
        // And it is the same row the augmentation-only lane produces, which is the check that
        // the reading is SPARQL's and not this module's invention.
        assert_eq!(rows_of(&result), rows_of(&augmentation_only(&ds, &query)));
    }

    /// THE COMBINED APPROACH NEVER LOSES AN ANSWER THE AUGMENTATION ALREADY FINDS.
    ///
    /// Over a corpus of query shapes, the combined lane's answers are a SUPERSET of the
    /// whole-vocabulary augmentation's. That is the property the row-dropping filter violated
    /// — it could only ever remove rows — and it is asserted here as a property rather than as
    /// one example, together with its two non-vacuity conditions: some query where the
    /// augmentation answers something at all, and some query where the combined lane answers
    /// STRICTLY more.
    #[test]
    fn combined_answers_are_a_superset_of_the_augmentations() {
        let ds = combined_corpus_ontology();
        let a = format!("{COMBINED_NS}A");
        let big_b = format!("{COMBINED_NS}B");
        let r = format!("{COMBINED_NS}r");
        let s = format!("{COMBINED_NS}s");
        let little_a = format!("{COMBINED_NS}a");
        let corpus = vec![
            format!("SELECT ?x WHERE {{ ?x a <{a}> }}"),
            format!("SELECT ?y WHERE {{ ?y a <{big_b}> }}"),
            format!("SELECT ?x ?y WHERE {{ ?x <{s}> ?y }}"),
            format!("SELECT ?x WHERE {{ ?x <{r}> ?y . ?y a <{big_b}> }}"),
            format!("SELECT ?y WHERE {{ ?x <{r}> ?y . ?y a <{big_b}> }}"),
            format!("SELECT ?x ?y WHERE {{ ?x <{r}> ?y . ?y a <{big_b}> }}"),
            format!(
                "SELECT ?x ?y WHERE {{ ?x a <{a}> . OPTIONAL {{ ?x <{r}> ?y . ?y a <{big_b}> }} }}"
            ),
            format!("SELECT DISTINCT ?x WHERE {{ ?x a ?t }}"),
            format!("SELECT ?x WHERE {{ ?x a <{a}> }} ORDER BY ?x"),
            format!("SELECT ?x WHERE {{ ?x a <{a}> . FILTER(?x = <{little_a}>) }}"),
            format!("SELECT ?x ?y WHERE {{ {{ ?x a <{a}> }} UNION {{ ?y a <{big_b}> }} }}"),
            format!("SELECT ?x WHERE {{ ?x <{s}> ?z . OPTIONAL {{ ?x <{r}> ?y }} }}"),
        ];
        let mut some_augmentation_answer = false;
        let mut some_strict_superset = false;
        for query in &corpus {
            let (result, report) = owl_direct(&ds, query);
            assert_eq!(report.regime(), Regime::OwlDirect);
            let combined = shapes_of(&result);
            let fallback = shapes_of(&augmentation_only(&ds, query));
            assert!(
                fallback.is_subset(&combined),
                "the combined lane LOST an answer the augmentation finds\n  query: {query}\n  \
                 augmentation: {fallback:?}\n  combined: {combined:?}"
            );
            // And no answer it does return mentions the internal witness label.
            for row in &rows_of(&result) {
                for cell in row {
                    assert!(
                        !cell.contains("purrdfCombinedWitness"),
                        "a witness label reached an answer of {query}: {cell}"
                    );
                }
            }
            some_augmentation_answer |= !fallback.is_empty();
            some_strict_superset |= combined.len() > fallback.len();
        }
        assert!(
            some_augmentation_answer,
            "the corpus is vacuous: the augmentation answered nothing anywhere"
        );
        assert!(
            some_strict_superset,
            "the corpus never exercises the combined approach's own answer"
        );
    }

    /// A `CONSTRUCT` TEMPLATE NEVER EMITS A WITNESS LABEL.
    ///
    /// `CONSTRUCT { ?x ex:saw ?y } WHERE { ?x r ?y . ?y a B }` used to emit a triple whose
    /// object was the internal `_:purrdfCombinedWitness…` label — the row filter was a
    /// documented no-op for a graph result. `?y` is a template variable, so it is observable,
    /// so the restriction empties the sequence and the template is never handed the term.
    #[test]
    fn a_construct_template_never_emits_a_witness_label() {
        let query = format!(
            "CONSTRUCT {{ ?x <{COMBINED_NS}saw> ?y }} \
             WHERE {{ ?x <{COMBINED_NS}r> ?y . ?y a <{COMBINED_NS}B> }}"
        );
        let (result, _) = owl_direct(&some_values_from_ontology(), &query);
        let SparqlResult::Graph(graph) = result else {
            panic!("expected a graph result");
        };
        for quad in graph.owned_quads() {
            let rendered = format!("{quad:?}");
            assert!(
                !rendered.contains("purrdfCombinedWitness"),
                "a witness reached the constructed graph: {rendered}"
            );
        }
        assert_eq!(
            graph.quad_count(),
            0,
            "no certain answer binds the template's ?y, so the graph is empty"
        );
    }

    /// A `DESCRIBE` GRAPH IS SCRUBBED TOO, and this is why the scrub exists: no variable of
    /// `DESCRIBE <a>` names the witness, so nothing the solution-sequence restriction can do
    /// keeps the witness-bearing triple `a r _:w` out of the described graph.
    #[test]
    fn a_describe_graph_carries_no_witness_triple() {
        let query = format!("DESCRIBE <{COMBINED_NS}a>");
        let (result, _) = owl_direct(&some_values_from_ontology(), &query);
        let SparqlResult::Graph(graph) = result else {
            panic!("expected a graph result");
        };
        let rendered: Vec<String> = graph.owned_quads().map(|q| format!("{q:?}")).collect();
        for quad in &rendered {
            assert!(
                !quad.contains("purrdfCombinedWitness"),
                "a witness reached the described graph: {quad}"
            );
        }
        // Non-vacuity in both directions: the description is NOT empty, and the one triple the
        // scrub had to remove — the chase's `a r <witness>` — is the one that is gone. The
        // dataset this query ran over does hold it (the certain-answer test above matches on
        // it), so its absence here is the scrub's work and not the dataset's.
        assert!(
            rendered
                .iter()
                .any(|quad| quad.contains(&format!("{COMBINED_NS}A"))),
            "the description must still carry the asserted type: {rendered:?}"
        );
        assert!(
            !rendered
                .iter()
                .any(|quad| quad.contains(&format!("{COMBINED_NS}r"))),
            "the witness-bearing role assertion must be gone: {rendered:?}"
        );
    }

    /// `COUNT` COUNTS THE RESTRICTED SEQUENCE.
    ///
    /// The aggregate is computed inside the engine, so a filter over the RETURNED rows could
    /// never reach it — `SELECT (COUNT(?y) AS ?n)` had already counted the witnesses by the
    /// time the single aggregate row arrived. Restricting the binding before evaluation is
    /// what puts the aggregate on the right side of the filter.
    #[test]
    fn count_does_not_count_witnesses() {
        let ds = some_values_from_ontology();
        let counted = format!(
            "SELECT (COUNT(?y) AS ?n) WHERE {{ ?x <{COMBINED_NS}r> ?y . ?y a <{COMBINED_NS}B> }}"
        );
        let (result, _) = owl_direct(&ds, &counted);
        let SparqlResult::Solutions { rows, .. } = &result else {
            panic!("expected a solution sequence");
        };
        let n = rows[0][0].as_ref().expect("?n is bound");
        assert_eq!(
            n,
            &TermValue::typed_literal("0", "http://www.w3.org/2001/XMLSchema#integer"),
            "a chase witness is not a certain answer and must not be counted"
        );
        // `COUNT(*)` counts ROWS rather than a named variable, and the rows are the ones a
        // witness would have supplied — so it is zero for the same reason.
        let starred = format!(
            "SELECT (COUNT(*) AS ?n) WHERE {{ ?x <{COMBINED_NS}r> ?y . ?y a <{COMBINED_NS}B> }}"
        );
        let (result, _) = owl_direct(&ds, &starred);
        let SparqlResult::Solutions { rows, .. } = &result else {
            panic!("expected a solution sequence");
        };
        assert_eq!(
            rows[0][0].as_ref().expect("?n is bound"),
            &TermValue::typed_literal("0", "http://www.w3.org/2001/XMLSchema#integer")
        );
    }

    /// An `ASK` still answers TRUE through a witness: it exposes no term, and the boolean is
    /// exactly the entailment the witness is evidence for.
    #[test]
    fn an_ask_is_answered_true_by_a_witness() {
        let query = format!("ASK {{ ?x <{COMBINED_NS}r> ?y . ?y a <{COMBINED_NS}B> }}");
        let (result, _) = owl_direct(&some_values_from_ontology(), &query);
        assert!(matches!(result, SparqlResult::Boolean(true)));
    }

    // ── The `non-horn-tbox` boundary is DISCLOSED, on the surface a caller reads ───────

    /// THE BOUNDARY LINE RENDERS. An ontology outside the Horn fragment is answered by the
    /// fallback, and the report the caller receives SAYS SO — through the same renderer the
    /// CLI and the three host bindings emit.
    ///
    /// Every fallback run used to report `boundaries: []` while three prose sites promised
    /// this disclosure, because nothing anywhere constructed the variant.
    #[test]
    fn a_disqualified_ontology_reports_the_non_horn_tbox_boundary() {
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri(RDF_TYPE);
        let class = b.intern_iri(OWL_CLASS);
        let equiv = b.intern_iri(OWL_EQUIVALENT_CLASS);
        let a = b.intern_iri(&format!("{COMBINED_NS}A"));
        let big_b = b.intern_iri(&format!("{COMBINED_NS}B"));
        let little_a = b.intern_iri(&format!("{COMBINED_NS}a"));
        b.push_quad(a, ty, class, None);
        b.push_quad(big_b, ty, class, None);
        b.push_quad(a, equiv, big_b, None);
        b.push_quad(little_a, ty, a, None);
        let ds = b.freeze().expect("freeze");

        let query = format!("ASK {{ <{COMBINED_NS}a> a <{COMBINED_NS}B> }}");
        let (result, report) = owl_direct(&ds, &query);
        // The fallback still ANSWERS — the boundary is a disclosure, not a refusal.
        assert!(matches!(result, SparqlResult::Boolean(true)));
        assert!(
            report
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == Construct::NonHornTBox),
            "{:?}",
            report.boundaries()
        );
        assert_eq!(
            report.completeness(),
            purrdf_entail::Completeness::ExactWithinBoundaries
        );
        let rendered = purrdf_validate::regime::render_reasoning_report(&report);
        assert!(
            rendered.contains("\nboundary non-horn-tbox "),
            "the boundary must be a LINE the operator reads:\n{rendered}"
        );

        // And a run that stayed in the fragment does NOT name it.
        let (_, in_fragment) = owl_direct(&some_values_from_ontology(), &query);
        assert!(
            !in_fragment
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == Construct::NonHornTBox),
            "{:?}",
            in_fragment.boundaries()
        );
    }

    /// `rdfs:subPropertyOf` — THE axiom the old blacklist ignored — now disqualifies, and the
    /// certain answer it licenses arrives through the fallback's augmentation.
    ///
    /// Before the whitelist this ontology was declared applicable: the lowering emitted no
    /// clause for the sub-property axiom, the chase derived no `q`-edge, `ex:a` was NOT
    /// returned, and no boundary said anything had been skipped.
    #[test]
    fn the_sub_property_certain_answer_arrives_through_the_fallback() {
        let mut b = RdfDatasetBuilder::new();
        let r = b.intern_iri(&format!("{COMBINED_NS}r"));
        let q = b.intern_iri(&format!("{COMBINED_NS}q"));
        let sub_property = b.intern_iri(RDFS_SUBPROPERTY);
        let little_a = b.intern_iri(&format!("{COMBINED_NS}a"));
        let little_b = b.intern_iri(&format!("{COMBINED_NS}b"));
        b.push_quad(r, sub_property, q, None);
        b.push_quad(little_a, r, little_b, None);
        let ds = b.freeze().expect("freeze");

        let query = format!("SELECT ?x WHERE {{ ?x <{COMBINED_NS}q> ?y }}");
        let (result, report) = owl_direct(&ds, &query);
        assert!(
            report
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == Construct::NonHornTBox),
            "the sub-property axiom must disqualify the combined approach: {:?}",
            report.boundaries()
        );
        let rows = rows_of(&result);
        assert!(
            rows.iter().any(|row| row
                .iter()
                .any(|cell| cell.contains(&format!("{COMBINED_NS}a")))),
            "ex:a is a certain answer through q and must arrive via the fallback: {rows:?}"
        );
    }
}
