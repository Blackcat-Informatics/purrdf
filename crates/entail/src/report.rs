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
    /// Every rule the regime is defined by was available, and the run met NO boundary.
    ///
    /// The strongest thing this crate can say about a closure: the rule table was complete
    /// and nothing outside the rule table got in the way either. A report that claims
    /// `Exact` with a non-empty boundary list is contradicting itself; see
    /// [`ReasoningReport::overclaims`].
    Exact,
    /// Every rule the regime is defined by was available, and the run STILL met a
    /// construct it could not fully handle.
    ///
    /// A complete rule table and an incomplete closure are not a contradiction, and this
    /// variant is what stops them being reported as one. `OWL-RL` reaches it on essentially
    /// every input: the chase fires all seventy-eight rules of Tables 4–9, and three of
    /// them (`dt-type2`, `dt-eq`, `dt-diff`) quantify over ALL literals while a forward
    /// chase can only range over the ones the dataset holds — the
    /// [`Construct::DatatypeValueSpace`] boundary — and `eq-ref` concludes about literal
    /// SUBJECTS the RDF 1.2 IR cannot hold — the [`Construct::GeneralizedRdf`] boundary.
    ///
    /// [`Self::for_regime`] never returns this: it is a function of the INVENTORY alone
    /// and knows nothing about a run. The report assembled for a RUN narrows
    /// [`Self::Exact`] to it when that run actually met a boundary, which is the only
    /// place both facts are in scope.
    ExactWithinBoundaries,
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
    /// // OWL 2 RL defines 78 rules, and this crate's chase fires all of them.
    /// assert!(Completeness::for_regime(Regime::OwlRl).missing().is_empty());
    /// // RDFS's four blank-node-minting rules are evaluated by the restricted chase, so
    /// // its table is complete too.
    /// assert!(Completeness::for_regime(Regime::Rdfs).missing().is_empty());
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
    ///
    /// True for [`Self::Exact`] AND for [`Self::ExactWithinBoundaries`]: both say the rule
    /// TABLE was complete, and they differ only in whether the run met a construct outside
    /// it. A caller asking "did the chase have every rule?" gets one answer; a caller
    /// asking "is this closure everything the regime entails?" must read
    /// [`ReasoningReport::boundaries`] too, which is exactly the distinction the second
    /// variant exists to make visible.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        matches!(self, Self::Exact | Self::ExactWithinBoundaries)
    }

    /// The missing rules, in specification table order; empty for both exact variants.
    #[must_use]
    pub fn missing(&self) -> &[RuleId] {
        match self {
            Self::Exact | Self::ExactWithinBoundaries => &[],
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
/// Variants are declared in the order a report lists them: the three the CHASE observes
/// first (the input held the construct, or a conclusion was actually abandoned because of
/// it), then the two that are INHERENT to a chase lane and hold for every input, then the
/// six the OWL-Direct reverse mapping raises when an axiom it cannot fully handle is read,
/// and last the one the OWL-Direct QUERY layer raises — about the shape of the question
/// rather than about the ontology. The derived [`Ord`] follows that declaration order.
///
/// # The OWL-Direct block exists so an axiom is never SILENTLY dropped
///
/// The reverse mapping ([`materialize_dl`](crate::materialize_dl)) once answered `Ok(())`
/// for any structural triple it did not recognize, so `owl:propertyChainAxiom`,
/// `owl:imports`, a datatype restriction and a mistyped `owl:` term all vanished without a
/// word — and one of them was worse than vanishing, because a chain axiom fell into the
/// catch-all and was ingested as a role ASSERTION whose object was the RDF list head. The
/// six variants below are what replaced that: every OWL 2 construct the layer reads either
/// reaches the knowledge base or raises one of them, and
/// `every_owl2_construct_is_handled_or_bounded` is the test that admits no third answer.
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
    /// The SURROGATE blank nodes `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` invent.
    Surrogate,
    /// `owl:propertyChainAxiom` — a COMPLEX role inclusion.
    PropertyChain,
    /// A number restriction over a role a property chain or a transitivity axiom makes
    /// NON-SIMPLE, which OWL 2 DL forbids.
    NonSimpleRole,
    /// OWL 2's data ranges — the concrete domain.
    DataRange,
    /// `owl:topObjectProperty` / `owl:bottomObjectProperty` and their data siblings — the
    /// two BUILT-IN roles whose extension the semantics fixes.
    BuiltinRole,
    /// `owl:imports` — an ontology document this run was not handed.
    OntologyImport,
    /// A term of the reserved `owl:`/`rdf:`/`rdfs:` vocabulary the reverse mapping does not
    /// recognize.
    UnrecognizedTerm,
    /// A NON-DISTINGUISHED variable in a query basic graph pattern — a blank node the
    /// `OWL-Direct` layer was handed that is not part of a class expression.
    NonDistinguishedVariable,
}

impl Construct {
    /// Every construct, in declaration order — the order a report lists boundaries in.
    pub(crate) const ALL: [Self; 13] = [
        Self::NamedGraph,
        Self::TripleTerm,
        Self::GeneralizedRdf,
        Self::AxiomaticTriples,
        Self::DatatypeValueSpace,
        Self::Surrogate,
        Self::PropertyChain,
        Self::NonSimpleRole,
        Self::DataRange,
        Self::BuiltinRole,
        Self::OntologyImport,
        Self::UnrecognizedTerm,
        Self::NonDistinguishedVariable,
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
            Self::Surrogate => "surrogate",
            Self::PropertyChain => "property-chain",
            Self::NonSimpleRole => "non-simple-role",
            Self::DataRange => "data-range",
            Self::BuiltinRole => "builtin-role",
            Self::OntologyImport => "ontology-import",
            Self::UnrecognizedTerm => "unrecognized-term",
            Self::NonDistinguishedVariable => "non-distinguished-variable",
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
                 and a conclusion built AROUND such a term carries it through unchanged. \
                 owl:sameAs does NOT substitute inside one either: eq-rep-s, eq-rep-p and \
                 eq-rep-o rewrite a triple's own positions, so <<( :a :p :b )>> and \
                 <<( :a :p :c )>> stay two terms even when :b owl:sameAs :c, and the \
                 congruence is complete over terms rather than over their contents"
            }
            Self::GeneralizedRdf => {
                "a conclusion whose subject position would hold a literal or a triple term, \
                 or whose predicate position would hold anything but an IRI, is a \
                 generalized-RDF triple, which the RDF 1.2 dataset IR cannot represent; \
                 such a conclusion is derived in the evaluator's own term space and then \
                 abandoned when the answer is materialized, rather than a term being \
                 fabricated for it. It is still a PREMISE, so nothing downstream is lost. \
                 Rules that can conclude into subject position (rdfs3 / prp-rng, \
                 prp-symp, prp-inv1, prp-inv2, eq-ref, eq-rep-p) are therefore incomplete \
                 over literal objects, and every conclusion of dt-type2, dt-eq and dt-diff \
                 is unrepresentable by construction, because each of the three puts a \
                 literal in subject position"
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
                 rdf:dirLangString, xsd:string) and no others. The OWL 2 RL rules \
                 quantified over value spaces (dt-type2, dt-eq, dt-diff) DO fire, over \
                 the literals the dataset holds rather than over the infinitely many a \
                 value space contains, and value-space membership is decided through the \
                 candidate datatype's LEXICAL space, so a value in a datatype's value \
                 space whose lexical form is not in that datatype's lexical space is not \
                 found: \"1.0\"^^xsd:decimal is not typed xsd:integer. A datatype \
                 purrdf-xsd does not model is not judged either way"
            }
            Self::Surrogate => {
                "rdfD1, rdfD1a, rdfs14 and rdfs14a all conclude about a FRESH blank node — \
                 the surrogate the specification writes `_:nnn`. All four FIRE: the \
                 restricted chase mints each surrogate as a frontier-addressed Skolem \
                 witness, so re-deriving the same obligation reuses the same witness and \
                 the fixpoint converges, and the closure is closed under everything the \
                 surrogate then licenses. What does not reach the answer is the surrogate \
                 ITSELF: every conclusion mentioning one is dropped at the materialization \
                 boundary and counted here. \
                 \
                 That is a requirement of SPARQL's entailment regimes, not a shortcut. The \
                 regime's answers are drawn from the SCOPING GRAPH, and a surrogate is not \
                 in it, so a solution binding a variable to one is not an answer the regime \
                 admits. The W3C case `rdfs13` is the proof: it asks `?L rdf:type \
                 rdfs:Literal` over a graph whose only literal is \"foo\" and requires ZERO \
                 rows, while rdfD1's surrogate gives `_:nnn rdf:type xsd:string` and hence \
                 `_:nnn rdf:type rdfs:Literal` through rdfs1, rdfs13 and rdfs9. Emitting \
                 that row would be WRONG, where withholding it is merely incomplete. \
                 \
                 Nothing surrogate-FREE is lost by the exclusion: replacing a term by a \
                 fresh blank node can only weaken a triple, so every conclusion that does \
                 not mention a surrogate was already licensed by the triple the surrogate \
                 stands for"
            }
            Self::PropertyChain => {
                "owl:propertyChainAxiom states a COMPLEX role inclusion p₁ ∘ … ∘ pₙ ⊑ p, \
                 and the ALCOIQ completion procedure here decides a hierarchy of SIMPLE \
                 roles: it closes a role over its asserted sub-roles and inverses, which \
                 is a reachability question, whereas a chain axiom needs the role hierarchy \
                 compiled into a non-deterministic finite automaton per role and the \
                 hierarchy itself checked for REGULARITY (SROIQ's acyclicity condition on \
                 the chain order) before that automaton exists. Neither is implemented, so \
                 a chain axiom raises this boundary instead of being read. It is emphatically \
                 not dropped: the reverse mapping's catch-all used to ingest one as a ROLE \
                 ASSERTION over the axiom's RDF list head, which stated something the \
                 ontology does not say. The OWL 2 RL lane's prp-spo2 DOES walk a chain, so \
                 this boundary is the DL lane's alone"
            }
            Self::NonSimpleRole => {
                "OWL 2 DL requires the role of a number restriction (owl:minCardinality, \
                 owl:maxCardinality, owl:cardinality and their qualified forms) to be \
                 SIMPLE — not transitive, not the super-role of a transitive role, and not \
                 the head of a property chain — because counting successors of a composite \
                 role is undecidable. An ontology that violates the restriction is not \
                 OWL 2 DL, so the tableau neither decides it nor guesses at it: the \
                 restriction is read and this boundary is raised beside the answer, naming \
                 the syntactic condition the input broke"
            }
            Self::DataRange => {
                "OWL 2's data ranges — owl:onDatatype with owl:withRestrictions facets, \
                 owl:datatypeComplementOf, owl:onDataRange, owl:onProperties and an \
                 owl:oneOf over literals — are CONCRETE-DOMAIN expressions, and this \
                 tableau has no concrete-domain decision procedure: a literal is an opaque \
                 term with no value-space structure, so \"5\"^^xsd:integer and \
                 \"5.0\"^^xsd:decimal are two terms and a facet such as xsd:minInclusive \
                 cannot be evaluated at all. Reading a data range as an ABSTRACT class \
                 expression would be a WRONG answer rather than an incomplete one — it \
                 would make the datatype an ordinary named class and admit models the \
                 datatype map forbids — so the range is not read and this boundary is \
                 raised. A data PROPERTY assertion is still ingested: its object is an \
                 opaque term, which is exactly what an abstract role edge needs"
            }
            Self::BuiltinRole => {
                "owl:topObjectProperty / owl:topDataProperty (the UNIVERSAL role, whose \
                 extension is every pair of the domain) and owl:bottomObjectProperty / \
                 owl:bottomDataProperty (the EMPTY role) are built-in: their extension is \
                 fixed by the semantics rather than by the ontology. The role machinery \
                 here addresses a role by its IRI and closes it over the ASSERTED hierarchy \
                 and inverses only, so a built-in role would read as an ordinary named one \
                 — an assertion over the empty role would fail to clash, and a universal \
                 role would connect nothing. Raising the boundary says which of the two \
                 answers the run is not entitled to"
            }
            Self::OntologyImport => {
                "owl:imports names another ontology DOCUMENT, and OWL 2's imports closure \
                 is the union of the importing ontology with every document it transitively \
                 names. This layer reasons over the dataset it was handed and fetches \
                 nothing — PurRDF performs no I/O, has no network and must stay \
                 wasm32-clean — so the imported axioms are premises this run did not have. \
                 A caller who wants them merges the documents before calling; the boundary \
                 is what stops the run pretending the merge already happened"
            }
            Self::UnrecognizedTerm => {
                "a term in the reserved owl:, rdf: or rdfs: vocabulary that the OWL-2-RDF \
                 reverse mapping here does not recognize. Reserved vocabulary is not user \
                 vocabulary, so ingesting such a triple as an ordinary role assertion would \
                 be a WRONG reading rather than an incomplete one — it would put a \
                 structural term in the ABox as though it were an individual. The triple is \
                 therefore neither read nor discarded in silence: it raises this boundary, \
                 which is also what a mistyped or newer-than-this-release OWL term looks \
                 like from inside"
            }
            Self::NonDistinguishedVariable => {
                "SPARQL reads a blank node in a query as an EXISTENTIAL variable, and the \
                 OWL-Direct layer answers a basic graph pattern by injecting the entailed \
                 GROUND atoms over the query's vocabulary and letting simple entailment \
                 join them. That decomposition is exact for a substitution into the scoping \
                 graph — `KB ⊨ (t₁ ∧ … ∧ tₙ)σ` iff `KB ⊨ tᵢσ` for each i, because each \
                 conjunct is then ground — and it does NOT hold for a non-distinguished \
                 variable: an open-world model may satisfy `∃x. A(x) ∧ B(x)` through an \
                 ANONYMOUS element, and no finite augmentation can name one. This is a \
                 property of the problem rather than a shortfall of the construction, so \
                 the run reports the residue instead of claiming to have closed it. A query \
                 blank node that IS the scaffold of a class expression is not this: it is \
                 ground syntax the reverse mapping reads, and it raises nothing"
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
/// # It reaches the caller on the ERROR, not in a report
///
/// OWL 2 RL derives an inconsistency through the seventeen rules whose conclusion is
/// `false` — `eq-diff1`, `eq-diff2`, `eq-diff3`, `prp-irp`, `prp-asyp`, `prp-pdw`,
/// `prp-adp`, `prp-npa1`, `prp-npa2`, `cls-nothing2`, `cls-com`, `cls-maxc1`, `cls-maxqc1`,
/// `cls-maxqc2`, `cax-dw`, `cax-adc`, `dt-not-type` — and the `OWL-RL` and `D` lanes
/// evaluate all of them. A body match on one is
/// [`EntailError::Inconsistent`](crate::EntailError) carrying this value: an inconsistent
/// knowledge base entails every triple, so there is no closure to hand back and no report
/// to attach it to.
///
/// [`ReasoningReport::inconsistency`] is therefore `None` on every report that exists, and
/// that is now a CHECKED fact rather than a vacuous one — "seventeen rules looked and
/// found nothing", where before it meant "nothing looked". The RDF and RDFS lanes have no
/// inconsistency rule at all, so their `None` is a statement about the calculus.
///
/// # Which rule, which facts, which graph
///
/// The premises are the ASSERTED triples that satisfied the rule, in the rule's own
/// premise order, so a reader can line them up against the specification's rule-table
/// entry. A premise that matched one of this crate's INTERNAL relations — the RDF-list
/// index `eq-diff2`, `eq-diff3`, `prp-adp` and `cax-adc` read, the value-space judgement
/// `dt-not-type` reads — is bookkeeping rather than an asserted triple and is left out:
/// a caller looking for `LIST(head, index, member)` in their data would not find it.
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
    /// Conclusions dropped because they mention a SURROGATE blank node the chase invented.
    /// The [`Construct::Surrogate`] boundary's observation; see its reason for why a
    /// SPARQL entailment regime may not answer with one.
    pub(crate) surrogate_drops: u64,
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
            surrogate_drops: 0,
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

    /// Record one conclusion withheld because it mentions a surrogate blank node.
    pub(crate) const fn drop_surrogate(&mut self) {
        self.surrogate_drops += 1;
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
    /// How many conclusions were withheld because they mention a surrogate blank node.
    withheld_surrogates: u64,
}

impl ReasoningReport {
    /// Assemble the report for a run of `regime` over `ds` that measured `stats`.
    ///
    /// The completeness is the INVENTORY's answer, narrowed by this run's evidence: a
    /// complete rule table that still met a boundary is
    /// [`Completeness::ExactWithinBoundaries`], because saying `Exact` beside a boundary
    /// list is the overclaim [`Self::overclaims`] exists to forbid. Narrowing here rather
    /// than in [`Completeness::for_regime`] is deliberate — that function is a pure
    /// function of the inventory and has no run to look at.
    pub(crate) fn of_run(ds: &RdfDataset, regime: Regime, stats: &RunStats) -> Self {
        let boundaries = boundaries(ds, regime, stats);
        let completeness = match Completeness::for_regime(regime) {
            Completeness::Exact if !boundaries.is_empty() => Completeness::ExactWithinBoundaries,
            other => other,
        };
        Self {
            regime,
            completeness,
            rules_fired: fired_rules(regime, stats),
            boundaries,
            budget: stats.budget,
            contract_hash: calculus_contract_hash(regime),
            // A run that WITNESSES an inconsistency is refused, so a report that exists at
            // all is a report of a run that found none; see [`InconsistencyWitness`].
            inconsistency: None,
            withheld_surrogates: stats.surrogate_drops,
        }
    }

    /// Assemble the report for an `OWL-Direct` run that met `boundaries`.
    ///
    /// The DL lane has no rule TABLE — it is a tableau, so [`rules`] and [`implemented`]
    /// are both empty for it and [`Completeness::for_regime`] answers
    /// [`Completeness::Exact`] vacuously. What it does have is CONSTRUCTS, and this
    /// constructor is where they narrow that vacuous `Exact` to
    /// [`Completeness::ExactWithinBoundaries`]: a run over an ontology carrying an
    /// `owl:propertyChainAxiom` has no missing rule to report and is still not a complete
    /// answer, and saying `Exact` beside a non-empty boundary list is precisely the
    /// overclaim [`Self::overclaims`] forbids.
    ///
    /// `boundaries` arrives as a set, so it is already deduplicated; it is re-ordered here
    /// into [`Construct`] declaration order, which is the order every report lists
    /// boundaries in.
    ///
    /// The budget is zero on all three coordinates and that is a fact about the lane
    /// rather than an unfilled field: the tableau is not `purrdf-datalog`'s evaluator and
    /// consumes none of its three ceilings. Its own bound is a step cap whose exhaustion is
    /// [`EntailError::Build`](crate::EntailError), never a truncated answer.
    pub(crate) fn of_dl_run(boundaries: &std::collections::BTreeSet<Construct>) -> Self {
        let boundaries: Vec<Boundary> = Construct::ALL
            .into_iter()
            .filter(|construct| boundaries.contains(construct))
            .map(Boundary::of)
            .collect();
        let completeness = match Completeness::for_regime(Regime::OwlDirect) {
            Completeness::Exact if !boundaries.is_empty() => Completeness::ExactWithinBoundaries,
            other => other,
        };
        Self {
            regime: Regime::OwlDirect,
            completeness,
            rules_fired: Vec::new(),
            boundaries,
            budget: BudgetReport::new(0, 0, 0),
            contract_hash: calculus_contract_hash(Regime::OwlDirect),
            // The tableau reports an unsatisfiable knowledge base as
            // `EntailError::Unsatisfiable`, which carries no rule and no premise set, so
            // there is no `InconsistencyWitness` to attach and a report that exists is a
            // report of a satisfiable knowledge base.
            inconsistency: None,
            // The tableau invents no surrogate: it decides satisfiability rather than
            // materializing a closure, so there is nothing to withhold.
            withheld_surrogates: 0,
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
    /// Always `None`, and that is a CHECKED consistency claim rather than an unfilled
    /// field: a run that witnesses an inconsistency is REFUSED, so the witness reaches the
    /// caller on [`EntailError::Inconsistent`](crate::EntailError) and a report exists only
    /// for a run that found none. See [`InconsistencyWitness`].
    #[must_use]
    pub fn inconsistency(&self) -> Option<&InconsistencyWitness> {
        self.inconsistency.as_ref()
    }

    /// How many conclusions were WITHHELD because they mention a surrogate blank node.
    ///
    /// `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` conclude about a fresh `_:nnn`, and a
    /// SPARQL entailment regime draws its answers from the scoping graph — so a solution
    /// binding a variable to a surrogate is not an answer it admits, and every conclusion
    /// mentioning one is dropped at the materialization boundary. This is the count of
    /// them, and it is what makes the [`Construct::Surrogate`] boundary a MEASUREMENT
    /// rather than a standing disclaimer.
    ///
    /// It is also the only thing a caller can observe about those four rules: their
    /// conclusions are withheld by construction, so they can never appear in
    /// [`Self::rules_fired`] — which counts triples that entered the closure — and a
    /// non-zero count here is the evidence that they fired at all. Zero for every lane
    /// that states none of the four (`Simple`, `OWL-RL`, `D`, `OWL-Direct`).
    #[must_use]
    pub const fn withheld_surrogates(&self) -> u64 {
        self.withheld_surrogates
    }

    /// Whether this report claims more than its own evidence supports.
    ///
    /// True when [`Completeness::Exact`] — the variant that means "and nothing got in the
    /// way" — is reported alongside a non-empty [`Self::boundaries`]: the run would be
    /// saying it answered everything and, in the same breath, naming a construct it could
    /// not handle. [`Completeness::ExactWithinBoundaries`] is the honest way to say the
    /// first half of that, and it does not trip the gate.
    ///
    /// No report this crate produces may return `true`, and the crate's tests assert it
    /// for every run they make; the method is public so a consumer assembling reports from
    /// several runs can apply the same gate.
    #[must_use]
    pub fn overclaims(&self) -> bool {
        matches!(self.completeness, Completeness::Exact) && !self.boundaries.is_empty()
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
        Regime::Rdf | Regime::Rdfs | Regime::OwlRl | Regime::D => DatasetSurvey::of(ds),
        // Not this chase's lanes: `Simple` copies faithfully, and the other two never
        // reach here (`materialize` refuses them).
        Regime::Simple | Regime::OwlDirect | Regime::Rif => return Vec::new(),
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
            Construct::Surrogate => stats.surrogate_drops > 0,
            // The six OWL-Direct boundaries are the reverse mapping's, raised by
            // `ReasoningReport::of_dl_run` from the axioms it actually read. No chase lane
            // parses an OWL class expression at all, so none of them can be met here — and
            // the arm is written out rather than defaulted so a seventh construct has to
            // decide which side of the split it is on.
            Construct::PropertyChain
            | Construct::NonSimpleRole
            | Construct::DataRange
            | Construct::BuiltinRole
            | Construct::OntologyImport
            | Construct::UnrecognizedTerm
            | Construct::NonDistinguishedVariable => false,
        })
        .map(Boundary::of)
        .collect()
}
