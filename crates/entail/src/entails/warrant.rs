// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The evidence for a `yes`, and the check that re-decides it.
//!
//! # A verdict a caller has to believe is not evidence
//!
//! This crate's discipline is that every claim comes with something the reader can
//! re-decide rather than re-read: a [`ChaseProof`](crate::ChaseProof) re-derives its head
//! from its premises, a [`Justification`](crate::Justification) re-decides both its
//! sufficiency and its minimality. An entailment verdict owes the same, and
//! [`EntailmentWarrant`] is what it owes.
//!
//! [`verify`] then re-decides it. It runs no reasoner and re-derives nothing —
//! deliberately, because that is a different claim with a different checker. The two
//! decompose:
//!
//! * **"the closure follows from the premise"** is the chase's claim, and
//!   [`explain_conclusion`](crate::explain_conclusion) answers it with a `ChaseProof` whose
//!   `check` re-derives each step against the clause program.
//! * **"the conclusion follows from the closure"** is this claim, and it is finite and
//!   purely combinatorial — a graph homomorphism, or a set of lookups against a refutation's
//!   own closure — checkable in one pass over the conclusion.
//!
//! Folding them into one checker would mean `verify` had to re-run the chase, which costs
//! what the original call cost and gives a caller no independent check at all.
//!
//! # ONE ARM PER MECHANISM, and each arrives with its producer
//!
//! [`EntailmentWarrant`] is an enum with exactly as many arms as this crate has mechanisms
//! for reaching a `yes`, and it has never had one more than that. There are seven, six of
//! which are one lane each:
//!
//! * [`Homomorphism`](EntailmentWarrant::Homomorphism) — the chase-and-graph-match procedure
//!   OWL 2 Profiles §4.3 states the RL entailment relation in terms of, minted by
//!   [`super::homomorphism`]. Its evidence is the blank-node MAPPING that made the
//!   conclusion true, together with the closure it maps into.
//! * [`Refutation`](EntailmentWarrant::Refutation) — assert the conclusion's negation,
//!   re-chase, and read the profile's own inconsistency calculus as the proof, minted by
//!   [`super::refutation`]. Its evidence is a [`Refutation`](super::Refutation) per negative
//!   fact — the clash, the minimal premise subset that produces it, and the closure its rule
//!   body instances lie in — beside the ordinary mapping for whatever part of the conclusion
//!   was not negative.
//! * [`Freeze`](EntailmentWarrant::Freeze) — instantiate a schema axiom's universally
//!   quantified body over constants the premise does not mention, re-chase, and read the
//!   derived head as the proof, minted by [`super::freeze`]. Its evidence is a
//!   [`Generalization`](super::Generalization) per axiom — the closure triples establishing
//!   the axiom's MEMBERSHIP half, and one frozen instance per implication carrying the
//!   constants, the body, the head and the closure they lie in.
//! * [`Comprehension`](EntailmentWarrant::Comprehension) — mint the anonymous class
//!   expressions the conclusion names, under the typing side conditions the RDF-Based
//!   comprehension conditions impose, minted by [`super::comprehension`]. Its evidence is the
//!   triples MINTED, the closure triples that LICENSE them, and the witness map that says
//!   which fresh node stood in for which of the conclusion's own.
//! * [`Reflexivity`](EntailmentWarrant::Reflexivity) — read the conclusion's own self-loops
//!   `x p x` off the premise's `owl:ReflexiveProperty` typings, minted by
//!   [`super::reflexivity`]. Its evidence is the self-loops MINTED and the typing that
//!   licenses each.
//! * [`DataRange`](EntailmentWarrant::DataRange) — decide an `rdfs:range` axiom by
//!   intersecting the premise's declared ranges and asking whether the intersection is
//!   contained in the conclusion's, minted by [`super::datarange`]. Its evidence is the
//!   declarations the containment was decided over, per axiom.
//!
//! …and the seventh is not a lane at all:
//!
//! * [`Composite`](EntailmentWarrant::Composite) — TWO OR MORE of the six, folded, minted by
//!   [`super::entails`]'s own fold and by nothing else. A conclusion graph is a conjunction
//!   and entailment is monotone over one, so a conclusion stating a negative fact beside a
//!   schema axiom is entailed when each half is; the fold threads the residual through every
//!   lane in the fixed cost order [`super::entails`] tries them in, and matches whatever
//!   survives. Its evidence is the
//!   ORDERED list of contributing warrants together with the binding of the surviving
//!   residual — see [`CompositeWarrant`].
//!
//! An arm nothing constructs would be a state [`verify`] could not check and no caller could
//! ever be handed, which is the same unrepresentable-contradiction discipline that keeps
//! [`DlCertificate`](crate::DlCertificate) from storing a completeness beside the boundary
//! list that contradicts it. An eighth mechanism brings its own arm and its own producer,
//! together, or it does not arrive.
//!
//! # The RESIDUAL homomorphism belongs to the ANSWER, not to a lane
//!
//! Every warrant carries a [`binding`](EntailmentWarrant::binding), and it is always the
//! mapping of the triples NO lane discharged — the one final homomorphism the fold runs, once,
//! against `closure ∪ Σ(minted)`. A lane no longer matches a residual of its own, because its
//! residual is not the answer's: a triple refutation left behind may be one freeze discharges,
//! and a lane that scored it as a miss would be answering a question the fold had not finished
//! asking. On a composite the binding lives on the composite and every constituent's is empty,
//! which is the same "nothing stored that can be derived" discipline as everywhere else here:
//! one binding, one owner.
//!
//! # What `verify` actually checks, and why it needs the premise
//!
//! [`verify`] REPLAYS the fold's reading — which is pure syntax plus closure lookups, and runs
//! no reasoner. For each constituent in order it re-reads the conclusion, re-checks that
//! constituent's own evidence, and subtracts what that constituent discharged; then it maps the
//! surviving residual under the warrant's binding into `closure ∪ Σ(minted)`. So a composite is
//! re-decided by exactly the same accounting that produced it, and a warrant claiming a
//! discharge the conclusion does not license fails at the step that recomputes it.
//!
//! For the homomorphism arm, three things, all of them necessary and none of them a
//! reasoner:
//!
//! 1. Every triple of the conclusion, with the warrant's mapping applied, is a triple of the
//!    warrant's closure. A mapping that leaves a conclusion blank node unbound produces no
//!    triple to look for and is rejected rather than treated as satisfied.
//! 2. Every default-graph triple of the premise is a triple of the warrant's closure. This
//!    is what BINDS the warrant to the premise it was issued for: a closure is `premise +
//!    conclusions about it`, so a warrant minted for some other premise fails here instead
//!    of being replayed against this one.
//! 3. The conclusion is read from the caller's dataset on the spot, so a warrant cannot be
//!    replayed against a different conclusion either.
//!
//! It is `O(|conclusion| · log|closure|)` for the mapping plus `O(|premise| · log|closure|)`
//! for the binding — one lookup per triple, no search, because the search already happened
//! and its result is what the warrant carries.
//!
//! For the refutation arm it is the residual mapping above plus
//! [`Refutation::check`](super::Refutation::check) per negative fact — and the conclusion is
//! LOWERED again on the spot, so a warrant cannot be replayed against a conclusion whose
//! negative facts are different ones.
//!
//! For the freeze arm it is the residual mapping plus, per
//! generalisation: the axiom's typings are triples of the premise closure; the constants are
//! distinct blank nodes occurring in NEITHER the premise nor the conclusion — the hypothesis
//! of the theorem on constants, re-decided against the caller's own documents rather than
//! taken on the generator's word; the body and head are that shape's implication instantiated
//! over exactly those constants; and each frozen closure holds the premise, the body and
//! either the head or a `false`-concluding rule's satisfied body.
//!
//! For the comprehension arm the conclusion is READ again and the mint RECOMPUTED from the
//! warrant's witness map, then compared: a warrant whose minted list is not what this
//! conclusion licenses under those witnesses fails. Beside that, every witness is re-checked
//! to be a blank node naming nothing in either document (and distinct from the other
//! witnesses), every licence is re-looked-up in the premise closure, and the binding is
//! replayed against `closure ∪ minted`.
//!
//! The reflexivity arm is the same shape with no witnesses to check: the conclusion is read
//! again, the self-loops it licenses under the warrant's closure are recomputed and compared,
//! each licensing typing is re-looked-up, and the binding is replayed.
//!
//! The data-range arm is the residual mapping plus, per axiom: the cited
//! declarations really are the closure's declarations for that property, and the containment
//! is RE-DECIDED — which is arithmetic over value spaces rather than inference, so it is a
//! check and not a second run of anything.
//!
//! The composite arm is every constituent's own check above, run in the warrant's own order
//! against the pending set each of them saw, and then the residual mapping once. A constituent
//! that is itself a composite is rejected outright: the fold never nests one, so a nested arm
//! is a forgery rather than a shape this crate produces.
//!
//! # What `verify` does NOT check
//!
//! It does not re-derive any closure, and it therefore cannot detect a closure that never
//! followed from the premise in the first place. That is the chase's claim and the chase's
//! checker; saying so here rather than implying otherwise is the point of splitting them.

use std::collections::BTreeSet;

use purrdf_core::{RdfDataset, TermValue};

use crate::Regime;
use crate::entails::comprehension::{ComprehensionWarrant, verify_comprehension};
use crate::entails::datarange::{DataRangeWarrant, verify_datarange};
use crate::entails::freeze::{FreezeWarrant, verify_freeze};
use crate::entails::graph::{Triple, default_graph_triples};
use crate::entails::homomorphism::{Binding, Closure, substitute};
use crate::entails::pattern::{conclusion_patterns, patterns_at};
use crate::entails::reflexivity::{ReflexivityWarrant, verify_reflexivity};
use crate::entails::refutation::{RefutationWarrant, verify_refutation};

/// WHICH of this crate's seven mechanisms answered a conclusion-directed question.
///
/// The NAME of the answer's provenance, separated from the answer's evidence so it can be
/// carried where the evidence cannot: an [`EntailmentWarrant`] holds whole closures and only
/// exists for a `yes`, whereas this is a `Copy` tag that exists for every outcome and crosses
/// the C and WASM boundaries as [`Self::as_str`].
///
/// # It is a function of the OUTCOME, and it is never stored beside one
///
/// Every producer derives it: [`EntailmentWarrant::mechanism`] reads the warrant's own arm,
/// [`UndecidedReason::mechanism`](super::UndecidedReason::mechanism) reads which lane stopped
/// early, and a [`MissReason`](super::MissReason) is always [`Self::StrictTable`] because no
/// mechanism ever refutes. So a mechanism that disagrees with the outcome beside it is not a
/// state this crate detects and rejects — it is a state nothing constructs, which is the
/// same discipline that keeps a completeness out of [`ReasoningReport`](crate::ReasoningReport)
/// and out of [`DlCertificate`](crate::DlCertificate).
///
/// # [`Self::StrictTable`] is the answer, not the absence of one
///
/// Five of the seven arms exist because the rule table has NO head of the conclusion's shape;
/// the sixth is the table itself and the seventh is two or more of them at once. A question the table decides — matched into its closure, or
/// proven absent from it — is answered by [`Self::StrictTable`], and that is a positive claim:
/// the seventy-eight rules of OWL 2 Profiles §4.3 (or the regime's own table) were run once
/// over the premise and the conclusion was read off the result. It is the ONLY arm that can
/// accompany a [`NotEntailed`](super::EntailmentOutcome::NotEntailed), because refuting needs
/// a completeness claim and only the table has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntailmentMechanism {
    /// The regime's own rule table, run once over the premise, and the conclusion matched
    /// into (or proven absent from) the closure.
    StrictTable,
    /// The conclusion's negation asserted into the premise and the table re-run, with a
    /// `false`-concluding rule read as the proof.
    Refutation,
    /// A schema axiom's universally quantified body instantiated over fresh constants and
    /// the table re-run, with the derived head read as the proof.
    Freeze,
    /// The anonymous class expressions the conclusion names, minted under the RDF-Based
    /// comprehension conditions' typing side conditions.
    Comprehension,
    /// The conclusion's own self-loops read off the premise's `owl:ReflexiveProperty`
    /// typings.
    Reflexivity,
    /// An `rdfs:range` axiom decided by containment between XSD value spaces.
    DataRange,
    /// TWO OR MORE of the six above, folded over one conclusion — each discharging the triples
    /// it reads and handing the rest on, with the surviving residual matched once at the end.
    ///
    /// Spelled `composite` rather than by any constituent's name, because a conclusion reached
    /// by refutation AND freeze was not reached by refutation: naming either would tell a
    /// consumer that one mechanism sufficed, which is exactly the claim the arm exists to
    /// deny. [`EntailmentWarrant::Composite`] carries WHICH lanes contributed, in order.
    Composite,
}

impl EntailmentMechanism {
    /// Every mechanism, in the order [`super::entails`] reaches for them — the table first,
    /// then the cost order [`super::entails`] tries the other five in.
    pub const ALL: [Self; 7] = [
        Self::StrictTable,
        Self::Refutation,
        Self::Freeze,
        Self::Comprehension,
        Self::Reflexivity,
        Self::DataRange,
        Self::Composite,
    ];

    /// A short, stable name — the spelling every host renders and the C and WASM boundaries
    /// carry.
    ///
    /// Never an enum ORDINAL across a boundary: an ordinal is a number whose meaning lives in
    /// this file, so inserting a seventh arm would silently renumber every consumer's
    /// interpretation of an old one.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StrictTable => "strict-table",
            Self::Refutation => "refutation",
            Self::Freeze => "freeze",
            Self::Comprehension => "comprehension",
            Self::Reflexivity => "reflexivity",
            Self::DataRange => "data-range",
            Self::Composite => "composite",
        }
    }

    /// The SEMANTIC BOUNDARY of the rule table this mechanism crosses — why the table alone
    /// could not answer, in the table's own terms.
    ///
    /// A function of the mechanism exactly as [`Boundary::reason`](crate::Boundary::reason) is
    /// a function of its construct, and for the same reason: a mechanism and a hand-written
    /// explanation of it, stored separately, drift.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::StrictTable => {
                "no boundary was crossed: the conclusion was decided against the closure the \
                 regime's own rule table produces, in one run, by the graph homomorphism OWL 2 \
                 Profiles §4.3 states the entailment relation in terms of. This is the only \
                 mechanism that can accompany a NOT-ENTAILED, because refuting needs the \
                 completeness half of a theorem and only the table has one"
            }
            Self::Refutation => {
                "NO HEAD IN TABLES 4-9 IS A NEGATIVE FACT. Not one rule of the OWL 2 RL/RDF \
                 table concludes an owl:differentFrom or a membership in an owl:complementOf \
                 class, so a premise can entail one while a forward chase derives nothing to \
                 match against. Seventeen of the seventy-eight rules DO conclude false, which \
                 is to say the table carries its own inconsistency calculus — so the \
                 conclusion's negation is asserted into the premise and the SAME table is run \
                 again, over a premise whose consistency was established first, and the clash \
                 is the entailment"
            }
            Self::Freeze => {
                "NO HEAD IN TABLES 4-9 IS A SCHEMA AXIOM. Every head is an assertional triple \
                 over named terms or false, so a property characteristic such as \
                 p rdf:type owl:TransitiveProperty is a conclusion no forward chase produces. \
                 It ABBREVIATES a universally quantified Horn implication, and an implication \
                 is decided by generalisation on constants: its body is instantiated over \
                 constants occurring in neither document, the SAME table is run again, and the \
                 derived head is the proof"
            }
            Self::Comprehension => {
                "A COMPREHENSION CONDITION ASSERTS THE EXISTENCE OF A RESOURCE NOTHING NAMES. \
                 OWL 2's RDF-Based Semantics licenses the anonymous class expressions a \
                 conclusion describes outright, subject to a typing side condition on their \
                 operands — and a rule set that produced one for every licensed shape would \
                 produce infinitely many, which is why no table states it. The scaffolds the \
                 conclusion names are minted over blank nodes checked absent from both \
                 documents, and the side condition is established against the premise's own \
                 closure first"
            }
            Self::Reflexivity => {
                "owl:ReflexiveProperty IS OUTSIDE THE OWL 2 RL SYNTAX, so Profiles §4.3 states \
                 no rule for it at all — and a rule that did state it would range over every \
                 resource, an O(|terms|) closure widening in a lane every consumer runs by \
                 default, to answer a question only a conclusion ever asks. The conclusion's \
                 own self-loops x p x are instead read off the premise's reflexive typings, \
                 one lookup per conclusion triple, which is a positive establishment and \
                 therefore needs no completeness theorem and no profile membership"
            }
            Self::DataRange => {
                "A RULE TABLE HAS NO ARITHMETIC. Deciding that an rdfs:range axiom follows \
                 needs the XSD value spaces themselves — xsd:byte ⊑ xsd:short is not something \
                 any join over triples can discover, and an intersection of several declared \
                 ranges may be contained in a datatype the premise never mentions. The \
                 containment is decided by purrdf-xsd over those value spaces, three-valued, so \
                 that an undecidable facet reads as UNDECIDED rather than as a refutation"
            }
            Self::Composite => {
                "A CONCLUSION GRAPH IS A CONJUNCTION, AND ENTAILMENT IS MONOTONE OVER ONE. No \
                 single boundary was crossed: two or more of the mechanisms above were, one per \
                 half of a conclusion neither could read alone. Each lane discharged the triples \
                 it reads and handed the RESIDUAL on rather than scoring it as a miss, and the \
                 triples that survived every lane were matched once against the closure the \
                 lanes widened. Reporting any one constituent's name here would say that one \
                 mechanism sufficed, which is the claim this mechanism exists to deny"
            }
        }
    }
}

impl std::fmt::Display for EntailmentMechanism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The evidence that a premise entails a conclusion, tagged by the mechanism that produced
/// it.
///
/// See the [module docs](self) for why there are exactly six arms and what an arm owes.
#[derive(Debug, Clone)]
pub enum EntailmentWarrant {
    /// The conclusion MAPPED into the closure of the premise.
    Homomorphism(HomomorphismWarrant),
    /// The conclusion's negative facts were REFUTED and the rest mapped.
    Refutation(RefutationWarrant),
    /// The conclusion's schema axioms were FROZEN over fresh constants and chased, and the
    /// rest mapped.
    Freeze(FreezeWarrant),
    /// The conclusion's anonymous class expressions were COMPREHENDED under their typing side
    /// conditions, and the whole conclusion mapped into the extended closure.
    Comprehension(ComprehensionWarrant),
    /// The conclusion's self-loops were read off the premise's REFLEXIVE property typings,
    /// and the whole conclusion mapped into the extended closure.
    Reflexivity(ReflexivityWarrant),
    /// The conclusion's `rdfs:range` axioms were decided by datatype CONTAINMENT, and the
    /// rest mapped.
    DataRange(DataRangeWarrant),
    /// TWO OR MORE of the six above each discharged the part of the conclusion it reads, and
    /// what survived them all mapped.
    Composite(CompositeWarrant),
}

impl EntailmentWarrant {
    /// The regime whose closure this warrant is against — the identity of the claim, not
    /// part of its check.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        match self {
            Self::Homomorphism(w) => w.regime(),
            Self::Refutation(w) => w.regime(),
            Self::Freeze(w) => w.regime(),
            Self::Comprehension(w) => w.regime(),
            Self::Reflexivity(w) => w.regime(),
            Self::DataRange(w) => w.regime(),
            Self::Composite(w) => w.regime(),
        }
    }

    /// WHICH mechanism minted this warrant.
    ///
    /// Read off the arm rather than stored beside it, so a warrant cannot name a mechanism
    /// other than the one that built it. The homomorphism arm answers
    /// [`EntailmentMechanism::StrictTable`]: its evidence IS the regime's own rule table,
    /// run once, with the conclusion matched into the closure.
    #[must_use]
    pub const fn mechanism(&self) -> EntailmentMechanism {
        match self {
            Self::Homomorphism(_) => EntailmentMechanism::StrictTable,
            Self::Refutation(_) => EntailmentMechanism::Refutation,
            Self::Freeze(_) => EntailmentMechanism::Freeze,
            Self::Comprehension(_) => EntailmentMechanism::Comprehension,
            Self::Reflexivity(_) => EntailmentMechanism::Reflexivity,
            Self::DataRange(_) => EntailmentMechanism::DataRange,
            Self::Composite(_) => EntailmentMechanism::Composite,
        }
    }

    /// The mapping: what each existential of the conclusion's RESIDUAL was bound to.
    ///
    /// The residual is the triples NO mechanism discharged — a negative fact has no mapping,
    /// which is why it had to be refuted, and a schema axiom has none either, which is why it
    /// had to be frozen. For the comprehension and reflexivity arms nothing is discharged at
    /// all: those two only WIDEN the closure, so the residual is the whole conclusion and it
    /// maps into `closure ∪ minted`.
    ///
    /// On a [`Composite`](Self::Composite) this is the composite's own, and every constituent's
    /// is empty: one residual, one mapping, one owner.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        match self {
            Self::Homomorphism(w) => w.binding(),
            Self::Refutation(w) => w.binding(),
            Self::Freeze(w) => w.binding(),
            Self::Comprehension(w) => w.binding(),
            Self::Reflexivity(w) => w.binding(),
            Self::DataRange(w) => w.binding(),
            Self::Composite(w) => w.binding(),
        }
    }

    /// How many distinct triples the premise closure this warrant is against holds.
    ///
    /// The closure itself is not exposed: a warrant is evidence for one conclusion, not an
    /// alternative way to read a closure out of a call that already returns one.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        match self {
            Self::Homomorphism(w) => w.closure_size(),
            Self::Refutation(w) => w.closure_size(),
            Self::Freeze(w) => w.closure_size(),
            Self::Comprehension(w) => w.closure_size(),
            Self::Reflexivity(w) => w.closure_size(),
            Self::DataRange(w) => w.closure_size(),
            Self::Composite(w) => w.closure_size(),
        }
    }

    /// This warrant with the fold's residual `binding` attached.
    ///
    /// Crate-internal, and the ONE place a lane warrant acquires a binding: a lane establishes
    /// what it reads and never matches a residual of its own, so the mapping arrives from
    /// [`entails`](super::entails)'s single final homomorphism once every lane has had its
    /// turn. A composite already owns the binding and a nested one would be two owners, so the
    /// arm is written out and refuses rather than defaulting.
    pub(crate) fn with_binding(self, binding: Binding) -> Self {
        match self {
            Self::Homomorphism(w) => Self::Homomorphism(w.with_binding(binding)),
            Self::Refutation(w) => Self::Refutation(w.with_binding(binding)),
            Self::Freeze(w) => Self::Freeze(w.with_binding(binding)),
            Self::Comprehension(w) => Self::Comprehension(w.with_binding(binding)),
            Self::Reflexivity(w) => Self::Reflexivity(w.with_binding(binding)),
            Self::DataRange(w) => Self::DataRange(w.with_binding(binding)),
            Self::Composite(w) => Self::Composite(w),
        }
    }

    /// The premise closure this warrant is against.
    pub(crate) const fn closure(&self) -> &Closure {
        match self {
            Self::Homomorphism(w) => w.closure(),
            Self::Refutation(w) => w.closure(),
            Self::Freeze(w) => w.closure(),
            Self::Comprehension(w) => w.closure(),
            Self::Reflexivity(w) => w.closure(),
            Self::DataRange(w) => w.closure(),
            Self::Composite(w) => w.closure(),
        }
    }
}

/// The evidence of a HOMOMORPHISM: the mapping that made the conclusion true, and the
/// closure it maps into.
#[derive(Debug, Clone)]
pub struct HomomorphismWarrant {
    /// The regime the closure was computed under.
    regime: Regime,
    /// What each existential of the conclusion was bound to.
    binding: Binding,
    /// The closure the mapping lands in.
    closure: Closure,
}

impl HomomorphismWarrant {
    /// Mint a warrant. Crate-internal: the only producer is the homomorphism search.
    pub(crate) const fn new(regime: Regime, binding: Binding, closure: Closure) -> Self {
        Self {
            regime,
            binding,
            closure,
        }
    }

    /// The regime whose closure this warrant is against.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The mapping: what each of the conclusion's existentials was bound to.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        &self.binding
    }

    /// How many distinct triples the closure this warrant is against holds.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        self.closure.len()
    }

    /// The closure this warrant is against.
    pub(crate) const fn closure(&self) -> &Closure {
        &self.closure
    }

    /// This warrant with `binding` in place of its own.
    pub(crate) fn with_binding(mut self, binding: Binding) -> Self {
        self.binding = binding;
        self
    }

    /// The image of `conclusion` under this warrant's mapping — the closure triples that
    /// witness it — or `None` if the mapping leaves a position of the conclusion unbound.
    ///
    /// This is what a caller renders when it wants to SHOW why an entailment holds, and it
    /// is computed rather than stored so it cannot disagree with the mapping.
    #[must_use]
    pub fn witnesses(&self, conclusion: &RdfDataset) -> Option<Vec<[TermValue; 3]>> {
        conclusion_patterns(conclusion)
            .iter()
            .map(|pat| {
                Some([
                    substitute(&pat[0], &self.binding)?,
                    substitute(&pat[1], &self.binding)?,
                    substitute(&pat[2], &self.binding)?,
                ])
            })
            .collect()
    }
}

/// The evidence of a FOLD: which mechanisms contributed, in order, and the mapping of what
/// none of them discharged.
///
/// # Why the constituents are ORDERED and why the order is not the fold's choice
///
/// It is the fixed cost order [`super::entails`] folds the mechanisms in — an array in the
/// source, not a search's arrival order. A composite is
/// therefore a function of the premise and the conclusion alone: two runs over one question
/// produce the same constituents in the same sequence, and [`verify`] can REPLAY the fold's
/// accounting — re-reading the conclusion once per constituent, subtracting what each
/// discharged — instead of trusting an index list the warrant carries. A warrant whose
/// constituents were in the order a search happened to reach them would be unreplayable, and
/// an unreplayable warrant is a claim rather than evidence.
///
/// # What it does NOT carry
///
/// The discharged triples, and the minted ones. Both are functions of the conclusion, the
/// closure and the constituent list, so [`verify`] recomputes them; storing a copy would be
/// storing something that can disagree with the evidence beside it.
#[derive(Debug, Clone)]
pub struct CompositeWarrant {
    /// The regime every constituent's closure was computed under.
    regime: Regime,
    /// The contributing warrants, in the fixed cost order the fold tries them. Never fewer
    /// than two — one is that lane's own warrant — and never itself a composite.
    parts: Vec<EntailmentWarrant>,
    /// What each existential of the SURVIVING residual was bound to.
    binding: Binding,
    /// The premise's own closure, which the residual maps into once the constituents' mints
    /// are added to it.
    closure: Closure,
}

impl CompositeWarrant {
    /// Mint a composite. Crate-internal: the only producer is
    /// [`entails`](super::entails)'s fold.
    pub(crate) const fn new(
        regime: Regime,
        parts: Vec<EntailmentWarrant>,
        binding: Binding,
        closure: Closure,
    ) -> Self {
        Self {
            regime,
            parts,
            binding,
            closure,
        }
    }

    /// The regime every constituent ran under.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The contributing warrants, in the order the fold tried their mechanisms.
    #[must_use]
    pub fn parts(&self) -> &[EntailmentWarrant] {
        &self.parts
    }

    /// WHICH mechanisms contributed, in the same order.
    ///
    /// Derived from [`Self::parts`] rather than stored beside it, so the list cannot name a
    /// mechanism no constituent came from.
    #[must_use]
    pub fn mechanisms(&self) -> Vec<EntailmentMechanism> {
        self.parts
            .iter()
            .map(EntailmentWarrant::mechanism)
            .collect()
    }

    /// The mapping that discharged whatever no constituent did.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        &self.binding
    }

    /// How many distinct triples the premise closure this warrant is against holds.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        self.closure.len()
    }

    /// The premise closure the residual maps into.
    pub(crate) const fn closure(&self) -> &Closure {
        &self.closure
    }
}

impl std::fmt::Display for CompositeWarrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} mechanisms folded:", self.parts.len())?;
        for mechanism in self.mechanisms() {
            write!(f, " {mechanism}")?;
        }
        Ok(())
    }
}

/// Where a mechanism sits in the fixed cost order [`entails`](super::entails) folds them in.
///
/// `None` for the two that are not lanes: the rule table is the fold's own final match rather
/// than a constituent, and a composite never nests one. [`verify`] requires a composite's
/// constituents to be STRICTLY INCREASING in this order, which is what makes a composite
/// replayable — a reordered warrant would replay against a different pending set at every
/// step, and one with a repeated lane would account for the same triples twice.
const fn fold_rank(mechanism: EntailmentMechanism) -> Option<usize> {
    match mechanism {
        EntailmentMechanism::Refutation => Some(0),
        EntailmentMechanism::Freeze => Some(1),
        EntailmentMechanism::Comprehension => Some(2),
        EntailmentMechanism::Reflexivity => Some(3),
        EntailmentMechanism::DataRange => Some(4),
        EntailmentMechanism::StrictTable | EntailmentMechanism::Composite => None,
    }
}

/// What one constituent of a fold accounted for, recomputed rather than trusted.
#[derive(Debug, Default)]
pub(crate) struct Replay {
    /// The conclusion triples it discharged, by index into the conclusion's frozen order.
    pub(crate) discharged: BTreeSet<usize>,
    /// The triples it licensed into the closure.
    pub(crate) minted: Vec<Triple>,
}

/// Re-check ONE constituent's own evidence and recompute what it accounted for.
///
/// `None` is a failed check. Runs no reasoner: every lane's checker is lookups against the
/// closure the constituent carries, plus a re-reading of the conclusion.
fn replay(
    part: &EntailmentWarrant,
    premise: &RdfDataset,
    conclusion: &RdfDataset,
    triples: &[Triple],
    pending: &BTreeSet<usize>,
) -> Option<Replay> {
    // The closure of a premise holds the premise: every lane copies the input through and adds
    // conclusions ABOUT it. A constituent whose closure does not is evidence about some other
    // premise, and checking it here rather than once per lane is what keeps a composite from
    // smuggling one in beside five honest ones.
    let held: BTreeSet<Triple> = default_graph_triples(premise).into_iter().collect();
    if !held.iter().all(|triple| part.closure().contains(triple)) {
        return None;
    }
    match part {
        // The residual homomorphism is the fold's own, checked once by `verify` below, so this
        // arm accounts for nothing and mints nothing.
        EntailmentWarrant::Homomorphism(_) => Some(Replay::default()),
        EntailmentWarrant::Refutation(refuted) => {
            verify_refutation(refuted, premise, conclusion, triples, pending)
        }
        EntailmentWarrant::Freeze(frozen) => {
            verify_freeze(frozen, premise, conclusion, triples, pending)
        }
        EntailmentWarrant::Comprehension(comprehended) => {
            verify_comprehension(comprehended, premise, conclusion, triples, pending)
        }
        EntailmentWarrant::Reflexivity(reflexive) => {
            verify_reflexivity(reflexive, conclusion, triples, pending)
        }
        EntailmentWarrant::DataRange(ranged) => {
            verify_datarange(ranged, conclusion, triples, pending)
        }
        // The fold never nests one, so a nested composite is a forgery rather than a shape
        // this crate produces. Written out rather than recursed into, because recursing would
        // make the constituent order — the whole of a composite's replayability — a tree
        // nothing pins.
        EntailmentWarrant::Composite(_) => None,
    }
}

/// Re-decide a warrant, without running a reasoner.
///
/// Returns whether `w` really is evidence that `premise` entails `conclusion` under the
/// mechanism that minted it. See the [module docs](self) for what each arm checks, what this
/// does not check, and which checker owns that.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_entail::{EntailmentOutcome, ImportMap, Regime, entails, verify};
///
/// let mut b = RdfDatasetBuilder::new();
/// let cat = b.intern_iri("http://example.org/Cat");
/// let animal = b.intern_iri("http://example.org/Animal");
/// let tom = b.intern_iri("http://example.org/tom");
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// b.push_quad(cat, sub, animal, None);
/// b.push_quad(tom, ty, cat, None);
/// let premise = b.freeze().expect("freeze");
///
/// // `tom a Animal` is not asserted; `cax-sco` derives it.
/// let mut c = RdfDatasetBuilder::new();
/// let tom = c.intern_iri("http://example.org/tom");
/// let ty = c.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// let animal = c.intern_iri("http://example.org/Animal");
/// c.push_quad(tom, ty, animal, None);
/// let conclusion = c.freeze().expect("freeze");
///
/// let certificate =
///     entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new()).expect("a consistent run");
/// let EntailmentOutcome::Entailed(warrant) = certificate.outcome() else {
///     panic!("cax-sco derives it");
/// };
/// assert!(verify(warrant, &premise, &conclusion));
/// // …and it says WHICH closure triple witnesses it.
/// let purrdf_entail::EntailmentWarrant::Homomorphism(mapped) = warrant else {
///     panic!("an ordinary conclusion is reached by matching");
/// };
/// assert_eq!(mapped.witnesses(&conclusion).expect("covered").len(), 1);
/// ```
#[must_use]
pub fn verify(w: &EntailmentWarrant, premise: &RdfDataset, conclusion: &RdfDataset) -> bool {
    let triples = default_graph_triples(conclusion);
    let mut pending: BTreeSet<usize> = (0..triples.len()).collect();
    let mut minted: Vec<Triple> = Vec::new();
    // A composite is its constituents in order; anything else is its own single constituent.
    // One code path, so a lane warrant and a lane INSIDE a composite are checked identically.
    let parts: &[EntailmentWarrant] = match w {
        EntailmentWarrant::Composite(composite) => {
            // The fold never mints a composite of fewer than two — one lane is that lane's own
            // warrant — so a shorter list is a shape nothing here produces.
            if composite.parts().len() < 2 {
                return false;
            }
            // …and it mints them in `MECHANISMS` order, so a warrant whose constituents are in
            // any other order is one this crate did not produce and could not replay: each
            // step's reading depends on what the steps before it discharged.
            let mut previous: Option<usize> = None;
            for mechanism in composite.mechanisms() {
                let Some(rank) = fold_rank(mechanism) else {
                    return false;
                };
                if previous.is_some_and(|seen| seen >= rank) {
                    return false;
                }
                previous = Some(rank);
            }
            composite.parts()
        }
        other => std::slice::from_ref(other),
    };
    for part in parts {
        let Some(accounted) = replay(part, premise, conclusion, &triples, &pending) else {
            return false;
        };
        pending.retain(|index| !accounted.discharged.contains(index));
        minted.extend(accounted.minted);
    }

    // …and the surviving residual is the ordinary homomorphism claim, checked the ordinary
    // way, against the closure the constituents widened.
    let closure = w.closure();
    let extended = if minted.is_empty() {
        None
    } else {
        Some(closure.extended_with(minted))
    };
    let target = extended.as_ref().unwrap_or(closure);
    let binding = w.binding();
    let image: Option<Vec<Triple>> = patterns_at(&triples, &pending)
        .iter()
        .map(|pat| {
            Some([
                substitute(&pat[0], binding)?,
                substitute(&pat[1], binding)?,
                substitute(&pat[2], binding)?,
            ])
        })
        .collect();
    let Some(image) = image else {
        return false;
    };
    if !image.iter().all(|triple| target.contains(triple)) {
        return false;
    }
    // The closure of a premise holds the premise. Already checked per constituent; checked
    // again for the warrant's own closure, which is the one the residual just mapped into.
    default_graph_triples(premise)
        .iter()
        .all(|triple| closure.contains(triple))
}

#[cfg(test)]
mod tests {
    use super::EntailmentMechanism;

    /// EVERY mechanism has a name and a reason of its OWN.
    ///
    /// The name is what crosses the C and WASM boundaries and what a rendered report's
    /// `mechanism` line carries, so two arms sharing one spelling would make a consumer
    /// unable to tell which mechanism answered — the exact fact the line exists to state.
    /// The reason is the semantic boundary of the rule table the mechanism crosses, and an
    /// arm that borrowed another's would be describing a limit it does not have.
    #[test]
    fn every_mechanism_has_a_name_and_a_reason_of_its_own() {
        let mut names: Vec<&str> = EntailmentMechanism::ALL
            .into_iter()
            .map(EntailmentMechanism::as_str)
            .collect();
        assert_eq!(names.len(), 7);
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(count, names.len(), "two mechanisms share a spelling");

        let mut reasons: Vec<&str> = EntailmentMechanism::ALL
            .into_iter()
            .map(EntailmentMechanism::reason)
            .collect();
        for (mechanism, reason) in EntailmentMechanism::ALL.into_iter().zip(reasons.iter()) {
            assert!(!reason.is_empty(), "{mechanism} states no reason");
            // A rendered `mechanism` line is ONE line, so a reason holding a newline would
            // split it into two facts a consumer would parse as one unknown and one
            // malformed.
            assert!(!reason.contains('\n'), "{mechanism}'s reason spans lines");
            assert_eq!(mechanism.to_string(), mechanism.as_str());
        }
        reasons.sort_unstable();
        let count = reasons.len();
        reasons.dedup();
        assert_eq!(count, reasons.len(), "two mechanisms share a reason");
    }

    /// The names are the SPELLINGS a host renders, pinned so a rename is a deliberate act.
    ///
    /// They cross the C ABI and the WASM boundary as these strings and never as an enum
    /// ordinal, so a consumer's reading of `refutation` cannot be moved by a seventh arm
    /// being inserted above it.
    #[test]
    fn the_mechanism_spellings_are_pinned() {
        assert_eq!(
            EntailmentMechanism::ALL.map(EntailmentMechanism::as_str),
            [
                "strict-table",
                "refutation",
                "freeze",
                "comprehension",
                "reflexivity",
                "data-range",
                "composite",
            ]
        );
    }
}
