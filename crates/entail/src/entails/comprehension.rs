// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **comprehension** mechanism: mint the anonymous class expressions the conclusion
//! names, and only those.
//!
//! # The gap this closes
//!
//! W3C's `webont-i5-5-005` has the whole premise `a rdf:type owl:Class` and concludes
//!
//! ```text
//! _:c rdf:type owl:Class . _:c owl:unionOf _:l .
//! _:l rdf:type rdf:List . _:l rdf:first a . _:l rdf:rest rdf:nil .
//! ```
//!
//! and `webont-i5-26-010` concludes, from `p rdf:type owl:ObjectProperty`, an anonymous
//! `owl:Restriction` on `p` with `owl:minCardinality 1`. Neither conclusion says anything
//! about any individual. What each says is that a certain CLASS EXISTS — its blank nodes are
//! existentials — and the RDF-Based semantics says so too, in its **comprehension
//! conditions**: given a class `a ∈ IC`, the interpretation domain contains a class whose
//! extension is `unionOf(a)`, together with the list that describes it.
//!
//! No forward chase reaches this, and no rule could. The OWL 2 RL rule table's heads are
//! assertional triples over terms the premise already names; a comprehension condition
//! asserts the existence of a resource nothing names, and a rule set that produced one for
//! every licensed shape would produce infinitely many.
//!
//! # The typing SIDE CONDITION is the whole difficulty
//!
//! A comprehension condition is licensed, not free. RDF-Based comprehension licenses a class
//! for `unionOf(a)` only for `a ∈ IC` — and `i5-5-005`'s premise asserts `a rdf:type
//! owl:Class`, which is exactly WHY the case is a published entailment rather than a
//! published non-entailment. Minting unconditionally would derive conclusions W3C publishes
//! as NOT entailed.
//!
//! So every operand's membership is established through the shared `Membership` check, against
//! the premise's own closure. That lookup is itself an entailment check — the
//! [`homomorphism`](super::homomorphism) mechanism's test applied to a ground triple — rather
//! than a syntactic look at the premise's bytes, so a typing the chase DERIVED counts exactly
//! as one the premise asserted. `an_operand_that_is_not_a_class_is_not_comprehended` is the
//! falsifiable form.
//!
//! # A HORN theory entails a ground disjunction only by entailing a disjunct
//!
//! Membership in a comprehended union is the ground disjunction `C₁(x) ∨ … ∨ Cₙ(x)`, and a
//! disjunction is where a chase normally stops being enough: deciding one in general needs
//! case analysis, which a forward chase over definite rules cannot perform.
//!
//! It is enough here, and the reason is a property of the theory rather than a convenience.
//! The OWL 2 RL rule table is a set of DEFINITE Horn clauses (plus the seventeen whose head is
//! `false`, which are constraints), and a consistent Horn theory has a LEAST model — the one
//! the chase computes. For such a theory `T ⊨ D₁ ∨ … ∨ Dₙ` holds exactly when `T ⊨ Dᵢ` for
//! some `i`: if every disjunct failed in the least model, the least model would be a
//! countermodel of the disjunction. So looking for ONE disjunct in the closure is not a
//! weaker test that happens to work — the case split a disjunctive reasoner would perform
//! cannot reach anything this lookup misses. The premise's consistency, which
//! `super::prepare` established before this module runs, is what makes the least model exist
//! at all.
//!
//! Intersection needs no such argument: `C₁(x) ∧ … ∧ Cₙ(x)` is a conjunction, and every
//! conjunct is looked up.
//!
//! # What is minted, and what makes a minted node SAFE
//!
//! Only the scaffolds the conclusion itself names, with every scaffold-internal blank node
//! replaced by a witness the crate's checked fresh-symbol generator minted. The renaming
//! is not cosmetic. A premise and
//! a conclusion parse as SEPARATE datasets and their blank-node scopes are numbered
//! independently, so `_:l` of the conclusion and `_:l` of the premise can be the same
//! `(label, scope)` pair while denoting different nodes. Minting the conclusion's own labels
//! into the premise's closure would then attach `owl:unionOf` to a node the PREMISE
//! constrains — asserting something about the caller's data that nothing licenses. The
//! witnesses are checked absent from both documents before anything is minted, and
//! [`verify`](super::verify) re-decides that absence.
//!
//! `a_conclusion_naming_the_dl_fresh_prefix_does_not_alias` pins the specific way this can be
//! got wrong: the DL query layer's own counter emits `purrdfDLq{n}` with NO collision check,
//! and a conclusion is free to contain `_:purrdfDLq0`. This module uses the checked generator,
//! so such a conclusion is answered without its blank node ever being confused with a witness.
//!
//! # Applicability is a WHITELIST, and an unread scaffold DISQUALIFIES
//!
//! The constructors read are exactly `owl:unionOf`, `owl:intersectionOf` and `owl:Restriction`
//! over the six restriction constraints, all of them over NAMED operands. Anything else — a
//! nested anonymous operand, an `owl:oneOf`, a second constraint on one restriction, a
//! membership triple on a restriction (deciding one is a counting question this mechanism does
//! not answer) — refuses the WHOLE conclusion rather than leaving the offending triple to be
//! matched. The direction is [`super::negation`]'s and the argument transfers
//! verbatim: a mechanism that minted the scaffold it recognized and left an unread one beside
//! it would report `Entailed` for a conclusion half of which nothing established.
//!
//! # Determinism
//!
//! Scaffolds are read in the conclusion's own frozen triple order, operands in list order,
//! witnesses in mint order, and a union membership cites the FIRST disjunct in member order
//! that the closure holds. Two runs over one premise and one conclusion mint the same triples
//! and cite the same licences, on `wasm32` as on native.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::{RdfDataset, TermValue};
use purrdf_xsd::range::{DataRange, Known};
use purrdf_xsd::{XsdDatatype, parse};

use crate::engine::surface_of;
use crate::entails::fresh::{FreshBlanks, labels_of};
use crate::entails::graph::{Triple, default_graph_triples, show};
use crate::entails::homomorphism::{Binding, Closure};
use crate::entails::membership::Membership;
use crate::entails::warrant::{EntailmentMechanism, EntailmentWarrant, Replay};
use crate::entails::{Attempt, Established, Question, UndecidedReason};
use crate::vocab::{
    OWL_ALLVALUESFROM, OWL_CARDINALITY, OWL_CLASS, OWL_DISJOINTUNIONOF, OWL_HASSELF, OWL_HASVALUE,
    OWL_INTERSECTIONOF, OWL_MAXCARDINALITY, OWL_MAXQUALIFIEDCARDINALITY, OWL_MINCARDINALITY,
    OWL_MINQUALIFIEDCARDINALITY, OWL_ONDATARANGE, OWL_ONDATATYPE, OWL_ONEOF, OWL_ONPROPERTIES,
    OWL_ONPROPERTY, OWL_QUALIFIEDCARDINALITY, OWL_RESTRICTION, OWL_SOMEVALUESFROM, OWL_UNIONOF,
    OWL_WITHRESTRICTIONS, RDF_FIRST, RDF_LIST, RDF_NIL, RDF_REST, RDF_TYPE, RDFS_CLASS,
};
use crate::{EntailError, Regime};

// ── The evidence ───────────────────────────────────────────────────────────────────────

/// The evidence that a premise entails a conclusion whose anonymous class expressions were
/// comprehended.
///
/// Three parts, and each answers a different question:
///
/// * [`Self::minted`] is what was added to the premise's closure — the scaffold triples with
///   their blank nodes replaced by witnesses, plus whatever memberships were licensed;
/// * [`Self::licences`] is the closure triples that LICENSE those mints — the operands'
///   typings, and for a union membership the disjunct that holds;
/// * [`Self::binding`] is the ordinary homomorphism of the whole conclusion into
///   `closure ∪ minted`, so no part of the conclusion is discharged by assertion.
#[derive(Debug, Clone)]
pub struct ComprehensionWarrant {
    /// The regime the closure was computed under.
    regime: Regime,
    /// What each existential of the conclusion was bound to.
    binding: Binding,
    /// The premise's own closure, unextended.
    closure: Closure,
    /// Scaffold blank-node surface → the witness minted for it.
    witnesses: BTreeMap<String, TermValue>,
    /// The triples the comprehension conditions licensed, in reading order.
    minted: Vec<Triple>,
    /// The closure triples that license them, in reading order.
    licences: Vec<Triple>,
}

impl ComprehensionWarrant {
    /// The regime whose closure licensed the comprehension.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The mapping: what each existential of the conclusion was bound to.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        &self.binding
    }

    /// The triples the comprehension conditions licensed into existence.
    #[must_use]
    pub fn minted(&self) -> &[Triple] {
        &self.minted
    }

    /// The premise-closure triples that license [`Self::minted`].
    ///
    /// Every one of them is a ground entailment of the premise, so a reader who doubts the
    /// mint can check the licence without re-running anything.
    #[must_use]
    pub fn licences(&self) -> &[Triple] {
        &self.licences
    }

    /// How many distinct triples the PREMISE closure this warrant is against holds.
    ///
    /// The minted triples are not counted here: they are not conclusions of the chase and
    /// folding them into its size would misreport what the chase produced.
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

/// Which anonymous class expression a scaffold node denotes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Constructor {
    /// `owl:unionOf (C₁ … Cₙ)` over named classes.
    Union(Vec<TermValue>),
    /// `owl:intersectionOf (C₁ … Cₙ)` over named classes.
    Intersection(Vec<TermValue>),
    /// `owl:Restriction` on a named property, with exactly one constraint.
    Restriction {
        /// The property the restriction is on.
        property: TermValue,
        /// Which constraint, and its operand.
        constraint: Constraint,
    },
}

/// The one constraint an `owl:Restriction` scaffold carries.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Constraint {
    /// `owl:someValuesFrom C` / `owl:allValuesFrom C` over a NAMED class.
    Values {
        /// Which of the two predicates was written.
        predicate: &'static str,
        /// The named class filler.
        class: TermValue,
    },
    /// `owl:hasValue v` for a NAMED individual.
    HasValue(TermValue),
    /// `owl:minCardinality` / `owl:maxCardinality` / `owl:cardinality` over a literal whose
    /// value is a non-negative integer.
    Cardinality {
        /// Which of the three predicates was written.
        predicate: &'static str,
        /// The literal, verbatim.
        count: TermValue,
    },
}

/// One recognized anonymous class expression of the conclusion.
#[derive(Debug)]
struct Scaffold {
    /// The conclusion's blank node that denotes it.
    node: TermValue,
    /// Whether the conclusion also typed the node `owl:Class` / `rdfs:Class`, and as which.
    class_typings: Vec<TermValue>,
    /// What it denotes.
    constructor: Constructor,
    /// The conclusion's own collection cells, in walk order. Empty for a restriction.
    cells: Vec<TermValue>,
    /// The named individuals the conclusion asserts into it, in triple order.
    instances: Vec<TermValue>,
}

/// The conclusion split into the scaffolds this mechanism mints and everything else.
struct Reading {
    /// The recognized scaffolds, in the conclusion's own triple order.
    scaffolds: Vec<Scaffold>,
    /// Every blank node any scaffold consumed, by surface.
    consumed_nodes: Vec<TermValue>,
}

/// What this mechanism made of a conclusion.
///
/// The middle arm is the whole of Part Two's discipline here: a class constructor this lane
/// RECOGNIZES and cannot read is an admission of incapacity, and the conclusion's other
/// triples keep their ordinary obligation rather than being scored against a mint that never
/// happened.
enum Read {
    /// The conclusion names no anonymous class expression at all.
    NotApplicable,
    /// It names one this lane recognizes and declines to read, rendered one per refusal.
    Declined(Vec<String>),
    /// It names scaffolds this lane mints.
    Scaffolds(Reading),
}

/// The class constructors this lane RECOGNIZES on a blank node and does not read.
///
/// Named so their presence is an admission rather than a silence. Each states an anonymous
/// class or data range the RDF-Based comprehension conditions do license — `owl:oneOf` is a
/// nominal, `owl:disjointUnionOf` a union with a disjointness side condition, the qualified
/// cardinalities count over a class, `owl:hasSelf` is a self-restriction, and the datatype
/// facet vocabulary describes a derived data range — and none of the five this module reads
/// is any of them.
const UNREAD_CONSTRUCTORS: [&str; 10] = [
    OWL_ONEOF,
    OWL_DISJOINTUNIONOF,
    OWL_HASSELF,
    OWL_MINQUALIFIEDCARDINALITY,
    OWL_MAXQUALIFIEDCARDINALITY,
    OWL_QUALIFIEDCARDINALITY,
    OWL_ONDATARANGE,
    OWL_ONPROPERTIES,
    OWL_ONDATATYPE,
    OWL_WITHRESTRICTIONS,
];

/// Whether `term` is the IRI `iri`.
fn is(term: &TermValue, iri: &str) -> bool {
    matches!(term, TermValue::Iri(value) if value == iri)
}

/// Whether `term` names an IRI.
fn is_named(term: &TermValue) -> bool {
    matches!(term, TermValue::Iri(_))
}

/// Whether `count` is a literal whose value lies in the `xsd:nonNegativeInteger` value space.
///
/// The comprehension condition for a cardinality restriction quantifies over the non-negative
/// integers, so a literal outside that value space licenses nothing. The W3C corpus writes
/// `"1"^^xsd:int`, which is in it — the check is over VALUES rather than over the datatype
/// IRI, because `xsd:int` and `xsd:nonNegativeInteger` are two names selecting overlapping
/// parts of one value space.
fn is_non_negative_integer(count: &TermValue) -> bool {
    let TermValue::Literal {
        lexical_form,
        datatype,
        language,
        ..
    } = count
    else {
        return false;
    };
    if language.is_some() {
        return false;
    }
    let Some(kind) = XsdDatatype::from_iri(datatype) else {
        return false;
    };
    let Ok(value) = parse(lexical_form, kind) else {
        return false;
    };
    purrdf_xsd::range::contains(
        &DataRange::Datatype(XsdDatatype::NonNegativeInteger),
        &value,
    ) == Known::Yes
}

/// One conclusion graph, indexed the two ways the recognizer reads it.
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

/// Split `conclusion` into the anonymous class expressions this mechanism mints and the rest.
///
/// `None` is "not applicable": either the conclusion names no anonymous class expression (so
/// there is nothing to comprehend) or it names one this module does not fully read (so
/// proceeding would leave a statement of the conclusion unaccounted for).
fn read(conclusion: &RdfDataset) -> Read {
    let indexed = Indexed::of(conclusion);
    let mut scaffolds: Vec<Scaffold> = Vec::new();
    let mut consumed: BTreeSet<usize> = BTreeSet::new();
    let mut consumed_nodes: Vec<TermValue> = Vec::new();
    let mut declined: Vec<String> = Vec::new();

    for index in 0..indexed.triples.len() {
        if consumed.contains(&index) {
            continue;
        }
        let [subject, predicate, object] = &indexed.triples[index];
        // A class constructor this lane names and does not read is an ADMISSION, not a
        // residual: the conclusion asserts that a class of that description exists, and
        // nothing here tests it.
        if matches!(subject, TermValue::Blank { .. })
            && UNREAD_CONSTRUCTORS
                .iter()
                .any(|construct| is(predicate, construct))
        {
            declined.push(format!(
                "{} {} {}: a class constructor this lane does not read",
                show(subject),
                show(predicate),
                show(object)
            ));
            continue;
        }
        // A scaffold node is a BLANK node that heads one of the three read constructors.
        // Everything else on this pass is somebody else's triple.
        let is_restriction_typing = is(predicate, RDF_TYPE) && is(object, OWL_RESTRICTION);
        let heads_scaffold = is(predicate, OWL_UNIONOF)
            || is(predicate, OWL_INTERSECTIONOF)
            || is_restriction_typing;
        if !heads_scaffold {
            continue;
        }
        if !matches!(subject, TermValue::Blank { .. }) {
            // A NAMED anonymous-class scaffold is a class AXIOM about the caller's own
            // vocabulary — `C owl:unionOf (…)` says what `C` IS — which comprehension does
            // not license and this module does not read.
            declined.push(format!(
                "{} {} {}: a NAMED class expression is an axiom about the caller's own \
                 vocabulary, which no comprehension condition licenses",
                show(subject),
                show(predicate),
                show(object)
            ));
            continue;
        }
        let node = subject.clone();
        match recognize(&indexed, &node, &mut consumed, &mut consumed_nodes) {
            Ok(scaffold) => scaffolds.push(scaffold),
            Err(why) => declined.push(why),
        }
    }

    if !declined.is_empty() {
        declined.sort_unstable();
        declined.dedup();
        return Read::Declined(declined);
    }
    if scaffolds.is_empty() {
        return Read::NotApplicable;
    }

    // THE CLOSING CHECK. Every triple mentioning a consumed blank node must itself have been
    // consumed: a scaffold node read as an obligation in one place and as an existential to
    // be matched in another is two readings of one node, of which at most one can be right.
    let scaffold_surfaces: BTreeSet<String> = consumed_nodes.iter().map(surface_of).collect();
    for (index, triple) in indexed.triples.iter().enumerate() {
        if consumed.contains(&index) {
            continue;
        }
        if triple
            .iter()
            .any(|position| scaffold_surfaces.contains(&surface_of(position)))
        {
            return Read::Declined(vec![format!(
                "{} {} {}: it mentions a scaffold node this lane already consumed, so the node \
                 has two readings and at most one of them can be right",
                show(&triple[0]),
                show(&triple[1]),
                show(&triple[2])
            )]);
        }
    }

    Read::Scaffolds(Reading {
        scaffolds,
        consumed_nodes,
    })
}

/// Recognize the anonymous class expression `node`, or refuse the whole conclusion.
fn recognize(
    indexed: &Indexed,
    node: &TermValue,
    consumed: &mut BTreeSet<usize>,
    consumed_nodes: &mut Vec<TermValue>,
) -> Result<Scaffold, String> {
    let refuse = |why: &str| Err(format!("the class expression at {}: {why}", show(node)));
    let own = indexed.subject_of(node);
    let mut class_typings: Vec<TermValue> = Vec::new();
    let mut restriction_typed = false;
    let mut operands: Option<(&'static str, TermValue)> = None;
    let mut property: Option<TermValue> = None;
    let mut constraint: Option<Constraint> = None;

    for &index in own {
        let [_, predicate, object] = &indexed.triples[index];
        if is(predicate, RDF_TYPE) {
            if is(object, OWL_CLASS) || is(object, RDFS_CLASS) {
                class_typings.push(object.clone());
            } else if is(object, OWL_RESTRICTION) {
                restriction_typed = true;
            } else {
                return refuse("it carries a typing this lane does not read");
            }
        } else if is(predicate, OWL_UNIONOF) || is(predicate, OWL_INTERSECTIONOF) {
            if operands.is_some() {
                return refuse("it carries two constructors, so it denotes neither");
            }
            let which = if is(predicate, OWL_UNIONOF) {
                OWL_UNIONOF
            } else {
                OWL_INTERSECTIONOF
            };
            operands = Some((which, object.clone()));
        } else if is(predicate, OWL_ONPROPERTY) {
            if property.is_some() {
                return refuse("it restricts two properties at once");
            }
            if !is_named(object) {
                return refuse("it restricts a property expression rather than a named property");
            }
            property = Some(object.clone());
        } else {
            let Some(read) = constraint_of(predicate, object) else {
                return refuse("it carries a constraint this lane does not read");
            };
            if constraint.is_some() {
                return refuse("it carries two constraints, so it denotes neither");
            }
            constraint = Some(read);
        }
    }

    let own_set: BTreeSet<usize> = own.iter().copied().collect();
    let mut cells: BTreeSet<usize> = BTreeSet::new();
    let mut cell_nodes: Vec<TermValue> = Vec::new();
    let constructor = match (operands, property, constraint, restriction_typed) {
        (Some((which, head)), None, None, false) => {
            let members = walk(indexed, &head, node, &mut cells, &mut cell_nodes)?;
            if members.is_empty() {
                // The empty union is `owl:Nothing` and the empty intersection is
                // `owl:Thing`; both are NAMED classes of the vocabulary, so a conclusion
                // stating one anonymously is asking a different question.
                return refuse(
                    "an empty collection makes it owl:Nothing or owl:Thing, both of which are \
                     named classes and neither of which is an anonymous class expression",
                );
            }
            if which == OWL_UNIONOF {
                Constructor::Union(members)
            } else {
                Constructor::Intersection(members)
            }
        }
        (None, Some(property), Some(constraint), true) => Constructor::Restriction {
            property,
            constraint,
        },
        _ => {
            return refuse(
                "it mixes a collection constructor with a restriction, or states a restriction \
                 missing its property or its constraint",
            );
        }
    };

    // Every OTHER mention of the node must be a membership assertion `x rdf:type node` for a
    // NAMED `x`. A mention anywhere else means the node is load-bearing somewhere this module
    // did not look.
    let mut instances = Vec::new();
    let mut membership_indices = Vec::new();
    for &index in indexed.mentioning(node) {
        if own_set.contains(&index) {
            continue;
        }
        let [subject, predicate, object] = &indexed.triples[index];
        if !is(predicate, RDF_TYPE) || surface_of(object) != surface_of(node) || !is_named(subject)
        {
            return refuse("it is mentioned somewhere this lane did not look");
        }
        // A membership in a RESTRICTION is a counting or witness question — "does `x` have a
        // `p`-successor?" — which this mechanism does not answer. Refused rather than minted.
        if matches!(constructor, Constructor::Restriction { .. }) {
            return refuse(
                "a membership in a restriction is a counting or witness question, which \
                 minting the restriction does not answer",
            );
        }
        instances.push(subject.clone());
        membership_indices.push(index);
    }

    consumed.extend(own_set);
    consumed.extend(cells);
    consumed.extend(membership_indices);
    consumed_nodes.push(node.clone());
    consumed_nodes.extend(cell_nodes.iter().cloned());
    Ok(Scaffold {
        node: node.clone(),
        class_typings,
        constructor,
        cells: cell_nodes,
        instances,
    })
}

/// The restriction constraint `predicate object` states, if it is one of the six read.
fn constraint_of(predicate: &TermValue, object: &TermValue) -> Option<Constraint> {
    for which in [OWL_SOMEVALUESFROM, OWL_ALLVALUESFROM] {
        if is(predicate, which) {
            return is_named(object).then(|| Constraint::Values {
                predicate: which,
                class: object.clone(),
            });
        }
    }
    if is(predicate, OWL_HASVALUE) {
        return is_named(object).then(|| Constraint::HasValue(object.clone()));
    }
    for which in [OWL_MINCARDINALITY, OWL_MAXCARDINALITY, OWL_CARDINALITY] {
        if is(predicate, which) {
            return is_non_negative_integer(object).then(|| Constraint::Cardinality {
                predicate: which,
                count: object.clone(),
            });
        }
    }
    None
}

/// Walk the RDF collection headed by `head`, collecting its NAMED members and its cells.
///
/// A cell must be a BLANK node with exactly one `rdf:first`, exactly one `rdf:rest` and at
/// most an `rdf:type rdf:List` typing beside them, pointed at only by its predecessor; every
/// member must be a named class; and the walk must reach `rdf:nil`. Anything else refuses —
/// the discipline [`crate::lists`] applies inside the chase, for the same reason.
///
/// The `rdf:type rdf:List` typing is ADMITTED rather than merely tolerated: the list
/// comprehension condition puts every comprehended list in `ICEXT(rdf:List)`, so a conclusion
/// stating it is stating something the same condition licenses, and W3C's `i5-5-005` does.
fn walk(
    indexed: &Indexed,
    head: &TermValue,
    previous: &TermValue,
    cells: &mut BTreeSet<usize>,
    cell_nodes: &mut Vec<TermValue>,
) -> Result<Vec<TermValue>, String> {
    let mut members = Vec::new();
    let mut current = head.clone();
    let mut from = previous.clone();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    while !is(&current, RDF_NIL) {
        let refuse = |why: &str| Err(format!("the operand list at {}: {why}", show(&current)));
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
                        "a NESTED anonymous operand is a class expression whose own axioms \
                         this lane has not read",
                    );
                }
                member = Some(object.clone());
            } else if is(predicate, RDF_REST) {
                if rest.is_some() {
                    return refuse("the cell carries two rdf:rest values");
                }
                rest = Some(object.clone());
            } else if !(is(predicate, RDF_TYPE) && is(object, RDF_LIST)) {
                return refuse("the cell carries a triple that is not part of a collection");
            }
        }
        // Reached from exactly one place, and that place is the predecessor.
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
        cell_nodes.push(current.clone());
        members.push(member);
        from = current;
        current = rest;
    }
    Ok(members)
}

// ── Minting ────────────────────────────────────────────────────────────────────────────

/// What one reading licensed, as triples to add and triples that license them.
struct Licensed {
    /// The triples the comprehension conditions license.
    minted: Vec<Triple>,
    /// The closure triples that license them.
    licences: Vec<Triple>,
}

/// The witness for a scaffold-internal term: a minted blank node, or the term itself.
fn witness(term: &TermValue, witnesses: &BTreeMap<String, TermValue>) -> TermValue {
    witnesses
        .get(&surface_of(term))
        .cloned()
        .unwrap_or_else(|| term.clone())
}

/// Mint `reading`'s scaffolds under `witnesses`, or `None` if the closure licenses nothing.
///
/// A pure function of the reading, the witness map and the closure — which is what lets
/// [`verify`](super::verify) recompute it and compare rather than trust the warrant's list.
fn license(
    reading: &Reading,
    witnesses: &BTreeMap<String, TermValue>,
    closure: &Closure,
) -> Option<Licensed> {
    let mut minted = Vec::new();
    let mut licences = Vec::new();
    for scaffold in &reading.scaffolds {
        let node = witness(&scaffold.node, witnesses);
        for typing in &scaffold.class_typings {
            minted.push([node.clone(), TermValue::iri(RDF_TYPE), typing.clone()]);
        }
        match &scaffold.constructor {
            Constructor::Union(members) | Constructor::Intersection(members) => {
                let predicate = if matches!(scaffold.constructor, Constructor::Union(_)) {
                    OWL_UNIONOF
                } else {
                    OWL_INTERSECTIONOF
                };
                // THE SIDE CONDITION: comprehension licenses the class only over operands
                // that are already classes.
                for member in members {
                    licences.push(Membership::Class.establish(closure, member)?);
                }
                minted.extend(list_triples(
                    &node,
                    predicate,
                    members,
                    &scaffold.cells,
                    witnesses,
                ));
                for individual in &scaffold.instances {
                    licences.extend(membership_licences(
                        &scaffold.constructor,
                        members,
                        individual,
                        closure,
                    )?);
                    minted.push([individual.clone(), TermValue::iri(RDF_TYPE), node.clone()]);
                }
            }
            Constructor::Restriction {
                property,
                constraint,
            } => {
                // A restriction is comprehended over a PROPERTY; `IOOP` and `IODP` are both
                // inside `IP`, so either declaration establishes it.
                licences.push(Membership::Property.establish(closure, property)?);
                minted.push([
                    node.clone(),
                    TermValue::iri(RDF_TYPE),
                    TermValue::iri(OWL_RESTRICTION),
                ]);
                minted.push([
                    node.clone(),
                    TermValue::iri(OWL_ONPROPERTY),
                    property.clone(),
                ]);
                match constraint {
                    Constraint::Values { predicate, class } => {
                        licences.push(Membership::Class.establish(closure, class)?);
                        minted.push([node.clone(), TermValue::iri(*predicate), class.clone()]);
                    }
                    // Every IRI denotes a resource, so `owl:hasValue` over a named
                    // individual carries no further condition to establish.
                    Constraint::HasValue(value) => {
                        minted.push([node.clone(), TermValue::iri(OWL_HASVALUE), value.clone()]);
                    }
                    // The count's membership of the non-negative integers was decided by
                    // VALUE when the constraint was read, and no closure triple states it.
                    Constraint::Cardinality { predicate, count } => {
                        minted.push([node.clone(), TermValue::iri(*predicate), count.clone()]);
                    }
                }
            }
        }
    }
    Some(Licensed { minted, licences })
}

/// The closure triples that license `individual`'s membership in `constructor`, if any do.
fn membership_licences(
    constructor: &Constructor,
    members: &[TermValue],
    individual: &TermValue,
    closure: &Closure,
) -> Option<Vec<Triple>> {
    let typed = |class: &TermValue| {
        let triple = [individual.clone(), TermValue::iri(RDF_TYPE), class.clone()];
        closure.contains(&triple).then_some(triple)
    };
    match constructor {
        // The ground disjunction: ONE disjunct is enough, and over a Horn theory it is also
        // necessary. The first in member order, so the citation is reproducible.
        Constructor::Union(_) => members.iter().find_map(typed).map(|triple| vec![triple]),
        // The conjunction: every conjunct is owed, and every one of them is cited.
        Constructor::Intersection(_) => members.iter().map(typed).collect(),
        // Unreachable: a membership triple on a restriction refuses the whole conclusion at
        // reading time. Written out rather than defaulted so a constructor added to the
        // whitelist has to decide whether it can decide a membership.
        Constructor::Restriction { .. } => None,
    }
}

/// The `predicate` triple plus the collection cells that carry `members`.
///
/// The cells are the CONCLUSION's own, renamed through `witnesses`, so the minted collection
/// is the conclusion's collection with its nodes replaced and nothing else. Every cell is
/// also typed `rdf:List`, which the list comprehension condition licenses whether or not the
/// conclusion happened to state it.
fn list_triples(
    node: &TermValue,
    predicate: &'static str,
    members: &[TermValue],
    cells: &[TermValue],
    witnesses: &BTreeMap<String, TermValue>,
) -> Vec<Triple> {
    let cell_at = |index: usize| {
        cells
            .get(index)
            .map_or_else(|| TermValue::iri(RDF_NIL), |cell| witness(cell, witnesses))
    };
    let mut out = vec![[node.clone(), TermValue::iri(predicate), cell_at(0)]];
    for (index, member) in members.iter().enumerate() {
        let cell = cell_at(index);
        out.push([
            cell.clone(),
            TermValue::iri(RDF_TYPE),
            TermValue::iri(RDF_LIST),
        ]);
        out.push([cell.clone(), TermValue::iri(RDF_FIRST), member.clone()]);
        out.push([cell, TermValue::iri(RDF_REST), cell_at(index + 1)]);
    }
    out
}

// ── The mechanism ──────────────────────────────────────────────────────────────────────

/// Try to establish `conclusion` from `premise` by comprehending the class expressions it
/// names.
///
/// # Errors
///
/// [`EntailError::MatchBudget`] if the final match exhausts its budget.
pub(crate) fn attempt(q: &Question<'_>) -> Result<Attempt, EntailError> {
    let Question {
        premise,
        conclusion,
        regime,
        closure,
        ..
    } = *q;
    // WHITELIST, not blacklist. The comprehension conditions this mechanism applies are the
    // RDF-Based semantics', and `OWL-RL` is the only lane whose closure this crate reads
    // against them; the four others fall out.
    if !matches!(regime, Regime::OwlRl) {
        return Ok(Attempt::NotApplicable);
    }
    let reading = match read(conclusion) {
        Read::Scaffolds(reading) => reading,
        Read::NotApplicable => return Ok(Attempt::NotApplicable),
        Read::Declined(constructs) => {
            return Ok(Attempt::Disqualified(UndecidedReason::ConstructNotRead {
                lane: EntailmentMechanism::Comprehension,
                constructs,
            }));
        }
    };

    let witnesses = mint_witnesses(&reading, premise, conclusion);
    let Some(licensed) = license(&reading, &witnesses, closure) else {
        return Ok(Attempt::NotEstablished);
    };

    // This lane DISCHARGES nothing: it widens the closure with triples the comprehension
    // conditions license, and the conclusion's own scaffold triples keep their full obligation
    // to map onto them. `entails` runs that match once, at the end, over what survives every
    // lane.
    Ok(Attempt::Entailed(Box::new(Established {
        warrant: EntailmentWarrant::Comprehension(ComprehensionWarrant {
            regime,
            binding: Binding::new(),
            closure: closure.clone(),
            witnesses,
            minted: licensed.minted.clone(),
            licences: licensed.licences,
        }),
        discharged: BTreeSet::new(),
        minted: licensed.minted,
    })))
}

/// One witness per scaffold-internal blank node, none of them naming anything either document
/// names.
fn mint_witnesses(
    reading: &Reading,
    premise: &RdfDataset,
    conclusion: &RdfDataset,
) -> BTreeMap<String, TermValue> {
    let mut fresh = FreshBlanks::avoiding(&[premise, conclusion]);
    let mut witnesses = BTreeMap::new();
    for node in &reading.consumed_nodes {
        witnesses
            .entry(surface_of(node))
            .or_insert_with(|| fresh.mint());
    }
    witnesses
}

/// Re-decide a comprehension warrant against the caller's own premise and conclusion.
///
/// Called by [`verify`](super::verify), which owns the doc comment a caller reads. It runs no
/// reasoner: the conclusion is READ again on the spot, the mint is RECOMPUTED from the
/// warrant's witness map and compared, every witness is re-checked absent from both
/// documents, every licence is re-looked-up in the closure, and the binding is replayed.
pub(crate) fn verify_comprehension(
    w: &ComprehensionWarrant,
    premise: &RdfDataset,
    conclusion: &RdfDataset,
    _triples: &[Triple],
    _pending: &BTreeSet<usize>,
) -> Option<Replay> {
    let Read::Scaffolds(reading) = read(conclusion) else {
        return None;
    };
    // THE WITNESSES ARE FRESH, decided against the caller's own documents rather than taken
    // on the generator's word. A witness that named a node of either document would let the
    // mint attach an axiom to data nobody stated it about.
    let mut forbidden = labels_of(premise);
    forbidden.extend(labels_of(conclusion));
    let mut minted_labels: BTreeSet<&str> = BTreeSet::new();
    for value in w.witnesses.values() {
        let TermValue::Blank { label, .. } = value else {
            return None;
        };
        if forbidden.contains(label) {
            return None;
        }
        minted_labels.insert(label.as_str());
    }
    // Every scaffold node the conclusion states must have a witness, and distinct nodes must
    // have distinct ones — a shared witness would merge two classes the conclusion keeps
    // apart.
    if !reading
        .consumed_nodes
        .iter()
        .all(|node| w.witnesses.contains_key(&surface_of(node)))
    {
        return None;
    }
    if minted_labels.len() != w.witnesses.values().collect::<BTreeSet<_>>().len() {
        return None;
    }

    let licensed = license(&reading, &w.witnesses, &w.closure)?;
    if licensed.minted != w.minted || licensed.licences != w.licences {
        return None;
    }
    if !w.licences.iter().all(|triple| w.closure.contains(triple)) {
        return None;
    }

    Some(Replay {
        discharged: BTreeSet::new(),
        minted: w.minted.clone(),
    })
}

impl std::fmt::Display for ComprehensionWarrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} comprehended triple{} licensed by {} closure triple{}",
            self.minted.len(),
            if self.minted.len() == 1 { "" } else { "s" },
            self.licences.len(),
            if self.licences.len() == 1 { "" } else { "s" },
        )?;
        for triple in &self.minted {
            write!(
                f,
                "\n  {} {} {}",
                show(&triple[0]),
                show(&triple[1]),
                show(&triple[2])
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};

    use super::{Read, is_non_negative_integer, read};
    use crate::entails::graph::default_graph_triples;
    use crate::entails::{EntailmentOutcome, EntailmentWarrant, ImportMap, entails, verify};
    use crate::vocab::{
        OWL_CLASS, OWL_HASVALUE, OWL_INTERSECTIONOF, OWL_MINCARDINALITY, OWL_OBJECTPROPERTY,
        OWL_ONEOF, OWL_ONPROPERTY, OWL_ONTOLOGY, OWL_RESTRICTION, OWL_SOMEVALUESFROM, OWL_UNIONOF,
        RDF_FIRST, RDF_LIST, RDF_NIL, RDF_REST, RDF_TYPE, XSD_INT, XSD_STRING,
    };
    use crate::{Materialization, Regime, RuleId, extensions, implemented, materialize, rules};

    const A: &str = "http://example.org/a";
    const B: &str = "http://example.org/b";
    const P: &str = "http://example.org/p";
    const X: &str = "http://example.org/x";

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

    /// W3C `webont-i5-5-005`'s conclusion: the anonymous class `unionOf(a)`.
    fn union_conclusion(cell_label: &str) -> Arc<RdfDataset> {
        graph(&[
            ("_c", RDF_TYPE, OWL_CLASS),
            ("_c", OWL_UNIONOF, cell_label),
            (cell_label, RDF_TYPE, RDF_LIST),
            (cell_label, RDF_FIRST, A),
            (A, RDF_TYPE, OWL_CLASS),
            (cell_label, RDF_REST, RDF_NIL),
        ])
    }

    /// W3C `webont-i5-26-010`'s conclusion: an anonymous `minCardinality 1` on `p`.
    fn restriction_conclusion() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let ontology = b.intern_blank("o", BlankScope::DEFAULT);
        let ty = b.intern_iri(RDF_TYPE);
        let ontology_class = b.intern_iri(OWL_ONTOLOGY);
        b.push_quad(ontology, ty, ontology_class, None);
        let node = b.intern_blank("n", BlankScope::DEFAULT);
        let restriction = b.intern_iri(OWL_RESTRICTION);
        b.push_quad(node, ty, restriction, None);
        let on_property = b.intern_iri(OWL_ONPROPERTY);
        let p = b.intern_iri(P);
        b.push_quad(node, on_property, p, None);
        let object_property = b.intern_iri(OWL_OBJECTPROPERTY);
        b.push_quad(p, ty, object_property, None);
        let min_cardinality = b.intern_iri(OWL_MINCARDINALITY);
        let one = crate::interner::intern_into(&mut b, &TermValue::typed_literal("1", XSD_INT));
        b.push_quad(node, min_cardinality, one, None);
        b.freeze().expect("freeze")
    }

    fn decide(premise: &RdfDataset, conclusion: &RdfDataset) -> EntailmentOutcome {
        entails(premise, conclusion, Regime::OwlRl, &ImportMap::new())
            .expect("a consistent premise")
            .into_parts()
            .0
    }

    // ── The mechanism reaches what the rule table cannot ───────────────────────────────

    /// AN ANONYMOUS UNION CLASS IS ENTAILED BY ITS OPERAND BEING A CLASS, and the warrant
    /// re-checks.
    #[test]
    fn an_anonymous_union_is_comprehended_and_the_warrant_verifies() {
        let premise = graph(&[(A, RDF_TYPE, OWL_CLASS)]);
        let conclusion = union_conclusion("_l");
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("comprehension licenses unionOf(a) for a class a");
        };
        let EntailmentWarrant::Comprehension(comprehended) = &warrant else {
            panic!("no rule of Tables 4-9 concludes an anonymous class");
        };
        assert_eq!(comprehended.regime(), Regime::OwlRl);
        assert_eq!(
            comprehended.licences(),
            [[
                TermValue::iri(A),
                TermValue::iri(RDF_TYPE),
                TermValue::iri(OWL_CLASS)
            ]],
            "the operand's own typing is the licence"
        );
        assert!(!comprehended.minted().is_empty());
        assert!(!comprehended.to_string().is_empty());
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// AN ANONYMOUS CARDINALITY RESTRICTION is comprehended over a declared property.
    #[test]
    fn an_anonymous_restriction_is_comprehended() {
        let premise = graph(&[
            ("_o", RDF_TYPE, OWL_ONTOLOGY),
            (P, RDF_TYPE, OWL_OBJECTPROPERTY),
        ]);
        let conclusion = restriction_conclusion();
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("comprehension licenses a minCardinality restriction on a property");
        };
        assert!(matches!(&warrant, EntailmentWarrant::Comprehension(_)));
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// MEMBERSHIP IN A COMPREHENDED UNION IS THE GROUND DISJUNCTION, and one disjunct in the
    /// closure decides it — the Horn property, exercised.
    #[test]
    fn one_disjunct_decides_membership_in_a_comprehended_union() {
        let premise = graph(&[
            (A, RDF_TYPE, OWL_CLASS),
            (B, RDF_TYPE, OWL_CLASS),
            (X, RDF_TYPE, B),
        ]);
        let conclusion = graph(&[
            ("_c", OWL_UNIONOF, "_l1"),
            ("_l1", RDF_FIRST, A),
            ("_l1", RDF_REST, "_l2"),
            ("_l2", RDF_FIRST, B),
            ("_l2", RDF_REST, RDF_NIL),
            (X, RDF_TYPE, "_c"),
        ]);
        assert!(matches!(
            decide(&premise, &conclusion),
            EntailmentOutcome::Entailed(_)
        ));
        // …and an individual in NEITHER disjunct is not a member.
        let conclusion = graph(&[
            ("_c", OWL_UNIONOF, "_l1"),
            ("_l1", RDF_FIRST, A),
            ("_l1", RDF_REST, RDF_NIL),
            (X, RDF_TYPE, "_c"),
        ]);
        assert!(!matches!(
            decide(&premise, &conclusion),
            EntailmentOutcome::Entailed(_)
        ));
    }

    /// An INTERSECTION needs every conjunct, not one.
    #[test]
    fn every_conjunct_decides_membership_in_a_comprehended_intersection() {
        let both = |types: &[&str]| {
            let mut triples = vec![(A, RDF_TYPE, OWL_CLASS), (B, RDF_TYPE, OWL_CLASS)];
            for class in types {
                triples.push((X, RDF_TYPE, class));
            }
            graph(&triples)
        };
        let conclusion = graph(&[
            ("_c", OWL_INTERSECTIONOF, "_l1"),
            ("_l1", RDF_FIRST, A),
            ("_l1", RDF_REST, "_l2"),
            ("_l2", RDF_FIRST, B),
            ("_l2", RDF_REST, RDF_NIL),
            (X, RDF_TYPE, "_c"),
        ]);
        assert!(matches!(
            decide(&both(&[A, B]), &conclusion),
            EntailmentOutcome::Entailed(_)
        ));
        assert!(!matches!(
            decide(&both(&[A]), &conclusion),
            EntailmentOutcome::Entailed(_)
        ));
    }

    // ── ADVERSARIAL: the side condition and the witness ────────────────────────────────

    /// THE TYPING SIDE CONDITION IS THE WHOLE DIFFICULTY. An operand nothing types as a class
    /// is not comprehended — minting unconditionally would derive published non-entailments.
    #[test]
    fn an_operand_that_is_not_a_class_is_not_comprehended() {
        let premise = graph(&[(A, RDF_TYPE, B)]);
        assert!(
            !matches!(
                decide(&premise, &union_conclusion("_l")),
                EntailmentOutcome::Entailed(_)
            ),
            "RDF-Based comprehension licenses unionOf(a) only for a in IC"
        );
    }

    /// A CONCLUSION NAMING THE DL LAYER'S FRESH PREFIX MUST NOT ALIAS A WITNESS.
    ///
    /// `owl_dl::query`'s counter emits `purrdfDLq{n}` with no collision check; this module
    /// uses the CHECKED generator, so a conclusion containing `_:purrdfDLq0` is answered with
    /// that node still its own.
    #[test]
    fn a_conclusion_naming_the_dl_fresh_prefix_does_not_alias() {
        let premise = graph(&[(A, RDF_TYPE, OWL_CLASS)]);
        let conclusion = union_conclusion("_purrdfDLq0");
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("the conclusion is entailed whatever its blank nodes are called");
        };
        let EntailmentWarrant::Comprehension(comprehended) = &warrant else {
            panic!("comprehended");
        };
        for triple in comprehended.minted() {
            for term in triple {
                if let TermValue::Blank { label, .. } = term {
                    assert_ne!(
                        label, "purrdfDLq0",
                        "a witness aliased the conclusion's node"
                    );
                }
            }
        }
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// …and the same holds when the conclusion names THIS module's own prefix, which is what
    /// the lengthening in `FreshBlanks` is for.
    #[test]
    fn a_conclusion_naming_this_modules_prefix_does_not_alias() {
        let premise = graph(&[(A, RDF_TYPE, OWL_CLASS)]);
        let conclusion = union_conclusion("_purrdfEntailsFresh0");
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("entailed");
        };
        assert!(verify(&warrant, &premise, &conclusion));
    }

    // ── Applicability is a whitelist ───────────────────────────────────────────────────

    /// EVERY unrecognized shape disqualifies the whole conclusion.
    #[test]
    fn every_unrecognized_shape_disqualifies() {
        type Case = (
            &'static str,
            Vec<(&'static str, &'static str, &'static str)>,
        );
        let cases: [Case; 8] = [
            (
                "a NAMED union is a class axiom, not a comprehension",
                vec![
                    ("http://example.org/C", OWL_UNIONOF, "_l1"),
                    ("_l1", RDF_FIRST, A),
                    ("_l1", RDF_REST, RDF_NIL),
                ],
            ),
            (
                "a nested anonymous operand is not read",
                vec![
                    ("_c", OWL_UNIONOF, "_l1"),
                    ("_l1", RDF_FIRST, "_d"),
                    ("_l1", RDF_REST, RDF_NIL),
                ],
            ),
            (
                "a constructor this module does not read",
                vec![
                    ("_c", OWL_ONEOF, "_l1"),
                    ("_l1", RDF_FIRST, A),
                    ("_l1", RDF_REST, RDF_NIL),
                    ("_c", OWL_UNIONOF, RDF_NIL),
                ],
            ),
            (
                "two constructors on one node",
                vec![
                    ("_c", OWL_UNIONOF, RDF_NIL),
                    ("_c", OWL_INTERSECTIONOF, RDF_NIL),
                ],
            ),
            (
                "a restriction with two constraints",
                vec![
                    ("_c", RDF_TYPE, OWL_RESTRICTION),
                    ("_c", OWL_ONPROPERTY, P),
                    ("_c", OWL_SOMEVALUESFROM, A),
                    ("_c", OWL_HASVALUE, B),
                ],
            ),
            (
                "a restriction with no property",
                vec![
                    ("_c", RDF_TYPE, OWL_RESTRICTION),
                    ("_c", OWL_SOMEVALUESFROM, A),
                ],
            ),
            (
                "a membership in a restriction is a counting question",
                vec![
                    ("_c", RDF_TYPE, OWL_RESTRICTION),
                    ("_c", OWL_ONPROPERTY, P),
                    ("_c", OWL_SOMEVALUESFROM, A),
                    (X, RDF_TYPE, "_c"),
                ],
            ),
            (
                "a scaffold node mentioned somewhere else is load-bearing there",
                vec![
                    ("_c", OWL_UNIONOF, "_l1"),
                    ("_l1", RDF_FIRST, A),
                    ("_l1", RDF_REST, RDF_NIL),
                    ("_c", RDF_FIRST, B),
                ],
            ),
        ];
        for (why, triples) in cases {
            let Read::Declined(reasons) = read(&graph(&triples)) else {
                panic!("{why}: a recognized-and-declined shape is an ADMISSION, never a shrug");
            };
            assert!(!reasons.is_empty(), "{why}: the refusal names nothing");
        }
        // …and an EMPTY collection, whose union and intersection are the named `owl:Nothing`
        // and `owl:Thing`.
        assert!(matches!(
            read(&graph(&[("_c", OWL_UNIONOF, RDF_NIL)])),
            Read::Declined(_)
        ));
    }

    /// A conclusion naming no anonymous class expression is NOT this module's business, and
    /// it does not pretend otherwise by declining something it never recognized.
    #[test]
    fn an_ordinary_conclusion_is_not_applicable() {
        assert!(matches!(
            read(&graph(&[(A, RDF_TYPE, OWL_CLASS)])),
            Read::NotApplicable
        ));
    }

    /// A cardinality operand outside the non-negative integers licenses nothing.
    #[test]
    fn a_cardinality_must_be_a_non_negative_integer() {
        assert!(is_non_negative_integer(&TermValue::typed_literal(
            "1", XSD_INT
        )));
        assert!(is_non_negative_integer(&TermValue::typed_literal(
            "0",
            crate::vocab::XSD_NONNEGATIVEINTEGER
        )));
        assert!(!is_non_negative_integer(&TermValue::typed_literal(
            "-1", XSD_INT
        )));
        assert!(!is_non_negative_integer(&TermValue::typed_literal(
            "one", XSD_STRING
        )));
        assert!(!is_non_negative_integer(&TermValue::iri(A)));
    }

    // ── The inventory, and `verify` as a CHECK ─────────────────────────────────────────

    /// STRICT MATERIALIZATION GAINS NOTHING: no rule concludes an anonymous class.
    #[test]
    fn materialization_still_does_not_produce_these_conclusions() {
        let (closure, _) = materialize(&graph(&[(A, RDF_TYPE, OWL_CLASS)]), Materialization::OwlRl)
            .expect("consistent");
        assert!(
            !default_graph_triples(&closure)
                .iter()
                .any(|[_, p, _]| p == &TermValue::iri(OWL_UNIONOF)),
            "no rule of Tables 4-9 concludes an owl:unionOf class"
        );
    }

    /// THE NORMATIVE INVENTORY IS UNTOUCHED.
    #[test]
    fn the_comprehension_lane_adds_no_rule() {
        assert_eq!(rules(Regime::OwlRl).len(), 78);
        assert_eq!(implemented(Regime::OwlRl), rules(Regime::OwlRl));
        assert_eq!(extensions(Regime::OwlRl), [RuleId::ExtEqDiffSym]);
    }

    /// A comprehension warrant does not replay against another premise or conclusion.
    #[test]
    fn a_comprehension_warrant_does_not_replay() {
        let premise = graph(&[(A, RDF_TYPE, OWL_CLASS)]);
        let conclusion = union_conclusion("_l");
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("entailed");
        };
        assert!(verify(&warrant, &premise, &conclusion));
        assert!(!verify(
            &warrant,
            &graph(&[(B, RDF_TYPE, OWL_CLASS)]),
            &conclusion
        ));
        assert!(!verify(
            &warrant,
            &premise,
            &graph(&[(A, RDF_TYPE, OWL_CLASS)])
        ));
    }

    /// The whole answer is a function of the inputs: two runs mint the same triples.
    #[test]
    fn the_comprehension_lane_is_deterministic() {
        let run = || {
            let EntailmentOutcome::Entailed(EntailmentWarrant::Comprehension(w)) =
                decide(&graph(&[(A, RDF_TYPE, OWL_CLASS)]), &union_conclusion("_l"))
            else {
                panic!("entailed");
            };
            (w.minted().to_vec(), w.licences().to_vec())
        };
        assert_eq!(run(), run());
    }
}
