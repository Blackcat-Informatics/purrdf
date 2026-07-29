// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Description-Logic reasoner: the standard OWL 2 Direct-Semantics services, each
//! certified.
//!
//! [`Reasoner`] is a **façade**, not an engine. It owns the reverse-mapped knowledge base
//! and the named vocabulary to range over, and it delegates every question to a service
//! that has exactly one reason to change:
//!
//! | service | module | what it decides |
//! |---|---|---|
//! | consistency | [`certificate`] | does the ontology have a model at all |
//! | class satisfiability | [`certificate`] | can this class have an instance |
//! | classification | [`classify`] | the subsumption relation over the named classes |
//! | realization | [`realize`] | the entailed types of the named individuals |
//! | instance retrieval | [`realize`] | which named individuals are in a class |
//! | axiom entailment | [`axiom`] | does the ontology entail this axiom |
//! | module extraction | [`module`] | which axioms an ontology needs for a signature |
//! | profile certification | [`mod@profile`] | which OWL 2 profiles the ontology is provably in |
//!
//! Query-directed materialization ([`materialize_dl_reported`](crate::materialize_dl_reported)) is the
//! seventh service and keeps its own entry point: it answers with a DATASET rather than a
//! verdict, and folding a dataset-returning call into this surface would make the façade
//! two things.
//!
//! # Every service returns a certificate
//!
//! Each answer arrives wrapped in [`Certified`], carrying a [`DlCertificate`] that says how
//! complete it is. That is not decoration — see [`certificate`] for why the chase's
//! [`Completeness`](crate::Completeness) is structurally incapable of reporting a tableau's
//! incompleteness, and what this one measures instead.
//!
//! # Why some services take `&mut self`
//!
//! A `Reasoner` is a working state, not a snapshot. Asking about a class or an individual
//! the ontology never mentioned INTERNS it — that is the correct Direct-Semantics reading,
//! since an unconstrained name is a real name with real (mostly negative) answers — and
//! interning grows the concept table. So the three services that take a term from the
//! caller take `&mut self`, and the three that range over the ontology's own vocabulary
//! take `&self`. Nothing is hidden behind interior mutability: the signature says which
//! calls can grow the state.
//!
//! # Determinism
//!
//! Named classes and individuals are visited in ascending interned-term-id order — which
//! is parse order, a function of the dataset alone — every decision matrix is a dense
//! positional `Vec`, and every emitted sequence is sorted by a total, dataset-independent term key. No result is
//! read out of a hash map, and the tableau's budget is a STEP count rather than a clock
//! reading, so two runs over one dataset produce byte-identical answers and byte-identical
//! certificates, on native targets and on `wasm32` alike.

use std::collections::BTreeSet;

use purrdf_core::{RdfDataset, TermValue};

use crate::EntailError;
use crate::owl_dl::concept::Concept;
use crate::owl_dl::parser::Vocab;
use crate::owl_dl::query::{build_data_index, collect_named_classes};
use crate::owl_dl::tableau::{Assumptions, step_cap};
use crate::owl_dl::{Kb, class_concept};
use crate::report::Construct;

pub mod axiom;
pub mod certificate;
pub mod classify;
pub mod module;
pub mod profile;
pub mod realize;

pub use axiom::DlAxiom;
pub use certificate::{Certified, DlCertificate, DlCompleteness, Verdict};
pub use classify::ClassHierarchy;
pub use module::{ConservativeKeep, ModuleExtraction, ModuleMethod, extract_module};
pub use profile::{OwlProfile, ProfileCertificate, ProfileViolation, profile};
pub use realize::Realization;

use axiom::{FreshSymbols, both, disjoint, holds_role, holds_role_inclusion, reaches};
use certificate::Session;
use classify::{Subsumptions, subsumes};
use realize::is_instance;

/// A total, dataset-independent sort key for a term.
///
/// Every sequence a reasoner service emits is sorted by this, which is what makes the
/// answers reproducible *and* readable — interned-id order is deterministic too, but it is
/// parse order, so a caller diffing two answers would be reading the input's quad order
/// rather than the reasoner's. The leading discriminant keeps the four term kinds from
/// interleaving; within a kind the order is lexicographic over the term's own identity
/// coordinates, including the RDF 1.2 base direction, so two literals that differ only in
/// direction do not compare equal.
pub(crate) fn term_key(term: &TermValue) -> (u8, String) {
    match term {
        TermValue::Iri(iri) => (0, iri.clone()),
        TermValue::Blank { label, scope } => (1, format!("{}\u{1f}{label}", scope.0)),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => (
            2,
            format!(
                "{datatype}\u{1f}{}\u{1f}{}\u{1f}{lexical_form}",
                language.as_deref().unwrap_or(""),
                match direction {
                    Some(purrdf_core::RdfTextDirection::Ltr) => "ltr",
                    Some(purrdf_core::RdfTextDirection::Rtl) => "rtl",
                    None => "",
                }
            ),
        ),
        TermValue::Triple { s, p, o } => {
            let (sk, sv) = term_key(s);
            let (pk, pv) = term_key(p);
            let (ok, ov) = term_key(o);
            (
                3,
                format!("{sk}\u{1f}{sv}\u{1e}{pk}\u{1f}{pv}\u{1e}{ok}\u{1f}{ov}"),
            )
        }
    }
}

/// The OWL 2 Direct-Semantics reasoning services over one dataset.
///
/// See the [module docs](self) for the service list, the certificate discipline, and why
/// three of the six take `&mut self`.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_entail::reasoner::{DlAxiom, Reasoner, Verdict};
///
/// let mut b = RdfDatasetBuilder::new();
/// let cat = b.intern_iri("http://example.org/Cat");
/// let animal = b.intern_iri("http://example.org/Animal");
/// let tom = b.intern_iri("http://example.org/tom");
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// b.push_quad(cat, sub, animal, None);
/// b.push_quad(tom, ty, cat, None);
/// let dataset = b.freeze().expect("freeze");
///
/// let mut reasoner = Reasoner::new(&dataset).expect("reverse-map the ontology");
/// assert_eq!(*reasoner.consistency().answer(), Verdict::True);
///
/// // The subsumption is asserted, so of course it is entailed…
/// let axiom = DlAxiom::SubClassOf {
///     sub: purrdf_core::TermValue::iri("http://example.org/Cat"),
///     sup: purrdf_core::TermValue::iri("http://example.org/Animal"),
/// };
/// let answer = reasoner.entails(&axiom).expect("consistent");
/// assert_eq!(*answer.answer(), Verdict::True);
/// // …and the certificate says the whole ontology was read: `Decided` is a variant
/// // `completeness` only returns when `boundaries` is empty.
/// assert!(answer.certificate().completeness().is_decided());
/// assert!(answer.certificate().boundaries().is_empty());
///
/// // The class assertion is not asserted, and IS entailed.
/// let derived = DlAxiom::ClassAssertion {
///     individual: purrdf_core::TermValue::iri("http://example.org/tom"),
///     class: purrdf_core::TermValue::iri("http://example.org/Animal"),
/// };
/// assert_eq!(*reasoner.entails(&derived).expect("consistent").answer(), Verdict::True);
/// ```
pub struct Reasoner {
    /// The reverse-mapped knowledge base.
    kb: Kb,
    /// The interned reserved vocabulary, for reading `owl:Thing`/`owl:Nothing` as `⊤`/`⊥`.
    vocab: Vocab,
    /// The named classes to range over: `(term id, concept id)`, ascending by term id.
    classes: Vec<(u32, u32)>,
    /// The named individuals to range over, ascending by term id.
    individuals: Vec<u32>,
    /// The per-decision step cap.
    cap: u64,
    /// The constructs the reverse mapping could not turn into DL clauses.
    boundaries: BTreeSet<Construct>,
    /// The refutation symbol generator, seeded to avoid every blank label in the data.
    fresh: FreshSymbols,
}

impl std::fmt::Debug for Reasoner {
    /// The SHAPE of the knowledge base, not its contents.
    ///
    /// A `Kb` is thousands of interned ids; printing it would produce an unreadable dump
    /// and would expose an internal representation through a derive. What a reader of a
    /// debug line actually wants is how big the problem is and how much budget each
    /// decision gets — so the knowledge base, the interned vocabulary and the refutation
    /// symbol generator are deliberately elided, which `finish_non_exhaustive` says out
    /// loud rather than leaving a reader to wonder.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reasoner")
            .field("classes", &self.classes.len())
            .field("individuals", &self.individuals.len())
            .field("step_cap", &self.cap)
            .field("boundaries", &self.boundaries)
            .finish_non_exhaustive()
    }
}

impl Reasoner {
    /// Reverse-map `ds` into a knowledge base and open the reasoning services over it.
    ///
    /// The named vocabulary is fixed here — every IRI the data uses in a class-denoting
    /// position, plus `owl:Thing` and `owl:Nothing`, plus every named individual — so that
    /// [`Reasoner::classify`] and [`Reasoner::realize`] range over a set that is a function
    /// of the dataset rather than of the order somebody asked questions in.
    ///
    /// # Errors
    ///
    /// [`EntailError::Parse`] on a malformed OWL class-expression graph;
    /// [`EntailError::Build`] or [`EntailError::Unsatisfiable`] if applying an
    /// `owl:hasKey` axiom exhausts the tableau or finds the ontology already unsatisfiable.
    pub fn new(ds: &RdfDataset) -> Result<Self, EntailError> {
        let mut kb = Kb::from_dataset(ds)?;
        let vocab = Vocab::intern(&mut kb.interner);
        let index = build_data_index(ds, &mut kb.interner);
        let named = collect_named_classes(&kb.interner, &index, &vocab);
        let classes: Vec<(u32, u32)> = named
            .iter()
            .map(|&c| (c, kb.table.intern(class_concept(&vocab, c))))
            .collect();
        let individuals: Vec<u32> = kb.individuals.iter().copied().collect();
        kb.finalize();
        let cap = step_cap(&kb);
        let boundaries = kb.boundaries().clone();
        let fresh = FreshSymbols::for_interner(&kb.interner);
        Ok(Self {
            kb,
            vocab,
            classes,
            individuals,
            cap,
            boundaries,
            fresh,
        })
    }

    /// Narrow the per-decision step cap to `cap`.
    ///
    /// **Narrow only.** The value is clamped to [`Reasoner::step_cap`], which is a pure
    /// function of the knowledge base's size, so this can lower the ceiling and can never
    /// raise it. Ceilings in this repository are measured and reported, not tuned upward
    /// until a hard instance fits — and the honest way to make a hard instance answerable
    /// is to make the reasoner cheaper, which shows up as a smaller
    /// [`DlCertificate::steps`] rather than as a bigger cap.
    ///
    /// It exists so the [`DlCompleteness::BudgetExhausted`] path is reachable from a test
    /// rather than being a branch nobody has ever executed.
    #[must_use]
    pub fn with_step_cap(mut self, cap: u64) -> Self {
        self.cap = self.cap.min(cap);
        self
    }

    /// The per-decision step cap every tableau run of this reasoner runs under.
    #[must_use]
    pub const fn step_cap(&self) -> u64 {
        self.cap
    }

    /// The named classes this reasoner ranges over, in the order it visits them.
    #[must_use]
    pub fn signature(&self) -> Vec<TermValue> {
        self.classes
            .iter()
            .map(|&(term, _)| self.kb.interner.value(term).clone())
            .collect()
    }

    /// The named individuals this reasoner ranges over, in the order it visits them.
    #[must_use]
    pub fn named_individuals(&self) -> Vec<TermValue> {
        self.individuals
            .iter()
            .map(|&term| self.kb.interner.value(term).clone())
            .collect()
    }

    /// Whether the ontology has a model.
    ///
    /// The only service that does not fail on an unsatisfiable knowledge base, because it
    /// is the service that DETECTS one: it answers [`Verdict::False`] where every other
    /// service answers [`EntailError::Unsatisfiable`].
    #[must_use]
    pub fn consistency(&self) -> Certified<Verdict> {
        let mut session = Session::new(&self.kb, self.cap);
        let decision = session.decide(&Assumptions::of_kb());
        let answer = if decision.exhausted {
            Verdict::Unknown
        } else if decision.consistent {
            Verdict::True
        } else {
            Verdict::False
        };
        Certified::new(answer, session.certificate(&self.boundaries))
    }

    /// Whether `class` can have an instance in some model of the ontology.
    ///
    /// `owl:Thing` is satisfiable in any consistent ontology (DL interpretation domains are
    /// non-empty) and `owl:Nothing` never is; a class the ontology never mentions is
    /// satisfiable, because nothing constrains it. All three fall out of the refutation
    /// rather than being special-cased.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the ontology has no model at all — every class is
    /// then vacuously unsatisfiable and the answer would say nothing.
    pub fn class_satisfiability(
        &mut self,
        class: &TermValue,
    ) -> Result<Certified<Verdict>, EntailError> {
        let concept = self.concept_of(class);
        let (mut session, usable) = self.open()?;
        let answer = if usable {
            let decision = session.decide(&Assumptions {
                fresh_types: &[concept],
                ..Assumptions::of_kb()
            });
            if decision.exhausted {
                Verdict::Unknown
            } else if decision.consistent {
                Verdict::True
            } else {
                Verdict::False
            }
        } else {
            Verdict::Unknown
        };
        Ok(Certified::new(
            answer,
            session.certificate(&self.boundaries),
        ))
    }

    /// The subsumption hierarchy over the ontology's named classes.
    ///
    /// Costs one tableau decision per ORDERED pair of named classes plus the consistency
    /// check — `n² + 1` in all, which [`DlCertificate::decisions`] reports so the cost is a
    /// measurement rather than a surprise.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the ontology has no model: every class then
    /// subsumes every other and the hierarchy would be a complete graph carrying no
    /// information.
    pub fn classify(&self) -> Result<Certified<ClassHierarchy>, EntailError> {
        let (mut session, usable) = self.open()?;
        let answer = if usable {
            let matrix = Subsumptions::decide(&mut session, &self.classes);
            ClassHierarchy::derive(&self.kb, &self.classes, &matrix)
        } else {
            ClassHierarchy::default()
        };
        Ok(Certified::new(
            answer,
            session.certificate(&self.boundaries),
        ))
    }

    /// The entailed types of the ontology's named individuals, and the most specific of
    /// them.
    ///
    /// Classifies first, because "most specific" is a question about the hierarchy; the two
    /// share one session, so [`DlCertificate::steps`] covers both passes.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the ontology has no model.
    pub fn realize(&self) -> Result<Certified<Realization>, EntailError> {
        let (mut session, usable) = self.open()?;
        let answer = if usable {
            let matrix = Subsumptions::decide(&mut session, &self.classes);
            Realization::decide(&mut session, &self.individuals, &self.classes, &matrix)
        } else {
            Realization::default()
        };
        Ok(Certified::new(
            answer,
            session.certificate(&self.boundaries),
        ))
    }

    /// The named individuals entailed to be instances of `class`, sorted.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the ontology has no model — every individual would
    /// then be an instance of every class.
    pub fn instances(
        &mut self,
        class: &TermValue,
    ) -> Result<Certified<Vec<TermValue>>, EntailError> {
        let concept = self.concept_of(class);
        let (mut session, usable) = self.open()?;
        let mut answer: Vec<TermValue> = Vec::new();
        if usable {
            for &individual in &self.individuals {
                if is_instance(&mut session, individual, concept).is_true() {
                    answer.push(self.kb.interner.value(individual).clone());
                }
            }
        }
        answer.sort_by_key(term_key);
        Ok(Certified::new(
            answer,
            session.certificate(&self.boundaries),
        ))
    }

    /// Whether the ontology entails `axiom`.
    ///
    /// Decided by refutation — the ontology plus the axiom's negation is tested for a
    /// model. See [`axiom`] for each variant's encoding and for where the fresh symbols a
    /// role-inclusion refutation needs come from.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the ontology has no model, in which case every
    /// axiom is entailed and the answer would be worthless.
    pub fn entails(&mut self, axiom: &DlAxiom) -> Result<Certified<Verdict>, EntailError> {
        let plan = self.plan(axiom);
        let (mut session, usable) = self.open()?;
        let answer = if usable {
            plan.run(&mut session)
        } else {
            Verdict::Unknown
        };
        Ok(Certified::new(
            answer,
            session.certificate(&self.boundaries),
        ))
    }

    /// Intern everything `axiom` needs and reduce it to the refutation(s) that decide it.
    ///
    /// Separated from [`Reasoner::entails`] because interning needs `&mut self` while the
    /// session borrows `&self`, and because it puts every axiom's encoding in one readable
    /// place instead of spreading eight of them through a borrow-juggling function.
    fn plan(&mut self, axiom: &DlAxiom) -> Refutation {
        match axiom {
            DlAxiom::SubClassOf { sub, sup } => {
                let sub = self.concept_of(sub);
                let sup = self.concept_of(sup);
                Refutation::Subsumption { sub, sup }
            }
            DlAxiom::EquivalentClasses { left, right } => {
                let left = self.concept_of(left);
                let right = self.concept_of(right);
                Refutation::Equivalence { left, right }
            }
            DlAxiom::DisjointClasses { left, right } => {
                let left = self.concept_of(left);
                let right = self.concept_of(right);
                Refutation::Disjointness { left, right }
            }
            DlAxiom::ClassAssertion { individual, class } => {
                let individual = self.term_of(individual);
                let concept = self.concept_of(class);
                Refutation::Membership {
                    individual,
                    concept,
                }
            }
            DlAxiom::ObjectPropertyAssertion {
                subject,
                property,
                object,
            } => {
                let subject = self.term_of(subject);
                let property = self.term_of(property);
                let object = self.term_of(object);
                let negated_reach = self.negated(reaches(property, object));
                Refutation::RoleAssertion {
                    subject,
                    negated_reach,
                }
            }
            // `a = b` is `a : {b}` — a nominal membership — so refuting it means asserting
            // `a : ¬{b}`, which is exactly what `Membership` does to the concept it is
            // handed. Hence the plain nominal here…
            DlAxiom::SameIndividual { left, right } => {
                let left = self.term_of(left);
                let right = self.term_of(right);
                let concept = self.interned(Concept::nominal(vec![right]));
                Refutation::Membership {
                    individual: left,
                    concept,
                }
            }
            // …and its negation here: refuting `a ≠ b` means ASSERTING `a : {b}`, so the
            // concept handed over is `¬{b}` and `Membership` negates it back.
            DlAxiom::DifferentIndividuals { left, right } => {
                let left = self.term_of(left);
                let right = self.term_of(right);
                let concept = self.negated(Concept::nominal(vec![right]));
                Refutation::Membership {
                    individual: left,
                    concept,
                }
            }
            DlAxiom::SubObjectPropertyOf { sub, sup } => {
                let sub = self.term_of(sub);
                let sup = self.term_of(sup);
                let x = self.fresh.mint(&mut self.kb.interner);
                let y = self.fresh.mint(&mut self.kb.interner);
                let negated_reach = self.negated(reaches(sup, y));
                Refutation::RoleInclusion {
                    x,
                    sub,
                    y,
                    negated_reach,
                }
            }
        }
    }

    /// Intern `concept` and finalize the negation cache, returning its id.
    fn interned(&mut self, concept: Concept) -> u32 {
        let id = self.kb.table.intern(concept);
        self.kb.table.finalize();
        id
    }

    /// Intern `concept`, finalize the negation cache, and return the id of its NEGATION.
    fn negated(&mut self, concept: Concept) -> u32 {
        let id = self.interned(concept);
        self.kb.table.negate(id)
    }

    /// The concept id a class term denotes, interning it if the ontology never used it.
    ///
    /// `owl:Thing` and `owl:Nothing` become `⊤` and `⊥` rather than opaque atomic classes,
    /// which is what makes the boundary answers of every service correct without a special
    /// case at each one.
    fn concept_of(&mut self, class: &TermValue) -> u32 {
        let term = self.kb.interner.intern(class.clone());
        let id = self.kb.table.intern(class_concept(&self.vocab, term));
        self.kb.table.finalize();
        id
    }

    /// The term id an individual or property term denotes, interning it if new.
    fn term_of(&mut self, term: &TermValue) -> u32 {
        self.kb.interner.intern(term.clone())
    }

    /// Open a session, refusing an unsatisfiable ontology up front.
    ///
    /// The `bool` is whether the session is USABLE: `false` means the consistency check
    /// itself ran out of budget, so nothing downstream can be trusted and the caller
    /// returns an empty answer under a [`DlCompleteness::BudgetExhausted`] certificate
    /// rather than reasoning on top of an unknown.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the ontology has no model.
    fn open(&self) -> Result<(Session<'_>, bool), EntailError> {
        let mut session = Session::new(&self.kb, self.cap);
        let decision = session.decide(&Assumptions::of_kb());
        if decision.exhausted {
            return Ok((session, false));
        }
        if !decision.consistent {
            return Err(EntailError::Unsatisfiable);
        }
        Ok((session, true))
    }
}

/// One axiom reduced to the tableau question(s) that decide it.
///
/// Every field is an already-interned id, so running a refutation needs no mutable access
/// to the reasoner — which is what lets [`Reasoner::entails`] intern first and reason
/// second without fighting the borrow checker or hiding the state behind a `RefCell`.
enum Refutation {
    /// `sub ⊑ sup`, over a fresh anonymous witness.
    Subsumption {
        /// The subsumed concept id.
        sub: u32,
        /// The subsuming concept id.
        sup: u32,
    },
    /// `left ≡ right`, as two subsumptions.
    Equivalence {
        /// One concept id.
        left: u32,
        /// The other.
        right: u32,
    },
    /// `left ⊓ right ⊑ ⊥`.
    Disjointness {
        /// One concept id.
        left: u32,
        /// The other.
        right: u32,
    },
    /// `individual : concept`.
    Membership {
        /// The individual's term id.
        individual: u32,
        /// The concept id it is claimed to belong to.
        concept: u32,
    },
    /// `subject property object`, as `subject : ∃property.{object}`.
    RoleAssertion {
        /// The subject's term id.
        subject: u32,
        /// The interned `¬∃property.{object}`.
        negated_reach: u32,
    },
    /// `sub ⊑ sup` between two roles, over a fresh pair.
    RoleInclusion {
        /// The fresh subject symbol.
        x: u32,
        /// The sub-property's term id.
        sub: u32,
        /// The fresh object symbol.
        y: u32,
        /// The interned `¬∃sup.{y}`.
        negated_reach: u32,
    },
}

impl Refutation {
    /// Decide this refutation in `session`.
    fn run(&self, session: &mut Session<'_>) -> Verdict {
        match *self {
            Self::Subsumption { sub, sup } => subsumes(session, sub, sup),
            Self::Equivalence { left, right } => {
                let forward = subsumes(session, left, right);
                // A demonstrably failed direction settles the equivalence, so the second
                // decision is not spent. `both` would give the same answer; not asking is
                // what keeps `DlCertificate::steps` an honest measure of work done.
                if matches!(forward, Verdict::False) {
                    return Verdict::False;
                }
                both(forward, subsumes(session, right, left))
            }
            Self::Disjointness { left, right } => disjoint(session, left, right),
            Self::Membership {
                individual,
                concept,
            } => is_instance(session, individual, concept),
            Self::RoleAssertion {
                subject,
                negated_reach,
            } => holds_role(session, subject, negated_reach),
            Self::RoleInclusion {
                x,
                sub,
                y,
                negated_reach,
            } => holds_role_inclusion(session, x, sub, y, negated_reach),
        }
    }
}
