// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reading a conclusion graph's NEGATIVE FACTS, and what asserting their negation is.
//!
//! # What a negative fact is, and why it needs its own reading
//!
//! Every head in OWL 2 Profiles §4.3's rule table is either an assertional triple over
//! named terms or `false`. Not one of the seventy-eight concludes that two individuals are
//! DIFFERENT, or that an individual is NOT in a class — so a conclusion of either shape is
//! unreachable by forward chaining however complete the table's coverage is, and matching it
//! into a closure will always miss. It follows only by REFUTATION, and a refutation needs to
//! know exactly which triples of the conclusion state the negative fact and exactly what
//! asserting its negation means. That reading is this module.
//!
//! Three shapes are read, and they are the three the W3C entailment corpus publishes:
//!
//! | conclusion | negative fact | negation asserted into the premise |
//! |---|---|---|
//! | `a owl:differentFrom b` | `a ≠ b` | `a owl:sameAs b` |
//! | `i rdf:type [ owl:complementOf C ]` | `¬C(i)` | `i rdf:type C` |
//! | `[ a owl:AllDifferent ; owl:members (m₁ … mₙ) ]` | `mᵢ ≠ mⱼ` for every pair | one `owl:sameAs` per pair |
//!
//! The third is not a fourth mechanism: OWL 2's `AllDifferent` axiom is defined to be the
//! conjunction of its `n(n−1)/2` pairwise inequalities, so it LOWERS to the first, and the
//! collection is entailed exactly when every pair refutes. Reading it any other way — as one
//! obligation, or as "enough" pairs — would be a different axiom.
//!
//! # Applicability is decided by WHITELIST, and an unknown DISQUALIFIES
//!
//! This module's reading has three outcomes and the difference between two of them is the
//! whole point. NOT-APPLICABLE says the conclusion states no negative fact at all, so a
//! refutation would have nothing to prove and the caller answers exactly what it would have
//! answered anyway. DECLINED says the conclusion states something this module RECOGNIZES and
//! cannot read — and that is an admission of incapacity, never a refutation, so it travels to
//! [`super::EntailmentOutcome::Undecided`] naming the construct.
//!
//! The direction is the one [`crate::combined`]'s module docs argue for at length, and the
//! argument transfers verbatim: a BLACKLIST of negative constructs could not support the
//! claim this lowering makes, because the claim is about EVERY statement of the conclusion.
//! A refutation that discharged the `owl:differentFrom` it recognized and quietly left an
//! `owl:NegativePropertyAssertion` beside it unread would report `Entailed` for a conclusion
//! half of which nothing had established.
//!
//! So a conclusion triple leaves this module in exactly one of two states, never a third:
//!
//! * **consumed** by a recognized negative fact, and discharged by that fact's refutation;
//! * **residual**, and discharged by mapping into the premise's closure like any other
//!   conclusion triple — [`super::homomorphism`]'s ordinary obligation, not a weaker one —
//!   or by another mechanism, which [`super::entails`] gets to try because the residual is
//!   THREADED through the remaining lanes rather than handed straight to the closure.
//!
//! and any of the five constructs `NEGATIVE_CONSTRUCTS` names that this module cannot read
//! in one of its three shapes disqualifies the whole conclusion. A construct nobody has
//! written yet is refused rather than dropped, which is the direction an unknown has to fall
//! for "every statement was accounted for" to mean anything.
//!
//! # Determinism
//!
//! Facts are emitted in the conclusion's own frozen triple order, and a collection's pairs
//! in member order, so two runs over one conclusion produce the same obligations in the same
//! sequence — which is what makes a warrant comparable against a re-lowering of the same
//! conclusion.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::{RdfDataset, TermValue};

use crate::engine::surface_of;
use crate::entails::graph::{Triple, default_graph_triples, show};
use crate::entails::pattern::{PatTriple, patterns_at};
use crate::vocab::{
    OWL_ALLDIFFERENT, OWL_CLASS, OWL_COMPLEMENTOF, OWL_DATATYPECOMPLEMENTOF, OWL_DIFFERENTFROM,
    OWL_DISTINCTMEMBERS, OWL_MEMBERS, OWL_NEGATIVEPROPERTYASSERTION, OWL_SAMEAS, RDF_FIRST,
    RDF_NIL, RDF_REST, RDF_TYPE, RDFS_CLASS,
};

/// The reserved terms whose presence in a conclusion means a NEGATIVE FACT.
///
/// The whitelist's other half: a conclusion mentioning any of these is one this module must
/// read completely or refuse completely, because each of them states something no rule of
/// the table concludes and which therefore cannot be discharged by matching. The first three
/// are the shapes [`lower`] reads; the last two are shapes it does NOT read
/// (`owl:NegativePropertyAssertion`, and the datatype complement of a data range), and they
/// are named here precisely so their presence disqualifies instead of falling through to a
/// residual match that would report "not entailed" about a conclusion nothing tested.
///
/// A reserved term that is not here — `owl:disjointWith`, `owl:AllDisjointClasses`,
/// `owl:IrreflexiveProperty`, `owl:Nothing` — states a SCHEMA axiom or an inconsistency
/// rather than a negative fact about individuals. Such a triple is residual: it is
/// discharged by mapping into the closure, which is the obligation it already had and which
/// this module neither weakens nor pretends to have improved.
pub(crate) const NEGATIVE_CONSTRUCTS: [&str; 5] = [
    OWL_DIFFERENTFROM,
    OWL_COMPLEMENTOF,
    OWL_ALLDIFFERENT,
    OWL_NEGATIVEPROPERTYASSERTION,
    OWL_DATATYPECOMPLEMENTOF,
];

/// A fact that follows from a premise only by REFUTATION.
///
/// Each variant carries the terms of the fact rather than the conclusion triples that state
/// it, so [`Self::negation`] can DERIVE what has to be asserted instead of storing it. That
/// is what lets [`verify`](super::verify) re-lower a conclusion and compare the facts a
/// warrant claims against the facts the conclusion actually states, rather than trusting a
/// triple list the warrant carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegativeFact {
    /// Two individuals are different — `a owl:differentFrom b`, or one pair of an
    /// `owl:AllDifferent` collection.
    Distinct {
        /// The left individual.
        left: TermValue,
        /// The right individual.
        right: TermValue,
    },
    /// An individual is NOT in a class — `i rdf:type [ owl:complementOf C ]`.
    NotAnInstanceOf {
        /// The individual.
        individual: TermValue,
        /// The class it is not an instance of. Named: a class EXPRESSION in this position
        /// is not a shape this module reads.
        class: TermValue,
    },
}

impl NegativeFact {
    /// The assertion whose inconsistency proves this fact.
    ///
    /// # Why exactly these two shapes, and why the shape matters beyond soundness
    ///
    /// `owl:differentFrom` is interpreted as inequality and `owl:sameAs` as equality, so
    /// `a owl:sameAs b` is the negation of `a owl:differentFrom b` and `i rdf:type C` is the
    /// negation of `i rdf:type [ owl:complementOf C ]`, under the RDF-Based semantics of the
    /// vocabulary and with no appeal to a closed world.
    ///
    /// Both shapes are also, deliberately, ORDINARY ASSERTIONAL TRIPLES over terms the
    /// premise's seed already knows how to hold: neither predicate is `rdf:first`,
    /// `rdf:rest` or one of the seven list-valued OWL predicates, and neither carries a
    /// literal. That is what entitles the engine's own `Refuter::refute` to
    /// reuse the premise's collection walk and value-space judgements instead of recomputing
    /// them — a performance property that is only sound because the negation cannot disturb
    /// either, which is asserted rather than assumed.
    #[must_use]
    pub fn negation(&self) -> Vec<Triple> {
        match self {
            Self::Distinct { left, right } => {
                vec![[left.clone(), TermValue::iri(OWL_SAMEAS), right.clone()]]
            }
            Self::NotAnInstanceOf { individual, class } => {
                vec![[individual.clone(), TermValue::iri(RDF_TYPE), class.clone()]]
            }
        }
    }
}

impl std::fmt::Display for NegativeFact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Distinct { left, right } => {
                write!(f, "{} owl:differentFrom {}", show(left), show(right))
            }
            Self::NotAnInstanceOf { individual, class } => write!(
                f,
                "{} rdf:type [ owl:complementOf {} ]",
                show(individual),
                show(class)
            ),
        }
    }
}

/// A conclusion graph split into what refutation discharges and what it leaves behind.
#[derive(Debug)]
pub(crate) struct Lowering {
    /// The indices — into the conclusion's own frozen triple order — that the recognized
    /// negative facts CONSUMED.
    ///
    /// An index set rather than a residual pattern list, because [`super::entails`] threads
    /// one residual through every mechanism in turn and can only subtract what each lane
    /// consumed if the lanes speak about the same triples by the same names.
    pub(crate) consumed: BTreeSet<usize>,
    /// The negative facts, in the conclusion's own triple order and then member order.
    pub(crate) facts: Vec<NegativeFact>,
}

impl Lowering {
    /// The conclusion triples no negative fact consumed, as match patterns.
    pub(crate) fn residual(&self, triples: &[Triple]) -> Vec<PatTriple> {
        let keep: BTreeSet<usize> = (0..triples.len())
            .filter(|index| !self.consumed.contains(index))
            .collect();
        patterns_at(triples, &keep)
    }
}

/// What this module made of a conclusion graph.
///
/// Three inhabitants, and the second exists because "I read nothing here" and "I read
/// something here and cannot handle it" carry opposite epistemic weight. See the
/// [module docs](self).
#[derive(Debug)]
pub(crate) enum Read {
    /// The conclusion states no negative fact this module reads, so a refutation would have
    /// nothing to prove.
    NotApplicable,
    /// The conclusion states something this module RECOGNIZES and declines to read, rendered
    /// one entry per refusal. An admission, never a refutation.
    Declined(Vec<String>),
    /// The conclusion lowers, and here is the split. Always carries at least one fact.
    Lowered(Lowering),
}

/// One conclusion graph, indexed the three ways the recognizers need to read it.
struct Indexed {
    /// Every default-graph triple, in the dataset's frozen quad order.
    triples: Vec<Triple>,
    /// Subject surface → the indices of the triples it is the subject of.
    by_subject: BTreeMap<String, Vec<usize>>,
    /// Term surface → the indices of the triples mentioning it in ANY position.
    mentions: BTreeMap<String, Vec<usize>>,
}

impl Indexed {
    /// Index `ds`'s default graph.
    fn of(ds: &RdfDataset) -> Self {
        let triples = default_graph_triples(ds);
        let mut by_subject: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut mentions: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, triple) in triples.iter().enumerate() {
            by_subject
                .entry(surface_of(&triple[0]))
                .or_default()
                .push(index);
            for position in triple {
                let slot = mentions.entry(surface_of(position)).or_default();
                if slot.last() != Some(&index) {
                    slot.push(index);
                }
            }
        }
        Self {
            triples,
            by_subject,
            mentions,
        }
    }

    /// The indices of the triples `term` is the subject of.
    fn subject_of(&self, term: &TermValue) -> &[usize] {
        self.by_subject
            .get(&surface_of(term))
            .map_or(&[][..], Vec::as_slice)
    }

    /// The indices of the triples mentioning `term` anywhere.
    fn mentioning(&self, term: &TermValue) -> &[usize] {
        self.mentions
            .get(&surface_of(term))
            .map_or(&[][..], Vec::as_slice)
    }
}

/// Whether `term` names an IRI.
fn is_named(term: &TermValue) -> bool {
    matches!(term, TermValue::Iri(_))
}

/// Whether `term` is the IRI `iri`.
fn is(term: &TermValue, iri: &str) -> bool {
    matches!(term, TermValue::Iri(value) if value == iri)
}

/// Split `conclusion` into the negative facts refutation must discharge and the triples it
/// leaves behind.
///
/// See [`Read`] for the three answers and why the middle one is not the first. A returned
/// [`Read::Lowered`] always carries at least one fact, so a caller can read it as "there is
/// something here only refutation can reach".
pub(crate) fn lower(conclusion: &RdfDataset) -> Read {
    let indexed = Indexed::of(conclusion);
    let mut consumed: BTreeSet<usize> = BTreeSet::new();
    let mut facts: Vec<NegativeFact> = Vec::new();
    let mut declined: Vec<String> = Vec::new();

    for index in 0..indexed.triples.len() {
        let [subject, predicate, object] = &indexed.triples[index];
        // The whitelist is walked from its own TABLE: a triple that carries none of the five
        // negative constructs in the two positions an axiom writes them is residual and this
        // loop is not about it. A triple that carries one must leave through one of the
        // three recognizers, and the `else` below is what turns an unread mention into a
        // disqualification instead of a residual match nobody would notice was a refutation
        // that never happened.
        if !NEGATIVE_CONSTRUCTS
            .iter()
            .any(|construct| is(predicate, construct) || is(object, construct))
        {
            continue;
        }
        let read = if is(predicate, OWL_DIFFERENTFROM) {
            // An existential inequality — "there is something different from b" — is not a
            // fact whose negation this module can assert: it would have to choose a witness.
            if is_named(subject) && is_named(object) {
                consumed.insert(index);
                facts.push(NegativeFact::Distinct {
                    left: subject.clone(),
                    right: object.clone(),
                });
                Ok(())
            } else {
                Err(
                    "an owl:differentFrom over an existential names no witness whose \
                     identity could be negated"
                        .to_owned(),
                )
            }
        } else if is(predicate, OWL_COMPLEMENTOF) {
            complement(&indexed, subject, &mut consumed, &mut facts)
        } else if is(predicate, RDF_TYPE) && is(object, OWL_ALLDIFFERENT) {
            all_different(&indexed, subject, &mut consumed, &mut facts)
        } else {
            Err(format!(
                "{} states a negative construct this lane does not read",
                show_triple(&indexed.triples[index])
            ))
        };
        if let Err(why) = read {
            declined.push(why);
        }
    }

    if !declined.is_empty() {
        declined.sort_unstable();
        declined.dedup();
        return Read::Declined(declined);
    }
    if facts.is_empty() {
        return Read::NotApplicable;
    }

    // THE WHITELIST'S CLOSING CHECK. Every triple is now either consumed or residual, and a
    // residual triple that still mentions a consumed blank node would mean a scaffold node
    // was read as an obligation in one place and as an existential to be matched in another
    // — two readings of one node, of which at most one can be right.
    let mut scaffold: BTreeSet<String> = BTreeSet::new();
    for &index in &consumed {
        for position in &indexed.triples[index] {
            if matches!(position, TermValue::Blank { .. }) {
                scaffold.insert(surface_of(position));
            }
        }
    }
    for (index, triple) in indexed.triples.iter().enumerate() {
        if consumed.contains(&index) {
            continue;
        }
        if triple
            .iter()
            .any(|position| scaffold.contains(&surface_of(position)))
        {
            return Read::Declined(vec![format!(
                "{} mentions a node a negative fact already consumed, so the node has two \
                 readings and at most one of them can be right",
                show_triple(triple)
            )]);
        }
    }
    Read::Lowered(Lowering { consumed, facts })
}

/// The lowering of `conclusion`, or `None` for either of the two refusals.
///
/// [`verify`](super::verify) and [`precondition`](super::precondition) both need the split
/// and neither needs the reason a refusal gave, so they read this rather than re-matching
/// [`Read`]'s three arms.
pub(crate) fn lowering(conclusion: &RdfDataset) -> Option<Lowering> {
    match lower(conclusion) {
        Read::Lowered(lowering) => Some(lowering),
        Read::NotApplicable | Read::Declined(_) => None,
    }
}

/// Render a triple the way a refusal names it.
fn show_triple(triple: &Triple) -> String {
    format!(
        "{} {} {}",
        show(&triple[0]),
        show(&triple[1]),
        show(&triple[2])
    )
}

/// Recognize the anonymous complement class `node`, or refuse the whole conclusion.
///
/// The shape, exactly: `node` is a BLANK node whose every triple is one `owl:complementOf`
/// naming a class of the caller's vocabulary, optionally beside an `owl:Class`/`rdfs:Class`
/// typing — and whose every OTHER mention is as the object of `x rdf:type node` for a named
/// `x`. Each of those typings is one obligation.
///
/// Everything else refuses, and each refusal is a real distinction rather than caution:
///
/// * a NAMED subject makes `C owl:complementOf D` a class AXIOM about the caller's own
///   vocabulary, which is a schema conclusion and not a negative fact about an individual;
/// * a class EXPRESSION filler would make the asserted negation `i rdf:type _:d` for a node
///   whose own axioms this module has not read, so the refutation would be over a premise
///   nobody checked;
/// * any other predicate on the node — a second `owl:complementOf`, an `owl:unionOf`, an
///   annotation — means the node denotes something other than "the complement of C", and
///   negating membership in it would be negating membership in a different class;
/// * a mention in any other position means the node is load-bearing somewhere this module
///   did not look;
/// * NO typing at all leaves the complement class asserted of nobody, so the triples would
///   be consumed and prove nothing — the silent drop the whitelist exists to prevent.
fn complement(
    indexed: &Indexed,
    node: &TermValue,
    consumed: &mut BTreeSet<usize>,
    facts: &mut Vec<NegativeFact>,
) -> Result<(), String> {
    let refuse = |why: &str| Err(format!("{} owl:complementOf …: {why}", show(node)));
    if !matches!(node, TermValue::Blank { .. }) {
        return refuse(
            "a NAMED complement is a class axiom about the caller's own vocabulary, not a \
             negative fact about an individual",
        );
    }
    let own = indexed.subject_of(node);
    let mut filler: Option<TermValue> = None;
    for &index in own {
        let [_, predicate, object] = &indexed.triples[index];
        if is(predicate, OWL_COMPLEMENTOF) {
            if filler.is_some() {
                return refuse("the node carries two complements, so it denotes neither");
            }
            if !is_named(object) {
                return refuse(
                    "the complement of a class EXPRESSION would negate membership in a class \
                     whose own axioms nothing here read",
                );
            }
            filler = Some(object.clone());
        } else if is(predicate, RDF_TYPE) {
            if !is(object, OWL_CLASS) && !is(object, RDFS_CLASS) {
                return refuse("the node carries a typing other than owl:Class or rdfs:Class");
            }
        } else {
            return refuse("the node carries a predicate that makes it denote something else");
        }
    }
    let Some(filler) = filler else {
        return refuse("the node states no complement at all");
    };

    let own_set: BTreeSet<usize> = own.iter().copied().collect();
    let mut instances = Vec::new();
    for &index in indexed.mentioning(node) {
        if own_set.contains(&index) {
            continue;
        }
        let [subject, predicate, object] = &indexed.triples[index];
        if !is(predicate, RDF_TYPE) || surface_of(object) != surface_of(node) || !is_named(subject)
        {
            return refuse("the node is mentioned somewhere this lane did not look");
        }
        instances.push((index, subject.clone()));
    }
    if instances.is_empty() {
        return refuse("the complement class is asserted of nobody, so it proves nothing");
    }

    consumed.extend(own_set);
    for (index, individual) in instances {
        consumed.insert(index);
        facts.push(NegativeFact::NotAnInstanceOf {
            individual,
            class: filler.clone(),
        });
    }
    Ok(())
}

/// Recognize the `owl:AllDifferent` collection `node` and lower it to its pairs, or refuse
/// the whole conclusion.
///
/// The shape, exactly: `node` is a BLANK node typed `owl:AllDifferent` exactly once,
/// carrying exactly one `owl:members` or `owl:distinctMembers` (OWL 2's spelling and OWL 1's
/// — the axiom is the same, and both appear in the W3C corpus) whose object is a
/// well-formed RDF collection of at least two NAMED individuals, and mentioned nowhere else.
///
/// It lowers to the `n(n−1)/2` pairwise inequalities OWL 2's `DifferentIndividuals` axiom is
/// DEFINED as, in member order. The collection is entailed exactly when every one of them
/// refutes; a caller that stopped at the first would be deciding a weaker axiom.
///
/// Fewer than two members refuses rather than succeeding vacuously. `AllDifferent()` and
/// `AllDifferent(a)` constrain nothing, so "entailed" would be the right ANSWER — but it
/// would be an answer this module reached without testing anything, and a mechanism that can
/// say yes without running is one whose yes carries no information.
fn all_different(
    indexed: &Indexed,
    node: &TermValue,
    consumed: &mut BTreeSet<usize>,
    facts: &mut Vec<NegativeFact>,
) -> Result<(), String> {
    let refuse = |why: &str| Err(format!("{} a owl:AllDifferent: {why}", show(node)));
    if !matches!(node, TermValue::Blank { .. }) {
        return refuse("a NAMED collection node is not a shape this lane reads");
    }
    let own = indexed.subject_of(node);
    let mut typed = 0_usize;
    let mut head: Option<TermValue> = None;
    for &index in own {
        let [_, predicate, object] = &indexed.triples[index];
        if is(predicate, RDF_TYPE) {
            if !is(object, OWL_ALLDIFFERENT) {
                return refuse("the node carries a typing other than owl:AllDifferent");
            }
            typed += 1;
        } else if is(predicate, OWL_MEMBERS) || is(predicate, OWL_DISTINCTMEMBERS) {
            if head.is_some() {
                return refuse("the node carries two member lists, so it states two axioms");
            }
            head = Some(object.clone());
        } else {
            return refuse("the node carries a predicate that makes it denote something else");
        }
    }
    if typed != 1 {
        return refuse("the node is typed owl:AllDifferent other than exactly once");
    }
    let own_set: BTreeSet<usize> = own.iter().copied().collect();
    // The node is the collection's only anchor: a mention anywhere else means something this
    // module did not read points at it.
    if indexed
        .mentioning(node)
        .iter()
        .any(|index| !own_set.contains(index))
    {
        return refuse("the node is mentioned somewhere this lane did not look");
    }

    let Some(head) = head else {
        return refuse("the node states no member list");
    };
    let mut cells: BTreeSet<usize> = BTreeSet::new();
    let members = walk(indexed, &head, node, &mut cells)?;
    if members.len() < 2 {
        return refuse(
            "a collection of fewer than two members constrains nothing, so answering it would \
             be saying yes without testing anything",
        );
    }

    consumed.extend(own_set);
    consumed.extend(cells);
    for (position, left) in members.iter().enumerate() {
        for right in &members[position + 1..] {
            facts.push(NegativeFact::Distinct {
                left: left.clone(),
                right: right.clone(),
            });
        }
    }
    Ok(())
}

/// Walk the RDF collection headed by `head`, collecting its members and its cells' triples.
///
/// A cell must be a BLANK node with exactly one `rdf:first`, exactly one `rdf:rest` and no
/// other triple, pointed at only by its predecessor, and every member must be a named
/// individual; the walk must reach `rdf:nil`. Anything else refuses — the same discipline
/// [`crate::lists`] applies inside the chase, for the same reason: reasoning over the
/// well-formed PREFIX of a broken collection answers a question the caller did not ask.
///
/// `previous` is the node that points at `head`, so the exclusivity check ("this cell is
/// reached from exactly one place") can be made without a second index.
fn walk(
    indexed: &Indexed,
    head: &TermValue,
    previous: &TermValue,
    cells: &mut BTreeSet<usize>,
) -> Result<Vec<TermValue>, String> {
    let mut members = Vec::new();
    let mut current = head.clone();
    let mut from = previous.clone();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    while !is(&current, RDF_NIL) {
        let refuse = |why: &str| Err(format!("the member list at {}: {why}", show(&current)));
        if !matches!(current, TermValue::Blank { .. }) {
            return refuse("a collection cell must be a blank node");
        }
        if !seen.insert(surface_of(&current)) {
            return refuse("the collection is cyclic");
        }
        let own = indexed.subject_of(&current);
        let own_set: BTreeSet<usize> = own.iter().copied().collect();
        let mut member: Option<TermValue> = None;
        let mut rest: Option<TermValue> = None;
        for &index in own {
            let [_, predicate, object] = &indexed.triples[index];
            if is(predicate, RDF_FIRST) {
                if member.is_some() {
                    return refuse("the cell carries two rdf:first values");
                }
                if !is_named(object) {
                    return refuse(
                        "a member that is not a named individual has no identity to \
                                   separate",
                    );
                }
                member = Some(object.clone());
            } else if is(predicate, RDF_REST) {
                if rest.is_some() {
                    return refuse("the cell carries two rdf:rest values");
                }
                rest = Some(object.clone());
            } else {
                return refuse("the cell carries a triple that is not part of a collection");
            }
        }
        // Reached from exactly one place, and that place is the predecessor: a cell two
        // collections share is a cell this walk cannot consume on either's behalf.
        for &index in indexed.mentioning(&current) {
            if own_set.contains(&index) {
                continue;
            }
            let [subject, _, object] = &indexed.triples[index];
            if surface_of(subject) != surface_of(&from)
                || surface_of(object) != surface_of(&current)
            {
                return refuse("the cell is reached from more than one place");
            }
        }
        let (Some(member), Some(rest)) = (member, rest) else {
            return refuse("the cell is missing its rdf:first or its rdf:rest");
        };
        cells.extend(own_set);
        members.push(member);
        from = current;
        current = rest;
    }
    Ok(members)
}

#[cfg(test)]
mod tests {
    use purrdf_core::{BlankScope, RdfDatasetBuilder};

    use super::{NegativeFact, Read, lower, lowering};
    use crate::vocab::{
        OWL_ALLDIFFERENT, OWL_CLASS, OWL_COMPLEMENTOF, OWL_DIFFERENTFROM, OWL_DISTINCTMEMBERS,
        OWL_MEMBERS, OWL_NEGATIVEPROPERTYASSERTION, OWL_SAMEAS, RDF_FIRST, RDF_NIL, RDF_REST,
        RDF_TYPE,
    };
    use purrdf_core::{RdfDataset, TermValue};
    use std::sync::Arc;

    /// `s`, `p`, `o` where a leading `_` names a blank node and anything else an IRI.
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

    const A: &str = "http://example.org/a";
    const B: &str = "http://example.org/b";
    const C: &str = "http://example.org/c";
    const K: &str = "http://example.org/K";

    /// `a owl:differentFrom b` lowers to one fact whose negation is `a owl:sameAs b`.
    #[test]
    fn a_different_from_lowers_to_one_pair() {
        let conclusion = graph(&[(A, OWL_DIFFERENTFROM, B)]);
        let lowered = lowering(&conclusion).expect("recognized");
        assert!(
            lowered
                .residual(&crate::entails::graph::default_graph_triples(&conclusion))
                .is_empty()
        );
        assert_eq!(
            lowered.facts,
            [NegativeFact::Distinct {
                left: TermValue::iri(A),
                right: TermValue::iri(B),
            }]
        );
        assert_eq!(
            lowered.facts[0].negation(),
            [[
                TermValue::iri(A),
                TermValue::iri(OWL_SAMEAS),
                TermValue::iri(B)
            ]]
        );
    }

    /// The anonymous complement class lowers, and the triples that state it are CONSUMED
    /// while the unrelated triple beside it stays residual.
    #[test]
    fn a_complement_typing_consumes_its_scaffold_and_leaves_the_rest() {
        let conclusion = graph(&[
            ("_c", RDF_TYPE, OWL_CLASS),
            ("_c", OWL_COMPLEMENTOF, K),
            (A, RDF_TYPE, "_c"),
            (K, RDF_TYPE, OWL_CLASS),
        ]);
        let lowered = lowering(&conclusion).expect("recognized");
        assert_eq!(
            lowered.facts,
            [NegativeFact::NotAnInstanceOf {
                individual: TermValue::iri(A),
                class: TermValue::iri(K),
            }]
        );
        assert_eq!(
            lowered
                .residual(&crate::entails::graph::default_graph_triples(&conclusion))
                .len(),
            1,
            "`K a owl:Class` still has to match"
        );
        assert_eq!(
            lowered.facts[0].negation(),
            [[
                TermValue::iri(A),
                TermValue::iri(RDF_TYPE),
                TermValue::iri(K)
            ]]
        );
    }

    /// A three-member `owl:AllDifferent` lowers to THREE pairs — `n(n−1)/2`, not `n`.
    #[test]
    fn an_all_different_collection_lowers_to_every_pair() {
        for members in [OWL_MEMBERS, OWL_DISTINCTMEMBERS] {
            let conclusion = graph(&[
                ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
                ("_x", members, "_l1"),
                ("_l1", RDF_FIRST, A),
                ("_l1", RDF_REST, "_l2"),
                ("_l2", RDF_FIRST, B),
                ("_l2", RDF_REST, "_l3"),
                ("_l3", RDF_FIRST, C),
                ("_l3", RDF_REST, RDF_NIL),
            ]);
            let lowered = lowering(&conclusion)
                .unwrap_or_else(|| panic!("{members} is a recognized spelling"));
            assert!(
                lowered
                    .residual(&crate::entails::graph::default_graph_triples(&conclusion))
                    .is_empty(),
                "the whole scaffold is consumed"
            );
            assert_eq!(
                lowered.facts,
                [
                    NegativeFact::Distinct {
                        left: TermValue::iri(A),
                        right: TermValue::iri(B)
                    },
                    NegativeFact::Distinct {
                        left: TermValue::iri(A),
                        right: TermValue::iri(C)
                    },
                    NegativeFact::Distinct {
                        left: TermValue::iri(B),
                        right: TermValue::iri(C)
                    },
                ]
            );
        }
    }

    /// A conclusion with nothing negative in it is NOT this module's business.
    #[test]
    fn an_ordinary_conclusion_is_not_applicable() {
        assert!(matches!(
            lower(&graph(&[(A, RDF_TYPE, K)])),
            Read::NotApplicable
        ));
    }

    // ── The whitelist: an unrecognized shape DISQUALIFIES ──────────────────────────────

    /// EVERY way the three shapes can be malformed refuses the whole conclusion — one case
    /// per refusal the recognizers make, driven through the real entry point.
    #[test]
    fn every_unrecognized_shape_disqualifies() {
        /// A refusal to drive: why it refuses, and the conclusion graph that refuses.
        type Case = (
            &'static str,
            Vec<(&'static str, &'static str, &'static str)>,
        );
        let cases: [Case; 12] = [
            (
                "a negative construct this module does not read",
                vec![("_n", RDF_TYPE, OWL_NEGATIVEPROPERTYASSERTION)],
            ),
            (
                "an existential inequality has no witness to negate",
                vec![("_b", OWL_DIFFERENTFROM, B)],
            ),
            (
                "a NAMED complement is a class axiom, not a negative fact",
                vec![(K, OWL_COMPLEMENTOF, C), (A, RDF_TYPE, K)],
            ),
            (
                "a complement of a class EXPRESSION is not read",
                vec![("_c", OWL_COMPLEMENTOF, "_d"), (A, RDF_TYPE, "_c")],
            ),
            (
                "a complement node carrying anything else denotes something else",
                vec![
                    ("_c", OWL_COMPLEMENTOF, K),
                    ("_c", OWL_MEMBERS, RDF_NIL),
                    (A, RDF_TYPE, "_c"),
                ],
            ),
            (
                "a complement class nobody is asserted to be in proves nothing",
                vec![("_c", RDF_TYPE, OWL_CLASS), ("_c", OWL_COMPLEMENTOF, K)],
            ),
            (
                "a complement node mentioned in another position is load-bearing elsewhere",
                vec![
                    ("_c", OWL_COMPLEMENTOF, K),
                    (A, RDF_TYPE, "_c"),
                    ("_c", RDF_FIRST, B),
                ],
            ),
            (
                "an AllDifferent with a second member list",
                vec![
                    ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
                    ("_x", OWL_MEMBERS, RDF_NIL),
                    ("_x", OWL_DISTINCTMEMBERS, RDF_NIL),
                ],
            ),
            (
                "an AllDifferent over fewer than two members constrains nothing",
                vec![
                    ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
                    ("_x", OWL_MEMBERS, "_l1"),
                    ("_l1", RDF_FIRST, A),
                    ("_l1", RDF_REST, RDF_NIL),
                ],
            ),
            (
                "an AllDifferent list that never reaches rdf:nil",
                vec![
                    ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
                    ("_x", OWL_MEMBERS, "_l1"),
                    ("_l1", RDF_FIRST, A),
                ],
            ),
            (
                "an AllDifferent cell carrying an extra triple",
                vec![
                    ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
                    ("_x", OWL_MEMBERS, "_l1"),
                    ("_l1", RDF_FIRST, A),
                    ("_l1", RDF_REST, "_l2"),
                    ("_l1", RDF_TYPE, K),
                    ("_l2", RDF_FIRST, B),
                    ("_l2", RDF_REST, RDF_NIL),
                ],
            ),
            (
                "an AllDifferent over a blank member",
                vec![
                    ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
                    ("_x", OWL_MEMBERS, "_l1"),
                    ("_l1", RDF_FIRST, "_m"),
                    ("_l1", RDF_REST, "_l2"),
                    ("_l2", RDF_FIRST, B),
                    ("_l2", RDF_REST, RDF_NIL),
                ],
            ),
        ];
        for (why, triples) in cases {
            let Read::Declined(reasons) = lower(&graph(&triples)) else {
                panic!("{why}: a recognized-and-declined shape is an ADMISSION, never a shrug");
            };
            assert!(!reasons.is_empty(), "{why}: the refusal names nothing");
        }
    }

    /// A cyclic member list refuses rather than looping — the cycle the chase's own
    /// collection walk refuses too.
    #[test]
    fn a_cyclic_member_list_refuses() {
        let Read::Declined(reasons) = lower(&graph(&[
            ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
            ("_x", OWL_MEMBERS, "_l1"),
            ("_l1", RDF_FIRST, A),
            ("_l1", RDF_REST, "_l2"),
            ("_l2", RDF_FIRST, B),
            ("_l2", RDF_REST, "_l1"),
        ])) else {
            panic!("a cycle is a shape this lane recognizes and declines");
        };
        assert!(
            reasons.iter().any(|why| why.contains("member list")),
            "{reasons:?}"
        );
    }

    /// The lowering is a function of the conclusion alone: two runs agree.
    #[test]
    fn the_lowering_is_deterministic() {
        let conclusion = graph(&[
            ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
            ("_x", OWL_MEMBERS, "_l1"),
            ("_l1", RDF_FIRST, A),
            ("_l1", RDF_REST, "_l2"),
            ("_l2", RDF_FIRST, B),
            ("_l2", RDF_REST, RDF_NIL),
        ]);
        let first = lowering(&conclusion).expect("recognized");
        let second = lowering(&conclusion).expect("recognized");
        assert_eq!(first.facts, second.facts);
    }

    /// Every fact renders something a human can act on.
    #[test]
    fn every_fact_renders() {
        for fact in [
            NegativeFact::Distinct {
                left: TermValue::iri(A),
                right: TermValue::iri(B),
            },
            NegativeFact::NotAnInstanceOf {
                individual: TermValue::iri(A),
                class: TermValue::iri(K),
            },
        ] {
            assert!(!fact.to_string().is_empty(), "{fact:?}");
        }
    }
}
