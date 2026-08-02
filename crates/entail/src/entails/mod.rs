// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **Conclusion-directed** entailment: does this premise entail this conclusion?
//!
//! [`materialize`] answers a different question. It computes a CLOSURE
//! — everything the premise entails, as a dataset — and hands it over. That is the right
//! shape for a caller that will go on asking many questions of one premise, and the wrong
//! shape for a caller with one question, because turning a closure into a verdict is not
//! the obvious membership test it looks like: a conclusion's blank nodes are existentials
//! that have to be MAPPED, an inconsistent premise entails everything, and a failure to
//! find a mapping means nothing at all unless the rule set is complete for the premise it
//! ran on. Every one of those steps has been got wrong somewhere, so this module does them
//! once, in the library, with the evidence attached.
//!
//! # The three answers, and why there are three
//!
//! [`EntailmentOutcome`] has no boolean anywhere in it:
//!
//! * [`Entailed`](EntailmentOutcome::Entailed) carries an [`EntailmentWarrant`] — the
//!   mapping that made it true — which [`verify`] re-decides without running a reasoner.
//! * [`NotEntailed`](EntailmentOutcome::NotEntailed) carries a [`MissReason`] and is a
//!   PROOF: it is returned only when the procedure is complete for this premise, so the
//!   absence of a mapping is the absence of an entailment.
//! * [`Undecided`](EntailmentOutcome::Undecided) carries an [`UndecidedReason`] and is
//!   returned when no mapping was found and the procedure is NOT complete here. Collapsing
//!   it into `NotEntailed` would turn a limitation of this library into a false statement
//!   about the caller's ontology, and it is the single most consequential distinction in
//!   this module.
//!
//! # Consistency is established FIRST, and it hard-fails
//!
//! An inconsistent knowledge base entails every triple. So a service that tested membership
//! in the closure of an inconsistent premise would answer `Entailed` for literally
//! everything, correctly and uselessly, and a caller reading `Entailed` would have no way
//! to tell that answer apart from a real one.
//!
//! The check is not a separate pass that could be skipped or reordered: it is the chase
//! itself, and it happens before any conclusion of that chase is readable. Seventeen of OWL
//! 2 RL's seventy-eight rules conclude `false` — `eq-diff1..3`, `prp-irp`, `prp-asyp`,
//! `prp-pdw`, `prp-adp`, `prp-npa1`, `prp-npa2`, `cls-nothing2`, `cls-com`, `cls-maxc1`,
//! `cls-maxqc1`, `cls-maxqc2`, `cax-dw`, `cax-adc` and `dt-not-type`, the last of which is
//! also the `D` lane's — and a body match on any of them makes [`materialize`] return
//! [`EntailError::Inconsistent`] instead of a closure. So there is no closure for this
//! module to match against, and the refusal propagates to the caller carrying the
//! [`InconsistentRun`] witness that says which rule fired on which
//! asserted triples. `Simple`, `RDF` and `RDFS` state no rule whose head is `false`, so for
//! those three the check is VACUOUS rather than skipped: there is no rule that could have
//! detected an inconsistency, and this module does not pretend one ran.
//!
//! # Which regimes, and why not all seven
//!
//! The parameter is a [`Regime`] and not a [`Materialization`],
//! because a `Materialization` has seven inhabitants and two of them are defined by an
//! input this signature does not carry: `OWL-Direct` is query-directed and `RIF` entails
//! under a rule set the caller wrote. Accepting them here and quietly doing something else
//! would be worse than refusing, so they are refused —
//! [`EntailError::UnsupportedRegime`], a caller-visible error naming the regime, never a
//! fallback to a weaker one. The five rule-table regimes (`Simple`, `RDF`, `RDFS`,
//! `OWL-RL`, `D`) are served, each with its own completeness condition; see
//! [`precondition`] for which theorem each condition is the hypothesis of.
//!
//! # `owl:imports` is resolved or refused
//!
//! OWL 2 defines an ontology's imports closure to BE the ontology, so a premise that imports
//! a document this call was not handed is a DIFFERENT premise from the one the caller asked
//! about. Every answer over it would be about that other premise, so an unresolvable import
//! is [`EntailError::UnresolvedImport`] naming the document — never a silently truncated
//! premise. See [`imports`].
//!
//! # The answer arrives WITH the run that produced it
//!
//! [`entails`] returns an [`EntailmentCertificate`]: the outcome above, and the
//! [`ReasoningReport`] of the chase underneath it. A verdict alone carries the MECHANISM's
//! evidence and none of the chase's, so a caller reading `NotEntailed` could not ask whether
//! the rule table it came out of was complete, which rules fired, or which calculus ran —
//! and had to reconstruct all three from prose. The certificate also names the mechanism, on
//! its report, so a rendered answer says which of the six below — or which combination of
//! them — reached it. See
//! [`certificate`].
//!
//! # Six mechanisms, and all of them are named
//!
//! * [`homomorphism`] — the chase-and-graph-match procedure OWL 2 Profiles §4.3 states the
//!   RL entailment relation in terms of. It is complete for every conclusion the rule table
//!   can produce.
//! * [`refutation`] — assert the conclusion's negation into the premise, re-chase, and read
//!   the profile's own seventeen `false`-concluding rules as the proof. It exists because
//!   the rule table produces no NEGATIVE FACT at all: no head in Tables 4–9 is an
//!   `owl:differentFrom` or a membership in an `owl:complementOf` class, so a premise can
//!   entail one while a forward chase derives nothing to match against.
//! * [`freeze`] — instantiate a schema axiom's universally quantified body over constants the
//!   premise does not mention, re-chase, and read the derived head as the proof. It exists
//!   because the rule table claims no completeness for a SCHEMA AXIOM: Theorem PR1's
//!   conclusion hypothesis admits only assertional conclusions, so `p owl:propertyChainAxiom
//!   (p p)` can entail `p rdf:type owl:TransitiveProperty` — a characteristic no head in
//!   Tables 4–9 has the shape of — while the chase derives nothing to match against. The
//!   lane covers inclusions on the same warrant, and there the table is not silent at all:
//!   `scm-sco`, `scm-eqc1`/`scm-eqc2`, `scm-spo` and `scm-eqp1`/`scm-eqp2` conclude
//!   `rdfs:subClassOf`, `owl:equivalentClass`, `rdfs:subPropertyOf` and
//!   `owl:equivalentProperty` and all of them fire. Firing is not completeness.
//! * [`comprehension`] — mint the anonymous class expressions the conclusion names, under the
//!   typing side conditions the RDF-Based comprehension conditions impose. It exists because
//!   a comprehension condition asserts the existence of a resource NOTHING NAMES, and a rule
//!   set that produced one for every licensed shape would produce infinitely many.
//! * [`reflexivity`] — read the conclusion's own self-loops `x p x` off the premise's
//!   `owl:ReflexiveProperty` typings. It exists because `owl:ReflexiveProperty` is outside
//!   the OWL 2 RL syntax so the profile states no rule for it at all, and because a rule that
//!   DID state it would range over every resource — an `O(|terms|)` closure widening in a lane
//!   every consumer runs by default, to answer a question only a conclusion ever asks.
//! * [`datarange`] — intersect the premise's declared `rdfs:range` datatypes and ask whether
//!   the intersection is CONTAINED in the conclusion's. It exists because deciding that needs
//!   the XSD value spaces and a rule table has no arithmetic — `xsd:byte ⊑ xsd:short` is not
//!   something any join over triples can discover.
//!
//! None of the five extra mechanisms adds a rule — `rules`, `implemented` and `extensions`
//! are untouched by all of them — and each runs only after the premise's consistency has been
//! established, which is the hypothesis every one of the soundness arguments rests on. See
//! their module docs for those arguments, written out.
//!
//! [`EntailmentWarrant`] has one arm per mechanism, each minted by the mechanism it names,
//! and a SEVENTH — [`Composite`](EntailmentWarrant::Composite) — that arrived with its own
//! producer: the fold below, which is the only thing that constructs one. This crate does not
//! pre-declare states that nothing constructs.
//!
//! The five extra lanes are run for a question with NOTHING TO PROJECT, and not otherwise.
//! That is a statement about the question rather than about the entry point: a basic graph
//! pattern with no `?v` in it IS a conclusion graph, so [`certain_answers`] routes it through
//! the same shared spine and reaches the same answer [`entails`] does.
//!
//! A PROJECTED variable is where they stop, and deliberately. Refutation decides a ground
//! negative fact, and a projected variable ranging over one is a different question — "which
//! individuals is `a` entailed to differ from?" would need a refutation per candidate over
//! the whole domain, which is not what [`certain_answers`] computes and not what it would be
//! honest to let it claim. Freeze-and-chase decides a ground SCHEMA AXIOM, and a projected
//! variable there would be "which properties is the premise entailed to make transitive?",
//! which needs one frozen chase per candidate property for the same reason. Comprehension
//! decides the existence of a class the conclusion DESCRIBES, and there is no answer to
//! project: the minted witness is not a term of the scoping graph, so no SPARQL entailment
//! regime admits it as a binding. Reflexivity decides a self-loop over a term the CONCLUSION
//! names, and a projected variable there would range over every resource — which is the
//! closure widening this crate declines to perform in the materialization lane, arriving by
//! another door. Datatype containment decides a ground `rdfs:range` AXIOM, and a projected
//! variable there would range over the datatype map rather than over the premise's terms.
//!
//! Declining to answer is not the same as answering "there is none", so a lane that would have
//! been needed and was not run reaches the caller as a LIMIT. Each lane therefore has a second
//! entry point — a RECOGNITION, which runs its own whitelist over the question and decides
//! nothing — and each non-empty recognition becomes an
//! [`UndecidedReason::ConstructNotRead`], so [`CertainAnswers::is_complete`] is then `false`.
//! Without it a question needing one of the five came back as an empty row set with an empty
//! limit list — which renders as "no certain answers, exhaustively", about a question nothing
//! had tested.
//!
//! # The mechanisms COMPOSE, because entailment is monotone over conjunction
//!
//! Each of the extra mechanisms splits the conclusion into the triples it establishes and the
//! RESIDUAL triples. The residual is not handed to the closure by the lane that produced it:
//! it is THREADED through the remaining lanes in a fixed cost order, and only whatever
//! survives all five is matched — against `closure ∪ Σ(minted)`, because comprehension and
//! reflexivity add licensed triples a later obligation may legitimately reference.
//!
//! It has to work that way because a conclusion GRAPH is a conjunction and entailment is
//! monotone over one: if `P ⊨ C₁` and `P ⊨ C₂` then `P ⊨ C₁ ∪ C₂`. A conclusion stating a
//! negative fact BESIDE a schema axiom is entailed when each half is, and an implementation
//! in which each lane scored the other's half as an unmatched residual would answer `no` to a
//! question whose answer is `yes` — and would answer it as a PROOF, because the fall-through
//! reads "no mechanism reached it" as "nothing entails it".
//!
//! An answer that needed two or more lanes is an
//! [`EntailmentWarrant::Composite`] carrying the contributing
//! warrants in that same fixed cost order, and it renders as
//! [`EntailmentMechanism::Composite`] — never as any single lane's name, which would be a
//! false attribution. One lane and the final match is still that lane's own warrant, so a
//! `refutation` answer is still spelled `refutation`.
//!
//! # Determinism
//!
//! Everything below is a function of the inputs alone: the closure's frozen quad order, a
//! stable most-constrained-first pattern sort, `BTreeMap`/`BTreeSet` iteration, and a STEP
//! budget rather than a clock. Two runs over one premise and one question return the same
//! verdict, the same binding, and the same diagnosis, on `wasm32` as on native.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};

use crate::interner::intern_into;
use crate::owl_dl::query::QTriple;
use crate::report::{InconsistentRun, ReasoningReport};
use crate::{EntailError, Materialization, Regime, materialize};

pub mod answers;
pub mod certificate;
pub mod comprehension;
pub mod datarange;
pub mod freeze;
pub mod homomorphism;
pub mod imports;
pub mod negation;
pub mod precondition;
pub mod reflexivity;
pub mod refutation;
pub mod warrant;

// Four support modules with no public items of their own: the owned triple view both sides
// of a match are read through, the pattern the question is compiled to, the generator of
// names no input uses, and the typing side condition every schema conclusion carries.
// `VarKey` is the one thing a caller sees out of any of them, and it is re-exported below.
mod fresh;
mod graph;
mod membership;
mod pattern;

pub use answers::CertainAnswers;
pub use certificate::EntailmentCertificate;
pub use comprehension::ComprehensionWarrant;
pub use datarange::{DataRangeWarrant, RangeContainment};
pub use freeze::{FREEZE_BUDGET, FreezeWarrant, FrozenInstance, FrozenOutcome, Generalization};
pub use homomorphism::{Binding, MATCH_BUDGET, MissReason};
pub use imports::ImportMap;
pub use negation::NegativeFact;
pub use pattern::VarKey;
pub use precondition::UndecidedReason;
pub use reflexivity::ReflexivityWarrant;
pub use refutation::{REFUTATION_BUDGET, Refutation, RefutationWarrant};
pub use warrant::{
    CompositeWarrant, EntailmentMechanism, EntailmentWarrant, HomomorphismWarrant, verify,
};

use fresh::FreshBlanks;
use graph::{Triple, default_graph_triples, show};
use homomorphism::{Closure, show_pattern};
use pattern::{Pat, PatTriple, bgp_patterns, conclusion_patterns, patterns_at, projected_vars};

/// What a conclusion-directed question answered.
///
/// Three answers, never two. See the [module docs](self) for why `Undecided` cannot be
/// folded into `NotEntailed`.
#[derive(Debug, Clone)]
pub enum EntailmentOutcome {
    /// The premise entails the conclusion, and here is the evidence.
    Entailed(EntailmentWarrant),
    /// The premise does NOT entail the conclusion. A proof: the procedure was complete for
    /// this premise, so the absence of a mapping is the absence of an entailment.
    NotEntailed(MissReason),
    /// No mapping was found AND the procedure is not complete for this premise, so nothing
    /// is proven in either direction.
    Undecided(UndecidedReason),
}

/// What ONE mechanism made of a conclusion.
///
/// Every mechanism beyond [`homomorphism`] answers in these five states and no others, which
/// is what lets [`MECHANISMS`] be a list rather than a nest of special cases. Three of them
/// look alike and carry three different epistemic weights:
///
/// * `NotApplicable` — "this conclusion says nothing I read". The caller answers exactly what
///   it would have answered without this lane.
/// * `NotEstablished` — "it is my question and I did not reach it". Handed back unchanged; a
///   mechanism never refutes, because refuting needs a completeness claim and [`precondition`]
///   is where those live.
/// * `Disqualified` — "I RECOGNIZE a construct here and I decline to read it". That is an
///   admission of incapacity, and an admission must never become a refutation, so it routes to
///   [`EntailmentOutcome::Undecided`] naming what was declined. Collapsing it into
///   `NotApplicable`, which is what this enum used to do, is how a whitelist refusal came out
///   of the service as a proof.
pub(crate) enum Attempt {
    /// The lane does not apply: the regime is not one it serves, or the conclusion states
    /// nothing it reads.
    NotApplicable,
    /// The lane RECOGNIZES a construct of this conclusion and declines to read it, so nothing
    /// tested it in either direction.
    Disqualified(UndecidedReason),
    /// Part or all of the conclusion is established, and here is the evidence together with
    /// exactly which triples it discharged and what it minted.
    ///
    /// Boxed because a warrant carries whole closures and this enum is returned by value from
    /// every mechanism, including the ones that almost always decline.
    Entailed(Box<Established>),
    /// The lane applies and did NOT establish what it recognized.
    NotEstablished,
    /// The lane applies and stopped early, so it proved nothing in either direction.
    Undecided(UndecidedReason),
}

/// What one mechanism CONTRIBUTED to a conclusion.
///
/// The four parts are what makes the fold in [`entails`] possible. A lane that returned only
/// a warrant would leave the caller unable to say which of the conclusion's obligations are
/// still outstanding, which is exactly the information a second lane needs.
pub(crate) struct Established {
    /// The evidence for what this lane established. Its binding is EMPTY: the residual
    /// homomorphism belongs to the answer, not to a lane, and [`entails`] fills it in on the
    /// single-lane path or carries it on the composite.
    pub(crate) warrant: EntailmentWarrant,
    /// The conclusion triples this lane DISCHARGED, by index into the conclusion's own frozen
    /// triple order. Empty for a lane that only widens the closure.
    pub(crate) discharged: BTreeSet<usize>,
    /// The triples this lane licensed INTO the closure — comprehension's minted scaffolds,
    /// reflexivity's self-loops. Every one of them is entailed by the premise, so the final
    /// match may legitimately land in them.
    pub(crate) minted: Vec<Triple>,
    /// Constructs this lane RECOGNIZED and declined to read while establishing something else,
    /// rendered — the same strings its [`Attempt::Disqualified`] would have carried.
    ///
    /// One pass can do both: `p` is declared reflexive and the conclusion states a self-loop
    /// at a name AND one at an existential, so the lane mints the first and declines the
    /// second. A lane that could only report a refusal INSTEAD of a mint would drop the
    /// second, and a dropped admission falls through to the final match and is reported as a
    /// refutation — the exact substitution of a proof for an admission [`Attempt`]'s doc bans.
    /// So it travels beside the evidence, and [`fold`] withholds on it.
    pub(crate) declined: Vec<String>,
}

/// One conclusion-directed question, as a mechanism reads it.
///
/// Carried as a struct rather than as five parameters because [`pending`](Self::pending) is
/// the one a reader must not forget: a lane that answered about a triple another lane already
/// discharged would double-count it, and one that ignored the field would re-decide a
/// question already decided.
pub(crate) struct Question<'a> {
    /// The premise, with its imports resolved.
    pub(crate) premise: &'a RdfDataset,
    /// The conclusion, whole — every lane's whitelist is a claim about ALL of it.
    pub(crate) conclusion: &'a RdfDataset,
    /// The regime, which every lane gates itself on by whitelist.
    pub(crate) regime: Regime,
    /// The premise's own closure, indexed.
    pub(crate) closure: &'a Closure,
    /// The conclusion's default-graph triples, in its own frozen order — the index space every
    /// lane's `discharged` set is expressed in.
    pub(crate) triples: &'a [Triple],
    /// The indices no earlier lane has discharged yet.
    pub(crate) pending: &'a BTreeSet<usize>,
}

/// What one mechanism READS of a question, with nothing decided either way.
///
/// The recognition half of a lane, split out from the decision half because the two questions
/// have different answers and different costs. `attempt` asks "is this established?" and pays
/// for a re-chase to find out; this asks "is this MINE?" and is the lane's own whitelist run
/// over the question's syntax and the closure's index — no chase, no search.
///
/// [`certain_answers`] is the caller that needs the split. It runs the rule table and nothing
/// else, so a question one of the five lanes reads is one whose answer set it cannot claim to
/// have enumerated — and it has to say so WITHOUT running the lane, because a projected
/// variable over what a lane decides is a different question than the lane answers. An empty
/// recognition is therefore the whole claim that not running the lane cost nothing.
#[derive(Default)]
pub(crate) struct Recognized {
    /// The question's triples this lane reads, by index into [`Question::triples`].
    pub(crate) read: BTreeSet<usize>,
    /// Constructs this lane NAMES and declines to read at all, rendered — the same strings
    /// its [`Attempt::Disqualified`] would have carried.
    pub(crate) declined: Vec<String>,
}

impl Recognized {
    /// Whether this lane reads nothing here, which is the claim that a service declining to
    /// run it has lost nothing by declining.
    fn is_empty(&self) -> bool {
        self.read.is_empty() && self.declined.is_empty()
    }
}

/// One mechanism, with both of its entry points and the name it answers under.
///
/// A struct rather than three parallel arrays, because a lane that arrived with a decision
/// procedure and no recognizer would be invisible to [`certain_answers`]'s completeness claim
/// — which is exactly the defect the recognition half was added to close, re-introduced by a
/// table that let the two halves be listed separately.
struct Lane {
    /// Which of the seven mechanisms this is — the name a limit or a warrant carries.
    mechanism: EntailmentMechanism,
    /// DECIDE: establish part or all of the conclusion, or say why not. The fold's entry
    /// point.
    attempt: fn(&Question<'_>) -> Result<Attempt, EntailError>,
    /// RECOGNIZE: say what of the question this lane reads, deciding none of it.
    recognizes: fn(&Question<'_>) -> Recognized,
}

/// The mechanisms beyond [`homomorphism`], in the order [`entails`] folds them.
///
/// Order is a COST ordering: a conclusion no mechanism reads pays only the cheapest
/// applicability tests before falling through. It is also, and deliberately, the ONLY thing
/// that decides the order of a composite warrant's constituents — a fixed array rather than
/// the order a search happened to reach, so the same question over the same premise always
/// produces the same warrant, on `wasm32` as on native.
///
/// Two lanes can read one predicate — `rdfs:range` is a freeze shape when its object is a
/// class and a data-range shape when its object is a datatype — which is why every lane
/// filters what it recognizes through [`Question::pending`]: the earlier lane's discharge
/// removes the triple from the later lane's question rather than leaving it to be decided
/// twice, possibly two ways.
const MECHANISMS: [Lane; 5] = [
    Lane {
        mechanism: EntailmentMechanism::Refutation,
        attempt: refutation::attempt,
        recognizes: refutation::recognizes,
    },
    Lane {
        mechanism: EntailmentMechanism::Freeze,
        attempt: freeze::attempt,
        recognizes: freeze::recognizes,
    },
    Lane {
        mechanism: EntailmentMechanism::Comprehension,
        attempt: comprehension::attempt,
        recognizes: comprehension::recognizes,
    },
    Lane {
        mechanism: EntailmentMechanism::Reflexivity,
        attempt: reflexivity::attempt,
        recognizes: reflexivity::recognizes,
    },
    Lane {
        mechanism: EntailmentMechanism::DataRange,
        attempt: datarange::attempt,
        recognizes: datarange::recognizes,
    },
];

/// The plan for a regime, or a refusal.
///
/// Written as a total match with no wildcard so an eighth regime cannot be added without
/// deciding, here, whether this service can serve it.
const fn plan_for(regime: Regime) -> Result<Materialization<'static>, EntailError> {
    match regime {
        Regime::Simple => Ok(Materialization::Simple),
        Regime::Rdf => Ok(Materialization::Rdf),
        Regime::Rdfs => Ok(Materialization::Rdfs),
        Regime::OwlRl => Ok(Materialization::OwlRl),
        Regime::D => Ok(Materialization::D),
        // Defined by an input this signature does not carry: the query's class expressions,
        // and the caller's rule document. Refused rather than approximated by a weaker lane.
        Regime::OwlDirect | Regime::Rif => Err(EntailError::UnsupportedRegime(regime)),
    }
}

/// One prepared run: the premise's imports resolved, its consistency established, and its
/// closure indexed.
struct Prepared {
    /// The imports closure, when the premise imported anything. `None` is the common case
    /// and costs nothing: a premise that imports nothing is its own effective premise, and
    /// copying it to say so would be a full dataset copy per call.
    merged: Option<Arc<RdfDataset>>,
    /// The indexed closure a question is matched against.
    closure: Closure,
    /// What the run did, which two of the completeness conditions are read from.
    report: ReasoningReport,
}

impl Prepared {
    /// The premise the run was actually over.
    fn effective<'a>(&'a self, premise: &'a RdfDataset) -> &'a RdfDataset {
        self.merged.as_deref().unwrap_or(premise)
    }
}

/// Resolve imports, run the chase, and index the closure.
///
/// Everything that can refuse happens here, in the order the refusals have to happen in: an
/// unresolvable import before the chase (because it changes what the premise IS), and the
/// chase's own inconsistency refusal before any conclusion of it is readable.
///
/// # The report is corrected for the merge THIS function made
///
/// [`materialize`] surveys the dataset it is handed, and the dataset handed to it here is the
/// MERGED premise — which still carries the `owl:imports` triples the merge resolved. So the
/// chase raises [`Construct::UnresolvedOntologyImport`], which is the honest reading from
/// where it stands and the wrong one from here: `imports::resolve` above refuses the whole
/// call with [`EntailError::UnresolvedImport`] on any document its map does not resolve, so
/// reaching this line at all proves every declared import was resolved and merged.
///
/// This is therefore the one place both facts are in scope, and it is where the boundary is
/// restated as [`Construct::ResolvedOntologyImport`] — on the closure's report and on the
/// report an inconsistent run refuses with, because a caller reading a refusal's certificate
/// is owed the same true statement as a caller reading a verdict's.
fn prepare(
    premise: &RdfDataset,
    regime: Regime,
    imports: &ImportMap,
) -> Result<Prepared, EntailError> {
    let plan = plan_for(regime)?;
    let merged = imports::resolve(premise, imports)?;
    // `resolve` answers `None` for a premise that names no document, and that premise's run
    // has no import boundary to restate — so the correction is applied exactly when a merge
    // actually happened.
    let Some(merged) = merged else {
        let (closure, report) = materialize(premise, plan)?;
        return Ok(Prepared {
            merged: None,
            closure: Closure::of(default_graph_triples(&closure)),
            report,
        });
    };
    let (closure, report) = materialize(&merged, plan).map_err(resolved_imports_error)?;
    Ok(Prepared {
        merged: Some(merged),
        closure: Closure::of(default_graph_triples(&closure)),
        report: report.with_resolved_imports(),
    })
}

/// `error`, with any REPORT it carries restated for a run whose imports were resolved.
///
/// Only [`EntailError::Inconsistent`] carries a [`ReasoningReport`]; every other variant is
/// the absence of a run and has no boundary list to correct. Written as a total match so a
/// later error that starts carrying a report has to decide here rather than silently ship the
/// pre-merge boundary.
fn resolved_imports_error(error: EntailError) -> EntailError {
    match error {
        EntailError::Inconsistent(run) => {
            let (witness, report) = run.into_parts();
            EntailError::Inconsistent(Box::new(InconsistentRun::new(
                witness,
                report.with_resolved_imports(),
            )))
        }
        other @ (EntailError::Build(_)
        | EntailError::Parse(_)
        | EntailError::Evaluate(_)
        | EntailError::Chase(_)
        | EntailError::MalformedList(_)
        | EntailError::UnsupportedRegime(_)
        | EntailError::UnresolvedImport(_)
        | EntailError::MatchBudget
        | EntailError::Unsatisfiable) => other,
    }
}

/// The certain answers of `bgp` over `premise` under `regime`.
///
/// A row is a substitution the knowledge base ENTAILS the pattern under — true in every
/// model, not merely present in one closure — over the premise's own terms, as SPARQL's
/// entailment regimes require. Every row is sound unconditionally; whether the row set is
/// exhaustive is [`CertainAnswers::is_complete`], derived from the same completeness
/// conditions [`entails`] uses.
///
/// A `?v` of the pattern is projected and appears in [`CertainAnswers::vars`] — including a
/// `?v` inside an RDF 1.2 triple term, which is the SAME variable as one of that name
/// outside it, so the join is enforced rather than split into two unrelated variables; a
/// blank node of the pattern is a non-distinguished variable, constrained by the match and
/// not projected, which is what SPARQL says a query blank node is.
///
/// # NOTHING TO PROJECT IS [`entails`]'S QUESTION, AND IS ANSWERED BY [`entails`]'S FOLD
///
/// A pattern with no `?v` in it is a conclusion GRAPH — every position is a term or a blank
/// node, which is exactly what an RDF graph is — so it is routed through the same shared spine
/// [`entails`] runs, and reaches whichever of the seven mechanisms answers it. The two entry
/// points cannot disagree about such a question because there is one implementation of it;
/// what differs is the PRESENTATION, and only that. A `yes` is the one row over zero columns,
/// a `no` is the empty relation, and an `undecided` is the empty relation WITH the reason as
/// a limit.
///
/// # A PROJECTED VARIABLE OVER A LANE'S QUESTION IS A LIMIT, NEVER A SILENCE
///
/// With something to project the five lanes beyond the rule table are not run, and the
/// [module docs](self) argue at length why: "which individuals is `a` entailed to differ
/// from?" needs a refutation per candidate over the whole domain, and the same holds for
/// freeze, comprehension, reflexivity and datatype containment. That argument licenses not
/// RUNNING them. It does not license reporting an empty row set as EXHAUSTIVE when one of
/// them would have been needed — which is a claim about the caller's data made out of this
/// library's own incapacity.
///
/// So every lane is asked what it RECOGNIZES, which is its own whitelist over the
/// question's syntax and costs no chase, and each lane that reads anything here
/// contributes an [`UndecidedReason::ConstructNotRead`] naming itself and the constructs.
/// [`CertainAnswers::is_complete`] is then `false`, and the row set says "none was found"
/// rather than "there is none".
///
/// The run that produced the rows travels with them, as [`CertainAnswers::report`]. Rows
/// without it are half an answer for the same reason a verdict without it is: a caller
/// reading an empty row set beside [`CertainAnswers::is_complete`] needs to know which rule
/// table the closure came out of, and there is no second call to get it from.
///
/// # Errors
///
/// [`EntailError::UnsupportedRegime`] for a regime defined by an input this signature does
/// not carry; [`EntailError::UnresolvedImport`] for an `owl:imports` the map does not
/// resolve; [`EntailError::Inconsistent`] for a premise with no model, carrying the witness
/// and the run's report; [`EntailError::MatchBudget`] if the match exhausts
/// [`MATCH_BUDGET`]; and whatever [`materialize`] refuses with.
///
/// ```
/// use purrdf_core::{RdfDatasetBuilder, TermValue};
/// use purrdf_entail::{ImportMap, QNode, QTriple, Regime, certain_answers};
///
/// let mut b = RdfDatasetBuilder::new();
/// let cat = b.intern_iri("http://example.org/Cat");
/// let animal = b.intern_iri("http://example.org/Animal");
/// let tom = b.intern_iri("http://example.org/tom");
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// b.push_quad(cat, sub, animal, None);
/// b.push_quad(tom, ty, cat, None);
/// let premise = b.freeze().expect("freeze");
///
/// // `?c` ranges over the ENTAILED types of `tom`, not the asserted one.
/// let bgp = [QTriple {
///     s: QNode::Term(TermValue::iri("http://example.org/tom")),
///     p: QNode::Term(TermValue::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")),
///     o: QNode::Var("c".to_owned()),
/// }];
/// let answers = certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new())
///     .expect("a consistent premise");
/// assert_eq!(answers.vars(), ["c"]);
/// assert!(answers.rows().iter().any(|row| row == &[TermValue::iri("http://example.org/Animal")]));
/// ```
pub fn certain_answers(
    premise: &RdfDataset,
    bgp: &[QTriple],
    regime: Regime,
    imports: &ImportMap,
) -> Result<CertainAnswers, EntailError> {
    let prepared = prepare(premise, regime, imports)?;
    let pats = bgp_patterns(bgp);
    let names = projected_vars(&pats);
    // NOTHING TO PROJECT: this is `entails`'s question, so it is `entails`'s fold that answers
    // it. Routed rather than re-implemented — one implementation, two presentations.
    //
    // The WHOLE pattern reaches it, before the survey filter below exists. A verdict is a
    // statement about the conjunction the caller asked about, and withholding a conjunct from
    // it would make an unentailed question answer `entailed` — so the filter is built after
    // this returns and there is no ordering in which it could reach a verdict.
    if names.is_empty()
        && let Some(question) = as_graph(&pats, &names)
    {
        let Prepared {
            merged,
            closure,
            report,
        } = prepared;
        let outcome = decide(
            merged.as_deref().unwrap_or(premise),
            &question.graph,
            regime,
            closure,
            &report,
        )?;
        return Ok(verdict_answers(regime, &outcome, report));
    }

    // THE LANE SURVEY READS A GRAPH, AND A TRIPLE WITH AN OPEN PREDICATE IS NOT ONE.
    //
    // Every lane's whitelist is stated over an RDF graph, and RDF says a predicate is an IRI —
    // so an open predicate has no term to substitute that a graph can hold, `as_graph` cannot
    // freeze one, and it would answer `None` for the whole question. That would cost the OTHER
    // triples their survey: a pattern pairing `?s ?p ?o` with `?x owl:differentFrom ex:Peter`
    // would lose the refutation lane's limit as well, which is the silence this crate spends
    // its whole design preventing. So the open-predicate triples are withheld from the survey
    // graph and nothing else is, and what their openness costs is disclosed BY NAME as
    // `UndecidedReason::OpenPredicate` from the precondition below.
    //
    // BOTH kinds of variable are withheld, and that is the same list `open_predicates` reports:
    // a non-distinguished predicate is a blank node, which is no more an RDF predicate than a
    // projected variable's substitution is, and a survey that read one would be reading
    // something the rule table's whitelists are not stated over. The two predicates agree
    // because the position — not the projection — is what decides both.
    let surveyed: Vec<PatTriple> = pats
        .iter()
        .filter(|triple| !matches!(triple[1], Pat::Var(_)))
        .cloned()
        .collect();
    let question = as_graph(&surveyed, &names);

    // The table's own completeness conditions. `decided_by_refutation` is EMPTY here and that
    // is a claim rather than a default: the refutation lane is not run on this path, so no
    // triple of the question is decided by the profile's inconsistency calculus — and every
    // lane that would have read one contributes its own limit below instead.
    let mut limits = precondition::limits(
        regime,
        prepared.effective(premise),
        &prepared.report,
        &pats,
        &BTreeSet::new(),
    );
    if let Some(question) = &question {
        limits.extend(unreachable_lanes(
            prepared.effective(premise),
            question,
            regime,
            &prepared.closure,
        ));
    }
    let vars: Vec<VarKey> = names.iter().cloned().map(VarKey::Projected).collect();
    let rows: BTreeSet<Vec<TermValue>> = homomorphism::find_all(pats, &prepared.closure, &vars)?;
    Ok(CertainAnswers::new(
        regime,
        names,
        rows.into_iter().collect(),
        limits,
        prepared.report,
        // The rule table, and only the rule table: this path enumerates by matching the
        // closure and nothing else ran. The zero-projected-variable path above names whichever
        // of the seven actually answered.
        EntailmentMechanism::StrictTable,
    ))
}

/// One verdict, presented as the relation SPARQL says an answer with nothing to project is.
///
/// The join identity, in both directions: a `yes` is the ONE row over zero columns — the empty
/// substitution — and a `no` is the empty relation. So [`CertainAnswers::is_empty`] reads as
/// the verdict and no caller has to learn a second convention.
///
/// The limit list is the third answer and ONLY the third answer, which is what keeps
/// [`CertainAnswers::is_complete`] saying something true here. `Entailed` and `NotEntailed`
/// are both DECIDED, and over zero columns the empty substitution is the only substitution
/// there is — so in either case the row set holds every certain answer and IS exhaustive.
/// `Undecided` carries its reason as the one limit, the same value [`entails`] renders.
fn verdict_answers(
    regime: Regime,
    outcome: &EntailmentOutcome,
    report: ReasoningReport,
) -> CertainAnswers {
    let (rows, limits) = match outcome {
        EntailmentOutcome::Entailed(_) => (vec![Vec::new()], Vec::new()),
        EntailmentOutcome::NotEntailed(_) => (Vec::new(), Vec::new()),
        EntailmentOutcome::Undecided(reason) => (Vec::new(), vec![reason.clone()]),
    };
    CertainAnswers::new(
        regime,
        Vec::new(),
        rows,
        limits,
        report,
        certificate::mechanism_of(outcome),
    )
}

/// The lanes [`certain_answers`] does not run that this question would have needed.
///
/// One [`UndecidedReason::ConstructNotRead`] per recognizing lane, in the fixed [`MECHANISMS`]
/// cost order so the list is a function of the question rather than of a search. A lane that
/// recognizes nothing contributes nothing, which is what keeps an ordinary assertional pattern
/// [`complete`](CertainAnswers::is_complete) — making every answer incomplete would satisfy
/// the honesty requirement and destroy the service.
///
/// No lane is RUN: [`Recognized`] is each lane's own whitelist over the question's syntax and
/// the closure's index, so this costs no chase, no refutation and no frozen instance. That is
/// the whole point — the [module docs](self) argue that a projected variable over what a lane
/// decides is a different question, and this reports that fact instead of searching for its
/// answer.
fn unreachable_lanes(
    premise: &RdfDataset,
    question: &AsGraph,
    regime: Regime,
    closure: &Closure,
) -> Vec<UndecidedReason> {
    // Every triple is outstanding: nothing on this path discharged anything, so every lane is
    // asked about the whole question.
    let pending: BTreeSet<usize> = (0..question.triples.len()).collect();
    let asked = Question {
        premise,
        conclusion: &question.graph,
        regime,
        closure,
        triples: &question.triples,
        pending: &pending,
    };
    let mut limits = Vec::new();
    for lane in MECHANISMS {
        let recognized = (lane.recognizes)(&asked);
        if recognized.is_empty() {
            continue;
        }
        let mut constructs: Vec<String> = recognized
            .read
            .iter()
            .filter_map(|&index| question.rendered.get(index))
            .map(|shown| {
                format!(
                    "{shown}: a projected variable over what this lane decides ranges over the \
                     whole domain rather than over the premise's terms, and this service \
                     enumerates over the rule table alone"
                )
            })
            .collect();
        constructs.extend(recognized.declined);
        constructs.sort_unstable();
        constructs.dedup();
        limits.push(UndecidedReason::ConstructNotRead {
            lane: lane.mechanism,
            constructs,
        });
    }
    limits
}

/// The question, as the GRAPH every mechanism beyond [`homomorphism`] reads it out of.
///
/// A basic graph pattern and a conclusion graph are the same object with one difference —
/// whether the caller wants to SEE what a variable was bound to — so a pattern whose projected
/// variables have been replaced by blank nodes IS an RDF graph. Building it is what lets
/// [`certain_answers`] reach [`entails`]'s own fold rather than a second copy of it.
struct AsGraph {
    /// The graph itself.
    graph: Arc<RdfDataset>,
    /// Its own frozen triple order — the index space a [`Recognized`] speaks in, and NOT the
    /// caller's pattern order, because freezing deduplicates and re-orders.
    triples: Vec<Triple>,
    /// One rendering per triple above, in the caller's OWN syntax: `?x` where a projected
    /// variable was substituted away, so a limit names the construct the caller wrote rather
    /// than the blank node this module minted to stand in for it.
    rendered: Vec<String>,
}

/// `pats` with every projected variable replaced by a blank node no pattern names.
///
/// `None` when the substituted triples are not an RDF graph — a literal in subject position is
/// the reachable case. Such a pattern has no solution in ANY graph and no [`entails`] question
/// corresponds to it, so the ordinary match answers it with the empty relation it deserves;
/// what a hard error here would add is a refusal the caller never asked for.
fn as_graph(pats: &[PatTriple], names: &[String]) -> Option<AsGraph> {
    let mut fresh = FreshBlanks::avoiding_labels(&blank_labels(pats));
    // Minted in the pattern set's own first-occurrence variable order, so the substitution —
    // and therefore the frozen graph and every limit read off it — is a function of the
    // question alone.
    let substitution: BTreeMap<String, TermValue> = names
        .iter()
        .map(|name| (name.clone(), fresh.mint()))
        .collect();

    let mut origin: BTreeMap<Triple, usize> = BTreeMap::new();
    let mut builder = RdfDatasetBuilder::new();
    for (index, pat) in pats.iter().enumerate() {
        let triple = [
            substituted(&pat[0], &substitution)?,
            substituted(&pat[1], &substitution)?,
            substituted(&pat[2], &substitution)?,
        ];
        let s = intern_into(&mut builder, &triple[0]);
        let p = intern_into(&mut builder, &triple[1]);
        let o = intern_into(&mut builder, &triple[2]);
        builder.push_quad(s, p, o, None);
        // FIRST occurrence wins: two identical patterns freeze to one triple, and the earlier
        // is the one the caller would look for.
        origin.entry(triple).or_insert(index);
    }
    let graph = builder.freeze().ok()?;
    let triples = default_graph_triples(&graph);
    let rendered = triples
        .iter()
        .map(|triple| {
            origin.get(triple).map_or_else(
                || {
                    format!(
                        "{} {} {}",
                        show(&triple[0]),
                        show(&triple[1]),
                        show(&triple[2])
                    )
                },
                |&index| show_pattern(&pats[index]),
            )
        })
        .collect();
    Some(AsGraph {
        graph,
        triples,
        rendered,
    })
}

/// Every blank-node label `pats` names, at any depth — what a substitution must avoid.
fn blank_labels(pats: &[PatTriple]) -> BTreeSet<String> {
    fn walk(pat: &Pat, out: &mut BTreeSet<String>) {
        match pat {
            Pat::Var(VarKey::Blank { label, .. }) => {
                out.insert(label.clone());
            }
            Pat::Triple(inner) => {
                for position in inner.iter() {
                    walk(position, out);
                }
            }
            Pat::Var(VarKey::Projected(_)) | Pat::Ground(_) => {}
        }
    }
    let mut out = BTreeSet::new();
    for triple in pats {
        for position in triple {
            walk(position, &mut out);
        }
    }
    out
}

/// One pattern position as a TERM, under `substitution`.
///
/// `None` for a projected variable `substitution` does not name, which cannot happen for a
/// substitution built from [`projected_vars`] of the same patterns and is refused rather than
/// defaulted so it stays that way.
fn substituted(pat: &Pat, substitution: &BTreeMap<String, TermValue>) -> Option<TermValue> {
    Some(match pat {
        Pat::Ground(term) => term.clone(),
        Pat::Var(VarKey::Blank { label, scope }) => TermValue::Blank {
            label: label.clone(),
            scope: *scope,
        },
        Pat::Var(VarKey::Projected(name)) => substitution.get(name)?.clone(),
        Pat::Triple(inner) => TermValue::Triple {
            s: Box::new(substituted(&inner[0], substitution)?),
            p: Box::new(substituted(&inner[1], substitution)?),
            o: Box::new(substituted(&inner[2], substitution)?),
        },
    })
}

/// Decide one conclusion-directed question against a prepared run.
///
/// THE SHARED SPINE. Both public entry points reach their answer through this and neither
/// carries a second copy of it, so [`entails`] and [`certain_answers`] cannot disagree about a
/// question they can both be asked: the match first, because a found mapping is a proof that
/// needs no precondition; then the [`fold`], because a conclusion the rule table has no head
/// for is exactly the case a match cannot reach and one of the five lanes can.
///
/// # Errors
///
/// [`EntailError::MatchBudget`] from either match, and whatever the fold's own re-chases
/// refuse with.
fn decide(
    premise: &RdfDataset,
    conclusion: &RdfDataset,
    regime: Regime,
    closure: Closure,
    report: &ReasoningReport,
) -> Result<EntailmentOutcome, EntailError> {
    let pats: Vec<PatTriple> = conclusion_patterns(conclusion);
    Ok(match homomorphism::find_one(pats, &closure)? {
        // A found mapping is a proof, and it needs no precondition: the rule set is sound, so
        // a conclusion mapped into the closure is entailed whatever the premise's syntax.
        Ok(binding) => EntailmentOutcome::Entailed(EntailmentWarrant::Homomorphism(
            HomomorphismWarrant::new(regime, binding, closure),
        )),
        // No mapping. Before that is read as anything, the other mechanisms get their turn.
        // They run HERE and not earlier because each is strictly more expensive — a full
        // re-chase per negative fact or per frozen implication — and because the premise's
        // consistency, which every soundness argument requires, is what `prepare` established.
        Err(_) => fold(premise, conclusion, regime, closure, report)?,
    })
}

/// Does `premise` entail `conclusion` under `regime`?
///
/// The zero-projected-variable specialisation of [`certain_answers`]: an RDF graph is a
/// conjunction of triples whose blank nodes are existentially quantified, so a conclusion
/// GRAPH is a basic graph pattern with nothing to project, and its answer is a verdict
/// rather than a relation.
///
/// # The specialisation is REAL: one spine, two presentations
///
/// Both entry points reach their answer through one shared spine — the match, then the fold over
/// all five extra lanes, then the same [`precondition`] conditions — and neither carries a
/// second copy of it. So `entails(P, C)` and `certain_answers(P, patterns(C))` cannot disagree
/// about `C`: they are one call with two renderings of its result, and the test
/// `entails_and_certain_answers_never_disagree` ranges over every mechanism to keep it that
/// way. What differs is only that the binding is read as the WARRANT for a yes rather than as
/// an answer.
///
/// The two do differ where a question separates them, which is a question `entails` cannot be
/// asked: a PROJECTED variable. There the five extra lanes are not run, because "which
/// individuals is `a` entailed to differ from?" is a different question from "is `a` entailed
/// to differ from `b`?" — and [`certain_answers`] reports that as a named limit rather than as
/// an exhaustive empty answer.
///
/// # The verdict arrives WITH the run that produced it
///
/// The return is an [`EntailmentCertificate`], never a bare [`EntailmentOutcome`]. The
/// outcome carries the MECHANISM's evidence — a mapping, a refutation, a frozen chase — and
/// none of the chase's, so a caller reading `NotEntailed` had no way to ask whether the rule
/// table underneath it was complete, which rules fired, or which calculus it ran under.
/// [`EntailmentCertificate::report`] is that run, and it names the mechanism too. There is no
/// certificate-free entry point to route around it; see [`certificate`] for why the answer and
/// its provenance are one value.
///
/// # Errors
///
/// As [`certain_answers`].
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_entail::{EntailmentMechanism, EntailmentOutcome, ImportMap, Regime, entails};
///
/// let mut b = RdfDatasetBuilder::new();
/// let p = b.intern_iri("http://example.org/p");
/// let x = b.intern_iri("http://example.org/x");
/// let y = b.intern_iri("http://example.org/y");
/// let z = b.intern_iri("http://example.org/z");
/// let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// let transitive = b.intern_iri("http://www.w3.org/2002/07/owl#TransitiveProperty");
/// b.push_quad(p, ty, transitive, None);
/// b.push_quad(x, p, y, None);
/// b.push_quad(y, p, z, None);
/// let premise = b.freeze().expect("freeze");
///
/// // `x p z` follows by `prp-trp` and is not asserted.
/// let mut c = RdfDatasetBuilder::new();
/// let x = c.intern_iri("http://example.org/x");
/// let p = c.intern_iri("http://example.org/p");
/// let z = c.intern_iri("http://example.org/z");
/// c.push_quad(x, p, z, None);
/// let conclusion = c.freeze().expect("freeze");
///
/// let certificate = entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new())
///     .expect("a consistent premise");
/// assert!(matches!(certificate.outcome(), EntailmentOutcome::Entailed(_)));
/// // …and the certificate names the run that answered it.
/// assert_eq!(certificate.mechanism(), EntailmentMechanism::StrictTable);
/// assert_eq!(certificate.regime(), Regime::OwlRl);
/// assert!(!certificate.is_budget_exhausted());
/// assert!(certificate.report().rules_fired().iter().any(|&(rule, _)| rule.as_str() == "prp-trp"));
/// ```
pub fn entails(
    premise: &RdfDataset,
    conclusion: &RdfDataset,
    regime: Regime,
    imports: &ImportMap,
) -> Result<EntailmentCertificate, EntailError> {
    // Destructured rather than held whole: the closure MOVES into a homomorphism warrant
    // inside `decide` while the report moves into the certificate at the end, and two fields
    // of one binding cannot be handed to two owners through a method call.
    let Prepared {
        merged,
        closure,
        report,
    } = prepare(premise, regime, imports)?;
    let outcome = decide(
        merged.as_deref().unwrap_or(premise),
        conclusion,
        regime,
        closure,
        &report,
    )?;
    Ok(EntailmentCertificate::new(outcome, report))
}

/// Thread the conclusion's residual through every mechanism, then match whatever survives.
///
/// The loop FOLDS rather than stopping at the first lane that answers: entailment is monotone
/// over the conjunction a conclusion graph is, so a conclusion two lanes each read half of is
/// entailed when each half is. See the [module docs](self).
///
/// A lane's refusal is COLLECTED rather than returned on the spot, for the same reason: a lane
/// that declines a construct has said nothing about the conclusion's other triples, and
/// letting its refusal end the fold would withhold an answer another lane was about to reach.
/// The refusals are read only if the surviving residual does not map.
fn fold(
    premise: &RdfDataset,
    conclusion: &RdfDataset,
    regime: Regime,
    closure: Closure,
    report: &ReasoningReport,
) -> Result<EntailmentOutcome, EntailError> {
    let triples = default_graph_triples(conclusion);
    let mut pending: BTreeSet<usize> = (0..triples.len()).collect();
    let mut minted: Vec<Triple> = Vec::new();
    let mut parts: Vec<EntailmentWarrant> = Vec::new();
    let mut withheld: Vec<UndecidedReason> = Vec::new();
    for lane in MECHANISMS {
        let question = Question {
            premise,
            conclusion,
            regime,
            closure: &closure,
            triples: &triples,
            pending: &pending,
        };
        match (lane.attempt)(&question)? {
            Attempt::Entailed(established) => {
                let Established {
                    warrant,
                    discharged,
                    minted: licensed,
                    declined,
                } = *established;
                parts.push(warrant);
                pending.retain(|index| !discharged.contains(index));
                minted.extend(licensed);
                // A lane that minted something may STILL have declined something else, and the
                // admission is withheld exactly as a `Disqualified` one is: it is read only if
                // the surviving residual does not map, and then it names the construct nothing
                // tested rather than letting the failed match speak for it.
                if !declined.is_empty() {
                    withheld.push(UndecidedReason::ConstructNotRead {
                        lane: lane.mechanism,
                        constructs: declined,
                    });
                }
            }
            // "I stopped looking" and "I recognize this and decline to read it" are both
            // admissions, and an admission is never allowed to become a refutation.
            Attempt::Disqualified(reason) | Attempt::Undecided(reason) => withheld.push(reason),
            Attempt::NotApplicable | Attempt::NotEstablished => {}
        }
    }

    // Whatever no lane discharged keeps its ordinary obligation, against the closure the lanes
    // widened: a comprehended scaffold is entailed, so a later conclusion triple may land in
    // one.
    let extended = if minted.is_empty() {
        None
    } else {
        Some(closure.extended_with(minted))
    };
    let target = extended.as_ref().unwrap_or(&closure);
    let residual = patterns_at(&triples, &pending);
    match homomorphism::find_one(residual, target)? {
        Ok(binding) => Ok(EntailmentOutcome::Entailed(compose(
            regime, parts, binding, closure,
        ))),
        Err(miss) => {
            // Nothing reached it. What that MEANS is a lane's own admission if one was made,
            // and otherwise the precondition's answer — never any search's.
            let refutable = negation::lowering(conclusion)
                .map_or_else(BTreeSet::new, |lowering| lowering.consumed);
            let limits = precondition::limits(
                regime,
                premise,
                report,
                &conclusion_patterns(conclusion),
                &refutable,
            );
            Ok(
                match withheld
                    .into_iter()
                    .next()
                    .or_else(|| limits.into_iter().next())
                {
                    Some(reason) => EntailmentOutcome::Undecided(reason),
                    None => EntailmentOutcome::NotEntailed(miss),
                },
            )
        }
    }
}

/// The warrant for a fold that reached the conclusion.
///
/// Three shapes, and the arity decides which: no contributing lane is the ordinary
/// homomorphism, one lane is that lane's own warrant with the residual binding filled in, and
/// two or more is a [`Composite`](EntailmentWarrant::Composite). The middle case is what keeps
/// a `refutation` answer spelled `refutation` rather than renamed by a fold that happened to
/// run five lanes to reach it.
fn compose(
    regime: Regime,
    parts: Vec<EntailmentWarrant>,
    binding: Binding,
    closure: Closure,
) -> EntailmentWarrant {
    let mut parts = parts;
    match parts.len() {
        0 => EntailmentWarrant::Homomorphism(HomomorphismWarrant::new(regime, binding, closure)),
        1 => parts
            .pop()
            .expect("a one-element vector pops")
            .with_binding(binding),
        _ => EntailmentWarrant::Composite(CompositeWarrant::new(regime, parts, binding, closure)),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};

    use super::{
        CertainAnswers, CompositeWarrant, EntailmentMechanism, EntailmentOutcome,
        EntailmentWarrant, ImportMap, MissReason, UndecidedReason, certain_answers, entails,
        verify,
    };
    use crate::owl_dl::query::{QNode, QTriple};
    use crate::{EntailError, Regime};

    const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
    const DISJOINT: &str = "http://www.w3.org/2002/07/owl#disjointWith";
    const SOMEVALUES: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
    const ONPROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
    const RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";

    fn graph(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let s = b.intern_iri(s);
            let p = b.intern_iri(p);
            let o = b.intern_iri(o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    /// `A ⊑ ∃p.B`, `x a A` — an existential on the SUPERCLASS side, which is OUTSIDE the OWL
    /// 2 RL syntax, so Theorem PR1's completeness half does not apply to it.
    fn non_rl_premise() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/A");
        let sub = b.intern_iri(SUBCLASS);
        let restriction = b.intern_blank("r", purrdf_core::BlankScope::DEFAULT);
        let ty = b.intern_iri(TYPE);
        let class = b.intern_iri(RESTRICTION);
        let on = b.intern_iri(ONPROPERTY);
        let p = b.intern_iri("http://example.org/p");
        let some = b.intern_iri(SOMEVALUES);
        let bb = b.intern_iri("http://example.org/B");
        b.push_quad(a, sub, restriction, None);
        b.push_quad(restriction, ty, class, None);
        b.push_quad(restriction, on, p, None);
        b.push_quad(restriction, some, bb, None);
        let x = b.intern_iri("http://example.org/x");
        b.push_quad(x, ty, a, None);
        b.freeze().expect("freeze")
    }

    /// The premise `A ⊑ B`, `x a A` — enough for `cax-sco` to type `x` a `B`.
    fn subclass_premise() -> Arc<RdfDataset> {
        graph(&[
            ("http://example.org/A", SUBCLASS, "http://example.org/B"),
            ("http://example.org/x", TYPE, "http://example.org/A"),
        ])
    }

    /// The outcome of one question, with its certificate discarded — for the assertions
    /// that are about the verdict alone.
    fn outcome(premise: &RdfDataset, conclusion: &RdfDataset, regime: Regime) -> EntailmentOutcome {
        entails(premise, conclusion, regime, &ImportMap::new())
            .expect("consistent")
            .into_parts()
            .0
    }

    /// A DERIVED conclusion is entailed, and the warrant re-checks.
    #[test]
    fn a_derived_conclusion_is_entailed_and_the_warrant_verifies() {
        let premise = subclass_premise();
        let conclusion = graph(&[("http://example.org/x", TYPE, "http://example.org/B")]);
        let EntailmentOutcome::Entailed(warrant) = outcome(&premise, &conclusion, Regime::OwlRl)
        else {
            panic!("cax-sco derives it");
        };
        assert_eq!(warrant.regime(), Regime::OwlRl);
        assert!(verify(&warrant, &premise, &conclusion));
        // The warrant is against THIS premise: a closure that does not hold the premise's
        // own triples is not a warrant for it.
        let other = graph(&[("http://example.org/q", TYPE, "http://example.org/Q")]);
        assert!(!verify(&warrant, &other, &conclusion));
        // …and it is against THIS conclusion.
        let unrelated = graph(&[("http://example.org/x", TYPE, "http://example.org/Never")]);
        assert!(!verify(&warrant, &premise, &unrelated));
    }

    /// A conclusion nothing derives, over a premise inside OWL 2 RL, is a PROOF of
    /// non-entailment — and it says which triple was missing.
    #[test]
    fn an_rl_premise_refutes_and_names_the_missing_triple() {
        let premise = subclass_premise();
        let conclusion = graph(&[("http://example.org/x", TYPE, "http://example.org/Never")]);
        let EntailmentOutcome::NotEntailed(MissReason::NoCandidate(missing)) =
            outcome(&premise, &conclusion, Regime::OwlRl)
        else {
            panic!("the closure of an RL premise refutes");
        };
        assert_eq!(missing.len(), 1);
        assert!(missing[0].contains("Never"), "{missing:?}");
    }

    /// THE CENTRAL DISTINCTION: a premise OUTSIDE OWL 2 RL cannot refute.
    ///
    /// `owl:someValuesFrom` in SUPERCLASS position is outside the RL syntax, so Theorem PR1's
    /// completeness half does not apply and a failed match proves nothing. The same
    /// conclusion over an RL premise refutes (above), so this is a fact about the
    /// PRECONDITION rather than about the conclusion.
    #[test]
    fn a_non_rl_premise_is_undecided_rather_than_refuted() {
        let premise = non_rl_premise();
        let conclusion = graph(&[("http://example.org/x", TYPE, "http://example.org/Never")]);
        let EntailmentOutcome::Undecided(UndecidedReason::PremiseOutsideRl(violations)) =
            outcome(&premise, &conclusion, Regime::OwlRl)
        else {
            panic!("an existential in superclass position is outside OWL 2 RL");
        };
        assert!(!violations.is_empty());
    }

    /// AN INCONSISTENT PREMISE ENTAILS EVERYTHING, SO IT IS REFUSED.
    ///
    /// Falsifiable against the failure mode this service is arranged to prevent: without the
    /// consistency check the closure would be matched anyway, and the answer for this
    /// conclusion — for EVERY conclusion — would be `Entailed`.
    #[test]
    fn an_inconsistent_premise_refuses_rather_than_entailing_everything() {
        let premise = graph(&[
            ("http://example.org/A", DISJOINT, "http://example.org/B"),
            ("http://example.org/x", TYPE, "http://example.org/A"),
            ("http://example.org/x", TYPE, "http://example.org/B"),
        ]);
        let conclusion = graph(&[("http://example.org/anything", TYPE, "http://example.org/At")]);
        let Err(EntailError::Inconsistent(run)) =
            entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new())
        else {
            panic!("two disjoint classes with a shared instance is `cax-dw`");
        };
        assert_eq!(run.report().regime(), Regime::OwlRl);
        assert!(run.report().inconsistency().is_some());
    }

    /// A blank node of the conclusion is an EXISTENTIAL, and the warrant says what it was.
    #[test]
    fn a_conclusion_blank_node_is_bound_and_the_binding_is_the_warrant() {
        let premise = subclass_premise();
        let mut c = RdfDatasetBuilder::new();
        let some = c.intern_blank("who", purrdf_core::BlankScope::DEFAULT);
        let ty = c.intern_iri(TYPE);
        let bb = c.intern_iri("http://example.org/B");
        c.push_quad(some, ty, bb, None);
        let conclusion = c.freeze().expect("freeze");

        let EntailmentOutcome::Entailed(warrant) = outcome(&premise, &conclusion, Regime::OwlRl)
        else {
            panic!("`_:who a B` holds of `x`");
        };
        assert_eq!(warrant.binding().len(), 1);
        assert_eq!(
            warrant.binding().values().next(),
            Some(&TermValue::iri("http://example.org/x"))
        );
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// The two regimes defined by an input this signature does not carry are REFUSED, by
    /// name, rather than served by a weaker lane.
    #[test]
    fn a_regime_this_service_cannot_serve_is_named() {
        let premise = subclass_premise();
        let conclusion = graph(&[("http://example.org/x", TYPE, "http://example.org/B")]);
        for regime in [Regime::OwlDirect, Regime::Rif] {
            let Err(EntailError::UnsupportedRegime(refused)) =
                entails(&premise, &conclusion, regime, &ImportMap::new())
            else {
                panic!("{regime:?} is defined by an input this signature does not carry");
            };
            assert_eq!(refused, regime);
        }
        // …and the five that ARE served all answer.
        for regime in [
            Regime::Simple,
            Regime::Rdf,
            Regime::Rdfs,
            Regime::OwlRl,
            Regime::D,
        ] {
            entails(&premise, &conclusion, regime, &ImportMap::new())
                .unwrap_or_else(|e| panic!("{regime:?}: {e}"));
        }
    }

    /// `Simple` entailment is the identity closure plus the match, so it entails what is
    /// ASSERTED and refutes what is not — and its refutation is a proof, because the
    /// interpolation lemma leaves nothing to be incomplete about.
    #[test]
    fn simple_entailment_refutes_without_a_precondition() {
        let premise = subclass_premise();
        let asserted = graph(&[("http://example.org/x", TYPE, "http://example.org/A")]);
        assert!(matches!(
            outcome(&premise, &asserted, Regime::Simple),
            EntailmentOutcome::Entailed(_)
        ));
        let derived = graph(&[("http://example.org/x", TYPE, "http://example.org/B")]);
        assert!(
            matches!(
                outcome(&premise, &derived, Regime::Simple),
                EntailmentOutcome::NotEntailed(_)
            ),
            "Simple entailment draws no conclusion, and says so as a PROOF"
        );
    }

    /// `D` can prove an entailment and never refutes one, and it says which limit that is.
    #[test]
    fn the_d_lane_proves_but_does_not_refute() {
        let premise = subclass_premise();
        let derived = graph(&[("http://example.org/x", TYPE, "http://example.org/B")]);
        assert!(matches!(
            outcome(&premise, &derived, Regime::D),
            EntailmentOutcome::Undecided(UndecidedReason::DatatypeValueSpace)
        ));
    }

    /// A projected variable enumerates the ENTAILED bindings, and the answer set says
    /// whether it is exhaustive.
    #[test]
    fn certain_answers_enumerate_entailed_bindings_and_disclose_completeness() {
        let premise = subclass_premise();
        let bgp = [QTriple {
            s: QNode::Term(TermValue::iri("http://example.org/x")),
            p: QNode::Term(TermValue::iri(TYPE)),
            o: QNode::Var("c".to_owned()),
        }];
        let answers: CertainAnswers =
            certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new()).expect("consistent");
        assert_eq!(answers.vars(), ["c"]);
        assert!(answers.is_complete(), "{:?}", answers.limits());
        for class in ["http://example.org/A", "http://example.org/B"] {
            assert!(
                answers
                    .rows()
                    .iter()
                    .any(|row| row == &[TermValue::iri(class)]),
                "{class} is an entailed type of x: {:?}",
                answers.rows()
            );
        }
        // The rows are deduplicated and ordered by the row itself, so two runs agree.
        let again =
            certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new()).expect("consistent");
        assert_eq!(answers.rows(), again.rows());
    }

    // ── COMPOSITION: entailment is monotone over the conjunction a conclusion graph is ───

    const COMPLEMENTOF: &str = "http://www.w3.org/2002/07/owl#complementOf";
    const DIFFERENTFROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
    const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
    const OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
    const TRANSITIVE: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
    const REFLEXIVE: &str = "http://www.w3.org/2002/07/owl#ReflexiveProperty";
    const CHAIN: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";
    const ONEOF: &str = "http://www.w3.org/2002/07/owl#oneOf";
    const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
    const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
    const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

    const BOY: &str = "http://example.org/Boy";
    const GIRL: &str = "http://example.org/Girl";
    const STEWIE: &str = "http://example.org/Stewie";
    const PETER: &str = "http://example.org/Peter";
    const P: &str = "http://example.org/p";
    const KNOWS: &str = "http://example.org/knows";

    /// A default-graph dataset; a leading `_` names a blank node, anything else an IRI.
    fn mixed(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let term = |b: &mut RdfDatasetBuilder, value: &str| match value.strip_prefix('_') {
                Some(label) => b.intern_blank(label, purrdf_core::BlankScope::DEFAULT),
                None => b.intern_iri(value),
            };
            let s = term(&mut b, s);
            let p = term(&mut b, p);
            let o = term(&mut b, o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    /// A premise that gives each of three lanes something to establish, and nothing else.
    ///
    /// `Boy ⊓ Girl = ⊥` with `Stewie : Boy` is the refutation lane's shape; `p ∘ p ⊑ p` is
    /// the freeze lane's; `knows` being reflexive is the reflexivity lane's.
    fn three_lane_premise() -> Arc<RdfDataset> {
        mixed(&[
            (BOY, TYPE, OWL_CLASS),
            (GIRL, TYPE, OWL_CLASS),
            (BOY, DISJOINT, GIRL),
            (STEWIE, TYPE, BOY),
            (P, TYPE, OBJECT_PROPERTY),
            (P, CHAIN, "_l1"),
            ("_l1", FIRST, P),
            ("_l1", REST, "_l2"),
            ("_l2", FIRST, P),
            ("_l2", REST, NIL),
            (KNOWS, TYPE, REFLEXIVE),
        ])
    }

    /// `Stewie : ¬Girl` — the refutation lane's half of a conclusion.
    const COMPLEMENT_HALF: [(&str, &str, &str); 3] = [
        ("_c", TYPE, OWL_CLASS),
        ("_c", COMPLEMENTOF, GIRL),
        (STEWIE, TYPE, "_c"),
    ];

    /// The mechanisms a warrant names, composite or not.
    fn contributors(warrant: &EntailmentWarrant) -> Vec<EntailmentMechanism> {
        match warrant {
            EntailmentWarrant::Composite(composite) => composite.mechanisms(),
            other => vec![other.mechanism()],
        }
    }

    /// TWO MECHANISMS AND A HOMOMORPHISM, over ONE conclusion.
    ///
    /// Each half is entailed on its own — `Stewie : ¬Girl` by refutation, `p` transitive by
    /// freezing `p ∘ p ⊑ p` — and `Girl a owl:Class` maps into the closure like any other
    /// triple. Falsifiable against the exact defect this fold replaced: with each lane
    /// matching a residual of its own, refutation scored the transitivity axiom as an
    /// unmatched residual and freeze scored the complement scaffold as one, so neither
    /// established anything and the fall-through answered `NotEntailed` — a PROOF, of
    /// something false, because entailment is monotone over conjunction.
    #[test]
    fn two_mechanisms_and_a_residual_compose_into_one_warrant() {
        let premise = three_lane_premise();
        let mut triples = COMPLEMENT_HALF.to_vec();
        triples.push((P, TYPE, TRANSITIVE));
        triples.push((GIRL, TYPE, OWL_CLASS));
        let conclusion = mixed(&triples);

        // Each half alone is reached by exactly one lane, and by its own name.
        assert_eq!(
            contributors(&entailed(&premise, &mixed(&COMPLEMENT_HALF))),
            [EntailmentMechanism::Refutation]
        );
        assert_eq!(
            contributors(&entailed(&premise, &mixed(&[(P, TYPE, TRANSITIVE)]))),
            [EntailmentMechanism::Freeze]
        );

        // …and both together are reached by BOTH, under a name that is neither of theirs.
        let warrant = entailed(&premise, &conclusion);
        assert_eq!(
            warrant.mechanism(),
            EntailmentMechanism::Composite,
            "a composite answer that rendered as one constituent's name would tell a consumer \
             that one mechanism sufficed"
        );
        assert_eq!(
            contributors(&warrant),
            [EntailmentMechanism::Refutation, EntailmentMechanism::Freeze]
        );
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// THREE mechanisms over one conclusion, plus the residual match.
    #[test]
    fn three_mechanisms_compose_over_one_conclusion() {
        let premise = three_lane_premise();
        let mut triples = COMPLEMENT_HALF.to_vec();
        triples.push((P, TYPE, TRANSITIVE));
        triples.push((STEWIE, KNOWS, STEWIE));
        triples.push((STEWIE, TYPE, BOY));
        let conclusion = mixed(&triples);

        let warrant = entailed(&premise, &conclusion);
        assert_eq!(
            contributors(&warrant),
            [
                EntailmentMechanism::Refutation,
                EntailmentMechanism::Freeze,
                EntailmentMechanism::Reflexivity,
            ],
            "the constituents are the fixed MECHANISMS cost order, not the conclusion's own"
        );
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// THE ANSWER IS A FUNCTION OF THE INPUTS ALONE, not of the order the caller wrote.
    ///
    /// The same conclusion with its triples permuted returns the identical verdict and the
    /// identical warrant — the same constituents, in the same order, with the same binding.
    #[test]
    fn a_permuted_conclusion_returns_the_identical_warrant() {
        let premise = three_lane_premise();
        let mut forward = COMPLEMENT_HALF.to_vec();
        forward.push((P, TYPE, TRANSITIVE));
        forward.push((GIRL, TYPE, OWL_CLASS));
        let mut backward = forward.clone();
        backward.reverse();
        // …and one more permutation that puts the FREEZE axiom first, so the conclusion's own
        // triple order disagrees with the fold's cost order rather than merely differing.
        let mut freeze_first = vec![(P, TYPE, TRANSITIVE), (GIRL, TYPE, OWL_CLASS)];
        freeze_first.extend(COMPLEMENT_HALF);

        let mut seen: Vec<(Vec<EntailmentMechanism>, String)> = Vec::new();
        for order in [forward, backward, freeze_first] {
            let conclusion = mixed(&order);
            let warrant = entailed(&premise, &conclusion);
            assert!(verify(&warrant, &premise, &conclusion));
            seen.push((contributors(&warrant), format!("{:?}", warrant.binding())));
        }
        assert_eq!(seen[0], seen[1]);
        assert_eq!(seen[0], seen[2]);
        assert_eq!(
            seen[0].0,
            [EntailmentMechanism::Refutation, EntailmentMechanism::Freeze]
        );
    }

    /// `verify` ACCEPTS a composite and REJECTS every way of doctoring one.
    ///
    /// Four forgeries, one per thing the check re-decides: another premise, another
    /// conclusion, a REORDERED constituent list, and a composite of one.
    #[test]
    fn a_composite_warrant_does_not_replay() {
        let premise = three_lane_premise();
        let mut triples = COMPLEMENT_HALF.to_vec();
        triples.push((P, TYPE, TRANSITIVE));
        triples.push((GIRL, TYPE, OWL_CLASS));
        let conclusion = mixed(&triples);
        let warrant = entailed(&premise, &conclusion);
        assert!(verify(&warrant, &premise, &conclusion));

        // Another PREMISE: no constituent's closure holds it.
        assert!(!verify(&warrant, &subclass_premise(), &conclusion));
        // Another CONCLUSION: the halves it states are different ones.
        assert!(!verify(
            &warrant,
            &premise,
            &mixed(&[(P, TYPE, TRANSITIVE), (GIRL, TYPE, OWL_CLASS)])
        ));

        let EntailmentWarrant::Composite(composite) = &warrant else {
            panic!("two mechanisms compose into a composite");
        };
        // REORDERED: a warrant whose constituents are not in `MECHANISMS` order replays
        // against a different pending set at every step, so it is not a warrant at all.
        let mut reversed = composite.parts().to_vec();
        reversed.reverse();
        let forged = EntailmentWarrant::Composite(CompositeWarrant::new(
            composite.regime(),
            reversed,
            composite.binding().clone(),
            forged_closure(&warrant),
        ));
        assert!(!verify(&forged, &premise, &conclusion));

        // A composite of ONE is a shape the fold never mints: one lane is that lane's own
        // warrant, so a single-constituent composite is a relabelling of it.
        let alone = EntailmentWarrant::Composite(CompositeWarrant::new(
            composite.regime(),
            vec![composite.parts()[0].clone()],
            composite.binding().clone(),
            forged_closure(&warrant),
        ));
        assert!(!verify(&alone, &premise, &conclusion));
    }

    /// AN EXISTENTIAL NEGATIVE FACT IS AN ADMISSION, NEVER A REFUTATION.
    ///
    /// `_:x owl:differentFrom Peter` asks whether SOMETHING is entailed different from
    /// `Peter`; the refutation lane would have to choose a witness to negate, and it declines.
    /// The ground question over the same premise IS decided, so this is a fact about the
    /// existential rather than about the premise.
    #[test]
    fn an_existential_negative_fact_is_undecided_and_names_the_construct() {
        let mut triples = default_disjoint();
        triples.push((PETER, TYPE, GIRL));
        let premise = mixed(&triples);

        assert!(matches!(
            outcome(
                &premise,
                &mixed(&[(STEWIE, DIFFERENTFROM, PETER)]),
                Regime::OwlRl
            ),
            EntailmentOutcome::Entailed(_)
        ));

        let EntailmentOutcome::Undecided(UndecidedReason::ConstructNotRead { lane, constructs }) =
            outcome(
                &premise,
                &mixed(&[("_x", DIFFERENTFROM, PETER)]),
                Regime::OwlRl,
            )
        else {
            panic!("declining to search for a witness is an admission, not a refutation");
        };
        assert_eq!(lane, EntailmentMechanism::Refutation);
        assert!(
            constructs.iter().any(|why| why.contains("witness")),
            "{constructs:?}"
        );
    }

    /// A CONSTRUCT A LANE'S WHITELIST REFUSES IS AN ADMISSION, NEVER A REFUTATION.
    ///
    /// `owl:oneOf` is a class constructor the comprehension lane names and does not read, and
    /// the answer says so by name rather than reporting a proof of non-entailment about a
    /// conclusion nothing tested.
    #[test]
    fn a_whitelist_refused_constructor_is_undecided_and_names_the_construct() {
        let premise = mixed(&[("http://example.org/a", TYPE, OWL_CLASS)]);
        let conclusion = mixed(&[
            ("_u", TYPE, OWL_CLASS),
            ("_u", ONEOF, "_l1"),
            ("_l1", FIRST, "http://example.org/a"),
            ("_l1", REST, NIL),
        ]);
        let EntailmentOutcome::Undecided(UndecidedReason::ConstructNotRead { lane, constructs }) =
            outcome(&premise, &conclusion, Regime::OwlRl)
        else {
            panic!("a construct a whitelist refuses is an admission of incapacity");
        };
        assert_eq!(lane, EntailmentMechanism::Comprehension);
        assert!(
            constructs.iter().any(|why| why.contains("oneOf")),
            "{constructs:?}"
        );
    }

    /// A CONCLUSION OUTSIDE PR1'S SECOND HALF IS UNDECIDED, and it names the triple.
    ///
    /// `p rdf:type owl:IrreflexiveProperty` is a property characteristic no head in Tables
    /// 4–9 has, and no lane reads it either — freeze excludes it because its defining
    /// condition is not Horn. So nothing tested it, and the conclusion-side clause says so.
    #[test]
    fn a_schema_conclusion_no_lane_reads_is_undecided_and_names_the_triple() {
        let premise = mixed(&[(P, TYPE, OBJECT_PROPERTY)]);
        let conclusion = mixed(&[(P, TYPE, "http://www.w3.org/2002/07/owl#IrreflexiveProperty")]);
        let EntailmentOutcome::Undecided(UndecidedReason::ConclusionOutsideRl(triples)) =
            outcome(&premise, &conclusion, Regime::OwlRl)
        else {
            panic!("no mechanism reads an owl:IrreflexiveProperty conclusion");
        };
        assert_eq!(triples.len(), 1);
        assert!(triples[0].contains("IrreflexiveProperty"), "{triples:?}");
    }

    /// …AND `NotEntailed` IS STILL REACHED, WITH A REAL PROOF.
    ///
    /// The cheap way to make everything above pass is to answer `Undecided` everywhere, which
    /// would break the service rather than fix it. This is the genuine case the three answers
    /// exist for: an RL-syntax premise, an assertional ground conclusion over named terms,
    /// every mechanism run to completion, and no mapping. Both a class assertion and a
    /// property assertion, because the conclusion-side clause reads the two differently.
    #[test]
    fn an_assertional_conclusion_over_an_rl_premise_is_still_refuted() {
        let premise = mixed(&default_disjoint());
        for conclusion in [
            mixed(&[(STEWIE, TYPE, GIRL)]),
            mixed(&[(STEWIE, KNOWS, PETER)]),
            mixed(&[(STEWIE, TYPE, GIRL), (STEWIE, KNOWS, PETER)]),
        ] {
            let EntailmentOutcome::NotEntailed(miss) =
                outcome(&premise, &conclusion, Regime::OwlRl)
            else {
                panic!("an assertional conclusion over an RL premise is REFUTED, not shrugged at");
            };
            assert!(!miss.summary().is_empty());
        }
        // …and the same premise still refutes a negative fact the refutation lane RAN on:
        // there is no unique-name assumption, so nothing separates two unrelated individuals.
        assert!(matches!(
            outcome(
                &premise,
                &mixed(&[(STEWIE, DIFFERENTFROM, PETER)]),
                Regime::OwlRl
            ),
            EntailmentOutcome::NotEntailed(_)
        ));
    }

    /// `Boy ⊓ Girl = ⊥`, `Stewie : Boy` — an OWL 2 RL premise, as triples.
    fn default_disjoint() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            (BOY, TYPE, OWL_CLASS),
            (GIRL, TYPE, OWL_CLASS),
            (BOY, DISJOINT, GIRL),
            (STEWIE, TYPE, BOY),
        ]
    }

    /// The warrant of an entailed answer, or a panic naming the question that failed.
    fn entailed(premise: &RdfDataset, conclusion: &RdfDataset) -> EntailmentWarrant {
        match outcome(premise, conclusion, Regime::OwlRl) {
            EntailmentOutcome::Entailed(warrant) => warrant,
            other => panic!("expected an entailment, got {other:?}"),
        }
    }

    /// A copy of `warrant`'s own premise closure, for building a forgery to be rejected.
    fn forged_closure(warrant: &EntailmentWarrant) -> crate::entails::homomorphism::Closure {
        warrant.closure().clone()
    }

    // ── `entails` IS `certain_answers` WITH NOTHING TO PROJECT ───────────────────────────

    const RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
    const DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
    const UNIONOF: &str = "http://www.w3.org/2002/07/owl#unionOf";
    const RDF_LIST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#List";
    const XSD_BYTE: &str = "http://www.w3.org/2001/XMLSchema#byte";
    const XSD_SHORT: &str = "http://www.w3.org/2001/XMLSchema#short";
    const IRREFLEXIVE: &str = "http://www.w3.org/2002/07/owl#IrreflexiveProperty";
    const A: &str = "http://example.org/a";

    /// The conclusion graph `ds` as the basic graph pattern `patterns(C)`.
    ///
    /// Every term is a term and every blank node stays a blank node — which SPARQL reads as a
    /// non-distinguished variable and RDF 1.2 Semantics reads as an existential, the same
    /// reading — so nothing is projected and the two questions are the SAME question.
    fn patterns_of(ds: &RdfDataset) -> Vec<QTriple> {
        crate::entails::graph::default_graph_triples(ds)
            .into_iter()
            .map(|[s, p, o]| QTriple {
                s: QNode::Term(s),
                p: QNode::Term(p),
                o: QNode::Term(o),
            })
            .collect()
    }

    /// `_:c owl:unionOf (ex:a)` with `ex:a` a class — the comprehension lane's shape.
    fn union_conclusion() -> Arc<RdfDataset> {
        mixed(&[
            ("_c", TYPE, OWL_CLASS),
            ("_c", UNIONOF, "_l"),
            ("_l", TYPE, RDF_LIST),
            ("_l", FIRST, A),
            (A, TYPE, OWL_CLASS),
            ("_l", REST, NIL),
        ])
    }

    /// `_:u owl:oneOf (ex:a)` — a class constructor the comprehension lane names and declines.
    fn one_of_conclusion() -> Arc<RdfDataset> {
        mixed(&[
            ("_u", TYPE, OWL_CLASS),
            ("_u", ONEOF, "_l1"),
            ("_l1", FIRST, A),
            ("_l1", REST, NIL),
        ])
    }

    /// THE TWO ENTRY POINTS NEVER DISAGREE, over every mechanism there is.
    ///
    /// A conclusion GRAPH is a basic graph pattern with nothing to project, so
    /// `entails(P, C)` and `certain_answers(P, patterns(C))` are one question — and this
    /// ranges over every one of the seven mechanisms, `composite` included, plus a refutation,
    /// plus all three shapes of `undecided`. Falsifiable against the exact defect it replaced:
    /// `certain_answers` ran the homomorphism lane ALONE, so `ex:Stewie owl:differentFrom
    /// ex:Peter` came back as an empty row set with an empty limit list — "no certain answers,
    /// exhaustively" — while `entails` proved it by refutation on the byte-identical question.
    ///
    /// Four claims per case, because agreeing about the verdict alone would let the two
    /// disagree about everything a caller reads beside it: the VERDICT, the MECHANISM, whether
    /// the answer is DECIDED (which over zero columns is exactly whether the row set is
    /// exhaustive, since the empty substitution is the only substitution there is), and the
    /// REASON an undecided answer carries.
    #[test]
    fn entails_and_certain_answers_never_disagree() {
        let ranges = mixed(&[(P, TYPE, DATATYPE_PROPERTY), (P, RANGE, XSD_BYTE)]);
        let mut composite = COMPLEMENT_HALF.to_vec();
        composite.push((P, TYPE, TRANSITIVE));
        composite.push((GIRL, TYPE, OWL_CLASS));
        let mut with_peter = default_disjoint();
        with_peter.push((PETER, TYPE, GIRL));

        let cases: [(&str, Arc<RdfDataset>, Arc<RdfDataset>, EntailmentMechanism); 11] = [
            (
                "the table derives it",
                subclass_premise(),
                graph(&[("http://example.org/x", TYPE, "http://example.org/B")]),
                EntailmentMechanism::StrictTable,
            ),
            (
                "the table refutes it",
                subclass_premise(),
                graph(&[("http://example.org/x", TYPE, "http://example.org/Never")]),
                EntailmentMechanism::StrictTable,
            ),
            (
                "a negative fact, by refutation",
                mixed(&with_peter),
                mixed(&[(STEWIE, DIFFERENTFROM, PETER)]),
                EntailmentMechanism::Refutation,
            ),
            (
                "a schema axiom, by freezing",
                three_lane_premise(),
                mixed(&[(P, TYPE, TRANSITIVE)]),
                EntailmentMechanism::Freeze,
            ),
            (
                "an anonymous class, by comprehension",
                mixed(&[(A, TYPE, OWL_CLASS)]),
                union_conclusion(),
                EntailmentMechanism::Comprehension,
            ),
            (
                "a self-loop, by reflexivity",
                three_lane_premise(),
                mixed(&[(STEWIE, KNOWS, STEWIE)]),
                EntailmentMechanism::Reflexivity,
            ),
            (
                "a range axiom, by datatype containment",
                ranges,
                mixed(&[(P, RANGE, XSD_SHORT)]),
                EntailmentMechanism::DataRange,
            ),
            (
                "two lanes and a residual, composed",
                three_lane_premise(),
                mixed(&composite),
                EntailmentMechanism::Composite,
            ),
            (
                "undecided: the premise is outside OWL 2 RL",
                non_rl_premise(),
                graph(&[("http://example.org/x", TYPE, "http://example.org/Never")]),
                EntailmentMechanism::StrictTable,
            ),
            (
                "undecided: the conclusion is outside OWL 2 RL",
                mixed(&[(P, TYPE, OBJECT_PROPERTY)]),
                mixed(&[(P, TYPE, IRREFLEXIVE)]),
                EntailmentMechanism::StrictTable,
            ),
            (
                "undecided: a lane recognizes a construct and declines it",
                mixed(&[(A, TYPE, OWL_CLASS)]),
                one_of_conclusion(),
                EntailmentMechanism::Comprehension,
            ),
        ];

        // Every one of the seven is exercised, so the table cannot decay into eleven cases
        // that all take one path.
        let covered: BTreeSet<EntailmentMechanism> =
            cases.iter().map(|&(_, _, _, lane)| lane).collect();
        assert_eq!(
            covered.len(),
            7,
            "every mechanism must be represented: {covered:?}"
        );

        for (name, premise, conclusion, expected) in cases {
            let certificate = entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new())
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let answers = certain_answers(
                &premise,
                &patterns_of(&conclusion),
                Regime::OwlRl,
                &ImportMap::new(),
            )
            .unwrap_or_else(|e| panic!("{name}: {e}"));

            assert_eq!(certificate.mechanism(), expected, "{name}");
            // THE VERDICT. A `yes` is the one row over zero columns; a `no` is no row at all.
            assert_eq!(
                matches!(certificate.outcome(), EntailmentOutcome::Entailed(_)),
                !answers.is_empty(),
                "{name}: {:?} against {:?}",
                certificate.outcome(),
                answers.rows()
            );
            // THE MECHANISM, which a caller renders beside the verdict.
            assert_eq!(certificate.mechanism(), answers.mechanism(), "{name}");
            // DECIDED and EXHAUSTIVE are one claim here, and both entry points make it.
            assert_eq!(certificate.is_decided(), answers.is_complete(), "{name}");
            // …and an undecided answer carries the SAME reason, not merely the same shape.
            if let EntailmentOutcome::Undecided(reason) = certificate.outcome() {
                assert_eq!(answers.limits(), std::slice::from_ref(reason), "{name}");
            }
            assert!(answers.vars().is_empty(), "{name}");
        }
    }

    /// …and they agree under every regime this service serves, not only `OWL-RL`.
    #[test]
    fn the_two_entry_points_agree_under_every_served_regime() {
        let premise = subclass_premise();
        for regime in [
            Regime::Simple,
            Regime::Rdf,
            Regime::Rdfs,
            Regime::OwlRl,
            Regime::D,
        ] {
            for object in [
                "http://example.org/A",
                "http://example.org/B",
                "http://example.org/Never",
            ] {
                let conclusion = graph(&[("http://example.org/x", TYPE, object)]);
                let certificate = entails(&premise, &conclusion, regime, &ImportMap::new())
                    .expect("a consistent premise");
                let answers = certain_answers(
                    &premise,
                    &patterns_of(&conclusion),
                    regime,
                    &ImportMap::new(),
                )
                .expect("a consistent premise");
                assert_eq!(
                    matches!(certificate.outcome(), EntailmentOutcome::Entailed(_)),
                    !answers.is_empty(),
                    "{regime:?} {object}"
                );
                assert_eq!(
                    certificate.is_decided(),
                    answers.is_complete(),
                    "{regime:?} {object}"
                );
                assert_eq!(
                    certificate.mechanism(),
                    answers.mechanism(),
                    "{regime:?} {object}"
                );
            }
        }
    }

    // ── A LANE NOT RUN IS A LIMIT, NOT A SILENCE ────────────────────────────────────────

    /// A PROJECTED VARIABLE OVER A REFUTATION IS AN INCOMPLETE ANSWER THAT SAYS SO.
    ///
    /// `?x owl:differentFrom ex:Peter` asks which individuals are entailed different from
    /// `Peter`, which needs a refutation per candidate over the whole domain — so this service
    /// declines to answer it, exactly as the module docs argue it should. What it must not do
    /// is render that as an EXHAUSTIVE empty answer: `ex:Stewie` demonstrably IS a certain
    /// answer, because `entails` proves the ground question by refutation. Falsifiable against
    /// the defect: before the limit existed this rendered `var x` and nothing else.
    #[test]
    fn a_projected_variable_over_a_refutation_is_a_named_limit() {
        let mut triples = default_disjoint();
        triples.push((PETER, TYPE, GIRL));
        let premise = mixed(&triples);

        // The GROUND question is decided, and `ex:Stewie` is the answer the projected one
        // would have to contain.
        assert!(matches!(
            outcome(
                &premise,
                &mixed(&[(STEWIE, DIFFERENTFROM, PETER)]),
                Regime::OwlRl
            ),
            EntailmentOutcome::Entailed(_)
        ));

        let bgp = [QTriple {
            s: QNode::Var("x".to_owned()),
            p: QNode::Term(TermValue::iri(DIFFERENTFROM)),
            o: QNode::Term(TermValue::iri(PETER)),
        }];
        let answers =
            certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new()).expect("consistent");
        assert_eq!(answers.vars(), ["x"]);
        assert!(
            answers.rows().is_empty(),
            "this service does not search the domain for a witness: {:?}",
            answers.rows()
        );
        assert!(
            !answers.is_complete(),
            "an empty row set beside an empty limit list claims `ex:Stewie` is not an answer"
        );
        let [UndecidedReason::ConstructNotRead { lane, constructs }] = answers.limits() else {
            panic!("the limit must NAME the lane: {:?}", answers.limits());
        };
        assert_eq!(*lane, EntailmentMechanism::Refutation);
        assert!(
            constructs
                .iter()
                .any(|why| why.contains("differentFrom") || why.contains("witness")),
            "{constructs:?}"
        );
    }

    /// A VARIABLE INSIDE AN RDF 1.2 TRIPLE TERM IS THE SAME VARIABLE AS ONE OUTSIDE IT.
    ///
    /// [`QNode::Triple`] exists so that a name used above and below a triple-term boundary
    /// is ONE variable, joined by the match. Asserted semantically, over two premises that
    /// differ only in whether the join holds: a construction that split the name into two
    /// variables returns the same row for both, which is an EXTRA row — a substitution that
    /// does not satisfy the pattern — and no limit line makes such a row an answer.
    #[test]
    fn a_name_used_inside_and_outside_a_triple_term_is_one_variable() {
        let premise = |quoted_subject: &str| {
            let mut b = RdfDatasetBuilder::new();
            let a = b.intern_iri("http://example.org/a");
            let p = b.intern_iri("http://example.org/p");
            let q = b.intern_iri("http://example.org/q");
            let r = b.intern_iri("http://example.org/r");
            let subject = b.intern_iri(quoted_subject);
            let quoted = b.intern_triple(subject, q, r);
            b.push_quad(a, p, quoted, None);
            b.freeze().expect("freeze")
        };
        // `?x <p> <<( ?x <q> <r> )>>`
        let bgp = [QTriple {
            s: QNode::Var("x".to_owned()),
            p: QNode::Term(TermValue::iri("http://example.org/p")),
            o: QNode::Triple {
                s: Box::new(QNode::Var("x".to_owned())),
                p: Box::new(QNode::Term(TermValue::iri("http://example.org/q"))),
                o: Box::new(QNode::Term(TermValue::iri("http://example.org/r"))),
            },
        }];

        // The quoted subject is somebody ELSE, so no substitution satisfies both
        // occurrences.
        let answers = certain_answers(
            &premise("http://example.org/b"),
            &bgp,
            Regime::Simple,
            &ImportMap::new(),
        )
        .expect("consistent");
        assert_eq!(answers.vars(), ["x"]);
        assert!(
            answers.rows().is_empty(),
            "`?x` cannot be <a> and <b> at once: {:?}",
            answers.rows()
        );

        // …and the same pattern over a premise that DOES satisfy the join answers it.
        let answers = certain_answers(
            &premise("http://example.org/a"),
            &bgp,
            Regime::Simple,
            &ImportMap::new(),
        )
        .expect("consistent");
        assert_eq!(
            answers.rows(),
            [vec![TermValue::iri("http://example.org/a")]]
        );
    }

    /// …AND AN ORDINARY ASSERTIONAL PATTERN IS STILL EXHAUSTIVE.
    ///
    /// The cheap way to pass the test above is to make every answer incomplete, which would
    /// destroy the service rather than fix it. These are the patterns no lane reads anything
    /// in — so nothing was left untested, the rule table's own conditions all hold, and
    /// `is_complete` is a claim this service is entitled to make.
    #[test]
    fn a_pattern_no_lane_reads_is_still_exhaustive() {
        for (premise, bgp) in [
            (
                subclass_premise(),
                vec![QTriple {
                    s: QNode::Term(TermValue::iri("http://example.org/x")),
                    p: QNode::Term(TermValue::iri(TYPE)),
                    o: QNode::Var("c".to_owned()),
                }],
            ),
            (
                subclass_premise(),
                vec![QTriple {
                    s: QNode::Var("s".to_owned()),
                    p: QNode::Term(TermValue::iri(TYPE)),
                    o: QNode::Term(TermValue::iri("http://example.org/B")),
                }],
            ),
            (
                mixed(&default_disjoint()),
                vec![QTriple {
                    s: QNode::Var("s".to_owned()),
                    p: QNode::Term(TermValue::iri(TYPE)),
                    o: QNode::Var("o".to_owned()),
                }],
            ),
        ] {
            let answers = certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new())
                .expect("consistent");
            assert!(
                answers.is_complete(),
                "nothing beyond the table was needed: {:?}",
                answers.limits()
            );
            assert!(!answers.rows().is_empty());
            assert_eq!(answers.mechanism(), EntailmentMechanism::StrictTable);
        }
    }

    /// AN OPEN PREDICATE IS A LIMIT, AND HERE IS THE ANSWER IT MISSES.
    ///
    /// `?s ?p ?o` reads every triple of the closure, and reporting that as the whole relation
    /// is a claim about a question the closure does not answer. `p ∘ p ⊑ p` entails `p rdf:type
    /// owl:TransitiveProperty` — the freeze lane proves it, right below — and no head of Tables
    /// 4–9 concludes a property CHARACTERISTIC, so the row is absent from an answer that used
    /// to render no `limit` line at all. It names the POSITION that costs it rather than a
    /// lane, because every lane and every schema predicate is inside what a `?p` ranges over —
    /// including the schema predicates Table 9's `scm-*` rules DO conclude, for which the row
    /// may be present and the enumeration still not exhaustive, since Theorem PR1 claims no
    /// completeness for a schema conclusion whether or not some rule derives one.
    #[test]
    fn an_open_predicate_is_a_limit_naming_the_position() {
        let premise = three_lane_premise();
        let bgp = [QTriple {
            s: QNode::Var("s".to_owned()),
            p: QNode::Var("p".to_owned()),
            o: QNode::Var("o".to_owned()),
        }];
        let answers =
            certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new()).expect("consistent");
        assert!(!answers.rows().is_empty(), "the closure is enumerated");
        assert_eq!(answers.mechanism(), EntailmentMechanism::StrictTable);

        // The certain answer the rule table's closure does not hold…
        let missed = vec![
            TermValue::iri(P),
            TermValue::iri(TYPE),
            TermValue::iri(TRANSITIVE),
        ];
        assert!(
            !answers.rows().contains(&missed),
            "no head of Tables 4-9 concludes a property characteristic"
        );
        // …is entailed, by a mechanism this service did not run.
        assert!(matches!(
            outcome(&premise, &graph(&[(P, TYPE, TRANSITIVE)]), Regime::OwlRl),
            EntailmentOutcome::Entailed(_)
        ));
        // So the row set is NOT exhaustive, and it says so naming the open position.
        assert!(!answers.is_complete());
        let open: Vec<&Vec<String>> = answers
            .limits()
            .iter()
            .filter_map(|limit| match limit {
                UndecidedReason::OpenPredicate(triples) => Some(triples),
                _ => None,
            })
            .collect();
        assert_eq!(
            open,
            [&vec!["?s ?p ?o".to_owned()]],
            "{:?}",
            answers.limits()
        );
    }

    /// AN OPEN PREDICATE COSTS THE OTHER TRIPLES NOTHING.
    ///
    /// The lane survey is stated over an RDF graph and a `?p` has no term a graph can hold, so
    /// the whole question used to lose the survey the moment one triple opened its predicate.
    /// Only the open triples are withheld from it: the refutation lane still names itself for
    /// the triple that is its own, beside the open-predicate limit.
    #[test]
    fn an_open_predicate_does_not_silence_the_survey_of_its_neighbours() {
        let premise = three_lane_premise();
        let bgp = [
            QTriple {
                s: QNode::Var("s".to_owned()),
                p: QNode::Var("p".to_owned()),
                o: QNode::Var("o".to_owned()),
            },
            QTriple {
                s: QNode::Var("x".to_owned()),
                p: QNode::Term(TermValue::iri(DIFFERENTFROM)),
                o: QNode::Term(TermValue::iri(STEWIE)),
            },
        ];
        let answers =
            certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new()).expect("consistent");
        assert!(
            answers
                .limits()
                .iter()
                .any(|limit| limit.mechanism() == EntailmentMechanism::Refutation),
            "the neighbour keeps its own lane's limit: {:?}",
            answers.limits()
        );
        assert!(
            answers
                .limits()
                .iter()
                .any(|limit| matches!(limit, UndecidedReason::OpenPredicate(_))),
            "{:?}",
            answers.limits()
        );
    }

    /// A NON-DISTINGUISHED PREDICATE IS OPEN IN THE SAME SENSE, AND BOTH PREDICATES SAY SO.
    ///
    /// `open_predicates` reports BOTH kinds of predicate variable, because the position and
    /// not the projection is what the rule table's whitelists cannot range over. The survey
    /// filter has to agree: a non-distinguished predicate is a blank node, RDF says a
    /// predicate is an IRI, and `RdfDatasetBuilder::freeze` refuses one — so a survey graph
    /// built with it inside is no graph at all and the WHOLE question loses its survey,
    /// which is the silence the filter exists to prevent.
    ///
    /// No host can reach this: the N-Triples parser refuses a blank predicate, so the pattern
    /// below can only be built by a Rust caller of this crate — which is why the assertion
    /// lives here and not in a boundary test.
    #[test]
    fn a_non_distinguished_predicate_is_withheld_from_the_survey_like_a_projected_one() {
        let premise = three_lane_premise();
        let bgp = [
            QTriple {
                s: QNode::Var("s".to_owned()),
                p: QNode::Term(TermValue::blank("np")),
                o: QNode::Var("o".to_owned()),
            },
            QTriple {
                s: QNode::Var("x".to_owned()),
                p: QNode::Term(TermValue::iri(DIFFERENTFROM)),
                o: QNode::Term(TermValue::iri(STEWIE)),
            },
        ];
        let answers =
            certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new()).expect("consistent");
        // The blank predicate is reported as open, naming the triple the caller wrote…
        let open: Vec<&Vec<String>> = answers
            .limits()
            .iter()
            .filter_map(|limit| match limit {
                UndecidedReason::OpenPredicate(triples) => Some(triples),
                _ => None,
            })
            .collect();
        assert_eq!(
            open,
            [&vec!["?s _:np#0 ?o".to_owned()]],
            "{:?}",
            answers.limits()
        );
        // …and it is withheld from the survey graph, so the neighbour still gets surveyed.
        assert!(
            answers
                .limits()
                .iter()
                .any(|limit| limit.mechanism() == EntailmentMechanism::Refutation),
            "the neighbour keeps its own lane's limit: {:?}",
            answers.limits()
        );
    }

    /// A VERDICT IS DECIDED AGAINST THE WHOLE PATTERN, NEVER THE SURVEY'S FILTERED COPY.
    ///
    /// A pattern with nothing to project IS a conclusion graph, and the survey filter above
    /// withholds triples from a SURVEY. Handing the filtered copy to the fold instead would
    /// drop a conjunct, and dropping a conjunct only ever makes an entailment easier — so the
    /// question below must not answer `yes` on the strength of its SECOND triple alone.
    ///
    /// `Peter` is a term the premise never mentions, so no triple of the closure has it in
    /// both end positions and the first conjunct holds under no substitution for `_:np`.
    /// `Stewie : Boy` is asserted, so the second conjunct is entailed on its own — which is
    /// exactly what a truncated question would report as the answer to the conjunction.
    #[test]
    fn a_withheld_predicate_never_shrinks_the_question_a_verdict_is_decided_against() {
        let premise = three_lane_premise();
        let bgp = [
            QTriple {
                s: QNode::Term(TermValue::iri(PETER)),
                p: QNode::Term(TermValue::blank("np")),
                o: QNode::Term(TermValue::iri(PETER)),
            },
            QTriple {
                s: QNode::Term(TermValue::iri(STEWIE)),
                p: QNode::Term(TermValue::iri(TYPE)),
                o: QNode::Term(TermValue::iri(BOY)),
            },
        ];
        let answers =
            certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new()).expect("consistent");
        assert!(answers.vars().is_empty(), "nothing is projected");
        assert!(
            answers.rows().is_empty(),
            "the first conjunct holds under no substitution, so the conjunction does not \
             either: {:?}",
            answers.rows()
        );
        // The second conjunct on its own IS entailed, which is what makes the assertion above
        // a statement about the question's SHAPE rather than about this premise.
        assert!(matches!(
            outcome(&premise, &graph(&[(STEWIE, TYPE, BOY)]), Regime::OwlRl),
            EntailmentOutcome::Entailed(_)
        ));
    }

    /// Every lane that would have been needed names itself, not only the refutation one.
    #[test]
    fn each_unreachable_lane_names_itself_in_a_limit() {
        let ranges = mixed(&[(P, TYPE, DATATYPE_PROPERTY), (P, RANGE, XSD_BYTE)]);
        let cases: [(&str, Arc<RdfDataset>, Vec<QTriple>, EntailmentMechanism); 4] = [
            (
                "which properties is the premise entailed to make transitive?",
                three_lane_premise(),
                vec![QTriple {
                    s: QNode::Var("p".to_owned()),
                    p: QNode::Term(TermValue::iri(TYPE)),
                    o: QNode::Term(TermValue::iri(TRANSITIVE)),
                }],
                EntailmentMechanism::Freeze,
            ),
            (
                "which anonymous unions does the premise license?",
                mixed(&[(A, TYPE, OWL_CLASS)]),
                vec![
                    QTriple {
                        s: QNode::Var("c".to_owned()),
                        p: QNode::Term(TermValue::iri(TYPE)),
                        o: QNode::Term(TermValue::iri(OWL_CLASS)),
                    },
                    QTriple {
                        s: QNode::Var("c".to_owned()),
                        p: QNode::Term(TermValue::iri(UNIONOF)),
                        o: QNode::Term(TermValue::blank("l")),
                    },
                    QTriple {
                        s: QNode::Term(TermValue::blank("l")),
                        p: QNode::Term(TermValue::iri(FIRST)),
                        o: QNode::Term(TermValue::iri(A)),
                    },
                    QTriple {
                        s: QNode::Term(TermValue::blank("l")),
                        p: QNode::Term(TermValue::iri(REST)),
                        o: QNode::Term(TermValue::iri(NIL)),
                    },
                ],
                EntailmentMechanism::Comprehension,
            ),
            (
                "which terms `knows` themselves?",
                three_lane_premise(),
                vec![QTriple {
                    s: QNode::Var("x".to_owned()),
                    p: QNode::Term(TermValue::iri(KNOWS)),
                    o: QNode::Var("x".to_owned()),
                }],
                EntailmentMechanism::Reflexivity,
            ),
            (
                "what is `p` declared as, GIVEN that its range widens to xsd:short?",
                ranges,
                vec![
                    QTriple {
                        s: QNode::Term(TermValue::iri(P)),
                        p: QNode::Term(TermValue::iri(RANGE)),
                        o: QNode::Term(TermValue::iri(XSD_SHORT)),
                    },
                    QTriple {
                        s: QNode::Term(TermValue::iri(P)),
                        p: QNode::Term(TermValue::iri(TYPE)),
                        o: QNode::Var("t".to_owned()),
                    },
                ],
                EntailmentMechanism::DataRange,
            ),
        ];
        for (name, premise, bgp, lane) in cases {
            let answers = certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new())
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(
                answers
                    .limits()
                    .iter()
                    .any(|limit| limit.mechanism() == lane),
                "{name}: {:?}",
                answers.limits()
            );
            assert!(!answers.is_complete(), "{name}");
        }
    }
}
