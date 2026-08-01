// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The vendored W3C OWL 2 **entailment** corpus: ingester, grader, and ledger.
//!
//! # Why this module exists
//!
//! [`crate::owl2`] grades *satisfiability*: it asks the `OWL-Direct` tableau
//! whether an ontology has a model. That says nothing about the OWL 2 **RL** rule
//! table, which is a forward-materialization chase over a declared rule program.
//! Until this module existed, the only thing grading that chase was fixtures
//! written by the same change that wrote the rules — the implementation and its
//! oracle had a common author.
//!
//! This module is the independent oracle. Its cases are W3C's, taken from
//! <https://www.w3.org/2009/11/owl-test/all.rdf>, and it grades PurRDF's
//! [`purrdf_entail::Regime::OwlRl`] closure against W3C's published entailment
//! verdicts.
//!
//! # It grades the SHIPPED procedure, not a copy of it
//!
//! Every verdict below comes from one call to [`purrdf_entail::entails()`] — the
//! library's conclusion-directed entailment service, which is what a caller gets.
//! This harness parses two documents and compares an answer; it materializes
//! nothing, matches nothing, and owns no blank-node matcher of its own. It used to
//! own one, and that is precisely the failure mode a corpus is supposed to close: a
//! grader with its own matcher grades a procedure no caller can invoke, and the two
//! can drift without a single test going red.
//!
//! # What is graded, and how
//!
//! Two lanes, distinguished by which target file a case directory carries:
//!
//! * **positive** (`conclusion.rdf`) — the W3C published
//!   `otest:PositiveEntailmentTest`: the premise entails the conclusion. PurRDF
//!   passes when the OWL-RL closure of the premise *simple-entails* the
//!   conclusion, i.e. the conclusion graph maps into the closure with its blank
//!   nodes read as existentials. The vendored positives are exactly the cases
//!   W3C declares `otest:profile RL` under `otest:semantics RDF-BASED`.
//! * **negative** (`non-conclusion.rdf`) — the W3C published
//!   `otest:NegativeEntailmentTest`: the premise does **not** entail the
//!   non-conclusion. Because a rule chase is *sound*, deriving it would be an
//!   unsoundness, so PurRDF passes when the non-conclusion does **not** map into
//!   the closure. This lane is profile-independent: soundness is owed on every
//!   case, not only on RL-profile ones.
//!
//! # What `otest:profile RL` does and does not promise
//!
//! It says the case's *ontology* is inside the OWL 2 RL profile. It does **not**
//! say the OWL 2 RL/RDF rule table (Profiles §4.3) reaches the conclusion: the
//! measured ledger below shows W3C tagging cases `RL` whose **conclusion** uses
//! `owl:complementOf`, which is not even in the RL syntax. So a divergence on one
//! of these 27 is not automatically a defect — but it is always something the
//! ledger must name concretely, and [`RlGap`] draws the line between "the rule
//! table omits a sound, in-shape rule" ([`RlGap::MissingRule`]) and "no rule of
//! this shape could have that head" ([`RlGap::NegativeConclusion`],
//! [`RlGap::SchemaConclusion`], [`RlGap::ConstructOutsideRl`]).
//!
//! # The chase is measured with its EXTENSION, and the extension is named
//!
//! `purrdf_entail::Regime::OwlRl`'s calculus is OWL 2 Profiles §4.3 Tables 4–9
//! plus whatever `purrdf_entail::extensions(Regime::OwlRl)` lists — today one rule,
//! `ext-eq-diff-sym`, symmetry of `owl:differentFrom`. That is the closure this
//! module grades, because it is the closure a caller gets. It does not blur the two
//! claims: `rules(OwlRl)` and `implemented(OwlRl)` are both still exactly 78, the
//! extension appears in neither, and every rendered report carries an `extension`
//! line naming it — so a reader of this scoreboard can tell which agreements the
//! normative table reached on its own.
//!
//! # The chase is also measured with its EXTRA MECHANISMS, which add no rule
//!
//! `purrdf_entail::entails()` reaches a conclusion five ways, and twelve of the cases
//! graded here are reached only by one of the four beyond matching.
//!
//! * **Refutation.** Seventeen of the seventy-eight rules conclude `false`, which is
//!   to say the table carries its own inconsistency calculus; so a conclusion whose
//!   head no rule has — an `owl:differentFrom`, a membership in an
//!   `owl:complementOf` class, an `owl:AllDifferent` collection — is decided by
//!   asserting its negation into the premise and re-running the SAME table, over a
//!   premise whose consistency was established first. Eight cases.
//! * **Freeze-and-chase.** A schema axiom such as `p rdf:type
//!   owl:TransitiveProperty` abbreviates a universally quantified Horn implication,
//!   and such an implication is decided by generalisation on constants: freeze its
//!   body over constants the premise does not mention, re-run the SAME table, and
//!   look for its head. One case (`chain2trans1`), whose head arrives through
//!   `prp-spo2`.
//! * **Comprehension.** A conclusion may assert that a CLASS EXISTS — an anonymous
//!   `owl:unionOf`, an anonymous `owl:Restriction` — which the RDF-Based semantics'
//!   own comprehension conditions license, subject to a typing side condition on the
//!   operands. The scaffolds the conclusion names are minted over blank nodes checked
//!   absent from both documents, and nothing else is. Two cases
//!   (`webont-i5-5-005`, `webont-i5-26-010`).
//! * **Reflexivity.** `owl:ReflexiveProperty` is outside the OWL 2 RL syntax, so the
//!   profile states no rule for it — and a rule that DID state it would range over
//!   every resource. The conclusion's own self-loops `x p x` are instead read off the
//!   premise's reflexive typings, one lookup per conclusion triple. One case
//!   (`new-feature-reflexiveproperty-001`).
//!
//! All four are worth separating from the extension above because they are a
//! different kind of thing. `ext-eq-diff-sym` is a rule PurRDF states that no
//! specification does, and it is declared as such. None of these mechanisms states
//! anything: the rule inventory is byte-for-byte the same seventy-eight before and
//! after, and the four `the_*_lane_adds_no_rule` tests in `purrdf-entail` assert it.
//! What changed is how many times the table is run, what it is run over, and what the
//! run's `false` is read as.
//!
//! Grading a positive case is one-sided in the honest direction: matching proves
//! entailment (the chase is sound), and failing to match is always a real,
//! reportable limit of the lane. Grading a negative case is one-sided the other
//! way: a match is a proven unsoundness, and a non-match is the expected answer —
//! which makes the negative lane a *soundness* gate with weak discriminating
//! power, and it is reported as such rather than counted as if it proved
//! completeness. The positive lane carries the discrimination: a reasoner that
//! derives nothing at all fails all 27 positive cases.
//!
//! # Three outcomes, never two
//!
//! Matching [`crate::owl2`]'s discipline, a run has three buckets ([`Grade`]):
//!
//! * **agree** — PurRDF's closure gave the published answer;
//! * **withhold** — PurRDF *refused to decide*: the RDF/XML would not parse, an
//!   `owl:imports` could not be resolved, the chase returned an
//!   [`EntailError`](purrdf_entail::EntailError), or — on a positive case — the
//!   service answered [`Answer::Undecided`] because the premise is outside the OWL 2
//!   RL syntax and Theorem PR1's completeness half therefore does not apply. A
//!   refusal is a capability gap and is never scored as a pass;
//! * **disagree** — PurRDF produced a closure and it gave the other answer.
//!
//! [`Answer`] itself has FOUR inhabitants, because the library distinguishes four
//! things: a proof of entailment, a proof of non-entailment, "no mapping was found
//! and I am not entitled to call that a refutation", and "there was no closure to
//! test". [`grade`] is where the last two stop being interchangeable, and its own
//! doc comment carries the argument for why the negative lane reads `Undecided` as
//! the soundness observation it is while the positive lane reads it as a gap.
//!
//! Every withhold and every disagreement must appear in [`LEDGER`] with a typed
//! [`RlGap`]. An unledgered one fails the harness, a ledgered one that starts
//! agreeing fails it, and a ledger entry naming a case that is not vendored fails
//! it — so the ledger can neither rot nor inflate the budget.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub use purrdf_entail::{EntailmentOutcome, MissReason, UndecidedReason};

/// What the W3C published for a case, which is also which target file it carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// `otest:PositiveEntailmentTest`: the premise entails `conclusion.rdf`.
    Positive,
    /// `otest:NegativeEntailmentTest`: the premise does *not* entail
    /// `non-conclusion.rdf`.
    Negative,
}

impl Direction {
    /// The target file name a case of this direction carries.
    #[must_use]
    pub const fn target_file(self) -> &'static str {
        match self {
            Self::Positive => "conclusion.rdf",
            Self::Negative => "non-conclusion.rdf",
        }
    }

    /// The token used in the census and in the harness log.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }

    /// Whether a closure that *matches* the target is the published answer.
    #[must_use]
    pub const fn expects_match(self) -> bool {
        matches!(self, Self::Positive)
    }
}

/// Why PurRDF's OWL 2 RL lane diverges from the published verdict on a ledgered
/// case.
///
/// Each variant names the concrete rule or construct responsible, so the ledger
/// doubles as an inventory of what the rule table does not yet do. A catch-all
/// would defeat the point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlGap {
    /// The conclusion is a **positive assertional triple over named terms** that a
    /// sound rule of OWL 2 RL's own shape would produce, and no such rule fires.
    ///
    /// Read this variant carefully, because it is where the "`OWL-RL` 78 / 78"
    /// headline and W3C conformance come apart. 78 / 78 is a statement about
    /// *table coverage*: PurRDF implements every one of the 78 rules of OWL 2
    /// Profiles §4.3 Tables 4–9. It is not a statement about entailment
    /// conformance, and this variant is the case where both are true at once —
    /// the missing rule is sound and shaped exactly like a rule the table already
    /// has, yet **it is not one of the 78**, so complete coverage of the table
    /// still does not reach the conclusion.
    ///
    /// Closing one of these means adding a rule *beyond* the normative table,
    /// which is a decision with a cost and is why the scoreboard counts these
    /// separately instead of burying them among the profile's structural limits.
    /// That cost is paid by DECLARING the extra rule as an extension rather than
    /// by widening the table: `purrdf_entail::extensions` names every rule the
    /// chase fires that no specification states, `rules` and `implemented` name
    /// none of them, and a rendered report carries an `extension` line — so the
    /// lane is exactly Tables 4–9 plus a list a caller can read and reject.
    ///
    /// The ledger holds no entry of this kind today: the one it did hold
    /// (`webont-differentfrom-001`, symmetry of `owl:differentFrom`) was closed
    /// that way. The variant stays because the classification is what a future
    /// divergence of this shape must be filed under, and because
    /// [`Self::is_actionable`] is the predicate that separates "PurRDF could
    /// reach this" from "no conforming RL rule set could".
    MissingRule,
    /// The conclusion is a **schema axiom** — a property characteristic, an
    /// `rdfs:range`, an anonymous class expression. Every head in the OWL 2 RL/RDF
    /// rule table (Profiles §4.3) is either an assertional triple over named terms
    /// or `false`; not one concludes a new axiom of these shapes, so no conforming
    /// RL rule set derives them.
    ///
    /// `owl:AllDifferent` used to be on that list and is not any more, which is the
    /// distinction the list is for: an `owl:AllDifferent` collection LOOKS like a
    /// schema axiom and IS, by OWL 2's own definition, the conjunction of its
    /// `n(n−1)/2` pairwise inequalities — so it lowers to
    /// [`Self::NegativeConclusion`]'s shape and is reached by refuting every pair.
    /// A property CHARACTERISTIC left the same way and for the same reason:
    /// `p rdf:type owl:TransitiveProperty` is `p ∈ IOOP` conjoined with a
    /// universally quantified Horn implication, and an implication is decided by
    /// instantiating its body over fresh constants and re-running the table. So did
    /// an anonymous CLASS EXPRESSION, whose existence the RDF-Based semantics'
    /// comprehension conditions license outright given a typing side condition on its
    /// operands. A conclusion belongs here when nothing it can be decomposed into has
    /// a head in the table either, and nothing licenses it directly.
    SchemaConclusion,
    /// The conclusion is a **negative fact** — an `owl:differentFrom`, or
    /// membership in an `owl:complementOf` class. It follows from the premise only
    /// by refutation: assume the negation, reach `false`.
    ///
    /// The ledger holds no entry of this kind today, and the six it did hold are
    /// the reason the wording above is a description rather than an excuse. They
    /// were filed under "a forward chase over definite rules cannot perform
    /// refutation", which conflated two things: the rule table has no rule whose
    /// HEAD is a negative fact — true, and still true — but seventeen of its
    /// seventy-eight rules conclude `false`, and those seventeen *are* an
    /// inconsistency calculus. A refutation needs no new rule, only a second run of
    /// the same table over the premise plus the conclusion's negation, and
    /// [`purrdf_entail::entails()`] performs one.
    ///
    /// The variant stays because the CLASSIFICATION is still what a conclusion of
    /// this shape is, and because a future one that the refutation lane cannot
    /// reach — a negative construct outside the three shapes it reads — has to be
    /// filed somewhere that says so. It is not [`Self::is_actionable`], because
    /// reaching one is not a matter of adding a rule to the table.
    NegativeConclusion,
    /// The entailment turns on an OWL 2 construct **outside the OWL 2 RL syntax**,
    /// for which the profile's rule table states no rule at all.
    ///
    /// The ledger holds no entry of this kind today. The one it did hold
    /// (`new-feature-reflexiveproperty-001`, `owl:ReflexiveProperty`) is closed, and
    /// closing it did not add a rule: the profile still states none, and a rule that
    /// stated one would range over every resource in a lane every consumer runs by
    /// default. It was closed by ESTABLISHING the conclusion positively from the
    /// semantic condition, which needs no completeness theorem and therefore no
    /// profile membership.
    ///
    /// The variant stays because a construct outside the syntax whose conclusion
    /// nothing establishes positively has to be filed somewhere that says so. It is
    /// not [`Self::is_actionable`]: reaching one is not a matter of adding a rule to
    /// the table.
    ConstructOutsideRl,
    /// The premise `owl:imports` a document the upstream manifest keeps as a
    /// separate support file rather than inline, so the vendored premise is not
    /// the whole premise and **no** reasoner could reach the conclusion from what
    /// is vendored. Ledgered rather than dropped, so the incompleteness of the
    /// upstream export stays visible.
    ImportsUnresolved,
    /// The premise or target RDF/XML did not parse, or the chase returned an
    /// error, so the run refused to decide.
    Refused,
    /// The chase derived a triple the W3C says is **not** entailed. This is an
    /// unsoundness: PurRDF asserted a conclusion it is not entitled to.
    UnsoundDerivation,
}

impl RlGap {
    /// A short human-readable label for the ledger tally and the log.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MissingRule => "missing-rule",
            Self::SchemaConclusion => "schema-conclusion",
            Self::NegativeConclusion => "negative-conclusion",
            Self::ConstructOutsideRl => "construct-outside-rl",
            Self::ImportsUnresolved => "imports-unresolved",
            Self::Refused => "refused",
            Self::UnsoundDerivation => "unsound-derivation",
        }
    }

    /// Whether this gap is an **unsoundness** — PurRDF asserting a conclusion the
    /// W3C contradicts — rather than an incompleteness.
    ///
    /// The distinction is the ledger's most consequential one: an incompleteness
    /// withholds a conclusion PurRDF is entitled to, whereas an unsoundness
    /// asserts one it is not. The match is exhaustive on purpose, so a new gap
    /// cannot be added without classifying itself here.
    #[must_use]
    pub const fn is_unsound(self) -> bool {
        match self {
            Self::MissingRule
            | Self::SchemaConclusion
            | Self::NegativeConclusion
            | Self::ConstructOutsideRl
            | Self::ImportsUnresolved
            | Self::Refused => false,
            Self::UnsoundDerivation => true,
        }
    }

    /// Whether this gap names a conclusion a **sound rule of RL's own shape**
    /// could reach — as opposed to one no rule of that shape can have as a head,
    /// or a defect in the vendored premise.
    ///
    /// The scoreboard reports this count separately, because it is the only part
    /// of the ledger that is a decision to take rather than a description of the
    /// profile's structural limits.
    #[must_use]
    pub const fn is_actionable(self) -> bool {
        match self {
            Self::MissingRule | Self::UnsoundDerivation => true,
            Self::SchemaConclusion
            | Self::NegativeConclusion
            | Self::ConstructOutsideRl
            | Self::ImportsUnresolved
            | Self::Refused => false,
        }
    }
}

/// One ledgered divergence: the case's directory name plus its typed gap.
#[derive(Debug)]
pub struct LedgerEntry {
    /// The case directory name under `entailment-suite/w3c-owl2-rl/cases/`.
    pub case: &'static str,
    /// Why PurRDF diverges.
    pub gap: RlGap,
}

/// The divergence ledger: every vendored entailment case PurRDF does not answer
/// as the W3C published it.
///
/// Nothing is skipped at discovery time — all 50 cases run, and a case absent
/// from this table must agree.
pub const LEDGER: &[LedgerEntry] = &[
    // --- NO MISSING RULE. The one entry this table used to open with is CLOSED. -
    //     `webont-differentfrom-001` is `a owl:differentFrom b` entailing
    //     `b owl:differentFrom a`. Its head is a positive assertional triple over
    //     two named individuals, exactly the shape `prp-symp` already has, and
    //     stating it is sound — and it is NOT one of the 78 rules of OWL 2
    //     Profiles §4.3 Tables 4–9, because Table 4's `owl:differentFrom` rules
    //     (`eq-diff1..3`) only ever conclude `false`.
    //
    //     PurRDF now states it, as `purrdf_entail::RuleId::ExtEqDiffSym`, in a
    //     rule family that is declared to be OUTSIDE every specification table:
    //     `extensions(Regime::OwlRl)` names it, `rules(Regime::OwlRl)` and
    //     `implemented(Regime::OwlRl)` are still exactly the 78 and do not, and a
    //     rendered report carries an `extension ext-eq-diff-sym` line so a caller
    //     that must act only on normative conclusions can tell. So this case now
    //     AGREES, and it agrees through a rule that is labelled as PurRDF's rather
    //     than smuggled into a count that says W3C's.
    //
    //     Every other entry below is a structural limit of the OWL 2 RL profile
    //     rather than something PurRDF withholds: `RlGap::is_actionable` is now
    //     false for all of them, and `only_missing_rules_are_actionable` pins that
    //     the actionable set is EMPTY.
    // --- NO NEGATIVE CONCLUSION EITHER. That block is CLOSED, all six of it. ---
    //     Each of the six concluded a negative fact — an `owl:differentFrom`, or
    //     membership in an anonymous `owl:complementOf` class — and each was
    //     ledgered on the ground that a forward chase over definite rules cannot
    //     run a refutation backwards. That premise was true and the conclusion
    //     drawn from it was too strong: the rule table has no rule whose HEAD is a
    //     negative fact, but seventeen of its seventy-eight rules conclude `false`,
    //     and those seventeen ARE the profile's inconsistency calculus. Asserting
    //     the conclusion's negation into the premise and re-running the SAME table
    //     sends `cax-dw`, `cax-adc`, `prp-pdw`, `prp-adp` or `eq-diff1` to
    //     `false` — those five are the rules the eight cases actually clash on,
    //     measured rather than guessed — and, over a premise whose consistency
    //     was established first, that inconsistency IS the entailment.
    //     `new-feature-objectqcr-002` shows the shape at its longest: the
    //     asserted `Stewie a Woman` lets `cls-maxqc3` derive `Stewie sameAs Meg`
    //     against a `maxQualifiedCardinality 1`, and `eq-diff1` then clashes it
    //     with the premise's own `Stewie owl:differentFrom Meg`.
    //
    //     `purrdf_entail::entails()` now does exactly that, as a second mechanism
    //     with its own `EntailmentWarrant` arm and its own reasoner-free checker.
    //     It adds NO rule: `rules(Regime::OwlRl)` and `implemented(Regime::OwlRl)`
    //     are still exactly the 78 and `extensions(Regime::OwlRl)` is still the one
    //     `ext-eq-diff-sym`. So these six agree through the normative table, run a
    //     second time.
    // --- SCHEMA CONCLUSION: the head shape does not exist in the rule table ---
    //     The three `webont-i5-8-*` cases conclude an `rdfs:range` WIDENED to a
    //     containing XSD datatype (`xsd:byte ⊑ xsd:short`, which is why they are
    //     sound at all). Every head in OWL 2 RL/RDF's rule table is an assertional
    //     triple over named terms or `false`; not one concludes an axiom, so these
    //     are outside what the profile's rule set can produce rather than outside
    //     what this implementation happens to do.
    //
    //     Five cases have left this block. Two went with the six above:
    //     `new-feature-disjoint{data,object}properties-002` conclude an
    //     `owl:AllDifferent` collection, which reads as a schema axiom and IS the
    //     conjunction of its `n(n−1)/2` pairwise inequalities — so it lowers to the
    //     negative facts of the previous block and is reached the same way, one
    //     refutation per pair, all of them required.
    //
    //     The third is `chain2trans1`, which concludes `p rdf:type
    //     owl:TransitiveProperty` from `p owl:propertyChainAxiom (p p)`. Still no
    //     rule of Tables 4–9 has that head, and none ever will. But the axiom
    //     ABBREVIATES a universally quantified Horn implication, and a Horn
    //     implication is decided by GENERALISATION ON CONSTANTS: freeze
    //     `_:a p _:b . _:b p _:c` over constants the premise does not mention, re-run
    //     the SAME table, and `prp-spo2` — one of the seventy-eight — derives
    //     `_:a p _:c`, which is transitivity's own condition. The membership conjunct
    //     (`p ∈ IOOP`) is a lookup in the premise's own closure. So this agrees
    //     through the normative table, run once more over two extra atoms, and the
    //     rule inventory is untouched.
    //
    //     The other two are `webont-i5-5-005`, whose conclusion is an anonymous
    //     `owl:unionOf` class, and `webont-i5-26-010`, whose conclusion is an
    //     anonymous `owl:Restriction`. Neither says anything about any individual;
    //     each says a CLASS EXISTS, and the RDF-Based semantics says so too, in its
    //     COMPREHENSION CONDITIONS. Those conditions are licensed rather than free —
    //     `unionOf(a)` is licensed only for `a ∈ IC`, and `i5-5-005`'s premise
    //     asserting `a rdf:type owl:Class` is exactly why the case is a published
    //     entailment — so the typing side condition is established against the
    //     premise's own closure before anything is minted, and only the scaffolds the
    //     conclusion names are minted, over blank nodes checked absent from both
    //     documents. No rule could do this: a comprehension condition asserts a
    //     resource nothing names, and a rule set producing one per licensed shape
    //     would produce infinitely many.
    LedgerEntry {
        case: "webont-i5-8-006",
        gap: RlGap::SchemaConclusion,
    },
    LedgerEntry {
        case: "webont-i5-8-008",
        gap: RlGap::SchemaConclusion,
    },
    LedgerEntry {
        case: "webont-i5-8-009",
        gap: RlGap::SchemaConclusion,
    },
    // --- NO CONSTRUCT OUTSIDE OWL 2 RL EITHER. That block is CLOSED. ----------
    //     `new-feature-reflexiveproperty-001` asserts `knows a owl:ReflexiveProperty`
    //     and concludes `Peter knows Peter`. `owl:ReflexiveProperty` is still outside
    //     the OWL 2 RL syntax and Profiles §4.3 still states no `prp-rfl` rule — so
    //     the premise is ALSO outside the syntax, which is why a failed match here
    //     could never be read as a refutation either. The conclusion is therefore
    //     established POSITIVELY, which needs no completeness theorem: OWL 2's
    //     RDF-Based Semantics puts `<x,x>` in `EXT(p)` for every `x ∈ IR` when `p` is
    //     reflexive, and every IRI of the conclusion's vocabulary denotes an element
    //     of `IR`, so `Peter knows Peter` holds in every model of the premise.
    //
    //     Deliberately NOT closed by an `ext-prp-rfl` rule. Such a clause has no join
    //     to constrain its subject, so it would fire once per (term × reflexive
    //     property) pair in a lane every consumer runs by default, would put literals
    //     in subject position, and would move the calculus contract hash across every
    //     committed golden. `strict_materialization_is_unchanged` in `purrdf-entail`
    //     is the falsifiable form: `Materialization::OwlRl` produces exactly what it
    //     produced before, and `extensions(Regime::OwlRl)` is still the one
    //     `ext-eq-diff-sym`.
    // --- THE VENDORED PREMISE IS NOT THE WHOLE PREMISE ------------------------
    //     `webont-imports-011`'s premise `owl:imports` `support011-A`, which the
    //     upstream `all.rdf` export does not carry inline — the conclusion is about
    //     a class defined only in that support document. No reasoner reaches it
    //     from the vendored bytes. Ledgered rather than dropped so the gap in the
    //     upstream export stays visible instead of being silently deselected.
    LedgerEntry {
        case: "webont-imports-011",
        gap: RlGap::ImportsUnresolved,
    },
];

/// Look a case up in [`LEDGER`].
#[must_use]
pub fn ledger_lookup(case: &str) -> Option<RlGap> {
    LEDGER.iter().find(|e| e.case == case).map(|e| e.gap)
}

/// One vendored entailment case.
#[derive(Debug)]
pub struct RlCase {
    /// The case directory name, the slugified `otest:identifier`.
    pub name: String,
    /// The verbatim W3C RDF/XML premise ontology.
    pub premise: PathBuf,
    /// The verbatim W3C RDF/XML conclusion / non-conclusion ontology.
    pub target: PathBuf,
    /// Which of the two the target is.
    pub direction: Direction,
}

/// What PurRDF answered for a case.
///
/// Four answers, because [`purrdf_entail::entails()`] distinguishes four things and
/// collapsing any pair of them would put a claim in PurRDF's mouth that it did not make.
#[derive(Debug)]
pub enum Answer {
    /// The OWL-RL closure of the premise entails the target graph, and the service
    /// returned the blank-node mapping that proves it.
    Entailed,
    /// The closure was computed, does **not** contain the target graph, **and** the
    /// premise is inside the OWL 2 RL syntax — so by Profiles Theorem PR1 this is a
    /// refutation and not merely a failure to find something. Carries the diagnosis of
    /// what was missing.
    NotEntailed(MissReason),
    /// The closure was computed and does not contain the target graph, but the premise
    /// is OUTSIDE the OWL 2 RL syntax, so Theorem PR1's completeness half does not apply
    /// and nothing is proven either way.
    ///
    /// Distinct from [`Self::Withheld`] in the way that matters to this corpus: a
    /// closure was computed and tested, so the *soundness* observation the negative lane
    /// grades was actually made. What is missing is the entitlement to call the
    /// observation a refutation.
    Undecided(UndecidedReason),
    /// The run produced no closure to test at all — the RDF/XML would not parse, an
    /// `owl:imports` could not be resolved, the premise was inconsistent, or an
    /// evaluation ceiling was reached. Carries the refusal's own message.
    Withheld(String),
}

/// How PurRDF's answer compares to the published verdict.
#[derive(Debug)]
pub enum Grade {
    /// Answered, and matching the published verdict.
    Agree,
    /// Refused to answer, with the refusal's message.
    Withhold(String),
    /// Answered, and contradicting the published verdict.
    Disagree {
        /// The direction the W3C published.
        published: Direction,
        /// `Some(reason)` when the closure LACKED the target graph — which
        /// contradicts a `Positive` case (an incompleteness). `None` when the
        /// closure CONTAINED it — which contradicts a `Negative` case (an
        /// unsoundness).
        miss: Option<MissReason>,
    },
}

/// The vendored corpus root (`entailment-suite/w3c-owl2-rl`).
#[must_use]
pub fn suite_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("entailment-suite/w3c-owl2-rl")
}

/// Discover every vendored case under `root/cases`, in case-name order.
///
/// A case's direction is read from which target file it carries, so there is no
/// derived metadata file to drift out of step with the payload: a directory with
/// both targets, or neither, is a hard error rather than a silent default.
///
/// # Errors
///
/// Returns a message if the corpus root cannot be read, if it holds a
/// non-directory entry, or if a case directory does not hold exactly a
/// `premise.rdf` plus exactly one of `conclusion.rdf` / `non-conclusion.rdf`.
pub fn discover(root: &Path) -> Result<Vec<RlCase>, String> {
    let cases_dir = root.join("cases");
    let mut names: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(&cases_dir)
        .map_err(|e| format!("cannot read {}: {e}", cases_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", cases_dir.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("cannot stat {}: {e}", entry.path().display()))?;
        if !file_type.is_dir() {
            return Err(format!(
                "{}: unexpected non-directory entry in the corpus root",
                entry.path().display()
            ));
        }
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();

    let mut cases = Vec::with_capacity(names.len());
    for name in names {
        let dir = cases_dir.join(&name);
        let premise = dir.join("premise.rdf");
        if !premise.is_file() {
            return Err(format!("{}: missing premise", premise.display()));
        }
        let positive = dir.join(Direction::Positive.target_file());
        let negative = dir.join(Direction::Negative.target_file());
        let direction = match (positive.is_file(), negative.is_file()) {
            (true, false) => Direction::Positive,
            (false, true) => Direction::Negative,
            (true, true) => {
                return Err(format!(
                    "{}: carries BOTH conclusion.rdf and non-conclusion.rdf; a case is one \
                     direction or the other",
                    dir.display()
                ));
            }
            (false, false) => {
                return Err(format!(
                    "{}: carries neither conclusion.rdf nor non-conclusion.rdf",
                    dir.display()
                ));
            }
        };
        let target = if direction.expects_match() {
            positive
        } else {
            negative
        };
        cases.push(RlCase {
            name,
            premise,
            target,
            direction,
        });
    }
    Ok(cases)
}

/// Parse an RDF/XML document with PurRDF's first-party codec.
///
/// The base IRI is synthetic and `example.org`-scoped, per the repository's
/// no-fabricated-vocabulary rule. Every vendored document declares its own
/// `xml:base`, so the supplied base is not consulted (asserted by the corpus
/// tripwire in the harness).
fn parse(path: &Path, base: &str) -> Result<std::sync::Arc<purrdf_core::RdfDataset>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    purrdf::parse_dataset(&bytes, "application/rdf+xml", Some(base))
        .map_err(|e| format!("RDF/XML parse of {}: {e}", path.display()))
}

/// Answer one case, through the library's conclusion-directed entailment service.
///
/// This harness owns **no** reasoning of its own. It parses two documents and hands
/// them to [`purrdf_entail::entails()`], which resolves `owl:imports`, establishes the
/// premise's consistency, runs the `OWL-RL` chase, checks Theorem PR1's syntactic
/// precondition and matches the target graph into the closure. There is exactly one
/// blank-node matcher in this workspace and it is that one — a second copy here is how
/// a corpus comes to grade an implementation that is not the one callers get.
///
/// The empty [`ImportMap`](purrdf_entail::ImportMap) is the honest configuration for
/// this corpus: the upstream `all.rdf` export inlines no support document, so there is
/// nothing to resolve an import to, and a premise that imports one refuses by name
/// instead of being reasoned over as though the missing axioms said nothing.
#[must_use]
pub fn decide(case: &RlCase) -> Answer {
    let base = format!("http://example.org/w3c-owl2-rl/{}", case.name);
    let premise = match parse(&case.premise, &base) {
        Ok(dataset) => dataset,
        Err(e) => return Answer::Withheld(e),
    };
    let target = match parse(&case.target, &base) {
        Ok(dataset) => dataset,
        Err(e) => return Answer::Withheld(e),
    };
    match purrdf_entail::entails(
        &premise,
        &target,
        purrdf_entail::Regime::OwlRl,
        &purrdf_entail::ImportMap::new(),
    ) {
        Ok(EntailmentOutcome::Entailed(_)) => Answer::Entailed,
        Ok(EntailmentOutcome::NotEntailed(reason)) => Answer::NotEntailed(reason),
        Ok(EntailmentOutcome::Undecided(reason)) => Answer::Undecided(reason),
        Err(e) => Answer::Withheld(format!("OWL-RL entailment: {e}")),
    }
}

/// Answer `case` and grade the answer against its published direction.
///
/// # The two lanes ask different questions of the same answer
///
/// A **positive** case publishes an entailment, so PurRDF has to reach it: only
/// [`Answer::Entailed`] agrees, and both a refutation and an
/// [`Answer::Undecided`] are divergences that the ledger must name.
///
/// A **negative** case publishes a NON-entailment, and what this corpus can actually
/// grade there is *soundness*: deriving the non-conclusion would be PurRDF asserting
/// something W3C contradicts. Soundness is owed unconditionally — every rule of the
/// OWL 2 RL/RDF table is a valid inference over arbitrary RDF graphs, whatever the
/// premise's syntax — so the negative lane's pass condition is exactly "the closure was
/// computed and does not contain the target". [`Answer::NotEntailed`] and
/// [`Answer::Undecided`] both report precisely that, and they differ only in what they
/// additionally CLAIM: `NotEntailed` claims a refutation (which needs Theorem PR1's
/// precondition), `Undecided` claims nothing beyond the observation. So both agree here,
/// and [`Answer::Withheld`] — where no closure was computed at all, and the observation
/// was never made — does not.
///
/// That asymmetry is not a lenience introduced for the negative lane; it is the reason
/// the module docs above call that lane "a soundness gate with weak discriminating
/// power". The discrimination lives in the positive lane, where an `Undecided` is
/// scored as the capability gap it is.
#[must_use]
pub fn grade(case: &RlCase) -> Grade {
    match decide(case) {
        Answer::Withheld(why) => Grade::Withhold(why),
        Answer::Entailed if case.direction.expects_match() => Grade::Agree,
        Answer::NotEntailed(_) | Answer::Undecided(_) if !case.direction.expects_match() => {
            Grade::Agree
        }
        Answer::Undecided(reason) => Grade::Withhold(reason.to_string()),
        Answer::Entailed => Grade::Disagree {
            published: case.direction,
            miss: None,
        },
        Answer::NotEntailed(reason) => Grade::Disagree {
            published: case.direction,
            miss: Some(reason),
        },
    }
}

/// One graded case, kept so the harness can report and cross-check the ledger.
#[derive(Debug)]
pub struct GradedCase {
    /// The case name.
    pub name: String,
    /// The direction the W3C published.
    pub direction: Direction,
    /// How PurRDF's answer compared.
    pub grade: Grade,
    /// Its ledger entry, if it has one.
    pub ledgered: Option<RlGap>,
}

/// The whole corpus run.
#[derive(Debug, Default)]
pub struct RlSummary {
    /// Every case, in case-name order.
    pub cases: Vec<GradedCase>,
}

impl RlSummary {
    /// Cases that agreed with the published verdict and are not ledgered.
    #[must_use]
    pub fn agreed(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| matches!(c.grade, Grade::Agree) && c.ledgered.is_none())
            .count()
    }

    /// Cases that diverged (withheld or disagreed) and are ledgered.
    #[must_use]
    pub fn ledgered(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| !matches!(c.grade, Grade::Agree) && c.ledgered.is_some())
            .count()
    }

    /// Cases that diverged with NO ledger entry. A hard failure.
    #[must_use]
    pub fn unledgered(&self) -> Vec<&GradedCase> {
        self.cases
            .iter()
            .filter(|c| !matches!(c.grade, Grade::Agree) && c.ledgered.is_none())
            .collect()
    }

    /// Ledgered cases that now AGREE — a stale entry. A hard failure, so a closed
    /// gap must be removed from the table rather than left to rot.
    #[must_use]
    pub fn stale(&self) -> Vec<&GradedCase> {
        self.cases
            .iter()
            .filter(|c| matches!(c.grade, Grade::Agree) && c.ledgered.is_some())
            .collect()
    }

    /// How many cases were published in each direction, as `(positive,
    /// negative)`.
    #[must_use]
    pub fn by_direction(&self) -> (usize, usize) {
        let positive = self
            .cases
            .iter()
            .filter(|c| c.direction == Direction::Positive)
            .count();
        (positive, self.cases.len() - positive)
    }

    /// Ledgered divergences that name something PurRDF could fix inside the rule
    /// table, rather than a conclusion no conforming OWL 2 RL rule set reaches.
    ///
    /// Reported separately so the ledger's size is never mistaken for a to-do
    /// list: most of it describes the profile, and this is the part that does not.
    #[must_use]
    pub fn actionable(&self) -> usize {
        self.cases
            .iter()
            .filter(|c| {
                !matches!(c.grade, Grade::Agree) && c.ledgered.is_some_and(RlGap::is_actionable)
            })
            .count()
    }

    /// The single machine-readable line the conformance matrix can scrape.
    #[must_use]
    pub fn scoreboard_line(&self) -> String {
        format!(
            "OWL2-RL-ENTAILMENT: agreed {} ledgered {} unledgered {} stale {} total {} actionable {}",
            self.agreed(),
            self.ledgered(),
            self.unledgered().len(),
            self.stale().len(),
            self.cases.len(),
            self.actionable(),
        )
    }

    /// A per-gap tally of the ledger, in label order, for the run log.
    #[must_use]
    pub fn ledger_tally(&self) -> String {
        let mut counts: Vec<(&'static str, usize, bool)> = Vec::new();
        for case in &self.cases {
            let Some(gap) = case.ledgered else { continue };
            if matches!(case.grade, Grade::Agree) {
                continue;
            }
            if let Some(slot) = counts.iter_mut().find(|(l, _, _)| *l == gap.label()) {
                slot.1 += 1;
            } else {
                counts.push((gap.label(), 1, gap.is_unsound()));
            }
        }
        counts.sort_unstable();
        let mut out = String::new();
        for (label, n, unsound) in counts {
            let mark = if unsound { " [UNSOUND]" } else { "" };
            let _ = write!(out, "\n  {n:>3}  {label}{mark}");
        }
        out
    }

    /// A detailed report of everything that must fail the harness.
    #[must_use]
    pub fn failure_report(&self) -> String {
        let mut lines = Vec::new();
        for case in self.unledgered() {
            let detail = match &case.grade {
                Grade::Agree => unreachable!("agreeing cases are not unledgered"),
                Grade::Withhold(why) => format!("WITHHELD ({why})"),
                Grade::Disagree {
                    published: Direction::Positive,
                    miss,
                } => format!(
                    "W3C published a POSITIVE entailment and the OWL-RL closure does not contain \
                     the conclusion — the RL rule table is INCOMPLETE for a case W3C declared \
                     inside the RL profile ({})",
                    miss.as_ref()
                        .map_or_else(|| "no diagnosis".to_owned(), MissReason::summary)
                ),
                Grade::Disagree {
                    published: Direction::Negative,
                    ..
                } => "W3C published a NEGATIVE entailment and the OWL-RL closure DOES contain \
                      the non-conclusion — the RL rule table is UNSOUND"
                    .to_owned(),
            };
            lines.push(format!(
                "  • UNLEDGERED DIVERGENCE {}: {detail} — add it to LEDGER with a typed RlGap, \
                 or fix the rule table",
                case.name
            ));
        }
        for case in self.stale() {
            lines.push(format!(
                "  • STALE LEDGER ENTRY {}: it now AGREES with the published verdict — remove it \
                 from LEDGER",
                case.name
            ));
        }
        lines.join("\n")
    }
}

/// Grade every case under `root`.
///
/// # Errors
///
/// Returns a message if the corpus cannot be discovered, or if [`LEDGER`] names a
/// case that is not vendored (an entry for a case that no longer exists would
/// silently inflate the budget).
pub fn run(root: &Path) -> Result<RlSummary, String> {
    let cases = discover(root)?;
    for entry in LEDGER {
        if !cases.iter().any(|c| c.name == entry.case) {
            return Err(format!(
                "LEDGER names {:?}, which is not a vendored case under {}",
                entry.case,
                root.display()
            ));
        }
    }
    let mut summary = RlSummary::default();
    for case in &cases {
        summary.cases.push(GradedCase {
            name: case.name.clone(),
            direction: case.direction,
            grade: grade(case),
            ledgered: ledger_lookup(&case.name),
        });
    }
    Ok(summary)
}

/// Render the measured divergences as a paste-ready [`LEDGER`] skeleton.
///
/// Used by the `--ignored` regeneration path after a re-vendor. Every emitted
/// entry gets a `TypeMe` placeholder rather than a guessed [`RlGap`]: the point of
/// the ledger is the typed reason, and a machine cannot supply it.
#[must_use]
pub fn render_ledger_skeleton(summary: &RlSummary) -> String {
    let mut out = String::from(
        "// Paste into LEDGER and replace every `RlGap::TypeMe` with the\n// rule or construct \
         actually responsible.\n",
    );
    for case in &summary.cases {
        let detail = match &case.grade {
            Grade::Agree => continue,
            Grade::Withhold(why) => format!("withheld: {why}"),
            Grade::Disagree { published, miss } => format!(
                "published {} / {}",
                published.label(),
                miss.as_ref().map_or_else(
                    || "OWL-RL closure CONTAINS the target".to_owned(),
                    MissReason::summary
                )
            ),
        };
        let known = case
            .ledgered
            .map_or_else(|| "RlGap::TypeMe".to_owned(), |g| format!("RlGap::{g:?}"));
        let _ = write!(
            out,
            "\n// {detail}\nLedgerEntry {{ case: {:?}, gap: {known} }},",
            case.name
        );
    }
    out
}

// -------------------------------------------------------------------------
// The upstream census
// -------------------------------------------------------------------------

/// One row of `census.tsv`: an upstream W3C test case and its disposition here.
#[derive(Debug, Clone)]
pub struct CensusRow {
    /// The upstream `otest:identifier`.
    pub identifier: String,
    /// The slugified case directory name.
    pub case: String,
    /// The `otest:` test types the case declares, `;`-joined.
    pub otest_types: String,
    /// The `otest:semantics` values, `;`-joined.
    pub semantics: String,
    /// The `otest:profile` values, `;`-joined (`-` when none).
    pub profiles: String,
    /// The `otest:status`.
    pub status: String,
    /// The `otest:normativeSyntax` values, `;`-joined.
    pub normative_syntax: String,
    /// Whether the case carries an `otest:rdfXmlPremiseOntology`.
    pub premise: String,
    /// `conclusion`, `non-conclusion`, or `none`.
    pub conclusion: String,
    /// This case's disposition in the OWL 2 RL entailment corpus.
    pub rl_corpus: String,
    /// This case's disposition in the OWL 2 DL consistency corpus.
    pub dl_corpus: String,
    /// For a consistency-shaped case the DL corpus does **not** vendor: what
    /// PurRDF's `OWL-Direct` tableau actually did with it when probed. See
    /// `PROVENANCE.md` for the measurement's date and per-case budget.
    ///
    /// This is the column that makes the DL corpus's exclusions auditable: a
    /// reader can count, by name, how many upstream cases were left out and how
    /// many of those the reasoner genuinely cannot decide.
    pub dl_probe: String,
}

/// The census's column header, asserted so a re-vendor cannot silently reshape it.
pub const CENSUS_HEADER: &str = "identifier\tcase\totest_types\tsemantics\tprofiles\tstatus\t\
     normative_syntax\tpremise\tconclusion\trl_corpus\tdl_corpus\tdl_probe";

/// Read `census.tsv` from a vendored root.
///
/// # Errors
///
/// Returns a message if the file cannot be read, if its header is not
/// [`CENSUS_HEADER`], or if any row does not have exactly twelve columns.
pub fn read_census(root: &Path) -> Result<Vec<CensusRow>, String> {
    let path = root.join("census.tsv");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut lines = text.lines();
    let header = lines.next().unwrap_or_default();
    if header != CENSUS_HEADER {
        return Err(format!(
            "{}: header is {header:?}, expected {CENSUS_HEADER:?}",
            path.display()
        ));
    }
    let mut rows = Vec::new();
    for (n, line) in lines.enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        let [
            identifier,
            case,
            otest_types,
            semantics,
            profiles,
            status,
            normative_syntax,
            premise,
            conclusion,
            rl_corpus,
            dl_corpus,
            dl_probe,
        ] = f[..]
        else {
            return Err(format!(
                "{}:{}: {} columns, expected 12",
                path.display(),
                n + 2,
                f.len()
            ));
        };
        rows.push(CensusRow {
            identifier: identifier.to_owned(),
            case: case.to_owned(),
            otest_types: otest_types.to_owned(),
            semantics: semantics.to_owned(),
            profiles: profiles.to_owned(),
            status: status.to_owned(),
            normative_syntax: normative_syntax.to_owned(),
            premise: premise.to_owned(),
            conclusion: conclusion.to_owned(),
            rl_corpus: rl_corpus.to_owned(),
            dl_corpus: dl_corpus.to_owned(),
            dl_probe: dl_probe.to_owned(),
        });
    }
    Ok(rows)
}

/// Tally a census column into `(value, count)` pairs in value order.
#[must_use]
pub fn census_tally(rows: &[CensusRow], column: fn(&CensusRow) -> &str) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows {
        *counts.entry(column(row)).or_default() += 1;
    }
    counts.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_has_no_duplicate_cases() {
        let mut names: Vec<&str> = LEDGER.iter().map(|e| e.case).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(count, names.len(), "LEDGER holds a duplicate case entry");
    }

    #[test]
    fn no_ledgered_gap_is_an_unsoundness() {
        let unsound: Vec<&str> = LEDGER
            .iter()
            .filter(|e| e.gap.is_unsound())
            .map(|e| e.case)
            .collect();
        assert!(
            unsound.is_empty(),
            "the set of unsound divergences must be EMPTY — an unsoundness is the OWL-RL chase \
             deriving a triple the W3C says is NOT entailed, which no incompleteness can excuse. \
             These claim otherwise and must be reviewed by hand: {unsound:?}"
        );
    }

    #[test]
    fn only_missing_rules_are_actionable() {
        // The ledger's size is not a to-do list. Exactly the entries that name a
        // sound, in-shape rule the table omits (or an unsoundness) are actionable;
        // everything else describes what the OWL 2 RL profile itself cannot reach.
        let actionable: Vec<&str> = LEDGER
            .iter()
            .filter(|e| e.gap.is_actionable())
            .map(|e| e.case)
            .collect();
        assert_eq!(
            actionable,
            [] as [&str; 0],
            "the actionable set changed; a new entry means the rule table owes a conclusion it \
             does not produce, and a lost entry means one was fixed — either way, say so in the \
             ledger comment and here"
        );
    }

    #[test]
    fn direction_and_target_file_agree() {
        assert_eq!(Direction::Positive.target_file(), "conclusion.rdf");
        assert_eq!(Direction::Negative.target_file(), "non-conclusion.rdf");
        assert!(Direction::Positive.expects_match());
        assert!(!Direction::Negative.expects_match());
    }
}
