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
//! ninth service and keeps its own entry point: it answers with a DATASET rather than a
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
//! # …and every service returns a PROOF TERM
//!
//! A certificate is a MEASUREMENT the search took of itself: a service that answered wrongly
//! reports exactly the same certificate as one that answered rightly. So each answer also
//! carries a [`ServiceProof`] ([`Certified::proof`]) that binds this service's own question,
//! names every tableau run it made together with the assumptions that run received, and says
//! for every claim the answer reports which run decides it. [`ServiceProof::verify`] replays
//! all of it against the CONSUMER's own ontology, and [`ServiceProof::covers`] is where the
//! answer and the proof are compared claim for claim. See [`mod@proof`].
//!
//! Two of the eight services decide SYNTACTICALLY and make no tableau run at all:
//! [`profile`](profile()) walks the axioms and [`extract_module`] runs a locality fixpoint.
//! Neither has a refutation to replay. `profile` therefore carries no proof term, and
//! `extract_module` carries one with ZERO runs whose claim is the extracted module's own
//! canonical identity — which binds the question honestly rather than inventing a search
//! neither performed.
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
//! read out of a hash map, and the tableau's two budgets are a STEP count and a WORK count
//! rather than clock readings, so two runs over one dataset produce byte-identical answers and
//! byte-identical certificates, on native targets and on `wasm32` alike.

use std::collections::BTreeSet;

use purrdf_core::{RdfDataset, TermValue};

use crate::EntailError;
use crate::owl_dl::concept::Concept;
use crate::owl_dl::graph::{Assumptions, Budget};
use crate::owl_dl::parser::Vocab;
use crate::owl_dl::query::{build_data_index, collect_named_classes};
use crate::owl_dl::{Kb, class_concept};
use crate::report::Construct;

pub mod axiom;
pub mod certificate;
pub mod classify;
pub mod module;
pub mod profile;
pub mod proof;
pub mod realize;

pub use axiom::DlAxiom;
pub use certificate::{Certified, DlCertificate, DlCompleteness, Verdict};
pub use classify::ClassHierarchy;
pub use module::{ConservativeKeep, ModuleExtraction, ModuleMethod, extract_module};
pub use profile::{OwlProfile, ProfileCertificate, ProfileViolation, profile};
pub use proof::{
    Claim, ClaimBasis, ClaimSubject, Question, RunAssumptions, RunProof, Service, ServiceProof,
    ServiceReplay, StopCause, StopReceipt,
};
pub use realize::Realization;

use axiom::{FreshSymbols, both, disjoint, holds_role, holds_role_inclusion, reaches};
use certificate::Session;
use classify::{Subsumptions, subsumes};
use proof::{ClaimBasis as Basis, refutation_claim};
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
    /// The per-decision budget: a round cap and a work cap.
    budget: Budget,
    /// The constructs the reverse mapping could not turn into DL clauses.
    boundaries: BTreeSet<Construct>,
    /// The refutation symbol generator, seeded to avoid every blank label in the data.
    fresh: FreshSymbols,
    /// The PRODUCER-INDEPENDENT identity of the dataset this reasoner was built from: BLAKE3
    /// over its RDFC-1.0 canonical N-Quads.
    ///
    /// Computed ONCE here rather than once per service call. It is a canonicalization of the
    /// whole dataset, it does not change between questions, and every [`ServiceProof`] this
    /// reasoner issues binds it so that a consumer can recompute it from their own copy of the
    /// data and refuse a proof produced for anything else.
    input: [u8; 32],
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
            .field("step_cap", &self.budget.steps)
            .field("work_cap", &self.budget.work)
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
        let budget = Budget::for_kb(&kb);
        let boundaries = kb.boundaries().clone();
        let fresh = FreshSymbols::for_interner(&kb.interner);
        Ok(Self {
            kb,
            vocab,
            classes,
            individuals,
            budget,
            boundaries,
            fresh,
            input: crate::owl_dl::proof::ontology_identity(ds),
        })
    }

    /// Apply `question`'s own interning, exactly as the service answering it would.
    ///
    /// The step that makes [`Reasoner::proof_context`] usable for a service other than
    /// consistency. Asking about a class or an axiom the ontology never mentioned INTERNS it —
    /// that is the correct Direct-Semantics reading — and interning grows the concept table the
    /// clause set is derived from. A context built without it would compute a different
    /// contract than the run did and reject an honest proof.
    ///
    /// This is the checker's half of [`TrustBaseEntry`](crate::TrustBaseEntry)'s
    /// `RefutationEncoding` entry: the consumer re-derives the question's encoding from the
    /// question they hold, rather than from anything the producer shipped. It runs no search
    /// and opens no session.
    pub fn prepare(&mut self, question: &Question) {
        match question {
            Question::ClassSatisfiability { class } | Question::InstanceRetrieval { class } => {
                self.concept_of(class);
            }
            Question::AxiomEntailment { axiom } => {
                // The plan itself is discarded: what is wanted is the INTERNING it performed,
                // which is exactly what `Reasoner::entails` does before it reasons.
                let _plan = self.plan(axiom);
            }
            // Consistency, classification and realization range over the ontology's own
            // vocabulary, which `Reasoner::new` already interned; module extraction opens no
            // knowledge base at all.
            Question::Consistency
            | Question::Classification { .. }
            | Question::Realization { .. }
            | Question::ModuleExtraction { .. } => {}
        }
    }

    /// A CHECKING CONTEXT for the proof terms this reasoner's services issue.
    ///
    /// The context a [`ServiceProof`]'s runs are verified against. It has to be built from a
    /// reasoner whose knowledge base was prepared the SAME way — a service interns the concepts
    /// its question needs before it reasons, and a context built from the dataset alone would
    /// derive a different clause set and reject an honest proof. So a consumer checking an
    /// `entails` proof asks the same question of a fresh reasoner first, then takes this.
    ///
    /// It opens no session and runs no search: the only things it computes are the
    /// clausification of the knowledge base and the two digests a proof term is bound to.
    ///
    /// ```
    /// use purrdf_core::{RdfDatasetBuilder, TermValue};
    /// use purrdf_entail::reasoner::{DlAxiom, Question, Reasoner};
    ///
    /// let mut b = RdfDatasetBuilder::new();
    /// let cat = b.intern_iri("http://example.org/Cat");
    /// let animal = b.intern_iri("http://example.org/Animal");
    /// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
    /// b.push_quad(cat, sub, animal, None);
    /// let ds = b.freeze().expect("freeze");
    ///
    /// let axiom = DlAxiom::SubClassOf {
    ///     sub: TermValue::iri("http://example.org/Cat"),
    ///     sup: TermValue::iri("http://example.org/Animal"),
    /// };
    /// let question = Question::AxiomEntailment { axiom: Box::new(axiom.clone()) };
    ///
    /// let mut reasoner = Reasoner::new(&ds).expect("reverse-map");
    /// let answer = reasoner.entails(&axiom).expect("consistent");
    ///
    /// // The consumer checks with a reasoner of their OWN, over their own copy of the data.
    /// let mut checker = Reasoner::new(&ds).expect("reverse-map");
    /// checker.prepare(&question);
    /// let ctx = checker.proof_context();
    /// let certificate = answer.certificate();
    /// let replay = answer
    ///     .proof()
    ///     .verify(&ds, &question, Some(certificate), &ctx)
    ///     .expect("a genuine proof checks");
    /// assert_eq!(replay.runs(), replay.replayed());
    /// ```
    #[must_use]
    pub fn proof_context(self) -> crate::DlProofContext {
        crate::DlProofContext::of_prepared_kb(self.kb, self.input)
    }

    /// The question [`Reasoner::classify`] answers over this reasoner's own class list.
    #[must_use]
    pub fn classification_question(&self) -> Question {
        Question::Classification {
            classes: self.signature(),
        }
    }

    /// The question [`Reasoner::realize`] answers over this reasoner's own vocabulary.
    #[must_use]
    pub fn realization_question(&self) -> Question {
        Question::Realization {
            individuals: self.named_individuals(),
            classes: self.signature(),
        }
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
        self.budget.steps = self.budget.steps.min(cap);
        self
    }

    /// The per-decision step cap every tableau run of this reasoner runs under.
    #[must_use]
    pub const fn step_cap(&self) -> u64 {
        self.budget.steps
    }

    /// Narrow the per-decision WORK cap to `cap`.
    ///
    /// **Narrow only**, clamped exactly as [`Reasoner::with_step_cap`] is and for the same
    /// reason: the ceiling is a pure function of the knowledge base's size, and the honest way
    /// to make a hard instance answerable is to make the reasoner cheaper — which shows up as
    /// a smaller [`DlCertificate::work`] rather than as a bigger cap.
    ///
    /// The cap this narrows bounds what [`Reasoner::step_cap`] structurally cannot: the
    /// matcher, scan, closure and clone work done INSIDE a round. An ontology that makes each
    /// round enormously expensive without making the search take more rounds is bounded here
    /// and nowhere else.
    #[must_use]
    pub fn with_work_cap(mut self, cap: u64) -> Self {
        self.budget.work = self.budget.work.min(cap);
        self
    }

    /// The per-decision work cap every tableau run of this reasoner runs under.
    #[must_use]
    pub const fn work_cap(&self) -> u64 {
        self.budget.work
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
        let mut session = self.open_session();
        let decision = session.decide(&Assumptions::of_kb());
        // `stopped` is checked beside `exhausted`: a run the caller cancelled has closed
        // some branches and not others, exactly like a capped one, and `decision.consistent`
        // is meaningless under either — see [`Decision`].
        let answer = if decision.exhausted || decision.stopped {
            Verdict::Unknown
        } else if decision.consistent {
            Verdict::True
        } else {
            Verdict::False
        };
        // The one service whose claim is the RUN itself: a consistent ontology is exactly a
        // clash-free completion, so this is the one place a `True` rests on a countermodel
        // rather than on a refutation. Filed by hand for that reason.
        let run = session.last_run();
        let basis = match answer {
            Verdict::True => Basis::ExhibitedModel { run },
            Verdict::False => Basis::ClosedRefutation { runs: vec![run] },
            Verdict::Unknown => Basis::Undecided { run },
        };
        let claim = Claim::new(ClaimSubject::Consistent, basis);
        self.seal(session, Question::Consistency, vec![claim], answer)
    }

    /// Open a session over this reasoner's knowledge base, budget and input identity.
    fn open_session(&self) -> Session<'_> {
        Session::new(&self.kb, self.budget, self.input)
    }

    /// Seal a session into an answer, its certificate and its proof term.
    ///
    /// The single seam every certified service goes through, so a service that filed no claims
    /// is a service whose proof term visibly binds nothing rather than one that quietly
    /// returned a bare answer.
    fn seal<T>(
        &self,
        session: Session<'_>,
        question: Question,
        claims: Vec<Claim>,
        answer: T,
    ) -> Certified<T> {
        let certificate = session.certificate(&self.boundaries);
        let proof = session.proof(question, claims, &certificate);
        Certified::new(answer, certificate, proof)
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
        let mut basis = Basis::NotDecided;
        let answer = if usable {
            let decision = session.decide(&Assumptions {
                fresh_types: &[concept],
                ..Assumptions::of_kb()
            });
            let run = session.last_run();
            // See [`Reasoner::consistency`]: `stopped` is read beside `exhausted` for the
            // same reason. Satisfiability is witnessed by a MODEL, so a `True` here rests on
            // the run's clash-free completion rather than on a closed refutation.
            if decision.exhausted || decision.stopped {
                basis = Basis::Undecided { run };
                Verdict::Unknown
            } else if decision.consistent {
                basis = Basis::ExhibitedModel { run };
                Verdict::True
            } else {
                basis = Basis::ClosedRefutation { runs: vec![run] };
                Verdict::False
            }
        } else {
            Verdict::Unknown
        };
        let claim = Claim::new(
            ClaimSubject::ClassSatisfiable {
                class: class.clone(),
            },
            basis,
        );
        let question = Question::ClassSatisfiability {
            class: class.clone(),
        };
        Ok(self.seal(session, question, vec![claim], answer))
    }

    /// The subsumption hierarchy over the ontology's named classes.
    ///
    /// Costs ONE consequence-based saturation over the whole clause set, plus the
    /// consistency check, plus one tableau decision for each pair the saturation could
    /// neither derive nor rule out. Inside the fragment that calculus is complete for — an
    /// `EL⁺⁺`-shaped Horn terminology with no inverse role, no nominal, no disjunction, no
    /// universal restriction and no cardinality restriction — the residue is empty and the
    /// whole taxonomy costs the single consistency decision; outside it the residual pairs are
    /// refuted the way they always were. [`DlCertificate::decisions`] reports how many
    /// tableau runs it actually took, so the cost stays a measurement rather than a surprise.
    ///
    /// # Errors
    ///
    /// [`EntailError::Unsatisfiable`] if the ontology has no model: every class then
    /// subsumes every other and the hierarchy would be a complete graph carrying no
    /// information.
    pub fn classify(&self) -> Result<Certified<ClassHierarchy>, EntailError> {
        let (mut session, usable) = self.open()?;
        let (answer, claims) = if usable {
            let matrix = Subsumptions::decide(&mut session, &self.classes);
            let answer = ClassHierarchy::derive(&self.kb, &self.classes, &matrix);
            let claims = matrix.claims(&self.kb, &self.classes);
            (answer, claims)
        } else {
            (ClassHierarchy::default(), Vec::new())
        };
        Ok(self.seal(session, self.classification_question(), claims, answer))
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
        let (answer, claims) = if usable {
            let matrix = Subsumptions::decide(&mut session, &self.classes);
            let (answer, claims) =
                Realization::decide(&mut session, &self.individuals, &self.classes, &matrix);
            (answer, claims)
        } else {
            (Realization::default(), Vec::new())
        };
        Ok(self.seal(session, self.realization_question(), claims, answer))
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
        let mut claims = Vec::new();
        if usable {
            for &individual in &self.individuals {
                let verdict = is_instance(&mut session, individual, concept);
                let run = session.last_run();
                let name = self.kb.interner.value(individual).clone();
                if verdict.is_true() {
                    answer.push(name.clone());
                }
                // Every individual asked about gets a claim, established or not: an answer
                // that omitted the negative ones would leave a reader unable to tell an
                // individual the search RULED OUT from one it never reached.
                claims.push(refutation_claim(
                    ClaimSubject::Type {
                        individual: name,
                        class: class.clone(),
                    },
                    verdict,
                    &[run],
                ));
            }
        }
        answer.sort_by_key(term_key);
        let question = Question::InstanceRetrieval {
            class: class.clone(),
        };
        Ok(self.seal(session, question, claims, answer))
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
        let mut runs = Vec::new();
        let answer = if usable {
            plan.run(&mut session, &mut runs)
        } else {
            Verdict::Unknown
        };
        let claim = refutation_claim(
            ClaimSubject::Axiom {
                axiom: Box::new(axiom.clone()),
            },
            answer,
            &runs,
        );
        let question = Question::AxiomEntailment {
            axiom: Box::new(axiom.clone()),
        };
        Ok(self.seal(session, question, vec![claim], answer))
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
        let mut session = self.open_session();
        let decision = session.decide(&Assumptions::of_kb());
        // A stopped consistency check is exactly as unusable as an exhausted one: neither
        // tells the caller whether the ontology has a model, so both leave the session
        // unusable rather than falling through to `!decision.consistent`.
        if decision.exhausted || decision.stopped {
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
    /// Decide this refutation in `session`, pushing the index of every run it makes onto
    /// `runs`.
    ///
    /// The run list is what binds the answer: an equivalence is TWO subsumptions, and a claim
    /// that named only one of them would rest on half a decision. So the indices are collected
    /// here, where the decisions are made, rather than reconstructed afterwards from a count.
    fn run(&self, session: &mut Session<'_>, runs: &mut Vec<usize>) -> Verdict {
        let ask = |session: &mut Session<'_>, verdict: Verdict, runs: &mut Vec<usize>| {
            runs.push(session.last_run());
            verdict
        };
        match *self {
            Self::Subsumption { sub, sup } => {
                let verdict = subsumes(session, sub, sup);
                ask(session, verdict, runs)
            }
            Self::Equivalence { left, right } => {
                let forward = subsumes(session, left, right);
                let forward = ask(session, forward, runs);
                // A demonstrably failed direction settles the equivalence, so the second
                // decision is not spent. `both` would give the same answer; not asking is
                // what keeps `DlCertificate::steps` an honest measure of work done.
                if matches!(forward, Verdict::False) {
                    return Verdict::False;
                }
                let backward = subsumes(session, right, left);
                let backward = ask(session, backward, runs);
                both(forward, backward)
            }
            Self::Disjointness { left, right } => {
                let verdict = disjoint(session, left, right);
                ask(session, verdict, runs)
            }
            Self::Membership {
                individual,
                concept,
            } => {
                let verdict = is_instance(session, individual, concept);
                ask(session, verdict, runs)
            }
            Self::RoleAssertion {
                subject,
                negated_reach,
            } => {
                let verdict = holds_role(session, subject, negated_reach);
                ask(session, verdict, runs)
            }
            Self::RoleInclusion {
                x,
                sub,
                y,
                negated_reach,
            } => {
                let verdict = holds_role_inclusion(session, x, sub, y, negated_reach);
                ask(session, verdict, runs)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::RdfDatasetBuilder;
    use purrdf_datalog::StopSignal;

    use super::*;

    /// A stop signal that has already fired — see `certificate::tests::AlreadyStopped` for
    /// why this is how the stop-aware path is reached here: [`Reasoner::new`] never threads a
    /// signal in on its own, so the test attaches one to the built [`Kb`] directly.
    #[derive(Debug)]
    struct AlreadyStopped;

    impl StopSignal for AlreadyStopped {
        fn stopped(&self) -> bool {
            true
        }
    }

    /// A trivially consistent one-axiom ontology, reasoned over with the stop signal already
    /// firing before the first decision this `Reasoner` makes.
    fn stopped_reasoner() -> Reasoner {
        let mut b = RdfDatasetBuilder::new();
        let cat = b.intern_iri("https://example.org/Cat");
        let animal = b.intern_iri("https://example.org/Animal");
        let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
        b.push_quad(cat, sub, animal, None);
        let ds = b.freeze().expect("freeze");
        let mut reasoner = Reasoner::new(&ds).expect("a bare subclass axiom is consistent");
        reasoner.kb.stop = Some(Arc::new(AlreadyStopped) as Arc<dyn StopSignal>);
        reasoner
    }

    /// [`Reasoner::consistency`] — the first of the three sites. Before this fix, a stopped
    /// decision (`exhausted: false, consistent: false`) fell through to `Verdict::False`: a
    /// host cancellation reported as "no model".
    #[test]
    fn a_stopped_consistency_check_answers_unknown_not_false() {
        let reasoner = stopped_reasoner();
        let answer = reasoner.consistency();
        assert_eq!(
            *answer.answer(),
            Verdict::Unknown,
            "a cancellation must not be reported as a refutation"
        );
        assert_eq!(
            answer.certificate().completeness(),
            DlCompleteness::BudgetExhausted
        );
        assert!(answer.certificate().stopped());
    }

    /// [`Reasoner::class_satisfiability`] — the second site. It must not surface the stopped
    /// consistency pre-check as [`EntailError::Unsatisfiable`], and its own answer must be
    /// `Unknown`.
    #[test]
    fn a_stopped_class_satisfiability_check_answers_unknown_not_a_refutation() {
        let mut reasoner = stopped_reasoner();
        let cat = TermValue::iri("https://example.org/Cat");
        let answer = reasoner
            .class_satisfiability(&cat)
            .expect("a stopped pre-check must not surface as EntailError::Unsatisfiable");
        assert_eq!(*answer.answer(), Verdict::Unknown);
        assert!(answer.certificate().stopped());
    }

    /// [`Reasoner::open`] — the third site, shared by `classify`/`realize`/`instances`/
    /// `entails`. A stopped consistency check must leave the session UNUSABLE rather than
    /// being read as `!decision.consistent` and surfaced as [`EntailError::Unsatisfiable`].
    #[test]
    fn open_reports_a_stopped_session_as_unusable_rather_than_unsatisfiable() {
        let reasoner = stopped_reasoner();
        let hierarchy = reasoner
            .classify()
            .expect("a stopped consistency check must not surface as EntailError::Unsatisfiable");
        assert!(hierarchy.certificate().stopped());
        assert_eq!(
            hierarchy.certificate().completeness(),
            DlCompleteness::BudgetExhausted
        );
    }
}
