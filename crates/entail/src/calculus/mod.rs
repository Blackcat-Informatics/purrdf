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
//! * [`eq`] — OWL 2 Profiles §4.3 Table 4, the equality rules;
//! * [`prp`] — Table 5, the property-axiom rules with no RDFS counterpart;
//! * [`cls`] — Table 6, the class-expression rules;
//! * [`cax`] — Table 7, the class-axiom rules with no RDFS counterpart;
//! * [`dt`] — Table 8, the datatype rules;
//! * [`scm`] — Table 9, the schema-vocabulary rules with no RDFS counterpart.
//!
//! This module concatenates whatever those seven export, in the fixed family order `rdfs`,
//! `eq`, `prp`, `cls`, `cax`, `dt`, `scm` — RDF/RDFS first, then the OWL 2 RL tables in
//! table order — and turns the result into [`ChaseRule`], its inventory bindings and the
//! clause program. Adding a rule is therefore an edit to ONE family module: nothing here
//! names an individual rule, so two families can grow at once without touching the same
//! lines.
//!
//! # A rule that concludes `false` is LOWERED, not refused
//!
//! Seventeen of the 78 OWL 2 RL rules conclude `false`: a body match is an INCONSISTENCY
//! WITNESS, not a triple. Each is DECLARED with the specification's own body and
//! `HeadForm::Inconsistency`, and [`program_with_attribution`] lowers it — mechanically,
//! by [`constraint_clause`] — into a clause whose head is one atom of the internal
//! [`CLASH_RELATION`](crate::lists::CLASH_RELATION). The evaluator therefore runs the
//! specification's own body, and [`crate::engine`] turns a clash row into
//! [`EntailError::Inconsistent`](crate::EntailError) carrying the matched body facts.
//!
//! # No rule of this calculus is stated with NEGATION
//!
//! Four rules carry an `i ≠ j` side condition and one an inequality of data values, and
//! none of the five is written with negation-as-failure. That is forced rather than
//! stylistic: this program quantifies over the PREDICATE position, so a negated body atom
//! puts its dependency edge inside a cycle through the variable-predicate wildcard and
//! `purrdf-datalog` refuses the whole program as non-stratifiable — correctly, because a
//! variable predicate really can range over the negated relation. Every inequality a rule
//! needs is therefore MATERIALIZED as a positive relation by a pre-pass; see
//! [`crate::lists`] and [`crate::datatypes`] for the two of them and for what each costs.
//!
//! The lowering is a FUNCTION of the declaration, not a second statement of it: there is
//! still exactly one place each rule is written, and `the_lowering_preserves_the_declared_body`
//! asserts the lowered clause's body is the declared clause's body, atom for atom. Nothing
//! is fabricated into the closure, because the head is an internal relation that no
//! materialization step can emit and the run that produced it is refused outright.

use purrdf_datalog::cache::{ContractHash, contract_hash};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause, HeadForm};

use crate::Regime;
use crate::lists::{CLASH_RELATION, INTERNAL_GRAPH, INTERNAL_SIGIL};
use crate::rules::RuleId;

pub(crate) mod cax;
pub(crate) mod cls;
pub(crate) mod dt;
pub(crate) mod eq;
pub(crate) mod prp;
pub(crate) mod rdfs;
pub(crate) mod scm;

use cax::cax_rules;
use cls::cls_rules;
use dt::dt_rules;
use eq::eq_rules;
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

/// An atom of an INTERNAL relation — `relation(first, second, third)`.
///
/// The predicate is an interner-local id rather than an IRI, because PurRDF mints no
/// vocabulary and an internal relation is not vocabulary; see [`crate::lists`] for the
/// three-arguments-in-four-positions convention and for how such an id is kept out of the
/// materialized answer.
fn internal(
    relation: &'static str,
    first: ClauseTerm,
    second: ClauseTerm,
    third: ClauseTerm,
) -> ClauseAtom {
    ClauseAtom::quad(first, ClauseTerm::literal(relation), second, third)
}

/// The graph term a BINARY internal relation's atoms name — see
/// [`INTERNAL_GRAPH`](crate::lists::INTERNAL_GRAPH) for why it is not the default graph.
fn internal_graph() -> ClauseTerm {
    ClauseTerm::literal(INTERNAL_GRAPH)
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

/// The head form this rule DECLARES, when it is not the atomic one.
///
/// A rule declared with `concludes: Inconsistency,` states clauses whose head is `false`.
/// It is lowered by [`constraint_clause`] and evaluated; the marker is what
/// [`crate::engine`] and the tests read to know a firing is a clash rather than a
/// conclusion.
macro_rules! declared_form {
    () => {
        None
    };
    ($form:ident) => {
        Some(HeadForm::$form)
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
///
/// # A rule that concludes `false` says so, and is LOWERED
///
/// One optional field, `concludes:`, names the head form the rule's own clauses carry
/// when it is not the atomic one:
///
/// ```text
/// /// `cax-dw` — two disjoint classes with a shared instance.
/// DisjointWith {
///     id: CaxDw,
///     lanes: [OwlRl],
///     clauses: cax::disjoint_with,
///     concludes: Inconsistency,   // the head is `false`
/// }
/// ```
///
/// Such a rule states its clauses like any other — the specification's own body, the
/// specification's own `false` — and [`program_with_attribution`] lowers it through
/// [`constraint_clause`] into a clause the evaluator runs. See the [module docs](self) for
/// why the lowering fabricates nothing.
macro_rules! declare_chase_rules {
    (
        $(
            $(#[$attr:meta])*
            $variant:ident {
                id: $id:ident,
                $( owl: $owl:ident, )?
                lanes: [ $( $lane:ident ),+ ],
                clauses: $clauses:path,
                $( concludes: $concludes:ident, )?
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
            /// `OWL-RL` fires nine of those thirteen plus the whole of Tables 4–9.
            /// `OWL-Direct` and `RIF` are not this chase's lanes at all, and no rule names
            /// them; `D` names exactly the four datatype rules that are not OWL-specific.
            pub(crate) const fn fires_under(self, regime: Regime) -> bool {
                match self {
                    $( Self::$variant => matches!(regime, $( Regime::$lane )|+), )*
                }
            }

            /// The head form this rule's OWN clauses carry, when it is not the atomic one.
            ///
            /// `Some(HeadForm::Inconsistency)` is a rule whose conclusion is `false`. Its
            /// clauses state exactly that; [`program_with_attribution`] lowers them
            /// through [`constraint_clause`] before the evaluator sees them, and
            /// [`crate::engine`] reads this marker to know a firing is a CLASH rather than
            /// a conclusion.
            pub(crate) const fn declared_head_form(self) -> Option<HeadForm> {
                match self {
                    $( Self::$variant => declared_form!($($concludes)?), )*
                }
            }

            /// Whether this rule's conclusion is `false` — an inconsistency, not a triple.
            pub(crate) const fn is_constraint(self) -> bool {
                matches!(self.declared_head_form(), Some(HeadForm::Inconsistency))
            }
        }

        /// The DL clauses that state — and, through [`crate::engine`], run — `rule`.
        ///
        /// Most rules are one clause. Some are several, and always for one of two
        /// reasons. A rule whose specification conclusion is a CONJUNCTION contributes one
        /// clause per conjunct — `scm-cls` four, `scm-eqc1`, `scm-op`, `scm-dp` and
        /// `scm-eqp1` two each — because a conjunctive head is not a Datalog clause, and
        /// `rdfs1`, `rdfs4` and `prp-ap` likewise state one clause per constant they
        /// quantify over. A rule whose premise walks an RDF COLLECTION contributes three:
        /// two that accumulate the traversal into an internal relation and one that reads
        /// it off the axiom (`prp-spo2`, `prp-key`). Either way the clause count is not the
        /// rule count, which is what [`program_with_attribution`] exists to keep straight.
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

collect_families! { { rdfs_rules, eq_rules, prp_rules, cls_rules, cax_rules, dt_rules, scm_rules } }

/// The internal id that names WHICH rule a [`CLASH_RELATION`] row came from.
///
/// One surface per rule, so two constraint rules that clash on the same pair of terms
/// produce two distinct rows and the witness names the rule that actually fired rather
/// than whichever of them won a tie. It leads with [`INTERNAL_SIGIL`], so it is disjoint
/// from every RDF term surface by construction (see [`crate::lists`]).
fn clash_marker(rule: ChaseRule) -> ClauseTerm {
    ClauseTerm::literal(format!("{INTERNAL_SIGIL}clash:{}", rule.index()))
}

/// Lower a `false`-headed clause into the constraint clause the evaluator runs.
///
/// The body is carried through UNCHANGED — the specification's own premises, in the
/// specification's own order, negated atoms included — and the head becomes one atom of
/// the internal [`CLASH_RELATION`]:
///
/// ```text
/// CLASH(⟪clash:i⟫, ?a, ?b)   :-   the declared body
/// ```
///
/// where `?a` and `?b` are the first two distinct variables the POSITIVE body atoms bind,
/// in authored order. Two variables and not zero, because a head with no variable would
/// collapse every match of the rule into one row and lose the witness; the positive atoms
/// and not all of them, because a variable only a negated atom mentions is not bound and
/// the clause would not be range-restricted. A rule with fewer than two such variables
/// repeats what it has — `cls-nothing2` binds only `?x` — which is a narrower witness, not
/// a wrong one.
///
/// The lowering is deliberately MECHANICAL. A hand-written second statement of a
/// constraint would be a second place the rule could be wrong;
/// `the_lowering_preserves_the_declared_body` asserts that this one changes nothing but
/// the head.
fn constraint_clause(rule: ChaseRule, clause: &DlClause) -> DlClause {
    let mut bound: Vec<ClauseTerm> = Vec::new();
    for atom in clause.body().iter().filter(|atom| !atom.is_negated()) {
        for term in atom.terms() {
            if term.is_var() && !bound.contains(term) {
                bound.push(term.clone());
            }
        }
    }
    let marker = clash_marker(rule);
    let first = bound.first().cloned().unwrap_or_else(|| marker.clone());
    let second = bound.get(1).cloned().unwrap_or_else(|| first.clone());
    DlClause::datalog(
        ClauseAtom::quad(marker, ClauseTerm::literal(CLASH_RELATION), first, second),
        clause.body().to_vec(),
    )
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
/// rules of Tables 4–9 in table order — restricted to the rules `regime`'s lane fires.
/// Several rules take more than one clause (a conjunctive conclusion, a constant
/// quantified over, or a collection traversal), so the clause count is not the rule count,
/// and the seventeen rules that conclude `false` appear here in their LOWERED form: the
/// specification's own body under a head that names an internal clash relation, which
/// [`materialize`](crate::materialize) turns into
/// [`EntailError::Inconsistent`](crate::EntailError) rather than into a triple. The result
/// is a pure function of `regime`: no map iteration,
/// no hashing, no allocation order reaches it.
///
/// `Simple` yields the empty program: the identity closure has no rules, and that is a
/// statement about the calculus, not an omission. `OWL-Direct` and `RIF` yield the empty
/// program too, because neither is defined by a fixed clause table this crate can
/// enumerate — a tableau and a caller-supplied rule set respectively.
///
/// ```
/// use purrdf_entail::{Regime, calculus_program};
///
/// assert!(calculus_program(Regime::Simple).is_empty());
/// // All eighteen RDF/RDFS patterns; `rdfs1`, `rdfs4` and `rdfD1a` take three clauses
/// // each, and `rdfD1` and `rdfs14` four (one mint plus one substitution per position).
/// assert_eq!(calculus_program(Regime::Rdfs).len(), 30);
/// // `D` is the five `dt-*` rules, and `dt-type1` states one clause per supported
/// // datatype — thirty-two of them.
/// assert_eq!(calculus_program(Regime::D).len(), 36);
/// // Nine of those RDFS patterns plus the whole of OWL 2 Profiles Tables 4-9.
/// assert!(calculus_program(Regime::OwlRl).len() > calculus_program(Regime::Rdfs).len());
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
/// the program. Several rules contribute more than one clause each, so the map is not the
/// identity and cannot be reconstructed from a rule count.
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
            // The DECLARED marker decides, not a re-reading of the clause: a rule says
            // once what it concludes, and `the_constraint_marker_matches_the_clauses`
            // checks the two against each other rather than letting one stand in for the
            // other.
            clauses.push(if rule.is_constraint() {
                constraint_clause(rule, &clause)
            } else {
                clause
            });
            attribution.push(rule);
        }
    }
    (clauses, attribution)
}

/// The rules `regime`'s lane declares with a `false` head, with the clauses that state
/// them.
///
/// The other half of [`program_with_attribution`]'s input: these are the rules whose
/// clauses reach the evaluator through [`constraint_clause`] rather than verbatim, and
/// `the_lowering_preserves_the_declared_body` is what checks the lowering against them.
#[cfg(test)]
pub(crate) fn declared_constraints(regime: Regime) -> Vec<(ChaseRule, Vec<DlClause>)> {
    ChaseRule::ALL
        .into_iter()
        .filter(|rule| rule.fires_under(regime) && rule.is_constraint())
        .map(|rule| (rule, clauses_for(rule)))
        .collect()
}

/// The [`ChaseRule`] a [`CLASH_RELATION`] row's SUBJECT surface names, if it names one.
///
/// The inverse of [`clash_marker`], used by [`crate::engine`] to attribute a clash to the
/// rule whose body matched. It is total over the markers this module mints and answers
/// `None` for anything else, so a surface that merely looks internal cannot be read as a
/// rule.
pub(crate) fn clash_rule(marker: &str) -> Option<ChaseRule> {
    ChaseRule::ALL
        .into_iter()
        .find(|&rule| clash_marker(rule).surface().as_deref() == Some(marker))
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
        calculus_program, clash_marker, clash_rule, clauses_for, constraint_clause,
        declared_constraints, fired_rule_ids, program_with_attribution,
    };
    use crate::{Regime, RuleId, implemented, rules};
    use purrdf_datalog::cache::contract_hash;
    use purrdf_datalog::chase::certify;
    use purrdf_datalog::clause::HeadForm;
    use purrdf_datalog::seminaive::compile;
    use std::collections::BTreeSet;

    /// EVERY rule that DECLARES a `false` head really states one, in every clause.
    ///
    /// The `concludes: Inconsistency,` marker and the clauses are two statements of one
    /// fact, and [`program_with_attribution`] trusts the marker; this is what stops the
    /// two drifting. A rule marked as a constraint whose clauses are atomic would be
    /// lowered into nonsense, and an unmarked rule whose clauses conclude `false` would
    /// reach `compile` and be refused at run time instead of here.
    #[test]
    fn the_constraint_marker_matches_the_clauses() {
        let mut constraints = 0_usize;
        for rule in ChaseRule::ALL {
            for clause in clauses_for(rule) {
                assert_eq!(
                    clause.head_form() == HeadForm::Inconsistency,
                    rule.is_constraint(),
                    "{rule:?} marks {:?} and states {}",
                    rule.declared_head_form(),
                    clause.head_form()
                );
            }
            if rule.is_constraint() {
                constraints += 1;
            }
        }
        // The seventeen OWL 2 RL rules whose conclusion is `false`.
        assert_eq!(constraints, 17);
    }

    /// THE LOWERING CHANGES THE HEAD AND NOTHING ELSE.
    ///
    /// A constraint reaches the evaluator through [`constraint_clause`], which is the one
    /// place a `false` head becomes an atom. If that function altered a body atom, dropped
    /// a negation or reordered a premise, the rule that RAN would not be the rule that was
    /// DECLARED — and the witness a caller gets would name premises the specification does
    /// not. This asserts, per clause, that the body survives verbatim and the head is one
    /// atom of the internal clash relation.
    #[test]
    fn the_lowering_preserves_the_declared_body() {
        let mut lowered = 0_usize;
        for regime in ALL_REGIMES {
            for (rule, clauses) in declared_constraints(regime) {
                for clause in &clauses {
                    assert_eq!(clause.head_form(), HeadForm::Inconsistency);
                    let constraint = constraint_clause(rule, clause);
                    assert_eq!(
                        constraint.body(),
                        clause.body(),
                        "{rule:?}: the lowering moved a premise"
                    );
                    assert_eq!(constraint.head_form(), HeadForm::Atomic);
                    let head = constraint.datalog_head().expect("an atomic head");
                    assert_eq!(head.subject(), &clash_marker(rule));
                    assert_eq!(
                        clash_rule(&clash_marker(rule).surface().expect("a constant")),
                        Some(rule),
                        "{rule:?}: a clash row must name its own rule"
                    );
                    // Range restriction is what `compile` would refuse; asserting it here
                    // names the rule rather than a clause index.
                    assert!(
                        compile(vec![constraint.clone()]).is_ok(),
                        "{rule:?}: the lowered clause is not admissible"
                    );
                    lowered += 1;
                }
            }
        }
        assert!(lowered > 0, "no constraint was exercised");
        // Two lanes declare constraints, and only two.
        assert!(declared_constraints(Regime::Rdfs).is_empty());
        assert_eq!(declared_constraints(Regime::OwlRl).len(), 17);
        assert_eq!(declared_constraints(Regime::D).len(), 1);
    }

    /// Every clash marker is distinct, so two constraint rules cannot mask each other's
    /// witness, and none of them is an RDF term surface.
    #[test]
    fn clash_markers_are_distinct_and_internal() {
        let markers: Vec<String> = ChaseRule::ALL
            .into_iter()
            .filter(|rule| rule.is_constraint())
            .map(|rule| clash_marker(rule).surface().expect("a constant"))
            .collect();
        let distinct: BTreeSet<&String> = markers.iter().collect();
        assert_eq!(distinct.len(), markers.len(), "{markers:?}");
        for marker in &markers {
            assert!(crate::lists::is_internal(marker), "{marker:?}");
        }
        assert_eq!(clash_rule("<http://example.org/p>"), None);
    }

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

    /// Every declared program is ADMISSIBLE to the engine that will run it.
    ///
    /// A declaration the evaluator would refuse is not a statement of a calculus, it is a
    /// typo. There are now two engines, and which one a lane reaches is a property of its
    /// program rather than of a hard-coded list — so this asserts the same thing of both:
    ///
    /// * a program of ATOMIC clauses must `compile`, which proves it is neither non-Datalog
    ///   in its head form nor unsafe in its range restriction, and — since the seventeen
    ///   constraints reach it LOWERED — that the lowering is admissible too;
    /// * a program stating an EXISTENTIAL clause must be refused by `compile` BY NAME (a
    ///   least-fixpoint evaluator over definite clauses has no semantics for `∃ȳ. …`) and
    ///   must be certified terminating by the chase's own analysis, which is what admits it
    ///   there.
    #[test]
    fn every_declared_program_is_admissible_to_the_engine_that_runs_it() {
        let mut existential_lanes = 0_usize;
        for regime in ALL_REGIMES {
            let program = calculus_program(regime);
            let existential = program
                .iter()
                .any(|clause| clause.head_form() == HeadForm::Existential);
            if existential {
                existential_lanes += 1;
                let refusal = compile(program.clone())
                    .expect_err("a least-fixpoint evaluator has no semantics for an existential");
                assert!(
                    refusal.to_string().contains("existential"),
                    "{regime:?}: the refusal must NAME the head form: {refusal}"
                );
                assert!(
                    certify(&program).is_certified(),
                    "{regime:?}: the chase must certify the lane it is asked to run —                      {}",
                    certify(&program)
                );
                continue;
            }
            assert!(
                compile(program).is_ok(),
                "{regime:?}'s declared program must be admissible"
            );
        }
        // The two lanes whose rule tables hold rdfD1 / rdfD1a / rdfs14 / rdfs14a.
        assert_eq!(existential_lanes, 2);
    }

    /// NO LANE BOTH INVENTS TERMS AND DECIDES INCONSISTENCY.
    ///
    /// A `false`-headed rule is lowered into a clause over the internal clash relation, and
    /// [`crate::engine`] turns a clash ROW into a witness by reading the derivation's
    /// SOURCES — the matched body facts. A chase derivation records the clause that
    /// concluded a fact, not the facts that satisfied its body, so a clash on the chase
    /// path could be detected but not EXPLAINED. Rather than let that become a witness with
    /// no premises, the two capabilities are kept in disjoint lanes, and this is the
    /// assertion that keeps them there: a lane that gains an existential rule and a
    /// constraint rule at once fails here, in the calculus, rather than silently degrading
    /// a witness at run time.
    #[test]
    fn no_lane_both_states_an_existential_and_a_constraint() {
        for regime in ALL_REGIMES {
            let existential = calculus_program(regime)
                .iter()
                .any(|clause| clause.head_form() == HeadForm::Existential);
            let constraint = ChaseRule::ALL
                .into_iter()
                .any(|rule| rule.fires_under(regime) && rule.is_constraint());
            assert!(
                !(existential && constraint),
                "{regime:?} states both an existential rule and a rule that concludes                  `false`; the chase cannot carry a clash witness's premises"
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
                (Regime::Rdf, 8),
                (Regime::Rdfs, 30),
                (Regime::OwlRl, PINNED_OWL_RL_CLAUSES),
                (Regime::OwlDirect, 0),
                (Regime::Rif, 0),
                (Regime::D, 36),
            ]
        );
    }

    /// The `OWL-RL` lane's clause count, pinned. It is not the rule count: `prp-ap` states
    /// nine clauses, `dt-type1` thirty-two, `eq-ref` three, the two collection traversals
    /// three each, and five rules state a conjunctive conclusion as two.
    const PINNED_OWL_RL_CLAUSES: usize = 135;

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
            (Regime::Rdf, PINNED_RDF_HASH),
            (Regime::Rdfs, PINNED_RDFS_HASH),
            (Regime::OwlRl, PINNED_OWL_RL_HASH),
            (Regime::OwlDirect, empty),
            (Regime::Rif, empty),
            (Regime::D, PINNED_D_HASH),
        ];
        for (regime, digest) in pinned {
            assert_eq!(
                calculus_contract_hash(regime).to_hex(),
                digest,
                "{regime:?}"
            );
        }
    }

    /// The `RDF` lane's calculus identity, moved deliberately by stating `rdfD1` and
    /// `rdfD1a` — the two RDF patterns whose conclusions are existentially quantified.
    /// A consumer holding a closure minted under the one-rule calculus can tell, which is
    /// the whole point of the digest.
    const PINNED_RDF_HASH: &str =
        "9724540a02daf06f349ba8aecf52f9e3e21abf7042d1bfcc630172baa7f23ee3";
    /// The `RDFS` lane's, moved for the same reason with `rdfs14` and `rdfs14a` besides.
    const PINNED_RDFS_HASH: &str =
        "f6a5eff90528f49ba8d3b83ae7feac8e3b71586f320fb478c6ddb60ca6dc979e";
    /// The `OWL-RL` lane's. It moved on this branch WITHOUT any OWL 2 RL rule changing,
    /// and the reason is worth stating rather than hiding: a rule that concludes `false` is
    /// lowered into a clause whose head names a clash marker built from the rule's
    /// DECLARATION INDEX, and declaring `rdfD1` and `rdfD1a` ahead of `rdfD2` — which is
    /// where RDF 1.2 Semantics §8.1.1 puts them — renumbers every rule after them. So the
    /// seventeen markers moved, the clauses that carry them moved, and the digest moved
    /// with them. Nothing this lane concludes changed, and that is exactly the case the
    /// digest is allowed to be conservative about: a consumer refusing a cached closure it
    /// could have kept is a cost, whereas trusting one minted under a different rule set is
    /// a defect.
    const PINNED_OWL_RL_HASH: &str =
        "61a9a84001b40b06a8dfe4b38e331ca1943893cbd191b272d6ffead048fa2de0";
    /// The `D` lane's, moved by the same renumbering — `dt-not-type` is a constraint too.
    const PINNED_D_HASH: &str = "20ac9aa48ce8dd9b25bc7239798566da2d1559458c6ed12befa520c3fdb16cb7";

    /// Two lanes with different rule sets have different calculus identities, and the
    /// three rule-free regimes share the empty program's identity.
    #[test]
    fn different_rule_sets_have_different_identities() {
        let rdf = calculus_contract_hash(Regime::Rdf);
        let rdfs = calculus_contract_hash(Regime::Rdfs);
        let owl = calculus_contract_hash(Regime::OwlRl);
        let d = calculus_contract_hash(Regime::D);
        assert_ne!(rdf, rdfs);
        assert_ne!(rdfs, owl);
        assert_ne!(rdf, owl);
        assert_ne!(d, owl);
        // No rules is itself a calculus, and the three rule-free regimes state it.
        let empty = calculus_contract_hash(Regime::Simple);
        for regime in [Regime::OwlDirect, Regime::Rif] {
            assert_eq!(calculus_contract_hash(regime), empty, "{regime:?}");
        }
        assert_ne!(empty, rdf);
        assert_ne!(empty, d, "`D` is no longer the empty calculus");
    }

    /// The lane membership is a partition the enum cannot silently widen: `RDFS` fires
    /// fourteen rules, `OWL-RL` seventy-eight, `RDF` one, `D` five, and nothing else fires
    /// at all.
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
        assert_eq!(count(Regime::Rdf), 3);
        assert_eq!(count(Regime::Rdfs), 18);
        // The whole of OWL 2 Profiles §4.3 Tables 4-9 — seventy-eight rules — plus the
        // three RDFS-shaped rules the lane fires under no OWL name.
        assert_eq!(count(Regime::OwlRl), 78 + 3);
        assert_eq!(count(Regime::D), 5);
        for regime in [Regime::Simple, Regime::OwlDirect, Regime::Rif] {
            assert_eq!(count(regime), 0, "{regime:?}");
        }
        // The seventeen rules whose conclusion is `false`, by the id they are reported
        // under. Named as a SET rather than counted, so a rule that quietly stops being a
        // constraint fails here rather than in a golden.
        let constraints: Vec<RuleId> = ChaseRule::ALL
            .into_iter()
            .filter(|rule| rule.fires_under(Regime::OwlRl) && rule.is_constraint())
            .map(|rule| rule.rule_id(true))
            .collect();
        assert_eq!(
            constraints,
            vec![
                RuleId::EqDiff1,
                RuleId::EqDiff2,
                RuleId::EqDiff3,
                RuleId::PrpIrp,
                RuleId::PrpAsyp,
                RuleId::PrpPdw,
                RuleId::PrpAdp,
                RuleId::PrpNpa1,
                RuleId::PrpNpa2,
                RuleId::ClsNothing2,
                RuleId::ClsCom,
                RuleId::ClsMaxc1,
                RuleId::ClsMaxqc1,
                RuleId::ClsMaxqc2,
                RuleId::CaxDw,
                RuleId::CaxAdc,
                RuleId::DtNotType,
            ]
        );
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
    /// clause list, and credits every rule the lane fires — including the rules that
    /// contribute more than one clause each, which is what makes the map non-trivial.
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
        // The conjunctive-conclusion rules are stated as two clauses each, so the
        // attribution names them twice and a clause index is NOT a rule index.
        let (_, owl) = program_with_attribution(Regime::OwlRl);
        for rule in [ChaseRule::EquivalentClass, ChaseRule::EquivalentProperty] {
            assert_eq!(
                owl.iter().filter(|&&r| r == rule).count(),
                2,
                "{rule:?} states two clauses"
            );
        }
        // `eq-ref` states three, one per triple position.
        assert_eq!(
            owl.iter().filter(|&&r| r == ChaseRule::Reflexive).count(),
            3
        );
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
