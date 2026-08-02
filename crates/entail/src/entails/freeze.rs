// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **freeze-and-chase** mechanism: decide a universally-quantified Horn axiom by
//! generalisation on constants.
//!
//! # The gap this closes
//!
//! [`homomorphism`](super::homomorphism) matches a conclusion into the closure, and
//! [`refutation`](super::refutation) decides a conclusion the table has no head for by
//! reading its seventeen `false`-concluding rules as an inconsistency calculus. Neither
//! reaches a SCHEMA AXIOM. `p rdf:type owl:TransitiveProperty` is not a negative fact, so
//! there is nothing to refute; and OWL 2 Profiles Theorem PR1 claims completeness only for
//! ASSERTIONAL conclusions — a `ClassAssertion`, an `ObjectPropertyAssertion`, a
//! `DataPropertyAssertion` or a `SameIndividual` over named individuals — so a schema axiom
//! missing from the closure is not a fact about the premise. W3C's `chain2trans1` is exactly
//! that case: `p owl:propertyChainAxiom (p p)` entails `p rdf:type owl:TransitiveProperty`,
//! and every mechanism above misses it.
//!
//! For a property CHARACTERISTIC the table has no head at all, so the miss is total. For an
//! INCLUSION it is not: Table 9's `scm-sco`, `scm-eqc1`/`scm-eqc2`, `scm-spo` and
//! `scm-eqp1`/`scm-eqp2` all conclude one, the chase fires them, and the derived edges do
//! reach the closure — they just close the ASSERTED hierarchy under transitivity and
//! equivalence rather than deriving an inclusion the ontology's other axioms force. So this
//! lane is needed for both shapes, and for one reason: PR1 does not claim completeness for
//! either, whatever `scm-*` happens to derive.
//!
//! # The mechanism, and the theorem it is an instance of
//!
//! An OWL schema axiom of the shapes below ABBREVIATES a universally-quantified implication.
//! `p rdf:type owl:TransitiveProperty` says
//!
//! > `p ∈ IOOP` **and** `∀x,y,z ∈ IR: p(x,y) ∧ p(y,z) → p(x,z)`
//!
//! and both halves are owed. The membership half is an ordinary ground question, answered by
//! looking in the premise's own closure. The implication half is decided by **generalisation
//! on constants**, which is the classical theorem
//!
//! > if `Γ ⊢ φ(c₁ … cₙ)` and `c₁ … cₙ` are distinct constants occurring nowhere in `Γ`, then
//! > `Γ ⊢ ∀x₁ … xₙ. φ(x₁ … xₙ)`.
//!
//! So: FREEZE the implication's body over distinct constants the premise does not mention —
//! `_:a p _:b`, `_:b p _:c` — CHASE `premise ∪ body`, and look for the head, `_:a p _:c`. For
//! `chain2trans1` the head arrives through **`prp-spo2`**, one of the normative seventy-eight:
//! the premise's own `p owl:propertyChainAxiom (p p)` is exactly a rule body that concludes
//! `_:a p _:c` from those two frozen atoms. Nothing outside the table did any work.
//!
//! # Why the frozen constants may be BLANK NODES
//!
//! The chase treats a blank node as an opaque constant. It interns it into the term table
//! beside every IRI, matches it in a rule body position like any other term, and never
//! quantifies over it — the surrogate machinery that DOES invent blank nodes is a different
//! path, reported through [`Construct::Surrogate`](crate::Construct::Surrogate), and it does
//! not touch a term the caller seeded. So a derivation over `premise ∪ {p(a,b), p(b,c)}` for
//! frozen blank nodes `a`, `b`, `c` is a derivation over three fresh CONSTANTS, which is the
//! hypothesis the theorem above needs.
//!
//! That reading is Skolemisation, and it is used in the direction Skolemisation preserves.
//! Reading the existentials of a graph as fresh constants is sound for the ENTAILING side —
//! `sk(G) ⊨ E` implies `G ⊨ E` is the direction that can fail, and it is not the direction
//! used here, because the frozen atoms are not part of the caller's premise at all: they are
//! the antecedent of an implication this module is proving, and the theorem on constants
//! discharges them by generalisation rather than by existential instantiation. Concretely,
//! the argument is:
//!
//! > Suppose the chase over `premise ∪ {body(a,b,c)}` derives `head(a,c)`. The rule set is
//! > SOUND, so `premise ∪ {body(a,b,c)} ⊨ head(a,c)`. Take any model `I` of `premise` and any
//! > `u,v,w ∈ IR` satisfying the body. Because `a`, `b`, `c` occur nowhere in `premise`, the
//! > interpretation `I'` that agrees with `I` everywhere and maps `a,b,c ↦ u,v,w` is still a
//! > model of `premise`, and it satisfies `body(a,b,c)`; hence it satisfies `head(a,c)`,
//! > i.e. `head(u,w)` holds in `I`. So `I` satisfies the implication, for every `I`. ∎
//!
//! The converse — that a genuinely entailed characteristic is always reached this way — is
//! NOT claimed, and this module never answers "not entailed". A frozen instance that derives
//! nothing is handed back to [`precondition`](super::precondition) exactly as a failed match
//! is. See [`combined`](crate::combined), and the `owl_dl::saturate` module beside it, for the
//! two other places in this crate where a soundness argument is written out beside the code it
//! licenses; this is the third, and it is the same discipline.
//!
//! # An inconsistent frozen instance establishes the axiom VACUOUSLY
//!
//! If `premise ∪ {body(a,b,c)}` is INCONSISTENT then, by the same substitution argument, no
//! model of `premise` has any `u,v,w` satisfying the body — so the implication holds with an
//! empty antecedent and the axiom is entailed. This is not an edge case to be tolerated but
//! an outcome to be REPORTED: a premise stating `p rdfs:domain owl:Nothing` entails every
//! characteristic of `p`, and a mechanism that let the frozen run's inconsistency escape as
//! [`EntailError::Inconsistent`] would report the CALLER's premise as having no model when it
//! has one. So the clash is caught here, carried in the warrant as
//! [`FrozenOutcome::Vacuous`], and re-checked like any other evidence.
//!
//! # Why NOT the DL tableau
//!
//! Because it would answer confidently and wrongly. `owl:propertyChainAxiom` is
//! [`Support::Bounded(Construct::PropertyChain)`](crate::Construct::PropertyChain) in the
//! reverse mapping's construct table, which means the DL knowledge base is built WITHOUT the
//! chain axiom. A tableau asked "is `p` transitive?" over that knowledge base is asked about
//! a premise from which the only axiom that makes it transitive has been dropped, and it
//! would answer `False` — a wrong answer with a certificate attached, which is worse than no
//! answer. The chase keeps the axiom, because `prp-spo2` is one of its own rules.
//!
//! # Applicability is a WHITELIST
//!
//! `SHAPES` below is the whole table, and a conclusion triple that is not one of its shapes is
//! RESIDUAL — it carries its ordinary obligation to map into the premise's closure, which is
//! neither weakened nor pretended to have been discharged. Two exclusions are worth naming
//! because they are decisions rather than omissions:
//!
//! * `owl:IrreflexiveProperty` and `owl:AsymmetricProperty` are property characteristics and
//!   are NOT here. Their defining conditions have `false` as the head — they are not Horn —
//!   so freezing a body and looking for a head is not what deciding them is. They belong to
//!   [`refutation`](super::refutation)'s calculus, not to this one.
//! * `owl:ReflexiveProperty` is not here either: its condition has an EMPTY body, so there is
//!   nothing to freeze and the chase would be asked to derive `a p a` for a constant nothing
//!   constrains. Deciding a conclusion `x p x` FROM an asserted reflexive property is a
//!   different question, and it is not this one.
//!
//! # The seed's pre-passes, and the whitelist that protects them
//!
//! The engine's `Refuter` seeds the premise once and re-closes it against each frozen body as an insert
//! delta, which is only sound because the delta cannot disturb the three pre-passes the seed
//! computed. A frozen body's atoms carry no literal (every term is an IRI of the conclusion
//! or a minted blank node), so the datatype pre-pass is untouched. The COLLECTION pre-pass is
//! protected by a check rather than by luck: a body atom's predicate position may hold one of
//! the axiom's own named terms, and a conclusion is free to say
//! `rdf:first rdf:type owl:TransitiveProperty`. Such an axiom is refused —
//! `a_list_valued_predicate_is_refused` is the falsifiable form — because freezing
//! `_:a rdf:first _:b` would make the seeded collection walk a walk of a different graph.
//!
//! # Cost, and the budget that bounds it
//!
//! One chase per implication instance, which is at most two per recognized conclusion triple.
//! That is LINEAR in the conclusion the caller wrote, with no combinatorial factor — but the
//! unit is a complete evaluation of the seventy-eight-rule program, so the count is still
//! bounded, by [`FREEZE_BUDGET`]. Exhausting it is
//! [`super::UndecidedReason::FreezeBudget`] and never an
//! establishment or a refusal: "I stopped" is not "there is nothing to find".
//!
//! # Determinism
//!
//! Axioms are read in the conclusion's own frozen triple order, implications in table order,
//! constants in mint order, and the premise is seeded once. Two runs over one premise and one
//! conclusion produce the same warrant, on `wasm32` as on native.

use std::collections::BTreeSet;

use purrdf_core::{RdfDataset, TermValue};

use crate::calculus::concludes_false;
use crate::engine::Refuter;
use crate::entails::fresh::{FreshBlanks, labels_of};
use crate::entails::graph::{Triple, default_graph_triples, show};
use crate::entails::homomorphism::{Binding, Closure};
use crate::entails::membership::Membership;
use crate::entails::warrant::{EntailmentMechanism, EntailmentWarrant, Replay};
use crate::entails::{Attempt, Established, Question, Recognized, UndecidedReason};
use crate::lists::LIST_VALUED;
use crate::report::InconsistencyWitness;
use crate::vocab::{
    OWL_EQUIVALENTCLASS, OWL_EQUIVALENTPROPERTY, OWL_FUNCTIONALPROPERTY,
    OWL_INVERSEFUNCTIONALPROPERTY, OWL_SAMEAS, OWL_SYMMETRICPROPERTY, OWL_TRANSITIVEPROPERTY,
    RDF_FIRST, RDF_REST, RDF_TYPE, RDFS_DOMAIN, RDFS_RANGE, RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF,
};
use crate::{EntailError, Regime};

/// The freeze budget, in CHASE RE-RUNS per [`entails`](super::entails) call.
///
/// A step count and never a clock reading, so the bound is reproducible on every target
/// including `wasm32`. Small, because the unit is enormous: one step is a complete evaluation
/// of the seventy-eight-rule program over the premise plus two frozen atoms. Sized so that
/// every conclusion in the W3C entailment corpus, and any hand-written conclusion stating a
/// few dozen schema axioms at once, finishes.
pub const FREEZE_BUDGET: u64 = 64;

// ── The shape table ────────────────────────────────────────────────────────────────────

/// One position of a frozen atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// The axiom triple's subject — a named class or property of the caller's vocabulary.
    Subject,
    /// The axiom triple's object, for a shape whose object is a second named term.
    Object,
    /// A universally-quantified variable, by index; one frozen constant per index.
    Var(usize),
    /// A fixed term of the reserved vocabulary.
    Iri(&'static str),
}

/// One universally-quantified Horn implication an axiom abbreviates.
#[derive(Debug)]
struct Implication {
    /// The atoms that are FROZEN over constants and asserted into the premise.
    body: &'static [[Slot; 3]],
    /// The atom the chase must then derive.
    head: [Slot; 3],
}

/// One recognized conclusion shape: how to spot it, what it presupposes, what it implies.
///
/// The membership columns are the conjunct a reader most often forgets is there — see
/// [`Membership`] — and leaving one unchecked would make this mechanism claim an axiom whose
/// typing half nothing established.
#[derive(Debug)]
struct Shape {
    /// The predicate of the conclusion triple that states this axiom.
    predicate: &'static str,
    /// `Some(class)` when the axiom is a TYPING and the object is that reserved class, so the
    /// axiom names one term; `None` when the object is the axiom's second named term.
    typed_as: Option<&'static str>,
    /// The membership the subject must have.
    subject_is: Membership,
    /// The membership the object must have, for a two-term axiom.
    object_is: Option<Membership>,
    /// The implications the axiom abbreviates. EVERY one must be established — an
    /// equivalence that held in one direction and not the other would not be an equivalence.
    implications: &'static [Implication],
}

/// The whitelist: every conclusion shape this mechanism reads, and nothing else.
///
/// Each row is the axiom's semantic condition transcribed. The membership columns are the
/// conjunct a reader most often forgets is there — `rdfs:subClassOf` is `c,d ∈ IC` AND
/// `ICEXT(c) ⊆ ICEXT(d)`, not the inclusion alone — and leaving one unchecked would make this
/// mechanism claim an axiom whose typing half nothing established.
static SHAPES: &[Shape] = &[
    // `p ∈ IOOP` and `∀x,y,z: p(x,y) ∧ p(y,z) → p(x,z)`.
    Shape {
        predicate: RDF_TYPE,
        typed_as: Some(OWL_TRANSITIVEPROPERTY),
        subject_is: Membership::ObjectProperty,
        object_is: None,
        implications: &[Implication {
            body: &[
                [Slot::Var(0), Slot::Subject, Slot::Var(1)],
                [Slot::Var(1), Slot::Subject, Slot::Var(2)],
            ],
            head: [Slot::Var(0), Slot::Subject, Slot::Var(2)],
        }],
    },
    // `p ∈ IOOP` and `∀x,y: p(x,y) → p(y,x)`.
    Shape {
        predicate: RDF_TYPE,
        typed_as: Some(OWL_SYMMETRICPROPERTY),
        subject_is: Membership::ObjectProperty,
        object_is: None,
        implications: &[Implication {
            body: &[[Slot::Var(0), Slot::Subject, Slot::Var(1)]],
            head: [Slot::Var(1), Slot::Subject, Slot::Var(0)],
        }],
    },
    // `p ∈ IP` and `∀x,y,z: p(x,y) ∧ p(x,z) → y = z`.
    Shape {
        predicate: RDF_TYPE,
        typed_as: Some(OWL_FUNCTIONALPROPERTY),
        subject_is: Membership::Property,
        object_is: None,
        implications: &[Implication {
            body: &[
                [Slot::Var(0), Slot::Subject, Slot::Var(1)],
                [Slot::Var(0), Slot::Subject, Slot::Var(2)],
            ],
            head: [Slot::Var(1), Slot::Iri(OWL_SAMEAS), Slot::Var(2)],
        }],
    },
    // `p ∈ IOOP` and `∀x,y,z: p(x,z) ∧ p(y,z) → x = y`.
    Shape {
        predicate: RDF_TYPE,
        typed_as: Some(OWL_INVERSEFUNCTIONALPROPERTY),
        subject_is: Membership::ObjectProperty,
        object_is: None,
        implications: &[Implication {
            body: &[
                [Slot::Var(0), Slot::Subject, Slot::Var(2)],
                [Slot::Var(1), Slot::Subject, Slot::Var(2)],
            ],
            head: [Slot::Var(0), Slot::Iri(OWL_SAMEAS), Slot::Var(1)],
        }],
    },
    // `c,d ∈ IC` and `ICEXT(c) ⊆ ICEXT(d)`.
    Shape {
        predicate: RDFS_SUBCLASSOF,
        typed_as: None,
        subject_is: Membership::Class,
        object_is: Some(Membership::Class),
        implications: &[Implication {
            body: &[[Slot::Var(0), Slot::Iri(RDF_TYPE), Slot::Subject]],
            head: [Slot::Var(0), Slot::Iri(RDF_TYPE), Slot::Object],
        }],
    },
    // `c,d ∈ IC` and `ICEXT(c) = ICEXT(d)` — the inclusion, both ways.
    Shape {
        predicate: OWL_EQUIVALENTCLASS,
        typed_as: None,
        subject_is: Membership::Class,
        object_is: Some(Membership::Class),
        implications: &[
            Implication {
                body: &[[Slot::Var(0), Slot::Iri(RDF_TYPE), Slot::Subject]],
                head: [Slot::Var(0), Slot::Iri(RDF_TYPE), Slot::Object],
            },
            Implication {
                body: &[[Slot::Var(0), Slot::Iri(RDF_TYPE), Slot::Object]],
                head: [Slot::Var(0), Slot::Iri(RDF_TYPE), Slot::Subject],
            },
        ],
    },
    // `p,q ∈ IP` and `EXT(p) ⊆ EXT(q)`.
    Shape {
        predicate: RDFS_SUBPROPERTYOF,
        typed_as: None,
        subject_is: Membership::Property,
        object_is: Some(Membership::Property),
        implications: &[Implication {
            body: &[[Slot::Var(0), Slot::Subject, Slot::Var(1)]],
            head: [Slot::Var(0), Slot::Object, Slot::Var(1)],
        }],
    },
    // `p,q ∈ IP` and `EXT(p) = EXT(q)`.
    Shape {
        predicate: OWL_EQUIVALENTPROPERTY,
        typed_as: None,
        subject_is: Membership::Property,
        object_is: Some(Membership::Property),
        implications: &[
            Implication {
                body: &[[Slot::Var(0), Slot::Subject, Slot::Var(1)]],
                head: [Slot::Var(0), Slot::Object, Slot::Var(1)],
            },
            Implication {
                body: &[[Slot::Var(0), Slot::Object, Slot::Var(1)]],
                head: [Slot::Var(0), Slot::Subject, Slot::Var(1)],
            },
        ],
    },
    // `p ∈ IP`, `c ∈ IC` and `∀x,y: p(x,y) → c(x)`.
    Shape {
        predicate: RDFS_DOMAIN,
        typed_as: None,
        subject_is: Membership::Property,
        object_is: Some(Membership::Class),
        implications: &[Implication {
            body: &[[Slot::Var(0), Slot::Subject, Slot::Var(1)]],
            head: [Slot::Var(0), Slot::Iri(RDF_TYPE), Slot::Object],
        }],
    },
    // `p ∈ IP`, `c ∈ IC` and `∀x,y: p(x,y) → c(y)`.
    Shape {
        predicate: RDFS_RANGE,
        typed_as: None,
        subject_is: Membership::Property,
        object_is: Some(Membership::Class),
        implications: &[Implication {
            body: &[[Slot::Var(0), Slot::Subject, Slot::Var(1)]],
            head: [Slot::Var(1), Slot::Iri(RDF_TYPE), Slot::Object],
        }],
    },
];

// ── The evidence ───────────────────────────────────────────────────────────────────────

/// How one frozen instance came out.
#[derive(Debug, Clone)]
pub enum FrozenOutcome {
    /// The chase DERIVED the head from the premise and the frozen body.
    Derived,
    /// `premise ∪ body` has no model at all, so the implication holds with an empty
    /// antecedent. Carries the `false`-concluding rule that fired and what satisfied it.
    Vacuous(InconsistencyWitness),
}

/// One implication, frozen over constants and decided by a chase.
#[derive(Debug, Clone)]
pub struct FrozenInstance {
    /// The constants the variables were frozen over, by variable index.
    constants: Vec<TermValue>,
    /// The body, instantiated — the atoms asserted into the premise.
    body: Vec<Triple>,
    /// The head, instantiated — the atom the chase had to reach.
    head: Triple,
    /// Which way it came out.
    outcome: FrozenOutcome,
    /// The closure of `premise ∪ body`: the seeded triples plus everything derived.
    closure: Closure,
}

impl FrozenInstance {
    /// The constants this instance froze the implication's variables over.
    ///
    /// Distinct, and absent from both the premise and the conclusion — which is the
    /// hypothesis of the theorem on constants and is re-decided by
    /// [`verify`](super::verify) rather than promised here.
    #[must_use]
    pub fn constants(&self) -> &[TermValue] {
        &self.constants
    }

    /// The frozen body: the atoms asserted into the premise.
    #[must_use]
    pub fn body(&self) -> &[Triple] {
        &self.body
    }

    /// The head the chase had to reach.
    #[must_use]
    pub const fn head(&self) -> &Triple {
        &self.head
    }

    /// Which way this instance came out.
    #[must_use]
    pub const fn outcome(&self) -> &FrozenOutcome {
        &self.outcome
    }

    /// How many distinct triples the closure of `premise ∪ body` holds.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        self.closure.len()
    }

    /// Re-decide this instance against `premise`, WITHOUT running a reasoner.
    ///
    /// `frozen` is the set of labels the whole warrant froze over, so the non-occurrence
    /// hypothesis is checked against the caller's own premise and conclusion rather than
    /// against the generator that produced them.
    fn check(&self, premise: &[Triple], forbidden: &BTreeSet<String>) -> bool {
        // 1. THE HYPOTHESIS OF THE THEOREM. Distinct constants, each a blank node, none of
        //    them occurring in the premise or in the conclusion. Without this the derivation
        //    is a statement about particular individuals and generalising it is a
        //    non-sequitur, so it is the first thing checked and not the last.
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for constant in &self.constants {
            let TermValue::Blank { label, .. } = constant else {
                return false;
            };
            if forbidden.contains(label) || !seen.insert(label.as_str()) {
                return false;
            }
        }
        // 2. The closure really is a closure OF the cited premise and the frozen body.
        if !premise
            .iter()
            .chain(&self.body)
            .all(|triple| self.closure.contains(triple))
        {
            return false;
        }
        // 3. …and it reached the head, or it reached `false`.
        match &self.outcome {
            FrozenOutcome::Derived => self.closure.contains(&self.head),
            FrozenOutcome::Vacuous(witness) => {
                concludes_false(witness.rule())
                    && witness.premises().iter().all(|body| {
                        self.closure.contains(&[
                            body.subject().clone(),
                            body.predicate().clone(),
                            body.object().clone(),
                        ])
                    })
            }
        }
    }
}

/// WHY one schema axiom of the conclusion holds: its membership half, and its implication
/// half instance by instance.
#[derive(Debug, Clone)]
pub struct Generalization {
    /// The conclusion triple this establishes.
    axiom: Triple,
    /// The closure triples that establish the axiom's memberships — `p ∈ IOOP`, `c ∈ IC`.
    typings: Vec<Triple>,
    /// One frozen instance per implication the axiom abbreviates, in table order.
    instances: Vec<FrozenInstance>,
}

impl Generalization {
    /// The conclusion triple this establishes.
    #[must_use]
    pub const fn axiom(&self) -> &Triple {
        &self.axiom
    }

    /// The closure triples that establish the axiom's membership half.
    #[must_use]
    pub fn typings(&self) -> &[Triple] {
        &self.typings
    }

    /// One frozen instance per implication, in table order.
    #[must_use]
    pub fn instances(&self) -> &[FrozenInstance] {
        &self.instances
    }
}

impl std::fmt::Display for Generalization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            show(&self.axiom[0]),
            show(&self.axiom[1]),
            show(&self.axiom[2])
        )
    }
}

/// The evidence that a premise entails a conclusion whose schema axioms were frozen and
/// chased.
///
/// Mixed like [`RefutationWarrant`](super::RefutationWarrant), and for the same reason: a
/// conclusion states a schema axiom BESIDE ordinary triples — `chain2trans1` concludes
/// `p rdf:type owl:TransitiveProperty` and, in the same graph, `<> rdf:type owl:Ontology`.
/// [`Self::binding`] is the ordinary homomorphism that discharged the residual triples and
/// [`Self::generalizations`] is one generalisation per axiom; every conclusion triple is
/// discharged by exactly one of the two.
#[derive(Debug, Clone)]
pub struct FreezeWarrant {
    /// The regime the closures were computed under.
    regime: Regime,
    /// What each existential of the RESIDUAL conclusion triples was bound to.
    binding: Binding,
    /// The premise's own closure, which the residual triples and the typings map into.
    closure: Closure,
    /// One generalisation per recognized axiom, in the conclusion's own triple order.
    generalizations: Vec<Generalization>,
}

impl FreezeWarrant {
    /// The regime whose chase established the implications.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The mapping that discharged the conclusion's residual (non-axiom) triples.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        &self.binding
    }

    /// One generalisation per schema axiom of the conclusion, in reading order.
    #[must_use]
    pub fn generalizations(&self) -> &[Generalization] {
        &self.generalizations
    }

    /// How many distinct triples the PREMISE closure this warrant is against holds.
    ///
    /// Each frozen instance's own closure is [`FrozenInstance::closure_size`]; they are
    /// different closures and are counted separately.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        self.closure.len()
    }

    /// The premise closure this warrant is against.
    pub(crate) const fn closure(&self) -> &Closure {
        &self.closure
    }

    /// This warrant with the fold's residual `binding` attached.
    pub(crate) fn with_binding(mut self, binding: Binding) -> Self {
        self.binding = binding;
        self
    }
}

// ── Reading a conclusion ───────────────────────────────────────────────────────────────

/// One recognized axiom triple, with the shape that recognized it.
struct Axiom {
    /// Its index in the conclusion's own frozen triple order — the name every lane of the
    /// fold refers to a conclusion triple by.
    index: usize,
    /// The conclusion triple.
    triple: Triple,
    /// The shape it matched.
    shape: &'static Shape,
}

/// The conclusion split into what this mechanism establishes and what it recognized and
/// declined.
struct Reading {
    /// The recognized schema axioms, in the conclusion's own triple order.
    axioms: Vec<Axiom>,
    /// The shapes this lane RECOGNIZED and declined to read, rendered. Never a residual: a
    /// triple this lane read the predicate of and refused the terms of is one it has admitted
    /// it cannot decide, and an admission must not become a refutation.
    declined: Vec<String>,
}

/// Whether `term` is the IRI `iri`.
fn is(term: &TermValue, iri: &str) -> bool {
    matches!(term, TermValue::Iri(value) if value == iri)
}

/// What this mechanism made of one conclusion triple.
enum ReadAs {
    /// Not a shape this lane reads at all — the triple is residual and keeps its ordinary
    /// obligation.
    No,
    /// A shape this lane READS, stated over terms it cannot read. An admission of incapacity,
    /// never a residual: a blank node in a named position is an EXISTENTIAL over classes or
    /// properties — "is there some transitive property?" — and a list-valued predicate in the
    /// axiom's own term position would make the seeded collection walk a walk of a different
    /// graph.
    Declined(String),
    /// A shape this lane reads, over terms it reads.
    Yes(&'static Shape),
}

/// The shape `triple` states, if this mechanism reads one.
fn recognize(triple: &Triple) -> ReadAs {
    let [subject, predicate, object] = triple;
    let Some(shape) = SHAPES.iter().find(|shape| {
        is(predicate, shape.predicate)
            && shape.typed_as.is_none_or(|class| is(object, class))
            && (shape.typed_as.is_some() == shape.object_is.is_none())
    }) else {
        return ReadAs::No;
    };
    let declined = |why: &str| {
        ReadAs::Declined(format!(
            "{} {} {}: {why}",
            show(&triple[0]),
            show(&triple[1]),
            show(&triple[2])
        ))
    };
    if !matches!(subject, TermValue::Iri(_)) {
        return declined(
            "an existential over the classes or properties an axiom is about is a different \
             question, and not one generalisation on constants answers",
        );
    }
    if shape.object_is.is_some() && !matches!(object, TermValue::Iri(_)) {
        return declined(
            "an existential in the axiom's second named position is a different question, and \
             not one generalisation on constants answers",
        );
    }
    // THE PRE-PASS WHITELIST. A named term of the axiom may land in a body atom's PREDICATE
    // position, and freezing `_:a rdf:first _:b` into a premise whose collection walk was
    // computed once would make that walk a walk of a different graph. Refused rather than
    // risked; see the module docs.
    let disturbs = |term: &TermValue| {
        matches!(term, TermValue::Iri(iri)
            if iri == RDF_FIRST || iri == RDF_REST || LIST_VALUED.contains(&iri.as_str()))
    };
    let predicate_slots = shape
        .implications
        .iter()
        .flat_map(|implication| implication.body.iter())
        .map(|atom| atom[1]);
    for slot in predicate_slots {
        let term = match slot {
            Slot::Subject => subject,
            Slot::Object => object,
            Slot::Var(_) | Slot::Iri(_) => continue,
        };
        if disturbs(term) {
            return declined(
                "freezing a body atom over a list-valued predicate would make the premise's \
                 own collection walk a walk of a different graph",
            );
        }
    }
    ReadAs::Yes(shape)
}

/// Split `conclusion`'s still-outstanding triples into the schema axioms this mechanism
/// establishes and the shapes it recognized and declined.
///
/// `pending` is what no earlier lane discharged: `p rdfs:range D` is a shape of this table AND
/// of [`datarange`](super::datarange)'s, and a triple already decided is not this lane's to
/// decide a second time.
fn read(triples: &[Triple], pending: &BTreeSet<usize>) -> Reading {
    let mut axioms = Vec::new();
    let mut declined = Vec::new();
    for (index, triple) in triples.iter().enumerate() {
        if !pending.contains(&index) {
            continue;
        }
        match recognize(triple) {
            ReadAs::Yes(shape) => axioms.push(Axiom {
                index,
                triple: triple.clone(),
                shape,
            }),
            ReadAs::Declined(why) => declined.push(why),
            ReadAs::No => {}
        }
    }
    declined.sort_unstable();
    declined.dedup();
    Reading { axioms, declined }
}

/// Instantiate `atom` against an axiom triple and a set of frozen constants.
fn instantiate(atom: &[Slot; 3], axiom: &Triple, constants: &[TermValue]) -> Triple {
    let resolve = |slot: &Slot| match slot {
        Slot::Subject => axiom[0].clone(),
        Slot::Object => axiom[2].clone(),
        Slot::Var(index) => constants[*index].clone(),
        Slot::Iri(iri) => TermValue::iri(*iri),
    };
    [resolve(&atom[0]), resolve(&atom[1]), resolve(&atom[2])]
}

/// How many distinct variables `implication` quantifies over.
fn arity(implication: &Implication) -> usize {
    implication
        .body
        .iter()
        .chain(std::iter::once(&implication.head))
        .flat_map(|atom| atom.iter())
        .filter_map(|slot| match slot {
            Slot::Var(index) => Some(index + 1),
            Slot::Subject | Slot::Object | Slot::Iri(_) => None,
        })
        .max()
        .unwrap_or(0)
}

// ── The mechanism ──────────────────────────────────────────────────────────────────────

/// What this lane READS of a question, with nothing frozen and nothing chased.
///
/// The same [`read`] the decision below opens with, run for its reading alone: an axiom this
/// lane recognizes is a SCHEMA statement, which Theorem PR1's conclusion hypothesis excludes
/// and the rule table therefore claims no completeness for, and a declined shape is one whose
/// predicate this lane reads over terms it cannot. Either way a service that does not run this
/// lane has left something untested.
pub(crate) fn recognizes(q: &Question<'_>) -> Recognized {
    if !matches!(q.regime, Regime::OwlRl) {
        return Recognized::default();
    }
    let reading = read(q.triples, q.pending);
    Recognized {
        read: reading.axioms.iter().map(|axiom| axiom.index).collect(),
        declined: reading.declined,
    }
}

/// Try to establish `conclusion` from `premise` by freezing and chasing.
///
/// `closure` is the premise's own closure, already computed and indexed by
/// [`prepare`](super::prepare) — which is also where the premise's CONSISTENCY was
/// established, without which a vacuous establishment below would prove nothing.
///
/// # Errors
///
/// Whatever the re-chase refuses with: [`EntailError::Evaluate`] for an evaluation ceiling,
/// [`EntailError::MalformedList`] for a premise whose OWL collections are not well formed,
/// and [`EntailError::MatchBudget`] from the residual match.
pub(crate) fn attempt(q: &Question<'_>) -> Result<Attempt, EntailError> {
    let Question {
        premise,
        conclusion,
        regime,
        closure,
        triples,
        pending,
    } = *q;
    // WHITELIST, not blacklist: the four other regimes fall out. `Simple`, `RDF` and `RDFS`
    // state no rule that could derive a frozen head from an OWL axiom, and `D` states no
    // completeness theorem this crate would read a derivation of one against.
    if !matches!(regime, Regime::OwlRl) {
        return Ok(Attempt::NotApplicable);
    }
    let reading = read(triples, pending);
    if reading.axioms.is_empty() {
        // Nothing to establish. A recognized-and-declined shape beside it is an ADMISSION of
        // incapacity and travels as one; nothing at all is simply not this lane's question.
        return Ok(if reading.declined.is_empty() {
            Attempt::NotApplicable
        } else {
            Attempt::Disqualified(UndecidedReason::ConstructNotRead {
                lane: EntailmentMechanism::Freeze,
                constructs: reading.declined,
            })
        });
    }

    // ESTABLISHMENT IS ALL-OR-NOTHING, so the budget is checked against the whole bill
    // before any of it is spent: a run that established half the axioms and then stopped
    // would have proved nothing while looking like it had.
    let needed: u64 = reading
        .axioms
        .iter()
        .map(|axiom| axiom.shape.implications.len() as u64)
        .sum();
    if needed > FREEZE_BUDGET {
        return Ok(Attempt::Undecided(UndecidedReason::FreezeBudget(needed)));
    }

    // The membership half first: it is a lookup, and an axiom whose typing nothing
    // establishes is not worth a chase.
    let mut typings_per_axiom = Vec::with_capacity(reading.axioms.len());
    for axiom in &reading.axioms {
        let mut typings = Vec::new();
        let memberships = [
            Some((axiom.triple[0].clone(), axiom.shape.subject_is)),
            axiom
                .shape
                .object_is
                .map(|membership| (axiom.triple[2].clone(), membership)),
        ];
        for (term, membership) in memberships.into_iter().flatten() {
            let found = membership.typings().iter().find_map(|class| {
                let triple = [
                    term.clone(),
                    TermValue::iri(RDF_TYPE),
                    TermValue::iri(*class),
                ];
                closure.contains(&triple).then_some(triple)
            });
            let Some(triple) = found else {
                return Ok(Attempt::NotEstablished);
            };
            typings.push(triple);
        }
        typings_per_axiom.push(typings);
    }

    let held = default_graph_triples(premise);
    let mut fresh = FreshBlanks::avoiding(&[premise, conclusion]);
    let mut refuter = Refuter::new(regime);
    let mut seeded = refuter.seed(premise)?;
    let mut generalizations = Vec::with_capacity(reading.axioms.len());
    for (axiom, typings) in reading.axioms.iter().zip(typings_per_axiom) {
        let mut instances = Vec::with_capacity(axiom.shape.implications.len());
        for implication in axiom.shape.implications {
            let constants: Vec<TermValue> = (0..arity(implication)).map(|_| fresh.mint()).collect();
            let body: Vec<Triple> = implication
                .body
                .iter()
                .map(|atom| instantiate(atom, &axiom.triple, &constants))
                .collect();
            let head = instantiate(&implication.head, &axiom.triple, &constants);
            let closed = refuter.close(&mut seeded, &body)?;
            let mut triples = held.clone();
            triples.extend(body.iter().cloned());
            triples.extend(closed.derived);
            let frozen = Closure::of(triples);
            let outcome = match closed.clash {
                // No model of the premise satisfies the frozen body, so the implication
                // holds with an empty antecedent. See the module docs for why that is an
                // establishment and not a failure.
                Some(witness) => FrozenOutcome::Vacuous(witness),
                None if frozen.contains(&head) => FrozenOutcome::Derived,
                None => return Ok(Attempt::NotEstablished),
            };
            instances.push(FrozenInstance {
                constants,
                body,
                head,
                outcome,
                closure: frozen,
            });
        }
        generalizations.push(Generalization {
            axiom: axiom.triple.clone(),
            typings,
            instances,
        });
    }

    Ok(Attempt::Entailed(Box::new(Established {
        warrant: EntailmentWarrant::Freeze(FreezeWarrant {
            regime,
            // The residual is the FOLD's, not this lane's; `entails` fills it in at the end.
            binding: Binding::new(),
            closure: closure.clone(),
            generalizations,
        }),
        discharged: reading.axioms.iter().map(|axiom| axiom.index).collect(),
        minted: Vec::new(),
        // One reading yields both: this conclusion states a shape this lane establishes AND
        // one it recognized and refused the terms of. The refusal is not cancelled by the
        // establishment beside it, so it travels with the evidence.
        declined: reading.declined,
    })))
}

/// Re-decide a freeze warrant against the caller's own premise and conclusion.
///
/// Called by [`verify`](super::verify), which owns the doc comment a caller reads. It runs no
/// reasoner: the conclusion is READ again on the spot, so a warrant cannot be replayed
/// against a different question, and each frozen instance is re-checked by lookups against
/// the closure it carries.
pub(crate) fn verify_freeze(
    w: &FreezeWarrant,
    premise: &RdfDataset,
    conclusion: &RdfDataset,
    triples: &[Triple],
    pending: &BTreeSet<usize>,
) -> Option<Replay> {
    let reading = read(triples, pending);
    // The axioms this warrant claims must be EXACTLY the axioms the conclusion still states,
    // in the same order: a warrant for a subset would leave part of the conclusion
    // unaccounted for, and one for a superset would be evidence about a different question.
    if reading.axioms.len() != w.generalizations.len() {
        return None;
    }
    let held: Vec<Triple> = default_graph_triples(premise);
    // The non-occurrence hypothesis is decided against the caller's OWN documents.
    let mut forbidden: BTreeSet<String> = labels_of(premise);
    forbidden.extend(labels_of(conclusion));

    for (axiom, generalization) in reading.axioms.iter().zip(&w.generalizations) {
        if axiom.triple != generalization.axiom {
            return None;
        }
        // The membership half, re-looked-up in the premise's closure.
        if !generalization
            .typings
            .iter()
            .all(|triple| w.closure.contains(triple))
        {
            return None;
        }
        if generalization.instances.len() != axiom.shape.implications.len() {
            return None;
        }
        for (implication, instance) in axiom
            .shape
            .implications
            .iter()
            .zip(&generalization.instances)
        {
            // The body and head must be THIS shape's implication, instantiated over the
            // constants the warrant cites — otherwise the chase decided some other question.
            if instance.constants.len() != arity(implication) {
                return None;
            }
            let body: Vec<Triple> = implication
                .body
                .iter()
                .map(|atom| instantiate(atom, &generalization.axiom, &instance.constants))
                .collect();
            let head = instantiate(
                &implication.head,
                &generalization.axiom,
                &instance.constants,
            );
            if instance.body != body || instance.head != head {
                return None;
            }
            if !instance.check(&held, &forbidden) {
                return None;
            }
        }
    }

    Some(Replay {
        discharged: reading.axioms.iter().map(|axiom| axiom.index).collect(),
        minted: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};

    use super::{
        Axiom, FREEZE_BUDGET, FrozenOutcome, ReadAs, SHAPES, Slot, arity, read, recognize,
    };
    use crate::entails::fresh::mentions_any;
    use crate::entails::graph::default_graph_triples;
    use crate::entails::{EntailmentOutcome, EntailmentWarrant, ImportMap, entails, verify};
    use crate::lists::LIST_VALUED;
    use crate::vocab::{
        OWL_CLASS, OWL_HASVALUE, OWL_NOTHING, OWL_OBJECTPROPERTY, OWL_ONPROPERTY, OWL_ONTOLOGY,
        OWL_PROPERTYCHAINAXIOM, OWL_RESTRICTION, OWL_SYMMETRICPROPERTY, OWL_TRANSITIVEPROPERTY,
        RDF_FIRST, RDF_NIL, RDF_REST, RDF_TYPE, RDFS_DOMAIN, RDFS_SUBCLASSOF,
    };
    use crate::{Materialization, Regime, RuleId, extensions, implemented, materialize, rules};

    const P: &str = "http://example.org/p";
    const Q: &str = "http://example.org/q";
    const A: &str = "http://example.org/A";
    const D: &str = "http://example.org/D";
    const V: &str = "http://example.org/v";
    const ONT: &str = "http://example.org/ontology";

    /// A default-graph dataset; a leading `_` names a blank node, anything else an IRI.
    fn graph(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let term = |b: &mut RdfDatasetBuilder, value: &str| match value.strip_prefix('_') {
                Some(label) => b.intern_blank(label, BlankScope::DEFAULT),
                None => b.intern_iri(value),
            };
            let s = term(&mut b, s);
            let p = term(&mut b, p);
            let o = term(&mut b, o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    /// W3C `chain2trans1`'s premise: `p ∘ p ⊑ p`, plus the ontology header.
    fn chain_premise() -> Arc<RdfDataset> {
        graph(&[
            (ONT, RDF_TYPE, OWL_ONTOLOGY),
            (P, RDF_TYPE, OWL_OBJECTPROPERTY),
            (P, OWL_PROPERTYCHAINAXIOM, "_l1"),
            ("_l1", RDF_FIRST, P),
            ("_l1", RDF_REST, "_l2"),
            ("_l2", RDF_FIRST, P),
            ("_l2", RDF_REST, RDF_NIL),
        ])
    }

    /// `A` is the `hasValue` restriction `∃p.{v}` and `p`'s domain is `D`, so `A ⊑ D` holds
    /// through an individual and no `scm-*` rule concludes it. `type_the_superclass` decides
    /// whether the premise also states `D ∈ IC`, which is the axiom's other conjunct.
    fn has_value_premise(type_the_superclass: bool) -> Arc<RdfDataset> {
        let mut triples = vec![
            (A, RDF_TYPE, OWL_CLASS),
            (A, RDF_TYPE, OWL_RESTRICTION),
            (A, OWL_ONPROPERTY, P),
            (A, OWL_HASVALUE, V),
            (P, RDF_TYPE, OWL_OBJECTPROPERTY),
            (P, RDFS_DOMAIN, D),
        ];
        if type_the_superclass {
            triples.push((D, RDF_TYPE, OWL_CLASS));
        }
        graph(&triples)
    }

    /// W3C `chain2trans1`'s conclusion.
    fn transitive_conclusion() -> Arc<RdfDataset> {
        graph(&[
            (ONT, RDF_TYPE, OWL_ONTOLOGY),
            (P, RDF_TYPE, OWL_TRANSITIVEPROPERTY),
        ])
    }

    fn decide(premise: &RdfDataset, conclusion: &RdfDataset) -> EntailmentOutcome {
        entails(premise, conclusion, Regime::OwlRl, &ImportMap::new())
            .expect("a consistent premise")
            .into_parts()
            .0
    }

    /// Every triple of `conclusion` still outstanding — the pending set the fold starts with.
    fn all_of(conclusion: &RdfDataset) -> BTreeSet<usize> {
        (0..default_graph_triples(conclusion).len()).collect()
    }

    // ── The mechanism reaches what the rule table cannot ───────────────────────────────

    /// A PROPERTY CHAIN `p ∘ p ⊑ p` ENTAILS TRANSITIVITY, and `prp-spo2` — one of the
    /// normative seventy-eight — is what derives the frozen head.
    #[test]
    fn a_property_chain_entails_transitivity_and_the_warrant_verifies() {
        let premise = chain_premise();
        let conclusion = transitive_conclusion();
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("p ∘ p ⊑ p is transitivity");
        };
        let EntailmentWarrant::Freeze(frozen) = &warrant else {
            panic!("no rule of Tables 4-9 has an owl:TransitiveProperty head");
        };
        assert_eq!(frozen.regime(), Regime::OwlRl);
        assert_eq!(frozen.generalizations().len(), 1);
        let generalization = &frozen.generalizations()[0];
        assert_eq!(generalization.typings().len(), 1, "p ∈ IOOP");
        assert_eq!(generalization.instances().len(), 1);
        assert!(matches!(
            generalization.instances()[0].outcome(),
            FrozenOutcome::Derived
        ));
        assert_eq!(generalization.instances()[0].body().len(), 2);
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// THE FROZEN CONSTANTS ARE ABSENT FROM BOTH DOCUMENTS. That non-occurrence is the
    /// hypothesis of the theorem on constants, so it is measured rather than assumed.
    #[test]
    fn the_frozen_constants_occur_in_neither_document() {
        let premise = chain_premise();
        let conclusion = transitive_conclusion();
        let EntailmentOutcome::Entailed(EntailmentWarrant::Freeze(frozen)) =
            decide(&premise, &conclusion)
        else {
            panic!("entailed by freezing");
        };
        let mut labels: BTreeSet<String> = BTreeSet::new();
        for instance in frozen.generalizations()[0].instances() {
            for constant in instance.constants() {
                let TermValue::Blank { label, .. } = constant else {
                    panic!("a frozen constant is a blank node");
                };
                assert!(labels.insert(label.clone()), "constants are distinct");
            }
        }
        for document in [&premise, &conclusion] {
            for triple in default_graph_triples(document) {
                for term in &triple {
                    assert!(
                        !mentions_any(term, &labels),
                        "{term:?} is a frozen constant"
                    );
                }
            }
        }
    }

    /// AN INCONSISTENT FROZEN INSTANCE IS AN ESTABLISHMENT, NOT AN ERROR.
    ///
    /// `p rdfs:domain owl:Nothing` makes `p` the empty property, so every characteristic of
    /// `p` holds vacuously. Falsifiable against the failure mode it prevents: without this
    /// arm the frozen chase's `EntailError::Inconsistent` would escape and report the
    /// CALLER's premise — which has a model — as having none.
    #[test]
    fn an_empty_property_is_symmetric_vacuously() {
        let premise = graph(&[
            (P, RDF_TYPE, OWL_OBJECTPROPERTY),
            (P, RDFS_DOMAIN, OWL_NOTHING),
        ]);
        let conclusion = graph(&[(P, RDF_TYPE, OWL_SYMMETRICPROPERTY)]);
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("a property with an empty domain is symmetric vacuously");
        };
        let EntailmentWarrant::Freeze(frozen) = &warrant else {
            panic!("reached by freezing");
        };
        let FrozenOutcome::Vacuous(witness) = frozen.generalizations()[0].instances()[0].outcome()
        else {
            panic!("the frozen body has no model");
        };
        assert_eq!(witness.rule(), RuleId::ClsNothing2);
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// An INCLUSION axiom is decided the same way, and this one is NOT reachable by the
    /// `scm-*` rules: `A` is a `hasValue` restriction on `p` and `p`'s domain is `D`, so
    /// `A ⊑ D` follows only through an INDIVIDUAL — `cls-hv1` then `prp-dom` — which is
    /// exactly what freezing one supplies.
    #[test]
    fn a_subclass_inclusion_is_established_by_freezing_an_instance() {
        let premise = has_value_premise(true);
        let conclusion = graph(&[(A, RDFS_SUBCLASSOF, D)]);
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("cls-hv1 then prp-dom types the frozen individual a D");
        };
        assert!(
            matches!(&warrant, EntailmentWarrant::Freeze(_)),
            "no scm-* rule concludes this inclusion"
        );
        assert!(verify(&warrant, &premise, &conclusion));
    }

    // ── ADVERSARIAL: the mechanism must be able to say NO ──────────────────────────────

    /// A PROPERTY NOTHING MAKES TRANSITIVE IS NOT ESTABLISHED TRANSITIVE — and the answer is
    /// UNDECIDED rather than a refutation, because this lane has no completeness theorem.
    ///
    /// Falsifiable against the failure mode a freeze-and-chase invites: a mechanism that
    /// answered `Entailed` whenever it recognized a shape would pass the corpus case above,
    /// because every ledgered case was a positive. And falsifiable against the OTHER failure
    /// mode, which the module docs have always disclaimed and the code used to contradict:
    /// "the converse is NOT claimed, and this module never answers not entailed". A frozen
    /// chase that derives nothing is silence, and Theorem PR1 says nothing about a conclusion
    /// stating a property characteristic, so there is no theorem to read a refutation out of.
    #[test]
    fn an_unconstrained_property_is_undecided_rather_than_refuted() {
        let premise = graph(&[(P, RDF_TYPE, OWL_OBJECTPROPERTY)]);
        let conclusion = graph(&[(P, RDF_TYPE, OWL_TRANSITIVEPROPERTY)]);
        let EntailmentOutcome::Undecided(crate::UndecidedReason::ConclusionOutsideRl(triples)) =
            decide(&premise, &conclusion)
        else {
            panic!("nothing in the premise composes p with itself, and nothing refutes it either");
        };
        assert_eq!(triples.len(), 1, "{triples:?}");
        assert!(triples[0].contains("TransitiveProperty"), "{triples:?}");
    }

    /// THE MEMBERSHIP HALF IS OWED. `rdfs:subClassOf` is `c,d ∈ IC` AND the inclusion, so an
    /// inclusion the frozen chase reaches over a term nothing types as a class does not
    /// establish the axiom. The same premise WITH the typing does (above), so this is a fact
    /// about the conjunct and not about the inclusion.
    #[test]
    fn an_untyped_term_fails_the_membership_half() {
        let premise = has_value_premise(false);
        let conclusion = graph(&[(A, RDFS_SUBCLASSOF, D)]);
        assert!(
            !matches!(
                decide(&premise, &conclusion),
                EntailmentOutcome::Entailed(_)
            ),
            "the axiom's typing conjunct is not established"
        );
    }

    /// A LIST-VALUED PREDICATE IS REFUSED. Freezing `_:a rdf:first _:b` would make the
    /// seed's collection walk a walk of a different graph, so the shape is not read at all.
    #[test]
    fn a_list_valued_predicate_is_refused() {
        for predicate in [RDF_FIRST, RDF_REST]
            .into_iter()
            .chain(LIST_VALUED.iter().copied())
        {
            let triple = [
                TermValue::iri(predicate),
                TermValue::iri(RDF_TYPE),
                TermValue::iri(OWL_TRANSITIVEPROPERTY),
            ];
            assert!(
                matches!(recognize(&triple), ReadAs::Declined(_)),
                "{predicate} would disturb the collection pre-pass, and declining it is an \
                 ADMISSION rather than a residual nobody would notice went untested"
            );
        }
        // …while an ordinary property of the caller's vocabulary is read.
        let ordinary = [
            TermValue::iri(P),
            TermValue::iri(RDF_TYPE),
            TermValue::iri(OWL_TRANSITIVEPROPERTY),
        ];
        assert!(matches!(recognize(&ordinary), ReadAs::Yes(_)));
    }

    /// AN EXISTENTIAL OVER PROPERTIES IS RECOGNIZED AND DECLINED, never quietly residual.
    ///
    /// "Is there some transitive property?" is a question this mechanism does not answer, and
    /// saying so is an ADMISSION: a residual that fell through to a failed match would have
    /// come out of the service as a proof of non-entailment.
    #[test]
    fn a_blank_named_position_is_declined_rather_than_dropped() {
        for triples in [
            vec![("_x", RDF_TYPE, OWL_TRANSITIVEPROPERTY)],
            vec![(A, RDFS_SUBCLASSOF, "_d")],
        ] {
            let conclusion = graph(&triples);
            let reading = read(&default_graph_triples(&conclusion), &all_of(&conclusion));
            assert!(reading.axioms.is_empty(), "{triples:?}");
            assert!(!reading.declined.is_empty(), "{triples:?}");
        }
    }

    /// …AND AN ESTABLISHMENT IN THE SAME READING DOES NOT CANCEL IT.
    ///
    /// `chain2trans1`'s conclusion with one existential axiom added: the reading holds an axiom
    /// this lane freezes AND a shape it refused the terms of, and the two are independent
    /// triples of one conclusion. The refusal is the answer a caller needs — nothing tested
    /// `_:x rdf:type owl:TransitiveProperty` in either direction — and it was reachable only
    /// while the lane established NOTHING, so establishing the axiom beside it silently
    /// replaced an admission with a failed match.
    #[test]
    fn a_declined_shape_survives_an_axiom_established_in_the_same_reading() {
        let premise = chain_premise();
        // Nothing in the premise's closure is typed `owl:TransitiveProperty` — deriving that
        // typing is what the frozen chase does, and it mints nothing — so the existential has
        // nothing to bind to and the residual really does miss.
        let conclusion = graph(&[
            (ONT, RDF_TYPE, OWL_ONTOLOGY),
            (P, RDF_TYPE, OWL_TRANSITIVEPROPERTY),
            ("_x", RDF_TYPE, OWL_TRANSITIVEPROPERTY),
        ]);
        let reading = read(&default_graph_triples(&conclusion), &all_of(&conclusion));
        assert_eq!(reading.axioms.len(), 1, "one axiom is established");
        assert_eq!(reading.declined.len(), 1, "…beside one refusal");
        let EntailmentOutcome::Undecided(crate::UndecidedReason::ConstructNotRead {
            lane,
            constructs,
        }) = decide(&premise, &conclusion)
        else {
            panic!("a recognized-and-declined shape is an ADMISSION, never a shrug");
        };
        assert_eq!(lane, crate::EntailmentMechanism::Freeze);
        assert_eq!(constructs, reading.declined, "{constructs:?}");
    }

    /// A conclusion this mechanism reads nothing in is NOT its business, and it does not
    /// pretend otherwise by declining something it never recognized.
    #[test]
    fn an_ordinary_conclusion_is_not_applicable() {
        let conclusion = graph(&[(A, RDF_TYPE, OWL_CLASS)]);
        let reading = read(&default_graph_triples(&conclusion), &all_of(&conclusion));
        assert!(reading.axioms.is_empty());
        assert!(reading.declined.is_empty());
    }

    // ── The table, and the inventory ───────────────────────────────────────────────────

    /// The table is well formed: a typing shape names one term, a two-term shape names two,
    /// and every fixed predicate of every body is safe for the seed's collection pre-pass.
    #[test]
    fn the_shape_table_is_well_formed() {
        for shape in SHAPES {
            assert_eq!(
                shape.typed_as.is_some(),
                shape.object_is.is_none(),
                "{:?}: a typing shape names one term and a relational shape names two",
                shape.predicate
            );
            assert!(!shape.implications.is_empty());
            for implication in shape.implications {
                assert!(
                    !implication.body.is_empty(),
                    "an empty body freezes nothing"
                );
                assert!(arity(implication) > 0);
                for atom in implication.body {
                    if let Slot::Iri(iri) = atom[1] {
                        assert_ne!(iri, RDF_FIRST);
                        assert_ne!(iri, RDF_REST);
                        assert!(!LIST_VALUED.contains(&iri));
                    }
                }
            }
        }
    }

    /// The lane is gated to `OWL-RL` by WHITELIST: the four other regimes fall out.
    #[test]
    fn only_the_owl_rl_lane_freezes() {
        let premise = chain_premise();
        let conclusion = transitive_conclusion();
        for regime in [Regime::Simple, Regime::Rdf, Regime::Rdfs, Regime::D] {
            assert!(
                !matches!(
                    entails(&premise, &conclusion, regime, &ImportMap::new())
                        .expect("consistent")
                        .outcome(),
                    EntailmentOutcome::Entailed(_)
                ),
                "{regime:?} states no rule a frozen head could arrive through"
            );
        }
    }

    /// STRICT MATERIALIZATION GAINS NOTHING. The closure of the premise still does not hold
    /// the conclusion; only the conclusion-directed service reaches it.
    #[test]
    fn materialization_still_does_not_produce_these_conclusions() {
        let (closure, _) =
            materialize(&chain_premise(), Materialization::OwlRl).expect("consistent");
        assert!(
            !default_graph_triples(&closure).contains(&[
                TermValue::iri(P),
                TermValue::iri(RDF_TYPE),
                TermValue::iri(OWL_TRANSITIVEPROPERTY),
            ]),
            "no rule of Tables 4-9 has an owl:TransitiveProperty head"
        );
    }

    /// THE NORMATIVE INVENTORY IS UNTOUCHED. Freeze-and-chase is a proof strategy over the
    /// declared calculus, not a widening of it.
    #[test]
    fn the_freeze_lane_adds_no_rule() {
        assert_eq!(rules(Regime::OwlRl).len(), 78);
        assert_eq!(implemented(Regime::OwlRl), rules(Regime::OwlRl));
        assert_eq!(extensions(Regime::OwlRl), [RuleId::ExtEqDiffSym]);
    }

    /// A conclusion needing more chases than the budget allows is UNDECIDED, never refuted.
    #[test]
    fn a_conclusion_past_the_budget_is_undecided() {
        let mut triples: Vec<(String, &str, &str)> = Vec::new();
        // Every axiom here abbreviates ONE implication, so the bill is the axiom count.
        for i in 0..=FREEZE_BUDGET {
            triples.push((
                format!("http://example.org/p{i}"),
                RDF_TYPE,
                OWL_TRANSITIVEPROPERTY,
            ));
        }
        let borrowed: Vec<(&str, &str, &str)> = triples
            .iter()
            .map(|(s, p, o)| (s.as_str(), *p, *o))
            .collect();
        let conclusion = graph(&borrowed);
        assert!(matches!(
            decide(&chain_premise(), &conclusion),
            EntailmentOutcome::Undecided(crate::UndecidedReason::FreezeBudget(_))
        ));
    }

    // ── `verify` is a CHECK, not a claim ───────────────────────────────────────────────

    /// A freeze warrant does not replay against another premise or another conclusion.
    #[test]
    fn a_freeze_warrant_does_not_replay() {
        let premise = chain_premise();
        let conclusion = transitive_conclusion();
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("entailed by freezing");
        };
        assert!(verify(&warrant, &premise, &conclusion));
        assert!(!verify(
            &warrant,
            &graph(&[(A, RDF_TYPE, OWL_CLASS)]),
            &conclusion
        ));
        assert!(!verify(
            &warrant,
            &premise,
            &graph(&[(Q, RDF_TYPE, OWL_TRANSITIVEPROPERTY)])
        ));
        // A conclusion this mechanism reads nothing in has no axiom list to agree with.
        assert!(!verify(
            &warrant,
            &premise,
            &graph(&[(A, RDF_TYPE, OWL_CLASS)])
        ));
    }

    /// The whole answer is a function of the inputs: two runs agree.
    #[test]
    fn the_freeze_lane_is_deterministic() {
        let run = || {
            let EntailmentOutcome::Entailed(EntailmentWarrant::Freeze(w)) =
                decide(&chain_premise(), &transitive_conclusion())
            else {
                panic!("entailed by freezing");
            };
            w.generalizations()
                .iter()
                .map(|g| {
                    (
                        g.to_string(),
                        g.instances()
                            .iter()
                            .map(|i| (i.constants().to_vec(), i.body().to_vec(), i.head().clone()))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    /// `Axiom` is read only through `read`; this keeps the field live for a reader who
    /// changes the recognizer without changing the warrant.
    #[test]
    fn a_read_axiom_carries_the_shape_that_recognized_it() {
        let conclusion = transitive_conclusion();
        let triples = default_graph_triples(&conclusion);
        let reading = read(&triples, &all_of(&conclusion));
        let Axiom {
            index,
            triple,
            shape,
        } = &reading.axioms[0];
        assert_eq!(
            triple, &triples[*index],
            "the index names the triple it read"
        );
        assert_eq!(triple[0], TermValue::iri(P));
        assert_eq!(shape.typed_as, Some(OWL_TRANSITIVEPROPERTY));
        assert_eq!(
            reading.axioms.len(),
            1,
            "the ontology header is nobody's axiom and stays outstanding"
        );
    }
}
