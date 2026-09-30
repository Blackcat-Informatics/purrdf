// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Entailment-aware SPARQL orchestration over the native PurRDF engines.

use purrdf_sparql_algebra::Child;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use purrdf_datalog::seminaive::{BudgetReport, EvalOptions};
use purrdf_entail::entails::imports::imported_iris;
use purrdf_entail::{
    Construct, EntailError, ImportMap, Materialization, QNode, QTriple, ReasoningReport, Regime,
    RuleSet, materialize_combined_until,
};
use purrdf_rdf::{
    DatasetView, RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfQuad, RdfTerm, SparqlRequest,
    SparqlResult, dataset_from_view,
};
use purrdf_sparql_algebra::{
    BlankNode, Expression, GraphPattern, GroundTerm, NamedNodePattern, OrderExpression,
    PropertyFunctionCall, Query, TermPattern, TriplePattern, Variable,
};
use purrdf_sparql_eval::convert;
use purrdf_sparql_eval::{
    BudgetExhausted, EvalError, GovernedOutcome, NativeSparqlEngine, PreparedQuery,
    PropertyFunctionRegistry, QueryGovernors, QueryOptions, StopCause, StopSignal, TrippedGovernor,
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
/// let mut session = ReasonerSession::open(data, 0, 0).expect("parses");
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

impl<'a> QueryEntailment<'a> {
    /// The query plan for `regime`, with `rules` as the RIF rule set.
    ///
    /// The one mapping from a resolved [`Regime`] to its query plan, total over the seven
    /// regimes: every host that resolves a regime (the owned [`QueryEntailmentPlan`], a
    /// command line) borrows its plan through here, so no two hosts can map one regime to
    /// two plans. `rules` is read only for [`Regime::Rif`], the one regime whose calculus
    /// is the caller's rather than a specification's.
    #[must_use]
    pub const fn for_regime(regime: Regime, rules: &'a RuleSet) -> Self {
        match regime {
            Regime::Simple => Self::Simple,
            Regime::Rdf => Self::Rdf,
            Regime::Rdfs => Self::Rdfs,
            Regime::OwlRl => Self::OwlRl,
            Regime::D => Self::D,
            Regime::OwlDirect => Self::OwlDirect,
            Regime::Rif => Self::Rif(rules),
        }
    }
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
        QueryEntailment::for_regime(self.regime, &self.rules)
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

/// How a caller's **dataset-derived** property-function relations are re-derived over the
/// closure an entailment-regime query is actually answered against.
///
/// # The problem this parameter exists to solve
///
/// A [`QueryOptions::property_functions`] registry is built by the caller, before the call.
/// For an ordinary query that is unremarkable: the caller holds the dataset the query will
/// run over, so a relation snapshotted from it answers about the same edges every other
/// pattern in the query reads. An entailment-regime query breaks that identity. The dataset
/// the query is evaluated over is the CLOSURE, which this function materializes internally
/// and the caller never holds — so a relation the caller snapshotted answers about the
/// PRE-closure data while the rest of the query reads the closure. The two halves of one
/// query then read two different datasets.
///
/// That is not a theoretical divergence. Over
/// `ex:sub rdfs:subPropertyOf ex:p . ex:a ex:p ex:b . ex:b ex:sub ex:c .`, under
/// [`QueryEntailment::Rdfs`], `SELECT ?end WHERE { ex:a ex:p+ ?end }` answers `ex:b, ex:c`
/// (the closure derives `ex:b ex:p ex:c`) while a
/// [`PathWitnessRelation`](purrdf_sparql_eval::PathWitnessRelation) over the same step
/// answers `ex:b` alone — a SHORT bag, returned complete, with no diagnostic. The
/// property-function seam hands a relation no dataset at evaluation time, so nothing
/// downstream can notice.
///
/// Supplying a rebuilder here is what closes it: the closure is materialized first, the
/// relations are derived from THAT dataset, and the query is then prepared and evaluated
/// against a registry that answers about the edges the rest of the query sees.
///
/// # [`NONE`](Self::NONE) is not a weaker setting; it is a different claim
///
/// Not every relation is derived from a dataset. A lookup table registered from host
/// memory answers identically no matter what the query runs over, and re-deriving it would
/// be meaningless work. [`NONE`] states exactly that — "every relation in `options` is
/// dataset-independent" — and is the correct value for such a registry, for an empty one,
/// and for [`QueryEntailment::Simple`], whose closure is the source dataset.
///
/// [`NONE`]: Self::NONE
pub struct ClosureRelations<'a> {
    /// Derives the relation registry from the materialized closure, or `None` when the
    /// caller's registry is dataset-independent and needs no re-derivation.
    rebuild: Option<&'a RelationRebuilder<'a>>,
}

/// The host callback [`ClosureRelations::rebuilt_by`] takes: the materialized closure in,
/// the registry to answer it with out.
///
/// Named rather than written inline because it appears in three positions — the field, the
/// constructor's parameter, and the private reader that calls it — and a
/// hand-repeated `dyn Fn` signature is three places for the three to drift apart.
pub type RelationRebuilder<'a> =
    dyn Fn(&RdfDataset) -> Result<PropertyFunctionRegistry, EvalError> + 'a;

impl std::fmt::Debug for ClosureRelations<'_> {
    /// Which of the two shapes this value has. The rebuilder itself is host code with no
    /// rendering of its own, so naming it would print a pointer nobody can act on.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosureRelations")
            .field("rebuild", &self.rebuild.map(|_| "<host rebuilder>"))
            .finish()
    }
}

impl<'a> ClosureRelations<'a> {
    /// Every relation in the caller's registry is dataset-independent, so the registry
    /// passed in `options` is the one the closure is queried with, unchanged.
    pub const NONE: Self = Self { rebuild: None };

    /// Re-derive the relation registry from the materialized closure with `build`.
    ///
    /// `build` is handed the exact dataset the SPARQL evaluation will run over, and the
    /// registry it returns replaces [`QueryOptions::property_functions`] for both the
    /// prepare and the evaluation of the closure's query — both, because a plan carries
    /// the identity of the registry it was prepared against and refuses to run against
    /// any other (see `purrdf_sparql_eval`'s `RegistryId`).
    #[must_use]
    pub const fn rebuilt_by(build: &'a RelationRebuilder<'a>) -> Self {
        Self {
            rebuild: Some(build),
        }
    }

    /// The [`RdfDiagnostic::code`] of the one combination this parameter refuses: a
    /// rebuilder supplied for an OWL Direct-Semantics run whose restricted chase minted
    /// existential witnesses.
    ///
    /// Owned here, and named here, so a host that wants to classify the refusal — the CLI
    /// raises it as a USAGE error rather than a runtime one, because the operator combined
    /// two flags — compares against this constant instead of retyping the string.
    pub const WITNESS_REFUSAL_CODE: &'static str = "reasoning-closure-relation-witness";

    /// The [`RdfDiagnostic::code`] carrying a rebuilder's OWN failure — the host's
    /// snapshot of the closure did not build. See [`Self::WITNESS_REFUSAL_CODE`] for why
    /// the string lives here.
    pub const REBUILD_FAILURE_CODE: &'static str = "reasoning-closure-relation-rebuild";
}

/// Re-derive `relations` over the materialized `closure`, or `Ok(None)` when the caller
/// declared its registry dataset-independent.
///
/// # Why a chase witness is refused here rather than filtered later
///
/// `surrogates` is the set of blank labels the OWL-Direct combined approach's restricted
/// chase minted as existential witnesses. Those terms are legitimate bindings for a
/// variable the answer never exposes and illegitimate for one it does, and
/// [`restrict_witness_bindings`] enforces that — but it enforces it by wrapping the
/// pattern leaves that READ the entailed graph, and it deliberately treats a
/// property-function call as the identity, on the stated grounds that nothing a call emits
/// can be a witness because its rows come from the host's registry rather than from the
/// closure.
///
/// Re-deriving a relation FROM the closure is precisely the change that would make that
/// sentence false: a walk over the chase's edges can reach a minted witness and hand it
/// back as an observable binding, past every filter built to stop it. So the combination
/// is refused, and refused on the narrowest condition that can carry the leak — a
/// rebuilder supplied AND witnesses actually minted. A non-Horn TBox mints none and takes
/// the whole-vocabulary augmentation instead; a Horn one whose chase fired no existential
/// rule mints none either. Both keep answering, with their relations derived over the
/// closure like every other regime's.
fn relations_over_closure(
    relations: &ClosureRelations<'_>,
    closure: &RdfDataset,
    surrogates: &BTreeSet<String>,
) -> Result<Option<PropertyFunctionRegistry>, ReasoningError> {
    let Some(build) = relations.rebuild else {
        return Ok(None);
    };
    if !surrogates.is_empty() {
        return Err(ReasoningError::Query(
            RdfDiagnostic::error(
                ClosureRelations::WITNESS_REFUSAL_CODE,
                "a property-function registry derived from the closure cannot be combined \
                 with an OWL Direct-Semantics run whose restricted chase minted existential \
                 witnesses: a relation walking the closure could return a minted blank node \
                 as an observable binding, which the entailment regime's scoping graph does \
                 not contain",
            )
            .with_detail(format!(
                "{} witness term(s) were minted. Query this ontology under an entailment \
                 regime that mints none (rdf, rdfs, owl-rl, d, rif, simple), or drop the \
                 dataset-derived relations from this call",
                surrogates.len()
            )),
        ));
    }
    build(closure).map(Some).map_err(|error| {
        ReasoningError::Query(RdfDiagnostic::error(
            ClosureRelations::REBUILD_FAILURE_CODE,
            format!("deriving the property-function relations over the closure failed: {error}"),
        ))
    })
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
/// # `options` reaches the closure's query exactly as it reaches any other
///
/// `options` is the SAME [`QueryOptions`] every other query entry point takes: the
/// SHACL-AF/native function registry, the property-function registry, and the
/// custom-aggregate registry are all *parse* configuration as much as evaluation
/// configuration (a registered relation's predicate becomes a call node only if the
/// registry that will evaluate it was in scope when the query was parsed), so `options` is
/// threaded into the parse of the QUERY ([`NativeSparqlEngine::prepare_query_with_options`])
/// and into evaluating it over the materialized closure
/// ([`NativeSparqlEngine::query_prepared`]) — never into materializing the closure itself,
/// which reads no registry of any kind. Pass [`QueryOptions::EMPTY`] to configure none,
/// which is the behaviour this function had before it took the parameter.
///
/// # `relations` is how a DATASET-DERIVED relation reaches the closure
///
/// `options.property_functions()` is built before this call and therefore before the closure
/// exists, so a relation snapshotted from the caller's dataset answers about the
/// PRE-closure edges while every other pattern in the query reads the closure.
/// [`ClosureRelations`] is the parameter that fixes the order — see its documentation for
/// the worked divergence and for when [`ClosureRelations::NONE`] is the right value.
///
/// # Errors
///
/// Returns [`ReasoningError::Query`] for SPARQL failures and
/// [`ReasoningError::Entailment`] for malformed or inconsistent knowledge bases — and for a
/// dataset whose `owl:imports` names a document it does not already hold
/// ([`EntailError::UnresolvedImport`]): this entry point takes no import table, so such a
/// premise is refused rather than closed without its imports.
/// [`query_with_entailment_closure_governed`] takes the table.
pub fn query_with_entailment<D: DatasetView>(
    engine: &NativeSparqlEngine,
    dataset: &D,
    request: SparqlRequest<'_>,
    entailment: QueryEntailment<'_>,
    options: QueryOptions<'_>,
    relations: &ClosureRelations<'_>,
) -> Result<(SparqlResult, ReasoningReport), ReasoningError> {
    // Parse first so invalid queries fail before potentially expensive closure work.
    // OWL Direct also inspects this same cached plan, avoiding a second parse/cache lookup.
    // Registry-aware: `options` decides which predicates in the QUERY become call nodes
    // and which `Custom` aggregate IRIs are admitted, exactly as every other prepared-plan
    // entry point does.
    let prepared_query =
        engine.prepare_query_with_options(request.query, request.base_iri, options)?;
    // Every lane hands back a `ReasoningReport` alongside the closure, and every one of
    // them is carried out of this function rather than dropped at this call site.
    // `collect_query_bgp` is bound outside the match because the OWL-Direct plan BORROWS
    // it; it is computed for that mode alone.
    let pattern = match entailment {
        QueryEntailment::OwlDirect => query_bgp(prepared_query.query()),
        _ => Vec::new(),
    };
    // `surrogates` is populated only when the OWL-Direct lane answered through the COMBINED
    // APPROACH (`purrdf_entail::materialize_combined`) rather than the whole-vocabulary
    // augmentation: the set of blank terms its restricted chase minted as existential
    // witnesses. A witness is not a certain answer for a variable whose binding the caller
    // can OBSERVE — the regime draws its answers from the scoping graph, and a minted
    // witness is not in it — so `restrict_witness_bindings` below forbids exactly those
    // bindings, at the point the binding is made rather than after the fact.
    //
    // The import table is EMPTY here: this entry point takes none, so a dataset that
    // imports a document it does not already hold is refused by name rather than closed as a
    // smaller premise. `query_with_entailment_closure_governed` is the one that takes one.
    let imports = ImportMap::new();
    let Closed {
        dataset: prepared,
        report,
        surrogates,
    } = close_premise(
        dataset,
        &EntailmentClosure::new(entailment, &imports),
        &pattern,
        None,
    )?;
    // The combined approach's filtration, in the only two places a witness can escape: the
    // solution sequence (forbidden BEFORE evaluation, so the algebra above the restriction
    // sees the filtered sequence) and a constructed graph (scrubbed after, because a
    // `DESCRIBE` draws triples from the dataset rather than from a variable binding).
    let surrogates = surrogates.unwrap_or_default();
    // Materialize, THEN register: a dataset-derived relation is re-derived over the closure
    // that is about to be queried, so the walk and the surrounding patterns read one
    // dataset rather than two. The re-parse is what makes the swap legal as well as
    // correct — a plan carries the identity of the registry it was prepared against, and a
    // registry built here is a different instance than the one `options` arrived with.
    let rebound = relations_over_closure(relations, &prepared, &surrogates)?;
    // The swap replaces the RELATION table only. Everything else the caller
    // configured — its declared parser namespaces, its aggregate registry — is
    // carried through, because an environment that had forgotten a declared
    // namespace would read a prefixed relation IRI as an ordinary data triple.
    let rebound_env = match rebound.as_ref() {
        Some(registry) => Some(options.env.with_relations(registry.clone()).map_err(|e| {
            ReasoningError::Query(RdfDiagnostic::error(
                "native-sparql-property-function",
                e.to_string(),
            ))
        })?),
        None => None,
    };
    let options = match rebound_env.as_ref() {
        Some(env) => options.with_env(env),
        None => options,
    };
    let prepared_query = if rebound.is_some() {
        engine.prepare_query_with_options(request.query, request.base_iri, options)?
    } else {
        prepared_query
    };
    // The rewrite is tagged with the SAME `options` the original plan was prepared under —
    // required by `PreparedQuery::rewritten`'s own contract, and necessary here: the
    // rewritten plan is evaluated under `options` below, and a mismatched tag would refuse
    // it (`check_plan_matches_relations`) rather than silently drop a registry.
    let restricted = (!surrogates.is_empty())
        .then(|| {
            PreparedQuery::rewritten(
                restrict_witness_bindings(prepared_query.query(), &surrogates),
                options,
            )
        })
        .transpose()?;
    let plan = restricted.as_ref().unwrap_or(&prepared_query);
    let mut result = engine.query_prepared(&prepared, plan, request.substitutions, options)?;
    let _ = withhold_surrogate_triples(&mut result, &surrogates);
    Ok((result, report))
}

/// What one entailment-regime query closes its premise UNDER: the regime, and the
/// premise's `owl:imports` table.
///
/// The configuration [`query_with_entailment_closure_governed`] takes. A premise that
/// carries an `owl:imports` states that its axioms are its own PLUS those of the documents
/// it names, so a query answered under a regime is answered over that merge; the table is
/// where those documents arrive, resolved through [`purrdf_entail::resolve_imports`]. PurRDF
/// fetches nothing, so the table is caller-supplied configuration and an import it does not
/// resolve is refused by name. [`ImportMap::declare_loaded`] names the IRIs the premise
/// itself was read under, so an import of one of them resolves in place.
///
/// It also carries the closure's EVALUATION LIMITS ([`Self::with_limits`]): the stored-fact
/// and join-step limits the four rule-table regimes (`Rdf`, `Rdfs`, `OwlRl`, `D`) are
/// materialized under, exactly as [`purrdf_entail::materialize_with`] takes them. Unstated,
/// they are the target's defaults. A limit can only refuse — a run past one fails with
/// [`EntailError::Evaluate`] or [`EntailError::Chase`] naming it — and never truncates a
/// closure, so the query governors, which price evaluation over the closure, never become
/// one.
#[derive(Debug, Clone, Copy)]
pub struct EntailmentClosure<'a> {
    /// The regime the closure is materialized under.
    entailment: QueryEntailment<'a>,
    /// The premise's `owl:imports` table.
    imports: &'a ImportMap,
    /// The evaluation limits the closure is materialized under.
    limits: EvalOptions,
}

impl<'a> EntailmentClosure<'a> {
    /// Close under `entailment`, over the premise's imports closure as `imports` resolves it,
    /// under the target's default evaluation limits. An empty map is the ordinary "imports
    /// nothing" case.
    #[must_use]
    pub fn new(entailment: QueryEntailment<'a>, imports: &'a ImportMap) -> Self {
        Self {
            entailment,
            imports,
            limits: EvalOptions::default(),
        }
    }

    /// The same closure, materialized under `limits`' stored-fact and join-step limits.
    #[must_use]
    pub const fn with_limits(mut self, limits: EvalOptions) -> Self {
        self.limits = limits;
        self
    }

    /// The evaluation limits the closure is materialized under.
    #[must_use]
    pub const fn limits(&self) -> &EvalOptions {
        &self.limits
    }

    /// The regime the closure is materialized under.
    #[must_use]
    pub const fn entailment(&self) -> QueryEntailment<'a> {
        self.entailment
    }

    /// The premise's `owl:imports` table.
    #[must_use]
    pub const fn imports(&self) -> &'a ImportMap {
        self.imports
    }
}

/// One materialized closure, ready to be queried.
struct Closed {
    /// The closure itself.
    dataset: Arc<RdfDataset>,
    /// The run that produced it.
    report: ReasoningReport,
    /// The existential witnesses the OWL-Direct combined approach minted, when that lane
    /// answered; `None` for every other lane.
    surrogates: Option<BTreeSet<String>>,
}

/// Close `dataset` under `closure`, over its `owl:imports` closure — the ONE place both query
/// entry points materialize, so the governed and the ungoverned lane cannot resolve imports
/// differently.
///
/// A dataset that imports nothing, with an empty table, is closed over the view directly.
/// Otherwise the view becomes an owned dataset, the closure is resolved
/// ([`purrdf_entail::resolve_imports`], which refuses an unresolved import and an unreached
/// entry) and the merge is closed; the report — and an inconsistent run's refusal — is then
/// restated `ontology-import-resolved`.
fn close_premise<D: DatasetView>(
    dataset: &D,
    closure: &EntailmentClosure<'_>,
    pattern: &[QTriple],
    stop: Option<&Arc<dyn purrdf_datalog::StopSignal>>,
) -> Result<Closed, ReasoningError> {
    let imports = closure.imports;
    let loaded: Vec<&str> = imports.loaded().collect();
    if imports.is_empty() && imported_iris(dataset, &loaded).is_empty() {
        return close_lane(dataset, closure.entailment, pattern, &closure.limits, stop);
    }
    let premise = dataset_from_view(dataset)?;
    let merged = purrdf_entail::resolve_imports(&premise, imports)?;
    let resolved = merged.is_some() || !imports.imported_iris(&premise).is_empty();
    let run = close_lane(
        merged.as_deref().unwrap_or(&premise),
        closure.entailment,
        pattern,
        &closure.limits,
        stop,
    );
    if !resolved {
        return run;
    }
    match run {
        Ok(closed) => Ok(Closed {
            report: closed.report.with_resolved_imports(),
            ..closed
        }),
        Err(ReasoningError::Entailment(error)) => {
            Err(ReasoningError::Entailment(error.with_resolved_imports()))
        }
        Err(other) => Err(other),
    }
}

/// Close `dataset` under one regime — seven modes, one call each.
///
/// `purrdf_entail::materialize_with` is total over `Materialization`, so this does not split
/// into "the regimes that materialize" and "the two that need their own entry point". The
/// OWL-Direct lane answers through the COMBINED APPROACH when the ontology's TBox is in the
/// certified Horn fragment (restricted-chase witnesses for the anonymous part, filtered by the
/// caller) rather than the whole-vocabulary augmentation, which is silently incomplete for a
/// query's non-distinguished variable; outside that fragment it falls back to the augmentation
/// and raises the ONE boundary only this call site can: [`Construct::NonHornTBox`], which says
/// the combined approach was tried first and declined.
fn close_lane<D: DatasetView>(
    dataset: &D,
    entailment: QueryEntailment<'_>,
    pattern: &[QTriple],
    limits: &EvalOptions,
    stop: Option<&Arc<dyn purrdf_datalog::StopSignal>>,
) -> Result<Closed, ReasoningError> {
    let plain = |(dataset, report): (Arc<RdfDataset>, ReasoningReport)| Closed {
        dataset,
        report,
        surrogates: None,
    };
    let closed = match entailment {
        // Simple runs no fixpoint (it cannot be stopped), so it builds its owned closure
        // directly; the reasoning lanes seed from the view with no rebuild.
        QueryEntailment::Simple => plain((dataset_from_view(dataset)?, simple_report())),
        QueryEntailment::Rdf => plain(purrdf_entail::materialize_with(
            dataset,
            Materialization::Rdf,
            limits,
            stop,
        )?),
        QueryEntailment::Rdfs => plain(purrdf_entail::materialize_with(
            dataset,
            Materialization::Rdfs,
            limits,
            stop,
        )?),
        QueryEntailment::OwlRl => plain(purrdf_entail::materialize_with(
            dataset,
            Materialization::OwlRl,
            limits,
            stop,
        )?),
        QueryEntailment::D => plain(purrdf_entail::materialize_with(
            dataset,
            Materialization::D,
            limits,
            stop,
        )?),
        QueryEntailment::OwlDirect => match materialize_combined_until(dataset, pattern, stop)? {
            Some(combined) => Closed {
                dataset: combined.dataset,
                report: combined.report,
                surrogates: Some(combined.surrogates),
            },
            None => {
                let (closure, report) = purrdf_entail::materialize_with(
                    dataset,
                    Materialization::OwlDirect(pattern),
                    limits,
                    stop,
                )?;
                plain((closure, report.with_boundary(Construct::NonHornTBox)))
            }
        },
        QueryEntailment::Rif(ruleset) => plain(purrdf_entail::materialize_with(
            dataset,
            Materialization::Rif(ruleset),
            limits,
            stop,
        )?),
    };
    Ok(closed)
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
/// behaviour exactly (plus `governors`): an ungoverned entailment query is the same call it
/// always was, and `options` reaches the closure's query the same way in both — see
/// [`query_with_entailment`]'s doc comment for what it does and does not reach.
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
/// **Phase one — materializing the closure — honours the STOP SIGNAL, under the
/// closure's own evaluation limits.** The SPARQL governors price query evaluation, not a
/// reasoning run, so none of them is translated into a limit on the closure: the closure is
/// computed under `purrdf-datalog`'s stored-fact and join-step limits
/// ([limits refuse; they never truncate](purrdf_datalog#limits-refuse-they-never-truncate)) —
/// the target's defaults here, and the caller's through
/// [`query_with_entailment_closure_governed`] and [`EntailmentClosure::with_limits`]. The stop
/// signal either lets the closure finish, in which case it is bit-for-bit the closure
/// [`query_with_entailment`] would have computed, or it ends the run with
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
/// [`GovernedEntailment`]. This entry point takes no import table, so a dataset whose
/// `owl:imports` names a document it does not already hold is refused
/// ([`EntailError::UnresolvedImport`]) rather than closed without it;
/// [`query_with_entailment_closure_governed`] takes the table.
///
/// # `relations` is how a DATASET-DERIVED relation reaches the closure
///
/// `options.property_functions()` is built before this call and therefore before the closure
/// exists, so a relation snapshotted from the caller's dataset answers about the
/// PRE-closure edges while every other pattern in the query reads the closure.
/// [`ClosureRelations`] is the parameter that fixes the order — see its documentation for
/// the worked divergence and for when [`ClosureRelations::NONE`] is the right value. It is
/// what the `purrdf query --path-relation --entailment …` and Python
/// `path_relations=`-plus-`entailment=` pairings both supply.
pub fn query_with_entailment_governed<D: DatasetView>(
    engine: &NativeSparqlEngine,
    dataset: &D,
    request: SparqlRequest<'_>,
    entailment: QueryEntailment<'_>,
    options: QueryOptions<'_>,
    relations: &ClosureRelations<'_>,
    governors: &QueryGovernors,
) -> Result<GovernedEntailment, ReasoningError> {
    let imports = ImportMap::new();
    query_with_entailment_closure_governed(
        engine,
        dataset,
        request,
        &EntailmentClosure::new(entailment, &imports),
        options,
        relations,
        governors,
    )
}

/// [`query_with_entailment_governed`] over the premise's `owl:imports` closure: the
/// closure is materialized over the dataset MERGED with every document `closure`'s import
/// table supplies.
///
/// OWL 2 defines an ontology's imports closure to BE the ontology, so a query answered under
/// a regime over a premise that imports a document is answered over the merge — the rule
/// [`purrdf_entail::materialize_with_imports`], [`purrdf_entail::entails`](fn@purrdf_entail::entails) and
/// [`purrdf_entail::certain_answers`] apply. Resolution is
/// [`purrdf_entail::resolve_imports`]: an `owl:imports` the table does not resolve and the
/// dataset does not already hold refuses the call with
/// [`EntailError::UnresolvedImport`], a table entry the closure never reaches with
/// [`EntailError::UnreachedImport`] — never an answer over a smaller premise. When the
/// closure was resolved the report states `ontology-import-resolved`.
///
/// A dataset that imports nothing, with an empty table, is closed over the view directly —
/// a pack is never rebuilt to find that out.
///
/// # Errors
///
/// As [`query_with_entailment_governed`], plus the import refusals above (as
/// [`ReasoningError::Entailment`]).
pub fn query_with_entailment_closure_governed<D: DatasetView>(
    engine: &NativeSparqlEngine,
    dataset: &D,
    request: SparqlRequest<'_>,
    closure: &EntailmentClosure<'_>,
    options: QueryOptions<'_>,
    relations: &ClosureRelations<'_>,
    governors: &QueryGovernors,
) -> Result<GovernedEntailment, ReasoningError> {
    let entailment = closure.entailment;
    // Parse first, exactly as the ungoverned lane does: an invalid query is a failure rather
    // than a budget, and it must be one before any closure work is charged for. Registry-aware
    // exactly as `query_with_entailment`'s parse is.
    let prepared_query =
        engine.prepare_query_with_options(request.query, request.base_iri, options)?;
    let pattern = match entailment {
        QueryEntailment::OwlDirect => query_bgp(prepared_query.query()),
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

    let materialized = close_premise(dataset, closure, &pattern, closure_stop);
    let (prepared, report, surrogates) = match materialized {
        Ok(Closed {
            dataset,
            report,
            surrogates,
        }) => (dataset, report, surrogates),
        // The one refusal that is an OUTCOME rather than a failure. The cause is what the
        // adapter observed when it fired, so the receipt names the caller's own signal.
        Err(ReasoningError::Entailment(EntailError::Stopped)) => {
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
        Err(error) => return Err(error),
    };

    // The combined approach's filtration, in the SAME two places and the same order the
    // ungoverned lane applies it: the restriction is in the algebra (so it is upstream of
    // any truncation), and the scrub is over the result (so it reaches a `DESCRIBE`'s
    // triples). See this function's documentation for why a partial answer needs both.
    let surrogates = surrogates.unwrap_or_default();
    // Materialize, THEN register — the same order, and for the same reason, as the
    // ungoverned lane's. See `relations_over_closure`.
    let rebound = relations_over_closure(relations, &prepared, &surrogates)?;
    // The swap replaces the RELATION table only. Everything else the caller
    // configured — its declared parser namespaces, its aggregate registry — is
    // carried through, because an environment that had forgotten a declared
    // namespace would read a prefixed relation IRI as an ordinary data triple.
    let rebound_env = match rebound.as_ref() {
        Some(registry) => Some(options.env.with_relations(registry.clone()).map_err(|e| {
            ReasoningError::Query(RdfDiagnostic::error(
                "native-sparql-property-function",
                e.to_string(),
            ))
        })?),
        None => None,
    };
    let options = match rebound_env.as_ref() {
        Some(env) => options.with_env(env),
        None => options,
    };
    let prepared_query = if rebound.is_some() {
        engine.prepare_query_with_options(request.query, request.base_iri, options)?
    } else {
        prepared_query
    };
    // Tagged with the SAME `options` the original plan was prepared under — see
    // `query_with_entailment`'s identical rewrite for why a mismatched tag would refuse the
    // plan rather than silently drop a registry.
    let restricted = (!surrogates.is_empty())
        .then(|| {
            PreparedQuery::rewritten(
                restrict_witness_bindings(prepared_query.query(), &surrogates),
                options,
            )
        })
        .transpose()?;
    let plan = restricted.as_ref().unwrap_or(&prepared_query);
    let outcome = engine.query_prepared_governed_view(
        &*prepared,
        plan,
        request.substitutions,
        options,
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
///   number, and `FOLD`'s own `ORDER BY` determines the encoded list value). Aggregates must
///   therefore see the restricted sequence. A `COUNT(*)` reads no variable and still counts
///   ROWS, so it makes every variable of the grouped pattern observable — row multiplicity
///   is a function of all of them.
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
/// filter over a non-distinguished variable means. An outer `ORDER BY` reads one to decide
/// row ORDER; the rows it orders are certain answers either way, and the witness labels are content
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
            for quad in template {
                collect_triple_pattern_variables(&quad.triple, &mut names);
                // A template GRAPH variable is observable: its binding decides
                // which graph the row's statement lands in, so the caller reads
                // the value back off the result dataset's graph name just as
                // directly as off a subject position. Leaving it out would let a
                // witness surrogate leak through the graph slot.
                if let Some(NamedNodePattern::Variable(variable)) = &quad.graph {
                    names.insert(variable.as_str().to_owned());
                }
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
fn find_projection(mut pattern: &GraphPattern) -> Option<BTreeSet<String>> {
    loop {
        match pattern {
            GraphPattern::Project { variables, .. } => {
                return Some(variables.iter().map(|v| v.as_str().to_owned()).collect());
            }
            GraphPattern::Filter { inner, .. }
            | GraphPattern::Graph { inner, .. }
            | GraphPattern::Extend { inner, .. }
            | GraphPattern::Service { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::Group { inner, .. } => pattern = inner,
            _ => return None,
        }
    }
}

/// Every variable an `Extend` expression, a `GROUP BY` key or an aggregate READS — the
/// variables whose bindings become returned VALUES rather than returned bindings.
///
/// `Expression::Exists`'s inner pattern is deliberately not descended into: an `EXISTS`
/// yields a boolean and no binding of its own escapes, so a witness inside one is invisible
/// for the same reason an `ASK`'s is.
fn collect_returned_value_variables(pattern: &GraphPattern, names: &mut BTreeSet<String>) {
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
        match pattern {
            GraphPattern::Extend {
                inner, expression, ..
            } => {
                collect_expression_variables(expression, names);
                pending.push(inner);
            }
            // `UNFOLD` READS its operand exactly as `BIND` does — those bindings
            // become returned VALUES inside a composite element rather than returned
            // bindings — and it BINDS its own two targets, which are fresh names the
            // entailed graph never supplied and so are not witnesses of it.
            GraphPattern::Unfold {
                inner, expression, ..
            } => {
                collect_expression_variables(expression, names);
                pending.push(inner);
            }
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => {
                names.extend(variables.iter().map(|v| v.as_str().to_owned()));
                for (_, aggregate) in aggregates {
                    if aggregate.args().is_empty() {
                        // `COUNT(*)` — the spec's empty exprlist, and (per
                        // `AggregateExpression::new`'s invariant) the ONLY aggregate that
                        // can ever have an empty `args`. It names no variable and returns
                        // row MULTIPLICITY, which every variable of the grouped pattern
                        // contributes to — so all of them are observable through it.
                        collect_all_variables(inner, names);
                    } else {
                        for arg in aggregate.args() {
                            collect_expression_variables(arg, names);
                        }
                    }
                    // Aggregate sort keys determine a returned composite value, so
                    // their witness bindings must be restricted before aggregation.
                    for key in aggregate.order_by() {
                        let (OrderExpression::Asc(expression) | OrderExpression::Desc(expression)) =
                            key;
                        collect_expression_variables(expression, names);
                    }
                }
                pending.push(inner);
            }
            GraphPattern::Join { left, right }
            | GraphPattern::Minus { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::LeftJoin { left, right, .. } => pending.extend([&**left, &**right]),
            GraphPattern::Union { arms } => pending.extend(arms.iter()),
            GraphPattern::Filter { inner, .. }
            | GraphPattern::Graph { inner, .. }
            | GraphPattern::Service { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => pending.push(inner),
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
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match expression {
            Expression::Variable(variable) | Expression::Bound(variable) => {
                names.insert(variable.as_str().to_owned());
            }
            Expression::NamedNode(_) | Expression::Literal(_) | Expression::Exists(_) => {}
            Expression::Or(operands) | Expression::And(operands) => pending.extend(operands.iter()),
            Expression::Arithmetic(first, steps) => {
                pending.push(first);
                pending.extend(steps.iter().map(|(_, operand)| operand));
            }
            Expression::Equal(left, right)
            | Expression::SameTerm(left, right)
            | Expression::Greater(left, right)
            | Expression::GreaterOrEqual(left, right)
            | Expression::Less(left, right)
            | Expression::LessOrEqual(left, right) => pending.extend([&**left, &**right]),
            Expression::UnaryPlus(inner)
            | Expression::UnaryMinus(inner)
            | Expression::Not(inner) => {
                pending.push(inner);
            }
            Expression::In(inner, list) => {
                pending.push(inner);
                pending.extend(list.iter());
            }
            Expression::If(condition, then, otherwise) => {
                pending.extend([&**condition, &**then, &**otherwise]);
            }
            Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
                pending.extend(list.iter());
            }
        }
    }
}

/// Every variable mentioned anywhere in `pattern` — the conservative answer, used where a
/// precise one is unavailable (`COUNT(*)`, or a `SELECT` with no projection to read).
fn collect_all_variables(pattern: &GraphPattern, names: &mut BTreeSet<String>) {
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
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
                    pending.push(inner);
                }
            }
            GraphPattern::Join { left, right }
            | GraphPattern::Minus { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::LeftJoin { left, right, .. } => pending.extend([&**left, &**right]),
            GraphPattern::Union { arms } => pending.extend(arms.iter()),
            GraphPattern::Extend {
                inner, variable, ..
            } => {
                names.insert(variable.as_str().to_owned());
                pending.push(inner);
            }
            GraphPattern::Unfold {
                inner,
                element,
                companion,
                ..
            } => {
                names.insert(element.as_str().to_owned());
                if let Some(companion) = companion {
                    names.insert(companion.as_str().to_owned());
                }
                pending.push(inner);
            }
            GraphPattern::Graph { name, inner } => {
                if let NamedNodePattern::Variable(variable) = name {
                    names.insert(variable.as_str().to_owned());
                }
                pending.push(inner);
            }
            GraphPattern::Group {
                inner, variables, ..
            } => {
                names.extend(variables.iter().map(|v| v.as_str().to_owned()));
                pending.push(inner);
            }
            GraphPattern::Filter { inner, .. }
            | GraphPattern::Service { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => pending.push(inner),
            GraphPattern::PropertyFunction(call) => collect_call_variables(call, names),
        }
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

/// `term`'s variable name, if it is one — descending, over a work list, into an RDF 1.2
/// quoted triple, whose nested variables bind exactly the way a top-level one does and
/// can therefore carry a witness just as visibly.
fn collect_term_pattern_variable(term: &TermPattern, names: &mut BTreeSet<String>) {
    let mut pending = vec![term];
    while let Some(term) = pending.pop() {
        match term {
            TermPattern::Variable(variable) => {
                names.insert(variable.as_str().to_owned());
            }
            TermPattern::Triple(triple) => {
                if let NamedNodePattern::Variable(variable) = &triple.predicate {
                    names.insert(variable.as_str().to_owned());
                }
                pending.extend([&triple.subject, &triple.object]);
            }
            TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {}
        }
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
            version,
        } => Query::Select {
            pattern: restrict(pattern),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
            version: version.clone(),
        },
        Query::Construct {
            template,
            pattern,
            dataset,
            base_iri,
            version,
        } => Query::Construct {
            template: template.clone(),
            pattern: restrict(pattern),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
            version: version.clone(),
        },
        Query::Describe {
            pattern,
            targets,
            dataset,
            base_iri,
            version,
        } => Query::Describe {
            pattern: restrict(pattern),
            targets: targets.clone(),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
            version: version.clone(),
        },
        Query::Ask {
            pattern,
            dataset,
            base_iri,
            version,
        } => Query::Ask {
            pattern: restrict(pattern),
            dataset: dataset.clone(),
            base_iri: base_iri.clone(),
            version: version.clone(),
        },
    }
}

/// [`restrict_witness_bindings`] over one graph pattern.
fn restrict_pattern(
    pattern: &GraphPattern,
    observable: &BTreeSet<String>,
    witnesses: &[Vec<Option<GroundTerm>>],
) -> GraphPattern {
    let recurse = |inner: &GraphPattern| Child::new(restrict_pattern(inner, observable, witnesses));
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
        GraphPattern::Union { arms } => GraphPattern::Union {
            arms: arms.map_ref(|arm| restrict_pattern(arm, observable, witnesses)),
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
        GraphPattern::Unfold {
            inner,
            expression,
            element,
            companion,
        } => GraphPattern::Unfold {
            inner: recurse(inner),
            expression: expression.clone(),
            element: element.clone(),
            companion: companion.clone(),
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
            left: Child::new(restricted),
            right: Child::new(GraphPattern::Values {
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
///
/// The walk runs over a work list in depth-first order: a quoted triple's subject is
/// examined next, with its object held back until the subject's whole nesting is done,
/// and the first witness found ends it.
fn term_mentions_surrogate(term: &RdfTerm, surrogates: &BTreeSet<String>) -> bool {
    let mut held: Vec<&RdfTerm> = Vec::new();
    let mut next = Some(term);
    while let Some(term) = next.take().or_else(|| held.pop()) {
        match term {
            RdfTerm::BlankNode(label) => {
                if label_is_surrogate(label, surrogates) {
                    return true;
                }
            }
            RdfTerm::Iri(_) | RdfTerm::Literal(_) => {}
            RdfTerm::Triple(triple) => {
                held.push(&triple.object);
                next = Some(&triple.subject);
            }
        }
    }
    false
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

/// Every basic-graph-pattern triple of `query`, in written order, as the [`QTriple`]s the
/// OWL 2 Direct-Semantics reasoner augments a dataset for.
///
/// The walk descends through every join, filter, graph, optional, union and modifier
/// wrapper; a property path, an inline `VALUES` and a property-function call hold no
/// triple pattern and contribute nothing. A triple whose subject or object is an RDF 1.2
/// triple term is skipped, since a triple term never scaffolds a class expression.
#[must_use]
pub fn query_bgp(query: &Query) -> Vec<QTriple> {
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
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
        match pattern {
            GraphPattern::Bgp { patterns } => output.extend(patterns.iter().filter_map(|pattern| {
                Some(QTriple {
                    s: term_to_qnode(&pattern.subject)?,
                    p: named_node_pattern_to_qnode(&pattern.predicate),
                    o: term_to_qnode(&pattern.object)?,
                })
            })),
            GraphPattern::Join { left, right }
            | GraphPattern::Minus { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::LeftJoin { left, right, .. } => {
                pending.extend([&**right, &**left]);
            }
            GraphPattern::Union { arms } => pending.extend(arms.iter().rev()),
            GraphPattern::Filter { inner, .. }
            | GraphPattern::Graph { inner, .. }
            | GraphPattern::Extend { inner, .. }
            // `UNFOLD` matches no triple in any graph either — it expands a value the
            // solution already carries — so it is transparent to this walk.
            | GraphPattern::Unfold { inner, .. }
            | GraphPattern::Service { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::Group { inner, .. } => pending.push(inner),
            // Leaves that hold no triple pattern. A property-function call matches no triple
            // in any graph — its rows come from the relation registry — so it contributes
            // nothing to the pattern that drives OWL Direct's query-directed augmentation,
            // exactly as a `Path` or an inline `VALUES` contributes nothing.
            GraphPattern::Path { .. }
            | GraphPattern::Values { .. }
            | GraphPattern::PropertyFunction(_) => {}
        }
    }
}

/// A subject or object pattern as a [`QNode`]: a variable, or the ground term's value
/// under the evaluator's own query-text conversion. `None` for an RDF 1.2 triple term,
/// which never scaffolds a class expression.
fn term_to_qnode(term: &TermPattern) -> Option<QNode> {
    match term {
        TermPattern::Variable(variable) => Some(QNode::Var(variable.as_str().to_owned())),
        TermPattern::Triple(_) => None,
        ground => convert::ground_term_pattern_to_value(ground, "a basic graph pattern")
            .ok()
            .map(QNode::Term),
    }
}

/// A predicate pattern as a [`QNode`].
fn named_node_pattern_to_qnode(pattern: &NamedNodePattern) -> QNode {
    match pattern {
        NamedNodePattern::NamedNode(node) => QNode::Term(convert::named_node_to_value(node)),
        NamedNodePattern::Variable(variable) => QNode::Var(variable.as_str().to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use purrdf_entail::{Atom, RifTerm, Rule, RuleSet};
    use purrdf_rdf::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};

    use super::*;

    use purrdf_iri::vocab::rdf::TYPE as RDF_TYPE;
    use purrdf_iri::vocab::rdfs::SUB_CLASS_OF as RDFS_SUBCLASS;

    #[test]
    fn every_regime_maps_to_its_own_query_plan_and_only_rif_reads_the_rules() {
        let rules = RuleSet::new();
        let plan = |regime| QueryEntailment::for_regime(regime, &rules);
        assert!(matches!(plan(Regime::Simple), QueryEntailment::Simple));
        assert!(matches!(plan(Regime::Rdf), QueryEntailment::Rdf));
        assert!(matches!(plan(Regime::Rdfs), QueryEntailment::Rdfs));
        assert!(matches!(plan(Regime::OwlRl), QueryEntailment::OwlRl));
        assert!(matches!(plan(Regime::D), QueryEntailment::D));
        assert!(matches!(
            plan(Regime::OwlDirect),
            QueryEntailment::OwlDirect
        ));
        assert!(
            matches!(plan(Regime::Rif), QueryEntailment::Rif(lent) if std::ptr::eq(lent, &raw const rules))
        );
    }

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

    /// A quad-producing `CONSTRUCT`'s template GRAPH variable is OBSERVABLE: its
    /// binding decides which named graph the row's statement lands in, so the
    /// caller reads it straight back off the result dataset's graph name. A
    /// walker that only descended into subject/predicate/object would let a
    /// witness surrogate out through the graph slot unnoticed.
    #[test]
    fn a_construct_template_graph_variable_is_observable() {
        use purrdf_sparql_algebra::SparqlParser;

        // `?g` appears ONLY in the graph slot; `?hidden` appears only in the
        // WHERE, so it is the control that keeps the assertion falsifiable.
        let query = SparqlParser::new()
            .parse_query(
                "CONSTRUCT { GRAPH ?g { ?s <http://example.org/p> ?o } } \
                 WHERE { GRAPH ?g { ?s <http://example.org/q> ?o } . \
                         ?s <http://example.org/r> ?hidden }",
            )
            .expect("the quad-producing CONSTRUCT parses");
        let observable = observable_variables(&query);
        assert!(
            observable.contains("g"),
            "the template graph variable must be observable, got {observable:?}"
        );
        assert!(observable.contains("s"), "{observable:?}");
        assert!(observable.contains("o"), "{observable:?}");
        assert!(
            !observable.contains("hidden"),
            "a WHERE-only variable the template drops is not observable, got {observable:?}"
        );

        // The `CONSTRUCT GRAPH ?g` whole-template shorthand reaches the same
        // slot, so it must report the same variable.
        let query = SparqlParser::new()
            .parse_query("CONSTRUCT GRAPH ?g { ?s <http://example.org/p> ?o } WHERE { ?s ?p ?o }")
            .expect("the shorthand parses");
        assert!(
            observable_variables(&query).contains("g"),
            "the shorthand's graph variable must be observable too"
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
        let bgp = query_bgp(&query);
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
        let mittens = builder.intern_iri("https://example.org/mittens");
        let rdf_type = builder.intern_iri(RDF_TYPE);
        let subclass = builder.intern_iri(RDFS_SUBCLASS);
        builder.push_quad(cat, subclass, animal, None);
        builder.push_quad(mittens, rdf_type, cat, None);
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
        let query = "ASK { <https://example.org/mittens> a <https://example.org/Animal> }";
        query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            mode,
            QueryOptions::EMPTY,
            &ClosureRelations::NONE,
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
            QueryOptions::EMPTY,
            &ClosureRelations::NONE,
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
                QueryOptions::EMPTY,
                &ClosureRelations::NONE,
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

    /// `options` reaches the CLOSURE's query exactly as it reaches an ordinary one: a
    /// caller-registered custom (here, statistical) aggregate resolves an `AGG(<iri>, …)`
    /// call over the RDFS-entailed closure — not merely over the raw asserted dataset.
    ///
    /// Before `query_with_entailment` took a `QueryOptions` parameter, this query answered
    /// the SAME unregistered-aggregate refusal an ordinary custom aggregate gets when no
    /// registry is supplied, regardless of what the caller passed — because there was
    /// nowhere to pass it. Threading `options` through the parse and the evaluation is what
    /// closes that: the closure computed here is provably non-trivial (three individuals
    /// typed `Cat` are entailed `Animal` only through the `rdfs:subClassOf` axiom), and the
    /// statistical aggregate folds a plain numeric predicate over it.
    #[test]
    fn a_statistical_aggregate_resolves_over_the_entailed_closure() {
        use purrdf_sparql_eval::AggregateRegistry;

        const VALUES_TTL: &str = concat!(
            "@prefix ex: <https://example.org/> .\n",
            "ex:s1 ex:value 1 .\n",
            "ex:s2 ex:value 2 .\n",
            "ex:s3 ex:value 3 .\n",
        );
        let dataset = purrdf_rdf::parse_dataset(VALUES_TTL.as_bytes(), "text/turtle", None)
            .expect("the fixture parses");

        let mut registry = AggregateRegistry::new();
        registry.register_statistical_aggregates("https://example.org/agg#");

        let query = "SELECT (AGG(<https://example.org/agg#MEDIAN>, ?v) AS ?m) \
                     WHERE { ?s <https://example.org/value> ?v }";
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::Rdfs,
            QueryOptions::new().with_env(
                &purrdf_sparql_eval::ExtensionEnv::over_aggregates(registry.clone())
                    .expect("the fixture declarations read cleanly"),
            ),
            &ClosureRelations::NONE,
        )
        .expect("the registered aggregate resolves over the entailed closure");
        assert_eq!(report.regime(), Regime::Rdfs);
        let SparqlResult::Solutions { rows, .. } = result else {
            panic!("expected a solution sequence");
        };
        assert_eq!(
            rows[0][0].as_ref().expect("?m is bound"),
            &TermValue::typed_literal("2", "http://www.w3.org/2001/XMLSchema#decimal"),
            "MEDIAN of {{1, 2, 3}} is 2, computed over the closure query_with_entailment answers"
        );
    }

    /// Omitting the aggregate registry (`QueryOptions::EMPTY`) over an entailment-regime
    /// query leaves an `AGG(<iri>, …)` call unregistered — exactly the ordinary refusal,
    /// unaffected by which regime the call is evaluated under. The counterpart regression
    /// to the positive case above: threading `options` through must not make an
    /// unregistered IRI silently succeed.
    #[test]
    fn an_unregistered_aggregate_still_refuses_under_entailment() {
        let query = "SELECT (AGG(<https://example.org/agg#MEDIAN>, ?x) AS ?m) \
                     WHERE { ?x a <https://example.org/Animal> }";
        let error = query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::Rdfs,
            QueryOptions::EMPTY,
            &ClosureRelations::NONE,
        )
        .expect_err("an unregistered custom aggregate must be refused, entailed or not");
        assert!(
            matches!(error, ReasoningError::Query(_)),
            "an unregistered aggregate is a QUERY failure, not an entailment one: {error}"
        );
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
            QueryOptions::EMPTY,
            &ClosureRelations::NONE,
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
    use purrdf_iri::vocab::owl::CLASS as OWL_CLASS;
    use purrdf_iri::vocab::owl::ON_PROPERTY as OWL_ON_PROPERTY;
    use purrdf_iri::vocab::owl::RESTRICTION as OWL_RESTRICTION;
    use purrdf_iri::vocab::owl::SOME_VALUES_FROM as OWL_SOME_VALUES_FROM;

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
            QueryOptions::EMPTY,
            &ClosureRelations::NONE,
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
            QueryOptions::EMPTY,
            &ClosureRelations::NONE,
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
            QueryOptions::EMPTY,
            &ClosureRelations::NONE,
        )
        .unwrap();
        assert_eq!(report.regime(), Regime::OwlDirect);
        assert!(matches!(result, SparqlResult::Boolean(true)));
    }

    // ── Filtration: the witness never reaches the caller, and no answer is lost ────────

    use purrdf_iri::vocab::owl::EQUIVALENT_CLASS as OWL_EQUIVALENT_CLASS;
    use purrdf_iri::vocab::rdfs::SUB_PROPERTY_OF as RDFS_SUBPROPERTY;

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
            QueryOptions::EMPTY,
            &ClosureRelations::NONE,
        )
        .expect("owl-direct answers")
    }

    /// Answer `query` over the WHOLE-VOCABULARY augmentation alone — the lane the combined
    /// approach replaced, evaluated exactly as this module's fallback arm evaluates it.
    fn augmentation_only(ds: &Arc<RdfDataset>, query: &str) -> SparqlResult {
        let engine = NativeSparqlEngine::new();
        let prepared = engine.prepare_query(query, None).expect("parse");
        let pattern = query_bgp(prepared.query());
        let (closure, _) =
            purrdf_entail::materialize(ds, Materialization::OwlDirect(&pattern)).expect("augment");
        engine
            .query_prepared(&closure, &prepared, &[], QueryOptions::EMPTY)
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
            "SELECT DISTINCT ?x WHERE { ?x a ?t }".to_string(),
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

#[cfg(test)]
mod term_walk_tests {
    //! The witness search against its recursive reference, and at a hundred thousand
    //! levels on a 128 KiB thread.

    use std::collections::BTreeSet;

    use purrdf_rdf::{RdfTerm, RdfTriple};

    use super::{label_is_surrogate, term_mentions_surrogate};

    fn reference(term: &RdfTerm, surrogates: &BTreeSet<String>) -> bool {
        match term {
            RdfTerm::BlankNode(label) => label_is_surrogate(label, surrogates),
            RdfTerm::Iri(_) | RdfTerm::Literal(_) => false,
            RdfTerm::Triple(triple) => {
                reference(&triple.subject, surrogates) || reference(&triple.object, surrogates)
            }
        }
    }

    // A SplitMix64 draw from the counter at `state`.
    use purrdf_testkit::rng::splitmix64_next as splitmix64;

    /// A generated owned term of at most `budget` triple terms, its blank nodes drawn
    /// from `w0`, `w1` and `b`.
    fn generated(state: &mut u64, budget: &mut usize) -> RdfTerm {
        if *budget > 0 && splitmix64(state).is_multiple_of(3) {
            *budget -= 1;
            let subject = generated(state, budget);
            let object = generated(state, budget);
            return RdfTerm::triple(RdfTriple::new(subject, "http://example.org/p", object));
        }
        match splitmix64(state) % 4 {
            0 => RdfTerm::iri("http://example.org/i"),
            1 => RdfTerm::blank_node("w0"),
            2 => RdfTerm::blank_node("w1"),
            _ => RdfTerm::blank_node("b"),
        }
    }

    /// Every generated term mentions a witness exactly when the recursive reference says
    /// it does.
    #[test]
    fn the_search_agrees_with_its_recursive_reference_on_generated_terms() {
        let surrogates = BTreeSet::from(["w1".to_owned()]);
        let mut mentioning = 0;
        for seed in 0..400_u64 {
            let (mut state, mut budget) = (seed, 8);
            let term = generated(&mut state, &mut budget);
            let found = term_mentions_surrogate(&term, &surrogates);
            assert_eq!(found, reference(&term, &surrogates), "seed {seed}");
            mentioning += usize::from(found);
        }
        assert!(mentioning > 0, "some generated term mentions a witness");
    }

    /// A triple term a hundred thousand levels deep, its innermost object a witness, is
    /// searched on a thread whose whole stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_term_is_searched_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let surrogates = BTreeSet::from(["w".to_owned()]);
            let mut term = RdfTerm::blank_node("w");
            for _ in 0..LEVELS {
                term = RdfTerm::triple(RdfTriple::new(
                    RdfTerm::iri("http://example.org/s"),
                    "http://example.org/p",
                    term,
                ));
            }
            assert!(term_mentions_surrogate(&term, &surrogates));
            // The owned model's derived drop descends once per level, so the chain
            // is taken apart one level at a time.
            while let RdfTerm::Triple(triple) = term {
                term = triple.object;
            }
        })
        .expect("the thread starts");
    }
}
