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
//! * [`ReasoningReport::extensions`] says which of the rules that COULD have fired are not
//!   in any specification table, so a closure that is larger than the normative rule set
//!   licenses says so rather than reading as if it were the specification's own.
//! * [`Boundary`] says which constructs the run could not fully handle, and why.
//! * [`ReasoningReport::contract_hash`] names the calculus, so a cached verdict minted
//!   under a different rule set can be refused rather than trusted.
//!
//! # There is no overclaim, because there is no field to disagree with
//!
//! The failure this report is built against is a certificate that says
//! [`Completeness::Exact`] — "every rule was available AND nothing got in the way" — while
//! [`ReasoningReport::boundaries`] names a construct the run could not handle. A reader of
//! such a report cannot tell which half to believe.
//!
//! That state is not detected here. It is UNREPRESENTABLE: completeness is not a field of
//! [`ReasoningReport`] at all. [`ReasoningReport::completeness`] COMPUTES it, as
//! [`Completeness::for_run`] over the report's own regime and its own boundary list, so
//! `Exact` beside a non-empty boundary list is a value no caller — inside this crate or
//! outside it — has a constructor for. `boundaries_beside_exact_is_unconstructible` is the
//! test that ranges over every regime and every boundary set and finds none.
//!
//! Two earlier revisions tried to police this instead. The first REPAIRED the field inside
//! the constructor and kept a `ReasoningReport::overclaims` predicate over it, which made
//! the predicate a compile-time constant. The second moved the repair out to
//! [`Completeness::for_run`] and had the emission paths CHECK the assembled report — but
//! every one of them called `for_run` with the very list it then stored, so the check still
//! could not fail, `EntailError::Overclaim` was unreachable, and the only test that saw the
//! contradiction hand-built it. Deriving the value instead of storing it ends the question:
//! there is nothing to check because there is nothing that can disagree.
//!
//! # Determinism
//!
//! Every sequence in a report has a fixed, documented order and none of them is built by
//! iterating a map: missing rules and fired rules are in specification table order, and
//! boundaries are in [`Construct`] declaration order. Two identical runs produce
//! byte-identical reports.

use core::fmt;

use purrdf_core::{RdfDataset, TermRef, TermValue};
use purrdf_datalog::cache::ContractHash;
use purrdf_datalog::chase::ChaseTermination;
use purrdf_datalog::seminaive::BudgetReport;

use crate::Regime;
use crate::calculus::{ChaseRule, calculus_contract_hash};
use crate::rules::{RuleId, extensions, implemented, rules};

/// A run's PROOF that the evaluation it describes had to stop.
///
/// The restricted chase invents terms — `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` conclude
/// about a fresh blank node — and a term-inventing fixpoint is not terminating by
/// construction the way a Datalog fixpoint over a fixed active domain is. So
/// `purrdf-datalog` does not assume it: `chase::certify` computes constant-refined weak
/// acyclicity over the clause set's position dependency graph and refuses a program whose
/// existential edge lies in a cycle. This is the certificate that admitted the program
/// PurRDF actually ran, carried out to the caller instead of discarded.
///
/// # It is a NEWTYPE, not `ChaseTermination` re-exported
///
/// `purrdf_datalog::chase::ChaseTermination` has two variants and only one of them can
/// reach a report: an uncertified program is
/// `ChaseError::NonTerminating` and produces no run at all. Storing that enum here would
/// put a variant in a report that no run can carry — the same unrepresentable-state
/// question [`Completeness`] answers by deriving rather than storing — so the report
/// carries the certified case's two numbers and there is no `Unbounded` value to read. Its
/// fields are private and its one (crate-internal) constructor refuses the other variant,
/// so an uncertified verdict has no way in even from inside this crate.
///
/// The sentence is still `purrdf-datalog`'s: [`fmt::Display`] delegates to
/// `ChaseTermination`'s, so the prose has one author and cannot drift.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_entail::{Materialization, materialize};
///
/// let ds = RdfDatasetBuilder::new().freeze().expect("an empty dataset");
/// // A chased lane carries its certificate.
/// let (_, rdfs) = materialize(&ds, Materialization::Rdfs).expect("closed");
/// let certificate = rdfs.termination().expect("the RDFS lane is chased");
/// assert!(certificate.existential_edges() > 0);
/// assert!(certificate.to_string().starts_with("weakly acyclic: "));
/// // A lane that invents no term has no obligation to prove, and says so.
/// let (_, owl) = materialize(&ds, Materialization::OwlRl).expect("closed");
/// assert!(owl.termination().is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminationCertificate {
    /// Distinct refined positions in the position dependency graph — the proof's size.
    positions: usize,
    /// Distinct existential edges checked, none of which lies in a cycle.
    existential_edges: usize,
}

impl TerminationCertificate {
    /// The certificate `termination` states, or `None` if it certified nothing.
    ///
    /// The refusal case is unreachable from a completed run — `chase` returns
    /// `ChaseError::NonTerminating` for it rather than an outcome — so this returning
    /// `None` for `Unbounded` is what makes the type's invariant hold by construction
    /// rather than by convention.
    pub(crate) const fn of_chase(termination: &ChaseTermination) -> Option<Self> {
        match termination {
            ChaseTermination::WeaklyAcyclic {
                positions,
                existential_edges,
            } => Some(Self {
                positions: *positions,
                existential_edges: *existential_edges,
            }),
            ChaseTermination::Unbounded { .. } => None,
        }
    }

    /// How many distinct refined positions the dependency graph holds — the proof's size.
    #[must_use]
    pub const fn positions(&self) -> usize {
        self.positions
    }

    /// How many distinct existential edges were checked, none of them in a cycle.
    #[must_use]
    pub const fn existential_edges(&self) -> usize {
        self.existential_edges
    }
}

impl fmt::Display for TerminationCertificate {
    /// `purrdf-datalog`'s own sentence, delegated rather than restated.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ChaseTermination::WeaklyAcyclic {
            positions: self.positions,
            existential_edges: self.existential_edges,
        }
        .fmt(f)
    }
}

/// How much of `regime`'s specified rule table was available to a run.
///
/// Derived from the inventory by [`Completeness::for_regime`], never asserted at a call
/// site: the value is a function of [`rules`] and [`implemented`] alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Completeness {
    /// Every rule the regime is defined by was available, and the run met NO boundary.
    ///
    /// The strongest thing this crate can say about a closure: the rule table was complete
    /// and nothing outside the rule table got in the way either. `Exact` beside a non-empty
    /// boundary list would contradict itself, which is why no report carries the two as
    /// independent facts — see [`ReasoningReport::completeness`].
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
    /// and knows nothing about a run. [`Self::for_run`] is the function that has both facts
    /// in scope, and it is the only place [`Self::Exact`] is narrowed to this.
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

    /// `regime`'s completeness for a RUN that met `boundaries`.
    ///
    /// [`Self::for_regime`] is a function of the inventory and cannot see a run;
    /// [`ReasoningReport::boundaries`] is a function of a run and cannot see the inventory.
    /// This is the one function both facts are in scope for, and it is where a complete
    /// rule table that still met a construct becomes [`Self::ExactWithinBoundaries`] —
    /// the honest way to say "every rule was available AND something got in the way".
    ///
    /// It is the DEFINITION of a report's completeness rather than a fix-up applied on the
    /// way in: [`ReasoningReport`] stores no completeness field, and
    /// [`ReasoningReport::completeness`] is exactly this call over the report's own regime
    /// and boundary list. So `Exact` beside a non-empty boundary list is not a state this
    /// crate detects and refuses — it is a state nothing can build.
    ///
    /// ```
    /// use purrdf_entail::{Boundary, Completeness, Construct, Regime};
    ///
    /// // No boundary met: the inventory's answer stands.
    /// assert_eq!(Completeness::for_run(Regime::Rdfs, &[]), Completeness::Exact);
    /// // A boundary met: the same complete rule table, said honestly.
    /// assert_eq!(
    ///     Completeness::for_run(Regime::Rdfs, &[Boundary::of(Construct::Surrogate)]),
    ///     Completeness::ExactWithinBoundaries
    /// );
    /// ```
    #[must_use]
    pub fn for_run(regime: Regime, boundaries: &[Boundary]) -> Self {
        match Self::for_regime(regime) {
            Self::Exact if !boundaries.is_empty() => Self::ExactWithinBoundaries,
            other => other,
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
/// The reverse mapping ([`materialize_dl_reported`](crate::materialize_dl_reported)) once answered `Ok(())`
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
    /// A TBox axiom outside the Horn fragment the combined approach's restricted chase
    /// can lower into a `DlClause` program — so query answering for a basic graph pattern
    /// carrying a non-distinguished variable fell back to whole-vocabulary augmentation
    /// instead of the combined approach's chase-and-filter answer.
    NonHornTBox,
}

impl Construct {
    /// Every construct, in declaration order — the order a report lists boundaries in.
    pub(crate) const ALL: [Self; 14] = [
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
        Self::NonHornTBox,
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
            Self::NonHornTBox => "non-horn-tbox",
        }
    }

    /// Why this construct is a boundary — the technical reason, not an apology.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NamedGraph => {
                "RDF HAS NO STANDARD ENTAILMENT RELATION FOR A DATASET, so what a run over \
                 one does is a DEFINED CHOICE rather than a derived consequence, and this \
                 boundary is where PurRDF states which choice it made. RDF 1.2 Semantics \
                 defines entailment over a GRAPH and SPARQL's entailment regimes are \
                 defined over the ACTIVE graph; neither says what a dataset entails, and \
                 the union, the per-graph reading and the quad-level reading are three \
                 different answers no specification picks between. PurRDF's defined \
                 behaviour is: the DEFAULT graph is closed against itself; each NAMED graph \
                 is closed against the union of itself and the default graph; and a \
                 conclusion lands in the graph that PRODUCED it, so a conclusion the \
                 default graph already draws on its own is a default-graph conclusion and \
                 is not restated in a named graph that also reached it. Two named graphs \
                 therefore never join — neither is ever in the other's seed — and the \
                 layout that motivates the whole choice works: a terminology in the default \
                 graph and instances in a named graph derive into the NAMED graph. The cost \
                 is real and is measured rather than hidden: a dataset with n named graphs \
                 is 1 + n evaluations of the same declared program, whose join steps are \
                 SUMMED into the budget while the two occupancy coordinates report the peak \
                 single store"
            }
            Self::TripleTerm => {
                "rdfs14 and rdfs14a replace a triple term with a FRESH blank node typed \
                 rdfs:Proposition, and both FIRE: the restricted chase evaluates their \
                 existentially quantified heads, minting each surrogate as a \
                 frontier-addressed Skolem witness. What is withheld is the SURROGATE \
                 itself — a SPARQL entailment regime draws its answers from the scoping \
                 graph and a minted blank node is not in it — so the rdfs:Proposition node \
                 is not a term any answer can bind, and every conclusion mentioning one is \
                 dropped at the materialization boundary and counted by \
                 ReasoningReport::withheld_surrogates. See the surrogate boundary for why \
                 that exclusion is REQUIRED rather than convenient. \
                 \
                 The triple term itself is interned as one atomic term the chase never \
                 looks inside, the closure states nothing about the triple the term quotes, \
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
                 datatyped literal, or for an inhabited value space, and both FIRE: the \
                 restricted chase evaluates their existentially quantified heads. What is \
                 withheld is the SURROGATE itself — a SPARQL entailment regime draws its \
                 answers from the scoping graph, which no minted blank node is in — and \
                 the surrogate boundary is where those withheld conclusions are counted. \
                 rdfs1 recognizes the datatypes \
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
                 and the SHOIQ(D) completion procedure here decides a hierarchy of SIMPLE \
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
                "a DATA RANGE this run could not decide EXACTLY. OWL 2's data ranges are \
                 concrete-domain expressions — subsets of the data domain rather than of \
                 owl:Thing — and the tableau decides them by asking purrdf-xsd whether the \
                 intersection of the ranges on a node is EMPTY, over the XSD value spaces \
                 themselves. So an owl:onDatatype with xsd:minInclusive / xsd:maxInclusive / \
                 xsd:minExclusive / xsd:maxExclusive / xsd:length / xsd:minLength / \
                 xsd:maxLength facets, an owl:datatypeComplementOf (complemented against the \
                 WHOLE data domain, so a complement of rdfs:Literal is empty), an \
                 intersection or union of data ranges, an owl:oneOf over literals and an \
                 owl:onDataRange in a qualified cardinality are all read and DECIDED; a class \
                 whose members must inhabit an empty range is unsatisfiable, a literal whose \
                 lexical form is outside its datatype's lexical space denotes nothing and \
                 makes the ontology inconsistent, and two literals are one element of the data \
                 domain exactly when they denote one VALUE — \"1\"^^xsd:integer and \
                 \"01\"^^xsd:integer always, \"5\"^^xsd:integer and \"5.0\"^^xsd:decimal \
                 because OWL 2's datatype map nests the integers in the decimals, and \
                 \"5\"^^xsd:float, \"5\"^^xsd:double and \"5\"^^xsd:decimal never, because \
                 that map makes those three value spaces pairwise disjoint. \
                 \
                 What is left, and what this boundary names, is where the decision procedure \
                 answers UNDECIDED rather than proving emptiness or exhibiting a value. It is \
                 raised on exactly that answer, so the boundary and the procedure cannot drift \
                 apart, and there are five ways to reach it. FIRST, the xsd:pattern and \
                 rdf:langRange facets: deciding whether a regular language intersected with a \
                 complemented one is empty is an automaton product construction, and \
                 rdf:langRange constrains rdf:langString's value space rather than an XSD one. \
                 SECOND, a datatype outside the modelled value space — owl:real, owl:rational, \
                 xsd:anyURI, a caller's own rdfs:Datatype — or a value beyond the representable \
                 domain: an unmodelled value space may OVERLAP a modelled one (every \
                 xsd:decimal value is an owl:real value), so it cannot be assumed disjoint \
                 either. THIRD, the TEMPORAL value spaces beyond what a listed set of values \
                 can say: a bound facet once the range is COMPLEMENTED, and xsd:dayTimeDuration \
                 or xsd:yearMonthDuration as whole datatypes, which are infinite proper \
                 subspaces of one duration space. The XSD order on all of them is PARTIAL — a \
                 timezone-less xsd:dateTime is incomparable with one whose offset falls inside \
                 the fourteen-hour indeterminacy window, and xsd:duration's order has two \
                 independent components — so an interval's complement is not again a union of \
                 intervals and the exact set algebra the decision rests on does not close \
                 there. What DOES decide is every temporal enumeration and its complement, and \
                 every uncomplemented bound: contradictory bounds prove emptiness and a \
                 satisfiable inclusive bound exhibits its own endpoint. FOURTH, a facet that \
                 does not apply to its base datatype's value space — a bound on a string, a \
                 length on a number, any facet on xsd:boolean, a bound drawn from a different \
                 value space than the base, a NaN bound — and an enumeration over \
                 xsd:float/xsd:double that must separate positive zero from negative zero, \
                 which the interval order over those spaces cannot. Neither is silently \
                 dropped: under a complement a dropped constraint SHRINKS the range and would \
                 invent an emptiness the ontology does not state. FIFTH, an n-ary data range \
                 (owl:onProperties over one owl:onDataRange), for which OWL 2 defines no \
                 datatype at all, so no datatype map entry exists to decide it against. \
                 \
                 An undecided range is never read as a clash. Every branch it touches stays \
                 open, which loses conclusions and invents none — the only direction a \
                 reasoner can be wrong in and recover"
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
            Self::NonHornTBox => {
                "the combined approach answers a basic graph pattern with a non-distinguished \
                 variable by lowering the TBox's SIMPLE existential shape — `A ⊑ B` and \
                 `A ⊑ ∃r.B` over named classes and a named role — into `purrdf-datalog`'s \
                 DL-clause IR, running the RESTRICTED CHASE to mint a frontier-Skolem witness \
                 for every existential, and filtering any answer that would bind a \
                 DISTINGUISHED (projected) variable to a minted witness. That lowering \
                 recognizes exactly two axiom shapes; an ontology whose TBox holds anything \
                 else reachable from a `rdfs:subClassOf` triple — an equivalence, a \
                 disjointness, a class expression on the SUBCLASS side, a restriction that is \
                 not a plain `owl:someValuesFrom` of a named class over a named property, or \
                 any of the other OWL 2 constructs the reverse mapping's own boundaries name \
                 — falls outside it, and so does a Horn-shaped TBox whose existentials the \
                 chase cannot certify terminating (a genuine schema-level cycle through the \
                 existential positions). Either way the run falls back to the pre-existing \
                 query-directed whole-vocabulary augmentation and its own — narrower — \
                 guarantees, which is sound but may miss a certain answer that only a \
                 non-distinguished variable's binding to a chase witness would have found"
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
/// # It reaches the caller on the ERROR, and it reaches it IN A REPORT
///
/// OWL 2 RL derives an inconsistency through the seventeen rules whose conclusion is
/// `false` — `eq-diff1`, `eq-diff2`, `eq-diff3`, `prp-irp`, `prp-asyp`, `prp-pdw`,
/// `prp-adp`, `prp-npa1`, `prp-npa2`, `cls-nothing2`, `cls-com`, `cls-maxc1`, `cls-maxqc1`,
/// `cls-maxqc2`, `cax-dw`, `cax-adc`, `dt-not-type` — and the `OWL-RL` and `D` lanes
/// evaluate all of them. A body match on one is
/// [`EntailError::Inconsistent`](crate::EntailError): an inconsistent knowledge base
/// entails every triple, so there is no closure to hand back.
///
/// There is still a RUN to describe, though, and losing it was a real hole: the refusal
/// used to carry this witness alone, so the one caller who most needed to know which rules
/// had fired, what the evaluation had cost and which calculus refused was the only caller
/// who got none of it — the report-free variant this crate says it does not have.
/// [`EntailError::Inconsistent`](crate::EntailError) therefore carries an
/// [`InconsistentRun`]: this witness AND the run's
/// [`ReasoningReport`], whose [`ReasoningReport::inconsistency`] is this same witness.
///
/// [`ReasoningReport::inconsistency`] is `None` on a report that accompanies a closure —
/// "seventeen rules looked and found nothing", a CHECKED fact rather than a vacuous one —
/// and `Some` on the report that accompanies a refusal. The RDF and RDFS lanes have no
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
    /// The graph whose closure refused; `None` is the default graph.
    graph: Option<TermValue>,
}

impl InconsistencyWitness {
    /// A witness that `rule` fired on `premises` while closing `graph`.
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

    /// The graph whose CLOSURE refused; `None` is the default graph.
    ///
    /// A dataset is closed graph by graph — the default graph against itself, each named
    /// graph against the union of itself and the default graph — so a named graph's run has
    /// both in its seed and a premise may be asserted in either. Naming the graph being
    /// CLOSED is therefore the accurate claim, and it is the one a caller needs: it says
    /// which closure is unusable. Reading it as "every premise is asserted here" would be an
    /// overclaim; the premises themselves are in [`Self::premises`], to be looked for in
    /// this graph and in the default graph.
    #[must_use]
    pub fn graph(&self) -> Option<&TermValue> {
        self.graph.as_ref()
    }
}

/// An inconsistent run's two halves: the evidence, and the certificate.
///
/// The payload of [`EntailError::Inconsistent`](crate::EntailError), and the reason that
/// error is not the exception to "the report is not optional". A caller whose data is
/// inconsistent gets no closure — an inconsistent knowledge base entails every triple —
/// but the run still happened, still cost a budget, still fired rules and still ran under a
/// named calculus, and all four are things the caller needs in order to act on the refusal:
/// which rules had already produced conclusions, how far the evaluation got, and which
/// contract hash the verdict was minted under.
///
/// The witness is reachable twice — [`Self::witness`] and
/// [`ReasoningReport::inconsistency`] on [`Self::report`] — and that is deliberate rather
/// than redundant. The first is where a caller matching on the error looks; the second is
/// what makes `inconsistency` an observable field on the report surface every host renders,
/// instead of a constant `none` no input could ever move.
#[derive(Debug, Clone)]
pub struct InconsistentRun {
    /// The rule that refused, and the asserted triples that satisfied it.
    witness: InconsistencyWitness,
    /// What the run had done when it stopped.
    report: ReasoningReport,
}

impl InconsistentRun {
    /// The run `witness` refused, described by `report`.
    ///
    /// `report`'s [`ReasoningReport::inconsistency`] is expected to be that same witness;
    /// `ReasoningReport::of_inconsistent_run` is how this crate builds the pair.
    #[must_use]
    pub const fn new(witness: InconsistencyWitness, report: ReasoningReport) -> Self {
        Self { witness, report }
    }

    /// The rule whose premises were all satisfied, and the triples that satisfied them.
    #[must_use]
    pub const fn witness(&self) -> &InconsistencyWitness {
        &self.witness
    }

    /// What the run had done when it stopped — budget, fired rules, boundaries, calculus.
    #[must_use]
    pub const fn report(&self) -> &ReasoningReport {
        &self.report
    }

    /// Take the two halves apart.
    #[must_use]
    pub fn into_parts(self) -> (InconsistencyWitness, ReasoningReport) {
        (self.witness, self.report)
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
    /// The proof that admitted the program, for a lane the restricted chase evaluated.
    ///
    /// `None` for a lane the semi-naive evaluator ran, and that is a statement rather than
    /// a missing measurement: a program of definite clauses invents no term, so its
    /// fixpoint is bounded by the active domain and there is no obligation to prove. The
    /// certificate is a function of the CLAUSE SET, so every graph of a multi-graph run
    /// computes the same one and [`RunStats::certify`] records it rather than combining
    /// it.
    pub(crate) termination: Option<TerminationCertificate>,
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
            termination: None,
        }
    }

    /// Credit one committed, materialized conclusion to `rule`.
    pub(crate) const fn commit(&mut self, rule: ChaseRule) {
        self.fired[rule.index()] += 1;
    }

    /// Fold ONE graph's evaluation into this tally.
    ///
    /// A dataset is closed graph by graph — see [`crate::engine`] for the defined semantics
    /// — so a run over a dataset with `n` named graphs is `1 + n` evaluations and the
    /// report has to describe all of them at once. The three coordinates are aggregated
    /// under their own meanings rather than under one convenient rule:
    ///
    /// * `join_steps` is WORK, so it SUMS. The number a caller wants is what the whole run
    ///   enumerated, and reporting one graph's slice of it would understate the cost of the
    ///   very semantics that multiplied it.
    /// * `stored_facts` and `term_arena_bytes` are OCCUPANCY of one store, and each
    ///   evaluation gets its own store which is dropped when it ends. The ceiling they are
    ///   measured against is per-store, so the honest aggregate is the PEAK — the largest
    ///   single store the run ever held — and summing them would report a footprint that
    ///   never existed at any instant.
    ///
    /// A dataset with no named graph is one evaluation, for which the sum and the peak are
    /// both that evaluation's own figure, so nothing about a single-graph run moves.
    pub(crate) fn absorb(&mut self, budget: BudgetReport) {
        self.budget = BudgetReport::new(
            self.budget.join_steps().saturating_add(budget.join_steps()),
            self.budget.stored_facts().max(budget.stored_facts()),
            self.budget
                .term_arena_bytes()
                .max(budget.term_arena_bytes()),
        );
    }

    /// Record `count` conclusions the RDF 1.2 IR could not hold.
    pub(crate) const fn drop_generalized(&mut self, count: u64) {
        self.generalized_rdf_drops += count;
    }

    /// Record `count` conclusions withheld because they mention a surrogate blank node.
    pub(crate) const fn drop_surrogate(&mut self, count: u64) {
        self.surrogate_drops += count;
    }

    /// Record the certificate that admitted this run's program.
    ///
    /// Assigned rather than folded: `certify` is a pure function of the clause set, so the
    /// `1 + n` graph evaluations of one `materialize` call all compute the same
    /// certificate, and combining `n` copies of one value would suggest they could differ.
    pub(crate) const fn certify(&mut self, certificate: Option<TerminationCertificate>) {
        if let Some(certificate) = certificate {
            self.termination = Some(certificate);
        }
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
    /// Whether any quad of ANY graph mentions a triple term.
    triple_term: bool,
}

impl DatasetSurvey {
    /// Survey `ds`.
    ///
    /// The triple-term question ranges over EVERY graph, not the default one alone,
    /// because every graph is now reasoned over: a named graph is closed against the union
    /// of itself and the default graph, so a triple term sitting in one is a term this
    /// crate's chase cannot look inside exactly as a default-graph one is.
    fn of(ds: &RdfDataset) -> Self {
        let mut survey = Self::default();
        for quad in ds.quad_refs() {
            if quad.g.is_some() {
                survey.named_graph = true;
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
    ///
    /// There is deliberately no `completeness` field beside it: completeness is a function
    /// of this regime and [`Self::boundaries`], and [`Self::completeness`] computes it. A
    /// stored copy could disagree with the boundary list it summarizes, and that
    /// disagreement — a certificate claiming a complete answer while naming a construct it
    /// could not handle — is the one thing this whole type exists to make impossible.
    regime: Regime,
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
    /// The termination proof that admitted the program, for a lane the chase evaluated.
    termination: Option<TerminationCertificate>,
}

impl ReasoningReport {
    /// A report holding exactly these facts.
    ///
    /// The one constructor every other one goes through, and the reason a
    /// self-contradicting certificate has no constructor anywhere: there is no
    /// `completeness` parameter. A report's completeness is
    /// `Completeness::for_run(regime, boundaries)` by definition and
    /// [`Self::completeness`] computes it on demand, so [`Completeness::Exact`] beside a
    /// non-empty `boundaries` is not a value a caller can pass in — not from this crate,
    /// and not from a consumer assembling a report of its own.
    ///
    /// The contract hash is not a parameter either: it is
    /// `calculus_contract_hash(regime)` by definition, and a report naming a calculus other
    /// than the one its regime declares would be a second contradiction with no honest
    /// reading.
    ///
    /// `rules_fired` and `boundaries` are stored in the order given; this crate supplies
    /// specification table order and [`Construct`] declaration order respectively, which is
    /// what makes two identical runs byte-identical.
    ///
    /// ```
    /// use purrdf_datalog::seminaive::BudgetReport;
    /// use purrdf_entail::{Boundary, Completeness, Construct, ReasoningReport, Regime};
    ///
    /// // Name a boundary and the completeness follows it — there is no second argument
    /// // that could have said `Exact` here.
    /// let bounded = ReasoningReport::new(
    ///     Regime::Rdfs,
    ///     Vec::new(),
    ///     vec![Boundary::of(Construct::Surrogate)],
    ///     BudgetReport::new(0, 0, 0),
    ///     None,
    ///     0,
    ///     None,
    /// );
    /// assert_eq!(bounded.completeness(), Completeness::ExactWithinBoundaries);
    ///
    /// // Name none, and the same regime's complete rule table reads `Exact`.
    /// let plain = ReasoningReport::new(
    ///     Regime::Rdfs,
    ///     Vec::new(),
    ///     Vec::new(),
    ///     BudgetReport::new(0, 0, 0),
    ///     None,
    ///     0,
    ///     None,
    /// );
    /// assert_eq!(plain.completeness(), Completeness::Exact);
    /// ```
    #[must_use]
    pub fn new(
        regime: Regime,
        rules_fired: Vec<(RuleId, u64)>,
        boundaries: Vec<Boundary>,
        budget: BudgetReport,
        inconsistency: Option<InconsistencyWitness>,
        withheld_surrogates: u64,
        termination: Option<TerminationCertificate>,
    ) -> Self {
        Self {
            regime,
            rules_fired,
            boundaries,
            budget,
            contract_hash: calculus_contract_hash(regime),
            inconsistency,
            withheld_surrogates,
            termination,
        }
    }

    /// Assemble the report for a run of `regime` over `ds` that measured `stats`.
    pub(crate) fn of_run(ds: &RdfDataset, regime: Regime, stats: &RunStats) -> Self {
        Self::of_chase_run(ds, regime, stats, None)
    }

    /// Assemble the report for a run of `regime` over `ds` that `witness` REFUSED.
    ///
    /// The certificate an inconsistent input gets. Everything in it describes the run up
    /// to the clash and is measured, not stubbed: the budget is what the evaluation had
    /// consumed, `rules_fired` is what had already been committed (a dataset is closed
    /// graph by graph, so a clash in the third graph leaves the first two's conclusions
    /// tallied), the boundaries are the ones that run met, and the contract hash names the
    /// calculus that refused. [`Self::inconsistency`] is `Some(witness)`, which is the
    /// state that makes that accessor a report of a fact rather than a constant.
    ///
    /// There is no closure to accompany it — an inconsistent knowledge base entails every
    /// triple — so this report reaches the caller on
    /// [`EntailError::Inconsistent`](crate::EntailError), inside the
    /// [`InconsistentRun`] that also carries the witness.
    pub(crate) fn of_inconsistent_run(
        ds: &RdfDataset,
        regime: Regime,
        stats: &RunStats,
        witness: InconsistencyWitness,
    ) -> Self {
        Self::of_chase_run(ds, regime, stats, Some(witness))
    }

    /// The shared body of [`Self::of_run`] and [`Self::of_inconsistent_run`].
    fn of_chase_run(
        ds: &RdfDataset,
        regime: Regime,
        stats: &RunStats,
        inconsistency: Option<InconsistencyWitness>,
    ) -> Self {
        Self::new(
            regime,
            fired_rules(regime, stats),
            boundaries(ds, regime, stats),
            stats.budget,
            inconsistency,
            stats.surrogate_drops,
            stats.termination,
        )
    }

    /// Assemble the report for an `OWL-Direct` run that met `boundaries`.
    ///
    /// The DL lane has no rule TABLE — it is a tableau, so [`rules`] and [`implemented`]
    /// are both empty for it and [`Completeness::for_regime`] answers
    /// [`Completeness::Exact`] vacuously. What it does have is CONSTRUCTS, and
    /// [`Completeness::for_run`] — which [`Self::completeness`] applies to whatever
    /// boundary list this report ends up carrying — is where they narrow that vacuous
    /// `Exact` to [`Completeness::ExactWithinBoundaries`]: a run over an ontology carrying
    /// an `owl:propertyChainAxiom` has no missing rule to report and is still not a
    /// complete answer.
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
        Self::new(
            Regime::OwlDirect,
            Vec::new(),
            boundaries,
            BudgetReport::new(0, 0, 0),
            // The tableau reports an unsatisfiable knowledge base as
            // `EntailError::Unsatisfiable`, which carries no rule and no premise set, so
            // there is no `InconsistencyWitness` to attach and a report that exists is a
            // report of a satisfiable knowledge base.
            None,
            // The tableau invents no surrogate: it decides satisfiability rather than
            // materializing a closure, so there is nothing to withhold.
            0,
            // No chase ran, so there is no acyclicity verdict to carry. The tableau's own
            // bound is a step cap, and exhausting it is a refusal rather than a run with a
            // weaker certificate.
            None,
        )
    }

    /// The regime the caller asked for.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// How much of the regime's specified rule table was available to the run, and whether
    /// the run met anything outside it.
    ///
    /// COMPUTED, not stored: exactly `Completeness::for_run(self.regime(), self.boundaries())`.
    /// That is what makes [`Completeness::Exact`] beside a non-empty [`Self::boundaries`]
    /// unconstructible rather than merely unwelcome — there is no field a bad assembly
    /// could set, so the certificate cannot contradict its own evidence and no run-time
    /// check has to look for the case.
    ///
    /// It returns by VALUE because it is a derivation. Callers that want the missing-rule
    /// slice across statements bind it first — `let completeness = report.completeness();`
    /// — rather than borrowing from a temporary.
    #[must_use]
    pub fn completeness(&self) -> Completeness {
        Completeness::for_run(self.regime, &self.boundaries)
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

    /// The rules the run's calculus states that NO specification table does — exactly what
    /// [`extensions`] answers for this report's own regime.
    ///
    /// The twin of `completeness().missing()`, and the other half of the same question. A
    /// `missing` id is a rule the specification defines and this chase does not fire, so
    /// the closure may be smaller than the regime requires. An extension is a rule this
    /// chase fires and no specification defines, so the closure may be LARGER — sound
    /// still, but larger — and a caller that must not act on a conclusion outside the
    /// normative table needs to be told which rules those are before it reads
    /// [`Self::rules_fired`].
    ///
    /// It is DERIVED from the regime, exactly as completeness is derived from the
    /// boundary list, so no report can carry an extension list that disagrees with the
    /// calculus its contract hash names. Empty for every lane but `OWL-RL`, whose single
    /// entry is `ext-eq-diff-sym`.
    ///
    /// ```
    /// use purrdf_core::RdfDatasetBuilder;
    /// use purrdf_entail::{Materialization, RuleId, materialize};
    ///
    /// let ds = RdfDatasetBuilder::new().freeze().expect("an empty dataset");
    /// let (_, report) = materialize(&ds, Materialization::OwlRl).expect("a consistent closure");
    /// assert_eq!(report.extensions(), &[RuleId::ExtEqDiffSym]);
    ///
    /// let (_, report) = materialize(&ds, Materialization::Rdfs).expect("a consistent closure");
    /// assert!(report.extensions().is_empty());
    /// ```
    #[must_use]
    pub fn extensions(&self) -> &'static [RuleId] {
        extensions(self.regime)
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
    /// `None` on the report that accompanies a CLOSURE — that run's seventeen `false`-headed
    /// rules looked and found nothing, which is a checked consistency claim rather than an
    /// unfilled field. `Some` on the report that accompanies a REFUSAL: an inconsistent
    /// input has no closure, but it still gets its certificate, carried inside
    /// [`EntailError::Inconsistent`](crate::EntailError)'s
    /// [`InconsistentRun`] beside the witness itself.
    ///
    /// So the two states are both reachable and both observable, and a caller can tell
    /// which one it is holding without matching on an error: see [`InconsistencyWitness`]
    /// for what the witness names.
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

    /// The PROOF that this run's evaluation had to stop, when one was needed.
    ///
    /// `Some` exactly for the two lanes whose rule tables state an existentially
    /// quantified conclusion — `RDF` (`rdfD1`, `rdfD1a`) and `RDFS` (those two plus
    /// `rdfs14`, `rdfs14a`). Those are evaluated by `purrdf-datalog`'s restricted chase,
    /// which INVENTS terms, and a term-inventing fixpoint has to be shown to converge
    /// rather than assumed to: the chase computes constant-refined weak acyclicity over
    /// the clause set's position dependency graph before it runs a round, and refuses the
    /// program outright if an existential edge lies in a cycle. This is that computation's
    /// verdict, carried out rather than thrown away.
    ///
    /// `None` for every other lane, and that is a claim rather than an absence: `Simple`,
    /// `OWL-RL` and `D` state no existential rule, so their programs invent no term, their
    /// fixpoints are bounded by the active domain, and there is nothing for an acyclicity
    /// analysis to be about. `OWL-Direct` and `RIF` are not this chase's lanes at all.
    ///
    /// # It can only ever be a certificate, and it is not always the same one
    ///
    /// The uncertified verdict never reaches here — an existential edge in a cycle is
    /// `ChaseError::NonTerminating`, so the run produces no report to carry it — which is
    /// why [`TerminationCertificate`] holds the certified case's numbers and no variant
    /// for the other. What the numbers say still varies: the certificate is a function of
    /// the CLAUSE SET, so the `RDF` lane's differs from the `RDFS` lane's, and a rule
    /// added to either moves that lane's. It does not vary with the data.
    #[must_use]
    pub const fn termination(&self) -> Option<TerminationCertificate> {
        self.termination
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
            // The eight OWL-Direct boundaries are the reverse mapping's (and the combined
            // approach's), raised by `ReasoningReport::of_dl_run` from the axioms and the
            // query it actually read. No chase lane parses an OWL class expression or runs
            // the combined approach's own TBox lowering at all, so none of them can be met
            // here — and the arm is written out rather than defaulted so a ninth construct
            // has to decide which side of the split it is on.
            Construct::PropertyChain
            | Construct::NonSimpleRole
            | Construct::DataRange
            | Construct::BuiltinRole
            | Construct::OntologyImport
            | Construct::UnrecognizedTerm
            | Construct::NonDistinguishedVariable
            | Construct::NonHornTBox => false,
        })
        .map(Boundary::of)
        .collect()
}
