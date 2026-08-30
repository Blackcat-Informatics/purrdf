// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]

//! Native, wasm-clean entailment for the PurRDF [`RdfDataset`] IR.
//!
//! A family of engines sits behind one façade, each the right tool for its regime.
//! The forward-materialization ("chase") engine closes a dataset's default graph under a
//! fixed RDF / RDFS / OWL-RL rule set to a fixpoint. That rule set is not written twice:
//! [`calculus_program`] renders it as DL clauses, and [`materialize`] evaluates exactly
//! those clauses through `purrdf-datalog`'s native semi-naive evaluator (no Nemo, no
//! `tokio`, no external reasoner), so this crate stays `wasm32`-clean and MIT/Apache.
//! `Simple` is the identity closure; `RDF`, `RDFS`, `OWL-RL` and `D` run the declared
//! program. The `OWL-RL` lane states the WHOLE of OWL 2 Profiles §4.3 Tables 4–9 — all 78
//! rules — and the seventeen of them whose conclusion is `false` are DECIDED rather than
//! drawn: a body match is [`EntailError::Inconsistent`] carrying an
//! [`InconsistencyWitness`], because an inconsistent knowledge base entails every triple
//! and a closure over it would answer a question nobody asked.
//!
//! The open-world `OWL-Direct` (Description-Logic tableau) and `RIF` (rule engine)
//! regimes need inputs the other five do not: the query's class expressions, and a parsed
//! rule set. That is a fact about the REGIMES, so it is stated in the type the caller
//! selects one with. [`materialize`] takes a [`Materialization`], not a [`Regime`], and a
//! `Materialization` carries each regime's own input as a non-optional field —
//! [`Materialization::OwlDirect`] a basic graph pattern, [`Materialization::Rif`] a
//! [`RuleSet`]. All seven inhabitants are served, so `materialize` is TOTAL over its
//! parameter and there is no regime this crate refuses.
//!
//! [`Regime`] stays: it is the REPORTING and IDENTITY type — what
//! [`ReasoningReport::regime`] names, what [`rules`] and [`implemented`] are indexed by,
//! what [`Regime::from_iri`] parses a `sparql:entailmentRegime` IRI into.
//! [`Materialization::regime`] is the map from the input to the identity. Splitting the two
//! is what lets "which regime is this" stay a seven-way copyable value while "run this
//! regime" carries the inputs that regime is defined by.
//!
//! The two lanes are still reachable directly, as [`materialize_dl_reported`] and
//! [`materialize_rif`]: `materialize` DELEGATES to them rather than restating them, so
//! there is one implementation of each lane and one report assembly on each emission path.
//!
//! # The Description-Logic services
//!
//! [`Reasoner`] is the tableau's own surface: consistency, class satisfiability,
//! classification, realization, instance retrieval and axiom entailment, each answering a
//! [`Certified<T>`](Certified) whose [`DlCertificate`] says how complete the answer is.
//! Beside it sit two services that need no reasoning at all — [`extract_module`], which
//! computes a syntactic-locality module for a signature, and [`profile()`], which certifies
//! an ontology against the OWL 2 profiles. See [`reasoner`] for why a tableau needs a
//! completeness notion of its own rather than the chase's [`Completeness`].
//!
//! It mints **no** vocabulary IRIs: every constant in `vocab` is a standard
//! `rdf:`/`rdfs:`/`owl:` IRI from the entailment spec itself. `D` (datatype)
//! entailment IS materializable: this crate realizes it as Simple entailment plus the
//! five `dt-*` rules of OWL 2 Profiles §4.3 Table 8, which is the part of D-entailment a
//! forward chase can produce, and reports the value-space boundary the rest of it lives
//! behind.
//!
//! What each regime *is* and what this crate currently *does* are both data, not
//! prose: [`rules`] returns the specification rule table a [`Regime`] is defined by
//! (78 [`RuleId`]s for `OWL-RL`, 18 for `RDFS`), and [`implemented`] returns the
//! subset the chase fires today. The difference is the regime's measurable gap.
//!
//! # Every answer can say WHY, and the two lanes say it differently
//!
//! [`explain`] is the audit surface, and it deliberately gives the two engines two types
//! rather than one, because they do not explain the same thing:
//!
//! * [`explain_conclusion`] answers a chase conclusion with a [`ChaseProof`] — the actual
//!   DERIVATION, whose [`check`](ChaseProof::check) re-derives the head from the premises
//!   against the clause program rather than re-reading the claim.
//! * [`justify`] answers a Description-Logic axiom with a [`Justification`] — a MINIMAL
//!   ENTAILING SUBSET of the ontology, found by black-box shrinking, whose two halves are
//!   re-decidable: [`is_sufficient`](Justification::is_sufficient) and
//!   [`is_minimal`](Justification::is_minimal).
//!
//! A tableau performs no derivation steps, so it has no proof to check; a justification is
//! the checkable analogue, and forcing one type on both would let a caller read a tableau
//! answer as though a rule had fired. Neither is named by a minted IRI — a justification is
//! a set of axioms already in the input, and where an identifier is useful it is a BLAKE3
//! CONTENT DIGEST.
//!
//! # Every call says what it did
//!
//! [`materialize`] returns a [`ReasoningReport`] with every closure — not on request, not
//! behind a second entry point. The report carries the regime's [`Completeness`] (derived
//! from the inventory above, so it improves by itself as rules are added), which rules
//! actually fired and how many conclusions each contributed, the [`Boundary`]s the run
//! met, what it consumed of the evaluation ceilings, and the
//! [`contract_hash`](ReasoningReport::contract_hash) of the calculus it ran — so a
//! consumer can refuse a cached closure minted under a different rule set instead of
//! trusting a sentence about it. See [`report`] for the whole shape, and for why a report
//! cannot claim [`Completeness::Exact`] beside a boundary it names — the completeness is
//! DERIVED from the boundary list rather than stored beside it.
//!
//! # The combined approach, for a non-distinguished query variable
//!
//! [`materialize_dl_reported`]'s own whole-vocabulary augmentation is exact for a basic
//! graph pattern all of whose variables are DISTINGUISHED (projected) — its module docs say
//! so explicitly, because the decomposition it relies on genuinely does not extend to a
//! non-distinguished (unprojected, or blank-node) variable: an open-world model may satisfy
//! an existential through an anonymous element no finite augmentation over named terms can
//! produce. [`combined`] is where that gap is closed for the fragment it can be closed for:
//! TBox axioms of the shape `A ⊑ B` and `A ⊑ ∃r.B` over named vocabulary lower into
//! `purrdf-datalog`'s DL-clause IR, the crate's own restricted chase materializes the
//! existential witnesses those axioms license as ordinary blank-node facts, and a caller
//! filters out any answer that would bind a DISTINGUISHED variable to one of them — the
//! combined approach of Lutz/Toman/Wolter and Stefanoni/Motik/Horrocks, applied over this
//! crate's own chase rather than a description-logic-specific canonical-model construction.
//! Outside that fragment [`materialize_combined`] reports "not applicable" and the caller
//! keeps using the whole-vocabulary augmentation, disclosing [`Construct::NonHornTBox`].

use std::sync::Arc;

use purrdf_core::{DatasetView, RdfDataset};
use purrdf_datalog::StopSignal;

pub(crate) mod axioms;
pub(crate) mod calculus;
pub mod combined;
pub(crate) mod datatypes;
pub(crate) mod engine;
pub mod entails;
pub mod explain;
pub(crate) mod interner;
pub(crate) mod lists;
pub(crate) mod owl_dl;
pub mod reasoner;
pub mod report;
pub mod rif;
mod rif_xml;
pub(crate) mod rules;
pub(crate) mod surrogates;
pub(crate) mod vocab;

pub use calculus::calculus_program;
pub use combined::{CombinedMaterialization, materialize_combined, materialize_combined_until};
pub use entails::{
    Binding, CertainAnswers, CompositeWarrant, ComprehensionWarrant, DataRangeWarrant,
    EntailmentCertificate, EntailmentMechanism, EntailmentOutcome, EntailmentWarrant,
    FREEZE_BUDGET, FreezeWarrant, FrozenInstance, FrozenOutcome, Generalization,
    HomomorphismWarrant, ImportMap, MATCH_BUDGET, MissReason, NegativeFact, REFUTATION_BUDGET,
    ReflexivityWarrant, Refutation, RefutationWarrant, UndecidedReason, VarKey, certain_answers,
    entails, verify,
};
pub use explain::{
    BackwardCheck, ChaseProof, ExplainError, Justification, explain_conclusion, justify,
};
pub use owl_dl::proof::{
    BlockingPair, BranchOutcome, BranchReplay, BranchStep, CALCULUS_VERSION as DL_CALCULUS_VERSION,
    CheckReport, ClashReplay, ClashStep, Completion, CompletionEdge, CompletionNode,
    CompletionReplay, DerivedConclusion, DlProof, DlProofContext, DlProofError,
    MAX_CHECK_WORK as DL_MAX_CHECK_WORK, MAX_RECORDED_STEPS as DL_MAX_RECORDED_STEPS, MergeCause,
    MergeLicence, MergeReplay, MergeStep, NodeRef, PartialReplay, ProofAlternative, ProofAnswer,
    ProofFact, ProofGround, ProofRole, RefutationReplay, ReservedRef,
    TRUST_BASE_VERSION as DL_TRUST_BASE_VERSION, TrustBaseEntry, ontology_identity,
    prove_consistency,
};
pub use owl_dl::query::{QNode, QTriple, materialize_dl_reported, materialize_dl_reported_until};
pub use reasoner::{
    Certified, Claim, ClaimBasis, ClaimSubject, ClassHierarchy, ConservativeKeep, DlAxiom,
    DlCertificate, DlCompleteness, ModuleExtraction, ModuleMethod, OwlProfile, ProfileCertificate,
    ProfileViolation, Question, Realization, Reasoner, RunAssumptions, RunProof, Service,
    ServiceProof, ServiceReplay, StopCause, StopReceipt, Verdict, extract_module,
    extract_module_with_proofs, profile,
};
pub use report::{
    Boundary, Completeness, Construct, InconsistencyWitness, InconsistentRun, ReasoningReport,
    TerminationCertificate, WitnessTriple,
};
pub use rif::{Atom, Fact, RifTerm, Rule, RuleSet, materialize_rif, materialize_rif_until};
pub use rif_xml::{ParsedRifDocument, RifImport, parse_rif_xml, resolve_rif_imports};
pub use rules::{ParseRuleIdError, RuleId, extensions, implemented, rules};

/// A SPARQL entailment regime (`sparql:entailmentRegime`), by its W3C IRI's local
/// name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// `entailment/Simple` — no entailment; the graph is its own closure.
    Simple,
    /// `entailment/RDF` — RDF entailment (the predicate-typing axiomatic rule: every
    /// resource in predicate position is an `rdf:Property`).
    Rdf,
    /// `entailment/RDFS` — RDFS entailment via the native chase.
    Rdfs,
    /// `entailment/OWL-RL` (a.k.a. OWL 2 RL) — RDFS + the OWL-RL-shaped rules.
    OwlRl,
    /// `entailment/OWL-Direct` — open-world OWL DL via the SHOIQ(D) tableau. Not a
    /// materialize-and-match affair; it needs the query's class expressions.
    OwlDirect,
    /// `entailment/RIF` — RIF-Core rule entailment; needs a parsed rule set.
    Rif,
    /// `entailment/D` — datatype entailment.
    ///
    /// Realized as Simple entailment plus the five `dt-*` rules of OWL 2 Profiles §4.3
    /// Table 8, which is the fixed rule table this crate can enumerate for it: the rest of
    /// D-entailment quantifies over infinite value spaces and is reported as the
    /// [`Construct::DatatypeValueSpace`] boundary rather than claimed.
    D,
}

impl Regime {
    /// Parse a regime IRI (e.g. `http://www.w3.org/ns/entailment/RDFS`).
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        match iri.rsplit('/').next()? {
            "Simple" => Some(Self::Simple),
            "RDF" => Some(Self::Rdf),
            "RDFS" => Some(Self::Rdfs),
            "OWL-RL" | "OWL-RDF-Based" => Some(Self::OwlRl),
            "OWL-Direct" => Some(Self::OwlDirect),
            "RIF" => Some(Self::Rif),
            "D" => Some(Self::D),
            _ => None,
        }
    }
}

/// A regime TOGETHER WITH the input that regime is defined by — the parameter
/// [`materialize`] is total over.
///
/// Five of the seven SPARQL entailment regimes are defined by a fixed rule table this
/// crate states, so naming them is the whole input. Two are not: `OWL-Direct` is
/// query-directed (the tableau augments the data for the class expressions the QUERY
/// mentions) and `RIF` entails under a rule set the CALLER wrote. A parameter that could
/// not express those two inputs would make `materialize` partial in its own signature —
/// a caller could hand it a value it is documented to accept and get a refusal instead of
/// an answer — so the parameter expresses them.
///
/// [`Regime`] remains the regime's IDENTITY: [`Self::regime`] maps this value onto it, and
/// that is what [`ReasoningReport::regime`], [`rules`], [`implemented`] and
/// [`calculus_program`] speak in. The two types answer different questions — "which regime
/// is this" and "run this regime" — and only the second one needs a rule set.
///
/// `Copy`, because every payload is a shared borrow: passing a plan costs a pointer pair,
/// never a clone of the rule set.
///
/// ```
/// use purrdf_entail::{Materialization, Regime, RuleSet};
///
/// let rules = RuleSet::new();
/// assert_eq!(Materialization::Rdfs.regime(), Regime::Rdfs);
/// // The two query-directed lanes carry their input, so naming them is not enough —
/// // and once it is supplied there is nothing left to refuse.
/// assert_eq!(Materialization::OwlDirect(&[]).regime(), Regime::OwlDirect);
/// assert_eq!(Materialization::Rif(&rules).regime(), Regime::Rif);
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Materialization<'a> {
    /// `entailment/Simple` — the identity closure.
    Simple,
    /// `entailment/RDF` — the RDF rule table.
    Rdf,
    /// `entailment/RDFS` — the eighteen RDFS patterns.
    Rdfs,
    /// `entailment/OWL-RL` — the whole of OWL 2 Profiles §4.3 Tables 4–9.
    OwlRl,
    /// `entailment/D` — Simple entailment plus the five `dt-*` rules of Table 8.
    D,
    /// `entailment/OWL-Direct` — the SHOIQ(D) tableau, directed by the query's basic graph
    /// pattern.
    ///
    /// The pattern is what makes this lane query-directed: a class expression written in
    /// the query is re-materialized and its instances retrieved, so the augmented dataset's
    /// SIMPLE-entailment answers to THAT query are the OWL Direct-Semantics certain
    /// answers. An EMPTY pattern is a legitimate input and not a degenerate one: it asks
    /// for the query-independent augmentation — the classification, the realization, the
    /// entailed role assertions and the `owl:sameAs` identifications the tableau decides
    /// about the ontology's own named terms — which is the whole answer when there is no
    /// query to direct it. See [`materialize_dl_reported`].
    OwlDirect(&'a [QTriple]),
    /// `entailment/RIF` — the caller's RIF-Core rule set, forward-chained to a fixpoint.
    ///
    /// The rule set is the caller's document: this crate declares no RIF rules of its own,
    /// mints no [`RuleId`] for a rule it did not author, and therefore cannot supply a
    /// default. See [`materialize_rif`], and [`parse_rif_xml`] for building a [`RuleSet`]
    /// from a normative RIF-in-XML document.
    Rif(&'a RuleSet),
}

impl Materialization<'_> {
    /// The [`Regime`] this plan runs — its reporting and inventory identity.
    ///
    /// The match is exhaustive with no wildcard arm, so the compiler is what forces this
    /// map to be revisited if either enum grows.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        match self {
            Self::Simple => Regime::Simple,
            Self::Rdf => Regime::Rdf,
            Self::Rdfs => Regime::Rdfs,
            Self::OwlRl => Regime::OwlRl,
            Self::D => Regime::D,
            Self::OwlDirect(_) => Regime::OwlDirect,
            Self::Rif(_) => Regime::Rif,
        }
    }
}

/// Why a closure could not be produced.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EntailError {
    /// Building the derived dataset failed.
    Build(String),
    /// A knowledge-base or rule document was malformed (e.g. an ill-formed OWL
    /// class-expression graph or an unrecognized RIF construct).
    Parse(String),
    /// A PROOF-CARRYING operation was asked of a reasoner that records no proofs.
    ///
    /// Recording is opt-in: [`Reasoner::new`] answers questions and keeps no evidence, while
    /// [`Reasoner::with_proofs`] pays for the ontology's RDFC-1.0 identity and the run traces
    /// bound to it. [`Reasoner::proof_context`] needs that identity — a checking context
    /// without it would reject every honest proof with an input mismatch nothing accounts for
    /// — so it refuses here instead, naming the constructor that would have worked.
    ///
    /// It is deliberately NOT how an absent proof term is reported. A service answer says that
    /// with [`Certified::proof`] returning `None`, because an answer produced without
    /// recording is a perfectly good answer; only asking such a reasoner to CHECK something is
    /// an error.
    ProofsNotRecorded,
    /// The declared calculus could not be evaluated to a fixpoint.
    ///
    /// [`materialize`] runs [`calculus_program`] through `purrdf-datalog`'s semi-naive
    /// evaluator, and that evaluator refuses rather than approximates: a program it has no
    /// semantics for, and — the case a caller will actually meet — an input that passes
    /// one of its three fixed evaluation ceilings. A budget refusal is TOTAL, which is why
    /// it is an error and not a boundary: there is no partial closure to hand back with a
    /// note attached, and a truncated closure presented as a complete one is exactly the
    /// failure a [`ReasoningReport`] exists to prevent. The carried
    /// [`EvalError`](purrdf_datalog::seminaive::EvalError) names which ceiling and what
    /// the run had consumed when it stopped.
    Evaluate(purrdf_datalog::seminaive::EvalError),
    /// The declared calculus states an EXISTENTIAL rule the restricted chase refused.
    ///
    /// `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` conclude about a FRESH blank node, and a
    /// least-fixpoint evaluator over definite clauses has no semantics for that head form,
    /// so the `RDF` and `RDFS` lanes run through `purrdf-datalog`'s restricted chase
    /// instead. The chase refuses rather than approximates, and the refusal a caller will
    /// actually meet is one of the three fixed evaluation ceilings —
    /// [`ChaseError::BudgetExhausted`](purrdf_datalog::chase::ChaseError::BudgetExhausted)
    /// — carrying an accurate report. The one refusal that is about the CALCULUS rather
    /// than the input is
    /// [`ChaseError::NonTerminating`](purrdf_datalog::chase::ChaseError::NonTerminating):
    /// the chase computes its own termination class from the clause set and runs only a
    /// program it certified, so a rule set whose position dependency graph puts an
    /// existential edge in a cycle is named rather than looped on. Neither declared lane
    /// is such a program, which is a CHECKED fact rather than a hope.
    Chase(purrdf_datalog::chase::ChaseError),
    /// An RDF collection an OWL 2 axiom points at is not a well-formed collection.
    ///
    /// `owl:intersectionOf`, `owl:unionOf`, `owl:oneOf`, `owl:members`,
    /// `owl:distinctMembers`, `owl:propertyChainAxiom` and `owl:hasKey` all REQUIRE their
    /// object to be an RDF collection, and the `OWL-RL` lane walks each one into an
    /// internal relation before evaluating. A cell with no `rdf:first`, with two, with no
    /// `rdf:rest`, with two, a walk that never reaches `rdf:nil`, or a cycle is a refusal
    /// rather than a truncation: reasoning over the well-formed PREFIX of a broken
    /// collection would answer a question the caller did not ask, and it would do so
    /// silently. The message names the collection's head, the cell the walk stopped at,
    /// and the fault.
    MalformedList(String),
    /// The knowledge base is inconsistent: every query would be entailed, so no
    /// meaningful answer set exists. A hard failure rather than a silent default.
    ///
    /// # The witness is not optional, and neither is the report
    ///
    /// Seventeen OWL 2 RL rules conclude `false`, and turning a body match on one of them
    /// into an error is a real behaviour change for a caller: ordinary dirty data — ONE
    /// `owl:disjointWith` violation, ONE ill-typed literal — stops being a closure that
    /// returns answers and becomes a refusal. That is correct, because an inconsistent
    /// knowledge base entails every triple and a closure over it would be an answer to a
    /// question nobody asked. It is also unusable without evidence, so the evidence is
    /// carried rather than offered: [`InconsistencyWitness`] names the rule whose premises
    /// were all satisfied, the asserted triples that satisfied them in that rule's own
    /// premise order, and the graph they were read from.
    ///
    /// The [`ReasoningReport`] is carried for the same reason. This crate says it has no
    /// report-free variant of [`materialize`], and a refusal that handed back a witness
    /// alone was one: the caller whose data is inconsistent learned nothing about which
    /// rules had already fired, what the evaluation had cost, which constructs the run had
    /// met, or which calculus hash refused. [`InconsistentRun`] carries both halves, and the
    /// report's [`ReasoningReport::inconsistency`] is the same witness — which is what makes
    /// that field an observable fact on every host's report surface rather than a constant
    /// `none`.
    ///
    /// Boxed because an error type is returned by value from every entailment entry point.
    Inconsistent(Box<InconsistentRun>),
    /// The `OWL-Direct` knowledge base is unsatisfiable, as the tableau found it.
    ///
    /// Distinct from [`Self::Inconsistent`] because the evidence is of a different kind: a
    /// tableau closes every branch of a search, it does not fire a named rule on named
    /// premises, so there is no [`RuleId`] to carry and no triple set that is THE witness.
    /// Reporting it under the chase's variant would mean inventing a rule id, which this
    /// crate does not do.
    Unsatisfiable,
    /// [`entails()`] / [`certain_answers`] were asked for a regime they are not total over.
    ///
    /// [`materialize`] is total over [`Materialization`] because that parameter CARRIES each
    /// regime's own input. The conclusion-directed signature does not: it takes a premise, a
    /// question and a [`Regime`], which is enough for the five regimes defined by a rule
    /// table this crate states and is not enough for the two defined by something else —
    /// `OWL-Direct` by the query's class expressions, `RIF` by the caller's rule document.
    ///
    /// The alternative to refusing is silently answering under a weaker regime, which
    /// produces a sound answer to a question the caller did not ask and labels it with the
    /// regime they did. So the refusal names the regime, and a caller that wants those two
    /// reaches [`materialize`] with the input they are defined by.
    UnsupportedRegime(Regime),
    /// A premise `owl:imports` a document the caller's [`ImportMap`] does not resolve.
    ///
    /// OWL 2 defines an ontology's imports closure to BE the ontology, so this is not a
    /// slightly smaller premise — it is a different one, and every answer over it would be
    /// about that different one. PurRDF fetches nothing and mints no vocabulary, so it has
    /// no way to guess what an ontology IRI names; the closure is caller-supplied
    /// configuration, and its absence is a refusal that carries the IRI so the caller learns
    /// exactly which document to supply.
    UnresolvedImport(String),
    /// A blank-node match visited [`MATCH_BUDGET`] candidate triples without finishing.
    ///
    /// Graph homomorphism is NP-complete in general, and a conclusion with many blank nodes
    /// over a large closure can make the backtracking search exponential. The budget is a
    /// STEP count rather than a clock reading, so the refusal is reproducible on every
    /// target — and it is an error rather than a verdict because "I stopped looking" and
    /// "there is nothing to find" are different claims and only one of them is true.
    MatchBudget,
    /// The caller's [`purrdf_datalog::StopSignal`] fired while the closure was
    /// still being computed.
    ///
    /// # Not a budget, and not a partial closure
    ///
    /// This crate's ceilings are constants for the reason
    /// [`purrdf_datalog`](purrdf_datalog#budgets-are-constants-not-knobs) states: a
    /// caller-supplied ceiling would make the ANSWER depend on the caller. A stop signal
    /// does not — it either lets the run finish, in which case the closure is bit-for-bit
    /// the one an ungoverned run produces, or it ends the run with nothing. This variant is
    /// the "with nothing" case, and it carries no partial closure by construction: there is
    /// no field on it a caller could read one out of.
    ///
    /// It is deliberately distinct from [`Self::Evaluate`] and [`Self::Chase`], which report
    /// a FIXED ceiling being passed by the program and the data. That is a reason to change
    /// the input; this is not a statement about the input at all.
    ///
    /// Reachable only from the `*_until` entry points ([`materialize_until`]); an ungoverned
    /// call names no signal and so can never see it.
    Stopped,
}

impl std::fmt::Display for EntailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Build(msg) => write!(f, "entailment build error: {msg}"),
            Self::Parse(msg) => write!(f, "entailment parse error: {msg}"),
            Self::ProofsNotRecorded => write!(
                f,
                "this reasoner records no proofs, so it has no ontology identity to check one \
                 against; build it with Reasoner::with_proofs"
            ),
            Self::Evaluate(error) => write!(f, "entailment evaluation error: {error}"),
            Self::Chase(error) => write!(f, "entailment chase error: {error}"),
            Self::MalformedList(msg) => write!(f, "entailment collection error: {msg}"),
            Self::Inconsistent(run) => write!(
                f,
                "knowledge base is inconsistent: {} was satisfied by {} asserted {}",
                run.witness().rule(),
                run.witness().premises().len(),
                if run.witness().premises().len() == 1 {
                    "triple"
                } else {
                    "triples"
                }
            ),
            Self::Unsatisfiable => {
                write!(f, "the OWL-Direct knowledge base is unsatisfiable")
            }
            Self::UnsupportedRegime(regime) => write!(
                f,
                "the conclusion-directed entailment service is not total over {regime:?}: that \
                 regime is defined by an input its signature does not carry"
            ),
            Self::UnresolvedImport(iri) => write!(
                f,
                "the premise owl:imports <{iri}>, which the supplied import map does not resolve"
            ),
            Self::MatchBudget => write!(
                f,
                "the blank-node match exceeded its {MATCH_BUDGET}-candidate budget"
            ),
            Self::Stopped => write!(
                f,
                "the caller's stop signal ended the run before the closure was computed: no \
                 closure was produced and none is claimed"
            ),
        }
    }
}

impl std::error::Error for EntailError {
    /// The wrapped cause, for the variants that carry one.
    ///
    /// Two of these variants exist ONLY to carry another crate's diagnostic —
    /// `Evaluate` holds the evaluator's ceiling report, `Chase` holds the termination
    /// analysis — and without this the standard chain stopped at the outermost message.
    /// A caller printing `{:#}`, or walking `Error::source`, saw "the chase refused"
    /// and could not reach WHICH ceiling refused it, though the value was right there;
    /// the only way through was to match this enum's concrete variant, which is what a
    /// trait-object error exists to avoid.
    ///
    /// The rest return `None` because they genuinely have no cause to name.
    /// `Build`, `Parse`, `MalformedList` and `UnresolvedImport` carry a `String` — a
    /// rendered message, not an error value — and `Unsatisfiable`, `MatchBudget` and
    /// `UnsupportedRegime` and `ProofsNotRecorded` are complete statements in themselves. `Inconsistent` carries
    /// an `InconsistentRun`, which is a WITNESS rather than a failure: it is evidence
    /// that the premise has no model, and the run it describes succeeded at producing
    /// it, so calling it the cause of this error would misdescribe both.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Evaluate(inner) => Some(inner),
            Self::Chase(inner) => Some(inner),
            Self::Build(_)
            | Self::Parse(_)
            | Self::MalformedList(_)
            | Self::Inconsistent(_)
            | Self::Unsatisfiable
            | Self::UnsupportedRegime(_)
            | Self::UnresolvedImport(_)
            | Self::MatchBudget
            | Self::ProofsNotRecorded
            | Self::Stopped => None,
        }
    }
}

/// Compute the entailment closure of `ds` under `plan`, and say what was done.
///
/// Returns the closure — a new dataset holding every original quad plus the inferred
/// triples, each in the graph that produced it; `Simple` returns a faithful copy —
/// together with the [`ReasoningReport`] for the run.
///
/// # It is TOTAL over its parameter
///
/// Every one of [`Materialization`]'s seven inhabitants is served. There is no regime this
/// function refuses, because the parameter carries what each regime needs rather than
/// naming a regime and hoping: [`Materialization::OwlDirect`] holds the query's basic
/// graph pattern and [`Materialization::Rif`] holds the rule set. Those two lanes are
/// DELEGATED to [`materialize_dl_reported`] and [`materialize_rif`] — one implementation
/// each, one report assembly each — rather than restated here.
///
/// # What a DATASET entails is a defined choice, and this is the choice
///
/// RDF 1.2 Semantics defines entailment over a GRAPH and SPARQL's entailment regimes are
/// defined over the ACTIVE graph. Neither says what a dataset entails, so a reasoner handed
/// one has to choose, and PurRDF's defined behaviour is:
///
/// * the DEFAULT graph is closed against itself;
/// * each NAMED graph is closed against the union of itself and the default graph;
/// * a conclusion lands in the graph that PRODUCED it, so a conclusion the default graph
///   already draws on its own is not restated in a named graph that also reached it.
///
/// Two named graphs therefore never join. Every run whose input holds a named graph reports
/// the [`Construct::NamedGraph`] boundary, whose reason states this as a defined choice
/// rather than a derived one — and whose cost is measured rather than hidden: `n` named
/// graphs is `1 + n` evaluations, whose join steps are summed into
/// [`ReasoningReport::budget`] while the two occupancy coordinates report the peak single
/// store.
///
/// # The report is not optional
///
/// There is deliberately no report-free variant of this function. A caller that ignores
/// the report must still bind it, because the alternative — two entry points, one of which
/// discards the evidence — is exactly how a partial rule set comes to be described as
/// complete: the cheap call wins, and nothing downstream can tell that "OWL 2 RL
/// entailment" meant twelve of seventy-eight rules. Binding it costs one `_`; not having
/// it cost this repository a documented overclaim.
///
/// # Errors
///
/// [`EntailError::Inconsistent`] if a rule that concludes `false` matched;
/// [`EntailError::Evaluate`] if the run passes one of `purrdf-datalog`'s three fixed
/// evaluation ceilings; [`EntailError::Build`] if the derived dataset cannot be frozen. The
/// two delegated lanes add their own: [`EntailError::Unsatisfiable`] and
/// [`EntailError::Parse`].
///
/// One of those errors describes a RUN, and it carries it. [`EntailError::Inconsistent`]
/// carries an [`InconsistentRun`] — the witness AND the [`ReasoningReport`] for everything
/// the run had done when it stopped — because "there is no report-free variant of this
/// function" has to hold for the caller whose data is bad, or it is not a rule. The rest
/// are the absence of a run: an exhausted budget carries its own accurate consumption
/// figures, and nothing was closed for a report to describe.
///
/// ```
/// use purrdf_entail::{Materialization, Regime, RuleId, materialize};
/// use purrdf_core::RdfDatasetBuilder;
///
/// let mut builder = RdfDatasetBuilder::new();
/// let cat = builder.intern_iri("http://example.org/Cat");
/// let animal = builder.intern_iri("http://example.org/Animal");
/// let tom = builder.intern_iri("http://example.org/tom");
/// let sub = builder.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let ty = builder.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// builder.push_quad(cat, sub, animal, None);
/// builder.push_quad(tom, ty, cat, None);
/// let dataset = builder.freeze().expect("freeze");
///
/// // rdfs9 re-types the instance — `tom` is an `Animal` as well as a `Cat`.
/// let (closure, report) = materialize(&dataset, Materialization::Rdfs).expect("rdfs");
/// assert!(report.rules_fired().iter().any(|&(r, n)| r == RuleId::Rdfs9 && n >= 1));
/// // …but it is far from the only conclusion: the RDFS lane asserts the axiomatic
/// // triples, so `Cat` and `Animal` are `rdfs:Class`es (rdfs2 / rdfs3 over the
/// // axiomatic domain and range of `rdfs:subClassOf`), each is therefore a sub-class of
/// // itself and of `rdfs:Resource`, and rdfs4 types every term an `rdfs:Resource`.
/// assert!(closure.quad_refs().count() > 3);
/// // RDFS defines 18 patterns and this crate fires all 18 — the four that conclude about
/// // a fresh blank node through `purrdf-datalog`'s restricted chase. The closure is still
/// // not everything the regime entails, and the report says so with a BOUNDARY rather
/// // than with a missing rule: a surrogate blank node is not an answer a SPARQL
/// // entailment regime admits, so every conclusion mentioning one is withheld.
/// assert!(report.completeness().missing().is_empty());
/// // A complete rule table AND a construct in the way: the derived completeness says both
/// // halves at once, because it is a function of the boundary list rather than a second
/// // claim beside it.
/// assert!(!report.boundaries().is_empty());
/// assert_eq!(report.completeness(), purrdf_entail::Completeness::ExactWithinBoundaries);
///
/// // …and the two lanes that need an input are reached the same way, through the same
/// // function, once that input is in the parameter.
/// let (_, dl) = materialize(&dataset, Materialization::OwlDirect(&[])).expect("owl-direct");
/// assert_eq!(dl.regime(), Regime::OwlDirect);
/// ```
pub fn materialize<D: DatasetView>(
    ds: &D,
    plan: Materialization<'_>,
) -> Result<(Arc<RdfDataset>, ReasoningReport), EntailError> {
    materialize_until(ds, plan, None)
}

/// [`materialize`], with a caller-owned latching stop signal polled across the closure.
///
/// # What a stop signal is, and what it is emphatically not
///
/// It is **not** a budget. `purrdf-datalog`'s three ceilings are constants for a stated
/// reason — [budgets are constants, not
/// knobs](purrdf_datalog#budgets-are-constants-not-knobs) — and nothing here weakens it: no
/// charge is configurable, no schedule is named, and no number a caller passes can change
/// which triples a closure holds. A [`purrdf_datalog::StopSignal`] is answer-blind by
/// construction. There are exactly two outcomes:
///
/// * the signal never fires, and this function returns **bit-for-bit** what [`materialize`]
///   returns for the same input — the poll is a load and a branch at a boundary the fixpoint
///   was going to cross anyway; or
/// * the signal fires, and this function returns [`EntailError::Stopped`] — **no** closure,
///   **no** report, nothing partial and nothing claimed.
///
/// So there is no pinnable profile here to get wrong, and no third outcome a consumer could
/// mistake for a complete closure. What it buys is the honesty of a host's wall deadline: a
/// materialized closure is routinely the expensive half of an entailment-regime query, and a
/// deadline that bounds only the query evaluated over the finished closure is a deadline
/// that expires exactly when it cannot be enforced.
///
/// # Where it is polled
///
/// Every lane, at the finest boundary that lane HAS:
///
/// * `Rdf`, `Rdfs` — once per restricted-chase round (`purrdf-datalog`'s
///   [`chase_until`](purrdf_datalog::chase::chase_until));
/// * `OwlRl`, `D` — once per semi-naive round
///   ([`evaluate_until`](purrdf_datalog::seminaive::evaluate_until));
/// * every rule-table lane, additionally once per NAMED GRAPH, because a dataset is closed
///   graph by graph and the copying between two evaluations is otherwise unpollable;
/// * `OwlDirect` — once per hypertableau derivation round and once per work item of the
///   consequence-based saturation, which are the boundaries its own step cap is charged at;
/// * `Rif` — once per semi-naive round of the RIF evaluator;
/// * `Simple` — before the copy. An identity closure runs no fixpoint, so there is no round
///   boundary inside it to take.
///
/// `stop` of `None` is exactly [`materialize`].
///
/// # Errors
///
/// [`EntailError::Stopped`] if the signal fired, plus every error [`materialize`] returns.
pub fn materialize_until<D: DatasetView>(
    ds: &D,
    plan: Materialization<'_>,
    stop: Option<&Arc<dyn StopSignal>>,
) -> Result<(Arc<RdfDataset>, ReasoningReport), EntailError> {
    if stop.is_some_and(|stop| stop.stopped()) {
        return Err(EntailError::Stopped);
    }
    let regime = plan.regime();
    let (closure, stats) = match plan {
        Materialization::Simple => (engine::copy_of(ds)?, report::RunStats::none()),
        Materialization::Rdf
        | Materialization::Rdfs
        | Materialization::OwlRl
        | Materialization::D => engine::close(ds, regime, stop)?,
        // The two query-directed lanes are DELEGATED, not restated: each already assembles
        // its own report, so returning here is what keeps one implementation per lane.
        Materialization::OwlDirect(query_bgp) => {
            return materialize_dl_reported_until(ds, query_bgp, stop);
        }
        Materialization::Rif(rules) => return materialize_rif_until(ds, rules, stop),
    };
    Ok((closure, ReasoningReport::of_run(ds, regime, &stats)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab::{
        OWL_SYMMETRICPROPERTY, OWL_TRANSITIVEPROPERTY, RDF_PROPERTY, RDF_TYPE, RDFS_SUBCLASSOF,
    };
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfTextDirection, TermRef, TermValue};

    fn iri(b: &mut RdfDatasetBuilder, s: &str) -> purrdf_core::TermId {
        b.intern_iri(s)
    }

    /// Build a dataset from `(s, p, o)` IRI triples in the default graph.
    fn dataset(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let s = iri(&mut b, s);
            let p = iri(&mut b, p);
            let o = iri(&mut b, o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    fn has(ds: &RdfDataset, s: &str, p: &str, o: &str) -> bool {
        ds.quad_refs().any(|q| {
            matches!(q.s, TermRef::Iri(si) if si == s)
                && matches!(q.p, TermRef::Iri(pi) if pi == p)
                && matches!(q.o, TermRef::Iri(oi) if oi == o)
        })
    }

    const A: &str = "http://example.org/A";
    const B: &str = "http://example.org/B";
    const C: &str = "http://example.org/C";
    const X: &str = "http://example.org/x";
    const Y: &str = "http://example.org/y";

    /// `owl:sameAs` — `eq-diff1`'s first premise.
    const OWL_SAMEAS: &str = "http://www.w3.org/2002/07/owl#sameAs";
    /// `owl:differentFrom` — what the one extension rule concludes.
    const OWL_DIFFERENTFROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";

    const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
    const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";

    #[test]
    fn rdfs_subclass_is_transitive_and_types_instances() {
        // A ⊑ B ⊑ C, x a A  ⇒  A ⊑ C, x a B, x a C.
        let ds = dataset(&[
            (A, RDFS_SUBCLASSOF, B),
            (B, RDFS_SUBCLASSOF, C),
            (X, RDF_TYPE, A),
        ]);
        let (closed, _report) = materialize(&ds, Materialization::Rdfs).expect("rdfs");
        assert!(
            has(&closed, A, RDFS_SUBCLASSOF, C),
            "subClassOf transitivity"
        );
        assert!(has(&closed, X, RDF_TYPE, B), "rdfs9 one hop");
        assert!(has(&closed, X, RDF_TYPE, C), "rdfs9 transitive typing");
    }

    #[test]
    fn rdfs_domain_and_range_type_endpoints() {
        // (p domain A),(p range B),(x p y) ⇒ (x a A),(y a B).
        let p = "http://example.org/p";
        let y = "http://example.org/y";
        let ds = dataset(&[(p, RDFS_DOMAIN, A), (p, RDFS_RANGE, B), (X, p, y)]);
        let (closed, _report) = materialize(&ds, Materialization::Rdfs).expect("rdfs");
        assert!(has(&closed, X, RDF_TYPE, A), "domain types subject");
        assert!(has(&closed, y, RDF_TYPE, B), "range types object");
    }

    #[test]
    fn owl_transitive_and_symmetric() {
        let p = "http://example.org/rel";
        let y = "http://example.org/y";
        let z = "http://example.org/z";
        let ds = dataset(&[
            (p, RDF_TYPE, OWL_TRANSITIVEPROPERTY),
            (p, RDF_TYPE, OWL_SYMMETRICPROPERTY),
            (X, p, y),
            (y, p, z),
        ]);
        let (closed, _report) = materialize(&ds, Materialization::OwlRl).expect("owl-rl");
        assert!(has(&closed, X, p, z), "transitive closure");
        assert!(has(&closed, y, p, X), "symmetric mirror");
        // RDFS-only must NOT apply the OWL rules.
        let (rdfs, _report) = materialize(&ds, Materialization::Rdfs).expect("rdfs");
        assert!(!has(&rdfs, X, p, z), "no transitive under RDFS regime");
    }

    /// EVERY inhabitant of [`Materialization`] materializes — the function is total.
    ///
    /// Falsifiable against the behaviour this replaced: `OwlDirect` and `Rif` returned
    /// `Err(Unsupported)` from this same call, on this same fixture. There is no longer an
    /// error variant for them to return, and the check that says so is not "the variant is
    /// gone" (which is a fact about the source) but "every plan answers with a closure
    /// whose report names the regime asked for" (which is a fact about the function).
    #[test]
    fn every_materialization_plan_answers() {
        let ds = dataset(&[(X, RDF_TYPE, A)]);
        let rules = RuleSet::new();
        for (plan, regime) in [
            (Materialization::Simple, Regime::Simple),
            (Materialization::Rdf, Regime::Rdf),
            (Materialization::Rdfs, Regime::Rdfs),
            (Materialization::OwlRl, Regime::OwlRl),
            (Materialization::D, Regime::D),
            (Materialization::OwlDirect(&[]), Regime::OwlDirect),
            (Materialization::Rif(&rules), Regime::Rif),
        ] {
            assert_eq!(plan.regime(), regime, "{regime:?}");
            let (closure, report) = materialize(&ds, plan).expect("every plan materializes");
            assert_eq!(report.regime(), regime, "{regime:?}");
            // Every lane carries the asserted data through; nothing is a stub that drops it.
            assert!(has(&closure, X, RDF_TYPE, A), "{regime:?}");
        }
    }

    /// `D` used to be refused for the same kind of reason the other two were, and is not
    /// any more: it is a rule table like any other.
    #[test]
    fn d_is_a_rule_table_lane_like_any_other() {
        let ds = dataset(&[(X, RDF_TYPE, A)]);
        // `D` is Simple entailment plus OWL 2 Profiles §4.3 Table 8, and it runs.
        let (closed, report) = materialize(&ds, Materialization::D).expect("d is materializable");
        assert_eq!(report.regime(), Regime::D);
        assert_eq!(rules(Regime::D).len(), 5);
        assert_eq!(implemented(Regime::D).len(), 5);
        // `dt-type1` is premise-free, so every supported datatype is typed in every `D`
        // closure — including the empty graph's.
        assert!(
            has(
                &closed,
                "http://www.w3.org/2001/XMLSchema#integer",
                RDF_TYPE,
                "http://www.w3.org/2000/01/rdf-schema#Datatype"
            ),
            "dt-type1 must type every datatype supported in OWL 2 RL"
        );
    }

    #[test]
    fn rdf_regime_types_predicates_as_property() {
        // Bare RDF entailment: the predicate of every triple is an rdf:Property
        // (rule `rdfD2`, spelled `rdf1` in RDF 1.0), even when the predicate is not
        // otherwise typed.
        let p = "http://example.org/ns#b";
        let y = "http://example.org/ns#c";
        let ds = dataset(&[(X, p, y)]);
        let (closed, _report) = materialize(&ds, Materialization::Rdf).expect("rdf");
        assert!(
            has(&closed, p, RDF_TYPE, RDF_PROPERTY),
            "predicate typed rdf:Property"
        );
        // Simple entailment must NOT derive it.
        let (simple, _report) = materialize(&ds, Materialization::Simple).expect("simple");
        assert!(
            !has(&simple, p, RDF_TYPE, RDF_PROPERTY),
            "no typing under Simple"
        );
    }

    #[test]
    fn rdfs_emission_order_is_deterministic() {
        // Each `close` call seeds a fresh, randomly-hashed `HashSet` of facts, so a
        // hash-order-dependent emission (the bug just fixed) would assign the novel
        // inferred vocabulary terms (e.g. `rdf:Property` from predicate typing) new
        // ids in different orders across two runs, diverging the id-sorted output.
        // A closure that introduces novel terms + an order-sensitive fingerprint of
        // the emitted quads therefore locks in the deterministic-emission contract.
        let p = "http://example.org/p";
        let q = "http://example.org/q";
        let y = "http://example.org/y";
        let input = &[
            (A, RDFS_SUBCLASSOF, B),
            (B, RDFS_SUBCLASSOF, C),
            (p, RDFS_DOMAIN, A),
            (p, RDFS_RANGE, B),
            (q, RDFS_DOMAIN, C),
            (X, p, y),
            (X, RDF_TYPE, A),
        ];
        let ds = dataset(input);

        // Two independently-seeded materializations of the SAME input.
        let (first, first_report) = materialize(&ds, Materialization::OwlRl).expect("owl-rl");
        let (second, second_report) = materialize(&ds, Materialization::OwlRl).expect("owl-rl");

        let fingerprint = |closed: &RdfDataset| -> Vec<String> {
            closed
                .quad_refs()
                .map(|q| format!("{:?}|{:?}|{:?}", q.s, q.p, q.o))
                .collect()
        };
        let fp_first = fingerprint(&first);
        let fp_second = fingerprint(&second);

        assert_eq!(
            fp_first, fp_second,
            "inferred-triple emission order must be deterministic across runs"
        );
        // The REPORT is deterministic for the same reason and by the same evidence: two
        // independently-seeded runs of one input must render identically, field for field.
        assert_eq!(
            format!("{first_report:?}"),
            format!("{second_report:?}"),
            "the reasoning report must be deterministic across runs"
        );
        // Prove inference actually happened, guarding against an empty-closure
        // false-positive (equal-but-trivial fingerprints).
        assert!(
            fp_first.len() > input.len(),
            "closure must derive novel triples for the guard to be meaningful"
        );
    }

    /// `owl:inverseOf` derives both directions, from the schema side and the data side.
    ///
    /// A golden by construction rather than by the engine: the closure of `p inverseOf q`
    /// over `(x p y)` and `(u q v)` is exactly the two mirrored triples, whichever premise
    /// arrives first. It guards the split of the inverse index into its `prp-inv1` and
    /// `prp-inv2` halves — a split that must move which RULE is credited and nothing else.
    #[test]
    fn inverse_of_mirrors_both_directions() {
        let p = "http://example.org/p";
        let q = "http://example.org/q";
        let y = "http://example.org/y";
        let u = "http://example.org/u";
        let v = "http://example.org/v";
        let ds = dataset(&[
            (p, "http://www.w3.org/2002/07/owl#inverseOf", q),
            (X, p, y),
            (u, q, v),
        ]);
        let (closed, report) = materialize(&ds, Materialization::OwlRl).expect("owl-rl");
        assert!(has(&closed, y, q, X), "prp-inv1 mirrors a p-triple into q");
        assert!(has(&closed, v, p, u), "prp-inv2 mirrors a q-triple into p");
        // Both halves are credited, under their own ids.
        let fired: Vec<&str> = report
            .rules_fired()
            .iter()
            .map(|&(rule, _)| rule.as_str())
            .collect();
        assert!(fired.contains(&"prp-inv1"), "{fired:?}");
        assert!(fired.contains(&"prp-inv2"), "{fired:?}");
        // A self-inverse property still mirrors, and still terminates.
        let selfish = dataset(&[(p, "http://www.w3.org/2002/07/owl#inverseOf", p), (X, p, y)]);
        let (closed, _) = materialize(&selfish, Materialization::OwlRl).expect("owl-rl");
        assert!(has(&closed, y, p, X));
    }

    // ── The reasoning report ────────────────────────────────────────────────────

    /// The four rule-table chase lanes, for the cross-cutting report invariants below.
    ///
    /// Not "the regimes `materialize` can run" — it runs all seven. These are the four
    /// whose whole input is a rule table this crate states, which is what makes the
    /// inventory arithmetic below (`rules(r)` minus `implemented(r)`) mean anything; the
    /// two query-directed lanes are defined by a caller's document and have no such table,
    /// and `D` is exercised on its own because it is the newest lane.
    const RUNNABLE: [Materialization<'static>; 4] = [
        Materialization::Simple,
        Materialization::Rdf,
        Materialization::Rdfs,
        Materialization::OwlRl,
    ];

    /// A fixture with enough schema to make every RDFS-lane rule fire at least once.
    fn schema_fixture() -> Arc<RdfDataset> {
        let p = "http://example.org/p";
        let q = "http://example.org/q";
        let y = "http://example.org/y";
        dataset(&[
            (A, RDFS_SUBCLASSOF, B),
            (B, RDFS_SUBCLASSOF, C),
            (A, RDF_TYPE, "http://www.w3.org/2000/01/rdf-schema#Class"),
            (p, RDF_TYPE, RDF_PROPERTY),
            (p, RDFS_DOMAIN, A),
            (p, RDFS_RANGE, B),
            (p, "http://www.w3.org/2000/01/rdf-schema#subPropertyOf", q),
            (X, p, y),
            (X, RDF_TYPE, A),
        ])
    }

    /// `completeness` is `rules(r)` minus `implemented(r)`, COMPUTED — and today's gap is
    /// additionally pinned so a later change that closes one has to say so here.
    #[test]
    fn completeness_is_derived_from_the_inventory_and_pinned() {
        let ds = schema_fixture();
        for plan in RUNNABLE {
            let regime = plan.regime();
            let (_, report) = materialize(&ds, plan).expect("runnable regime");
            // Computed, not asserted: the expected value is the inventory difference.
            let expected: Vec<RuleId> = rules(regime)
                .iter()
                .copied()
                .filter(|rule| !implemented(regime).contains(rule))
                .collect();
            assert_eq!(report.completeness().missing(), expected, "{regime:?}");
            assert_eq!(
                report.completeness().is_exact(),
                expected.is_empty(),
                "{regime:?}"
            );
            assert_eq!(report.regime(), regime);
        }

        // The ratchet. When a later change teaches the chase a rule these numbers MUST
        // fall, and this assertion is where that has to be acknowledged. Never widen it to
        // an inequality: "at most 66 missing" would pass forever without anyone noticing a
        // regression back up to 66.
        let gaps: Vec<(&str, usize)> = RUNNABLE
            .iter()
            .map(|&r| {
                let (_, report) = materialize(&ds, r).expect("a rule-table lane");
                (
                    match r.regime() {
                        Regime::Simple => "Simple",
                        Regime::Rdf => "RDF",
                        Regime::Rdfs => "RDFS",
                        _ => "OWL-RL",
                    },
                    report.completeness().missing().len(),
                )
            })
            .collect();
        assert_eq!(
            gaps,
            vec![("Simple", 0), ("RDF", 0), ("RDFS", 0), ("OWL-RL", 0)],
            "(regime, rules the regime defines that the chase does not fire)"
        );

        // `Simple` is exact because it has no rule table. `OWL-RL` is exact because the
        // chase fires all seventy-eight rules of Tables 4-9 — and it is exact WITHIN
        // BOUNDARIES, not flatly exact, because three of those rules quantify over all
        // literals and one concludes about literal subjects. The two are different claims
        // and the report makes both of them.
        let (_, simple) = materialize(&ds, Materialization::Simple).expect("simple");
        assert_eq!(simple.completeness(), Completeness::Exact);
        assert!(rules(Regime::Simple).is_empty());
        let (_, owl) = materialize(&ds, Materialization::OwlRl).expect("owl-rl");
        assert_eq!(
            owl.completeness(),
            Completeness::ExactWithinBoundaries,
            "a complete rule table beside a boundary is not a contradiction, and it is \
             not `Exact` either"
        );
        assert!(owl.completeness().is_exact());
        assert!(!owl.boundaries().is_empty());
    }

    /// The named missing rules are the right ones, not merely the right count — and for
    /// `OWL-RL` there are none left to name.
    #[test]
    fn the_missing_rules_are_named() {
        let ds = schema_fixture();
        let (_, report) = materialize(&ds, Materialization::OwlRl).expect("owl-rl");
        assert!(
            report.completeness().missing().is_empty(),
            "OWL 2 RL is complete: {:?}",
            report.completeness().missing()
        );
        // One rule from each of the six tables, named, so "complete" is checked against
        // the tables rather than against a count.
        for present in [
            RuleId::EqRef,
            RuleId::PrpTrp,
            RuleId::ClsSvf1,
            RuleId::CaxSco,
            RuleId::DtType1,
            RuleId::ScmSco,
        ] {
            assert!(implemented(Regime::OwlRl).contains(&present), "{present}");
        }

        // `RDFS` has NO gap left: the four rules that conclude about a fresh blank node
        // are evaluated by the restricted chase, which is the consumer the existential
        // head form was represented for. That is a claim about the RULE TABLE, and the
        // report makes the other claim separately — the surrogates those four invent do
        // not reach the answer, so the run is `ExactWithinBoundaries` and names the
        // `surrogate` boundary rather than saying `Exact`.
        let (_, rdfs) = materialize(&ds, Materialization::Rdfs).expect("rdfs");
        assert!(rdfs.completeness().missing().is_empty());
        assert_eq!(rdfs.completeness(), Completeness::ExactWithinBoundaries);
        for rule in [
            RuleId::RdfD1,
            RuleId::RdfD1a,
            RuleId::Rdfs14,
            RuleId::Rdfs14a,
        ] {
            assert!(implemented(Regime::Rdfs).contains(&rule), "{rule}");
        }
    }

    /// `Exact` NEVER sits beside a boundary — on every regime, over inputs chosen to trip
    /// every boundary the crate can emit.
    ///
    /// The absence of that property is what let plain "OWL 2 RL entailment" stand in the
    /// documentation of a twelve-rule chase. It is asserted here over the emitted reports
    /// rather than through a predicate the report carries about itself, so the check does
    /// not depend on that predicate being right.
    #[test]
    fn no_emitted_report_says_exact_beside_a_boundary() {
        for ds in [
            schema_fixture(),
            triple_term_fixture(),
            named_graph_fixture(),
            literal_object_fixture(),
            dataset(&[]),
        ] {
            for plan in RUNNABLE {
                let regime = plan.regime();
                let (_, report) = materialize(&ds, plan).expect("runnable regime");
                assert!(
                    report.completeness() != Completeness::Exact || report.boundaries().is_empty(),
                    "{regime:?} reported Exact alongside {:?}",
                    report.boundaries()
                );
            }
        }
    }

    /// THE CONTRADICTION HAS NO CONSTRUCTOR, which is why nothing checks for it.
    ///
    /// Two earlier revisions policed the state instead. The first narrowed
    /// [`Completeness::Exact`] inside the constructor and kept a `ReasoningReport::overclaims`
    /// predicate over the narrowed field, making every assertion of it a tautology. The
    /// second moved the narrowing to [`Completeness::for_run`] and had the three emission
    /// paths check the assembled report — but each of them called `for_run` with the very
    /// boundary list it then stored, so the check could not fail, `EntailError::Overclaim`
    /// was unreachable, and the only test that ever saw the contradiction hand-built it
    /// with a constructor production never used that way.
    ///
    /// [`ReasoningReport`] now stores no completeness at all: [`ReasoningReport::completeness`]
    /// derives it from the report's own regime and boundary list. This test is the proof
    /// that the derivation leaves no bad case — it ranges over every regime and every
    /// non-empty boundary set of size one and two, plus the empty one, and asks the
    /// constructor for a report each time. `Exact` appears only where the boundary list is
    /// empty, and there is no argument that could have made it appear anywhere else.
    #[test]
    fn boundaries_beside_exact_is_unconstructible() {
        let budget = purrdf_datalog::seminaive::BudgetReport::new(0, 0, 0);
        let build = |regime: Regime, boundaries: Vec<Boundary>| {
            ReasoningReport::new(regime, Vec::new(), boundaries, budget, None, 0, None)
        };
        for regime in [
            Regime::Simple,
            Regime::Rdf,
            Regime::Rdfs,
            Regime::OwlRl,
            Regime::D,
            Regime::OwlDirect,
            Regime::Rif,
        ] {
            // The empty boundary list is the ONLY input under which `Exact` is reachable,
            // and it is reachable there for every regime whose rule table is complete.
            assert!(
                build(regime, Vec::new()).boundaries().is_empty(),
                "{regime:?}"
            );
            for first in Construct::ALL {
                let one = build(regime, vec![Boundary::of(first)]);
                assert_ne!(
                    one.completeness(),
                    Completeness::Exact,
                    "{regime:?} {first}"
                );
                assert!(!one.boundaries().is_empty(), "{regime:?} {first}");
                for second in Construct::ALL {
                    let two = build(regime, vec![Boundary::of(first), Boundary::of(second)]);
                    assert_ne!(
                        two.completeness(),
                        Completeness::Exact,
                        "{regime:?} {first} {second}"
                    );
                }
            }
        }
    }

    /// An inconsistent input gets its CERTIFICATE, not just its witness.
    ///
    /// The refusal used to carry an [`InconsistencyWitness`] alone, which made
    /// [`ReasoningReport::inconsistency`] a field nothing could ever populate — `none` on
    /// every report on every host — and made the inconsistent run the one report-free call
    /// this crate says it does not have. Both halves are checked here: the report exists,
    /// and its `inconsistency` is the witness.
    #[test]
    fn an_inconsistent_run_still_returns_its_report() {
        let disjoint = "http://www.w3.org/2002/07/owl#disjointWith";
        let ds = dataset(&[(A, disjoint, B), (X, RDF_TYPE, A), (X, RDF_TYPE, B)]);
        let Err(EntailError::Inconsistent(run)) = materialize(&ds, Materialization::OwlRl) else {
            panic!("two disjoint classes with a shared instance is `cax-dw`");
        };
        let report = run.report();

        // The field that was previously a constant.
        assert_eq!(
            report.inconsistency().map(InconsistencyWitness::rule),
            Some(RuleId::CaxDw)
        );
        // Everything else a caller needs to act on the refusal, measured rather than
        // stubbed: the calculus that refused, the constructs the run met, and the cost.
        assert_eq!(report.regime(), Regime::OwlRl);
        assert!(!report.boundaries().is_empty());
        assert!(report.budget().join_steps() > 0, "the evaluation did work");
        // A consistent run over the same shape reports the absence, so `None` is a
        // finding rather than the only state the field has.
        let (_, consistent) = materialize(&dataset(&[(X, RDF_TYPE, A)]), Materialization::OwlRl)
            .expect("a consistent knowledge base");
        assert_eq!(consistent.inconsistency(), None);
    }

    /// The four surrogate rules are OBSERVABLE: a datatyped literal makes the count move.
    ///
    /// `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` fire, and their conclusions are withheld
    /// because a SPARQL entailment regime draws its answers from the scoping graph — so
    /// they can never appear in `rules_fired`, and this counter is the only evidence a
    /// caller has that they ran at all. A constant zero here would make six "implemented"
    /// rules unobservable from outside Rust.
    #[test]
    fn a_datatyped_literal_makes_the_withheld_surrogate_count_move() {
        let ds = literal_object_fixture();
        for plan in [Materialization::Rdf, Materialization::Rdfs] {
            let regime = plan.regime();
            let (_, report) = materialize(&ds, plan).expect("a surrogate-minting lane");
            assert!(
                report.withheld_surrogates() > 0,
                "{regime:?}: rdfD1/rdfD1a fired over the datatyped literal, so their \
                 withheld conclusions must be counted"
            );
            // And the count is what raises the boundary, so the two agree.
            assert!(
                report
                    .boundaries()
                    .iter()
                    .any(|b| b.construct() == Construct::Surrogate),
                "{regime:?}"
            );
        }
        // The lanes that state none of the four withhold nothing — the count is a
        // measurement of THIS run, not a standing disclaimer.
        for plan in [
            Materialization::Simple,
            Materialization::OwlRl,
            Materialization::D,
        ] {
            let regime = plan.regime();
            let (_, report) = materialize(&ds, plan).expect("a lane that mints no surrogate");
            assert_eq!(report.withheld_surrogates(), 0, "{regime:?}");
        }
    }

    /// THE EXTENSION FIRES, AND IT IS REPORTED AS AN EXTENSION.
    ///
    /// `a owl:differentFrom b` entails `b owl:differentFrom a` — W3C publishes it as
    /// `webont-differentfrom-001` — and no rule of OWL 2 Profiles §4.3 Tables 4–9 has an
    /// `owl:differentFrom` head, so the chase reaches it only through the extension this
    /// crate declares. This asserts BOTH halves of that sentence at once: the triple is in
    /// the closure, the rule that put it there is named, and the report says that rule is
    /// not one of the seventy-eight.
    #[test]
    fn the_different_from_extension_fires_and_is_labelled_as_one() {
        let ds = dataset(&[(X, OWL_DIFFERENTFROM, Y)]);
        let (closed, report) = materialize(&ds, Materialization::OwlRl).expect("a consistent run");

        // The conclusion.
        assert!(
            has(&closed, Y, OWL_DIFFERENTFROM, X),
            "the symmetric triple is not in the closure"
        );
        // …credited to the extension, and to nothing else.
        let fired: Vec<(RuleId, u64)> = report.rules_fired().to_vec();
        assert!(
            fired.contains(&(RuleId::ExtEqDiffSym, 1)),
            "the symmetric triple was not credited to ext-eq-diff-sym: {fired:?}"
        );
        // …and labelled. A caller wanting strictly normative behaviour reads exactly this.
        assert_eq!(report.extensions(), &[RuleId::ExtEqDiffSym]);
        for (rule, _) in &fired {
            assert_eq!(
                rule.is_extension(),
                report.extensions().contains(rule),
                "{rule} is labelled inconsistently with the report's extension list"
            );
        }
        // THE NORMATIVE STATEMENT IS UNMOVED. The extension is in neither inventory, so
        // `78 / 78` is still a claim about Tables 4-9 and about nothing else.
        assert_eq!(rules(Regime::OwlRl).len(), 78);
        assert_eq!(implemented(Regime::OwlRl).len(), 78);
        assert!(!rules(Regime::OwlRl).contains(&RuleId::ExtEqDiffSym));
        assert!(!implemented(Regime::OwlRl).contains(&RuleId::ExtEqDiffSym));
        // Every OTHER rule the run fired IS in the normative table (or is one of the three
        // RDFS-shaped rules OWL 2 RL/RDF omits from its own tables and this lane fires).
        for (rule, _) in &fired {
            if rule.is_extension() {
                continue;
            }
            assert!(
                implemented(Regime::OwlRl).contains(rule) || rules(Regime::Rdfs).contains(rule),
                "{rule} is neither normative nor declared an extension"
            );
        }
        // And no other lane gets it: `RDFS` says nothing about `owl:differentFrom`.
        let (rdfs, rdfs_report) = materialize(&ds, Materialization::Rdfs).expect("a closure");
        assert!(!has(&rdfs, Y, OWL_DIFFERENTFROM, X));
        assert!(rdfs_report.extensions().is_empty());
    }

    /// THE EXTENSION REFUSES NOTHING THE TABLE DID NOT ALREADY REFUSE.
    ///
    /// The only rules that read `owl:differentFrom` in a BODY are `eq-diff1..3`, and
    /// `eq-diff1` pairs it with `owl:sameAs` — which `eq-sym` already closes. So a clash
    /// the symmetric triple enables was reachable without it, and this is the pair of runs
    /// that shows it: `x sameAs y` with `x differentFrom y` refuses, and so does the
    /// mirror image, exactly as it did before the extension existed.
    #[test]
    fn the_different_from_extension_decides_no_new_run() {
        for (same, different) in [((X, Y), (X, Y)), ((X, Y), (Y, X)), ((Y, X), (X, Y))] {
            let ds = dataset(&[
                (same.0, OWL_SAMEAS, same.1),
                (different.0, OWL_DIFFERENTFROM, different.1),
            ]);
            let refusal = materialize(&ds, Materialization::OwlRl)
                .expect_err("same-and-different is an inconsistency");
            let EntailError::Inconsistent(run) = refusal else {
                panic!("expected an inconsistency for {same:?}/{different:?}");
            };
            assert_eq!(run.witness().rule(), RuleId::EqDiff1);
        }
        // And a graph that only says two things are different still CLOSES.
        let (_, report) = materialize(
            &dataset(&[(X, OWL_DIFFERENTFROM, Y)]),
            Materialization::OwlRl,
        )
        .expect("difference alone is consistent");
        assert_eq!(report.inconsistency(), None);
    }

    /// THE TERMINATION CERTIFICATE REACHES THE REPORT, AND IT IS NOT ONE CONSTANT.
    ///
    /// The chase proves weak acyclicity before it runs a round and used to discard the
    /// proof. It is carried now — for the two lanes that need one, and for no others,
    /// which is the honest split: a program that invents no term has no obligation to
    /// discharge, and rendering a proof for it would be a claim about an analysis that
    /// never ran.
    ///
    /// The two certified lanes do NOT agree, which is the fact worth pinning: the
    /// certificate is a function of the clause set, so `RDFS` (four existential rules)
    /// proves more than `RDF` (two), and a line that read the same for both would be
    /// carrying no information.
    #[test]
    fn the_termination_certificate_is_reported_where_a_chase_ran() {
        let ds = literal_object_fixture();
        let rdf = materialize(&ds, Materialization::Rdf).expect("closed").1;
        let rdfs = materialize(&ds, Materialization::Rdfs).expect("closed").1;
        let rdf_certificate = rdf.termination().expect("the RDF lane is chased");
        let rdfs_certificate = rdfs.termination().expect("the RDFS lane is chased");
        assert!(rdf_certificate.existential_edges() > 0);
        assert!(rdf_certificate.positions() > 0);
        assert_ne!(
            rdf_certificate, rdfs_certificate,
            "the certificate is a function of the CLAUSE SET, so two different rule tables \
             must not prove the same thing"
        );
        assert!(
            rdfs_certificate.existential_edges() > rdf_certificate.existential_edges(),
            "RDFS states rdfs14/rdfs14a besides rdfD1/rdfD1a"
        );
        // It is `purrdf-datalog`'s own sentence, not a second spelling of it.
        assert_eq!(
            rdf_certificate.to_string(),
            format!(
                "weakly acyclic: {} refined position(s), {} existential edge(s), none in a cycle",
                rdf_certificate.positions(),
                rdf_certificate.existential_edges()
            )
        );
        // It does NOT vary with the data — a certificate that moved per input would be
        // describing the run rather than the program that admitted it.
        let other = materialize(&dataset(&[(X, RDF_TYPE, A)]), Materialization::Rdfs)
            .expect("closed")
            .1;
        assert_eq!(other.termination(), Some(rdfs_certificate));
        // And the lanes that invent no term report no certificate at all.
        for plan in [
            Materialization::Simple,
            Materialization::OwlRl,
            Materialization::D,
        ] {
            let regime = plan.regime();
            let report = materialize(&ds, plan).expect("closed").1;
            assert_eq!(report.termination(), None, "{regime:?}");
        }
    }

    /// A dataset whose object position holds an RDF 1.2 triple term.
    fn triple_term_fixture() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(X);
        let p = b.intern_iri("http://example.org/says");
        let inner_s = b.intern_iri(A);
        let inner_p = b.intern_iri(RDFS_SUBCLASSOF);
        let inner_o = b.intern_iri(B);
        let quoted = b.intern_triple(inner_s, inner_p, inner_o);
        b.push_quad(s, p, quoted, None);
        let sub = b.intern_iri(RDFS_SUBCLASSOF);
        b.push_quad(inner_s, sub, inner_o, None);
        b.freeze().expect("freeze")
    }

    /// A dataset with a quad outside the default graph.
    fn named_graph_fixture() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(X);
        let ty = b.intern_iri(RDF_TYPE);
        let a = b.intern_iri(A);
        let g = b.intern_iri("http://example.org/g");
        b.push_quad(s, ty, a, None);
        b.push_quad(s, ty, a, Some(g));
        b.freeze().expect("freeze")
    }

    /// A dataset where a ranged property points at a LITERAL, so `rdfs3` would have to
    /// conclude into subject position and cannot.
    fn literal_object_fixture() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(X);
        let p = b.intern_iri("http://example.org/label");
        let rng = b.intern_iri(RDFS_RANGE);
        let a = b.intern_iri(A);
        let literal = b.intern_literal(purrdf_core::RdfLiteral::typed(
            "cat",
            "http://www.w3.org/2001/XMLSchema#string",
        ));
        b.push_quad(p, rng, a, None);
        b.push_quad(s, p, literal, None);
        b.freeze().expect("freeze")
    }

    /// Boundaries are not decorative: a real RL-lane construct emits one, with a reason.
    #[test]
    fn boundaries_are_emitted_for_real_constructs() {
        let has = |ds: &RdfDataset, plan: Materialization<'_>, construct: Construct| {
            let (_, report) = materialize(ds, plan).expect("runnable regime");
            report
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == construct)
        };

        // A triple term the chase cannot look inside — the RL lane's own boundary, not
        // the DL lane's.
        let quoted = triple_term_fixture();
        assert!(has(&quoted, Materialization::OwlRl, Construct::TripleTerm));
        assert!(has(&quoted, Materialization::Rdfs, Construct::TripleTerm));
        // …and the plain fixture, which has none, does not claim one.
        assert!(!has(
            &schema_fixture(),
            Materialization::OwlRl,
            Construct::TripleTerm
        ));

        // A quad outside the default graph.
        assert!(has(
            &named_graph_fixture(),
            Materialization::OwlRl,
            Construct::NamedGraph
        ));
        assert!(!has(
            &schema_fixture(),
            Materialization::OwlRl,
            Construct::NamedGraph
        ));

        // A conclusion that would need a literal in subject position.
        assert!(has(
            &literal_object_fixture(),
            Materialization::Rdfs,
            Construct::GeneralizedRdf
        ));
        assert!(!has(
            &schema_fixture(),
            Materialization::Rdfs,
            Construct::GeneralizedRdf
        ));

        // The two inherent boundaries hold for every input of their lane.
        assert!(has(
            &dataset(&[]),
            Materialization::Rdfs,
            Construct::DatatypeValueSpace
        ));
        assert!(has(
            &dataset(&[]),
            Materialization::Rdfs,
            Construct::AxiomaticTriples
        ));
        // OWL 2 RL/RDF omits the RDF/RDFS axiomatic triples, so its lane does not meet
        // that one.
        assert!(!has(
            &dataset(&[]),
            Materialization::OwlRl,
            Construct::AxiomaticTriples
        ));
        // `Simple` copies faithfully, so it meets none of them — which is what makes its
        // `Exact` honest.
        let (_, simple) = materialize(&quoted, Materialization::Simple).expect("simple");
        assert!(simple.boundaries().is_empty());

        // Every boundary carries a technical reason naming what it blocks.
        let (_, report) = materialize(&quoted, Materialization::Rdfs).expect("rdfs");
        assert!(!report.boundaries().is_empty());
        for boundary in report.boundaries() {
            assert!(!boundary.reason().is_empty());
            assert_eq!(boundary.reason(), boundary.construct().reason());
        }
        // In `Construct` declaration order, deduplicated.
        let constructs: Vec<Construct> = report
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct())
            .collect();
        let mut sorted = constructs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(constructs, sorted);
    }

    /// `rules_fired` names rules that really fired, in specification table order, and its
    /// counts sum to exactly the number of inferred triples.
    #[test]
    fn rules_fired_is_ordered_attributed_and_adds_up() {
        let ds = schema_fixture();
        for plan in RUNNABLE {
            let regime = plan.regime();
            let (closed, report) = materialize(&ds, plan).expect("runnable regime");
            let fired = report.rules_fired();

            // Specification table order, no repeats, no zero entries.
            let mut previous: Option<RuleId> = None;
            for &(rule, count) in fired {
                assert!(count > 0, "{regime:?} listed {rule} with a zero count");
                if let Some(previous) = previous {
                    assert!(
                        previous < rule,
                        "{regime:?} is out of table order at {rule}"
                    );
                }
                previous = Some(rule);
                // Every named rule is one the regime implements, or — under OWL-RL only —
                // one of the three RDFS-shaped rules that lane fires under no OWL name.
                assert!(
                    implemented(regime).contains(&rule)
                        || calculus::is_rdfs_shaped_extra(regime, rule),
                    "{regime:?} credited {rule}, which it does not implement"
                );
            }

            // The counts are conclusions COMMITTED, so they sum to the inferred triples.
            let inferred = closed.quad_refs().count() - ds.quad_refs().count();
            let total: u64 = fired.iter().map(|&(_, count)| count).sum();
            assert_eq!(
                usize::try_from(total).expect("count fits usize"),
                inferred,
                "{regime:?}: per-rule counts must sum to the inferred triples"
            );
        }

        // `Simple` infers nothing, so nothing fired — an empty list, not a zeroed one.
        let (_, simple) = materialize(&ds, Materialization::Simple).expect("simple");
        assert!(simple.rules_fired().is_empty());

        // The OWL-RL lane really does credit the three RDFS-shaped rules by their RDFS
        // names, which is the honest reading of what it fires.
        let (_, owl) = materialize(&ds, Materialization::OwlRl).expect("owl-rl");
        let names: Vec<&str> = owl.rules_fired().iter().map(|&(r, _)| r.as_str()).collect();
        assert!(names.contains(&"rdfs6"), "{names:?}");
        assert!(names.contains(&"cax-sco"), "{names:?}");
        assert!(!names.contains(&"rdfs9"), "the OWL lane uses the OWL name");
    }

    /// The report's contract hash is `purrdf-datalog`'s over this crate's declared
    /// calculus — recomputable by a consumer, and different for different rule sets.
    #[test]
    fn the_contract_hash_names_the_calculus() {
        let ds = schema_fixture();
        let mut seen = Vec::new();
        for plan in RUNNABLE {
            let regime = plan.regime();
            let (_, report) = materialize(&ds, plan).expect("runnable regime");
            assert_eq!(
                report.contract_hash(),
                purrdf_datalog::cache::contract_hash(&calculus_program(regime)),
                "{regime:?}"
            );
            seen.push((regime, report.contract_hash()));
        }
        // The three rule-bearing lanes are three different calculi.
        assert_ne!(seen[1].1, seen[2].1);
        assert_ne!(seen[2].1, seen[3].1);
        assert_ne!(seen[1].1, seen[3].1);
        // The hash is a property of the CALCULUS, not of the data it ran over.
        let (_, other) = materialize(&triple_term_fixture(), Materialization::Rdfs).expect("rdfs");
        assert_eq!(other.contract_hash(), seen[2].1);
    }

    /// The budget report carries real measurements, and `Simple` — which evaluates
    /// nothing — reports zero for all three.
    #[test]
    fn the_budget_reports_what_the_run_consumed() {
        let ds = schema_fixture();
        let (_, simple) = materialize(&ds, Materialization::Simple).expect("simple");
        assert_eq!(simple.budget().join_steps(), 0);
        assert_eq!(simple.budget().stored_facts(), 0);
        assert_eq!(simple.budget().term_arena_bytes(), 0);

        let (_, rdfs) = materialize(&ds, Materialization::Rdfs).expect("rdfs");
        assert!(
            rdfs.budget().join_steps() > 0,
            "the chase enumerated nothing"
        );
        assert!(
            rdfs.budget().stored_facts() >= ds.quad_refs().count(),
            "the store holds at least the seeded facts"
        );
        assert!(rdfs.budget().term_arena_bytes() > 0);
        // A candidate is enumerated for every committed conclusion and then some, so the
        // step count bounds the conclusion count.
        let committed: u64 = rdfs.rules_fired().iter().map(|&(_, n)| n).sum();
        assert!(rdfs.budget().join_steps() >= committed);
    }

    /// A CONSISTENT run reports no inconsistency, and that is now a CHECKED fact rather
    /// than a vacuous one.
    ///
    /// Before the seventeen `false`-headed rules were wired, `inconsistency() == None`
    /// meant "nothing looked"; it now means "seventeen rules looked and found nothing",
    /// which is the difference between an unchecked field and evidence.
    #[test]
    fn a_consistent_run_reports_no_inconsistency() {
        let ds = schema_fixture();
        for plan in RUNNABLE {
            let regime = plan.regime();
            let (_, report) = materialize(&ds, plan).expect("runnable regime");
            assert!(report.inconsistency().is_none(), "{regime:?}");
        }
        // And every rule that could have found one really is in the lane's rule set.
        for rule in [
            RuleId::EqDiff1,
            RuleId::PrpIrp,
            RuleId::PrpAsyp,
            RuleId::PrpPdw,
            RuleId::ClsNothing2,
            RuleId::ClsCom,
            RuleId::CaxDw,
            RuleId::DtNotType,
        ] {
            assert!(implemented(Regime::OwlRl).contains(&rule), "{rule}");
        }
    }

    /// AN INCONSISTENCY IS A REFUSAL, AND IT CARRIES ITS WITNESS.
    ///
    /// This is the behaviour change the seventeen `false`-headed rules bring: ONE
    /// `owl:disjointWith` violation turns a materialization from "returns answers" into an
    /// error. Correct — an inconsistent knowledge base entails every triple, so a closure
    /// over it answers a question nobody asked — and unusable without evidence, which is
    /// why the witness is carried rather than offered.
    #[test]
    fn a_disjointness_violation_is_a_refusal_with_a_witness() {
        let disjoint = "http://www.w3.org/2002/07/owl#disjointWith";
        let ds = dataset(&[(A, disjoint, B), (X, RDF_TYPE, A), (X, RDF_TYPE, B)]);

        let Err(EntailError::Inconsistent(run)) = materialize(&ds, Materialization::OwlRl) else {
            panic!("two disjoint classes with a shared instance is `cax-dw`");
        };
        let witness = run.witness();
        // The refusal carries the RUN, not the witness alone: the report describes what the
        // evaluation had done when it stopped, and its `inconsistency` IS this witness.
        assert_eq!(run.report().regime(), Regime::OwlRl);
        assert_eq!(run.report().inconsistency(), Some(witness));
        assert_eq!(
            run.report().contract_hash(),
            purrdf_datalog::cache::contract_hash(&calculus_program(Regime::OwlRl))
        );
        assert_eq!(witness.rule(), RuleId::CaxDw);
        // The premises are the specification's own, in the specification's own order.
        let premises: Vec<(TermValue, TermValue, TermValue)> = witness
            .premises()
            .iter()
            .map(|t| {
                (
                    t.subject().clone(),
                    t.predicate().clone(),
                    t.object().clone(),
                )
            })
            .collect();
        assert_eq!(
            premises,
            vec![
                (
                    TermValue::iri(A),
                    TermValue::iri(disjoint),
                    TermValue::iri(B)
                ),
                (
                    TermValue::iri(X),
                    TermValue::iri(RDF_TYPE),
                    TermValue::iri(A)
                ),
                (
                    TermValue::iri(X),
                    TermValue::iri(RDF_TYPE),
                    TermValue::iri(B)
                ),
            ]
        );
        // The witness names the graph whose CLOSURE refused. This fixture is a default-graph
        // dataset, so it is that one, and `None` IS the default graph.
        assert!(witness.graph().is_none());
        // The message names the rule, so a caller who only logs the error still learns
        // which axiom their data broke.
        let rendered = EntailError::Inconsistent(run).to_string();
        assert!(rendered.contains("cax-dw"), "{rendered}");

        // The RDFS lane says nothing about `owl:disjointWith`, so the same graph is
        // ordinary data there and closes without complaint. An inconsistency is a property
        // of a CALCULUS and a graph, never of a graph alone.
        assert!(materialize(&ds, Materialization::Rdfs).is_ok());
    }

    /// The witness is DETERMINISTIC: the same input names the same rule and the same
    /// premises on every run, which is what makes it usable in a golden.
    #[test]
    fn the_inconsistency_witness_is_deterministic() {
        let ds = dataset(&[
            (
                "http://example.org/irreflexive",
                RDF_TYPE,
                "http://www.w3.org/2002/07/owl#IrreflexiveProperty",
            ),
            (X, "http://example.org/irreflexive", X),
        ]);
        let render = || {
            let Err(EntailError::Inconsistent(run)) = materialize(&ds, Materialization::OwlRl)
            else {
                panic!("an irreflexive property relating something to itself is `prp-irp`");
            };
            // The whole refusal, report included: a budget or a fired-rule tally that
            // wobbled between runs would show up here as two different strings.
            format!("{run:?}")
        };
        assert_eq!(render(), render());
        let Err(EntailError::Inconsistent(run)) = materialize(&ds, Materialization::OwlRl) else {
            unreachable!("just asserted")
        };
        let witness = run.witness();
        assert_eq!(witness.rule(), RuleId::PrpIrp);
        assert_eq!(witness.premises().len(), 2);
    }

    // ── The defined dataset semantics ───────────────────────────────────────────

    /// Build a dataset from `(s, p, o, graph)` IRI quads.
    fn quads(rows: &[(&str, &str, &str, Option<&str>)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o, g) in rows {
            let s = iri(&mut b, s);
            let p = iri(&mut b, p);
            let o = iri(&mut b, o);
            let g = g.map(|g| iri(&mut b, g));
            b.push_quad(s, p, o, g);
        }
        b.freeze().expect("freeze")
    }

    /// Whether `ds` holds `(s, p, o)` in `graph`.
    fn has_in(ds: &RdfDataset, s: &str, p: &str, o: &str, graph: Option<&str>) -> bool {
        ds.quad_refs().any(|q| {
            matches!(q.s, TermRef::Iri(si) if si == s)
                && matches!(q.p, TermRef::Iri(pi) if pi == p)
                && matches!(q.o, TermRef::Iri(oi) if oi == o)
                && match (q.g, graph) {
                    (None, None) => true,
                    (Some(TermRef::Iri(gi)), Some(want)) => gi == want,
                    _ => false,
                }
        })
    }

    /// A fixture named graph.
    const G: &str = "http://example.org/g";
    /// A SECOND fixture named graph — the sibling a named graph must never join with.
    const H: &str = "http://example.org/h";

    /// THE LAYOUT THE SEMANTICS EXISTS FOR: schema in the default graph, instances in a
    /// named graph, conclusions in the NAMED graph.
    ///
    /// And the two things that must NOT happen beside it: the conclusion does not appear in
    /// the default graph, which holds no instance, and moving the schema into a sibling
    /// named graph loses the conclusion entirely.
    #[test]
    fn a_named_graph_is_closed_against_itself_and_the_default_graph() {
        let ds = quads(&[(A, RDFS_SUBCLASSOF, B, None), (X, RDF_TYPE, A, Some(G))]);
        for plan in [Materialization::Rdfs, Materialization::OwlRl] {
            let regime = plan.regime();
            let (closed, report) = materialize(&ds, plan).expect("runnable regime");
            assert!(
                has_in(&closed, X, RDF_TYPE, B, Some(G)),
                "{regime:?}: the default graph's terminology did not reach the named \
                 graph's instance"
            );
            assert!(
                !has_in(&closed, X, RDF_TYPE, B, None),
                "{regime:?}: a named graph's conclusion reached the default graph"
            );
            // The boundary is what says this is a DEFINED choice rather than a derived one.
            assert!(
                report
                    .boundaries()
                    .iter()
                    .any(|b| b.construct() == Construct::NamedGraph),
                "{regime:?}"
            );
        }

        // THE CROSS-GRAPH JOIN THAT MUST NOT HAPPEN. One term moves — the terminology goes
        // into a sibling named graph — and the conclusion is drawn in no graph at all.
        let split = quads(&[(A, RDFS_SUBCLASSOF, B, Some(H)), (X, RDF_TYPE, A, Some(G))]);
        for plan in [Materialization::Rdfs, Materialization::OwlRl] {
            let regime = plan.regime();
            let (closed, _) = materialize(&split, plan).expect("runnable regime");
            for graph in [None, Some(G), Some(H)] {
                assert!(
                    !has_in(&closed, X, RDF_TYPE, B, graph),
                    "{regime:?}: two named graphs joined, into {graph:?}"
                );
            }
            // The sibling WAS closed, so the absence above is the missing join rather than
            // a lane that stopped reasoning.
            assert!(
                closed.quad_refs().count() > split.quad_refs().count(),
                "{regime:?}: nothing at all was derived"
            );
        }
    }

    /// A conclusion the DEFAULT graph draws on its own is not restated in a named graph that
    /// also reached it.
    ///
    /// `D` is the sharpest witness available: its whole rule table is Table 8, `dt-type1` is
    /// premise-free, and the other four conclude only literal subjects. Every graph's run
    /// therefore draws exactly the same thirty-two `rdfs:Datatype` typings, and the closure
    /// of a two-graph dataset must hold thirty-two of them and not sixty-four.
    #[test]
    fn a_default_graph_conclusion_is_not_restated_in_a_named_graph() {
        let ds = quads(&[(A, RDFS_SUBCLASSOF, B, None), (X, RDF_TYPE, A, Some(G))]);
        let (closed, _) = materialize(&ds, Materialization::D).expect("d");
        let datatype = "http://www.w3.org/2000/01/rdf-schema#Datatype";
        let typings = closed
            .quad_refs()
            .filter(|q| matches!(q.o, TermRef::Iri(o) if o == datatype))
            .count();
        assert_eq!(typings, 32, "dt-type1 typed the datatypes once per graph");
        assert!(
            closed
                .quad_refs()
                .all(|q| !matches!(q.o, TermRef::Iri(o) if o == datatype) || q.g.is_none())
        );
    }

    /// THE COST OF THE SEMANTICS IS MEASURED, NOT HIDDEN — and the three coordinates are
    /// aggregated under their own meanings.
    ///
    /// `join_steps` is WORK and sums across the `1 + n` evaluations; `stored_facts` and
    /// `term_arena_bytes` are OCCUPANCY of one store, each evaluation gets its own, so they
    /// report the PEAK. Summing the occupancy coordinates would name a footprint that never
    /// existed at any instant, and reporting one graph's slice of the work would understate
    /// exactly the cost this semantics adds.
    #[test]
    fn the_budget_sums_the_work_and_peaks_the_occupancy() {
        let one_graph = quads(&[(A, RDFS_SUBCLASSOF, B, None)]);
        let two_graphs = quads(&[(A, RDFS_SUBCLASSOF, B, None), (X, RDF_TYPE, A, Some(G))]);
        let (_, single) = materialize(&one_graph, Materialization::Rdfs).expect("rdfs");
        let (_, dual) = materialize(&two_graphs, Materialization::Rdfs).expect("rdfs");

        // Two evaluations of a program whose seed differs by one quad: the work roughly
        // doubles, and it is REPORTED as the total rather than as one lane's share.
        assert!(
            dual.budget().join_steps() > single.budget().join_steps() * 3 / 2,
            "the second evaluation's work is missing from the budget: {} vs {}",
            dual.budget().join_steps(),
            single.budget().join_steps()
        );
        // The occupancy is a PEAK, so it grows by the extra graph's own facts and nowhere
        // near doubles. Anything at or above the sum would mean the coordinate was summed.
        assert!(
            dual.budget().stored_facts() < single.budget().stored_facts() * 2,
            "an occupancy coordinate was summed: {} vs {}",
            dual.budget().stored_facts(),
            single.budget().stored_facts()
        );
        assert!(dual.budget().stored_facts() >= single.budget().stored_facts());
        assert!(dual.budget().term_arena_bytes() < single.budget().term_arena_bytes() * 2);
    }

    /// An inconsistency found while closing a NAMED graph names that graph.
    ///
    /// The premise pair is split across the default graph and `g` — which is exactly what
    /// makes the run refuse, since neither graph is inconsistent on its own — so this also
    /// pins that a named graph really is closed against the union rather than against
    /// itself.
    #[test]
    fn an_inconsistency_in_a_named_graph_names_that_graph() {
        let disjoint = "http://www.w3.org/2002/07/owl#disjointWith";
        let ds = quads(&[
            (A, disjoint, B, None),
            (X, RDF_TYPE, A, Some(G)),
            (X, RDF_TYPE, B, Some(G)),
        ]);
        let Err(EntailError::Inconsistent(run)) = materialize(&ds, Materialization::OwlRl) else {
            panic!("the union of the default graph and g is inconsistent under cax-dw");
        };
        let witness = run.witness();
        assert_eq!(witness.rule(), RuleId::CaxDw);
        assert_eq!(witness.graph(), Some(&TermValue::iri(G)));

        // The same three triples with the two typings in DIFFERENT named graphs is
        // consistent, because neither union holds both.
        let split = quads(&[
            (A, disjoint, B, None),
            (X, RDF_TYPE, A, Some(G)),
            (X, RDF_TYPE, B, Some(H)),
        ]);
        assert!(materialize(&split, Materialization::OwlRl).is_ok());
    }

    /// An ILL-TYPED LITERAL is an inconsistency under `D` as well as under `OWL-RL`, and
    /// it is `dt-not-type` that says so.
    ///
    /// This is the second half of the behaviour change: ordinary dirty data — one literal
    /// whose lexical form its own datatype does not accept — refuses the run.
    #[test]
    fn an_ill_typed_literal_is_a_refusal_under_the_datatype_lanes() {
        let mut b = RdfDatasetBuilder::new();
        let s = iri(&mut b, X);
        let p = iri(&mut b, "http://example.org/age");
        let bad = b.intern_literal(purrdf_core::RdfLiteral::typed(
            "cat",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        b.push_quad(s, p, bad, None);
        let ds = b.freeze().expect("freeze");

        for plan in [Materialization::OwlRl, Materialization::D] {
            let regime = plan.regime();
            let Err(EntailError::Inconsistent(run)) = materialize(&ds, plan) else {
                panic!("{regime:?}: an ill-typed literal is `dt-not-type`");
            };
            let witness = run.witness();
            assert_eq!(witness.rule(), RuleId::DtNotType, "{regime:?}");
            // The witness names a TRIPLE that carries the bad literal, not merely the
            // literal: the internal `DT_ILL_TYPED` premise is bookkeeping, not an
            // asserted triple, so it is filtered out of the evidence. Which occurrence it
            // names is whichever the evaluator's total order reached first — under
            // `OWL-RL` that is `eq-ref`'s own `lt owl:sameAs lt`, which is an occurrence
            // of the literal like any other — so the check is on the position the rule
            // binds rather than on a particular carrier.
            assert_eq!(witness.premises().len(), 1, "{regime:?}");
            assert_eq!(
                witness.premises()[0].object(),
                &TermValue::typed_literal("cat", "http://www.w3.org/2001/XMLSchema#integer"),
                "{regime:?}"
            );
        }
        // A well-typed literal in the same shape closes fine.
        let mut b = RdfDatasetBuilder::new();
        let s = iri(&mut b, X);
        let p = iri(&mut b, "http://example.org/age");
        let good = b.intern_literal(purrdf_core::RdfLiteral::typed(
            "7",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        b.push_quad(s, p, good, None);
        let ds = b.freeze().expect("freeze");
        assert!(materialize(&ds, Materialization::D).is_ok());
        assert!(materialize(&ds, Materialization::OwlRl).is_ok());
    }

    /// A FUNCTIONAL DATA PROPERTY with two value-different values is an inconsistency, and
    /// that is the whole of Table 8 working with Tables 4 and 5 at once.
    ///
    /// `prp-fp` concludes `"1"^^xsd:integer owl:sameAs "2"^^xsd:integer` — a triple with a
    /// literal SUBJECT, so it is generalized RDF and never reaches the closure — `dt-diff`
    /// concludes the two are different, and `eq-diff1` puts the two together. It is the
    /// classic OWL 2 RL clash, it is unreachable without all three tables, and it is the
    /// only way a `owl:sameAs` between two literals can arise at all: the RDF 1.2 IR
    /// cannot hold one as an ASSERTION, because a literal may not be a subject.
    ///
    /// It also exercises the one-orientation `DT_DIFFERENT` relation end to end. The
    /// pre-pass emits `lt1 ≠ lt2` for `lt1 < lt2` only — halving the largest relation this
    /// crate materializes — and `eq-sym` supplies the mirror, so the clash is found
    /// whichever way round the derived equality happens to be committed.
    #[test]
    fn a_functional_data_property_with_two_values_is_inconsistent() {
        let functional = "http://www.w3.org/2002/07/owl#FunctionalProperty";
        let integer = "http://www.w3.org/2001/XMLSchema#integer";
        let build = |left: &str, right: &str| {
            let mut b = RdfDatasetBuilder::new();
            let p = iri(&mut b, "http://example.org/age");
            let ty = iri(&mut b, RDF_TYPE);
            let fp = iri(&mut b, functional);
            let x = iri(&mut b, X);
            let one = b.intern_literal(purrdf_core::RdfLiteral::typed(left, integer));
            let two = b.intern_literal(purrdf_core::RdfLiteral::typed(right, integer));
            b.push_quad(p, ty, fp, None);
            b.push_quad(x, p, one, None);
            b.push_quad(x, p, two, None);
            b.freeze().expect("freeze")
        };

        // Two DIFFERENT values: `prp-fp` then `dt-diff` then `eq-diff1`.
        let Err(EntailError::Inconsistent(run)) =
            materialize(&build("1", "2"), Materialization::OwlRl)
        else {
            panic!("a functional property with two value-different values must clash");
        };
        let witness = run.witness();
        assert_eq!(witness.rule(), RuleId::EqDiff1);
        assert_eq!(witness.premises().len(), 2, "{:?}", witness.premises());

        // Two SPELLINGS OF ONE value: `dt-eq` says they are the same thing, so there is
        // nothing to clash — and `eq-rep-o` carries the value across the spellings.
        let (closed, report) = materialize(&build("1", "01"), Materialization::OwlRl)
            .expect("one value, two spellings");
        assert!(report.inconsistency().is_none());
        assert!(
            closed.quads().any(|q| {
                closed.term_value(q.s) == TermValue::iri(X)
                    && closed.term_value(q.o) == TermValue::typed_literal("01", integer)
            }),
            "dt-eq and eq-rep-o must keep the equal-valued spelling on the subject"
        );
        // The `owl:sameAs` between the two literals is licensed and UNREPRESENTABLE, so it
        // is dropped at the boundary and the drop is reported rather than fabricated
        // around.
        assert!(
            report
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == Construct::GeneralizedRdf),
            "{:?}",
            report.boundaries()
        );
    }

    /// `owl:sameAs` substitutes in the PREDICATE position, and it is `eq-rep-p` that does
    /// it — the one rule of the calculus that rewrites a triple's predicate from a term
    /// bound in another atom's OBJECT position.
    ///
    /// It gets a test of its own because it is the rule an IR that addressed relations by
    /// predicate symbol could not express at all: `?p2` is data in the `owl:sameAs` triple
    /// and a relation name in the conclusion.
    #[test]
    fn equality_substitutes_in_the_predicate_position() {
        let same_as = "http://www.w3.org/2002/07/owl#sameAs";
        let p = "http://example.org/p";
        let q = "http://example.org/q";
        let y = "http://example.org/y";
        let ds = dataset(&[(p, same_as, q), (X, p, y)]);
        let (closed, report) = materialize(&ds, Materialization::OwlRl).expect("owl-rl");
        assert!(has(&closed, X, q, y), "eq-rep-p must rewrite the predicate");
        assert!(
            report
                .rules_fired()
                .iter()
                .any(|&(rule, count)| rule == RuleId::EqRepP && count >= 1),
            "{:?}",
            report.rules_fired()
        );
        // And the equivalence relation itself is closed: `eq-sym` mirrors the assertion
        // and `eq-ref` makes every term the same as itself.
        assert!(has(&closed, q, same_as, p), "eq-sym");
        assert!(has(&closed, X, same_as, X), "eq-ref");
    }

    /// `owl:sameAs` does NOT substitute inside an RDF 1.2 TRIPLE TERM, and the run says so.
    ///
    /// The chase interns a triple term as ONE atomic term and never looks inside it, so
    /// `<<( :x :p :y )>>` and `<<( :x :p :z )>>` stay two terms even when `:y owl:sameAs
    /// :z`. That is a documented boundary rather than silence: an implementation that
    /// substituted inside would be doing something the chase cannot see, and one that said
    /// nothing would let a caller believe the congruence was complete.
    #[test]
    fn equality_does_not_substitute_inside_a_triple_term() {
        let same_as = "http://www.w3.org/2002/07/owl#sameAs";
        let p = "http://example.org/p";
        let y = "http://example.org/y";
        let z = "http://example.org/z";

        let mut b = RdfDatasetBuilder::new();
        let x = iri(&mut b, X);
        let says = iri(&mut b, SAYS);
        let same = iri(&mut b, same_as);
        let yy = iri(&mut b, y);
        let zz = iri(&mut b, z);
        let pp = iri(&mut b, p);
        let quoted = b.intern_triple(x, pp, yy);
        b.push_quad(yy, same, zz, None);
        b.push_quad(x, says, quoted, None);
        let ds = b.freeze().expect("freeze");

        let (closed, report) = materialize(&ds, Materialization::OwlRl).expect("owl-rl");
        let substituted = quoted_value(X, p, TermValue::iri(z));
        assert!(
            !objects_of(&closed, X, SAYS).contains(&substituted),
            "the chase substituted inside a triple term"
        );
        // The original is carried through untouched…
        assert!(objects_of(&closed, X, SAYS).contains(&quoted_value(X, p, TermValue::iri(y))));
        // …and the boundary that licenses the omission is reported.
        assert!(
            report
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == Construct::TripleTerm),
            "{:?}",
            report.boundaries()
        );
    }

    /// Two runs of the same input render byte-identically, across every regime and every
    /// fixture — the whole report, not just the closure.
    #[test]
    fn reports_are_byte_identical_across_runs() {
        for ds in [
            schema_fixture(),
            triple_term_fixture(),
            named_graph_fixture(),
            literal_object_fixture(),
        ] {
            for plan in RUNNABLE {
                let regime = plan.regime();
                let (_, first) = materialize(&ds, plan).expect("runnable regime");
                let (_, second) = materialize(&ds, plan).expect("runnable regime");
                assert_eq!(
                    format!("{first:?}"),
                    format!("{second:?}"),
                    "{regime:?} report is not reproducible"
                );
            }
        }
    }

    /// A BLANK NODE survives the closure, in both positions it may occupy, and two blank
    /// nodes that differ only in SCOPE stay two nodes.
    ///
    /// The evaluator interns a term by its lexical surface, so a blank node has to be
    /// rendered to one and read back — and a scope is part of a blank node's identity
    /// (C0.2) while being absent from any standard surface syntax. Collapsing two scopes
    /// into one label would silently merge two individuals, which is unsound rather than
    /// merely lossy, so the fixture asserts the pair stays distinct through a rule that
    /// touches both positions.
    #[test]
    fn blank_nodes_survive_the_closure_and_their_scopes_do_not_collapse() {
        use purrdf_core::BlankScope;

        let mut b = RdfDatasetBuilder::new();
        let sub = iri(&mut b, RDFS_SUBCLASSOF);
        let ty = iri(&mut b, RDF_TYPE);
        let bb = iri(&mut b, B);
        let first = b.intern_blank("shared", BlankScope::DEFAULT);
        let second = b.intern_blank("shared", BlankScope(7));
        let class = b.intern_blank("class", BlankScope::DEFAULT);
        // `_:class ⊑ B`, and two same-labelled blanks from different scopes typed by it.
        b.push_quad(class, sub, bb, None);
        b.push_quad(first, ty, class, None);
        b.push_quad(second, ty, class, None);
        let ds = b.freeze().expect("freeze");

        let (closed, report) = materialize(&ds, Materialization::Rdfs).expect("rdfs");
        // rdfs9 re-types BOTH blank subjects. The interesting evidence is WHICH subjects
        // it produced, asserted below over the closure itself; the rule's tally is not
        // pinned here because the RDFS lane also asserts the axiomatic triples, so rdfs9
        // additionally re-types these nodes through `rdfs:Resource` and this fixture is
        // not about that arithmetic.
        assert!(
            report
                .rules_fired()
                .iter()
                .any(|&(rule, count)| rule == RuleId::Rdfs9 && count >= 2),
            "rdfs9 must be credited for both re-typings: {:?}",
            report.rules_fired()
        );
        let typed: Vec<TermValue> = closed
            .quads()
            .filter(|q| {
                closed.term_value(q.p) == TermValue::iri(RDF_TYPE)
                    && closed.term_value(q.o) == TermValue::iri(B)
            })
            .map(|q| closed.term_value(q.s))
            .collect();
        assert_eq!(
            typed.len(),
            2,
            "two scopes must stay two subjects: {typed:?}"
        );
        for value in &typed {
            let (label, _) = value.as_blank().expect("a blank subject stayed blank");
            assert_eq!(label, "shared");
        }
        assert_ne!(typed[0], typed[1], "the two scopes collapsed into one node");
        // The blank OBJECT position round-trips too: `_:class ⊑ B` is still about `_:class`.
        assert!(
            closed.quads().any(|q| {
                closed.term_value(q.p) == TermValue::iri(RDFS_SUBCLASSOF)
                    && closed.term_value(q.s).as_blank().map(|(l, _)| l) == Some("class")
            }),
            "the blank subject of the schema triple was not carried through"
        );
    }

    /// A ceiling is a REFUSAL, and it reaches the caller as one.
    ///
    /// `materialize` evaluates the declared program through `purrdf-datalog`, and that
    /// evaluator holds three fixed ceilings. There is no partial answer behind one: a
    /// truncated closure returned as a complete one is precisely the failure a
    /// [`ReasoningReport`] exists to prevent, so an exhausted budget is
    /// [`EntailError::Evaluate`] and the closure is not produced at all.
    ///
    /// The input is the smallest cross product that passes a ceiling: `p` carries 360
    /// `rdfs:domain` declarations and 380 triples use `p`, so rdfs2 alone must conclude
    /// 136 800 typings — more than [`MAX_STORED_FACTS`](purrdf_datalog::seminaive::MAX_STORED_FACTS)
    /// admits. The report is asserted to carry the OBSERVATION that proved the ceiling was
    /// passed rather than the ceiling itself, because a figure rounded down to the limit
    /// would tell a caller nothing about how far over they are.
    #[test]
    fn an_exhausted_budget_is_a_refusal_with_an_accurate_report() {
        use purrdf_datalog::chase::ChaseError;
        use purrdf_datalog::seminaive::{BudgetResource, MAX_STORED_FACTS};

        /// `rdfs:domain` declarations on `p`.
        const CLASSES: usize = 360;
        /// Triples that use `p`.
        const TRIPLES: usize = 380;

        let mut b = RdfDatasetBuilder::new();
        let p = iri(&mut b, "http://example.org/p");
        let domain = iri(&mut b, RDFS_DOMAIN);
        for index in 0..CLASSES {
            let class = iri(&mut b, &format!("http://example.org/C{index}"));
            b.push_quad(p, domain, class, None);
        }
        for index in 0..TRIPLES {
            let subject = iri(&mut b, &format!("http://example.org/x{index}"));
            let object = iri(&mut b, &format!("http://example.org/y{index}"));
            b.push_quad(subject, p, object, None);
        }
        let ds = b.freeze().expect("freeze");

        // The `RDFS` lane runs through the restricted chase (it states four existential
        // rules), so its ceiling refusal is the chase's — the SAME three fixed constants,
        // charged the same way, refused by name rather than truncated.
        let Err(EntailError::Chase(ChaseError::BudgetExhausted { resource, report })) =
            materialize(&ds, Materialization::Rdfs)
        else {
            panic!("a cross product past a fixed ceiling must be refused, not truncated");
        };
        assert_eq!(resource, BudgetResource::StoredFacts);
        assert!(
            report.stored_facts() > MAX_STORED_FACTS,
            "the report must carry the observation that passed the ceiling, not the \
             ceiling: {} vs {MAX_STORED_FACTS}",
            report.stored_facts()
        );
        // The refusal is the EVALUATOR's, not the façade's: the same input copies fine.
        let (copied, simple) = materialize(&ds, Materialization::Simple).expect("simple");
        assert_eq!(copied.quad_refs().count(), CLASSES + TRIPLES);
        assert_eq!(simple.budget().stored_facts(), 0);
    }

    #[test]
    fn simple_regime_is_identity() {
        let ds = dataset(&[(A, RDFS_SUBCLASSOF, B), (X, RDF_TYPE, A)]);
        let (closed, _report) = materialize(&ds, Materialization::Simple).expect("simple");
        // No inference: x is not typed B.
        assert!(!has(&closed, X, RDF_TYPE, B));
        assert!(has(&closed, X, RDF_TYPE, A));
    }

    // ── Rebuilding a conclusion AROUND a term the rules cannot look inside ──────
    //
    // rdfs7 / prp-spo1 rewrites a triple's PREDICATE and copies its object through
    // unchanged, so the object of the conclusion has to be re-interned into the emitted
    // dataset whatever kind of term it is. Substituting a different term there is
    // unsound: it asserts a triple nothing entails. These tests pin the round trip for
    // each object kind the rewrite can carry.

    /// Fixture property `example.org/says`.
    const SAYS: &str = "http://example.org/says";
    /// Fixture property `example.org/mentions`, the super-property of `says`.
    const MENTIONS: &str = "http://example.org/mentions";
    /// `rdfs:subPropertyOf`, the axiom that drives the rewrite.
    const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
    /// `rdfs:Resource` — the IRI the old fold substituted for a triple term. Named here
    /// only so its ABSENCE can be asserted.
    const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";

    /// `says ⊑ mentions` plus `x says <o>`, where `o` is whatever term `object` interns.
    ///
    /// The smallest input that makes rdfs7 / prp-spo1 build a conclusion AROUND `o`:
    /// the predicate changes, the object is carried through, and the emitted triple can
    /// only be right if `o` re-materializes as itself.
    fn rewrite_fixture(
        object: impl FnOnce(&mut RdfDatasetBuilder) -> purrdf_core::TermId,
    ) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let x = iri(&mut b, X);
        let says = iri(&mut b, SAYS);
        let mentions = iri(&mut b, MENTIONS);
        let spo = iri(&mut b, RDFS_SUBPROPERTYOF);
        let o = object(&mut b);
        b.push_quad(says, spo, mentions, None);
        b.push_quad(x, says, o, None);
        b.freeze().expect("freeze")
    }

    /// Every default-graph object of `(s, p)`, by value.
    fn objects_of(ds: &RdfDataset, s: &str, p: &str) -> Vec<TermValue> {
        ds.quads()
            .filter(|q| {
                q.g.is_none()
                    && ds.term_value(q.s) == TermValue::iri(s)
                    && ds.term_value(q.p) == TermValue::iri(p)
            })
            .map(|q| ds.term_value(q.o))
            .collect()
    }

    /// A triple term over three IRIs, by value.
    fn quoted_value(s: &str, p: &str, o: TermValue) -> TermValue {
        TermValue::Triple {
            s: Box::new(TermValue::iri(s)),
            p: Box::new(TermValue::iri(p)),
            o: Box::new(o),
        }
    }

    /// rdfs7 / prp-spo1 carries a triple-term object through the rewrite intact.
    ///
    /// `x says <<( A ⊑ B )>>` with `says ⊑ mentions` entails
    /// `x mentions <<( A ⊑ B )>>` and nothing else about `x mentions`. The engine used
    /// to emit `x mentions rdfs:Resource` here, which is not entailed by this input
    /// under any regime — a wrong triple, not a missing one.
    #[test]
    fn a_subproperty_rewrite_carries_a_triple_term_object_through() {
        let ds = rewrite_fixture(|b| {
            let s = b.intern_iri(A);
            let p = b.intern_iri(RDFS_SUBCLASSOF);
            let o = b.intern_iri(B);
            b.intern_triple(s, p, o)
        });
        let expected = quoted_value(A, RDFS_SUBCLASSOF, TermValue::iri(B));
        for plan in [Materialization::Rdfs, Materialization::OwlRl] {
            let regime = plan.regime();
            let (closed, report) = materialize(&ds, plan).expect("runnable regime");
            assert_eq!(
                objects_of(&closed, X, MENTIONS),
                vec![expected.clone()],
                "{regime:?}: the rewrite must conclude exactly the triple term"
            );
            assert!(
                !has(&closed, X, MENTIONS, RDFS_RESOURCE),
                "{regime:?}: a term was fabricated for the triple term"
            );
            // Opacity is the licensed part, and it is REPORTED: the chase never reasons
            // into the quoted triple. rdfs14 / rdfs14a do fire over it, but each concludes
            // about a fresh surrogate the answer may not bind, so nothing they draw reaches
            // the closure and the run says so with the triple-term boundary.
            assert!(
                report
                    .boundaries()
                    .iter()
                    .any(|boundary| boundary.construct() == Construct::TripleTerm),
                "{regime:?}: the triple-term boundary must be reported"
            );
        }
    }

    /// The reconstruction NESTS: a triple term whose object is itself a triple term
    /// round-trips to full depth through the same rewrite.
    #[test]
    fn a_subproperty_rewrite_carries_a_nested_triple_term_through() {
        let ds = rewrite_fixture(|b| {
            let a = b.intern_iri(A);
            let sco = b.intern_iri(RDFS_SUBCLASSOF);
            let bb = b.intern_iri(B);
            let inner = b.intern_triple(a, sco, bb);
            let p = b.intern_iri("http://example.org/p");
            b.intern_triple(a, p, inner)
        });
        let expected = quoted_value(
            A,
            "http://example.org/p",
            quoted_value(A, RDFS_SUBCLASSOF, TermValue::iri(B)),
        );
        for plan in [Materialization::Rdfs, Materialization::OwlRl] {
            let regime = plan.regime();
            let (closed, _report) = materialize(&ds, plan).expect("runnable regime");
            assert_eq!(
                objects_of(&closed, X, MENTIONS),
                vec![expected.clone()],
                "{regime:?}: the nested triple term was not rebuilt to depth"
            );
        }
    }

    /// A directional language-tagged literal keeps its base direction across the rewrite.
    ///
    /// Direction participates in literal identity (C0.1), so a conclusion that dropped it
    /// would be about a DIFFERENT literal than the premise was.
    #[test]
    fn a_subproperty_rewrite_preserves_a_literal_base_direction() {
        for direction in [RdfTextDirection::Ltr, RdfTextDirection::Rtl] {
            let ds = rewrite_fixture(|b| {
                b.intern_literal(purrdf_core::RdfLiteral {
                    lexical_form: "hello".to_owned(),
                    datatype: None,
                    language: Some("en".to_owned()),
                    direction: Some(direction),
                })
            });
            let expected = TermValue::Literal {
                lexical_form: "hello".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some("en".to_owned()),
                direction: Some(direction),
            };
            for plan in [Materialization::Rdfs, Materialization::OwlRl] {
                let regime = plan.regime();
                let (closed, _report) = materialize(&ds, plan).expect("runnable regime");
                assert_eq!(
                    objects_of(&closed, X, MENTIONS),
                    vec![expected.clone()],
                    "{regime:?}: {direction:?} was not preserved through the rewrite"
                );
            }
        }
    }

    /// A triple term in a position the rules CANNOT use fabricates nothing.
    ///
    /// `p rdfs:range A` with `x p <<( A ⊑ B )>>` would have rdfs3 / prp-rng conclude
    /// `<<( A ⊑ B )>> rdf:type A`, whose subject is a triple term — a generalized-RDF
    /// triple the IR cannot hold. The conclusion is abandoned and the drop is reported;
    /// what may never happen is a stand-in term being invented so the triple can be
    /// emitted anyway.
    #[test]
    fn a_triple_term_the_rules_cannot_conclude_into_fabricates_no_term() {
        let mut b = RdfDatasetBuilder::new();
        let x = iri(&mut b, X);
        let p = iri(&mut b, "http://example.org/p");
        let rng = iri(&mut b, RDFS_RANGE);
        let a = iri(&mut b, A);
        let sco = iri(&mut b, RDFS_SUBCLASSOF);
        let bb = iri(&mut b, B);
        let quoted = b.intern_triple(a, sco, bb);
        b.push_quad(p, rng, a, None);
        b.push_quad(x, p, quoted, None);
        let ds = b.freeze().expect("freeze");

        for plan in [Materialization::Rdfs, Materialization::OwlRl] {
            let regime = plan.regime();
            let (closed, report) = materialize(&ds, plan).expect("runnable regime");
            // Nothing was concluded ABOUT the triple term…
            assert!(
                !closed
                    .quads()
                    .any(|q| matches!(closed.term_value(q.s), TermValue::Triple { .. })),
                "{regime:?}: a triple term reached subject position"
            );
            // …and no stand-in was minted to carry the abandoned conclusion.
            assert!(
                !has(&closed, X, RDF_TYPE, A) && !has(&closed, RDFS_RESOURCE, RDF_TYPE, A),
                "{regime:?}: a term was fabricated for the abandoned conclusion"
            );
            assert!(
                report
                    .boundaries()
                    .iter()
                    .any(|boundary| boundary.construct() == Construct::GeneralizedRdf),
                "{regime:?}: the abandoned conclusion must be reported"
            );
        }
    }
}
