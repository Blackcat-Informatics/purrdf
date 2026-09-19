// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The compiler: admitted plan in, independently executable SPARQL out.
//!
//! [`compile`] is the narrow admission waist. It runs every semantic check in
//! [`admission`](crate::admission) and, when the plan is admitted, emits one
//! SPARQL `SELECT` per stratum. Each unit names its stratum's producers by their
//! registered IRIs and is executable through `purrdf-sparql-eval` with no
//! composition layer in the path — the text is the whole executable content, so
//! nothing can live between planning and execution.
//!
//! # Why text, not an algebra node
//!
//! A caller can take an emitted unit and hand it to the evaluator directly; the
//! conformance corpus already insists the evaluator is the single source of query
//! truth, and a text seam lets a caller stand on that same surface without
//! adopting this crate's types. An AST carried between `compile` and `execute`
//! would create a second, non-textual artifact in exactly the gap the design
//! closes.
//!
//! # What a unit contains
//!
//! For each stratum the plan declares, the unit is a `SELECT ?candidate` over the
//! one branch of that stratum's one producer. The branch is a sub-`SELECT` that
//! projects the producer's own declared candidate position under the common name
//! `?candidate`, so strata whose producers name their candidate in different
//! argument positions still read back through one column.
//!
//! # One stratum, one branch — there is no union to emit
//!
//! A stratum carries exactly one producer, refused at registration by
//! [`register_ranked`](purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked)
//! and again at this waist for an edited plan. So there is never a second branch
//! to compose with, and the `UNION` this emitter once wrote could only ever have
//! run against a configuration the registry now refuses to build.
//!
//! It was also never a *merge*. A `UNION` concatenates: the second producer's
//! rank-1 row surfaced at stratum rank `n+1`, below the whole of the first
//! producer's output, and decayed as though it had lost to rows it never competed
//! with; a candidate both producers named arrived twice in one stratum's stream,
//! which an honest `Unique` declaration says cannot happen; and a first producer
//! that filled the depth left the second contributing nothing while the trailer
//! reported a clean exhaustion. Ranks are comparable only within the list that
//! assigned them, which is exactly why the merge belongs either inside one
//! producer (same scoring law, no weight between the two) or across strata in the
//! fusion sum (different scoring laws — or one law whose bounded score cannot
//! carry the weight the host means between two classes), and never in a
//! concatenation here.
//!
//! # The request is in the text
//!
//! Every argument position a producer's declaration places a request facet into
//! carries that facet as a **constant**; only the positions nothing was placed
//! into are free `?cN` variables. The placement rule is
//! [`matching::place`](crate::matching::place) — the same function the planner
//! ran, so admission re-derives the planner's decision rather than a second
//! approximation of it.
//!
//! So `search("quick brown fox")` and `search("")` compile to different queries
//! wherever a producer declared a placement for the needle, which is the whole
//! point of compiling a request rather than an arity. Where a producer declared
//! **none** — a declaration that accepts the shape and writes no part of it —
//! the two compile to the same query, because that producer asked to be called
//! with the request absent from its arguments. That is not a silent loss: such a
//! term is never bound to the producer, and the plan names it in
//! [`Plan::unserved_terms`] as
//! [`UnservedReason::AcceptedWithoutPlacement`](crate::UnservedReason::AcceptedWithoutPlacement).
//! An empty unserved list therefore does mean the text carries every term.
//!
//! # The subject argument list is always parenthesized
//!
//! Even at subject arity one, the subject side is written `( ?c0 )`. A bare
//! subject would make the call's parse depend on the grammar tolerating whatever
//! term landed there (a literal subject, say), whereas a parenthesized group
//! followed by a registered property-function IRI is routed unambiguously to the
//! argument-list production.
//!
//! # The branch carries the stratum's depth, and so does the unit
//!
//! A branch whose producer takes no depth argument is bounded by its own
//! `LIMIT <depth + 1>`, and the unit carries the same bound on the outside. Both are
//! rendered from the depth by [`StratumUnit::sparql`] rather than written into a text
//! anything else holds, for the reason a section below gives. The inner
//! one is the producer's licence to stop early — the evaluator offers it to the
//! relation as a row ceiling — and the outer one is the stratum's contract with
//! fusion, which holds whatever bounds the branch below it carries. A producer
//! that declares a [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement) already
//! received the depth as an argument and bounds itself, so its branch carries no
//! `LIMIT`; the outer one still applies.
//!
//! No admitted plan carries a depth of zero, so nothing here emits `LIMIT 0`.
//! That bound reads no rows: whatever the relation holds, the unit hands back
//! nothing, and the stratum is then reported exhausted having emitted nothing —
//! the strongest completeness claim this layer makes, made about the bound rather
//! than about the data, and indistinguishable in every trailer field from an
//! honest empty answer. A bound may narrow a read and
//! must never eliminate one, so the planner floors every derived depth at one,
//! admission refuses a zero outright ([`AdmissionError::ZeroDepth`]), and
//! [`emitted_limit`] reads a declared row count of zero rather than obeying it —
//! the emitted bound is one row past the floored depth there as anywhere else.
//! Emptiness is reported by the producer, in the
//! receipt fusion verifies against the rows it actually pulled, and a stratum
//! that is to run at all runs deep enough to ask.
//!
//! A stratum that is to read nothing is expressed by carrying no depth entry —
//! which is also how the planner expresses it, since it records a depth only for
//! a stratum a surviving producer ranks under. Absence emits no unit and claims
//! nothing; a zero would have claimed everything.
//!
//! # The depth arrives already narrowed, and this stage narrows nothing
//!
//! A request states how much of the answer it is for
//! ([`ReadBound`](crate::ReadBound)), and the *planner* is what turns that into a
//! depth — see [`plan`](crate::plan) for the rule and its proof. So by the time a
//! plan reaches this waist the narrowing has happened, and
//! [`Plan::stratum_depths`] is the true read.
//!
//! This stage deliberately does not narrow a `LIMIT` on its own. Emitting five
//! rows for a depth the plan recorded as four hundred would make that recorded
//! depth a fiction: every field keyed to it — [`StratumUnit::depth()`],
//! [`PlannedResolution::requested_depth`], the executor's reading of the probe row
//! — would describe a read nobody took, and the identity a plan carries would
//! not distinguish the two reads at all. The bound belongs where the depth is
//! decided.
//!
//! What this stage does add is [`CompiledRetrieval::fused_bound`]: the request's
//! bound resolved once, against the plan in hand, into the row count a fusion of
//! these units must be run at. It travels with the streams so that fusing them at
//! some other bound is refused by name rather than served out of depths derived
//! for another question.
//!
//! # The emitted bound is one row deeper than the depth, and that row is a probe
//!
//! A unit bounded at exactly its depth cannot tell the two endings apart that
//! its consumer most needs to distinguish. A producer holding nine rows read at
//! depth three and a producer holding exactly three rows read at depth three
//! both hand back three rows and an empty cursor, so an executor that only ever
//! saw `depth` rows had to guess — and the guess it used to make was
//! [`ProducerStatus::Exhausted`](crate::ProducerStatus), which is the strongest
//! completeness claim this layer can utter, minted for a read the plan itself
//! cut short. That is the zero-depth fault one size larger: a bound on the read
//! silently becoming a statement about the answer.
//!
//! So the emitted bound is `depth + 1`, on the branch and on the unit. The extra
//! row is a **read, never a value**: [`execute`](crate::execute) emits at most
//! `depth` rows onto the stream and uses the arrival of the `depth + 1`-th only to
//! end the stream [`DepthReached`](crate::ProducerReceipt::DepthReached) instead of
//! `Exhausted`. No plan field, no identity and no recorded resolution moves by
//! one: [`PlannedResolution::requested_depth`] is the depth, and so is
//! [`StratumUnit::depth()`].
//!
//! # Every bound is *rendered* from the depth, and a caller's text is never certified
//!
//! The paragraph above is a claim about two numbers, so the question is what holds
//! them there. A [`StratumUnit`] used to carry the whole query text — bounds included
//! — as a writable field, and a bound in that text was a fourth input to the ending
//! nobody compared against the other three. The natural off-by-one was enough: a
//! caller assembling a bundle by hand writes `LIMIT <depth>`, because the depth is the
//! number this layer reasons about everywhere, and the read then returned `depth` rows
//! with no slot for the probe. `execute` writes `DepthReached` only when a row arrives
//! *past* the depth, so that read was reported `Exhausted` — the strongest
//! completeness claim this layer has — for a text that had cut it. A depth of three
//! over nine real rows reported `Exhausted { rows_emitted: 3 }`, and the trailer above
//! it read `Exact`.
//!
//! Pulling the *outer* bound out of that string and rendering it did not close this.
//! It moved the writable bound inward: the branch's own `LIMIT` was still inside the
//! text, and so was the rendered depth argument of a self-bounding producer. Where an
//! inner bound and an outer one disagree the **inner** one decides the read, so
//! lowering the branch's `LIMIT` from four to three — or lowering the depth argument
//! — produced `Exhausted { rows_emitted: 3 }` over nine rows again, with the same
//! numbers as the original fault. A bound anywhere in caller-writable text is the
//! same defect wherever in the text it sits.
//!
//! So no number the ending depends on is held as text at all. A compiled unit carries
//! its query as the *parts* it was assembled from — the producer's IRI, the argument
//! slots the request placed, the positions the candidate and block columns are read
//! from — and [`StratumUnit::sparql`] renders the whole query, both bounds included,
//! from [`StratumUnit::depth()`] on every read. The branch's row ceiling is
//! [`emitted_limit`]'s and the depth argument is [`depth_argument`]'s, the same two
//! functions that always computed them; what changed is that there is no longer a
//! string between them and the reader.
//!
//! Driving the executor over a query of a caller's own is still open, through
//! [`StratumUnit::new`], and it is a *different* kind of unit: the text is carried
//! verbatim, this layer bounds only its outside, and what bounds the caller wrote
//! inside it cannot be seen from here. Such a read therefore never ends
//! `Exhausted` — the ending names the caller's text as the stopper instead
//! ([`StreamEnding::SuppliedQueryEnded`](crate::StreamEnding)). That is not a refusal
//! of the seam and not a weaker check: it is the same rule the rest of this
//! vocabulary follows, which is that a completeness claim is only ever made about a
//! read this layer bounded.
//!
//! Validating a trailing `LIMIT` by inspecting the text would have been the weaker fix
//! three times over: it re-reads a bound the layer already knows, it can only ever
//! refuse a caller for writing the number this layer writes itself, and it sees
//! nothing at all of the bound one line further in.
//!
//! # The declared row bound does not cap the emitted bound, at any size
//!
//! `min(depth, declared) + 1` and `depth + 1` are the same number for every unit
//! this stage can emit, and writing the `min` anyway would be a guard that fires
//! for no reachable plan. Admission refuses a recorded depth above the registry's
//! declared row bound ([`AdmissionError::DepthBoundViolation`]), so `depth` is at or
//! below `declared` by the time anything is emitted and the `min` selects `depth`.
//! The `min`'s one interesting case was worse than useless: written *inside* the
//! probe — `min(depth + 1, declared)` — it erased the slot at exactly the depth that
//! sits on the declaration, and the read was then reported `Exhausted` for a bound
//! that cut it.
//!
//! A declared **zero** is the one arm where the two numbers ever parted, and it is
//! the arm the `min` got wrong. `min(depth, 0) + 1` is one, which at the planner's
//! floored depth of one is a bound *equal* to the depth: no row past the depth could
//! arrive, so the read was certified `Exhausted` whatever the index turned out to
//! hold — the [`ProbedDepth`] state the waist exists to make unwritable, reached
//! through a second door. A zero declaration is read rather than obeyed everywhere
//! else in this layer (the planner floors the depth at one, admission admits that
//! floor), so it does not cap the emitted bound either: the bound is `depth + 1`
//! there too. An empty index then reports `Exhausted { rows_emitted: 0 }` as a
//! *verified* claim, and an index that turns out to hold rows breaches its
//! declaration by name, exactly as a wrong declaration of any other size does.
//!
//! # Every depth that reaches here has room for its probe row, because the rest
//! are refused
//!
//! "The slot exists at every depth" is a claim about the depths this stage can be
//! handed, and there is exactly one it would be false for: `u32::MAX`, where the
//! row past the depth is not a number a `LIMIT` this emitter writes can hold. A
//! saturating `+ 1` there emitted a bound equal to the depth — the probe erased,
//! no row able to arrive past the depth, and therefore `Exhausted` reported for a
//! read the bound may have cut. Saturation looked like arithmetic hygiene and was
//! the same silent completeness claim as `LIMIT 0`, at the other end of the range.
//!
//! So it is refused rather than saturated, and refused at the waist where every
//! other depth invariant lives ([`AdmissionError::DepthWithoutProbe`], the mirror
//! of [`AdmissionError::ZeroDepth`]). The planner does not derive such a depth
//! either: it records a derived bound past that ceiling *at* the ceiling, where the
//! probe row still fits and the ending is still observable. What reaches
//! [`emitted_limit`] is a [`ProbedDepth`] — a depth the waist has already proved can
//! carry its probe — so the probe row is added with exact arithmetic and the emitted
//! `LIMIT` is never equal to the depth it bounds.
//!
//! The probe slot is not an over-refusal either, because it costs nothing when
//! the declaration is honest. A producer that declared it can yield `n` rows per
//! invocation and can really only yield `n` returns `n` rows into an `n + 1` row
//! bound, the slot comes back empty, and `Exhausted` is *verified* rather than
//! believed. A row arriving in it means the producer yielded an `n + 1`-th row
//! after promising there is none, and [`execute`](crate::execute) refuses that
//! run by name ([`ExecutionError::RowBoundBreached`](crate::ExecutionError))
//! rather than certifying the truncation as completeness.
//!
//! A registry that declared no row count at all promised nothing, so there is no
//! declaration for a row to breach: the probe is the only way to learn the
//! ending, and its arrival is the ordinary `DepthReached`.
//!
//! # The depth *argument* is bounded differently, because it is a request
//!
//! A producer that declares a [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement)
//! is handed the number rather than bounded by it, and the two are not the same
//! kind of thing. A `LIMIT` is applied by the evaluator to a cursor the producer
//! never hears about, so asking for one row more than the registry declared
//! costs nothing and tells the truth about what came back. An argument is a
//! *request*, and a request for `declared + 1` rows asks the producer to exceed
//! the declaration it registered — which a well-built producer refuses, because
//! serving it would be a short answer returned as a complete one. The
//! nearest-neighbour relation refuses exactly that, by name, against its own
//! configured guard.
//!
//! So the argument carries `max(1, min(depth + 1, declared row bound))` — the
//! probe where the declaration leaves room for it, and the declaration itself
//! where it does not. This is not the silent cap the `LIMIT` used to have.
//! The unit's own `LIMIT` still sits at `depth + 1` around such a branch, so a
//! self-bounding producer that returns more rows than it declared is still
//! caught and still refused ([`ExecutionError::RowBoundBreached`](crate::ExecutionError));
//! what is no longer done is asking it to.
//!
//! Where the two differ — a depth already at the declaration — the read cannot
//! reach past the depth at all, and that has a consequence the layer states rather
//! than papers over. Such a producer returns at most `depth` rows, so the slot past
//! the depth can never be filled, so **how the read ended is not observable**:
//! `depth` rows came back and nothing in the answer says whether a `depth + 1`-th
//! existed. `Exhausted` would be a completeness claim minted from the declaration,
//! and `DepthReached` would blame a planned depth that did not cut anything, so the
//! ending is neither — it is
//! [`StreamEnding::RowBoundReached`](crate::StreamEnding), which names the stopper
//! the read actually had: the producer's own declared bound.
//!
//! Under-declaring is *not* loud for this class of producer, and saying otherwise
//! would be the comfortable falsehood here. Under-declaring is caught elsewhere by
//! the probe row, and this is the one shape that never receives one. The depth
//! cannot rise above the declaration to go looking either — `capped` bounds every
//! derived depth by the declared row count, so no planner-written plan asks for
//! more, and raising the *argument* past the declaration is precisely the request a
//! conforming relation must refuse. What the layer can do honestly is report that it
//! read to the producer's bound and no further, and it does.
//!
//! Which of the two cases a unit was emitted for is not recorded beside its query: it
//! is read *off* that query, by [`read_reach`], because "the producer was handed the
//! depth as an argument" and "the branch carries no `LIMIT` of its own" are one fact
//! written once. What [`StratumUnit`] carries is the answer — how far its read could
//! reach — so [`execute`](crate::execute) reads the ending off the unit in hand rather
//! than re-deriving it from a registry three stages away.

use std::collections::BTreeMap;

use purrdf_sparql_eval::{PfDescriptor, RankedDeclaration, RegistryId};

use crate::admission::{
    AdmissionEnvironment, AdmissionError, MAX_READ_DEPTH, ProbedDepth, RowBound, Unprobeable,
    admit_plan,
};
use crate::execute::ExecutionError;
use crate::fuse::TopK;
use crate::id::PlanId;
use crate::iri::Iri;
use crate::matching::{Invocation, UnitArgument, render_slots};
use crate::plan::{Plan, ProducerBinding};
use crate::ranked_stream::StreamContract;
use crate::reciprocal_rank::MonotoneDepth;
use crate::render::{self, RenderError};
use crate::request::ReadBound;

/// One unit's query, and who wrote it.
///
/// The distinction is not bookkeeping: it decides what the layer may claim about
/// how that unit's read ended. A query this stage rendered carries bounds this
/// stage computed from a proved depth, so the arrival of the probe row is an
/// observation. A query a caller supplied is text this layer cannot see into — a
/// `LIMIT` inside a sub-`SELECT`, a `FILTER`, a pattern that simply matches less —
/// so how that read ended is not this layer's to certify. See this module's
/// header.
#[derive(Clone, Debug, PartialEq, Eq)]
enum UnitQuery {
    /// The query [`compile`] rendered, held as the parts it was assembled from so
    /// that every bound in it is arithmetic over the unit's own depth, performed
    /// when the text is asked for.
    Rendered(RenderedQuery),
    /// A query text a caller supplied, carried verbatim and bounded by nothing this
    /// layer wrote.
    Supplied(String),
}

/// One stratum's rendered query, as the parts a bound is written between.
///
/// Every field here is depth-independent: the producer's IRI, the argument slots
/// the request placed, the positions the candidate and block columns are read
/// from. The two numbers that are *not* depth-independent — the branch's row
/// ceiling and a self-bounding producer's depth argument — are deliberately absent,
/// and [`Self::text`] renders both from the depth the unit holds. That is the whole
/// reason this is a structure rather than a string: a string would have to spell
/// them, and a bound spelled beside the depth is a bound that can disagree with it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct RenderedQuery {
    /// The registered producer IRI the branch calls.
    producer: String,
    /// The subject-side argument list, in declared position order.
    ///
    /// The subject and object sides are held apart rather than as one list with an
    /// arity to split it at, because the split is decided once at emission from the
    /// descriptor's own arities; a single list would make every read of this value
    /// re-derive a partition that cannot change.
    subject: Vec<UnitArgument>,
    /// The object-side argument list, in declared position order.
    object: Vec<UnitArgument>,
    /// The argument position the candidate column is projected from.
    candidate: usize,
    /// The argument position each row's block is projected from, for a producer
    /// whose declaration names one; `None` for a producer that names no block per
    /// row, which renders the text this compiler emitted before blocks existed.
    block: Option<usize>,
}

impl RenderedQuery {
    /// Every argument position, subject side first, in declared position order.
    fn arguments(&self) -> impl Iterator<Item = &UnitArgument> {
        self.subject.iter().chain(self.object.iter())
    }

    /// Whether the producer called by the graph pattern this unit emits bounds
    /// *itself*, which is true of exactly the producer that declared a depth
    /// placement and received the depth as an argument.
    ///
    /// Read off the arguments rather than recorded beside them. "The producer takes
    /// the depth as an argument", "the branch carries no `LIMIT` of its own" and
    /// "the ending may be unobservable at a depth on the declaration" are three
    /// readings of one fact, and a field for it would be a second place for that
    /// fact to be written.
    fn bounds_itself(&self) -> bool {
        self.arguments().any(UnitArgument::is_depth)
    }

    /// The whole text this query runs at `depth` over a producer the registry bounds
    /// at `declared_rows`.
    ///
    /// Both bounds are rendered here, from that one depth: the branch's own row
    /// ceiling (or, for a self-bounding producer, the depth argument in its call)
    /// and the unit's outer bound. Neither is stored, so neither can be replaced.
    fn text(&self, depth: ProbedDepth, declared_rows: Option<u64>) -> String {
        let render = |argument: &UnitArgument| match argument {
            UnitArgument::Placed(text) => text.clone(),
            UnitArgument::Depth { datatype } => {
                render::typed_literal(&depth_argument(depth, declared_rows).to_string(), datatype)
            }
        };
        let subject_text = self
            .subject
            .iter()
            .map(render)
            .collect::<Vec<_>>()
            .join(" ");
        let object_text = self.object.iter().map(render).collect::<Vec<_>>().join(" ");
        let candidate = self.candidate;
        // One projection per position the unit reads back, in the order the executor
        // reads them: the candidate, then the block where the producer declared one.
        let (projected, projections) = match self.block {
            None => (
                format!("?{CANDIDATE_NAME}"),
                format!("(?c{candidate} AS ?{CANDIDATE_NAME})"),
            ),
            Some(block) => (
                format!("?{CANDIDATE_NAME} ?{BLOCK_NAME}"),
                format!("(?c{candidate} AS ?{CANDIDATE_NAME}) (?c{block} AS ?{BLOCK_NAME})"),
            ),
        };
        // A producer that took the depth as an argument bounds itself, with the
        // number rendered into that argument above — which carries the probe row
        // wherever its own declaration leaves room for it. One that did not is
        // bounded here, by the branch's own `LIMIT`, which carries the probe row
        // always. Per this module's header those are two different promises and are
        // deliberately not one number.
        let ceiling = if self.bounds_itself() {
            String::new()
        } else {
            format!(" LIMIT {}", emitted_limit(depth))
        };
        format!(
            "SELECT {projected} WHERE {{\n  {{ SELECT {projections} WHERE {{ ( {subject_text} ) \
             <{}> ( {object_text} ) }}{ceiling} }}\n}}\nLIMIT {}",
            self.producer,
            emitted_limit(depth)
        )
    }
}

/// How far a unit's read can reach, relative to the depth the unit records.
///
/// Derived once, from the depth, the registry's declared row bound and the query
/// itself, and never supplied: two spellings of "could the probe row have arrived"
/// are two chances to disagree about the one question the ending hangs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadReach {
    /// One row past the depth. The probe slot exists, so a read the depth cut is
    /// distinguishable from a read that ran out.
    PastDepth,
    /// Exactly the depth. The producer was handed its own declared bound as the
    /// depth argument and cannot be asked for a row past it, so a read that fills
    /// the depth has an ending nobody can observe.
    AtDepth,
    /// Not known. The query text is a caller's, so what bound the read was taken
    /// under is not a fact this layer holds: the probe row may have been cut by
    /// something inside that text, and its absence is therefore no evidence at all.
    Unknown,
}

/// Whether a unit built from these three facts can reach one row past its depth.
///
/// A caller's text answers nothing, so it reaches [`ReadReach::Unknown`] — and that
/// arm comes first because it is about the text rather than about the numbers beside
/// it.
///
/// For a rendered query the answer is arithmetic. The bound on a branch the
/// *evaluator* bounds is `depth + 1`, rendered from the depth by
/// [`RenderedQuery::text`], so such a read always reaches past the depth. A read the
/// *producer* bounds reaches as far as [`depth_argument`] asks for, which is
/// `max(1, min(depth + 1, declared))`: past the depth wherever the declaration
/// leaves room, and exactly the depth where it does not. This is that comparison,
/// spelled once, and it is the same arithmetic [`depth_argument`] performs rather
/// than a second opinion about it.
fn read_reach(depth: u32, declared_rows: Option<u64>, query: &UnitQuery) -> ReadReach {
    let rendered = match query {
        UnitQuery::Supplied(_) => return ReadReach::Unknown,
        UnitQuery::Rendered(rendered) => rendered,
    };
    match declared_rows {
        // Either the evaluator applies the bound — always one row past the depth —
        // or nothing was declared for the producer's own request to be capped by, so
        // it asked for the probe outright.
        _ if !rendered.bounds_itself() => ReadReach::PastDepth,
        None => ReadReach::PastDepth,
        // `max(1)` is the floor `depth_argument` applies: a declared zero is read
        // rather than obeyed, and the one row it is read for is the floored depth
        // itself, so the argument lands *on* the depth there too.
        Some(declared) if declared.max(1) <= u64::from(depth) => ReadReach::AtDepth,
        Some(_) => ReadReach::PastDepth,
    }
}

/// A set of facts no unit can describe a read with.
///
/// Raised by [`StratumUnit::new`] and by nothing else: a unit [`compile`] emits
/// carries an already-admitted depth, so the refusals below are a hand-built
/// bundle's, exactly as [`AdmissionError`]'s are a hand-built plan's. They are
/// refusals rather than documentation because the numbers they hold are the numbers
/// [`execute`](crate::execute) decides an ending from, and a bundle that lies about
/// them mints the same false completeness claim a hand-edited *plan* is refused for
/// at the admission waist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UnitError {
    /// The unit records a depth of zero, which reads nothing and proves nothing.
    #[error(
        "a stratum unit cannot record a depth of zero: a read of no rows proves nothing about the producer behind it"
    )]
    ZeroDepth,

    /// The unit records a depth so deep that the emitted bound cannot carry the
    /// probe row one past it, so how the read ended could not be observed.
    #[error(
        "a stratum unit cannot record depth {depth}: a read is emitted one row deeper than its \
         depth, so {ceiling} is the deepest depth whose ending can be observed and anything past \
         it would be reported exhausted without being read to its end"
    )]
    DepthWithoutProbe {
        /// The depth the unit recorded.
        depth: u32,
        /// The deepest depth whose emitted bound can still carry a probe row.
        ceiling: u32,
    },

    /// The unit records a depth above the row bound it says the registry declared,
    /// so the read it describes cannot be taken.
    ///
    /// The mirror of [`AdmissionError::DepthBoundViolation`] one stage later, and
    /// the same claim: a depth past the declaration is a read the producer cannot
    /// serve, and the rows it does return would then be certified as the whole of a
    /// deeper read. A declared zero is read as the floor of one, exactly as the
    /// planner and the waist read it.
    #[error(
        "a stratum unit records depth {depth} over a producer the registry bounds at {declared} \
         row(s) per invocation: the read it describes cannot be taken, and the rows it returns are \
         not the depth's"
    )]
    DepthBeyondDeclaration {
        /// The depth the unit recorded.
        depth: u32,
        /// The row bound the unit says the registry declared.
        declared: u64,
    },
}

/// One stratum's independently executable query, and the contract the rows it
/// returns will arrive under.
///
/// The query is read back as text through [`Self::sparql`], which renders every
/// bound in it from the depth this unit records.
///
/// # Nothing writable decides how a read ended
///
/// [`Self::depth()`], [`Self::declared_rows()`], the reach derived from them and the
/// bounds the text is run under are the whole of what [`execute`](crate::execute)
/// uses to say how a read ended. The first two are reachable only through
/// [`Self::new`], which refuses the combinations that have no honest ending; the
/// third is derived from them and never supplied; and the bounds are not fields at
/// all — [`Self::sparql`] renders them from the depth, into a query held as the parts
/// it was assembled from.
///
/// Each of those was a hole, and they were the same hole. A plain public `u32`
/// re-opened what the admission waist closes: a depth raised past the range the
/// emitted bound can probe made `Exhausted` reportable for a read the `LIMIT` cut,
/// and a declared bound lowered below the depth bypassed the waist's own
/// [`AdmissionError::DepthBoundViolation`] dimension. A writable *text* was the
/// fourth, and it was the one that took three attempts to close: a caller writing the
/// natural `LIMIT <depth>` got `Exhausted` over a stratum with rows to spare, with
/// `rows_emitted` equal to the rows pulled, so nothing downstream could catch it.
/// Pulling the *outer* bound out of that text and rendering it left the branch's own
/// bound behind, inside the same writable string — and the inner copy is the one that
/// wins, so the identical false `Exhausted` was still reachable by lowering it. So no
/// bound is text a caller can reach: what the unit holds is the parts, and both
/// numbers are arithmetic over the proved depth.
///
/// # A caller's own query is still runnable, and is never certified
///
/// Driving [`execute`](crate::execute) over a text of a caller's own is the point of
/// [`Self::new`], and it stays open. What such a unit cannot do is yield
/// [`ProducerStatus::Exhausted`](crate::ProducerStatus) — this layer's strongest
/// completeness claim — because the claim rests on a read this layer bounded one row
/// past the depth, and it cannot see what bound a caller's text carries. Its ending
/// names that text as the stopper instead
/// ([`StreamEnding::SuppliedQueryEnded`](crate::StreamEnding)), which is the same
/// doctrine the rest of this vocabulary follows: name the stopper, never certify what
/// was not observed. A row arriving *past* the depth is still an observation, so such
/// a read still ends `DepthReached`, and a producer that beats its own declaration is
/// still refused by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StratumUnit {
    /// The caller-supplied stratum the unit ranks within.
    pub stratum: Iri,
    /// The duplicate handling and the candidate domains the stratum's producer
    /// declared — read off the registry here, at the one stage that is already
    /// holding the declaration, and carried forward rather than re-fetched.
    ///
    /// A stratum carries exactly one producer, so this is that producer's own
    /// declaration with nothing derived: there is no second producer's contract
    /// to reconcile it with, and none to weaken it to. [`execute`](crate::execute)
    /// tags each stream with it and the fusion engine reads it before pulling a
    /// row, so the promise a stream is held to is the one the registry the unit
    /// was *compiled against* stated — an identity `execute` re-checks before it
    /// runs anything.
    pub contract: StreamContract,
    /// The query this unit runs, and who wrote it.
    ///
    /// Private, and private for the reason the depth is: it is not one opaque string
    /// but a value whose variants say different things about what the run may claim.
    /// A rendered query is the parts a bound is written between, so both bounds come
    /// out of [`Self::depth`] every time [`Self::sparql`] is called; a supplied query
    /// is a caller's text, read back through [`Self::supplied_query`] and never
    /// certified. A writable field here could be neither, because it could be
    /// replaced with a text carrying a bound of its own and nothing would know.
    query: UnitQuery,
    /// The stratum's planned depth: the most rows this unit may contribute to
    /// the answer, exactly as [`Plan::stratum_depths`] records it.
    ///
    /// This is **not** the `LIMIT` [`Self::sparql`] renders. Those bounds are one
    /// row deeper, at every depth a unit can be compiled for — a depth whose extra
    /// row is not expressible is refused before this stage rather than emitted
    /// without one, which is what the already-probed depth held here is — so that the
    /// executor can tell a read the depth cut from a read
    /// that ran out; see this module's header. The probe row is a read and never a
    /// value, so the number a consumer reasons about — and the number every other
    /// field of this bundle is keyed to — is this one.
    ///
    /// It travels on the unit rather than being looked up again from the plan
    /// for the reason [`Self::contract`] does: [`execute`](crate::execute) is
    /// handed the compiled bundle and nothing else, and a depth re-read from a
    /// plan at execution time would be a fact about that plan rather than about
    /// the text that is actually being run.
    ///
    /// Read through [`Self::depth()`]; private because it is checked, and held as an
    /// already-probed depth because [`Self::sparql`] adds the probe row to it with
    /// exact arithmetic — see this type's header.
    depth: ProbedDepth,
    /// The row bound the registry declared for this stratum's one producer, or
    /// `None` where it declared no access mode and so declared no bound at all.
    ///
    /// "Declared nothing" and "declared zero" are different facts and do not
    /// share a representation here for the same reason they do not share one in
    /// admission: a missing declaration can refuse nothing, while a zero is a
    /// measurement of the producer's data.
    ///
    /// [`execute`](crate::execute) needs it to read the probe row. A row
    /// arriving past [`Self::depth()`] means something further existed, and only
    /// this number says *what*: below the declaration it is the depth that cut
    /// the read, which is an ordinary
    /// [`ProducerStatus::DepthReached`](crate::ProducerStatus); at the
    /// declaration it is the producer yielding a row after promising there is
    /// none, which is
    /// [`ExecutionError::RowBoundBreached`](crate::ExecutionError). Without it
    /// the executor would have to treat the two alike, and the only ending it
    /// could pick for the second is the completeness claim that breach makes
    /// false.
    ///
    /// It travels on the unit for the reason [`Self::contract`] does: the
    /// declaration this unit was compiled against is the one the run must be
    /// judged by, and a bound re-read from a registry at execution time would be
    /// a fact about that registry rather than about the text being run.
    ///
    /// Read through [`Self::declared_rows()`]; private because it is checked against
    /// [`Self::depth()`] — see this type's header.
    declared_rows: Option<u64>,
    /// Whether this unit's read reaches one row past its own depth.
    ///
    /// Derived from the depth, the declaration and the query by [`read_reach`], never
    /// supplied, and read by [`execute`](crate::execute) to tell an ending it can
    /// observe from one it cannot.
    reach: ReadReach,
}

impl StratumUnit {
    /// A unit that runs the caller's own `query`, read to `depth` rows over a producer
    /// the registry bounds at `declared_rows`.
    ///
    /// This is the seam a caller starts at to drive [`execute`](crate::execute) over
    /// a bundle it assembled itself, and it is checked because these numbers
    /// decide what the run may claim about how the read ended. `declared_rows` is
    /// `None` for a producer whose registry declared no access mode and therefore no
    /// row count; "declared nothing" and "declared zero" are different facts here
    /// for the reason they are different facts at the waist.
    ///
    /// `query` is carried verbatim and is bounded by this layer only on the outside:
    /// [`Self::sparql`] is that text as a sub-`SELECT` of a query bounded at
    /// `LIMIT depth + 1`. It wraps rather than follows, because a text carrying a
    /// top-level bound of its own can hold no second one, and wrapped, both bounds
    /// stand — the caller's over the pattern it was written against, this layer's over
    /// whatever that resolves to. Whatever else the text
    /// bounds — a sub-`SELECT` of its own, a pattern that matches less — is the
    /// caller's and is not visible from here, so the read it describes is **never**
    /// certified [`Exhausted`](crate::ProducerStatus::Exhausted); see this type's
    /// header for what its ending is instead. There is no way to hand this
    /// constructor a query that *is* certified, deliberately: that text is
    /// [`compile`]'s to render, from parts no caller supplies.
    ///
    /// # Errors
    ///
    /// * [`UnitError::ZeroDepth`] when `depth` is zero.
    /// * [`UnitError::DepthWithoutProbe`] when `depth` is so deep that the bound one
    ///   row past it is not expressible, so no ending could be observed.
    /// * [`UnitError::DepthBeyondDeclaration`] when `depth` is above the declared row
    ///   bound, read — as everywhere in this layer — with a declared zero floored at
    ///   the one probing row.
    pub fn new(
        stratum: Iri,
        query: String,
        contract: StreamContract,
        depth: u32,
        declared_rows: Option<u64>,
    ) -> Result<Self, UnitError> {
        // Decided by the waist's own predicate rather than by a second copy of it,
        // and named here in this stage's vocabulary: "can this depth carry its probe
        // row" is one question, and a bundle and a plan must not be able to answer
        // it differently.
        let probed = ProbedDepth::checked(depth).map_err(|reason| match reason {
            Unprobeable::Zero => UnitError::ZeroDepth,
            Unprobeable::PastCeiling => UnitError::DepthWithoutProbe {
                depth,
                ceiling: MAX_READ_DEPTH,
            },
        })?;
        if let Some(declared) = declared_rows
            && declared.max(1) < u64::from(depth)
        {
            return Err(UnitError::DepthBeyondDeclaration { depth, declared });
        }
        Ok(Self::assembled(
            stratum,
            UnitQuery::Supplied(query),
            contract,
            probed,
            declared_rows,
        ))
    }

    /// The unit [`compile`] emits, with no check to make.
    ///
    /// Every condition [`Self::new`] refuses is already proved by the types at the
    /// call site: a [`ProbedDepth`] exists only for a depth the waist admitted as
    /// neither zero nor unprobeable, and the same waist refused the depth against
    /// this very [`RowBound`]. So this is not [`Self::new`] with the checks skipped —
    /// there is nothing left here for a check to decide, and a `Result` no caller
    /// could act on would be a second, weaker statement of the invariant the waist
    /// already holds.
    ///
    /// It is also the only constructor of a [`UnitQuery::Rendered`], which is what
    /// makes "the layer wrote this text" a fact about the type rather than a claim
    /// about a string.
    fn emitted(
        stratum: Iri,
        query: RenderedQuery,
        contract: StreamContract,
        depth: ProbedDepth,
        bound: RowBound,
    ) -> Self {
        Self::assembled(
            stratum,
            UnitQuery::Rendered(query),
            contract,
            depth,
            bound.rows(),
        )
    }

    /// The one place the fields are written, so the reach is derived exactly once.
    fn assembled(
        stratum: Iri,
        query: UnitQuery,
        contract: StreamContract,
        depth: ProbedDepth,
        declared_rows: Option<u64>,
    ) -> Self {
        Self {
            stratum,
            reach: read_reach(depth.get(), declared_rows, &query),
            query,
            contract,
            depth,
            declared_rows,
        }
    }

    /// The whole text this unit runs.
    ///
    /// For a query this layer rendered, that is the whole query assembled from its
    /// parts with both bounds — the branch's row ceiling and the unit's own — computed
    /// from [`Self::depth()`]. For a query a caller supplied, it is that text wrapped in
    /// a query carrying the unit's own bound, which is the only bound this layer can
    /// write over a text it did not assemble. It wraps rather than follows because a
    /// text carrying a top-level bound of its own can hold no second one after it, and
    /// wrapping needs to know nothing about the text.
    ///
    /// Derived rather than stored, so the bounds the read is taken under and the depth
    /// the ending is judged against cannot be different numbers. The probe row is
    /// added with exact rather than saturating arithmetic, because the depth this unit
    /// holds is one the waist already proved the row past it fits — see this module's
    /// header for what a saturated or caller-written bound cost.
    #[must_use]
    pub fn sparql(&self) -> String {
        match &self.query {
            UnitQuery::Rendered(rendered) => rendered.text(self.depth, self.declared_rows),
            UnitQuery::Supplied(text) => supplied_text(text, self.depth),
        }
    }

    /// The query text a caller supplied, or `None` for a query this layer rendered.
    ///
    /// Read-only, and the read is not the hazard: a caller may look at, log or re-run
    /// the text it handed over. What it cannot do is replace the text a *rendered*
    /// unit runs, because there is no text there to replace — only the parts, and two
    /// numbers this layer computes between them.
    #[must_use]
    pub fn supplied_query(&self) -> Option<&str> {
        match &self.query {
            UnitQuery::Rendered(_) => None,
            UnitQuery::Supplied(text) => Some(text),
        }
    }

    /// The stratum's planned depth: the most rows this unit may contribute.
    #[must_use]
    pub const fn depth(&self) -> u32 {
        self.depth.get()
    }

    /// The row bound the registry declared for this stratum's one producer, or
    /// `None` where it declared no access mode and so declared no bound at all.
    #[must_use]
    pub const fn declared_rows(&self) -> Option<u64> {
        self.declared_rows
    }

    /// Whether this unit's read reaches one row past its depth.
    pub(crate) const fn reach(&self) -> ReadReach {
        self.reach
    }
}

/// Which producer's read one unit's evidence is attributed to, and the promise that
/// read is held to — recorded when a bundle is assembled, so
/// [`execute`](crate::execute) can tell the units it was handed from the units the
/// bundle was built out of.
///
/// These two are the bundle's half of the hole [`StratumUnit`]'s own header describes.
/// The unit's numbers and its text are no longer writable, but the *tag* above them was:
/// a stratum IRI renamed onto its neighbour's attached one producer's status to the
/// other producer's read, a swap crossed both, and dropping a unit narrowed the answer
/// by a whole producer — each with a real plan identity on the bundle, an `Exact`
/// exactness on the fused answer, and nothing anywhere reporting it. The waist enforces
/// "a stratum missing from the compiled set is a producer missing from the emitted text"
/// from the plan into `compile`; this is the same implication carried the one stage
/// further, from `compile` into `execute`.
///
/// What it deliberately does **not** record is the unit's query, depth or declared row
/// bound. Substituting a text of one's own is the documented seam
/// ([`StratumUnit::new`]) and so is reading a stratum less deeply than the plan did; both
/// go through a checked constructor and neither can mint a false ending. Recording them
/// here would refuse the seam instead of the substitution above it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct UnitAttribution {
    /// The stratum the unit's rows are reported under.
    stratum: Iri,
    /// The duplicate policy and candidate domains its stream is held to.
    contract: StreamContract,
}

impl UnitAttribution {
    /// Read one unit's attribution off the unit.
    fn of(unit: &StratumUnit) -> Self {
        Self {
            stratum: unit.stratum.clone(),
            contract: unit.contract.clone(),
        }
    }
}

/// The admitted, compiled plan: the query units plus the identities that pin them.
///
/// `plan_id` is the plan's canonical identity, so a stream or answer can name the
/// pinned plan it descends from; `registry_id` and `registry_fingerprint` are the
/// registry the units were compiled against, so [`execute`](crate::execute) can
/// refuse to run the same text against a different registry and silently obtain a
/// different meaning.
///
/// # The units are writable and are checked
///
/// A bundle is assembled through [`Self::new`] — by [`compile`] or by a caller that
/// starts at [`execute`](crate::execute) — and records the attribution above each unit
/// as it is handed over. `execute` refuses a unit list that is no longer that one, so
/// re-tagging, swapping or dropping a unit after the fact is a named refusal rather than
/// an answer served under a real plan identity. What is compared is the unit count and
/// then each position's stratum, count first, because a removal shifts every position
/// after it and reporting the first shifted tag would name a unit nothing was done to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledRetrieval {
    /// Per-stratum query units, ordered by stratum IRI.
    ///
    /// Writable, and checked rather than sealed: a caller may substitute a unit's
    /// query — that is [`StratumUnit::new`]'s whole purpose — but the set of producers
    /// the bundle answers for, and which read each one's evidence describes, is fixed
    /// when the bundle is assembled. [`execute`](crate::execute) refuses a unit list
    /// that is not the one this bundle was built from
    /// ([`ExecutionError::UnitsNotAsAssembled`](crate::ExecutionError::UnitsNotAsAssembled)).
    /// The count and each position's stratum are what is compared; a unit's own numbers
    /// are not, because its constructor already refuses a dishonest pair of those.
    pub units: Vec<StratumUnit>,
    /// The canonical identity of the admitted plan.
    pub plan_id: PlanId,
    /// The live registry instance the units were compiled against.
    pub registry_id: RegistryId,
    /// The durable content fingerprint of the registry the units were compiled
    /// against.
    pub registry_fingerprint: String,
    /// The row bound these units' depths were derived for, as the [`TopK`] a
    /// fusion of them must be run at.
    ///
    /// The plan carries a [`ReadBound`], which is a caller's request; this is that
    /// request resolved against the plan in hand, once, at the stage that holds
    /// both. [`ReadBound::Bounded`] resolves to its own row count.
    /// [`ReadBound::Complete`] resolves to the sum of the depths the plan records
    /// — the count at which a fusion of these units provably cannot truncate,
    /// because every unit is bounded at its depth and a fused answer holds each
    /// candidate once.
    ///
    /// [`execute`](crate::execute) tags every stream with it and
    /// [`fuse`](crate::fuse) reads it back, so fusing a bundle at some other bound
    /// is a named refusal
    /// ([`FusionError::ReadBoundMismatch`](crate::FusionError::ReadBoundMismatch))
    /// rather than an answer served out of depths that were derived for a
    /// different question. It travels on the bundle for the reason
    /// [`StratumUnit::depth()`] does: `execute` is handed the bundle and nothing
    /// else.
    pub fused_bound: TopK,
    /// Per-stratum rank resolution this plan will fuse at, when the environment
    /// named the profile it will be fused under.
    ///
    /// Empty when it did not. A profile is deliberately not a planning input, so
    /// a caller that has not yet chosen one is not asked to, and gets no
    /// resolution evidence because none can honestly be computed.
    pub resolution: BTreeMap<Iri, PlannedResolution>,
    /// Which producer each unit answered for when the bundle was assembled, in
    /// order — see [`UnitAttribution`].
    ///
    /// Private, and private for the reason a [`StratumUnit`]'s depth is: it is what
    /// [`Self::units`] is checked *against*, so a field a caller could rewrite
    /// alongside the units would check nothing at all.
    attribution: Vec<UnitAttribution>,
}

impl CompiledRetrieval {
    /// Assemble a bundle out of `units` and the identities that pin them.
    ///
    /// This is the seam a caller starts at to drive [`execute`](crate::execute) over a
    /// bundle of its own, and it is a function rather than a struct literal for one
    /// reason: the attribution above each unit — which producer's read it answers for,
    /// and the promise that read is held to — is recorded here, from the units actually
    /// handed over, and is what those units are checked against later. A literal could
    /// not record it, and a bundle that recorded nothing could be re-tagged after the
    /// fact with a real plan identity still on it.
    ///
    /// Nothing is refused here. A caller assembling its own bundle is making its own
    /// claim about which producers answered, exactly as a caller assembling its own
    /// [`Plan`] is; what the record buys is that the claim cannot change afterwards.
    #[must_use]
    pub fn new(
        units: Vec<StratumUnit>,
        plan_id: PlanId,
        registry_id: RegistryId,
        registry_fingerprint: String,
        fused_bound: TopK,
        resolution: BTreeMap<Iri, PlannedResolution>,
    ) -> Self {
        let attribution = units.iter().map(UnitAttribution::of).collect();
        Self {
            units,
            plan_id,
            registry_id,
            registry_fingerprint,
            fused_bound,
            resolution,
            attribution,
        }
    }

    /// Refuse a unit list that is not the one this bundle was assembled from, naming
    /// what moved.
    ///
    /// Read by [`execute`](crate::execute) before anything runs, because every later
    /// step is keyed by the stratum a unit carries: the status map, the stream's own
    /// tag, and the per-stratum weight the fusion profile applies. A tag that moved
    /// after assembly misdirects all three at once.
    ///
    /// The count is reported before the per-position comparison, because a removal
    /// shifts every position after it and reporting the first shifted tag would name a
    /// unit nothing was done to.
    pub(crate) fn tagged_as_assembled(&self) -> Result<(), ExecutionError> {
        let held: Vec<UnitAttribution> = self.units.iter().map(UnitAttribution::of).collect();
        if held.len() != self.attribution.len() {
            return Err(ExecutionError::UnitsNotAsAssembled {
                plan: self.plan_id,
                reason: format!(
                    "it was assembled with {} unit(s) and holds {}, so a producer it \
                     answers for is one this bundle was not built to answer for",
                    self.attribution.len(),
                    held.len()
                ),
            });
        }
        for (position, (assembled, held)) in self.attribution.iter().zip(&held).enumerate() {
            if assembled.stratum != held.stratum {
                return Err(ExecutionError::UnitsNotAsAssembled {
                    plan: self.plan_id,
                    reason: format!(
                        "the unit at position {position} was assembled under stratum {} and \
                         now reports under {}, which attaches its read's evidence to \
                         another producer",
                        assembled.stratum, held.stratum
                    ),
                });
            }
            if assembled.contract != held.contract {
                return Err(ExecutionError::UnitsNotAsAssembled {
                    plan: self.plan_id,
                    reason: format!(
                        "the unit at position {position} reports under stratum {} with a \
                         duplicate policy or candidate domain the bundle was not assembled \
                         with, so the promise its stream is held to is not the one its \
                         producer declared",
                        held.stratum
                    ),
                });
            }
        }
        Ok(())
    }
}

/// What a planned depth costs in rank resolution under a named fusion profile.
///
/// This is evidence, not a verdict. A stratum read past its separating range
/// still produces a correct, deterministic answer — the declared tie-break is
/// total — at a coarser resolution, so the honest thing to hand back is the two
/// numbers and let the caller decide, rather than refuse the plan. It is
/// available here, at the waist, rather than only in the fused trailer, so a
/// caller learns what a plan will cost *before* paying to execute it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedResolution {
    /// Where this profile stops separating adjacent ranks in this stratum.
    pub separation: MonotoneDepth,
    /// The per-stratum depth the plan recorded.
    pub requested_depth: u32,
}

impl PlannedResolution {
    /// Whether every rank this plan reads is still separated from its
    /// neighbours by score alone.
    ///
    /// `false` does not mean the answer is wrong. It means ranks past the
    /// separating point are ordered by the tie-break's later keys — best stratum
    /// rank ascending, then canonical term bytes — rather than by the fused
    /// score. Ask [`FusionProfile::class_width`](crate::FusionProfile::class_width)
    /// how coarse that is.
    #[must_use]
    pub const fn fully_separated(self) -> bool {
        self.separation.covers(self.requested_depth as u64)
    }
}

/// The name of the variable every branch projects its candidate under, without
/// the `?` sigil.
///
/// Spelled once, because [`execute`](crate::execute) reads the emitted unit's
/// solutions back by exactly this name: two constants that could drift would let
/// the executor look for a column the compiler stopped writing.
pub(crate) const CANDIDATE_NAME: &str = "candidate";

/// The name of the variable a branch projects each row's block under, without
/// the `?` sigil.
///
/// Spelled once beside [`CANDIDATE_NAME`] and for the same reason: the executor
/// finds this column by exactly this name, and a second constant could drift.
///
/// The column is emitted only for a producer whose declaration names the position
/// to read it from
/// ([`RankedDeclaration::block_position`](purrdf_sparql_eval::RankedDeclaration)),
/// so a unit compiled for a producer that names no block per row is
/// byte-identical to the one this compiler emitted before blocks existed.
pub(crate) const BLOCK_NAME: &str = "block";

/// Admit `plan` against `env` and emit its per-stratum SPARQL units.
///
/// The admission checks are documented on [`AdmissionError`]; this function adds
/// only the emission. A plan that passes admission yields exactly one unit per
/// stratum the plan declares that also has at least one bound producer, ordered by
/// stratum IRI, each independently executable through `purrdf-sparql-eval`.
///
/// # Errors
///
/// The distinct [`AdmissionError`] variant of the first violated admission
/// dimension; see [`AdmissionError::dimension`].
pub fn compile(
    plan: &Plan,
    env: &AdmissionEnvironment<'_>,
) -> Result<CompiledRetrieval, AdmissionError> {
    let admitted = admit_plan(plan, env)?;

    let mut units = Vec::new();
    // The depths are read from the admitted view rather than from the plan, and
    // the map has one entry per entry of `Plan::stratum_depths`, so this iterates
    // exactly the strata the plan recorded a depth for. What it cannot iterate is
    // a depth nobody checked: a `ProbedDepth` exists only where the waist refused
    // neither a zero nor a depth too deep to carry its probe row, which is what
    // makes an emitted `LIMIT` equal to its own depth unwritable here rather than
    // merely unwritten.
    for (stratum, depth) in &admitted.stratum_depths {
        let Some(binding) = admitted.stratum_bindings.get(stratum) else {
            // A stratum the plan gives a depth but no producer has no relation
            // to run, so there is nothing to emit for it. That is the only case
            // this arm can reach: admission refuses a binding whose stratum the
            // plan records no depth for, so iterating the depth keys cannot skip
            // a bound producer. Without that check this `continue` would be the
            // silent narrowing — a producer dropped from the emitted text with
            // nothing reporting it.
            continue;
        };
        let declaration = ranked_declaration(binding, &admitted.descriptors)?;
        // The bound the registry declared for this stratum, taken from the map
        // admission already decided the depth against. A stratum the registry
        // declares nothing about cannot appear here — admission refuses a
        // binding whose stratum no ranked producer emits under — but the lookup
        // is still total, and the total answer is the one that enforces nothing:
        // `Undeclared` is silence, and silence bounds no read.
        let bound = admitted
            .stratum_row_bounds
            .get(stratum)
            .copied()
            .unwrap_or(RowBound::Undeclared);
        // The placement the waist derived and judged the depth against. A stratum with
        // a binding always has one — the waist places every binding it admits — and a
        // stratum without one never reaches here, because the lookup above already
        // skipped it.
        let Some(invocation) = admitted.stratum_invocations.get(stratum) else {
            return Err(malformed(
                binding,
                "was admitted without a placement, so there is no invocation to emit",
            ));
        };
        let query = emit_query(binding, declaration, &admitted.descriptors, invocation)?;
        units.push(StratumUnit::emitted(
            stratum.clone(),
            query,
            StreamContract::declared(declaration),
            *depth,
            bound,
        ));
    }
    units.sort_by(|left, right| left.stratum.cmp(&right.stratum));

    // The profile the answer will be fused under is read here for what it can
    // say about this plan's depths, and it says it rather than refusing it. A
    // stratum the profile does not weight contributes nothing to that fusion, so
    // a profile silent about it has nothing to report and gets no entry.
    let resolution = env.fusion_profile.map_or_else(BTreeMap::new, |profile| {
        admitted
            .stratum_depths
            .iter()
            .filter_map(|(stratum, depth)| {
                profile.monotone_depth(stratum).map(|separation| {
                    (
                        stratum.clone(),
                        PlannedResolution {
                            separation,
                            requested_depth: depth.get(),
                        },
                    )
                })
            })
            .collect()
    });

    Ok(CompiledRetrieval::new(
        units,
        plan.id(),
        admitted.instance_id,
        admitted.fingerprint,
        fused_bound(plan),
        resolution,
    ))
}

/// The [`TopK`] a fusion of this plan's units must be run at.
///
/// [`ReadBound::Bounded`] is already that number. [`ReadBound::Complete`] asked
/// for everything the strata hold, and what they hold is bounded by the depths the
/// plan records: each unit contributes at most its own depth rows, a fused answer
/// carries each candidate exactly once, so the sum over the depths is a count no
/// fusion of these units can reach — and therefore a bound that truncates
/// nothing. It is the honest resolution of "complete" for a bundle whose reads are
/// themselves bounded, rather than a number picked to be large.
///
/// The sum saturates. A saturated sum is still a bound nothing can reach, because
/// reaching it would need more distinct candidates than a `usize` can count.
fn fused_bound(plan: &Plan) -> TopK {
    match plan.read_bound {
        ReadBound::Bounded(top_k) => top_k,
        ReadBound::Complete => {
            TopK::new(plan.stratum_depths.values().fold(0_usize, |total, depth| {
                total.saturating_add(*depth as usize)
            }))
        }
    }
}

/// The ranked declaration `binding`'s producer supplied at registration.
///
/// Looked up once per stratum and handed to both readers — the emitter, which
/// needs its placements, and the unit's [`StreamContract`], which is two of its
/// fields. One lookup because one declaration: a second read could drift from
/// the first, and the text and the contract must describe the same producer.
fn ranked_declaration<'a>(
    binding: &ProducerBinding,
    descriptors: &'a BTreeMap<String, PfDescriptor>,
) -> Result<&'a RankedDeclaration, AdmissionError> {
    descriptors
        .get(&binding.producer)
        .ok_or_else(|| malformed(binding, "has no registry declaration to compile against"))?
        .ranked
        .as_ref()
        .ok_or_else(|| malformed(binding, "declares no ranked capability to compile against"))
}

/// The row bound the emitted text actually carries for a stratum planned at
/// `depth`.
///
/// One function, and the only source of a bound anywhere in this layer.
/// [`RenderedQuery::text`] calls it for the branch's own `LIMIT` and again for the
/// unit's outer bound; [`StratumUnit::sparql`] calls it for the outer bound it writes
/// onto a caller's own text, which is the only bound it can write there. Every one of
/// those is this arithmetic over the depth in hand — no spelling of the probe row is
/// stored anywhere for a second reading to disagree with, and the two readings that
/// used to exist did disagree, because one of them sat inside a string a caller could
/// replace.
///
/// `depth + 1`, at every depth and against every declaration, and this module's
/// header has the argument for why the registry's declared row bound does not
/// appear: admission proves the depth is at or below that declaration, so a
/// `min(depth, declared)` selects the depth in every emittable case, and the one
/// declaration it did not — a declared zero — is a declaration this layer reads
/// rather than obeys, where capping the bound to the depth erased the probe row and
/// certified `Exhausted` for a read nobody could see the end of.
///
/// The addition is exact rather than saturating, and it is exact because of the
/// argument's type. A saturating `+ 1` at `u32::MAX` emitted a bound *equal* to
/// the depth: no probe row could arrive, `execute` writes `DepthReached` only when
/// a row arrives past the depth, and the read was therefore reported `Exhausted`
/// — the strongest completeness claim this layer has — for a stratum the `LIMIT`
/// may well have cut. That is the fault this whole header is about, surviving at
/// the one depth where the mitigation was dropped. So the depth arrives as a
/// [`ProbedDepth`], which the waist mints only for a depth whose probe row fits
/// ([`AdmissionError::DepthWithoutProbe`]), and the row past it is added by
/// [`ProbedDepth::probe`] with nothing left to saturate.
fn emitted_limit(depth: ProbedDepth) -> u32 {
    depth.probe()
}

/// The whole text a unit runs over a query a caller supplied: that text as a
/// sub-`SELECT`, with this layer's bound on the result of the whole of it.
///
/// The layer's bound **wraps** the caller's text rather than following it, and the
/// difference is the difference between a query and an invalid one. Appended, the two
/// bounds are two `LimitClause`es of one `SolutionModifier`, which the grammar
/// (`LimitOffsetClauses ::= LimitClause OffsetClause? | OffsetClause LimitClause?`)
/// admits exactly one of; a text carrying its own top-level `LIMIT 2` was emitted as
/// `... LIMIT 2 LIMIT 13`, and where a parser keeps the last clause the caller's own
/// bound simply vanished — three rows came back from a text that asked for two, and the
/// ending named that text as the stopper for a read it had not stopped.
///
/// Wrapped, both bounds stand and neither is this layer's opinion about the other: the
/// caller's applies to the pattern it was written against, and this layer's applies to
/// whatever that whole text resolves to, which is the only result this layer can
/// honestly bound. That is also the outer bound the type's own contract promises, so the
/// promise is now true of the text rather than of the intention behind it.
///
/// # Why this is a wrap and not a refusal
///
/// A refusal would have to know whether a supplied text already carries a top-level
/// solution modifier, and knowing that means parsing it. This layer does not parse
/// SPARQL, and the seam deliberately admits a text that is not SPARQL at all —
/// [`execute`](crate::execute) reports the parser's own diagnostic as that stratum's
/// [`ProducerStatus::ExecutionFailed`](crate::ProducerStatus::ExecutionFailed), which
/// is a status a caller reaches on purpose. A gate at the constructor would therefore
/// have to refuse either every text it could not parse — closing the seam — or nothing,
/// which is no gate. Wrapping needs to know nothing about the text and leaves every
/// runnable text runnable.
///
/// The projection is `*` rather than the two columns
/// [`execute`](crate::execute) reads, because naming them would *add* those columns to
/// a text that did not project them: an unbound `?candidate` projected by this layer
/// reads back as a row whose candidate column is absent, where a text that projects no
/// candidate should be reported as exactly that.
fn supplied_text(text: &str, depth: ProbedDepth) -> String {
    format!(
        "SELECT * WHERE {{\n  {{ {text} }}\n}}\nLIMIT {}",
        emitted_limit(depth)
    )
}

/// The number handed to a producer that declares a
/// [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement), for a stratum planned
/// at `depth` over a producer the registry bounds at `declared_rows`.
///
/// `max(1, min(depth + 1, declared))`, which differs from [`emitted_limit`] at
/// exactly one depth — a depth already at the declaration — and differs there
/// because this number is a *request* rather than a ceiling; this module's
/// header has the argument. Asking a producer for `declared + 1` rows asks it to
/// contradict its own registration, and the shipped nearest-neighbour relation
/// refuses that against its configured guard rather than returning a short
/// answer as a complete one. The `min` here is therefore not the silent cap it
/// was on the `LIMIT`: the unit's own bound still reaches one row past the
/// declaration and still catches a producer that beats it.
///
/// The floor of one is the floor the planner and the waist already apply to a
/// declared zero, for the same reason: a declared zero would otherwise ask such a
/// producer for no rows at all, and an answer of nothing to a request for nothing
/// proves nothing about the index.
///
/// Where this number lands *on* the depth the read cannot reach past it, and
/// [`read_reach`] is where that is turned into the ending
/// [`execute`](crate::execute) is allowed to report.
///
/// It is called twice over one unit and must agree with itself both times: once at
/// emission, where [`place`] needs the value to occupy the argument slot and derive
/// the access mode it satisfies, and once per read of the unit's text, where
/// [`RenderedQuery::text`] renders it into that slot. Same function, same two
/// arguments, both of them facts the unit holds.
fn depth_argument(depth: ProbedDepth, declared_rows: Option<u64>) -> u32 {
    let probe = depth.probe();
    match declared_rows {
        Some(declared) => u32::try_from(declared)
            .unwrap_or(u32::MAX)
            .min(probe)
            .max(1),
        // Nothing was declared, so there is no registration for the request to
        // exceed and the probe is asked for outright.
        None => probe,
    }
}

/// Assemble one stratum's query over its one `binding`, as the parts a bound is
/// rendered between.
///
/// Neither bound is written here, and that is the whole point of the return type:
/// [`RenderedQuery::text`] renders the branch's row ceiling and the unit's own bound
/// from the depth, every time the text is asked for, so there is no string in between
/// for either number to be edited in.
///
/// The `invocation` is the waist's own — placement runs there, because the access mode
/// it derives is what the depth was admitted against. Emitting from a *second*
/// placement would build the text under a mode nothing had judged the depth by, which
/// is why this takes the invocation rather than deriving one.
fn emit_query(
    binding: &ProducerBinding,
    declaration: &RankedDeclaration,
    descriptors: &BTreeMap<String, PfDescriptor>,
    invocation: &Invocation,
) -> Result<RenderedQuery, AdmissionError> {
    let descriptor = descriptors
        .get(&binding.producer)
        .ok_or_else(|| malformed(binding, "has no registry declaration to compile against"))?;
    let subject = descriptor.subject_arity;
    let total = subject + descriptor.object_arity;
    if total == 0 {
        return Err(malformed(
            binding,
            "declares zero arguments, so a call cannot name a candidate",
        ));
    }

    let candidate = declaration.candidate_position;
    if candidate >= total || invocation.mode.is_bound(candidate) {
        // The registry validates that the candidate position exists and is never
        // a placement target, so this is a registry that moved under the plan.
        return Err(malformed(
            binding,
            "projects a candidate from a position that is not a free argument",
        ));
    }

    // The block column is the candidate column's sibling: a position the unit
    // *reads*, so it must exist and must be free for the invocation to bind
    // anything into. The registry validates both at registration; reaching this
    // means the registry moved under the plan, which is the same claim the
    // candidate's own check makes.
    let block = match declaration.block_position {
        None => None,
        Some(position) if position < total && !invocation.mode.is_bound(position) => Some(position),
        Some(_) => {
            return Err(malformed(
                binding,
                "projects each row's block from a position that is not a free argument",
            ));
        }
    };

    // Every slot is rendered here except the depth argument's, which comes back as the
    // datatype its producer declared and is rendered from the unit's own depth instead.
    // Which position that is comes from the placement itself rather than from a second
    // reading of the declaration, so the number in the emitted call is the one
    // `RenderedQuery::text` computes and never a copy of it.
    let arguments = render_slots(invocation).map_err(|error| unrenderable(binding, &error))?;
    let (subject_arguments, object_arguments) = arguments.split_at(subject);
    Ok(RenderedQuery {
        producer: binding.producer.clone(),
        subject: subject_arguments.to_vec(),
        object: object_arguments.to_vec(),
        candidate,
        block,
    })
}

/// A structural defect in the plan-plus-registry pair, named by producer.
fn malformed(binding: &ProducerBinding, what: &str) -> AdmissionError {
    AdmissionError::MalformedPlan {
        reason: format!("producer {} {what}", binding.producer),
    }
}

/// A placed value that has no SPARQL constant form.
///
/// [`place`] proves every slot it fills renders, so reaching this means the
/// registry moved between the two calls; it is reported on the same dimension
/// because it is the same claim.
fn unrenderable(binding: &ProducerBinding, error: &RenderError) -> AdmissionError {
    match Iri::parse(&binding.producer) {
        Ok(producer) => AdmissionError::UnsatisfiablePlacement {
            producer: Box::new(producer),
            rule: "unrenderable",
            detail: error.to_string(),
            invocation: None,
            declared: Vec::new(),
        },
        Err(invalid) => AdmissionError::MalformedPlan {
            reason: format!("plan binds invalid producer IRI {invalid}"),
        },
    }
}
