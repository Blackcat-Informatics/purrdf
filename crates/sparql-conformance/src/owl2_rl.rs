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
//! `purrdf_entail::entails()` reaches a conclusion seven ways, and fifteen of the cases
//! graded here are reached only by one of the five beyond matching. A sixteenth
//! needed no new mechanism at all, only the document its premise names — see
//! [`vendored_imports`].
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
//! * **Datatype containment.** A property's declared `rdfs:range` datatypes
//!   INTERSECT, and the intersection may be contained in a datatype the premise
//!   never mentions. Deciding that needs the XSD value spaces rather than a join
//!   over triples. Three cases (`webont-i5-8-006`, `-008`, `-009`), all of them
//!   WIDENINGS — `xsd:byte ⊑ xsd:short` — which is why they are sound.
//!
//! All five are worth separating from the extension above because they are a
//! different kind of thing. `ext-eq-diff-sym` is a rule PurRDF states that no
//! specification does, and it is declared as such. None of these mechanisms states
//! anything: the rule inventory is byte-for-byte the same seventy-eight before and
//! after, and the five `the_*_lane_adds_no_rule` tests in `purrdf-entail` assert it.
//! What changed is how many times the table is run, what it is run over, and what the
//! run's `false` is read as.
//!
//! * **Composite.** A conclusion GRAPH is a conjunction and entailment is monotone
//!   over one, so a conclusion stating a negative fact BESIDE a schema axiom is
//!   entailed when each half is. `purrdf_entail::entails()` therefore threads the
//!   residual through every lane in turn and matches only what survives; an answer
//!   that needed two or more of them reports `composite` rather than any single
//!   constituent's name. No vendored case needs it — each of the fifty is reached by
//!   one lane or by none — which is why its column reads `0/0` and why the column is
//!   printed anyway.
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
//! # `negative 23/23` is twenty-three agreements of TWO kinds, and the scoreboard says which
//!
//! Three of the twenty-three are DECIDED non-entailments: both halves of Theorem PR1's
//! hypothesis hold, so the closure's failure to contain the non-conclusion is a proof and
//! not merely a failure to find something. They are `new-feature-keys-004`,
//! `webont-imports-002` and `webont-miscellaneous-301`.
//!
//! The other twenty agree by ADMISSION. The closure was computed and does not contain the
//! non-conclusion — the soundness observation, in full — and the run claims nothing beyond
//! it, naming the entitlement it lacks: five because the premise is outside the RL syntax,
//! ten because the non-conclusion is not an assertional graph over named terms, five
//! because a lane recognised a construct of it and declined to read it.
//!
//! Both kinds AGREE, and that is the correct grading rather than a lenience. Soundness is
//! owed unconditionally — every rule of the table is a valid inference over arbitrary RDF
//! graphs, whatever the premise's syntax — so an `Undecided` reports the graded claim in
//! full. What the two kinds differ in is DISCRIMINATING POWER: a reasoner that derived
//! nothing at all would land all twenty-three in the admission buckets and still read
//! `negative 23/23`. So the composition is printed rather than left to be inferred —
//! [`RlSummary::negative_lane_line`] carries the whole split and
//! [`RlSummary::mechanism_line`] carries its two-way form beside the total, both DERIVED
//! from the answers through [`Disposition`] — and the per-case table lives in
//! `every_negative_case_is_answered_under_an_unexhausted_certificate`.
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

pub use purrdf_entail::{
    EntailmentCertificate, EntailmentMechanism, EntailmentOutcome, MissReason, UndecidedReason,
};

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
/// doubles as an inventory of what the rule table does not reach. A catch-all
/// would defeat the point.
///
/// # The table is EMPTY and every variant stays
///
/// [`LEDGER`] holds nothing: all fifty vendored cases answer as W3C published them. The
/// seven variants below are kept anyway, and the reason is the same one for each — a
/// classification is what a divergence of that SHAPE is filed under, and deleting it
/// would leave the next one with nowhere to go that says what it is. That is a statement
/// about the taxonomy, not a schedule: nothing here is owed, and none of these variants
/// is waiting for anything. [`RlGap::ALL`] is what
/// `every_gap_classifies_itself_on_both_axes` ranges over, so the classifications stay
/// checked while the ledger is empty.
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
    /// operands. So did an `rdfs:range` over a DATATYPE, which follows from the
    /// containment of the intersection of the premise's own declared ranges — a
    /// question about XSD value spaces rather than about triples.
    ///
    /// The ledger holds no entry of this kind today. The variant stays because a
    /// conclusion belongs here when nothing it can be decomposed into has a head in
    /// the table either, and nothing licenses it directly — which is still the right
    /// classification for a schema shape none of the mechanisms above reads.
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
    /// is vendored.
    ///
    /// The ledger holds no entry of this kind today, and the one it did hold is
    /// the reason this variant's fix was never a reasoning change: the missing
    /// document is vendored under `imports/` and supplied to the service as
    /// caller-owned configuration. The variant stays because a support document
    /// upstream does not publish at a fetchable URL would land here, and because it
    /// is the only [`RlGap`] that describes the CORPUS rather than the profile.
    ImportsUnresolved,
    /// The premise or target RDF/XML did not parse, or the chase returned an
    /// error, so the run refused to decide.
    Refused,
    /// The chase derived a triple the W3C says is **not** entailed. This is an
    /// unsoundness: PurRDF asserted a conclusion it is not entitled to.
    UnsoundDerivation,
}

impl RlGap {
    /// Every classification, in declaration order.
    ///
    /// The list a retained variant is read off, and the reason `every_gap_classifies_itself`
    /// cannot go vacuous: it ranges over THIS rather than over [`LEDGER`], so an empty
    /// ledger does not empty it.
    pub const ALL: [Self; 7] = [
        Self::MissingRule,
        Self::SchemaConclusion,
        Self::NegativeConclusion,
        Self::ConstructOutsideRl,
        Self::ImportsUnresolved,
        Self::Refused,
        Self::UnsoundDerivation,
    ];

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
/// It is EMPTY. Every one of the 50 vendored cases now answers as W3C published
/// it, so this table holds no entry and the commentary below is the record of what
/// each closed class was and how it closed — kept because the classification is
/// still what a future divergence must be filed under, and because a reader who
/// finds an empty ledger is owed the argument rather than the absence of one.
///
/// Nothing is skipped at discovery time — all 50 cases run, and a case absent
/// from this table must agree, which is now every case.
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
    // --- NO SCHEMA CONCLUSION EITHER. That block is CLOSED, all eight of it. ---
    //     Every head in OWL 2 RL/RDF's rule table is an assertional triple over
    //     named terms or `false`; not one concludes an axiom, and that is still
    //     true. What was too strong was the conclusion drawn from it — that a
    //     conclusion of this shape is therefore unreachable. It is reachable when
    //     the axiom DECOMPOSES into something the table does have a head for, or
    //     when the semantics licenses it outright.
    //
    //     Two went with the six above:
    //     `new-feature-disjoint{data,object}properties-002` conclude an
    //     `owl:AllDifferent` collection, which reads as a schema axiom and IS the
    //     conjunction of its `n(n−1)/2` pairwise inequalities — so it lowers to the
    //     negative facts of the previous block and is reached the same way, one
    //     refutation per pair, all of them required.
    //
    //     `chain2trans1` concludes `p rdf:type
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
    //     Two more are `webont-i5-5-005`, whose conclusion is an anonymous
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
    //
    //     The last three are `webont-i5-8-006`, `-008` and `-009`, which conclude an
    //     `rdfs:range` WIDENED to a containing XSD datatype. WIDENED, not narrowed:
    //     `xsd:byte ⊑ xsd:short`, which is why they are sound at all — the narrowing
    //     direction would be an unsoundness. A property's declared ranges INTERSECT,
    //     so `-008` needs `short ⊓ unsignedInt ⊑ unsignedShort` and `-009` needs
    //     `nonNegativeInteger ⊓ nonPositiveInteger = {0} ⊑ short`, neither of which
    //     is a containment between any two of the datatypes named. Deciding that
    //     needs the XSD value spaces, which a rule table has no arithmetic for, so
    //     it is decided by `purrdf_xsd::range::containment` — three-valued, with the
    //     NEGATIVE answer gated on the counterexample range being exactly decided,
    //     because a `bool`-shaped answer would read "cannot say" as "not entailed".
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
    // --- NO IMPORTS UNRESOLVED EITHER. That block is CLOSED, and it is the last.
    //     `webont-imports-011`'s premise `owl:imports` `support011-A`, and the
    //     conclusion is about a class defined only in that support document. The
    //     upstream `all.rdf` export does not carry it inline — an
    //     `otest:rdfXmlPremiseOntology` literal is ONE document — so no reasoner
    //     could reach the conclusion from the vendored bytes, and the entry said so.
    //
    //     The answer was not to reason harder but to vendor the missing document.
    //     `imports/support011-A.rdf` is W3C's, fetched from its own URL with its
    //     date and digest recorded in `PROVENANCE.md`, and it lives OUTSIDE `cases/`
    //     because `census_accounts_for_every_upstream_case` requires every case
    //     directory to have a census row and a support ontology is not a test case.
    //     `vendored_imports` keys it by the ontology IRI the document itself
    //     declares — not by its file name, which nothing would enforce — and hands
    //     it to `entails()` as caller-supplied CONFIGURATION, which is the only way
    //     this library ever learns what an ontology IRI denotes. It fetches nothing.
    //
    //     The resolution is transitive to a fixpoint, so `support011-A`'s own
    //     imports would be followed too; a resolver stopping at depth one would
    //     reason over a partial premise, which is the exact failure the import lane
    //     exists to prevent.
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

/// What a case's answer WAS, reduced to a bucket a scoreboard can tally.
///
/// [`Answer`] carries payloads and is consumed by the grading, and [`Grade`] throws the
/// answer's *kind* away — `Agree` is one word for "reached the conclusion" and for "did not
/// reach it and is not entitled to say so". This is the part [`Grade`] discards, kept as a
/// fixed, enumerable set of buckets with stable labels so the negative lane's composition
/// can be printed in a fixed order with its empty buckets included.
///
/// # Why the six `Undecided` buckets are spelled out
///
/// [`UndecidedReason`] has nine inhabitants and the `OWL-RL` lane produces exactly these
/// six: three are `RDF`/`RDFS`/`D` preconditions this corpus never runs. Folding the six
/// into one `Undecided` bucket would print `admitted 20` and stop, which is the same
/// silence one level down; carrying all nine would print three buckets that cannot become
/// non-zero, which is noise rather than disclosure. So [`Self::classify`] is a total match
/// that ERRORS on the other three rather than filing one somewhere it does not belong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// [`Answer::Entailed`] — the closure contains the target graph. The published answer
    /// on a positive case; an unsoundness on a negative one.
    Entailed,
    /// [`Answer::NotEntailed`] — a **decided** non-entailment. Both halves of Theorem PR1's
    /// hypothesis hold, so the closure's failure to contain the target is a refutation and
    /// not merely a failure to find something.
    Refuted,
    /// `Undecided(PremiseOutsideRl)` — the premise is outside the OWL 2 RL syntax.
    PremiseOutsideRl,
    /// `Undecided(ConclusionOutsideRl)` — the target is not an assertional graph over named
    /// terms, so no head in Tables 4–9 has its shape and the closure's silence about it was
    /// never evidence.
    ConclusionOutsideRl,
    /// `Undecided(ConstructNotRead)` — a lane recognised a construct of the target and
    /// declined to read it.
    ConstructNotRead,
    /// `Undecided(RefutationBudget)` — the refutation lane did not finish.
    RefutationBudget,
    /// `Undecided(FreezeBudget)` — the freeze-and-chase lane did not finish.
    FreezeBudget,
    /// `Undecided(DataRangeContainment)` — the datatype decision procedure did not decide a
    /// containment.
    DataRangeContainment,
    /// [`Answer::Withheld`] — no closure was computed at all, so the soundness observation
    /// the negative lane grades was never made.
    Withheld,
}

impl Disposition {
    /// Every bucket, in declaration order.
    ///
    /// The list `every_disposition_classifies_itself` ranges over, so
    /// [`Self::ADMISSIONS`] cannot drift away from [`Self::is_admission`]: one of them is a
    /// print order and the other is a predicate, and a bucket that appeared in one but not
    /// the other would make the scoreboard's `admitted` total disagree with the buckets
    /// printed under it.
    pub const ALL: [Self; 9] = [
        Self::Entailed,
        Self::Refuted,
        Self::PremiseOutsideRl,
        Self::ConclusionOutsideRl,
        Self::ConstructNotRead,
        Self::RefutationBudget,
        Self::FreezeBudget,
        Self::DataRangeContainment,
        Self::Withheld,
    ];

    /// The six `Undecided` buckets, in the fixed order the scoreboard prints them.
    ///
    /// An agreement in one of these is an ADMISSION: the closure was computed and does not
    /// contain the target — the whole of the soundness observation — with no entitlement to
    /// call that a refutation, and with the missing entitlement named.
    pub const ADMISSIONS: [Self; 6] = [
        Self::PremiseOutsideRl,
        Self::ConclusionOutsideRl,
        Self::ConstructNotRead,
        Self::RefutationBudget,
        Self::FreezeBudget,
        Self::DataRangeContainment,
    ];

    /// The token the scoreboard prints for this bucket.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Entailed => "entailed",
            Self::Refuted => "refuted",
            Self::PremiseOutsideRl => "premise-outside-rl",
            Self::ConclusionOutsideRl => "conclusion-outside-rl",
            Self::ConstructNotRead => "construct-not-read",
            Self::RefutationBudget => "refutation-budget",
            Self::FreezeBudget => "freeze-budget",
            Self::DataRangeContainment => "data-range-containment",
            Self::Withheld => "withheld",
        }
    }

    /// Whether an agreement in this bucket is an ADMISSION rather than a decided answer.
    #[must_use]
    pub const fn is_admission(self) -> bool {
        match self {
            Self::PremiseOutsideRl
            | Self::ConclusionOutsideRl
            | Self::ConstructNotRead
            | Self::RefutationBudget
            | Self::FreezeBudget
            | Self::DataRangeContainment => true,
            Self::Entailed | Self::Refuted | Self::Withheld => false,
        }
    }

    /// Which bucket `answer` falls in.
    ///
    /// # Errors
    ///
    /// Returns a message if the answer is an `Undecided` for one of the three reasons only
    /// the `RDF`, `RDFS` or `D` regimes produce. This corpus runs `Regime::OwlRl` and has no
    /// bucket for those, and inventing one — or quietly folding it into a neighbour — would
    /// print a number that means something other than what its label says.
    pub fn classify(answer: &Answer) -> Result<Self, String> {
        Ok(match answer {
            Answer::Entailed => Self::Entailed,
            Answer::NotEntailed(_) => Self::Refuted,
            Answer::Withheld(_) => Self::Withheld,
            Answer::Undecided(reason) => match reason {
                UndecidedReason::PremiseOutsideRl(_) => Self::PremiseOutsideRl,
                UndecidedReason::ConclusionOutsideRl(_) => Self::ConclusionOutsideRl,
                UndecidedReason::ConstructNotRead { .. } => Self::ConstructNotRead,
                UndecidedReason::RefutationBudget(_) => Self::RefutationBudget,
                UndecidedReason::FreezeBudget(_) => Self::FreezeBudget,
                UndecidedReason::DataRangeContainment(_) => Self::DataRangeContainment,
                other @ UndecidedReason::OpenPredicate(_) => {
                    return Err(format!(
                        "the OWL-RL lane answered Undecided({other}), which only a basic graph \
                         PATTERN can state: a conclusion graph's predicates are all IRIs, and \
                         this corpus asks conclusion-directed questions and projects nothing. \
                         Classify it here deliberately rather than letting it be counted as an \
                         admission this corpus measured"
                    ));
                }
                other @ (UndecidedReason::WithheldSurrogate(_)
                | UndecidedReason::AxiomaticSchema(_)
                | UndecidedReason::DatatypeValueSpace) => {
                    return Err(format!(
                        "the OWL-RL lane answered Undecided({other}), which is a reason only \
                         the RDF, RDFS or D regimes state; this corpus runs Regime::OwlRl and \
                         has no scoreboard bucket for it, so classify it here deliberately \
                         rather than letting it be counted as something else"
                    ));
                }
                // `UndecidedReason` is `#[non_exhaustive]`, so this arm is owed from OUTSIDE
                // the crate that defines it — the arms above can no longer be proven total
                // here. It refuses rather than folding, for the same reason every arm above
                // that refuses does: a bucket is what a number's label means, and a reason
                // with no bucket counted as a neighbour prints a figure that is not what it
                // says it is.
                //
                // The compile-time force is NOT lost, only moved to where the variant is
                // added: `UndecidedReason`'s `Display` is a total match in its own crate,
                // which `#[non_exhaustive]` does not relax, so a new reason fails to build
                // there first. This is the second gate, and it fails the corpus run by name
                // rather than silently.
                other => {
                    return Err(format!(
                        "the OWL-RL lane answered Undecided({other}), a reason this corpus \
                         has no scoreboard bucket for. It was added since these buckets were \
                         written: give it one, or refuse it beside the reasons above and say \
                         why — do not let it be counted as something else"
                    ));
                }
            },
        })
    }
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
/// "Exactly" below is CHECKED rather than described. A case directory holds a
/// `premise.rdf` and exactly one of `conclusion.rdf` / `non-conclusion.rdf`, and
/// any other entry — a stray support document, an editor backup, a second target
/// under a third name — is a hard error. The corpus is a byte-frozen payload
/// whose inventory is asserted, so a file nothing reads is either a re-vendor
/// that went wrong or a payload the grader is silently ignoring, and both are
/// things to be told about.
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
        // …and NOTHING else. See this function's doc for why an unread file is an
        // error rather than a shrug.
        let expected = ["premise.rdf", direction.target_file()];
        let mut extra: Vec<String> = Vec::new();
        for entry in
            std::fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        {
            let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !expected.contains(&name.as_str()) {
                extra.push(name);
            }
        }
        if !extra.is_empty() {
            extra.sort();
            return Err(format!(
                "{}: holds {extra:?} beside its premise and {}; a case directory holds \
                 exactly those two files, and a file the grader does not read is either a \
                 re-vendor that went wrong or payload being silently ignored",
                dir.display(),
                direction.target_file()
            ));
        }
        cases.push(RlCase {
            name,
            premise,
            target,
            direction,
        });
    }
    Ok(cases)
}

/// The vendored support documents an `owl:imports` names, keyed by ontology IRI.
///
/// # Why they live OUTSIDE `cases/`
///
/// Because `census_accounts_for_every_upstream_case` requires every directory under
/// `cases/` to have a census row, and a support document is not a test case: it has
/// no `otest:identifier`, no direction and no published verdict. Putting it under
/// `cases/` would either break that cross-check or force a fabricated census row,
/// and both are worse than a second directory.
///
/// # Why the key is read from the document rather than from its file name
///
/// An `owl:imports` names an ONTOLOGY IRI, and the document itself is what says
/// which ontology it is: its `owl:Ontology` subject. Deriving the key from the file
/// name would be a naming convention nothing enforces, and a re-vendor that renamed
/// a file would silently stop resolving an import — which is exactly the failure the
/// whole import lane exists to make loud. So the ontology declaration is read, and a
/// document with no `owl:Ontology` subject, more than one, or a blank-node one is a
/// hard error.
///
/// # Errors
///
/// Returns a message if `imports/` cannot be read, if a document does not parse, or
/// if a document does not declare exactly one named ontology.
pub fn vendored_imports(root: &Path) -> Result<purrdf_entail::ImportMap, String> {
    let dir = root.join("imports");
    let mut map = purrdf_entail::ImportMap::new();
    if !dir.is_dir() {
        return Ok(map);
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in
        std::fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?
    {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        paths.push(entry.path());
    }
    paths.sort();
    for path in paths {
        // The base is the same synthetic `example.org` one every vendored document is
        // parsed under, and it is not consulted: the tripwire below asserts that every
        // vendored document either declares its own `xml:base` or uses only absolute
        // IRIs.
        let base = format!(
            "http://example.org/w3c-owl2-rl/imports/{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        let document = parse(&path, &base)?;
        let iri = ontology_iri(&document).ok_or_else(|| {
            format!(
                "{}: does not declare exactly one named owl:Ontology, so \
                 there is no ontology IRI for an owl:imports to name it by",
                path.display()
            )
        })?;
        if map.insert(iri.clone(), document).is_some() {
            return Err(format!(
                "{}: two vendored support documents both declare the ontology {iri}",
                path.display()
            ));
        }
    }
    Ok(map)
}

/// The one named `owl:Ontology` subject of `ds`, or `None` if it does not have exactly one.
fn ontology_iri(ds: &purrdf_core::RdfDataset) -> Option<String> {
    let ty = ds.term_id_by_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
    let ontology = ds.term_id_by_iri("http://www.w3.org/2002/07/owl#Ontology")?;
    let mut found: Option<String> = None;
    for quad in ds.quads().filter(|quad| quad.p == ty && quad.o == ontology) {
        let purrdf_core::TermValue::Iri(iri) = ds.term_value(quad.s) else {
            return None;
        };
        if found.replace(iri).is_some() {
            return None;
        }
    }
    found
}

/// Every vendored RDF/XML document under `root`, in path order.
///
/// The whole payload, not the case documents alone: a base-independence or licensing
/// claim about "the vendored documents" that swept only `cases/` would stop being
/// checked the moment a payload arrived anywhere else, which is precisely what
/// happened when `imports/` did.
///
/// # Errors
///
/// Returns a message if any directory under `root` cannot be read.
pub fn vendored_documents(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in
            std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        {
            let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else if path.extension().is_some_and(|ext| ext == "rdf") {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, &mut out)?;
    out.sort();
    Ok(out)
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
/// `imports` is the corpus's own [`vendored_imports`] map. The upstream `all.rdf`
/// export inlines no support document — an `otest:rdfXmlPremiseOntology` literal is one
/// document, and an `owl:imports` in it names another — so a premise that imports one
/// used to refuse by name. It no longer has to: the support documents are vendored
/// beside the cases, from W3C's own URLs, and handed to the service as CONFIGURATION.
/// A premise that imports something the map does not resolve still refuses by name
/// rather than being reasoned over as though the missing axioms said nothing.
#[must_use]
pub fn decide(case: &RlCase, imports: &purrdf_entail::ImportMap) -> Answer {
    match certify(case, imports) {
        Ok(certificate) => match certificate.into_parts().0 {
            EntailmentOutcome::Entailed(_) => Answer::Entailed,
            EntailmentOutcome::NotEntailed(reason) => Answer::NotEntailed(reason),
            EntailmentOutcome::Undecided(reason) => Answer::Undecided(reason),
        },
        Err(why) => Answer::Withheld(why),
    }
}

/// Answer one case and hand back the WHOLE certificate — the verdict, the mechanism that
/// reached it, and the report of the chase underneath.
///
/// [`decide`] is this with everything but the verdict thrown away, and it is the narrower
/// call because most of the grading needs only the verdict. What this adds is the two facts
/// the grading cannot see: WHICH of [`purrdf_entail::entails()`]'s seven mechanisms answered, and
/// what the run that answered it did. A corpus that graded fifty verdicts without ever
/// naming the mechanism would report the same green whether the profile's own rule table
/// reached them or a second run over the premise's negation did, which is the distinction
/// this whole entailment surface is about.
///
/// # Errors
///
/// Returns a message if either document fails to parse, or if the service refuses — an
/// unresolvable `owl:imports`, an inconsistent premise, an exhausted ceiling. Those are the
/// [`Answer::Withheld`] cases, and they have no certificate because no run completed.
pub fn certify(
    case: &RlCase,
    imports: &purrdf_entail::ImportMap,
) -> Result<EntailmentCertificate, String> {
    let base = format!("http://example.org/w3c-owl2-rl/{}", case.name);
    let premise = parse(&case.premise, &base)?;
    let target = parse(&case.target, &base)?;
    purrdf_entail::entails(&premise, &target, purrdf_entail::Regime::OwlRl, imports)
        .map_err(|e| format!("OWL-RL entailment: {e}"))
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
pub fn grade(case: &RlCase, imports: &purrdf_entail::ImportMap) -> Grade {
    grade_answer(case, decide(case, imports))
}

/// Grade an answer already obtained, so one run can be read twice.
///
/// [`grade`] is this over its own [`decide`]. It is split out because [`run`] needs BOTH
/// the grade and the MECHANISM, and calling `decide` a second time to get the second would
/// chase the premise twice per case — and, worse, would let the two halves of one row come
/// from two different runs.
#[must_use]
fn grade_answer(case: &RlCase, answer: Answer) -> Grade {
    match answer {
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
    /// WHICH answer it was, as the bucket the scoreboard tallies.
    ///
    /// Kept beside [`Self::grade`] rather than derived from it, because it is exactly what
    /// the grade throws away: on the negative lane `Grade::Agree` is one word for a decided
    /// refutation and for an admission, and a scoreboard that only had the grade could not
    /// tell a reader which of the two it counted.
    pub disposition: Disposition,
    /// Its ledger entry, if it has one.
    pub ledgered: Option<RlGap>,
    /// WHICH of [`purrdf_entail::entails()`]'s seven mechanisms answered it.
    ///
    /// `None` only for a WITHHELD case, where no run completed and there is therefore no
    /// mechanism to name — an absence this corpus has none of today and which the type
    /// still has to be able to say.
    pub mechanism: Option<EntailmentMechanism>,
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

    /// The MECHANISM × PR1-CLAUSE scoreboard: a second machine-readable line.
    ///
    /// [`Self::scoreboard_line`] answers "did the corpus agree?" and, with an EMPTY
    /// ledger, answers it by subtracting zero from fifty — trivially correct and
    /// therefore near-vacuous. This answers the question that is not trivial: WHICH
    /// mechanism ANSWERED each case, split by the clause of Theorem PR1 the lane grades.
    ///
    /// # The two halves of the line count different things, deliberately
    ///
    /// The leading `positive P/T negative N/T` pair is the AGREEMENT count: it filters on
    /// [`Grade::Agree`], so it is the corpus's score. Everything after it — the
    /// per-mechanism buckets and `withheld` — is a CENSUS of the corpus by who answered,
    /// and it does NOT filter on the grade: a case one mechanism answered wrongly is still
    /// that mechanism's case. That is what makes the tail's arithmetic checkable, because
    /// `mechanism` is `Some` for exactly the cases that were answered, so the buckets plus
    /// `withheld` sum to the corpus with nothing unaccounted for. Filtering the buckets on
    /// agreement would break that identity and hide a disagreement's mechanism — which is
    /// the first thing a reader chasing one needs.
    ///
    /// With the corpus at 50 agreed of 50 the two readings coincide, and the difference is
    /// only visible once something disagrees. It is stated here rather than left to be
    /// rediscovered then.
    ///
    /// The two lanes grade different halves of that theorem. The POSITIVE lane grades
    /// the completeness half — W3C published an entailment and PurRDF has to reach it —
    /// and it is where the discrimination lives, because a reasoner that derives nothing
    /// at all fails every one of the 27. The NEGATIVE lane grades the soundness half —
    /// deriving one of these would be asserting something W3C contradicts — which is
    /// owed unconditionally and has weak discriminating power. Reporting one number for
    /// both would hide which half a change moved.
    ///
    /// Per mechanism the pair is `<positive>/<negative>`, over the cases that mechanism
    /// ANSWERED. A POSITIVE count on any of the six beyond `strict-table` is a conclusion
    /// that mechanism reached — and, while the corpus agrees, one it ESTABLISHED. A
    /// NEGATIVE count on one is not an unsoundness and is not a better score either: none
    /// of them ever refutes, so what a lane's name on a negative case reports is that the
    /// lane RECOGNIZED a construct of the non-conclusion and ADMITTED it could not read it,
    /// which reaches the caller as `Undecided` naming the construct. Printing the split is
    /// what makes both readable rather than inferable — a case moving from the table's
    /// bucket to a lane's is a case that stopped claiming a refutation it was not entitled
    /// to.
    ///
    /// The mechanism is spelled by its own `as_str`, and mechanisms with no case are
    /// still printed, because a lane dropping to zero is exactly the kind of change a
    /// line that only listed non-empty buckets would render invisible.
    ///
    /// # `negative N/N` carries its composition, because it is not one kind of result
    ///
    /// A negative agreement is either a DECIDED non-entailment or an ADMISSION, and
    /// printing one number for both is the same silence this line exists to break one level
    /// up. So the negative pair is followed by `(refuted R, admitted A)`, both DERIVED from
    /// the answers via [`Disposition`] and summing to the agreement count. The admissions'
    /// own split by reason is a line of its own — [`Self::negative_lane_line`] — because
    /// six more buckets here would make this one unreadable.
    #[must_use]
    pub fn mechanism_line(&self) -> String {
        let (positive_total, negative_total) = self.by_direction();
        let mut positive_agree = 0_usize;
        let mut negative_agree = 0_usize;
        for case in &self.cases {
            if !matches!(case.grade, Grade::Agree) {
                continue;
            }
            if case.direction == Direction::Positive {
                positive_agree += 1;
            } else {
                negative_agree += 1;
            }
        }
        let refuted = self.negative_agreements(Disposition::Refuted);
        let admitted: usize = Disposition::ADMISSIONS
            .into_iter()
            .map(|bucket| self.negative_agreements(bucket))
            .sum();
        let mut out = format!(
            "OWL2-RL-MECHANISMS: positive {positive_agree}/{positive_total} negative \
             {negative_agree}/{negative_total} (refuted {refuted}, admitted {admitted})"
        );
        for mechanism in EntailmentMechanism::ALL {
            let (mut positive, mut negative) = (0_usize, 0_usize);
            for case in &self.cases {
                if case.mechanism != Some(mechanism) {
                    continue;
                }
                if case.direction == Direction::Positive {
                    positive += 1;
                } else {
                    negative += 1;
                }
            }
            let _ = write!(out, " {} {positive}/{negative}", mechanism.as_str());
        }
        // A case whose run did not complete has no mechanism, so the buckets above do
        // not sum to the corpus. Printing the residue keeps the line's own arithmetic
        // checkable rather than leaving a reader to discover the shortfall.
        let unanswered = self
            .cases
            .iter()
            .filter(|case| case.mechanism.is_none())
            .count();
        let _ = write!(out, " withheld {unanswered}");
        out
    }

    /// How many NEGATIVE cases agreed with the published verdict from `bucket`.
    ///
    /// Filtered on the grade as well as the bucket, so this counts agreements rather than
    /// answers — an `Entailed` on a negative case is an unsoundness and must never be summed
    /// into the same total as a refutation.
    #[must_use]
    fn negative_agreements(&self, bucket: Disposition) -> usize {
        self.cases
            .iter()
            .filter(|case| {
                case.direction == Direction::Negative
                    && matches!(case.grade, Grade::Agree)
                    && case.disposition == bucket
            })
            .count()
    }

    /// How many NEGATIVE cases fell in `bucket` however they graded.
    #[must_use]
    fn negative_answers(&self, bucket: Disposition) -> usize {
        self.cases
            .iter()
            .filter(|case| case.direction == Direction::Negative && case.disposition == bucket)
            .count()
    }

    /// The NEGATIVE lane's composition: what the `negative N/N` pair is MADE OF.
    ///
    /// # Why this line exists
    ///
    /// `negative 23/23` reads as twenty-three of one thing and is twenty-three of two. A
    /// negative case agrees when the closure was computed and does not contain the
    /// non-conclusion, and that observation — which is the whole of the soundness claim the
    /// lane grades — is owed unconditionally and is genuinely made in every one of them. What
    /// differs is what may be CLAIMED beyond it:
    ///
    /// * **refuted** — both halves of Theorem PR1's hypothesis hold (the premise is inside
    ///   the OWL 2 RL syntax AND the non-conclusion is an assertional graph over named
    ///   terms), so the absence of a match IS a proof of non-entailment. This is the bucket
    ///   with discriminating power, and it is the only one that would notice a reasoner
    ///   deriving too little;
    /// * **admitted** — the observation was made and nothing beyond it is claimed, with the
    ///   missing entitlement NAMED. An admission is a correct agreement and a weak one: a
    ///   reasoner that derived nothing at all would land every case here.
    ///
    /// So the line is the arithmetic, written out: the negative total is the refutations plus
    /// the admissions plus anything that agreed with neither. Every count is derived from the
    /// answers through [`Disposition`], every bucket prints including the empty ones (a lane
    /// dropping to zero is precisely what a line listing only non-empty buckets would hide),
    /// and the order is [`Disposition::ADMISSIONS`]' declaration order rather than any
    /// iteration order.
    ///
    /// The four groups partition the lane because the bucket DETERMINES the grade here:
    /// `refuted` and all six admissions agree, `entailed` on a negative case is the
    /// unsoundness (`unsound`), and a withheld case computed no closure at all.
    #[must_use]
    pub fn negative_lane_line(&self) -> String {
        let (_, negative_total) = self.by_direction();
        let refuted = self.negative_agreements(Disposition::Refuted);
        let admitted: usize = Disposition::ADMISSIONS
            .into_iter()
            .map(|bucket| self.negative_agreements(bucket))
            .sum();
        let mut out = format!(
            "OWL2-RL-NEGATIVE: total {negative_total} = refuted {refuted} + admitted \
             {admitted} ("
        );
        for (n, bucket) in Disposition::ADMISSIONS.into_iter().enumerate() {
            let separator = if n == 0 { "" } else { ", " };
            let _ = write!(
                out,
                "{separator}{} {}",
                bucket.label(),
                self.negative_agreements(bucket)
            );
        }
        let _ = write!(
            out,
            ") + unsound {} + withheld {}",
            self.negative_answers(Disposition::Entailed),
            self.negative_answers(Disposition::Withheld),
        );
        out
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
/// Returns a message if the corpus cannot be discovered, if a vendored support
/// document cannot be read, or if [`LEDGER`] names a case that is not vendored (an
/// entry for a case that no longer exists would silently inflate the budget).
pub fn run(root: &Path) -> Result<RlSummary, String> {
    let cases = discover(root)?;
    let imports = vendored_imports(root)?;
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
        // ONE run per case, read twice. The grade and the mechanism are two halves of the
        // same answer, and obtaining them from two calls would let a scoreboard row
        // describe a run its own grade did not come from.
        let certified = certify(case, &imports);
        let mechanism = certified
            .as_ref()
            .ok()
            .map(EntailmentCertificate::mechanism);
        let answer = match certified {
            Ok(certificate) => match certificate.into_parts().0 {
                EntailmentOutcome::Entailed(_) => Answer::Entailed,
                EntailmentOutcome::NotEntailed(reason) => Answer::NotEntailed(reason),
                EntailmentOutcome::Undecided(reason) => Answer::Undecided(reason),
            },
            Err(why) => Answer::Withheld(why),
        };
        let disposition =
            Disposition::classify(&answer).map_err(|why| format!("{}: {why}", case.name))?;
        summary.cases.push(GradedCase {
            name: case.name.clone(),
            direction: case.direction,
            grade: grade_answer(case, answer),
            disposition,
            ledgered: ledger_lookup(&case.name),
            mechanism,
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

    /// EVERY classification places itself on both axes, and the placements are pinned.
    ///
    /// This replaces two tests that iterated [`LEDGER`]. With the ledger empty they had
    /// become tautologies — "no entry is unsound" and "no entry is actionable" are both
    /// true of an empty table however the variants are classified — so they proved
    /// nothing about the thing they were named for. This ranges over [`RlGap::ALL`]
    /// instead, which an empty ledger does not empty, and it pins WHICH variants are
    /// unsound and WHICH are actionable. A variant added without deciding both is a
    /// compile error at the two exhaustive matches and a failure here.
    ///
    /// The corpus-level claims those two tests were reaching for are made where they can
    /// actually be made — over the 50 graded cases, in
    /// `tests/owl2_rl_conformance.rs`: `no_case_diverges_from_the_published_verdict` and
    /// `every_negative_case_is_answered_under_an_unexhausted_certificate`.
    #[test]
    fn every_gap_classifies_itself_on_both_axes() {
        let unsound: Vec<&str> = RlGap::ALL
            .into_iter()
            .filter(|gap| gap.is_unsound())
            .map(RlGap::label)
            .collect();
        assert_eq!(
            unsound,
            ["unsound-derivation"],
            "exactly ONE classification is an unsoundness — the chase deriving a triple W3C \
             says is NOT entailed, which no incompleteness excuses. A second one arriving is a \
             decision to state here, not a default"
        );

        let actionable: Vec<&str> = RlGap::ALL
            .into_iter()
            .filter(|gap| gap.is_actionable())
            .map(RlGap::label)
            .collect();
        assert_eq!(
            actionable,
            ["missing-rule", "unsound-derivation"],
            "the actionable classifications are the two PurRDF could close inside the rule \
             table; every other one describes what the OWL 2 RL profile itself does not reach, \
             and misfiling one there would turn a structural limit into a to-do item"
        );

        // Labels are what the tally and the log print, so two variants sharing one would
        // make a ledger tally unreadable.
        let mut labels: Vec<&str> = RlGap::ALL.into_iter().map(RlGap::label).collect();
        assert_eq!(labels.len(), 7);
        labels.sort_unstable();
        let count = labels.len();
        labels.dedup();
        assert_eq!(count, labels.len(), "two RlGap variants share a label");
    }

    /// The scoreboard's print order and its predicate name the SAME six buckets.
    ///
    /// [`Disposition::ADMISSIONS`] is what [`RlSummary::negative_lane_line`] prints and sums
    /// into `admitted`, and [`Disposition::is_admission`] is what says a bucket is one. They
    /// are two statements of one taxonomy, so a bucket in one and not the other would print
    /// an `admitted` total that its own sub-buckets do not add up to — which is precisely the
    /// unfalsifiable number this line exists to replace.
    #[test]
    fn every_disposition_classifies_itself() {
        let admissions: Vec<Disposition> = Disposition::ALL
            .into_iter()
            .filter(|bucket| bucket.is_admission())
            .collect();
        assert_eq!(
            admissions,
            Disposition::ADMISSIONS,
            "the printed admission buckets and the is_admission predicate disagree"
        );

        // A decided answer is never an admission, and neither is a run that made no
        // observation at all — pinned by name so a re-classification has to be deliberate.
        assert!(!Disposition::Refuted.is_admission());
        assert!(!Disposition::Entailed.is_admission());
        assert!(!Disposition::Withheld.is_admission());

        // Labels are what the scoreboard prints, so two buckets sharing one would make the
        // negative lane's composition unreadable.
        let mut labels: Vec<&str> = Disposition::ALL
            .into_iter()
            .map(Disposition::label)
            .collect();
        assert_eq!(labels.len(), 9);
        labels.sort_unstable();
        let count = labels.len();
        labels.dedup();
        assert_eq!(count, labels.len(), "two Disposition buckets share a label");
    }

    #[test]
    fn direction_and_target_file_agree() {
        assert_eq!(Direction::Positive.target_file(), "conclusion.rdf");
        assert_eq!(Direction::Negative.target_file(), "non-conclusion.rdf");
        assert!(Direction::Positive.expects_match());
        assert!(!Direction::Negative.expects_match());
    }
}
