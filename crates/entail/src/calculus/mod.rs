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
//!
//! # One module per rule family
//!
//! The rules themselves are NOT written here. Each specification rule family owns a module
//! and states its own rules — the variant, its documentation, the name a firing is reported
//! under, the lanes that fire it, and the DL clauses that say what it concludes:
//!
//! * [`rdfs`] — the RDF pattern of RDF 1.2 Semantics §8.1.1 and the RDFS patterns of
//!   §9.2.1, including the nine the OWL 2 RL tables rename;
//! * [`prp`] — OWL 2 Profiles §4.3 Table 5, the property-axiom rules with no RDFS
//!   counterpart;
//! * [`cls`] — Table 6, the class-expression rules;
//! * [`cax`] — Table 7, the class-axiom rules with no RDFS counterpart;
//! * [`scm`] — Table 9, the schema-vocabulary rules with no RDFS counterpart.
//!
//! This module concatenates whatever those five export, in the fixed family order `rdfs`,
//! `prp`, `cls`, `cax`, `scm` — RDF/RDFS first, then the OWL 2 RL tables in table order —
//! and turns the result into [`ChaseRule`], its inventory bindings and the clause program.
//! Adding a rule is therefore an edit to ONE family module: nothing here names an
//! individual rule, so two families can grow at once without touching the same lines.
//!
//! An empty family table is a statement, not an omission: it says this crate's chase
//! implements none of that table yet, and the family module's documentation names the
//! rules it will hold when it does.

use purrdf_datalog::cache::{ContractHash, contract_hash};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};

use crate::Regime;
use crate::rules::RuleId;

pub(crate) mod cax;
pub(crate) mod cls;
pub(crate) mod prp;
pub(crate) mod rdfs;
pub(crate) mod scm;

use cax::cax_rules;
use cls::cls_rules;
use prp::prp_rules;
use rdfs::rdfs_rules;
use scm::scm_rules;

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

/// One entry of the accumulated rule table, counted as `1`, so the table sizes itself and
/// no family can get the width of a firing tally wrong.
macro_rules! one_rule {
    ($variant:ident) => {
        1
    };
}

/// The rule id a firing of one rule is REPORTED under, given the lane.
///
/// Two forms, because the specifications name these rules twice or once.
/// `reported_id!(owl, Rdfs2, PrpDom)` is a rule the OWL 2 RL tables RENAME: the
/// digit-numbered RDFS name outside the `OWL-RL` lane, the OWL name inside it.
/// `reported_id!(owl, PrpSymp)` is a rule with a single name in every lane that fires it —
/// either an OWL 2 RL rule the RDFS tables never had, or an RDFS pattern OWL 2 RL/RDF omits
/// from its own tables and which is therefore reported under its RDFS name even when the
/// `OWL-RL` lane fires it.
macro_rules! reported_id {
    ($owl:ident, $id:ident) => {
        RuleId::$id
    };
    ($owl:ident, $id:ident, $renamed:ident) => {
        if $owl { RuleId::$renamed } else { RuleId::$id }
    };
}

/// State the calculus from the concatenated family tables.
///
/// Every per-rule fact the crate needs is derived here from ONE declaration of that rule,
/// so a variant, the tag its firings are counted under, the lanes that fire it and the
/// clauses that state it cannot drift apart. Each entry reads
///
/// ```text
/// /// `prp-symp` — what the rule says, in one line.
/// Symmetric {
///     id: PrpSymp,             // the RuleId a firing is reported under
///     owl: PrpSymp,            // OPTIONAL: the OWL 2 RL tables' different name for it
///     lanes: [OwlRl],          // the Regimes whose lane fires it
///     clauses: prp::symmetric, // the fn stating it as DL clauses
/// }
/// ```
///
/// and the entries arrive in the order the families are concatenated, which is the order
/// the program is authored in and therefore the order its digest is taken over.
macro_rules! declare_chase_rules {
    (
        $(
            $(#[$attr:meta])*
            $variant:ident {
                id: $id:ident,
                $( owl: $owl:ident, )?
                lanes: [ $( $lane:ident ),+ ],
                clauses: $clauses:path,
            }
        ),* $(,)?
    ) => {
        /// One rule of the forward chase, named once for the whole crate.
        ///
        /// Variants are declared in specification order — the RDF pattern of RDF 1.2
        /// Semantics §8.1.1, then the RDFS patterns of §9.2.1 in numeric order, then the
        /// OWL 2 RL rules of Tables 4–9 that only the `OWL-RL` lane fires, in table order.
        /// [`ChaseRule::ALL`] and hence [`calculus_program`] follow that order, so both are
        /// byte-for-byte reproducible.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) enum ChaseRule {
            $( $(#[$attr])* $variant, )*
        }

        impl ChaseRule {
            /// How many chase rules there are — the width of a per-rule firing tally.
            pub(crate) const COUNT: usize = 0 $( + one_rule!($variant) )*;

            /// Every chase rule, in the declaration order documented on the enum.
            pub(crate) const ALL: [Self; Self::COUNT] = [ $( Self::$variant, )* ];

            /// This rule's index into a [`Self::COUNT`]-wide tally.
            pub(crate) const fn index(self) -> usize {
                self as usize
            }

            /// The specification rule id a firing is REPORTED under.
            ///
            /// `owl` selects the lane, exactly as it does in the chase: the OWL 2 RL tables
            /// give nine of these rules a different name from the RDFS tables (`rdfs2` is
            /// `prp-dom`, `rdfs11` is `scm-sco`, …), and a report must use the name of the
            /// calculus it ran.
            ///
            /// Three rules — `rdfs6`, `rdfs8` and `rdfs10` — have NO OWL 2 RL rule id,
            /// because OWL 2 RL/RDF omits them from its tables. The `OWL-RL` lane fires them
            /// all the same, so they are reported under their RDFS name rather than renamed
            /// to a neighbouring OWL rule that would not have licensed the conclusion. A
            /// consequence worth stating plainly: an `OWL-RL` report's `rules_fired` is NOT
            /// a subset of `rules(OwlRl)`.
            pub(crate) const fn rule_id(self, owl: bool) -> RuleId {
                match self {
                    $( Self::$variant => reported_id!(owl, $id $(, $owl)?), )*
                }
            }

            /// Whether `regime`'s lane fires this rule.
            ///
            /// `Simple` fires nothing (it is the identity closure); `RDF` fires the single
            /// predicate-typing rule; `RDFS` fires that one plus thirteen RDFS patterns;
            /// `OWL-RL` fires nine of those thirteen plus six OWL rules. `OWL-Direct`,
            /// `RIF` and `D` are not this chase's lanes at all, and no rule names them.
            pub(crate) const fn fires_under(self, regime: Regime) -> bool {
                match self {
                    $( Self::$variant => matches!(regime, $( Regime::$lane )|+), )*
                }
            }
        }

        /// The DL clauses that state — and, through [`crate::engine`], run — `rule`.
        ///
        /// Most rules are one clause. `scm-eqc1` and `scm-eqp1` are two each: their
        /// specification conclusion is a conjunction of two triples, and a conjunctive head
        /// is not a Datalog clause, so each direction is stated separately rather than
        /// encoded in a head form the evaluator refuses.
        ///
        /// The lane is not a parameter. Nine of these rules carry two specification NAMES —
        /// one RDFS, one OWL 2 RL — but the clause each names is the same clause, so the
        /// lane is read by [`ChaseRule::rule_id`] and by [`ChaseRule::fires_under`] and by
        /// nothing else.
        fn clauses_for(rule: ChaseRule) -> Vec<DlClause> {
            match rule {
                $( ChaseRule::$variant => $clauses(), )*
            }
        }
    };
}

/// Ask each family in turn for its rules, then state the calculus from all of them.
///
/// A family module cannot splice variants into an enum declared here, so it hands its table
/// over instead: this macro walks the family list, invokes each family's own macro with
/// itself as the continuation, and accumulates what comes back. When the list is empty every
/// family has contributed, and the accumulated table is declared, in family order, exactly
/// once.
///
/// The consequence is the point of the split: the family list below is the only thing this
/// module knows about which rules exist, and it names families, not rules.
macro_rules! collect_families {
    // Every family has contributed: state the calculus.
    ({} $($rules:tt)*) => {
        declare_chase_rules! { $($rules)* }
    };
    // Ask the next family for its rules, then carry on with the rest.
    ({ $family:ident $(, $rest:ident)* } $($rules:tt)*) => {
        $family! { collect_families, { $($rest),* }, $($rules)* }
    };
}

collect_families! { { rdfs_rules, prp_rules, cls_rules, cax_rules, scm_rules } }

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
/// // Fourteen RDF/RDFS patterns; `rdfs1` and `rdfs4` take three clauses each.
/// assert_eq!(calculus_program(Regime::Rdfs).len(), 18);
/// // Nine of those RDFS patterns plus six OWL rules, two of which take two clauses.
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
                (Regime::Rdfs, 18),
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

    /// Each lane's calculus identity, pinned byte for byte.
    ///
    /// The contract hash is a PUBLISHED identity: a consumer stores it beside a cached
    /// closure and refuses the closure when it moves. So it may only move when the rules
    /// move. Splitting the declaration across family modules, reordering a family's
    /// internals, or rewording a clause without changing what it concludes are all edits
    /// that must leave these digests exactly where they are — only adding, removing or
    /// restating a RULE may move one, deliberately, with this table updated in the same
    /// commit.
    #[test]
    fn the_contract_hashes_are_pinned() {
        let empty = "4151090ce6c2ecdae843e351420ddcf10f79e525c60b4c6d07bebeabaa07fbd5";
        let pinned = [
            (Regime::Simple, empty),
            (
                Regime::Rdf,
                "e3dfc92e2575713a6a555ef1dc5688d9086fe0a41c8eb0dd27aff5579db33158",
            ),
            (
                Regime::Rdfs,
                "3083bceef6ed2293040a176631681ad0ab5ae1d6a79c4f75327184f60972cc2c",
            ),
            (
                Regime::OwlRl,
                "369fa3fbbe648a4b2c381015990cc3a2f78e606a40102d5feb3f9fabba07ce45",
            ),
            (Regime::OwlDirect, empty),
            (Regime::Rif, empty),
            (Regime::D, empty),
        ];
        for (regime, digest) in pinned {
            assert_eq!(
                calculus_contract_hash(regime).to_hex(),
                digest,
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
    /// fourteen rules, `OWL-RL` nine of them plus six of its own, `RDF` one, and nothing
    /// else fires at all.
    ///
    /// `RDFS` is no longer a subset of `OWL-RL`, and that is deliberate: OWL 2 Profiles
    /// §4.3 omits the RDF and RDFS axiomatic triples, so the five rules the `RDFS` lane
    /// added on top of the shared nine (`rdfD2`, `rdfs1`, `rdfs4`, `rdfs12`, `rdfs13`)
    /// belong to a calculus the `OWL-RL` lane does not run.
    #[test]
    fn lane_membership_is_pinned() {
        let count = |regime: Regime| {
            ChaseRule::ALL
                .into_iter()
                .filter(|rule| rule.fires_under(regime))
                .count()
        };
        assert_eq!(count(Regime::Rdf), 1);
        assert_eq!(count(Regime::Rdfs), 14);
        assert_eq!(count(Regime::OwlRl), 15);
        for regime in [Regime::Simple, Regime::OwlDirect, Regime::Rif, Regime::D] {
            assert_eq!(count(regime), 0, "{regime:?}");
        }
        // The nine rules the two lanes SHARE are named, so a rule silently joining or
        // leaving the overlap fails here rather than in a golden.
        let shared: Vec<RuleId> = ChaseRule::ALL
            .into_iter()
            .filter(|rule| rule.fires_under(Regime::Rdfs) && rule.fires_under(Regime::OwlRl))
            .map(|rule| rule.rule_id(false))
            .collect();
        assert_eq!(
            shared,
            vec![
                RuleId::Rdfs2,
                RuleId::Rdfs3,
                RuleId::Rdfs5,
                RuleId::Rdfs6,
                RuleId::Rdfs7,
                RuleId::Rdfs8,
                RuleId::Rdfs9,
                RuleId::Rdfs10,
                RuleId::Rdfs11,
            ]
        );
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

    /// The RDF/RDFS family is concatenated ahead of the OWL 2 RL families.
    ///
    /// The program's order is the concatenation of the family tables, and the digest is a
    /// function of that order, so the family order is part of the calculus's identity
    /// rather than a filing convention. A rule the RDF or RDFS lane fires comes from
    /// [`rdfs`](super::rdfs) and a rule only `OWL-RL` fires comes from one of the OWL
    /// tables, so the two blocks must not interleave — which is a property of the family
    /// list, not of any rule, and stays true however far a family grows.
    #[test]
    fn the_rdfs_family_is_concatenated_first() {
        let rdf_or_rdfs: Vec<usize> = ChaseRule::ALL
            .into_iter()
            .filter(|rule| rule.fires_under(Regime::Rdf) || rule.fires_under(Regime::Rdfs))
            .map(ChaseRule::index)
            .collect();
        assert_eq!(
            rdf_or_rdfs,
            (0..rdf_or_rdfs.len()).collect::<Vec<_>>(),
            "the RDF/RDFS family occupies a prefix of the program, uninterrupted"
        );
    }
}
