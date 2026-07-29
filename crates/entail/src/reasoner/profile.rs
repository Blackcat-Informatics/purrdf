// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! OWL 2 profile certification: which of EL, QL, RL and DL an ontology is *provably* in.
//!
//! # Why a reasoner ships this
//!
//! The OWL 2 RL/RDF completeness theorem (Profiles §4.3, Theorem PR1) holds **for
//! ontologies in the RL profile**, for ground atomic conclusions. Without an executable
//! membership check, "this crate is complete for OWL 2 RL" is a claim nothing can test on
//! the ontology actually in front of it — the rule table can be complete while the input
//! is outside the profile the theorem quantifies over. [`profile`] is that check, and it is
//! the precondition a caller evaluates before believing the RL lane's report.
//!
//! # The doctrine: every check is a SUFFICIENT condition
//!
//! Certification here is **one-directional and deliberately so**:
//!
//! * A **clean certification proves membership.** If [`ProfileCertificate::certifies`]
//!   answers `true`, the ontology really is in that profile: every construct the profile's
//!   grammar excludes is checked for, and any construct this module cannot place is treated
//!   as occurring in *both* class-expression positions, so an unanalysable shape can only
//!   ever cause a violation and never hide one.
//! * A **violation proves only that the cheap structural condition failed.** It does NOT
//!   prove the ontology is outside the profile. Several OWL 2 constructs are legal in one
//!   class-expression position and illegal in the other, and the position analysis here is
//!   syntactic: it reads `rdfs:subClassOf`, `owl:equivalentClass`, `owl:disjointWith`,
//!   `rdfs:domain`, `rdfs:range`, `rdf:type` and the class-expression constructors, and
//!   propagates covariantly (contravariantly through `owl:complementOf`). An ontology whose
//!   membership depends on a shape outside that reading is reported as a violation rather
//!   than certified, because the alternative is a false certificate.
//!
//! That asymmetry is the whole point. A certifier that is wrong in the permissive direction
//! licenses a completeness claim the theorem does not support; one that is wrong in the
//! restrictive direction merely declines to license one. Only the second kind of error is
//! survivable, so every judgement call in this module is made in that direction.
//!
//! `Full` is certified unconditionally, and that is not a shrug: every RDF graph is an
//! OWL 2 Full ontology under the RDF-Based Semantics, so the certificate would be lying if
//! it withheld it.
//!
//! # Determinism
//!
//! The dataset is scanned into an ordered `BTreeMap` index, positions are propagated to a
//! fixpoint over that ordered structure, and the violation list is sorted by
//! (profile, term, subject, reason). Nothing is read out of a hash map and no result
//! depends on quad order.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::{RdfDataset, TermValue};

use super::term_key;
use crate::interner::Interner;
use crate::owl_dl::constructs::{Support, is_reserved, support_of};
use crate::owl_dl::parser::{TripleIndex, Vocab};
use crate::owl_dl::query::build_data_index;
use crate::report::Construct;
use crate::vocab::{OWL_DATATYPEPROPERTY, OWL_OBJECTPROPERTY, OWL_PROPERTYCHAINAXIOM};

/// An OWL 2 profile.
///
/// Declared from the most restrictive to the least, which is the order
/// [`ProfileCertificate::certified`] lists them in and the order [`ProfileViolation`]s sort
/// by. `Dl` and `Full` are not "profiles" in the Profiles-document sense — they are the two
/// species of OWL 2 itself — but a caller asking "what is this ontology?" wants all five in
/// one answer, and splitting them across two types would make the common question take two
/// calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OwlProfile {
    /// OWL 2 EL — the existential-restriction profile, `PTime`-complete for the standard
    /// reasoning tasks.
    El,
    /// OWL 2 QL — the query-rewriting profile, `AC⁰` in data complexity.
    Ql,
    /// OWL 2 RL — the rule profile, implementable by a forward chase over
    /// [`Regime::OwlRl`](crate::Regime::OwlRl)'s rule table.
    Rl,
    /// OWL 2 DL — the decidable species, subject to the global structural restrictions.
    Dl,
    /// OWL 2 Full — every RDF graph, under the RDF-Based Semantics.
    Full,
}

impl OwlProfile {
    /// Every profile, most restrictive first.
    pub const ALL: [Self; 5] = [Self::El, Self::Ql, Self::Rl, Self::Dl, Self::Full];

    /// A short, stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::El => "EL",
            Self::Ql => "QL",
            Self::Rl => "RL",
            Self::Dl => "DL",
            Self::Full => "Full",
        }
    }
}

impl std::fmt::Display for OwlProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One reason a profile could not be certified.
///
/// The reason is a `&'static str` chosen from a fixed set, exactly as
/// [`Construct::reason`](crate::Construct::reason) is, so a violation and its explanation
/// cannot drift apart the way a code path and a hand-written message would.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileViolation {
    /// The profile this blocks.
    profile: OwlProfile,
    /// The reserved term whose use blocked it.
    term: TermValue,
    /// The node the term was written on — the class expression, or the axiom's subject.
    subject: TermValue,
    /// Why the profile excludes it, and whether the exclusion is absolute or positional.
    reason: &'static str,
}

impl ProfileViolation {
    /// The profile this blocks.
    #[must_use]
    pub const fn profile(&self) -> OwlProfile {
        self.profile
    }

    /// The reserved term whose use blocked it.
    #[must_use]
    pub const fn term(&self) -> &TermValue {
        &self.term
    }

    /// The node the term was written on.
    #[must_use]
    pub const fn subject(&self) -> &TermValue {
        &self.subject
    }

    /// Why the profile excludes it.
    ///
    /// A reason that begins "only in" names a POSITIONAL exclusion — the construct is legal
    /// elsewhere in the profile, and this violation says only that the syntactic position
    /// analysis could not place this occurrence somewhere legal.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.reason
    }
}

impl std::fmt::Display for ProfileViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.profile.as_str(), self.reason)
    }
}

/// Which OWL 2 profiles an ontology is provably in, and what blocked the others.
///
/// See the [module docs](self) for the one-directional doctrine this certificate is built
/// on: a certification proves membership, a violation does not prove non-membership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileCertificate {
    /// Every violation found, sorted.
    violations: Vec<ProfileViolation>,
}

impl ProfileCertificate {
    /// Whether `profile` is certified — i.e. no violation of it was found.
    ///
    /// [`OwlProfile::Full`] is always certified; see the [module docs](self).
    #[must_use]
    pub fn certifies(&self, profile: OwlProfile) -> bool {
        !self
            .violations
            .iter()
            .any(|violation| violation.profile == profile)
    }

    /// Every certified profile, most restrictive first.
    #[must_use]
    pub fn certified(&self) -> Vec<OwlProfile> {
        OwlProfile::ALL
            .into_iter()
            .filter(|&profile| self.certifies(profile))
            .collect()
    }

    /// Every violation found, sorted by profile, then term, then subject, then reason.
    #[must_use]
    pub fn violations(&self) -> &[ProfileViolation] {
        &self.violations
    }

    /// The violations blocking one profile.
    #[must_use]
    pub fn violations_of(&self, profile: OwlProfile) -> Vec<&ProfileViolation> {
        self.violations
            .iter()
            .filter(|violation| violation.profile == profile)
            .collect()
    }
}

/// Certify `ds` against the OWL 2 profiles.
///
/// Purely syntactic: no tableau runs, no closure, no budget — which is what makes this an
/// executable PRECONDITION rather than a second reasoning pass. See the [module docs](self)
/// for what a certification proves and what a violation does not.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_entail::reasoner::{OwlProfile, profile};
///
/// let mut b = RdfDatasetBuilder::new();
/// let cat = b.intern_iri("http://example.org/Cat");
/// let animal = b.intern_iri("http://example.org/Animal");
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// b.push_quad(cat, sub, animal, None);
/// let dataset = b.freeze().expect("freeze");
///
/// // A bare sub-class axiom is in every profile.
/// let certificate = profile(&dataset);
/// assert_eq!(certificate.certified(), OwlProfile::ALL.to_vec());
/// assert!(certificate.violations().is_empty());
/// ```
#[must_use]
pub fn profile(ds: &RdfDataset) -> ProfileCertificate {
    let mut interner = Interner::default();
    let v = Vocab::intern(&mut interner);
    let extra = Extra {
        chain: interner.intern_iri(OWL_PROPERTYCHAINAXIOM),
        object_property: interner.intern_iri(OWL_OBJECTPROPERTY),
        data_property: interner.intern_iri(OWL_DATATYPEPROPERTY),
    };
    let index = build_data_index(ds, &mut interner);
    let scan = Scan::of(&index, &v);
    let mut violations = Vec::new();
    scan.class_expressions(&interner, &v, &index, &mut violations);
    scan.axioms(&interner, &v, extra.chain, &index, &mut violations);
    scan.description_logic(&interner, &v, &extra, &index, &mut violations);
    violations.sort_by(|a, b| {
        (a.profile, term_key(&a.term), term_key(&a.subject), a.reason).cmp(&(
            b.profile,
            term_key(&b.term),
            term_key(&b.subject),
            b.reason,
        ))
    });
    violations.dedup();
    ProfileCertificate { violations }
}

/// A class expression occurs on the SUBCLASS side of an inclusion.
const SUB: u8 = 1;
/// A class expression occurs on the SUPERCLASS side of an inclusion.
const SUP: u8 = 2;
/// Both sides — the conservative default for an occurrence this module cannot place.
const BOTH: u8 = SUB | SUP;

/// Swap the two class-expression positions, which is what `owl:complementOf` does to its
/// operand: `¬D ⊑ E` constrains `D` where `E` would be constrained, and vice versa.
const fn flip(positions: u8) -> u8 {
    match positions {
        SUB => SUP,
        SUP => SUB,
        other => other,
    }
}

/// The syntactic survey a certification is computed from.
struct Scan {
    /// Class-expression node → the positions it was found in.
    positions: BTreeMap<u32, u8>,
}

impl Scan {
    /// Survey `index`, seeding positions from the axiom shapes and propagating them into
    /// every sub-expression to a fixpoint.
    fn of(index: &TripleIndex, v: &Vocab) -> Self {
        let mut positions: BTreeMap<u32, u8> = BTreeMap::new();
        seed(index, v, &mut positions);
        // Propagate, then force any class expression still unplaced into BOTH positions and
        // propagate again. Forcing rather than skipping is the conservative direction: an
        // occurrence this module cannot place must satisfy the grammar for both sides, so an
        // unanalysable shape can cause a violation and can never hide one.
        loop {
            propagate(index, v, &mut positions);
            let unplaced: Vec<u32> = index
                .iter()
                .filter(|(node, preds)| {
                    former(preds, v).is_some() && positions.get(node).copied().unwrap_or(0) == 0
                })
                .map(|(&node, _)| node)
                .collect();
            if unplaced.is_empty() {
                return Self { positions };
            }
            for node in unplaced {
                positions.insert(node, BOTH);
            }
        }
    }

    /// The positions `node` was found in; `0` for a node that is not a class expression.
    fn at(&self, node: u32) -> u8 {
        self.positions.get(&node).copied().unwrap_or(0)
    }

    /// Check every placed class expression against the EL, QL and RL grammars.
    fn class_expressions(
        &self,
        interner: &Interner,
        v: &Vocab,
        index: &TripleIndex,
        out: &mut Vec<ProfileViolation>,
    ) {
        for (&node, preds) in index {
            let Some(former) = former(preds, v) else {
                continue;
            };
            let positions = self.at(node);
            if positions == 0 {
                continue;
            }
            let subject = interner.value(node).clone();
            let term = interner.value(former.term(v)).clone();
            let mut deny = |profile: OwlProfile, reason: &'static str| {
                out.push(ProfileViolation {
                    profile,
                    term: term.clone(),
                    subject: subject.clone(),
                    reason,
                });
            };
            if let Some(reason) = el_denies(former, index, v, node) {
                deny(OwlProfile::El, reason);
            }
            if let Some(reason) = ql_denies(former, positions, index, v, node) {
                deny(OwlProfile::Ql, reason);
            }
            if let Some(reason) = rl_denies(former, positions, index, interner, v, node) {
                deny(OwlProfile::Rl, reason);
            }
        }
    }

    /// Check the axiom-level vocabulary — the constructs whose profile membership does not
    /// depend on a class-expression position.
    fn axioms(
        &self,
        interner: &Interner,
        v: &Vocab,
        chain: u32,
        index: &TripleIndex,
        out: &mut Vec<ProfileViolation>,
    ) {
        for (&s, preds) in index {
            for (&p, objs) in preds {
                for &o in objs {
                    let denied = if p == v.ty {
                        typing_denials(o, v)
                    } else {
                        predicate_denials(p, chain, v)
                    };
                    for &(profile, reason) in denied {
                        out.push(ProfileViolation {
                            profile,
                            term: interner.value(if p == v.ty { o } else { p }).clone(),
                            subject: interner.value(s).clone(),
                            reason,
                        });
                    }
                }
            }
        }
    }

    /// Check the OWL 2 DL global restrictions this module can decide syntactically.
    fn description_logic(
        &self,
        interner: &Interner,
        v: &Vocab,
        extra: &Extra,
        index: &TripleIndex,
        out: &mut Vec<ProfileViolation>,
    ) {
        let Extra {
            chain,
            object_property,
            data_property,
        } = *extra;
        let non_simple = non_simple_roles(index, v, chain);
        for (&node, preds) in index {
            let subject = interner.value(node).clone();
            let mut deny = |term: u32, reason: &'static str| {
                out.push(ProfileViolation {
                    profile: OwlProfile::Dl,
                    term: interner.value(term).clone(),
                    subject: subject.clone(),
                    reason,
                });
            };
            // A number restriction must be over a SIMPLE role.
            let counts = [
                v.min_card,
                v.max_card,
                v.card,
                v.min_qcard,
                v.max_qcard,
                v.qcard,
            ]
            .into_iter()
            .find(|term| preds.contains_key(term));
            if let Some(count) = counts
                && preds
                    .get(&v.on_property)
                    .is_some_and(|roles| roles.iter().any(|role| non_simple.contains(role)))
            {
                deny(count, NON_SIMPLE_COUNT);
            }
            // A chain axiom needs SROIQ's regularity condition decided before the ontology
            // is OWL 2 DL, and nothing here decides it.
            if preds.contains_key(&chain) {
                deny(chain, CHAIN_REGULARITY);
            }
            // Object and data properties are disjoint in OWL 2 DL.
            if let Some(types) = preds.get(&v.ty)
                && types.contains(&object_property)
                && types.contains(&data_property)
            {
                deny(v.ty, PROPERTY_TYPE_SEPARATION);
            }
            // Reserved vocabulary may only be used as the OWL-2-RDF mapping prescribes, so a
            // reserved term the mapping does not name is a term used outside its
            // specification — which OWL 2 DL forbids.
            for &term in preds.keys() {
                if let TermValue::Iri(iri) = interner.value(term)
                    && is_reserved(iri)
                    && matches!(
                        support_of(iri),
                        Some(Support::Bounded(Construct::UnrecognizedTerm))
                    )
                {
                    deny(term, RESERVED_VOCABULARY);
                }
            }
        }
        // A collection an OWL 2 axiom points at must be a well-formed collection.
        for (&node, preds) in index {
            for &list_predicate in &[
                v.intersection,
                v.union,
                v.one_of,
                v.members,
                v.distinct_members,
                v.has_key,
                v.disjoint_union,
                chain,
            ] {
                for &head in preds.get(&list_predicate).map_or(&[][..], Vec::as_slice) {
                    if !well_formed_collection(index, v, head) {
                        out.push(ProfileViolation {
                            profile: OwlProfile::Dl,
                            term: interner.value(list_predicate).clone(),
                            subject: interner.value(node).clone(),
                            reason: MALFORMED_COLLECTION,
                        });
                    }
                }
            }
        }
    }
}

/// Why OWL 2 EL excludes `former`, if it does.
///
/// EL has a single class-expression category, so no position is consulted: a construct
/// EL's grammar omits is omitted everywhere.
fn el_denies(former: Former, index: &TripleIndex, v: &Vocab, node: u32) -> Option<&'static str> {
    match former {
        Former::Intersection | Former::SomeValues | Former::HasValue | Former::HasSelf => None,
        // `ObjectOneOf` is in EL with exactly ONE individual, and the arity is cheap to
        // read, so this is decided rather than conservatively denied.
        Former::OneOf => (one_of_arity(index, v, node) != 1).then_some(EL_ONE_OF_ARITY),
        Former::Union => Some(EL_NO_UNION),
        Former::Complement => Some(EL_NO_COMPLEMENT),
        Former::AllValues => Some(EL_NO_ALL_VALUES),
        Former::MinCardinality | Former::MaxCardinality | Former::ExactCardinality => {
            Some(EL_NO_CARDINALITY)
        }
    }
}

/// Why OWL 2 QL excludes `former` at `positions`, if it does.
fn ql_denies(
    former: Former,
    positions: u8,
    index: &TripleIndex,
    v: &Vocab,
    node: u32,
) -> Option<&'static str> {
    if positions & SUB != 0 {
        // QL's subClassExpression is a class or an UNQUALIFIED existential.
        let ok =
            former == Former::SomeValues && some_values_filler(index, v, node) == Some(v.thing);
        if !ok {
            return Some(QL_SUBCLASS);
        }
    }
    if positions & SUP != 0 {
        let ok = matches!(
            former,
            Former::Intersection | Former::Complement | Former::SomeValues
        );
        if !ok {
            return Some(QL_SUPERCLASS);
        }
    }
    None
}

/// Why OWL 2 RL excludes `former` at `positions`, if it does.
fn rl_denies(
    former: Former,
    positions: u8,
    index: &TripleIndex,
    interner: &Interner,
    v: &Vocab,
    node: u32,
) -> Option<&'static str> {
    if positions & SUB != 0
        && !matches!(
            former,
            Former::Intersection
                | Former::Union
                | Former::OneOf
                | Former::SomeValues
                | Former::HasValue
        )
    {
        return Some(RL_SUBCLASS);
    }
    if positions & SUP != 0 {
        let ok = match former {
            Former::Intersection | Former::Complement | Former::AllValues | Former::HasValue => {
                true
            }
            // RL admits a max-cardinality restriction only at 0 or 1, and the bound is a
            // literal this module can read, so the check is decided rather than denied.
            Former::MaxCardinality => max_cardinality_bound(index, interner, v, node)
                .is_some_and(|bound| bound == "0" || bound == "1"),
            _ => false,
        };
        if !ok {
            return Some(RL_SUPERCLASS);
        }
    }
    None
}

/// How many members an `owl:oneOf` node enumerates.
fn one_of_arity(index: &TripleIndex, v: &Vocab, node: u32) -> usize {
    index
        .get(&node)
        .and_then(|preds| preds.get(&v.one_of))
        .and_then(|heads| heads.first())
        .map_or(0, |&head| members(index, v, head, v.first).len())
}

/// The filler of an `owl:someValuesFrom` restriction.
fn some_values_filler(index: &TripleIndex, v: &Vocab, node: u32) -> Option<u32> {
    index
        .get(&node)
        .and_then(|preds| preds.get(&v.some_values))
        .and_then(|fillers| fillers.first())
        .copied()
}

/// The lexical form of a max-cardinality bound, if the restriction states exactly one.
fn max_cardinality_bound<'a>(
    index: &TripleIndex,
    interner: &'a Interner,
    v: &Vocab,
    node: u32,
) -> Option<&'a str> {
    let preds = index.get(&node)?;
    let bounds = preds.get(&v.max_card).or_else(|| preds.get(&v.max_qcard))?;
    let [bound] = bounds.as_slice() else {
        return None;
    };
    match interner.value(*bound) {
        TermValue::Literal { lexical_form, .. } => Some(lexical_form),
        _ => None,
    }
}

/// Whether the collection rooted at `head` terminates at `rdf:nil` with exactly one
/// `rdf:first` and one `rdf:rest` per cell.
fn well_formed_collection(index: &TripleIndex, v: &Vocab, head: u32) -> bool {
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut cell = head;
    while cell != v.nil {
        if !seen.insert(cell) {
            return false;
        }
        let Some(preds) = index.get(&cell) else {
            return false;
        };
        if preds.get(&v.first).map_or(0, Vec::len) != 1 {
            return false;
        }
        match preds.get(&v.rest) {
            Some(rest) if rest.len() == 1 => cell = rest[0],
            _ => return false,
        }
    }
    true
}

/// The roles a transitivity axiom or a property chain makes NON-SIMPLE.
///
/// A role is non-simple when it, or any role beneath it in the `rdfs:subPropertyOf`
/// hierarchy, is transitive or is the head of a property chain — the same closure
/// [`crate::owl_dl::parser`]'s `is_non_simple` computes, restated here over the raw
/// triples so that profile certification needs no knowledge base and no tableau.
fn non_simple_roles(index: &TripleIndex, v: &Vocab, chain: u32) -> BTreeSet<u32> {
    let mut composite: BTreeSet<u32> = BTreeSet::new();
    // super-property → its sub-properties.
    let mut role_sub: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (&s, preds) in index {
        for (&p, objs) in preds {
            for &o in objs {
                if (p == v.ty && o == v.transitive) || p == chain {
                    composite.insert(s);
                } else if p == v.sub_prop {
                    role_sub.entry(o).or_default().insert(s);
                }
            }
        }
    }
    let candidates: BTreeSet<u32> = role_sub
        .keys()
        .copied()
        .chain(composite.iter().copied())
        .collect();
    let mut out = BTreeSet::new();
    for candidate in candidates {
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        let mut stack = vec![candidate];
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            if composite.contains(&current) {
                out.insert(candidate);
                break;
            }
            if let Some(subs) = role_sub.get(&current) {
                stack.extend(subs.iter().copied());
            }
        }
    }
    out
}

/// The reserved terms a profile check needs that [`Vocab`] does not carry.
#[derive(Debug, Clone, Copy)]
struct Extra {
    /// `owl:propertyChainAxiom`.
    chain: u32,
    /// `owl:ObjectProperty`.
    object_property: u32,
    /// `owl:DatatypeProperty`.
    data_property: u32,
}

/// The profiles a `rdf:type` object excludes the ontology from, with the reason.
///
/// A `&'static` table rather than a chain of pushes: each row is one specification
/// statement, and a reader can check the table against OWL 2 Profiles §2 without following
/// control flow.
fn typing_denials(object: u32, v: &Vocab) -> &'static [(OwlProfile, &'static str)] {
    if object == v.functional {
        &[
            (OwlProfile::El, EL_FUNCTIONAL),
            (OwlProfile::Ql, QL_FUNCTIONAL),
        ]
    } else if object == v.inverse_functional {
        &[
            (OwlProfile::El, EL_NO_INVERSE),
            (OwlProfile::Ql, QL_FUNCTIONAL),
        ]
    } else if object == v.transitive {
        &[(OwlProfile::Ql, QL_TRANSITIVE)]
    } else if object == v.symmetric || object == v.asymmetric || object == v.irreflexive {
        &[(OwlProfile::El, EL_NO_ROLE_CHARACTERISTIC)]
    } else if object == v.reflexive {
        &[(OwlProfile::Rl, RL_REFLEXIVE)]
    } else if object == v.negative_property_assertion {
        &[(OwlProfile::Ql, QL_NEGATIVE_ASSERTION)]
    } else {
        &[]
    }
}

/// The profiles a predicate excludes the ontology from, with the reason.
fn predicate_denials(
    predicate: u32,
    chain: u32,
    v: &Vocab,
) -> &'static [(OwlProfile, &'static str)] {
    if predicate == v.inverse_of {
        &[(OwlProfile::El, EL_NO_INVERSE)]
    } else if predicate == chain {
        &[(OwlProfile::Ql, QL_CHAIN)]
    } else if predicate == v.has_key {
        &[(OwlProfile::Ql, QL_HAS_KEY)]
    } else if predicate == v.same_as {
        &[(OwlProfile::Ql, QL_SAME_AS)]
    } else if predicate == v.disjoint_union {
        &[
            (OwlProfile::El, NO_DISJOINT_UNION),
            (OwlProfile::Ql, NO_DISJOINT_UNION),
            (OwlProfile::Rl, NO_DISJOINT_UNION),
        ]
    } else if predicate == v.datatype_complement || predicate == v.with_restrictions {
        &[
            (OwlProfile::El, NO_DATA_RANGE),
            (OwlProfile::Ql, NO_DATA_RANGE),
            (OwlProfile::Rl, NO_DATA_RANGE),
        ]
    } else {
        &[]
    }
}

/// `owl:oneOf` is in OWL 2 EL only as a SINGLETON.
const EL_ONE_OF_ARITY: &str = "OWL 2 EL admits owl:oneOf only over exactly one individual (ObjectOneOf with a single \
     member); an enumeration of two or more is a disjunction, and EL has none";
/// EL has no disjunction.
const EL_NO_UNION: &str = "OWL 2 EL has no union: its class expressions are conjunctions of atomic classes, \
     existential restrictions, owl:hasValue and owl:hasSelf, which is what keeps its \
     reasoning problems PTime-complete";
/// EL has no negation.
const EL_NO_COMPLEMENT: &str = "OWL 2 EL has no complement: negation would restore the disjunction EL_NO_UNION \
     excludes, since ¬(¬C ⊓ ¬D) is C ⊔ D";
/// EL has no universal restriction.
const EL_NO_ALL_VALUES: &str = "OWL 2 EL has no universal restriction (owl:allValuesFrom); only the existential \
     owl:someValuesFrom is in its grammar";
/// EL has no cardinality restriction.
const EL_NO_CARDINALITY: &str = "OWL 2 EL has no cardinality restriction in any form — minimum, maximum, exact, or \
     qualified";
/// EL has no inverse.
const EL_NO_INVERSE: &str = "OWL 2 EL has no inverse roles, so neither owl:inverseOf nor \
     owl:InverseFunctionalProperty (which is functionality of an inverse) is in its \
     grammar";
/// EL admits functional DATA properties only.
const EL_FUNCTIONAL: &str = "OWL 2 EL admits FunctionalDataProperty but not FunctionalObjectProperty, and \
     owl:FunctionalProperty spells both; this check is position-insensitive, so a \
     functional DATA property is reported here even though EL admits it";
/// EL has no symmetry, asymmetry or irreflexivity.
const EL_NO_ROLE_CHARACTERISTIC: &str = "OWL 2 EL admits reflexivity and transitivity of a role but not symmetry, asymmetry or \
     irreflexivity — the three that need inverse or negated role atoms";
/// QL's subclass grammar.
const QL_SUBCLASS: &str = "only in superclass position: OWL 2 QL's subClassExpression is a named class or an \
     UNQUALIFIED existential owl:someValuesFrom owl:Thing, and nothing else — that \
     restriction is what makes QL query-rewritable in AC⁰";
/// QL's superclass grammar.
const QL_SUPERCLASS: &str = "only in subclass position: OWL 2 QL's superClassExpression is a named class, an \
     intersection, a complement of a subClassExpression, or an existential \
     owl:someValuesFrom, and nothing else";
/// QL has no functionality.
const QL_FUNCTIONAL: &str = "OWL 2 QL has neither FunctionalObjectProperty nor InverseFunctionalProperty: \
     functionality forces identifications, and QL's query rewriting never merges \
     individuals";
/// QL has no transitivity.
const QL_TRANSITIVE: &str = "OWL 2 QL has no TransitiveObjectProperty: a transitive role needs recursion, and QL's \
     data complexity bound admits none";
/// QL has no property chains.
const QL_CHAIN: &str = "OWL 2 QL has no complex role inclusion (owl:propertyChainAxiom): a chain is recursion \
     by another name";
/// QL has no keys.
const QL_HAS_KEY: &str = "OWL 2 QL has no HasKey axiom: a key identifies individuals, and QL \
     never merges them";
/// QL has no individual equality.
const QL_SAME_AS: &str = "OWL 2 QL has no SameIndividual axiom: asserting equality merges individuals, which \
     QL's query rewriting does not do";
/// QL has no negative assertions.
const QL_NEGATIVE_ASSERTION: &str =
    "OWL 2 QL has no NegativeObjectPropertyAssertion or NegativeDataPropertyAssertion";
/// RL's subclass grammar.
const RL_SUBCLASS: &str = "only in superclass position: OWL 2 RL's subClassExpression is a named class, an \
     intersection, a union, an enumeration, an existential owl:someValuesFrom or an \
     owl:hasValue — the shapes an RL rule can match in a rule BODY";
/// RL's superclass grammar.
const RL_SUPERCLASS: &str = "only in subclass position: OWL 2 RL's superClassExpression is a named class, an \
     intersection, a complement, a universal owl:allValuesFrom, an owl:hasValue, or a \
     max-cardinality restriction bounded by 0 or 1 — the shapes an RL rule can produce in \
     a rule HEAD";
/// RL has no reflexivity axiom.
const RL_REFLEXIVE: &str = "OWL 2 RL has no ReflexiveObjectProperty: reflexivity asserts a role edge at every \
     element of the domain, which no ground rule head can produce";
/// No profile admits `owl:disjointUnionOf`.
const NO_DISJOINT_UNION: &str = "no OWL 2 profile admits DisjointUnion: it is a union axiom and a pairwise-disjointness \
     axiom at once, and each half is excluded by at least one profile";
/// No profile admits a constructed data range.
const NO_DATA_RANGE: &str = "no OWL 2 profile admits a constructed data range (owl:datatypeComplementOf, or \
     owl:onDatatype with owl:withRestrictions facets); each profile's DataRange is a \
     datatype or an intersection of datatypes";
/// A number restriction over a non-simple role.
const NON_SIMPLE_COUNT: &str = "OWL 2 DL requires the role of a number restriction to be SIMPLE — not transitive, not \
     above a transitive role, and not the head of a property chain — because counting the \
     successors of a composite role is undecidable";
/// A property chain whose regularity is undecided.
const CHAIN_REGULARITY: &str = "OWL 2 DL requires the complex role inclusions to be REGULAR (SROIQ's acyclicity \
     condition on the chain order), and this certifier does not decide regularity; the \
     ontology may well be DL, and the check declines to say so rather than guessing";
/// Object and data properties overlap.
const PROPERTY_TYPE_SEPARATION: &str = "OWL 2 DL requires the object, data and annotation properties to be pairwise disjoint, \
     and this IRI is declared both an owl:ObjectProperty and an owl:DatatypeProperty";
/// Reserved vocabulary used outside the mapping.
const RESERVED_VOCABULARY: &str = "OWL 2 DL admits a term of the rdf:, rdfs: or owl: namespaces only where the OWL-2-RDF \
     mapping puts it, and this term is not one the mapping writes at all — a mistyped, \
     newer-than-this-release, or genuinely non-DL use";
/// A malformed RDF collection under an OWL 2 axiom.
const MALFORMED_COLLECTION: &str = "an OWL 2 axiom's operand list must be a well-formed RDF collection — one rdf:first and \
     one rdf:rest per cell, terminating at rdf:nil, without a cycle — and this one is not, \
     so the ontology is not a well-formed OWL 2 DL ontology at all";

/// Add `positions` to `node`'s entry.
fn mark(positions: &mut BTreeMap<u32, u8>, node: u32, add: u8) {
    *positions.entry(node).or_insert(0) |= add;
}

/// Seed positions from the axiom shapes that fix them.
fn seed(index: &TripleIndex, v: &Vocab, positions: &mut BTreeMap<u32, u8>) {
    for (&s, preds) in index {
        for (&p, objs) in preds {
            for &o in objs {
                if p == v.sub_class {
                    mark(positions, s, SUB);
                    mark(positions, o, SUP);
                } else if p == v.equiv_class {
                    // An equivalence is two inclusions, so each side is on both sides.
                    mark(positions, s, BOTH);
                    mark(positions, o, BOTH);
                } else if p == v.disjoint {
                    // `C ⊓ D ⊑ ⊥` — both operands are on the SUBCLASS side.
                    mark(positions, s, SUB);
                    mark(positions, o, SUB);
                } else if p == v.domain || p == v.range {
                    // `∃p.⊤ ⊑ C` and `⊤ ⊑ ∀p.C`: the named class is a SUPERCLASS.
                    mark(positions, o, SUP);
                } else if p == v.ty {
                    if o == v.all_disjoint_classes {
                        for &member in &members(index, v, s, v.members) {
                            mark(positions, member, SUB);
                        }
                    } else if !v.structural_types.contains(&o) {
                        // A class assertion `a : C` puts `C` on the SUPERCLASS side.
                        mark(positions, o, SUP);
                    }
                } else if p == v.disjoint_union {
                    mark(positions, s, BOTH);
                    for &member in &members(index, v, o, v.first) {
                        mark(positions, member, BOTH);
                    }
                } else if p == v.has_key {
                    mark(positions, s, SUB);
                }
            }
        }
    }
}

/// Push each placed class expression's positions into its operands, once.
///
/// Called to a fixpoint by [`Scan::of`]; one pass is not enough because an operand may be
/// placed after its parent was.
fn propagate(index: &TripleIndex, v: &Vocab, positions: &mut BTreeMap<u32, u8>) {
    loop {
        let mut changed = false;
        let snapshot: Vec<(u32, u8)> = positions.iter().map(|(&n, &p)| (n, p)).collect();
        for (node, pos) in snapshot {
            let Some(preds) = index.get(&node) else {
                continue;
            };
            let mut push = |target: u32, add: u8, positions: &mut BTreeMap<u32, u8>| {
                let before = positions.get(&target).copied().unwrap_or(0);
                if before | add != before {
                    positions.insert(target, before | add);
                    changed = true;
                }
            };
            for (&p, objs) in preds {
                for &o in objs {
                    // Covariant operands: every member of an intersection or a union, and
                    // the filler of a restriction, is constrained on the same side its
                    // parent is.
                    if p == v.intersection || p == v.union {
                        for &member in &members(index, v, o, v.first) {
                            push(member, pos, positions);
                        }
                    } else if p == v.some_values
                        || p == v.all_values
                        || p == v.on_class
                        || p == v.on_data_range
                    {
                        push(o, pos, positions);
                    } else if p == v.complement {
                        // Contravariant: negation swaps the side its operand is on.
                        push(o, flip(pos), positions);
                    }
                }
            }
        }
        if !changed {
            return;
        }
    }
}

/// Walk an RDF collection, returning the members it could reach.
///
/// Total by construction: a malformed collection yields the prefix it could walk rather
/// than an error, because a malformed collection is separately reported as an OWL 2 DL
/// violation and a certifier that refused to answer at all would be less useful than one
/// that answers conservatively.
fn members(index: &TripleIndex, v: &Vocab, head: u32, key: u32) -> Vec<u32> {
    // `key` selects between walking an RDF collection from `head` (`rdf:first`) and reading
    // an n-ary axiom node's `owl:members` list, which is one indirection further out.
    let head = if key == v.first {
        head
    } else {
        match index.get(&head).and_then(|preds| preds.get(&key)) {
            Some(list) => match list.first() {
                Some(&cell) => cell,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        }
    };
    let mut out = Vec::new();
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut cell = head;
    while cell != v.nil && seen.insert(cell) {
        let Some(preds) = index.get(&cell) else {
            return out;
        };
        if let Some(first) = preds.get(&v.first).and_then(|f| f.first()) {
            out.push(*first);
        }
        match preds.get(&v.rest).and_then(|r| r.first()) {
            Some(&next) => cell = next,
            None => return out,
        }
    }
    out
}

/// The class-expression constructor a node carries, if it is a class expression.
///
/// A node with two constructors is read as the first in this fixed order; the profile
/// grammars exclude such a node from every profile anyway, and a fixed order keeps the
/// answer deterministic.
fn former(preds: &BTreeMap<u32, Vec<u32>>, v: &Vocab) -> Option<Former> {
    let has = |p: u32| preds.contains_key(&p);
    if has(v.intersection) {
        Some(Former::Intersection)
    } else if has(v.union) {
        Some(Former::Union)
    } else if has(v.complement) {
        Some(Former::Complement)
    } else if has(v.one_of) {
        Some(Former::OneOf)
    } else if has(v.some_values) {
        Some(Former::SomeValues)
    } else if has(v.all_values) {
        Some(Former::AllValues)
    } else if has(v.has_value) {
        Some(Former::HasValue)
    } else if has(v.has_self) {
        Some(Former::HasSelf)
    } else if has(v.min_card) || has(v.min_qcard) {
        Some(Former::MinCardinality)
    } else if has(v.max_card) || has(v.max_qcard) {
        Some(Former::MaxCardinality)
    } else if has(v.card) || has(v.qcard) {
        Some(Former::ExactCardinality)
    } else {
        None
    }
}

/// A class-expression constructor, as the profile grammars distinguish them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Former {
    /// `owl:intersectionOf`.
    Intersection,
    /// `owl:unionOf`.
    Union,
    /// `owl:complementOf`.
    Complement,
    /// `owl:oneOf`.
    OneOf,
    /// `owl:someValuesFrom`.
    SomeValues,
    /// `owl:allValuesFrom`.
    AllValues,
    /// `owl:hasValue`.
    HasValue,
    /// `owl:hasSelf`.
    HasSelf,
    /// `owl:minCardinality` / `owl:minQualifiedCardinality`.
    MinCardinality,
    /// `owl:maxCardinality` / `owl:maxQualifiedCardinality`.
    MaxCardinality,
    /// `owl:cardinality` / `owl:qualifiedCardinality`.
    ExactCardinality,
}

impl Former {
    /// The vocabulary term this constructor is written with, in the same order [`former`]
    /// tests them, so a violation names the term the ontology actually used.
    const fn term(self, v: &Vocab) -> u32 {
        match self {
            Self::Intersection => v.intersection,
            Self::Union => v.union,
            Self::Complement => v.complement,
            Self::OneOf => v.one_of,
            Self::SomeValues => v.some_values,
            Self::AllValues => v.all_values,
            Self::HasValue => v.has_value,
            Self::HasSelf => v.has_self,
            Self::MinCardinality => v.min_card,
            Self::MaxCardinality => v.max_card,
            Self::ExactCardinality => v.card,
        }
    }
}
