// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What a reasoning run actually did, as a value.
//!
//! [`ReasoningReport`] accompanies every [`materialize`](crate::materialize) result. It
//! exists because the interesting failure of a reasoner is not a wrong triple — it is a
//! MISSING one presented as a complete answer. Without a report, "closed under 78 rules"
//! and "closed under 12" are the same call with the same return type, and the only place
//! the difference can live is prose, which nothing can gate. Here the difference is data:
//!
//! * [`Completeness`] is DERIVED from the rule inventory — [`rules`] minus [`implemented`] — never
//!   asserted. When a later change teaches the
//!   chase a rule, every report improves without anyone remembering to edit a claim.
//! * [`ReasoningReport::rules_fired`] says which rules produced conclusions, and how many.
//! * [`Boundary`] says which constructs the run could not fully handle, and why.
//! * [`ReasoningReport::contract_hash`] names the calculus, so a cached verdict minted
//!   under a different rule set can be refused rather than trusted.
//!
//! # The overclaim gate
//!
//! [`ReasoningReport::overclaims`] is the invariant a report must never violate:
//! [`Completeness::Exact`] while [`ReasoningReport::boundaries`] is non-empty is a claim
//! of completeness contradicted by the report's own evidence. It is a method rather than a
//! comment so a consumer can check it too; the crate's tests assert it for every run they
//! make.
//!
//! # Determinism
//!
//! Every sequence in a report has a fixed, documented order and none of them is built by
//! iterating a map: missing rules and fired rules are in specification table order, and
//! boundaries are in [`Construct`] declaration order. Two identical runs produce
//! byte-identical reports.

use purrdf_core::{RdfDataset, TermRef, TermValue};
use purrdf_datalog::cache::ContractHash;
use purrdf_datalog::seminaive::BudgetReport;

use crate::Regime;
use crate::calculus::{ChaseRule, calculus_contract_hash};
use crate::rules::{RuleId, implemented, rules};

/// How much of `regime`'s specified rule table was available to a run.
///
/// Derived from the inventory by [`Completeness::for_regime`], never asserted at a call
/// site: the value is a function of [`rules`] and [`implemented`] alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Completeness {
    /// Every rule the regime is defined by was available.
    ///
    /// Sound AND complete for the rule table — which is not the same as "answered
    /// everything the regime's semantics entail", because a rule table can be complete
    /// while a construct outside it is still a [`Boundary`]. A report that claims `Exact`
    /// with a non-empty boundary list is contradicting itself; see
    /// [`ReasoningReport::overclaims`].
    Exact,
    /// Some rule the regime is defined by was not available.
    ///
    /// Every conclusion drawn is still sound — the missing rules could only have ADDED
    /// conclusions — but the closure is not complete for the regime.
    SoundIncomplete {
        /// The rules `regime` defines that the chase does not fire, in specification
        /// table order.
        missing: Vec<RuleId>,
    },
}

impl Completeness {
    /// `regime`'s completeness, computed as [`rules`] minus [`implemented`].
    ///
    /// ```
    /// use purrdf_entail::{Completeness, Regime};
    ///
    /// // The identity closure has no rules to be missing.
    /// assert!(Completeness::for_regime(Regime::Simple).is_exact());
    /// // OWL 2 RL defines 78 rules; this crate's chase fires 12 of them.
    /// assert_eq!(Completeness::for_regime(Regime::OwlRl).missing().len(), 66);
    /// ```
    #[must_use]
    pub fn for_regime(regime: Regime) -> Self {
        let done = implemented(regime);
        let missing: Vec<RuleId> = rules(regime)
            .iter()
            .copied()
            .filter(|rule| !done.contains(rule))
            .collect();
        if missing.is_empty() {
            Self::Exact
        } else {
            Self::SoundIncomplete { missing }
        }
    }

    /// Whether every rule the regime defines was available.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        matches!(self, Self::Exact)
    }

    /// The missing rules, in specification table order; empty for [`Self::Exact`].
    #[must_use]
    pub fn missing(&self) -> &[RuleId] {
        match self {
            Self::Exact => &[],
            Self::SoundIncomplete { missing } => missing,
        }
    }
}

/// A construct a reasoning run could not fully handle.
///
/// Named by a Rust variant rather than by an IRI: PurRDF mints no vocabulary, so a
/// boundary is identified by the enum below and, where a specification names the rules it
/// blocks, by those rules' own ids in the [`Boundary::reason`] text.
///
/// Variants are declared in the order a report lists them: the three a RUN observes first
/// (the input held the construct, or a conclusion was actually abandoned because of it),
/// then the two that are INHERENT to a lane and hold for every input. The derived [`Ord`]
/// follows that declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Construct {
    /// Quads outside the default graph.
    NamedGraph,
    /// RDF 1.2 triple terms.
    TripleTerm,
    /// Conclusions RDF 1.2 syntax cannot hold — generalized-RDF triples.
    GeneralizedRdf,
    /// The infinite axiomatic-triple schemas of RDF and RDFS.
    AxiomaticTriples,
    /// Datatype value spaces, which are infinite.
    DatatypeValueSpace,
}

impl Construct {
    /// Every construct, in declaration order — the order a report lists boundaries in.
    pub(crate) const ALL: [Self; 5] = [
        Self::NamedGraph,
        Self::TripleTerm,
        Self::GeneralizedRdf,
        Self::AxiomaticTriples,
        Self::DatatypeValueSpace,
    ];

    /// A short, stable name for the construct.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NamedGraph => "named-graph",
            Self::TripleTerm => "triple-term",
            Self::GeneralizedRdf => "generalized-rdf",
            Self::AxiomaticTriples => "axiomatic-triples",
            Self::DatatypeValueSpace => "datatype-value-space",
        }
    }

    /// Why this construct is a boundary — the technical reason, not an apology.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NamedGraph => {
                "the chase reads and writes the default graph only, so quads in a named \
                 graph neither supply premises nor receive conclusions; they are carried \
                 through the closure unchanged"
            }
            Self::TripleTerm => {
                "rdfs14 and rdfs14a replace a triple term with a FRESH blank node typed \
                 rdfs:Proposition, and an existentially quantified head is not a Datalog \
                 clause: the evaluator mints no terms, so neither rule fires. A triple \
                 term is therefore interned as one atomic term the chase never looks \
                 inside, the closure states nothing about the triple the term quotes, \
                 and a conclusion built AROUND such a term carries it through unchanged"
            }
            Self::GeneralizedRdf => {
                "a conclusion whose subject position would hold a literal or a triple term, \
                 or whose predicate position would hold anything but an IRI, is a \
                 generalized-RDF triple, which the RDF 1.2 dataset IR cannot represent; \
                 such a conclusion is derived in the evaluator's own term space and then \
                 abandoned when the answer is materialized, rather than a term being \
                 fabricated for it, so rules that can conclude into subject position \
                 (rdfs3 / prp-rng, prp-symp, prp-inv1, prp-inv2) are incomplete over \
                 literal objects"
            }
            Self::AxiomaticTriples => {
                "the FINITE part of the RDF and RDFS axiomatic triples — the fixed \
                 domain, range, type and sub-class statements about the RDF and RDFS \
                 vocabulary itself — is asserted as a premise, so every conclusion it \
                 licenses is drawn; two things are not. The container-membership family \
                 rdf:_1, rdf:_2, … is unbounded and no forward chase can materialize an \
                 infinite set, so rdfs12 fires only on a container property the graph \
                 itself types. And the asserted axioms are premises rather than \
                 conclusions, so the closure does not restate them: they are entailed but \
                 not emitted"
            }
            Self::DatatypeValueSpace => {
                "rdfD1 and rdfD1a conclude about a FRESH blank node standing for a \
                 datatyped literal, or for an inhabited value space, and an \
                 existentially quantified head is not a Datalog clause: the evaluator \
                 mints no terms, so neither rule fires. rdfs1 recognizes the datatypes \
                 RDF 1.2 Semantics §8 makes mandatory (rdf:langString, \
                 rdf:dirLangString, xsd:string) and no others, and the OWL 2 RL rules \
                 quantified over value spaces (dt-type2, dt-eq, dt-diff) are not fired \
                 at all; literals are compared by their lexical form and datatype IRI, \
                 never by their data value"
            }
        }
    }
}

impl std::fmt::Display for Construct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One construct a run could not fully handle, and the technical reason.
///
/// The reason is a function of the construct — [`Boundary::of`] is the only constructor —
/// so the two can never drift apart the way a construct and a hand-written explanation
/// would.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Boundary {
    /// The construct.
    construct: Construct,
}

impl Boundary {
    /// The boundary for `construct`.
    #[must_use]
    pub const fn of(construct: Construct) -> Self {
        Self { construct }
    }

    /// The construct this boundary is about.
    #[must_use]
    pub const fn construct(self) -> Construct {
        self.construct
    }

    /// Why the construct could not be fully handled.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        self.construct.reason()
    }
}

impl std::fmt::Display for Boundary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.construct.as_str(), self.reason())
    }
}

/// One asserted triple named by an [`InconsistencyWitness`].
///
/// Terms are [`TermValue`]s — dataset-independent by construction — so a witness outlives
/// the dataset it was drawn from and can be compared across datasets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WitnessTriple {
    /// The subject term.
    subject: TermValue,
    /// The predicate term.
    predicate: TermValue,
    /// The object term.
    object: TermValue,
}

impl WitnessTriple {
    /// The triple `(subject, predicate, object)`.
    #[must_use]
    pub const fn new(subject: TermValue, predicate: TermValue, object: TermValue) -> Self {
        Self {
            subject,
            predicate,
            object,
        }
    }

    /// The subject term.
    #[must_use]
    pub const fn subject(&self) -> &TermValue {
        &self.subject
    }

    /// The predicate term.
    #[must_use]
    pub const fn predicate(&self) -> &TermValue {
        &self.predicate
    }

    /// The object term.
    #[must_use]
    pub const fn object(&self) -> &TermValue {
        &self.object
    }
}

/// Evidence that a knowledge base is inconsistent: which rule, which facts, which graph.
///
/// # Currently unreachable, and that is a fact about the rule set
///
/// A [`materialize`](crate::materialize) run never produces one today, and
/// [`ReasoningReport::inconsistency`] is therefore always `None`. That is an accurate
/// report, not a stub: OWL 2 RL derives an inconsistency through the seventeen rules whose
/// conclusion is `false` (`eq-diff1`, `eq-diff2`, `eq-diff3`, `prp-irp`, `prp-asyp`,
/// `prp-pdw`, `prp-adp`, `prp-npa1`, `prp-npa2`, `cls-nothing2`, `cls-com`, `cls-maxc1`,
/// `cls-maxqc1`, `cls-maxqc2`, `cax-dw`, `cax-adc`, `dt-not-type`), and every one of them
/// is in the regime's MISSING list — so no chase path can reach
/// [`EntailError::Inconsistent`](crate::EntailError). The RDF and RDFS lanes cannot reach
/// one either: neither calculus has an inconsistency rule at all. A
/// report that manufactured a witness would be inventing one; a report that omitted the
/// field would leave a consumer unable to tell "consistent" from "not checked", which is
/// what [`Completeness::SoundIncomplete`] is for.
///
/// The type is complete and constructible, so the change that adds one of those rules adds
/// a witness at the point it detects the clash and nothing else has to move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InconsistencyWitness {
    /// The rule whose premises were all satisfied.
    rule: RuleId,
    /// The asserted triples that satisfied them, in the rule's premise order.
    premises: Vec<WitnessTriple>,
    /// The graph the premises were read from; `None` is the default graph.
    graph: Option<TermValue>,
}

impl InconsistencyWitness {
    /// A witness that `rule` fired on `premises`, read from `graph`.
    ///
    /// `premises` is in the rule's own premise order, so a reader can line the triples up
    /// against the specification's rule table entry.
    #[must_use]
    pub const fn new(rule: RuleId, premises: Vec<WitnessTriple>, graph: Option<TermValue>) -> Self {
        Self {
            rule,
            premises,
            graph,
        }
    }

    /// The rule whose premises were all satisfied.
    #[must_use]
    pub const fn rule(&self) -> RuleId {
        self.rule
    }

    /// The asserted triples that satisfied them, in the rule's premise order.
    #[must_use]
    pub fn premises(&self) -> &[WitnessTriple] {
        &self.premises
    }

    /// The graph the premises were read from; `None` is the default graph.
    #[must_use]
    pub fn graph(&self) -> Option<&TermValue> {
        self.graph.as_ref()
    }
}

/// What one reasoning run consumed and produced, gathered by the engine that ran it.
///
/// Purely a carrier between [`crate::engine`] and [`ReasoningReport::of_run`]; it is the
/// engine's measurements, not the report's shape.
///
/// The budget is the evaluator's OWN [`BudgetReport`], carried through rather than
/// re-derived: `purrdf-datalog` counts candidate solutions, stored facts and term-arena
/// bytes as it runs, and a second tally kept alongside it here could only ever agree with
/// it by accident. What this crate adds is the two measurements the evaluator has no way
/// to make — which rule a committed derivation is attributed to under the active regime's
/// specification names, and how many conclusions the RDF 1.2 IR could not hold.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RunStats {
    /// Conclusions committed AND materialized, per [`ChaseRule`], indexed by
    /// [`ChaseRule::index`].
    pub(crate) fired: [u64; ChaseRule::COUNT],
    /// What the evaluation consumed of `purrdf-datalog`'s three fixed ceilings.
    pub(crate) budget: BudgetReport,
    /// Conclusions dropped because the RDF 1.2 IR cannot hold them — a literal or triple
    /// term in subject position, or a non-IRI in predicate position. The
    /// [`Construct::GeneralizedRdf`] boundary's observation.
    pub(crate) generalized_rdf_drops: u64,
}

impl RunStats {
    /// The measurements of a run that evaluated nothing — the `Simple` identity closure.
    pub(crate) fn none() -> Self {
        Self::of_budget(BudgetReport::new(0, 0, 0))
    }

    /// A fresh tally over an evaluation that consumed `budget`.
    pub(crate) fn of_budget(budget: BudgetReport) -> Self {
        Self {
            fired: [0; ChaseRule::COUNT],
            budget,
            generalized_rdf_drops: 0,
        }
    }

    /// Credit one committed, materialized conclusion to `rule`.
    pub(crate) const fn commit(&mut self, rule: ChaseRule) {
        self.fired[rule.index()] += 1;
    }

    /// Record one conclusion the RDF 1.2 IR could not hold.
    pub(crate) const fn drop_generalized(&mut self) {
        self.generalized_rdf_drops += 1;
    }
}

/// What the dataset itself contains that bears on a boundary.
///
/// One pass over the quads, so a boundary is emitted because the input actually holds the
/// construct rather than because the lane might in principle meet it.
#[derive(Debug, Clone, Copy, Default)]
struct DatasetSurvey {
    /// Whether any quad sits outside the default graph.
    named_graph: bool,
    /// Whether any default-graph quad mentions a triple term.
    triple_term: bool,
}

impl DatasetSurvey {
    /// Survey `ds`.
    fn of(ds: &RdfDataset) -> Self {
        let mut survey = Self::default();
        for quad in ds.quad_refs() {
            if quad.g.is_some() {
                survey.named_graph = true;
                continue;
            }
            if matches!(quad.s, TermRef::Triple { .. })
                || matches!(quad.p, TermRef::Triple { .. })
                || matches!(quad.o, TermRef::Triple { .. })
            {
                survey.triple_term = true;
            }
            if survey.named_graph && survey.triple_term {
                break;
            }
        }
        survey
    }
}

/// What a reasoning run did — returned with every closure, never optional.
///
/// See the [module docs](self) for why the report is mandatory and what each field can and
/// cannot say.
#[derive(Debug, Clone)]
pub struct ReasoningReport {
    /// The regime the caller asked for.
    regime: Regime,
    /// How much of that regime's rule table was available.
    completeness: Completeness,
    /// The rules that produced conclusions, and how many each produced.
    rules_fired: Vec<(RuleId, u64)>,
    /// The constructs the run could not fully handle.
    boundaries: Vec<Boundary>,
    /// What the run consumed of the three fixed evaluation ceilings.
    budget: BudgetReport,
    /// The identity of the calculus the run used.
    contract_hash: ContractHash,
    /// Evidence of an inconsistency, if one was detected.
    inconsistency: Option<InconsistencyWitness>,
}

impl ReasoningReport {
    /// Assemble the report for a run of `regime` over `ds` that measured `stats`.
    pub(crate) fn of_run(ds: &RdfDataset, regime: Regime, stats: &RunStats) -> Self {
        Self {
            regime,
            completeness: Completeness::for_regime(regime),
            rules_fired: fired_rules(regime, stats),
            boundaries: boundaries(ds, regime, stats),
            budget: stats.budget,
            contract_hash: calculus_contract_hash(regime),
            // No chase path can reach an inconsistency; see [`InconsistencyWitness`].
            inconsistency: None,
        }
    }

    /// The regime the caller asked for.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// How much of the regime's specified rule table was available to the run.
    #[must_use]
    pub const fn completeness(&self) -> &Completeness {
        &self.completeness
    }

    /// The rules that produced conclusions, and how many conclusions each produced.
    ///
    /// In specification table order ([`RuleId`]'s declaration order), with rules that
    /// produced nothing omitted rather than listed as zero — the list answers "what fired",
    /// and a rule that fired zero times did not fire.
    ///
    /// # What a count means, exactly
    ///
    /// One count is one triple this rule was CREDITED with adding to the closure. The
    /// evaluator commits a derived fact once, and it records exactly one derivation for
    /// it; a triple two rules both conclude is credited to one of them, and the second
    /// rule's re-derivation is not counted. Which one is not an arrival accident:
    /// `purrdf-datalog` picks the round's winner by a total order over observable
    /// provenance — proof height, then summed source heights, then the sorted source
    /// facts, then the clause's authored index — so the attribution is a function of the
    /// program and the data, and a triple derivable in fewer steps is credited to the rule
    /// that got there in fewer steps.
    ///
    /// The counts therefore sum to exactly the number of inferred triples in the result,
    /// which is the sum a reader can check. A conclusion the RDF 1.2 IR cannot hold is
    /// credited to nobody — it never becomes an inferred triple — and is reported as the
    /// [`Construct::GeneralizedRdf`] boundary instead. The counts are NOT a measure of a
    /// rule's total work: [`ReasoningReport::budget`]'s join-step count is.
    ///
    /// # The three rules an `OWL-RL` run names from the RDFS tables
    ///
    /// Under `OWL-RL` this list may name `rdfs6`, `rdfs8` and `rdfs10`, which are not in
    /// `rules(Regime::OwlRl)`: OWL 2 RL/RDF omits them from its tables, the chase fires
    /// them anyway, and naming them by their RDFS ids is more honest than renaming them to
    /// a neighbouring OWL rule that would not have licensed the conclusion. So this list is
    /// a subset of `implemented(regime)` for every regime EXCEPT `OWL-RL`, where it is a
    /// subset of `implemented(Regime::OwlRl)` plus those three.
    #[must_use]
    pub fn rules_fired(&self) -> &[(RuleId, u64)] {
        &self.rules_fired
    }

    /// The constructs the run could not fully handle, in [`Construct`] declaration order.
    #[must_use]
    pub fn boundaries(&self) -> &[Boundary] {
        &self.boundaries
    }

    /// What the run consumed of the three fixed evaluation ceilings.
    ///
    /// The coordinates carry `purrdf-datalog`'s meanings: candidate conclusions
    /// enumerated, facts held when the run stopped, and interned term surface bytes. A
    /// `Simple` run evaluates nothing and reports zero for all three.
    #[must_use]
    pub const fn budget(&self) -> BudgetReport {
        self.budget
    }

    /// The identity of the calculus this closure was minted under.
    ///
    /// Exactly `purrdf_datalog::cache::contract_hash(&calculus_program(regime))`. A
    /// consumer holding a cached closure compares this against the hash of the calculus it
    /// is willing to trust and refuses the closure if they differ — the point being that
    /// the comparison is a digest, not two prose claims about rule coverage.
    #[must_use]
    pub const fn contract_hash(&self) -> ContractHash {
        self.contract_hash
    }

    /// Evidence that the knowledge base is inconsistent, if any was found.
    ///
    /// Always `None` today. See [`InconsistencyWitness`] for why that is an accurate
    /// report rather than an unfinished one.
    #[must_use]
    pub fn inconsistency(&self) -> Option<&InconsistencyWitness> {
        self.inconsistency.as_ref()
    }

    /// Whether this report claims more than its own evidence supports.
    ///
    /// True when [`Completeness::Exact`] is reported alongside a non-empty
    /// [`Self::boundaries`]: the run says it had every rule the regime defines and, in the
    /// same breath, names a construct it could not handle. No report this crate produces
    /// may return `true`, and the crate's tests assert it for every run they make; the
    /// method is public so a consumer assembling reports from several runs can apply the
    /// same gate.
    #[must_use]
    pub fn overclaims(&self) -> bool {
        self.completeness.is_exact() && !self.boundaries.is_empty()
    }
}

/// The rules that fired, in specification table order, zero-count rules omitted.
///
/// Iterates [`RuleId::ALL`] — a `&'static` slice in table order — and sums the tallies of
/// the chase rules that report under each id, so no map iteration and no tally-array order
/// reaches the output. It SUMS rather than assigns because the id is what a report names
/// and the [`ChaseRule`] is what a firing is tallied against: the two are one-to-one
/// today, and a later rule stated under an id an existing rule already carries would be
/// added here rather than silently overwrite it.
fn fired_rules(regime: Regime, stats: &RunStats) -> Vec<(RuleId, u64)> {
    let owl = matches!(regime, Regime::OwlRl);
    RuleId::ALL
        .iter()
        .filter_map(|&id| {
            let count: u64 = ChaseRule::ALL
                .into_iter()
                .filter(|rule| rule.fires_under(regime) && rule.rule_id(owl) == id)
                .map(|rule| stats.fired[rule.index()])
                .sum();
            (count > 0).then_some((id, count))
        })
        .collect()
}

/// The boundaries a run of `regime` over `ds` met, in [`Construct`] declaration order.
///
/// Two kinds, and the distinction is deliberate:
///
/// * OBSERVED — the input actually held the construct ([`Construct::NamedGraph`],
///   [`Construct::TripleTerm`]) or the run actually dropped a conclusion because of it
///   ([`Construct::GeneralizedRdf`]). Emitted only when observed, so the list is evidence
///   about this run rather than a standing disclaimer.
/// * INHERENT — the lane meets the construct for every input, because the rules involved
///   quantify over an infinite set ([`Construct::AxiomaticTriples`],
///   [`Construct::DatatypeValueSpace`]). Emitted for every run of the lane.
///
/// `Simple` meets none of them: the identity closure copies every quad of every graph
/// faithfully, triple terms and literals included, so there is nothing it failed to
/// handle. That is also what keeps [`Completeness::Exact`] honest for that regime.
fn boundaries(ds: &RdfDataset, regime: Regime, stats: &RunStats) -> Vec<Boundary> {
    let survey = match regime {
        Regime::Rdf | Regime::Rdfs | Regime::OwlRl => DatasetSurvey::of(ds),
        // Not this chase's lanes: `Simple` copies faithfully, and the other three never
        // reach here (`materialize` refuses them).
        Regime::Simple | Regime::OwlDirect | Regime::Rif | Regime::D => return Vec::new(),
    };
    Construct::ALL
        .into_iter()
        .filter(|construct| match construct {
            Construct::NamedGraph => survey.named_graph,
            Construct::TripleTerm => survey.triple_term,
            Construct::GeneralizedRdf => stats.generalized_rdf_drops > 0,
            // RDF and RDFS fix the axiomatic triples; OWL 2 RL/RDF deliberately omits
            // them, so its lane does not meet this boundary.
            Construct::AxiomaticTriples => matches!(regime, Regime::Rdf | Regime::Rdfs),
            Construct::DatatypeValueSpace => true,
        })
        .map(Boundary::of)
        .collect()
}
