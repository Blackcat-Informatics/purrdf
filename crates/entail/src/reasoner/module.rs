// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Syntactic locality module extraction (SLME) — `BOT`, `TOP` and `STAR`.
//!
//! A *module* of an ontology `O` for a signature `Σ` is a subset `M ⊆ O` that entails
//! everything `O` entails over `Σ`. Modules are what make a large ontology tractable: a
//! reasoner runs over the module instead of the whole import closure, and every answer
//! about `Σ` is the answer it would have given about `O`.
//!
//! # The doctrine: SOUND, not minimal
//!
//! Over-extraction is acceptable. Under-extraction is not. A module that carries an
//! irrelevant axiom costs a caller some time; a module that drops a relevant one gives a
//! WRONG answer while looking like a faster one, and nothing downstream can tell. Every
//! judgement in this module is therefore made toward keeping:
//!
//! * a triple whose locality this module decides exactly is kept exactly;
//! * a triple it cannot classify — an unhandled predicate, a class expression under a blank
//!   node, an RDF 1.2 triple term — is kept **conservatively** whenever anything it names
//!   or reaches is in `Σ`, and the keep is REPORTED
//!   ([`ModuleExtraction::conservative_keeps`]) rather than passed off as an exact one.
//!
//! `a_module_entails_everything_the_full_ontology_entails_over_its_signature` is the
//! fixture that holds the doctrine to account: it classifies and realizes BOTH the full
//! ontology and the module through [`Reasoner`](super::Reasoner) and demands that every
//! entailment over `Σ` survives.
//!
//! # The three notions
//!
//! `Σ` starts as the caller's seed. A triple with a NAMED subject is an axiom; it is
//! classified under a locality notion, and keeping it adds the entities the rule names to
//! `Σ`, which is iterated to a fixpoint.
//!
//! * `BOT` — the `⊥`-locality fixpoint. Interprets every symbol outside `Σ` as `⊥`, which
//!   keeps everything BELOW the seed: sub-classes are followed upward to their subsumers.
//! * `TOP` — the `⊤`-locality fixpoint. Interprets every symbol outside `Σ` as `⊤`, which
//!   keeps everything ABOVE the seed.
//! * `STAR` — the nested `⊥⊤*` module: alternate a `BOT` pass and a `TOP` pass until the
//!   kept set stops growing. The smallest of the three, and still sound.
//!
//! After the fixpoint the module is every kept triple plus the blank-node closure of its
//! object, so a class expression is never truncated halfway.
//!
//! # Provenance
//!
//! The algorithm is standard (Cuenca Grau, Horrocks, Kazakov & Sattler, "Modular Reuse of
//! Ontologies: Theory and Practice", JAIR 2008); algorithms are not copyrightable. The
//! RDF-native shape of this implementation is lifted, with the copyright owner's
//! authorization, from Blackcat Informatics' `gmeow-ontology` and relicensed here under
//! `MIT OR Apache-2.0`.
//!
//! # Determinism
//!
//! Triples are visited in source quad order, `Σ` and the blank-node index are ordered maps,
//! the kept set is an ordered set of source indices, and the module is emitted in source
//! order. Nothing is read out of a hash map, and no fixpoint decision depends on iteration
//! order.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};

use super::proof::{Claim, ClaimBasis, ClaimSubject, Question, ServiceProof};
use super::term_key;
use crate::EntailError;
use crate::interner::{Interner, intern_into};
use crate::owl_dl::parser::Vocab;
use crate::owl_dl::proof::ontology_identity;
use crate::vocab::{
    OWL_ANNOTATIONPROPERTY, OWL_CLASS, OWL_DATATYPEPROPERTY, OWL_OBJECTPROPERTY, RDF_PROPERTY,
};

/// Which locality notion a module is extracted under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModuleMethod {
    /// `⊥`-locality: everything below the seed.
    Bot,
    /// `⊤`-locality: everything above the seed.
    Top,
    /// The nested `⊥⊤*` module — the smallest of the three, and still sound.
    Star,
}

impl ModuleMethod {
    /// Every method, in declaration order.
    pub const ALL: [Self; 3] = [Self::Bot, Self::Top, Self::Star];

    /// A short, stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bot => "BOT",
            Self::Top => "TOP",
            Self::Star => "STAR",
        }
    }
}

impl std::fmt::Display for ModuleMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One triple kept because the extractor could not decide its locality exactly.
///
/// Reported rather than silent, because a conservative keep is the difference between "this
/// module is minimal" and "this module is a superset" — and a caller sizing a module wants
/// to know which constructs made it bigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConservativeKeep {
    /// The axiom's subject.
    subject: TermValue,
    /// The predicate whose locality was not decided exactly.
    predicate: TermValue,
}

impl ConservativeKeep {
    /// The axiom's subject.
    #[must_use]
    pub const fn subject(&self) -> &TermValue {
        &self.subject
    }

    /// The predicate whose locality was not decided exactly.
    #[must_use]
    pub const fn predicate(&self) -> &TermValue {
        &self.predicate
    }
}

/// An extracted locality module, and what it took to extract it.
#[derive(Debug, Clone)]
pub struct ModuleExtraction {
    /// The module itself.
    module: Arc<RdfDataset>,
    /// The notion it was extracted under.
    method: ModuleMethod,
    /// How many named-subject axioms were kept.
    axioms: usize,
    /// The signature the fixpoint closed to, sorted.
    signature: Vec<TermValue>,
    /// The triples kept conservatively rather than by exact locality, sorted.
    conservative_keeps: Vec<ConservativeKeep>,
    /// The proof term binding the question this extraction answered, when one was asked for.
    proof: Option<ServiceProof>,
}

impl ModuleExtraction {
    /// The extracted module, as a dataset whose default graph holds the kept triples.
    #[must_use]
    pub fn module(&self) -> &Arc<RdfDataset> {
        &self.module
    }

    /// The notion the module was extracted under.
    #[must_use]
    pub const fn method(&self) -> ModuleMethod {
        self.method
    }

    /// How many named-subject axioms were kept.
    ///
    /// Not the module's triple count: the blank-node closure of a kept axiom's object rides
    /// along and is not an axiom of its own.
    #[must_use]
    pub const fn axioms(&self) -> usize {
        self.axioms
    }

    /// The signature the fixpoint closed to — the caller's seed plus every entity a kept
    /// axiom pulled in. Sorted.
    #[must_use]
    pub fn signature(&self) -> &[TermValue] {
        &self.signature
    }

    /// The triples kept conservatively rather than by exact locality, sorted.
    ///
    /// Empty is the strongest thing an extraction can say: every keep was decided by the
    /// locality rules, so the module is as small as those rules make it. A non-empty list
    /// is not an error — it is the SOUND direction of the doctrine, made visible.
    #[must_use]
    pub fn conservative_keeps(&self) -> &[ConservativeKeep] {
        &self.conservative_keeps
    }

    /// The PROOF TERM binding the question this extraction answered, when one was asked for.
    ///
    /// **It carries no tableau run, because this service makes none.** Locality-based module
    /// extraction is a syntactic fixpoint over the triples — there is no search, so there is
    /// no refutation to replay, and a proof term shaped like one would be a fiction. What it
    /// DOES bind is real and checkable: the ontology's producer-independent identity, the
    /// seed signature and the notion the caller asked for, and the extracted module's own
    /// canonical identity. A module proof presented against a different signature, a
    /// different notion or a different extraction is rejected;
    /// [`ServiceReplay::runs`](super::ServiceReplay::runs) answering zero is the report saying
    /// out loud that there was no search to check.
    ///
    /// # Zero runs and no proof at all are DIFFERENT answers
    ///
    /// This is the service where the distinction is sharpest, and it is why
    /// [`Certified::proof`](super::Certified::proof) is an `Option` too. `Some(proof)` with
    /// `proof.runs()` empty is a REAL measurement: the extractor decided syntactically, and
    /// the zero says there was no search to check. `None` — what [`extract_module`] returns —
    /// says nothing was measured at all, because nobody asked. Reading the second as the first
    /// would turn "never recorded" into "checked, and there was nothing to check".
    #[must_use]
    pub const fn proof(&self) -> Option<&ServiceProof> {
        self.proof.as_ref()
    }
}

/// Extract the locality module of `ds` for the seed signature `signature`, RECORDING NOTHING.
///
/// Reads the DEFAULT graph, like every other OWL-Direct entry point in this crate: OWL 2's
/// Direct Semantics is defined over one ontology, and a quad in a named graph is not part of
/// it. Quads outside the default graph are neither read nor emitted.
///
/// [`ModuleExtraction::proof`] is `None`. The proof term this service can issue binds TWO
/// RDFC-1.0 canonicalizations — the source ontology's identity and the extracted module's —
/// and neither is computed here. A caller who wants one asks
/// [`extract_module_with_proofs`]; the extracted module itself is identical either way.
///
/// # Errors
///
/// [`EntailError::Build`] if the extracted module cannot be frozen into a dataset.
///
/// ```
/// use purrdf_core::{RdfDatasetBuilder, TermValue};
/// use purrdf_entail::reasoner::{ModuleMethod, extract_module};
///
/// let mut b = RdfDatasetBuilder::new();
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let cat = b.intern_iri("http://example.org/Cat");
/// let mammal = b.intern_iri("http://example.org/Mammal");
/// let animal = b.intern_iri("http://example.org/Animal");
/// let fish = b.intern_iri("http://example.org/Fish");
/// b.push_quad(cat, sub, mammal, None);
/// b.push_quad(mammal, sub, animal, None);
/// b.push_quad(fish, sub, animal, None);
/// let dataset = b.freeze().expect("freeze");
///
/// // The ⊥-module for {Cat} follows the chain UP and leaves the sibling behind.
/// let seed = [TermValue::iri("http://example.org/Cat")];
/// let extracted = extract_module(&dataset, &seed, ModuleMethod::Bot).expect("extract");
/// assert_eq!(extracted.axioms(), 2);
/// assert!(extracted.conservative_keeps().is_empty());
/// ```
pub fn extract_module(
    ds: &RdfDataset,
    signature: &[TermValue],
    method: ModuleMethod,
) -> Result<ModuleExtraction, EntailError> {
    extract(ds, signature, method, false)
}

/// The same extraction, carrying a PROOF TERM bound to its question.
///
/// The proof makes no tableau run — this service opens none — and says so through
/// [`ServiceReplay::runs`](super::ServiceReplay::runs) answering zero. What it binds is the
/// source ontology's producer-independent identity, the seed signature, the locality notion,
/// and the extracted module's own canonical identity, so a proof presented against a different
/// signature, a different notion or a different extraction is rejected.
///
/// It costs two RDFC-1.0 canonicalizations that [`extract_module`] does not pay for, which is
/// why it is a separate entry point rather than the default.
///
/// # Errors
///
/// [`EntailError::Build`] if the extracted module cannot be frozen into a dataset.
///
/// ```
/// use purrdf_core::{RdfDatasetBuilder, TermValue};
/// use purrdf_entail::reasoner::{ModuleMethod, extract_module, extract_module_with_proofs};
///
/// let mut b = RdfDatasetBuilder::new();
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let cat = b.intern_iri("http://example.org/Cat");
/// let animal = b.intern_iri("http://example.org/Animal");
/// b.push_quad(cat, sub, animal, None);
/// let dataset = b.freeze().expect("freeze");
/// let seed = [TermValue::iri("http://example.org/Cat")];
///
/// // A ZERO-run proof: a real measurement saying there was no search to check.
/// let proved = extract_module_with_proofs(&dataset, &seed, ModuleMethod::Bot).expect("extract");
/// assert!(proved.proof().expect("recorded").runs().is_empty());
///
/// // …which is a different thing from having recorded nothing.
/// let plain = extract_module(&dataset, &seed, ModuleMethod::Bot).expect("extract");
/// assert!(plain.proof().is_none());
/// assert_eq!(plain.axioms(), proved.axioms(), "the module is the same either way");
/// ```
pub fn extract_module_with_proofs(
    ds: &RdfDataset,
    signature: &[TermValue],
    method: ModuleMethod,
) -> Result<ModuleExtraction, EntailError> {
    extract(ds, signature, method, true)
}

/// The shared body of the two entry points; `proofs` is whether to record a proof term.
fn extract(
    ds: &RdfDataset,
    signature: &[TermValue],
    method: ModuleMethod,
    proofs: bool,
) -> Result<ModuleExtraction, EntailError> {
    let mut interner = Interner::default();
    let v = Vocab::intern(&mut interner);
    let declarations: BTreeSet<u32> = [
        OWL_CLASS,
        OWL_OBJECTPROPERTY,
        OWL_DATATYPEPROPERTY,
        OWL_ANNOTATIONPROPERTY,
        RDF_PROPERTY,
    ]
    .into_iter()
    .map(|iri| interner.intern_iri(iri))
    .collect();

    let mut triples: Vec<(u32, u32, u32)> = Vec::new();
    for quad in ds.quads() {
        if quad.g.is_some() {
            continue;
        }
        let s = interner.intern(ds.term_value(quad.s));
        let p = interner.intern(ds.term_value(quad.p));
        let o = interner.intern(ds.term_value(quad.o));
        triples.push((s, p, o));
    }

    // Blank-node subject → the triples it carries, for the closure walks.
    let mut blanks: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (index, &(s, _, _)) in triples.iter().enumerate() {
        if matches!(interner.value(s), TermValue::Blank { .. }) {
            blanks.entry(s).or_default().push(index);
        }
    }

    let mut state = Extraction {
        sigma: signature
            .iter()
            .map(|term| interner.intern(term.clone()))
            .collect(),
        kept: BTreeSet::new(),
        conservative: BTreeSet::new(),
    };
    let context = Context {
        interner: &interner,
        vocab: &v,
        declarations: &declarations,
        triples: &triples,
        blanks: &blanks,
    };

    match method {
        ModuleMethod::Bot => state.fixpoint(&context, Notion::Bot),
        ModuleMethod::Top => state.fixpoint(&context, Notion::Top),
        ModuleMethod::Star => loop {
            let before = (state.kept.len(), state.sigma.len());
            state.fixpoint(&context, Notion::Bot);
            state.fixpoint(&context, Notion::Top);
            if (state.kept.len(), state.sigma.len()) == before {
                break;
            }
        },
    }

    let axioms = state.kept.len();
    // Both endpoints' closures ride along: a kept axiom's SUBJECT may itself be a complex
    // class expression, and half a class expression is not a class expression.
    let mut emitted: BTreeSet<usize> = state.kept.clone();
    for &index in &state.kept {
        context.closure(triples[index].0, &mut emitted);
        context.closure(triples[index].2, &mut emitted);
    }

    let mut b = RdfDatasetBuilder::new();
    for &index in &emitted {
        let (s, p, o) = triples[index];
        let s = intern_into(&mut b, interner.value(s));
        let p = intern_into(&mut b, interner.value(p));
        let o = intern_into(&mut b, interner.value(o));
        b.push_quad(s, p, o, None);
    }
    let module = b
        .freeze()
        .map_err(|e| EntailError::Build(format!("freeze locality module: {e}")))?;

    let mut closed: Vec<TermValue> = state
        .sigma
        .iter()
        .map(|&id| interner.value(id).clone())
        .collect();
    closed.sort_by_key(term_key);
    let mut conservative_keeps: Vec<ConservativeKeep> = state
        .conservative
        .iter()
        .map(|&(subject, predicate)| ConservativeKeep {
            subject: interner.value(subject).clone(),
            predicate: interner.value(predicate).clone(),
        })
        .collect();
    conservative_keeps.sort_by_key(|keep| (term_key(&keep.subject), term_key(&keep.predicate)));

    // The extracted module's own producer-independent identity: the claim a consumer checks
    // this proof term against, and the reason a proof of one extraction cannot stand for
    // another over the same signature.
    //
    // Both `ontology_identity` calls are RDFC-1.0 canonicalizations, and they happen only
    // inside this `then`: the non-recording entry point does not compute a digest it would
    // then drop.
    let proof = proofs.then(|| {
        let claim = Claim::new(
            ClaimSubject::Module {
                digest: ontology_identity(&module),
            },
            ClaimBasis::Syntactic,
        );
        ServiceProof::new(
            ontology_identity(ds),
            Question::ModuleExtraction {
                signature: signature.to_vec(),
                method,
            },
            Vec::new(),
            vec![claim],
            None,
            false,
        )
    });
    Ok(ModuleExtraction {
        module,
        method,
        axioms,
        signature: closed,
        conservative_keeps,
        proof,
    })
}

/// The locality notion one classification pass runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Notion {
    /// Interpret every symbol outside `Σ` as `⊥`.
    Bot,
    /// Interpret every symbol outside `Σ` as `⊤`.
    Top,
}

/// The read-only inputs every classification consults.
struct Context<'a> {
    /// The term interner the ids index.
    interner: &'a Interner,
    /// The reserved vocabulary.
    vocab: &'a Vocab,
    /// The `rdf:type` objects that mark a DECLARATION rather than an instance assertion.
    declarations: &'a BTreeSet<u32>,
    /// The source triples, in quad order.
    triples: &'a [(u32, u32, u32)],
    /// Blank-node subject → the indices of the triples it carries.
    blanks: &'a BTreeMap<u32, Vec<usize>>,
}

impl Context<'_> {
    /// Add the blank-node closure reachable from `term` to `out`.
    fn closure(&self, term: u32, out: &mut BTreeSet<usize>) {
        let mut stack = vec![term];
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        while let Some(node) = stack.pop() {
            if !matches!(self.interner.value(node), TermValue::Blank { .. }) || !seen.insert(node) {
                continue;
            }
            for &index in self.blanks.get(&node).map_or(&[][..], Vec::as_slice) {
                out.insert(index);
                stack.push(self.triples[index].2);
            }
        }
    }

    /// Every named IRI reachable from `term`, itself included when it is one.
    fn reachable_names(&self, term: u32, out: &mut BTreeSet<u32>) {
        if matches!(self.interner.value(term), TermValue::Iri(_)) {
            out.insert(term);
            return;
        }
        let mut stack = vec![term];
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        while let Some(node) = stack.pop() {
            if !matches!(self.interner.value(node), TermValue::Blank { .. }) || !seen.insert(node) {
                continue;
            }
            for &index in self.blanks.get(&node).map_or(&[][..], Vec::as_slice) {
                let (_, p, o) = self.triples[index];
                out.insert(p);
                match self.interner.value(o) {
                    TermValue::Iri(_) => {
                        out.insert(o);
                    }
                    TermValue::Blank { .. } => stack.push(o),
                    _ => {}
                }
            }
        }
    }

    /// Whether `id` is an IRI.
    fn is_iri(&self, id: u32) -> bool {
        matches!(self.interner.value(id), TermValue::Iri(_))
    }

    /// Whether `id` is a literal.
    fn is_literal(&self, id: u32) -> bool {
        matches!(self.interner.value(id), TermValue::Literal { .. })
    }

    /// The predicates whose locality this module decides EXACTLY. Every other predicate
    /// falls to the annotation rule or to the conservative test.
    fn is_logical(&self, predicate: u32) -> bool {
        let v = self.vocab;
        [
            v.sub_class,
            v.equiv_class,
            v.disjoint,
            v.sub_prop,
            v.domain,
            v.range,
            v.inverse_of,
            v.ty,
        ]
        .contains(&predicate)
    }

    /// Whether a BLANK-subject triple states an axiom rather than scaffolding a class
    /// expression.
    ///
    /// This is not a nicety. A GCI whose left-hand side is complex — `∃eats.⊤ ⊑ Predator`,
    /// written `_:ce rdfs:subClassOf :Predator` — has a blank subject, and nothing in the
    /// ontology has `_:ce` as an OBJECT, so a walk that classified named subjects only and
    /// pulled blank nodes in through object closures would never reach it. That axiom would
    /// vanish from every module, which is the under-extraction the whole doctrine forbids.
    /// So an axiom-stating predicate makes its triple an axiom whatever its subject is, and
    /// the module carries the closure of BOTH endpoints.
    ///
    /// Everything else on a blank subject — `owl:onProperty`, `owl:someValuesFrom`,
    /// `rdf:first`, an `rdf:type owl:Restriction` — is syntax for a class expression, and is
    /// pulled in by the closure walk exactly when the expression it belongs to is kept.
    fn is_axiom_predicate(&self, predicate: u32) -> bool {
        let v = self.vocab;
        [
            v.sub_class,
            v.equiv_class,
            v.disjoint,
            v.sub_prop,
            v.domain,
            v.range,
            v.inverse_of,
            v.equiv_prop,
            v.property_disjoint_with,
            v.disjoint_union,
            v.has_key,
            v.members,
            v.distinct_members,
        ]
        .contains(&predicate)
    }
}

/// What one triple's classification decided.
enum Decision {
    /// Local under the notion — drop it.
    Drop,
    /// Non-local by EXACT locality — keep it, adding these entities to `Σ`.
    Keep(Vec<u32>),
    /// Not classified exactly, but it touches `Σ` — keep it and say so.
    Conservative,
}

/// The growing signature, kept set and conservative-keep log.
struct Extraction {
    /// The signature, closed under every keep so far.
    sigma: BTreeSet<u32>,
    /// The indices of the kept named-subject axioms.
    kept: BTreeSet<usize>,
    /// `(subject, predicate)` of every conservatively kept axiom.
    conservative: BTreeSet<(u32, u32)>,
}

impl Extraction {
    /// Grow the signature and the kept set under one notion to a fixpoint.
    fn fixpoint(&mut self, context: &Context<'_>, notion: Notion) {
        loop {
            let mut changed = false;
            for (index, &(s, p, o)) in context.triples.iter().enumerate() {
                if self.kept.contains(&index) {
                    continue;
                }
                // A named subject always states an axiom. A blank subject states one only
                // when its predicate does — see `Context::is_axiom_predicate` for the GCI
                // with a complex left-hand side that this exists to keep.
                let decision = if context.is_iri(s) {
                    self.classify(context, notion, s, p, o)
                } else if context.is_axiom_predicate(p) {
                    self.conservative(context, s, o)
                } else {
                    continue;
                };
                match decision {
                    Decision::Drop => {}
                    Decision::Keep(add) => {
                        self.kept.insert(index);
                        self.sigma.extend(add);
                        changed = true;
                    }
                    Decision::Conservative => {
                        self.kept.insert(index);
                        self.conservative.insert((s, p));
                        // A conservative keep pulls everything it names into `Σ`, so an
                        // axiom anchored to any of those re-enters the fixpoint. Keeping
                        // the triple without growing `Σ` would be the under-extraction the
                        // doctrine forbids.
                        let mut names = BTreeSet::new();
                        context.reachable_names(s, &mut names);
                        context.reachable_names(o, &mut names);
                        self.sigma.extend(names);
                        changed = true;
                    }
                }
            }
            if !changed {
                return;
            }
        }
    }

    /// Classify one named-subject triple under `notion`.
    fn classify(
        &self,
        context: &Context<'_>,
        notion: Notion,
        subject: u32,
        predicate: u32,
        object: u32,
    ) -> Decision {
        let v = context.vocab;
        let in_sigma = |id: u32| self.sigma.contains(&id);

        // A property the caller ASKED for: every assertion over it is part of what the
        // signature means, whatever its endpoints are.
        if in_sigma(predicate) && !context.is_logical(predicate) {
            return if context.is_iri(object) {
                Decision::Keep(vec![subject, object])
            } else if context.is_literal(object) {
                Decision::Keep(vec![subject])
            } else {
                Decision::Conservative
            };
        }

        // A blank-node object is a class expression; exact locality over one is not decided
        // here, so it goes to the conservative test.
        if !context.is_iri(object) && !context.is_literal(object) {
            return self.conservative(context, subject, object);
        }

        if predicate == v.sub_class || predicate == v.sub_prop {
            // `C ⊑ D`. Under ⊥ the axiom is non-local when `C ∈ Σ`; under ⊤ when `D ∈ Σ`.
            let Some(object) = context.is_iri(object).then_some(object) else {
                return self.conservative(context, subject, object);
            };
            return match notion {
                Notion::Bot if in_sigma(subject) => Decision::Keep(vec![object]),
                Notion::Top if in_sigma(object) => Decision::Keep(vec![subject]),
                _ => Decision::Drop,
            };
        }
        if predicate == v.equiv_class || predicate == v.inverse_of {
            // Symmetric: an equivalence or an inverse pair is non-local as soon as EITHER
            // side is in `Σ`, under both notions.
            let Some(object) = context.is_iri(object).then_some(object) else {
                return self.conservative(context, subject, object);
            };
            return if in_sigma(subject) || in_sigma(object) {
                Decision::Keep(vec![subject, object])
            } else {
                Decision::Drop
            };
        }
        if predicate == v.disjoint {
            // `C ⊓ D ⊑ ⊥` says nothing about `Σ` unless BOTH sides are in it: with one side
            // outside, interpreting that side as `⊥` satisfies the axiom vacuously.
            let Some(object) = context.is_iri(object).then_some(object) else {
                return self.conservative(context, subject, object);
            };
            return if in_sigma(subject) && in_sigma(object) {
                Decision::Keep(Vec::new())
            } else {
                Decision::Drop
            };
        }
        if predicate == v.domain || predicate == v.range {
            // `∃p.⊤ ⊑ C` / `⊤ ⊑ ∀p.C` — non-local when the PROPERTY is in `Σ`.
            let Some(object) = context.is_iri(object).then_some(object) else {
                return self.conservative(context, subject, object);
            };
            return if in_sigma(subject) {
                Decision::Keep(vec![object])
            } else {
                Decision::Drop
            };
        }
        if predicate == v.ty {
            let Some(object) = context.is_iri(object).then_some(object) else {
                return self.conservative(context, subject, object);
            };
            return if context.declarations.contains(&object) {
                // A declaration names an entity without constraining a model, so it is
                // relevant exactly when the entity is.
                if in_sigma(subject) {
                    Decision::Keep(Vec::new())
                } else {
                    Decision::Drop
                }
            } else if in_sigma(subject) || in_sigma(object) {
                Decision::Keep(vec![subject, object])
            } else {
                Decision::Drop
            };
        }

        // An annotation — a non-logical predicate over a literal — is relevant exactly when
        // its subject is.
        if context.is_literal(object) {
            return if in_sigma(subject) {
                Decision::Keep(Vec::new())
            } else {
                Decision::Drop
            };
        }
        self.conservative(context, subject, object)
    }

    /// The conservative test: keep whenever the triple names or reaches anything in `Σ`.
    ///
    /// The SOUND fallback. It never drops a construct that touches the signature, which is
    /// the whole of the doctrine; what it costs is minimality, and that cost is reported
    /// rather than absorbed.
    fn conservative(&self, context: &Context<'_>, subject: u32, object: u32) -> Decision {
        if self.sigma.contains(&subject) {
            return Decision::Conservative;
        }
        // BOTH endpoints are walked. A complex left-hand side is a blank node whose closure
        // is where its vocabulary lives, so testing the object alone would drop
        // `∃eats.⊤ ⊑ Predator` for a seed containing `eats`.
        let mut names = BTreeSet::new();
        context.reachable_names(subject, &mut names);
        context.reachable_names(object, &mut names);
        if names.iter().any(|name| self.sigma.contains(name)) {
            return Decision::Conservative;
        }
        Decision::Drop
    }
}
