// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The chase's calculus, declared as DL-clause data — and evaluated from that declaration.
//!
//! [`ChaseRule`] names every rule the forward chase fires, once, so three things that must
//! agree cannot drift apart:
//!
//! * the tag a firing is COUNTED under ([`ChaseRule::rule_id`], which answers with the
//!   name the active regime's specification gives the rule);
//! * the entry the rule occupies in the inventory ([`crate::implemented`]);
//! * the DL clause that STATES the rule ([`calculus_program`]), whose digest is the
//!   report's [`contract_hash`](purrdf_datalog::cache::contract_hash).
//!
//! # The declaration IS the executable
//!
//! [`calculus_program`] renders the specification rules that [`crate::implemented`] names
//! — plus, for `OWL-RL`, the three RDFS-shaped rules that lane also fires and that have no
//! OWL 2 RL rule id — as [`DlClause`]s over spec `rdf:`/`rdfs:`/`owl:` IRIs. It is the
//! calculus's IDENTITY: hash it and a consumer can tell whether a cached closure was
//! minted under the rule set it is about to trust.
//!
//! It is also what runs. [`crate::engine`] hands these very clauses to
//! `purrdf-datalog`'s semi-naive evaluator, so the digest in a report names the clauses
//! that produced the closure the report accompanies rather than a parallel description of
//! them. There was once a hand-written chase beside this declaration, and it diverged from
//! it in two directions — broader triggers for the two reflexive rules, and narrower
//! conclusions where the RDF 1.2 IR could not hold what a rule concluded. The first
//! divergence is gone with the second implementation; the second is not a property of the
//! calculus at all but of the IR the answer is materialized into, and it surfaces as a
//! [`Boundary`](crate::Boundary) on the run that met it.
//!
//! Because the declaration is data, adding a rule to the chase changes the digest, which
//! is the property a cache consumer actually needs: no false negatives.

use purrdf_datalog::cache::{ContractHash, contract_hash};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};

use crate::Regime;
use crate::rules::RuleId;
use crate::vocab::{
    OWL_EQUIVALENTCLASS, OWL_EQUIVALENTPROPERTY, OWL_INVERSEOF, OWL_SYMMETRICPROPERTY,
    OWL_TRANSITIVEPROPERTY, RDF_PROPERTY, RDF_TYPE, RDFS_CLASS, RDFS_DOMAIN, RDFS_RANGE,
    RDFS_RESOURCE, RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF,
};

/// One rule of the forward chase, named once for the whole crate.
///
/// Variants are declared in specification order — the RDF pattern of RDF 1.2 Semantics
/// §8.1.1, then the RDFS patterns of §9.2.1 in numeric order, then the OWL 2 RL rules of
/// Tables 4–9 that only the `OWL-RL` lane fires, in table order. [`ChaseRule::ALL`] and
/// hence [`calculus_program`] follow that order, so both are byte-for-byte reproducible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ChaseRule {
    /// `rdfD2` — every predicate is an `rdf:Property`. The bare-`RDF` lane only.
    PredicateProperty,
    /// `rdfs2` / `prp-dom` — a domain declaration types the subject.
    Domain,
    /// `rdfs3` / `prp-rng` — a range declaration types the object.
    Range,
    /// `rdfs5` / `scm-spo` — `rdfs:subPropertyOf` is transitive.
    SubPropertyTransitive,
    /// `rdfs6` — a property is a sub-property of itself.
    SubPropertyReflexive,
    /// `rdfs7` / `prp-spo1` — a sub-property assertion re-predicates a triple.
    SubPropertyRewrite,
    /// `rdfs8` — a class is a sub-class of `rdfs:Resource`.
    ClassResource,
    /// `rdfs9` / `cax-sco` — a sub-class assertion re-types an instance.
    SubClassInstance,
    /// `rdfs10` — a class is a sub-class of itself.
    SubClassReflexive,
    /// `rdfs11` / `scm-sco` — `rdfs:subClassOf` is transitive.
    SubClassTransitive,
    /// `prp-symp` — a symmetric property mirrors its triples. `OWL-RL` only.
    Symmetric,
    /// `prp-trp` — a transitive property composes its triples. `OWL-RL` only.
    Transitive,
    /// `prp-inv1` — an `owl:inverseOf` assertion, read left to right. `OWL-RL` only.
    Inverse1,
    /// `prp-inv2` — an `owl:inverseOf` assertion, read right to left. `OWL-RL` only.
    Inverse2,
    /// `scm-eqc1` — `owl:equivalentClass` is mutual `rdfs:subClassOf`. `OWL-RL` only.
    EquivalentClass,
    /// `scm-eqp1` — `owl:equivalentProperty` is mutual `rdfs:subPropertyOf`. `OWL-RL`
    /// only.
    EquivalentProperty,
}

impl ChaseRule {
    /// Every chase rule, in the declaration order documented on the enum.
    pub(crate) const ALL: [Self; 16] = [
        Self::PredicateProperty,
        Self::Domain,
        Self::Range,
        Self::SubPropertyTransitive,
        Self::SubPropertyReflexive,
        Self::SubPropertyRewrite,
        Self::ClassResource,
        Self::SubClassInstance,
        Self::SubClassReflexive,
        Self::SubClassTransitive,
        Self::Symmetric,
        Self::Transitive,
        Self::Inverse1,
        Self::Inverse2,
        Self::EquivalentClass,
        Self::EquivalentProperty,
    ];

    /// How many chase rules there are — the width of a per-rule firing tally.
    pub(crate) const COUNT: usize = Self::ALL.len();

    /// This rule's index into a [`Self::COUNT`]-wide tally.
    pub(crate) const fn index(self) -> usize {
        self as usize
    }

    /// The specification rule id a firing is REPORTED under.
    ///
    /// `owl` selects the lane, exactly as it does in the chase: the OWL 2 RL tables give
    /// nine of these rules a different name from the RDFS tables (`rdfs2` is `prp-dom`,
    /// `rdfs11` is `scm-sco`, …), and a report must use the name of the calculus it ran.
    ///
    /// Three rules — `rdfs6`, `rdfs8` and `rdfs10` — have NO OWL 2 RL rule id, because
    /// OWL 2 RL/RDF omits them from its tables. The `OWL-RL` lane fires them all the same,
    /// so they are reported under their RDFS name rather than renamed to a neighbouring
    /// OWL rule that would not have licensed the conclusion. A consequence worth stating
    /// plainly: an `OWL-RL` report's `rules_fired` is NOT a subset of `rules(OwlRl)`.
    pub(crate) const fn rule_id(self, owl: bool) -> RuleId {
        match self {
            Self::PredicateProperty => RuleId::RdfD2,
            Self::Domain if owl => RuleId::PrpDom,
            Self::Domain => RuleId::Rdfs2,
            Self::Range if owl => RuleId::PrpRng,
            Self::Range => RuleId::Rdfs3,
            Self::SubPropertyTransitive if owl => RuleId::ScmSpo,
            Self::SubPropertyTransitive => RuleId::Rdfs5,
            Self::SubPropertyReflexive => RuleId::Rdfs6,
            Self::SubPropertyRewrite if owl => RuleId::PrpSpo1,
            Self::SubPropertyRewrite => RuleId::Rdfs7,
            Self::ClassResource => RuleId::Rdfs8,
            Self::SubClassInstance if owl => RuleId::CaxSco,
            Self::SubClassInstance => RuleId::Rdfs9,
            Self::SubClassReflexive => RuleId::Rdfs10,
            Self::SubClassTransitive if owl => RuleId::ScmSco,
            Self::SubClassTransitive => RuleId::Rdfs11,
            Self::Symmetric => RuleId::PrpSymp,
            Self::Transitive => RuleId::PrpTrp,
            Self::Inverse1 => RuleId::PrpInv1,
            Self::Inverse2 => RuleId::PrpInv2,
            Self::EquivalentClass => RuleId::ScmEqc1,
            Self::EquivalentProperty => RuleId::ScmEqp1,
        }
    }

    /// Whether `regime`'s lane fires this rule.
    ///
    /// `Simple` fires nothing (it is the identity closure); `RDF` fires the single
    /// predicate-typing rule; `RDFS` fires the nine RDFS patterns; `OWL-RL` fires those
    /// nine plus six OWL rules. `OWL-Direct`, `RIF` and `D` are not this chase's lanes at
    /// all.
    pub(crate) const fn fires_under(self, regime: Regime) -> bool {
        match regime {
            Regime::Rdf => matches!(self, Self::PredicateProperty),
            Regime::Rdfs => !matches!(
                self,
                Self::PredicateProperty
                    | Self::Symmetric
                    | Self::Transitive
                    | Self::Inverse1
                    | Self::Inverse2
                    | Self::EquivalentClass
                    | Self::EquivalentProperty
            ),
            Regime::OwlRl => !matches!(self, Self::PredicateProperty),
            Regime::Simple | Regime::OwlDirect | Regime::Rif | Regime::D => false,
        }
    }
}

/// A variable clause term.
fn var(name: &str) -> ClauseTerm {
    ClauseTerm::var(name)
}

/// A constant-IRI clause term.
fn iri(value: &str) -> ClauseTerm {
    ClauseTerm::iri(value)
}

/// `T(subject, predicate, object)` in the default graph, with a VARIABLE predicate.
fn quad(subject: ClauseTerm, predicate: ClauseTerm, object: ClauseTerm) -> ClauseAtom {
    ClauseAtom::quad(subject, predicate, object, ClauseTerm::DefaultGraph)
}

/// `T(subject, <predicate>, object)` in the default graph, with a CONSTANT predicate.
fn atom(subject: ClauseTerm, predicate: &str, object: ClauseTerm) -> ClauseAtom {
    ClauseAtom::positive(subject, predicate, object)
}

/// The DL clauses that state — and, through [`crate::engine`], run — `rule`.
///
/// Most rules are one clause. `scm-eqc1` and `scm-eqp1` are two each: their specification
/// conclusion is a conjunction of two triples, and a conjunctive head is not a Datalog
/// clause, so each direction is stated separately rather than encoded in a head form the
/// evaluator refuses.
///
/// The lane is not a parameter. Nine of these rules carry two specification NAMES — one
/// RDFS, one OWL 2 RL — but the clause each names is the same clause, so the lane is read
/// by [`ChaseRule::rule_id`] and by [`ChaseRule::fires_under`] and by nothing else.
fn clauses_for(rule: ChaseRule) -> Vec<DlClause> {
    let (s, p, o) = (var("?s"), var("?p"), var("?o"));
    match rule {
        // rdfD2: T(?s, ?p, ?o) ⇒ ?p rdf:type rdf:Property.
        ChaseRule::PredicateProperty => vec![DlClause::datalog(
            atom(p, RDF_TYPE, iri(RDF_PROPERTY)),
            vec![quad(s, var("?p"), o)],
        )],
        // rdfs2 / prp-dom: ?p rdfs:domain ?c, T(?x, ?p, ?y) ⇒ ?x rdf:type ?c.
        ChaseRule::Domain => vec![DlClause::datalog(
            atom(var("?x"), RDF_TYPE, var("?c")),
            vec![
                atom(var("?p"), RDFS_DOMAIN, var("?c")),
                quad(var("?x"), var("?p"), var("?y")),
            ],
        )],
        // rdfs3 / prp-rng: ?p rdfs:range ?c, T(?x, ?p, ?y) ⇒ ?y rdf:type ?c.
        ChaseRule::Range => vec![DlClause::datalog(
            atom(var("?y"), RDF_TYPE, var("?c")),
            vec![
                atom(var("?p"), RDFS_RANGE, var("?c")),
                quad(var("?x"), var("?p"), var("?y")),
            ],
        )],
        // rdfs5 / scm-spo: subPropertyOf is transitive.
        ChaseRule::SubPropertyTransitive => vec![DlClause::datalog(
            atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p3")),
            vec![
                atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
                atom(var("?p2"), RDFS_SUBPROPERTYOF, var("?p3")),
            ],
        )],
        // rdfs6: ?p rdf:type rdf:Property ⇒ ?p rdfs:subPropertyOf ?p.
        ChaseRule::SubPropertyReflexive => vec![DlClause::datalog(
            atom(var("?p"), RDFS_SUBPROPERTYOF, var("?p")),
            vec![atom(var("?p"), RDF_TYPE, iri(RDF_PROPERTY))],
        )],
        // rdfs7 / prp-spo1: ?p1 subPropertyOf ?p2, T(?x, ?p1, ?y) ⇒ T(?x, ?p2, ?y).
        ChaseRule::SubPropertyRewrite => vec![DlClause::datalog(
            quad(var("?x"), var("?p2"), var("?y")),
            vec![
                atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
                quad(var("?x"), var("?p1"), var("?y")),
            ],
        )],
        // rdfs8: ?c rdf:type rdfs:Class ⇒ ?c rdfs:subClassOf rdfs:Resource.
        ChaseRule::ClassResource => vec![DlClause::datalog(
            atom(var("?c"), RDFS_SUBCLASSOF, iri(RDFS_RESOURCE)),
            vec![atom(var("?c"), RDF_TYPE, iri(RDFS_CLASS))],
        )],
        // rdfs9 / cax-sco: ?c1 subClassOf ?c2, ?x rdf:type ?c1 ⇒ ?x rdf:type ?c2.
        ChaseRule::SubClassInstance => vec![DlClause::datalog(
            atom(var("?x"), RDF_TYPE, var("?c2")),
            vec![
                atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
                atom(var("?x"), RDF_TYPE, var("?c1")),
            ],
        )],
        // rdfs10: ?c rdf:type rdfs:Class ⇒ ?c rdfs:subClassOf ?c.
        ChaseRule::SubClassReflexive => vec![DlClause::datalog(
            atom(var("?c"), RDFS_SUBCLASSOF, var("?c")),
            vec![atom(var("?c"), RDF_TYPE, iri(RDFS_CLASS))],
        )],
        // rdfs11 / scm-sco: subClassOf is transitive.
        ChaseRule::SubClassTransitive => vec![DlClause::datalog(
            atom(var("?c1"), RDFS_SUBCLASSOF, var("?c3")),
            vec![
                atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
                atom(var("?c2"), RDFS_SUBCLASSOF, var("?c3")),
            ],
        )],
        // prp-symp: ?p a owl:SymmetricProperty, T(?x, ?p, ?y) ⇒ T(?y, ?p, ?x).
        ChaseRule::Symmetric => vec![DlClause::datalog(
            quad(var("?y"), var("?p"), var("?x")),
            vec![
                atom(var("?p"), RDF_TYPE, iri(OWL_SYMMETRICPROPERTY)),
                quad(var("?x"), var("?p"), var("?y")),
            ],
        )],
        // prp-trp: ?p a owl:TransitiveProperty, T(?x,?p,?y), T(?y,?p,?z) ⇒ T(?x,?p,?z).
        ChaseRule::Transitive => vec![DlClause::datalog(
            quad(var("?x"), var("?p"), var("?z")),
            vec![
                atom(var("?p"), RDF_TYPE, iri(OWL_TRANSITIVEPROPERTY)),
                quad(var("?x"), var("?p"), var("?y")),
                quad(var("?y"), var("?p"), var("?z")),
            ],
        )],
        // prp-inv1: ?p1 owl:inverseOf ?p2, T(?x, ?p1, ?y) ⇒ T(?y, ?p2, ?x).
        ChaseRule::Inverse1 => vec![DlClause::datalog(
            quad(var("?y"), var("?p2"), var("?x")),
            vec![
                atom(var("?p1"), OWL_INVERSEOF, var("?p2")),
                quad(var("?x"), var("?p1"), var("?y")),
            ],
        )],
        // prp-inv2: ?p1 owl:inverseOf ?p2, T(?x, ?p2, ?y) ⇒ T(?y, ?p1, ?x).
        ChaseRule::Inverse2 => vec![DlClause::datalog(
            quad(var("?y"), var("?p1"), var("?x")),
            vec![
                atom(var("?p1"), OWL_INVERSEOF, var("?p2")),
                quad(var("?x"), var("?p2"), var("?y")),
            ],
        )],
        // scm-eqc1: equivalentClass ⇒ subClassOf, both directions.
        ChaseRule::EquivalentClass => vec![
            DlClause::datalog(
                atom(var("?c1"), RDFS_SUBCLASSOF, var("?c2")),
                vec![atom(var("?c1"), OWL_EQUIVALENTCLASS, var("?c2"))],
            ),
            DlClause::datalog(
                atom(var("?c2"), RDFS_SUBCLASSOF, var("?c1")),
                vec![atom(var("?c1"), OWL_EQUIVALENTCLASS, var("?c2"))],
            ),
        ],
        // scm-eqp1: equivalentProperty ⇒ subPropertyOf, both directions.
        ChaseRule::EquivalentProperty => vec![
            DlClause::datalog(
                atom(var("?p1"), RDFS_SUBPROPERTYOF, var("?p2")),
                vec![atom(var("?p1"), OWL_EQUIVALENTPROPERTY, var("?p2"))],
            ),
            DlClause::datalog(
                atom(var("?p2"), RDFS_SUBPROPERTYOF, var("?p1")),
                vec![atom(var("?p1"), OWL_EQUIVALENTPROPERTY, var("?p2"))],
            ),
        ],
    }
}

/// The DL-clause program that STATES — and RUNS — `regime`'s calculus, in a fixed order.
///
/// # What it is
///
/// It is the calculus's IDENTITY. Every rule this crate's forward chase fires under
/// `regime` is rendered here as a [`DlClause`] over specification `rdf:`/`rdfs:`/`owl:`
/// IRIs — PurRDF mints none of its own — so hashing it with
/// `purrdf_datalog::cache::contract_hash` answers "which rule set was this closure minted
/// under?" with a digest instead of a sentence. That digest is what
/// [`ReasoningReport::contract_hash`](crate::ReasoningReport::contract_hash) carries, and
/// adding a rule to the chase moves it, which is the one property a cache consumer needs.
///
/// It is also the chase's SOURCE: [`materialize`](crate::materialize) evaluates exactly
/// this program through `purrdf-datalog`'s semi-naive evaluator, so there is no second
/// statement of the calculus for the digest to be right about and the run to be wrong
/// about.
///
/// The one thing the program does not decide is what the RDF 1.2 dataset IR can HOLD. A
/// rule that concludes into subject or predicate position can reach a term RDF 1.2 does
/// not admit there — a literal subject, above all — and such a conclusion is a
/// generalized-RDF triple that is derived in the evaluator's own term space and then
/// abandoned at the materialization boundary rather than fabricated around. That is
/// reported as a [`Boundary`](crate::Boundary) on the run that met it, with
/// [`Construct::GeneralizedRdf`](crate::Construct::GeneralizedRdf) as the reason.
///
/// # Order
///
/// Clauses appear in the chase's own rule-declaration order — the RDF pattern of RDF 1.2
/// Semantics §8.1.1, then the RDFS patterns of §9.2.1 in numeric order, then the OWL 2 RL
/// rules of Tables 4–9 that only the `OWL-RL` lane fires, in table order — restricted to
/// the rules `regime`'s lane fires. `scm-eqc1` and `scm-eqp1` take two clauses each (their
/// specification conclusion is a conjunction, which is not a Datalog head), contributing
/// their two directions in the order the specification writes them. The result is a pure
/// function of `regime`: no map iteration, no hashing, no allocation order reaches it.
///
/// `Simple` yields the empty program: the identity closure has no rules, and that is a
/// statement about the calculus, not an omission. `OWL-Direct`, `RIF` and `D` yield the
/// empty program too, because none of them is defined by a fixed clause table this crate
/// can enumerate — a tableau, a caller-supplied rule set and a datatype map respectively.
///
/// ```
/// use purrdf_entail::{Regime, calculus_program};
///
/// assert!(calculus_program(Regime::Simple).is_empty());
/// // Nine RDFS patterns, one clause each.
/// assert_eq!(calculus_program(Regime::Rdfs).len(), 9);
/// // Those nine plus six OWL rules, two of which take two clauses.
/// assert_eq!(calculus_program(Regime::OwlRl).len(), 17);
/// ```
#[must_use]
pub fn calculus_program(regime: Regime) -> Vec<DlClause> {
    program_with_attribution(regime).0
}

/// `regime`'s program, paired with the [`ChaseRule`] each clause states.
///
/// A [`Derivation`](purrdf_datalog::seminaive::Derivation) names its producing clause by
/// authored index, so attributing a firing means asking which rule authored clause `i` —
/// and the only honest way to answer is to build the answer in the SAME walk that builds
/// the program. Two rules contribute two clauses each (`scm-eqc1` and `scm-eqp1`), so the
/// map is not the identity and cannot be reconstructed from a rule count.
///
/// [`calculus_program`] is this function's first element, so the published program and the
/// attribution can never be out of step.
pub(crate) fn program_with_attribution(regime: Regime) -> (Vec<DlClause>, Vec<ChaseRule>) {
    let mut clauses = Vec::new();
    let mut attribution = Vec::new();
    for rule in ChaseRule::ALL {
        if !rule.fires_under(regime) {
            continue;
        }
        for clause in clauses_for(rule) {
            clauses.push(clause);
            attribution.push(rule);
        }
    }
    (clauses, attribution)
}

/// The identity of the calculus `regime` runs, as `purrdf-datalog` computes it.
///
/// Exactly `purrdf_datalog::cache::contract_hash(&calculus_program(regime))` — the same
/// digest a consumer gets by recomputing it, which is what makes the value in a
/// [`ReasoningReport`](crate::ReasoningReport) checkable rather than merely present.
pub(crate) fn calculus_contract_hash(regime: Regime) -> ContractHash {
    contract_hash(&calculus_program(regime))
}

/// The rule ids `regime`'s lane fires, in [`ChaseRule::ALL`] order, deduplicated.
///
/// Used by the tests to bind the chase's tags to [`crate::implemented`].
#[cfg(test)]
fn fired_rule_ids(regime: Regime) -> Vec<RuleId> {
    let owl = matches!(regime, Regime::OwlRl);
    let mut ids: Vec<RuleId> = ChaseRule::ALL
        .into_iter()
        .filter(|rule| rule.fires_under(regime))
        .map(|rule| rule.rule_id(owl))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// The rules the `OWL-RL` lane fires that OWL 2 RL/RDF gives no rule id.
///
/// RDF 1.2 Semantics §9.2.1 names them; OWL 2 Profiles §4.3 Tables 4–9 do not, because
/// OWL 2 RL/RDF deliberately omits "most, but not all" of the RDFS rules. They are
/// therefore absent from `rules(Regime::OwlRl)` while being present in an `OWL-RL`
/// report's fired list, and this constant is what the tests pin that against.
#[cfg(test)]
pub(crate) const OWL_RL_RDFS_SHAPED_EXTRAS: [RuleId; 3] =
    [RuleId::Rdfs6, RuleId::Rdfs8, RuleId::Rdfs10];

/// Whether `id` is a rule the report may name for `regime` even though
/// `rules(regime)` does not list it.
#[cfg(test)]
pub(crate) fn is_rdfs_shaped_extra(regime: Regime, id: RuleId) -> bool {
    matches!(regime, Regime::OwlRl) && OWL_RL_RDFS_SHAPED_EXTRAS.contains(&id)
}

/// Every regime, for the cross-cutting checks below and in [`crate::report`].
#[cfg(test)]
pub(crate) const ALL_REGIMES: [Regime; 7] = [
    Regime::Simple,
    Regime::Rdf,
    Regime::Rdfs,
    Regime::OwlRl,
    Regime::OwlDirect,
    Regime::Rif,
    Regime::D,
];

#[cfg(test)]
mod tests {
    use super::{
        ALL_REGIMES, ChaseRule, OWL_RL_RDFS_SHAPED_EXTRAS, calculus_contract_hash,
        calculus_program, fired_rule_ids, program_with_attribution,
    };
    use crate::{Regime, RuleId, implemented, rules};
    use purrdf_datalog::cache::contract_hash;
    use purrdf_datalog::seminaive::compile;
    use std::collections::BTreeSet;

    /// The chase's tags ARE the inventory: for every regime, the rule ids the lane fires
    /// equal `implemented(regime)` plus, for `OWL-RL` only, the three RDFS-shaped rules
    /// that lane fires under no OWL 2 RL name.
    ///
    /// This is the join that keeps a firing tally from claiming a rule the inventory does
    /// not, or the inventory from claiming a rule that never fires.
    #[test]
    fn the_fired_rule_ids_are_exactly_the_inventory() {
        for regime in ALL_REGIMES {
            let fired: BTreeSet<RuleId> = fired_rule_ids(regime).into_iter().collect();
            let mut expected: BTreeSet<RuleId> = implemented(regime).iter().copied().collect();
            if matches!(regime, Regime::OwlRl) {
                expected.extend(OWL_RL_RDFS_SHAPED_EXTRAS);
            }
            assert_eq!(fired, expected, "{regime:?}");
        }
    }

    /// The three extras really are absent from the OWL 2 RL rule table — the reason they
    /// cannot be reported under an OWL name.
    #[test]
    fn the_owl_rl_extras_have_no_owl_rule_id() {
        let owl: BTreeSet<RuleId> = rules(Regime::OwlRl).iter().copied().collect();
        for extra in OWL_RL_RDFS_SHAPED_EXTRAS {
            assert!(!owl.contains(&extra), "{extra} is in the OWL 2 RL tables");
            assert!(rules(Regime::Rdfs).contains(&extra), "{extra} is RDFS");
        }
    }

    /// Every declared program is a well-formed, admissible Datalog program.
    ///
    /// A declaration the evaluator would refuse is not a statement of a calculus, it is a
    /// typo; compiling it is the cheapest possible proof that it is neither non-Datalog in
    /// its head form nor unsafe in its range restriction.
    #[test]
    fn every_declared_program_compiles() {
        for regime in ALL_REGIMES {
            let program = calculus_program(regime);
            assert!(
                compile(program).is_ok(),
                "{regime:?}'s declared program must be admissible"
            );
        }
    }

    /// The program is a pure function of the regime, and its size is pinned.
    #[test]
    fn the_program_is_deterministic_and_pinned() {
        for regime in ALL_REGIMES {
            assert_eq!(
                format!("{:?}", calculus_program(regime)),
                format!("{:?}", calculus_program(regime)),
                "{regime:?}"
            );
        }
        let sizes: Vec<(Regime, usize)> = ALL_REGIMES
            .iter()
            .map(|&r| (r, calculus_program(r).len()))
            .collect();
        assert_eq!(
            sizes,
            vec![
                (Regime::Simple, 0),
                (Regime::Rdf, 1),
                (Regime::Rdfs, 9),
                (Regime::OwlRl, 17),
                (Regime::OwlDirect, 0),
                (Regime::Rif, 0),
                (Regime::D, 0),
            ]
        );
    }

    /// The contract hash is `purrdf-datalog`'s, over the declared program — not a second
    /// recipe that could drift from it.
    #[test]
    fn the_contract_hash_is_datalogs_over_the_declared_program() {
        for regime in ALL_REGIMES {
            assert_eq!(
                calculus_contract_hash(regime),
                contract_hash(&calculus_program(regime)),
                "{regime:?}"
            );
        }
    }

    /// Two lanes with different rule sets have different calculus identities, and the
    /// three rule-free regimes share the empty program's identity.
    #[test]
    fn different_rule_sets_have_different_identities() {
        let rdf = calculus_contract_hash(Regime::Rdf);
        let rdfs = calculus_contract_hash(Regime::Rdfs);
        let owl = calculus_contract_hash(Regime::OwlRl);
        assert_ne!(rdf, rdfs);
        assert_ne!(rdfs, owl);
        assert_ne!(rdf, owl);
        // No rules is itself a calculus, and all four rule-free regimes state it.
        let empty = calculus_contract_hash(Regime::Simple);
        for regime in [Regime::OwlDirect, Regime::Rif, Regime::D] {
            assert_eq!(calculus_contract_hash(regime), empty, "{regime:?}");
        }
        assert_ne!(empty, rdf);
    }

    /// The lane membership is a partition the enum cannot silently widen: `RDFS` fires
    /// nine rules, `OWL-RL` those nine plus six, `RDF` one, and nothing else fires at all.
    #[test]
    fn lane_membership_is_pinned() {
        let count = |regime: Regime| {
            ChaseRule::ALL
                .into_iter()
                .filter(|rule| rule.fires_under(regime))
                .count()
        };
        assert_eq!(count(Regime::Rdf), 1);
        assert_eq!(count(Regime::Rdfs), 9);
        assert_eq!(count(Regime::OwlRl), 15);
        for regime in [Regime::Simple, Regime::OwlDirect, Regime::Rif, Regime::D] {
            assert_eq!(count(regime), 0, "{regime:?}");
        }
        // Every RDFS rule is also an OWL-RL rule of this chase; the lane only adds.
        for rule in ChaseRule::ALL {
            assert!(
                !rule.fires_under(Regime::Rdfs) || rule.fires_under(Regime::OwlRl),
                "{rule:?} fires under RDFS but not OWL-RL"
            );
        }
    }

    /// The attribution is exactly as long as the program, is the published program's own
    /// clause list, and credits every rule the lane fires — including the two rules that
    /// contribute two clauses each, which is what makes the map non-trivial.
    #[test]
    fn the_attribution_indexes_the_published_program() {
        for regime in ALL_REGIMES {
            let (program, attribution) = program_with_attribution(regime);
            assert_eq!(program, calculus_program(regime), "{regime:?}");
            assert_eq!(program.len(), attribution.len(), "{regime:?}");
            let credited: BTreeSet<ChaseRule> = attribution.iter().copied().collect();
            let firing: BTreeSet<ChaseRule> = ChaseRule::ALL
                .into_iter()
                .filter(|rule| rule.fires_under(regime))
                .collect();
            assert_eq!(credited, firing, "{regime:?}");
        }
        // The two conjunctive-conclusion rules are stated as two clauses each, so the
        // attribution names them twice and a clause index is NOT a rule index.
        let (_, owl) = program_with_attribution(Regime::OwlRl);
        for rule in [ChaseRule::EquivalentClass, ChaseRule::EquivalentProperty] {
            assert_eq!(
                owl.iter().filter(|&&r| r == rule).count(),
                2,
                "{rule:?} states two clauses"
            );
        }
    }

    /// `index` is a dense, collision-free tally slot for every rule.
    #[test]
    fn indices_are_dense_and_unique() {
        let indices: Vec<usize> = ChaseRule::ALL.iter().map(|r| r.index()).collect();
        assert_eq!(indices, (0..ChaseRule::COUNT).collect::<Vec<_>>());
    }
}
