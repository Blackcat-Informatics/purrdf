// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! the one ending that names no stopper, said about the bound rather
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
//! [`ProducerStatus::Exhausted`](crate::ProducerStatus), which is the one ending
//! that names no stopper, minted for a read the plan itself
//! cut short. That is the zero-depth fault one size larger: a bound on the read
//! silently becoming a statement about the answer.
//!
//! So the emitted bound is `depth + 1`, on the branch and on the unit. The extra
//! row is a **read, never a value**: [`execute`](crate::execute) emits at most
//! `depth` rows onto the stream and uses the arrival of the `depth + 1`-th only to
//! end the stream [`DepthReached`](crate::ProducerReceipt::DepthReached) instead of
//! `Exhausted`. No plan field and no identity moves by one:
//! [`PlannedResolution::requested_depth`] is the depth, and so is
//! [`StratumUnit::depth()`]. The one number it does move is the one that asks
//! what the read *cost* rather than what the answer is made of —
//! [`StratumResolution::rows_materialised`](crate::StratumResolution), which
//! counts the rows the evaluator handed back and would understate the read by
//! exactly the row that makes its ending observable if it did not.
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
//! *past* the depth, so that read was reported `Exhausted` — the one ending that
//! names no stopper — for a text that had cut it. A depth of three
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
//! [`StratumUnit::new`], and it is a *different* kind of unit: the caller's query form
//! reaches the evaluator byte for byte — except that a dataset clause, which the
//! sub-`SELECT` grammar has no place for, is moved onto the wrapper, where it scopes
//! the same body — this layer bounds only its outside, and what
//! bounds the caller wrote inside it cannot be seen from here. Such a read therefore
//! never ends `Exhausted` — the ending names the caller's text as the stopper instead
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
//! This layer does read a supplied text once, at one seam and for two reasons that are
//! the same reason. [`StratumUnit::new`] parses it with the parser
//! [`execute`](crate::execute) will run, so a text that is not a query is refused
//! there — by name, at construction — rather than surfacing as a per-stratum failure
//! after a plan was admitted and other strata were read; and the parse reports where
//! the two clauses a whole query may write and a sub-`SELECT` may not — the **prologue**
//! and the **dataset clause** — are written, which the wrapping `SELECT` has to know.
//! Wrapping a text that declares `PREFIX` or `BASE` without lifting those directives out
//! first makes the whole query unparsable, and the caller then gets a parse error at a
//! byte offset of a query it never wrote. Wrapping one that writes `FROM` or `FROM NAMED`
//! without lifting that clause out was worse, because it parsed: the clause was read
//! inside the wrapper and then discarded, so the caller's query ran against a dataset it
//! had explicitly narrowed away from, and the rows came back under a perfectly ordinary
//! ending. Both clauses are now moved to where the grammar takes them, and each still
//! scopes the body it was written around. Nothing about the *bounds* is read from that
//! parse: the outer bound is still this layer's arithmetic over the proved depth, and
//! what the caller's own text bounds inside itself is still not this layer's to certify.
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
//! cannot rise above the declaration to go looking either — the planner's
//! `derived_bound` derives every depth downward from the declared row count, and the
//! one row it can add is the floor that keeps a zero-declaring producer's relation
//! invoked, so no planner-written plan asks for more, and raising the *argument* past
//! the declaration is precisely the request a conforming relation must refuse. What
//! the layer can do honestly is report that it read to the producer's bound and no
//! further, and it does.
//!
//! Which of the two cases a unit was emitted for is not recorded beside its query: it
//! is read *off* that query, by [`read_reach`], because "the producer was handed the
//! depth as an argument" and "the branch carries no `LIMIT` of its own" are one fact
//! written once. What [`StratumUnit`] carries is the answer — how far its read could
//! reach — so [`execute`](crate::execute) reads the ending off the unit in hand rather
//! than re-deriving it from a registry three stages away.
//!
//! # The exclusion lookup renders no depth at all
//!
//! Everything above is about the **streaming** text, and none of it changes. The
//! second text a capable stratum compiles to
//! ([`RenderedQuery::exclusion_text`]) leaves the depth position free — a blank
//! node, like every free position of a lookup other than the candidate's, since the
//! lookup reads nothing out of them — because it is asking a different question. A depth is an offer — how many
//! rows to rank — and a producer handed one answers *is this candidate among your best
//! n*, whose absences are not exclusions: a candidate at rank `n + 1` is one the stream
//! will still name, and a consumer that read its absence as an exclusion would refuse
//! the fused read as a contradiction. There is no number that renders the right
//! question here, so no number is rendered. A caller's own text gets the same lookup,
//! derived from each call its `?candidate` column is drawn from
//! ([`StratumUnit::exclusion_sparql`]), and each depth position is freed the same way
//! whatever number the caller wrote there.
//!
//! The candidate goes the other way. It is left as the one variable a caller binds per
//! lookup, and it is **declared** to the prepare rather than merely substituted into it
//! — [`execute`](crate::execute) prepares the lookup through
//! [`prepare_execution`](purrdf_sparql_eval::NativeSparqlEngine::prepare_execution)
//! with the candidate as its parameter — so the feasibility pass sees the candidate
//! bound and the depth free, and admits the
//! call in the producer's membership mode rather than in its ranked one. That pairing is
//! the whole of what makes a lookup against a self-bounding producer a lookup.

use std::collections::BTreeMap;
use std::ops::Range;

use purrdf_sparql_algebra::{
    BlankNode, GraphPattern, NamedNodePattern, ParserOptions, PropertyFunctionCall, Query,
    QueryDataset, SparqlParser, TermPattern, TriplePattern, Variable, pattern_to_select_query,
};
use purrdf_sparql_eval::{
    BindingPattern, CallReadShape, CandidateDomains, ColumnSource, ExclusionBasis, PfDescriptor,
    PreparedQuery, PropertyFunctionRegistry, RankedDeclaration, RegistryId,
};
use purrdf_text::Fixed;

use crate::admission::{
    AdmissionEnvironment, AdmissionError, BoundMode, MAX_READ_DEPTH, ProbedDepth, RowBound,
    Unprobeable, admit_plan,
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
/// observation. A query a caller supplied is text whose *bounds* this layer never
/// reads — it parses the text far enough to find where the two clauses it has to move
/// sit (the prologue, and the dataset clause), to refuse something that is not a query,
/// and — where the unit declares an exclusion basis — to find the calls its lookups
/// are asked of, and interprets nothing else — so a `LIMIT` inside a sub-`SELECT`, a
/// `FILTER` or a pattern that simply matches less are all the caller's, and how that
/// read ended is not this layer's to certify. See this module's header.
#[derive(Clone, Debug, PartialEq, Eq)]
enum UnitQuery {
    /// The query [`compile`] rendered, held as the parts it was assembled from so
    /// that every bound in it is arithmetic over the unit's own depth, performed
    /// when the text is asked for.
    Rendered(RenderedQuery),
    /// A query text a caller supplied, bounded by nothing this layer wrote.
    ///
    /// The text is the caller's own bytes and is read back as such
    /// ([`StratumUnit::supplied_query`]). What the layer holds beside it is the two
    /// positions it needs to bound the text without breaking it or changing what it
    /// means: where the caller's prologue ends and where its dataset clause sits, both
    /// as [`StratumUnit::new`]'s parse reported them.
    Supplied {
        /// The caller's text, exactly as it was handed over.
        text: String,
        /// The byte offset the query form starts at, so the `BASE`/`PREFIX`/`VERSION`
        /// directives in front of it can be hoisted above the wrapping `SELECT`. Zero
        /// for a text with no prologue, which is every text this seam saw before a
        /// prefixed one reached it.
        body_at: usize,
        /// The byte range the caller's `FROM` / `FROM NAMED` run occupies, so it can be
        /// written onto the wrapping `SELECT` — the one place a sub-`SELECT` grammar
        /// leaves for it — instead of being carried inside the wrapper where it parses
        /// and then decides nothing. `None` for a text that names no graphs, which is
        /// every text whose answer the store's own default dataset gives.
        dataset_at: Option<Range<usize>>,
    },
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
        let render = |position: usize, argument: &UnitArgument| match argument {
            UnitArgument::Placed(text) => text.clone(),
            // The variable the projections below read the candidate and the block
            // out of, under the one spelling both texts share.
            UnitArgument::Free => format!("?c{position}"),
            UnitArgument::Depth { datatype } => {
                render::typed_literal(&depth_argument(depth, declared_rows).to_string(), datatype)
            }
        };
        let subject_len = self.subject.len();
        let subject_text = self
            .subject
            .iter()
            .enumerate()
            .map(|(position, argument)| render(position, argument))
            .collect::<Vec<_>>()
            .join(" ");
        let object_text = self
            .object
            .iter()
            .enumerate()
            .map(|(position, argument)| render(subject_len + position, argument))
            .collect::<Vec<_>>()
            .join(" ");
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

    /// The whole text of this producer's **exclusion lookup**: the same call,
    /// with the candidate position left as the one variable a caller binds per
    /// lookup.
    ///
    /// This is the second query a capable stratum compiles to, and it is a
    /// different question from the one [`Self::text`] asks. That one asks *give
    /// me your best rows*; this one asks *do you hold this one*, and the answer
    /// it needs is a row count rather than a ranking — so nothing here projects
    /// a block, carries a depth the plan derived, or renders a rank.
    ///
    /// # Why the candidate is a variable and not a constant
    ///
    /// Because the text is prepared **once** per stratum, as a prepared
    /// execution whose one parameter is the candidate, and run once per
    /// candidate with that parameter bound. A text with the candidate spelled
    /// into it would be a fresh parse and a fresh feasibility pass per lookup —
    /// the cost this whole mechanism exists to avoid paying in rows. Binding it
    /// instead writes the candidate into the call's own candidate position, so
    /// the producer receives it as a **bound argument** rather than generating
    /// its rows and letting a join filter them.
    ///
    /// # Why the bound is two rows
    ///
    /// A registered basis requires a candidate-bound mode whose declared row
    /// bound is a point bound, so a conforming producer answers with nought or
    /// one row. Asking for two is how a producer that answers with more is
    /// *observed* rather than truncated into looking conforming — the same probe
    /// row [`Self::text`]'s own bound carries, for the same reason.
    ///
    /// # The depth argument, where the producer takes one: left FREE
    ///
    /// Rendered as a variable nothing binds, which is the whole of what makes a
    /// lookup against a self-bounding producer a lookup.
    ///
    /// A depth is an *offer* — how many rows to rank — and a producer that
    /// receives one answers the ranked question, `is this candidate among your
    /// best n`. That question's absences are not exclusions: a candidate at rank
    /// n+1 is one the stream will still name, and a consumer that read its
    /// absence as an exclusion would refuse the fused read as a contradiction.
    /// Rendering any number here — one included — would ask that question, and
    /// the number chosen would only decide how often the wrong answer happened.
    ///
    /// Left free, the call binds the candidate and the request and nothing else,
    /// which is a different point of the producer's mode lattice and the one a
    /// declared membership basis is admitted against. The producer answers *do
    /// you hold this term at all*, and every row a ranking of any depth could
    /// have named is a row that answer finds.
    ///
    /// The streaming text ([`Self::text`]) is untouched by this: it still renders
    /// the depth the plan derived, because it really is asking for a ranking.
    fn exclusion_text(&self) -> String {
        let subject_len = self.subject.len();
        let render_at = |position: usize, argument: &UnitArgument| -> String {
            if position == self.candidate {
                return format!("?{CANDIDATE_NAME}");
            }
            match argument {
                UnitArgument::Placed(text) => text.clone(),
                // Every other free position — the depth's included — is a blank node,
                // one distinct label per position so no two can be read as one. That
                // is SPARQL's own spelling of "nothing reads this": the lookup
                // projects the candidate and nothing else, so a value produced at
                // any other free position is discarded unread, and a blank written
                // once says so to the engine, which reports the position unobserved
                // to the producer (`PfArgs::is_unobserved`). A producer whose rows
                // carry a value it can only compute at corpus-wide cost — a rank —
                // is thereby told it need not, which is the difference between a
                // lookup whose cost is a point search and one that ranks a partition
                // per candidate.
                UnitArgument::Free | UnitArgument::Depth { .. } => format!("_:c{position}"),
            }
        };
        let subject_text = self
            .subject
            .iter()
            .enumerate()
            .map(|(position, argument)| render_at(position, argument))
            .collect::<Vec<_>>()
            .join(" ");
        let object_text = self
            .object
            .iter()
            .enumerate()
            .map(|(position, argument)| render_at(subject_len + position, argument))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "SELECT ?{CANDIDATE_NAME} WHERE {{ ( {subject_text} ) <{}> ( {object_text} ) }}\nLIMIT \
             {EXCLUSION_LIMIT}",
            self.producer
        )
    }

    /// The access pattern [`Self::exclusion_text`] invokes its producer in.
    ///
    /// The declaration's own derivation
    /// ([`RankedDeclaration::exclusion_lookup_mode`]), asked of the positions this
    /// text really fills: the candidate, and every position a placement rendered a
    /// constant into. Every other position — the depth, and a position no placement
    /// filled — is a blank in the text and free here, so this is the pattern the
    /// lookup's prepare is admitted in, derived without preparing it.
    fn exclusion_lookup_mode(&self, declaration: &RankedDeclaration) -> BindingPattern {
        declaration.exclusion_lookup_mode(self.arguments().count(), |position| {
            matches!(
                self.arguments().nth(position),
                Some(UnitArgument::Placed(_))
            )
        })
    }
}

/// The contract a rendered unit's stream is held to: its producer's declaration,
/// with the exclusion basis kept only where the lookup this unit would ask is one
/// the producer declares a mode for.
///
/// A declared basis is admitted at registration against the widest lookup any
/// text can ask — every position but the depth supplied — because a caller's own
/// text can fill a position no request facet reaches. A rendered unit cannot: its
/// lookup ([`RenderedQuery::exclusion_text`]) binds the candidate and the placed
/// constants and nothing else. So a producer whose only candidate-bound mode also
/// binds some other position is one this unit's lookup cannot be asked of, and
/// attaching the basis anyway would hand [`execute`](crate::execute) a lookup that
/// fails to prepare, failing the whole stratum at search time for evidence the
/// stratum's ranking read never needed.
///
/// The basis is the producer's, attached here without the caller asking for it,
/// so the unit that cannot use it declares
/// [`ExclusionBasis::Unavailable`] instead — the stratum still runs and still
/// ranks, and the fusion reads it as a stream that answers no lookup, which is
/// exactly true of it. That is visible where every stratum's basis is:
/// [`StratumUnit::contract`] and the fused trailer's
/// [`exclusion_bases`](crate::FusionTrailer::exclusion_bases). The question asked is
/// the one [`lookup_mode_is_declared`] asks of a caller's call: does some declared
/// mode [subsume](BindingPattern::subsumes) the lookup's pattern.
fn rendered_contract(
    declaration: &RankedDeclaration,
    query: &RenderedQuery,
    descriptor: &PfDescriptor,
) -> StreamContract {
    let mut contract = StreamContract::declared(declaration);
    if contract.exclusion.is_declared() {
        let lookup = query.exclusion_lookup_mode(declaration);
        let served = descriptor
            .modes
            .iter()
            .any(|declared| BindingPattern::from_code(&declared.code).subsumes(lookup));
        if !served {
            contract.exclusion = ExclusionBasis::Unavailable;
        }
    }
    contract
}

/// The row ceiling every exclusion lookup is read under.
///
/// One past the point bound a declared basis requires, so a producer that
/// answers a candidate-bound call with more than one row is observed rather than
/// silently truncated into looking like a conforming one. See
/// [`RenderedQuery::exclusion_text`].
pub(crate) const EXCLUSION_LIMIT: u64 = 2;

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
        UnitQuery::Supplied { .. } => return ReadReach::Unknown,
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

/// When one **run** produces each stratum's rows: all of them before the first is
/// readable, or each one as its consumer asks for it.
///
/// A schedule belongs to a run and never to the bundle. Every unit carries the depth
/// the planner derived for it, [`StratumUnit::sparql`] renders that depth, and both
/// schedules run exactly that text's invocation — the same arguments, the same bound,
/// the same probe row one past the depth. What a schedule decides is only *when* each
/// of those rows is produced, and so how many of them a consumer that stops early ever
/// pays for. It is therefore a parameter of [`execute_within`](crate::execute_within)
/// rather than a field of anything.
///
/// Neither schedule reads a row twice and neither takes a second invocation: an
/// on-demand read that is asked for more simply goes on reading the invocation it
/// opened, up to the planned depth and its probe row, and a consumer that never asks
/// again leaves the rest of it unproduced. That is why there is no fallback between
/// the two — a read that stopped short was stopped by its consumer, and a read its
/// consumer needed deeper was never cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadSchedule {
    /// Every stratum is read to its planned depth, and its rows held, before the
    /// first one is readable: the unfused rung [`execute`](crate::execute) returns,
    /// whose every stream already knows how it ended.
    Materialised,
    /// Every unit whose prepared text is one property-function call under
    /// row-for-row operators — projections, `OFFSET`-free `LIMIT`s, renaming `BIND`s
    /// and `FILTER`s evaluated row by row
    /// ([`PreparedQuery::is_call_read`](purrdf_sparql_eval::PreparedQuery::is_call_read))
    /// — is opened at its planned depth and read one row at a time as its stream is
    /// pulled, with the invocation held open between pulls; see
    /// [`search`](crate::search)'s header. That is every unit this layer rendered,
    /// and a caller's own text of the same shape: a `FILTER` it wrote drops rows as
    /// they are pulled, and a dropped row takes no rank. A unit whose text is anything
    /// else — a join, an `ORDER BY`, a dataset clause — is materialised under this
    /// schedule too, because it is not one invocation that can be held open.
    OnDemand,
}

/// A set of facts no unit can describe a read with, and a text that is no read at all.
///
/// Raised by [`StratumUnit::new`] and by nothing else: a unit [`compile`] emits
/// carries an already-admitted depth, so the refusals below are a hand-built
/// bundle's, exactly as [`AdmissionError`]'s are a hand-built plan's. They are
/// refusals rather than documentation because the numbers they hold are the numbers
/// [`execute`](crate::execute) decides an ending from, and a bundle that lies about
/// them mints the same false completeness claim a hand-edited *plan* is refused for
/// at the admission waist.
///
/// [`Self::NotAQuery`] is the one that is not about a number. It is here rather than
/// left to execution time because the constructor now reads the text anyway — the
/// wrapping bound cannot be written over a prologue without knowing where that
/// prologue ends — so the refusal costs nothing and arrives where the caller can still
/// act on it.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
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

    /// The unit runs a caller's own query text while declaring an exclusion
    /// basis, and that text is not one the lookup the basis promises can be
    /// derived from.
    ///
    /// An exclusion lookup is one of the caller's own calls asked a different
    /// question — with the candidate bound instead of ranked — and its `Excluded`
    /// answer is a statement about the text only where every value the text's
    /// `?candidate` column takes is a value that call emitted: then a candidate the
    /// call never emits is a candidate the text never names. The shape description
    /// an on-demand read also reads
    /// ([`CallReadShape::sources_of`](purrdf_sparql_eval::CallReadShape::sources_of))
    /// derives which calls those are, node by node. A call is one through
    /// projections, `FILTER`s, `DISTINCT`, `ORDER BY`, `LIMIT`/`OFFSET`, renaming
    /// `BIND`s, a join with any other pattern, the required side of an `OPTIONAL`,
    /// the left side of a `MINUS`, a `GROUP BY` key, and `GRAPH` — each of which only
    /// drops, merges, reorders or extends the solutions beneath it — and a text with
    /// several such calls asks each. A `UNION` each of whose branches binds the
    /// column from a call is one too, each branch an alternative a candidate must be
    /// excluded from. A text whose `?candidate` column no call is a source of is
    /// refused, and the `reason` names why: a `UNION` whose other branch binds it
    /// from something that is not a call, an `OPTIONAL` whose required side can bind
    /// it where the call on its optional side does not, a call only on the subtracted
    /// side of a `MINUS`, a computed `BIND`, `SELECT` expression or `GROUP BY`
    /// condition, or an aggregate, or a column only data, `VALUES` or an expression
    /// binds.
    ///
    /// Refused rather than quietly dropped, because dropping it is the silent
    /// half of the same defect: the contract would keep promising a lookup, the
    /// fusion engine would keep asking for one, and every ask would come back a
    /// failure at read time — a unit that cannot work, assembled without
    /// complaint. A caller whose text draws its candidates from no call declares
    /// [`ExclusionBasis::Unavailable`](purrdf_sparql_eval::ExclusionBasis),
    /// which is exactly true of a stream nothing can be looked up in.
    #[error(
        "a stratum unit running a caller-supplied query can declare an exclusion basis of \
         {basis} only when every value its ?candidate column takes was emitted by a \
         property-function call, because the exclusion lookup is such a call asked with the \
         candidate bound; this query's is not: {reason}"
    )]
    ExclusionNotRenderable {
        /// The basis the unit's contract declared.
        basis: &'static str,
        /// Why no call is a source of the text's `?candidate` column, in the shape
        /// description's own words.
        reason: String,
    },

    /// The supplied text is not a SPARQL query: the parser
    /// [`execute`](crate::execute) would have run refused it.
    ///
    /// Refused at construction rather than at execution, which is the whole of the
    /// difference this variant makes. The text was going to be run either way, and the
    /// diagnostic is the parser's own either way; what changes is that it names the
    /// caller's text at the moment the caller handed it over, instead of arriving as
    /// one stratum's [`ProducerStatus::ExecutionFailed`](crate::ProducerStatus) after a
    /// bundle was assembled and its other strata were read.
    ///
    /// A text the parser accepts here can still fail at execution and still reports
    /// that failure as its own stratum's status: this constructor parses with no
    /// relation registry, so a registered relation's predicate is an ordinary triple
    /// pattern to it, and the registry-aware parse the executor runs is the authority
    /// on everything about the seam. This refusal is therefore exactly "not a query",
    /// and never "not a query this registry likes".
    #[error("a stratum unit's supplied text is not a SPARQL query: {reason}")]
    NotAQuery {
        /// The parser's own diagnostic, carried rather than summarized.
        reason: String,
    },

    /// The supplied text is a query, but not a `SELECT`, and only a `SELECT` yields
    /// the solution rows a stratum is read as.
    ///
    /// The layer's bound wraps the text as a sub-`SELECT`, and the grammar's
    /// `SubSelect` admits exactly that form: an `ASK`, `CONSTRUCT` or `DESCRIBE`
    /// inside the wrapper is a syntax error at a byte offset of a query the caller
    /// never wrote. It would be a semantic refusal even if the grammar allowed it —
    /// an `ASK` answers with one boolean and the other two with triples, none of which
    /// is a row carrying a candidate and a rank. So there is no `ASK`, `CONSTRUCT` or
    /// `DESCRIBE` text that could ever run through this seam, and refusing it by its
    /// form name at the constructor costs no valid text anything.
    #[error(
        "a stratum unit's supplied text is a {form} query, and only a SELECT can be read as \
         ranked rows: the layer's bound wraps the text as a sub-SELECT, which the grammar admits \
         only for SELECT, and a {form} answers with no solution rows to rank"
    )]
    NotASelect {
        /// The query form the text was written in: `ASK`, `CONSTRUCT` or `DESCRIBE`.
        form: &'static str,
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
/// [`ProducerStatus::Exhausted`](crate::ProducerStatus) — the one ending that names
/// no stopper — because that ending rests on a read this layer bounded one row
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
    /// declaration: there is no second producer's contract to reconcile it with,
    /// and none to weaken it to. The one term a compiled unit can hold differently
    /// is the exclusion basis, and only downwards: a unit [`compile`] rendered
    /// declares [`ExclusionBasis::Unavailable`](purrdf_sparql_eval::ExclusionBasis)
    /// where its producer declared a basis but no mode serving the lookup that
    /// unit's text would ask, because that unit answers no lookup and its stream
    /// says so rather than failing at search time. [`execute`](crate::execute)
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
    /// Which declared access mode [`Self::declared_rows`] was read at.
    ///
    /// Carried for the reason the count itself is: the count is a function of the mode,
    /// and the refusal a producer that beats it earns
    /// ([`ExecutionError::RowBoundBreached`](crate::ExecutionError)) has to name the
    /// declaration its author must go and fix. For a multi-mode producer that mode is
    /// routinely *not* the invoked one — a coarser mode declaring less bounds the read
    /// tighter — so it cannot be re-derived from the invocation at execution time, and
    /// a number without it names a figure the invocation never declared.
    ///
    /// [`BoundMode::Undeclared`] for a unit a caller assembled through [`Self::new`],
    /// which records a count and no mode.
    declared_mode: BoundMode,
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
    /// `query` is read back verbatim through [`Self::supplied_query`], and it is
    /// bounded by this layer only on the outside: [`Self::sparql`] is the caller's
    /// query form as a sub-`SELECT` of a query bounded at `LIMIT depth + 1`, with any
    /// prologue the caller wrote hoisted above that wrapper and any dataset clause it
    /// wrote moved onto the wrapper's own `SELECT`. It wraps rather than
    /// follows, because a text carrying a top-level bound of its own can hold no second
    /// one, and wrapped, both bounds stand — the caller's over the pattern it was
    /// written against, this layer's over whatever that resolves to. Whatever else the
    /// text bounds — a sub-`SELECT` of its own, a pattern that matches less — is the
    /// caller's and is not visible from here, so the read it describes is **never**
    /// certified [`Exhausted`](crate::ProducerStatus::Exhausted); see this type's
    /// header for what its ending is instead. There is no way to hand this
    /// constructor a query that *is* certified, deliberately: that text is
    /// [`compile`]'s to render, from parts no caller supplies.
    ///
    /// # Why the text is parsed here
    ///
    /// The wrap is the reason, and the grammar's `SubSelect ::= SelectClause
    /// WhereClause SolutionModifier ValuesClause` gives it two halves. A sub-`SELECT`
    /// has no prologue, so a text declaring `PREFIX` or `BASE` — which is how
    /// essentially all SPARQL is written — cannot be wrapped as it stands: the
    /// directives have to be lifted above the wrapping `SELECT`. A sub-`SELECT` has no
    /// `DatasetClause` either, so a text writing `FROM` or `FROM NAMED` cannot be
    /// wrapped as it stands *and cannot be diagnosed* — it parsed, the clause was
    /// discarded, and the caller's query ran against the store's whole dataset instead
    /// of the graphs it named. Finding where either clause sits is a question only a
    /// parser can answer. So this constructor parses `query` with the same front end
    /// [`execute`](crate::execute) will run, takes both positions, and
    /// refuses a text that is not a query at all ([`UnitError::NotAQuery`]) rather than
    /// carrying it to a stratum failure later. The same parse names the query form, and
    /// a form other than `SELECT` is refused too ([`UnitError::NotASelect`]): the
    /// wrapper is a sub-`SELECT`, the grammar admits nothing else inside one, and an
    /// `ASK`, `CONSTRUCT` or `DESCRIBE` yields no solution rows to read as ranks in
    /// any case — so no text of those forms could ever have run here, and refusing
    /// them by name loses nothing valid. The caller's own bytes are what get wrapped;
    /// nothing is re-rendered from the parse, so the body reaches the evaluator
    /// unchanged but for the dataset clause cut out of it, which is re-written on the
    /// wrapper where it scopes that same body.
    ///
    /// The parse decides nothing else, and in particular decides no bound. It is run
    /// with no relation registry, so a registered producer's predicate is an ordinary
    /// triple pattern to it; the registry-aware parse at execution is still the
    /// authority on the seam, and a text it refuses still reports that refusal as its
    /// own stratum's
    /// [`ProducerStatus::ExecutionFailed`](crate::ProducerStatus).
    ///
    /// # A declared exclusion basis reads the text once more
    ///
    /// Where `contract` declares an exclusion basis, every value the text's
    /// `?candidate` column takes must be a value some property-function call of the
    /// text emitted, because the lookup that basis promises is such a call asked with
    /// the candidate bound — see [`UnitError::ExclusionNotRenderable`] for which calls
    /// those are. [`execute`](crate::execute) derives the lookups from the text as the
    /// registry prepares it. This constructor still has no registry, so it parses the
    /// text a second time with **every** bare predicate IRI read as a call — the
    /// widest reading any registry could give it — and asks the shape description an
    /// on-demand read also reads ([`CallReadShape`]). A registry can only read some of
    /// those calls as triple patterns instead, and a triple pattern is never a
    /// source, so what that reading refuses every registry refuses: a `?candidate`
    /// column a `UNION` branch, an `OPTIONAL`, a `MINUS`, a computed expression, an
    /// aggregate or `VALUES` keeps from any call. What it admits is checked again at execution,
    /// against the registry the unit runs under: a column no registered call is a
    /// source of, or one whose calls the registry holds no ranked declaration for,
    /// declares another basis for, or fills the declared candidate position of with
    /// another variable, is that stratum's
    /// [`ProducerStatus::ExecutionFailed`](crate::ProducerStatus), naming which.
    ///
    /// # Errors
    ///
    /// * [`UnitError::ZeroDepth`] when `depth` is zero.
    /// * [`UnitError::DepthWithoutProbe`] when `depth` is so deep that the bound one
    ///   row past it is not expressible, so no ending could be observed.
    /// * [`UnitError::DepthBeyondDeclaration`] when `depth` is above the declared row
    ///   bound, read — as everywhere in this layer — with a declared zero floored at
    ///   the one probing row.
    /// * [`UnitError::NotAQuery`] when the parser refuses `query`, carrying its
    ///   diagnostic.
    /// * [`UnitError::NotASelect`] when `query` parses as an `ASK`, `CONSTRUCT` or
    ///   `DESCRIBE`, naming the form.
    /// * [`UnitError::ExclusionNotRenderable`] when `contract` declares an exclusion
    ///   basis and no call of `query` is a source of its `?candidate` column, with a
    ///   `reason` naming what is in the way — see the section above for why the text
    ///   is read here with every bare predicate IRI taken as a call.
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
        let (body_at, dataset_at) = hoistable_clauses(&query)?;
        if contract.exclusion.is_declared() {
            candidate_drawn_from_calls(&query).map_err(|reason| {
                UnitError::ExclusionNotRenderable {
                    basis: contract.exclusion.as_str(),
                    reason,
                }
            })?;
        }
        Ok(Self::assembled(
            stratum,
            UnitQuery::Supplied {
                text: query,
                body_at,
                dataset_at,
            },
            contract,
            probed,
            declared_rows,
            // A caller's bundle records a count and no mode, so there is no
            // declaration behind this number for a refusal to name — and inventing
            // the invoked mode here would attribute the caller's own figure to a
            // registry read that never happened.
            BoundMode::Undeclared,
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
        invoked: BindingPattern,
    ) -> Self {
        Self::assembled(
            stratum,
            UnitQuery::Rendered(query),
            contract,
            depth,
            bound.rows(),
            // The attribution the waist's own `RowBound` carries, resolved against the
            // mode this very text is emitted under. Both facts are the waist's, taken
            // together here rather than re-derived at execution, where the invocation
            // is no longer in hand.
            bound.attributed(Some(invoked)),
        )
    }

    /// The one place the fields are written.
    const fn assembled(
        stratum: Iri,
        query: UnitQuery,
        contract: StreamContract,
        depth: ProbedDepth,
        declared_rows: Option<u64>,
        declared_mode: BoundMode,
    ) -> Self {
        Self {
            stratum,
            contract,
            query,
            depth,
            declared_rows,
            declared_mode,
        }
    }

    /// The whole text this unit runs.
    ///
    /// For a query this layer rendered, that is the whole query assembled from its
    /// parts with both bounds — the branch's row ceiling and the unit's own — computed
    /// from [`Self::depth()`]. For a query a caller supplied, it is that text wrapped in
    /// a query carrying the unit's own bound, which is the only bound this layer can
    /// write over a text it did not assemble. It wraps rather than follows because a
    /// text carrying a top-level bound of its own can hold no second one after it. The
    /// caller's prologue is hoisted in front of the wrapper and its dataset clause onto
    /// the wrapper's own `SELECT`, because a sub-`SELECT` can carry neither — see
    /// [`Self::new`] for why those moves are possible here at all, and for why each
    /// lands where it does.
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
            UnitQuery::Supplied {
                text,
                body_at,
                dataset_at,
            } => supplied_text(text, *body_at, dataset_at.as_ref(), self.depth),
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
            UnitQuery::Supplied { text, .. } => Some(text),
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

    /// The depth this unit's plan recorded, as the probed depth it was admitted
    /// as.
    ///
    /// The same number [`Self::depth()`] reports, carrying the proof that its
    /// probe row fits. Not public for the reason the field is not: nothing
    /// outside this module can build one, and that is what makes an unprobeable
    /// depth unreachable rather than merely unlikely.
    pub(crate) const fn probed_depth(&self) -> ProbedDepth {
        self.depth
    }

    /// The texts of this stratum's **exclusion lookups**, or `None` where its
    /// producer declared
    /// [`ExclusionBasis::Unavailable`](purrdf_sparql_eval::ExclusionBasis) and
    /// answers none.
    ///
    /// The lookups come as **alternatives**, each a list of lookups: a candidate is
    /// excluded when every alternative excludes it, and an alternative excludes it
    /// when any of its lookups finds no row.
    ///
    /// For a query this layer rendered there is one alternative of one lookup,
    /// rendered from the parts it was assembled from
    /// ([`RenderedQuery::exclusion_text`]), and `ranking` and `registry` decide
    /// nothing. Its mode was already checked against the producer's declared modes
    /// when the unit was compiled, and a unit whose lookup no declared mode serves
    /// was compiled declaring no basis ([`rendered_contract`]), so it reaches here
    /// only as `None`.
    ///
    /// For a query a caller supplied the lookups are derived from `ranking` — that
    /// query's own text, prepared against `registry` for its ranking read — through
    /// the one shape description an on-demand read also reads
    /// ([`PreparedQuery::call_read_shape`](purrdf_sparql_eval::PreparedQuery::call_read_shape)).
    /// It names the alternatives every value of the text's `?candidate` column was
    /// emitted by — one per branch of a `UNION` that binds it from a call, one in
    /// all for a text without such a choice — each a set of calls beside the call
    /// variable carrying the column through whatever projections, `FILTER`s, joins
    /// and renamings the caller wrote. A call qualifies when the registry holds a
    /// ranked declaration for it stating the unit's basis, and the call fills that
    /// declaration's candidate position with the variable the column is drawn from;
    /// each alternative's lookups are those of its qualifying calls, and an
    /// alternative with none refuses the derivation. A lookup is that call with that
    /// variable's positions left as the one parameter a lookup binds, the producer's
    /// declared depth position freed whatever the caller wrote there, every other
    /// constant the caller's own, and every other variable and blank node a blank of
    /// its own — exactly the lookup [`RenderedQuery::exclusion_text`] renders out of a
    /// rendered unit's parts, asked of a call the caller wrote, under `LIMIT 2` —
    /// except where the text **drives** the call: where patterns before it bind its
    /// inputs, those patterns are kept and the call is asked once per distinct binding
    /// of them (see [`qualifying_lookup`]).
    ///
    /// Within an alternative, every qualifying call is a proof of absence on its own:
    /// the alternative names only values each of its calls emitted. Across
    /// alternatives, each covers only the solutions of its own branch. So the stream
    /// asks the alternatives in the order returned here, each alternative's lookups
    /// in order until one finds no row, and answers `Possible` at the first
    /// alternative none of whose lookups excludes the candidate, and `Excluded` only
    /// when every alternative has one that does.
    ///
    /// # Why this is read from the prepared text and not at construction
    ///
    /// [`Self::new`] has no registry, so the parse it can make reads a registered
    /// predicate as an ordinary triple. It refuses the texts no registry can draw the
    /// column from a call for — it reads the text with every bare predicate IRI
    /// taken as a call, the widest reading any registry could give it. Which
    /// predicate *is* a call is a fact about the registry this unit runs against, and
    /// so is what the registry declared for it; both are read here, from the prepare
    /// [`execute`](crate::execute) runs anyway.
    ///
    /// # Errors
    ///
    /// `Some(Err(reason))` for a supplied text whose `?candidate` column no call of
    /// the prepared plan is a source of (a predicate the registry does not register
    /// reads as a triple pattern, which is none), or which has an alternative none of
    /// whose calls qualifies — the reason naming, for each of its calls, the
    /// declaration it lacks, the basis it declares instead, or the position it fills
    /// differently.
    /// [`execute`](crate::execute) reports it as the stratum's own
    /// [`ProducerStatus::ExecutionFailed`](crate::ProducerStatus), exactly as it
    /// reports a rendered lookup that will not prepare.
    pub(crate) fn exclusion_sparql(
        &self,
        ranking: &PreparedQuery,
        registry: &PropertyFunctionRegistry,
    ) -> Option<Result<Vec<Vec<LookupText>>, String>> {
        if !self.declares_exclusion() {
            return None;
        }
        Some(match &self.query {
            UnitQuery::Rendered(rendered) => {
                Ok(vec![vec![LookupText::point(rendered.exclusion_text())]])
            }
            UnitQuery::Supplied { .. } => {
                supplied_exclusion_texts(ranking, registry, self.contract.exclusion)
            }
        })
    }

    /// Whether this unit's contract declares an exclusion basis, and so whether
    /// [`Self::exclusion_sparql`] answers anything for it.
    pub(crate) const fn declares_exclusion(&self) -> bool {
        self.contract.exclusion.is_declared()
    }

    /// Whether this unit's read reaches one row past its planned depth.
    ///
    /// Derived rather than cached, because the answer is a function of the depth,
    /// the declared row bound and the query together — see [`read_reach`].
    pub(crate) fn reach(&self) -> ReadReach {
        read_reach(self.depth.get(), self.declared_rows, &self.query)
    }

    /// Which declared access mode [`Self::declared_rows`] was read at, for the refusal
    /// that names it. Not public: a caller reads the number it supplied or the number
    /// the compiler put there, and the attribution is the executor's to *report*, not a
    /// second dimension for a bundle to disagree about.
    pub(crate) const fn declared_mode(&self) -> BoundMode {
        self.declared_mode
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
/// an answer served under a real plan identity. What is compared is the unit count,
/// then each position's stratum and its stream contract — the duplicate policy and
/// candidate domains it was assembled under, which decide what fusion may skip and
/// what it must refuse. Count first, because a removal shifts every position after it
/// and reporting the first shifted tag would name a unit nothing was done to.
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
    /// The count, each position's stratum and its stream contract are what is compared;
    /// a unit's own numbers are not, because its constructor already refuses a dishonest
    /// pair of those.
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedResolution {
    /// Where this profile stops separating adjacent ranks in this stratum.
    pub separation: MonotoneDepth,
    /// The per-stratum depth the plan recorded.
    pub requested_depth: u32,
    /// The weights of every stratum of this bundle whose declared blocks meet
    /// this one's, this stratum's own included, in ascending stratum order.
    ///
    /// This is the set the fusion engine's threshold is a maximum over, read
    /// from the same declarations the engine will read: a candidate of this
    /// stratum lies in one of this stratum's blocks, and the strata that can
    /// still raise the threshold for that block are exactly the strata whose
    /// [`CandidateDomains`](purrdf_sparql_eval::CandidateDomains) intersect it.
    /// Ascending stratum order rather than sorted by value, so the list is a
    /// pure function of the plan and two compilations of one plan cannot
    /// disagree about it.
    ///
    /// It is the `sharing` argument of
    /// [`crossing_rank_at`](crate::crossing_rank_at): handed there with the
    /// profile's decay rule and the weights of the strata that name a candidate,
    /// it answers — at plan time, with no row read — the head rank at which that
    /// candidate first beats the threshold this stratum's sharers impose. Over
    /// strata whose blocks do not meet, that is a rank past the deepest row of
    /// the answer; over strata that share a block, it is a rank past the
    /// profile's smoothing constant, and only the declaration this set was
    /// derived from moves it.
    ///
    /// Empty only for a stratum this plan gave a depth and no producer, which
    /// emits no unit, contributes no stream and therefore has no declaration to
    /// meet anyone else's.
    pub sharing_weights: Vec<Fixed>,
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
    pub const fn fully_separated(&self) -> bool {
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
        let descriptor = admitted
            .descriptors
            .get(&binding.producer)
            .ok_or_else(|| malformed(binding, "has no registry declaration to compile against"))?;
        let contract = rendered_contract(declaration, &query, descriptor);
        units.push(StratumUnit::emitted(
            stratum.clone(),
            query,
            contract,
            *depth,
            bound,
            invocation.mode,
        ));
    }
    units.sort_by(|left, right| left.stratum.cmp(&right.stratum));

    // The profile the answer will be fused under is read here for what it can
    // say about this plan's depths, and it says it rather than refusing it. A
    // stratum the profile does not weight contributes nothing to that fusion, so
    // a profile silent about it has nothing to report and gets no entry.
    let resolution = env.fusion_profile.map_or_else(BTreeMap::new, |profile| {
        // The declarations the crossing derivation reads, taken off the units
        // this call just emitted rather than off the registry a second time: a
        // stratum contributes to a threshold only by contributing a stream, and
        // a unit is exactly the evidence that it will. Ascending stratum order
        // comes free with the sort above, which is what makes every weight list
        // below a pure function of the plan.
        let declared: Vec<(&Iri, &CandidateDomains)> = units
            .iter()
            .map(|unit| (&unit.stratum, &unit.contract.domains))
            .collect();
        admitted
            .stratum_depths
            .iter()
            .filter_map(|(stratum, depth)| {
                profile.monotone_depth(stratum).map(|separation| {
                    // Every stratum whose declared blocks meet this one's, this
                    // one included — a producer's declaration always meets
                    // itself — and weighted, because an unweighted stratum
                    // contributes nothing for a threshold to be a maximum of.
                    // A stratum with no unit has no declaration here and meets
                    // nobody, which is the honest empty answer rather than an
                    // `Unrestricted` fabricated on its behalf.
                    let mine = declared
                        .iter()
                        .find(|(named, _)| *named == stratum)
                        .map(|(_, domains)| *domains);
                    let sharing_weights = mine.map_or_else(Vec::new, |mine| {
                        declared
                            .iter()
                            .filter(|(_, theirs)| mine.intersects(theirs))
                            .filter_map(|(named, _)| profile.weight(named))
                            .collect()
                    });
                    (
                        stratum.clone(),
                        PlannedResolution {
                            separation,
                            requested_depth: depth.get(),
                            sharing_weights,
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
/// — the one ending that names no stopper — for a stratum the `LIMIT`
/// may well have cut. That is the fault this whole header is about, surviving at
/// the one depth where the mitigation was dropped. So the depth arrives as a
/// [`ProbedDepth`], which the waist mints only for a depth whose probe row fits
/// ([`AdmissionError::DepthWithoutProbe`]), and the row past it is added by
/// [`ProbedDepth::probe`] with nothing left to saturate.
fn emitted_limit(depth: ProbedDepth) -> u32 {
    depth.probe()
}

/// The whole text a unit runs over a query a caller supplied: the caller's prologue,
/// then their query form as a sub-`SELECT`, with this layer's bound on the result of
/// the whole of it.
///
/// `body_at` is where that prologue ends, as [`hoistable_clauses`] read it off the
/// parse [`StratumUnit::new`] already ran; `text[..body_at]` is hoisted in front of
/// the wrapper and `text[body_at..]` goes inside it.
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
/// # Why the prologue and the dataset clause are moved, and moved as text
///
/// The grammar's sub-`SELECT` is `SubSelect ::= SelectClause WhereClause
/// SolutionModifier ValuesClause`, and this wrapper inherits both of that
/// production's omissions.
///
/// It has no prologue in it: `PREFIX` and `BASE` are `Prologue`, which appears once, at
/// the front of a whole query. So a caller's text that declares either — which is how
/// essentially all real SPARQL is written — stops parsing the moment it is wrapped as it
/// stands, and the diagnostic a caller then sees points at a byte offset of a query this
/// layer wrote. Wrapping without hoisting fixed one narrow shape (a text carrying its own
/// top-level bound) and broke a strictly larger valid class, including that same shape
/// whenever it was prefixed.
///
/// It has no `DatasetClause` in it either, and that omission failed the other way — not
/// with a diagnostic but with an answer. A caller's `FROM` / `FROM NAMED` parsed inside
/// the wrapper and was then discarded, so a text naming one graph was executed against
/// the whole store: `FROM <…/no-such-graph>` returned rows read from the very default
/// graph the caller had excluded, under a genuine ending, with nothing anywhere saying
/// the clause had been ignored. A wrong answer under no refusal is the worse of the two
/// failures, because the first one at least tells its caller something is wrong.
///
/// So both are moved, and to the one position the grammar accepts each. The directives go
/// in front of the wrapper; the dataset clause goes onto the wrapper's own `SELECT`,
/// where it scopes that whole query and therefore the body inside it — which is exactly
/// the scope the caller wrote it to have, a dataset clause never having been a statement
/// about one group. Both halves keep their meaning: the prefixed names in the body
/// resolve against the caller's own declarations, and the body reads the caller's own
/// dataset.
///
/// It is the *text* that moves, not a re-rendering of the parsed form, and that is
/// deliberate. A prefixed name is resolved away at parse time, so the prologue cannot be
/// recovered from the algebra at all; and re-serializing the body would rewrite the
/// caller's surface spelling — the argument lists a relation call is written with are
/// collection syntax to a parser that has not been told which IRIs are relations, and
/// re-emitting them as the blank-node chains they lower to would hand the executor a
/// text whose registry-aware parse no longer sees a call. Splitting at the offsets the
/// parse reported moves nothing but those two clauses, so the body the evaluator reads is
/// byte for byte the body the caller wrote, **except** that the dataset clause is cut out
/// of it — the one edit, and the only one, because the sub-select grammar has nowhere to
/// put it. A text with neither clause emits the bytes it always emitted.
///
/// The projection is `*` rather than the two columns
/// [`execute`](crate::execute) reads, because naming them would *add* those columns to
/// a text that did not project them: an unbound `?candidate` projected by this layer
/// reads back as a row whose candidate column is absent, where a text that projects no
/// candidate should be reported as exactly that.
fn supplied_text(
    text: &str,
    body_at: usize,
    dataset_at: Option<&Range<usize>>,
    depth: ProbedDepth,
) -> String {
    // Both offsets are byte positions the parser reported between two of its own
    // tokens, so both are on character boundaries of this very text and every split
    // below is total. `body_at` is zero for a text with no prologue, where `prologue`
    // is empty; `dataset_at` is `None` for a text with no dataset clause, where
    // `dataset` is empty and the whole body is one slice — which together emit exactly
    // the query this seam emitted before either clause reached it.
    let (prologue, rest) = text.split_at(body_at.min(text.len()));
    let (dataset, body) = match dataset_at {
        // The recorded range ends at the token after the clause, so it carries its own
        // trailing whitespace: the body rejoins as `SELECT …` + `WHERE …` with the
        // spacing the caller wrote on either side of the excision.
        Some(at) => (
            &text[at.clone()],
            format!("{}{}", &text[body_at..at.start], &text[at.end..]),
        ),
        None => ("", rest.to_owned()),
    };
    format!(
        "{prologue}SELECT * {dataset}WHERE {{\n  {{ {body} }}\n}}\nLIMIT {}",
        emitted_limit(depth)
    )
}

/// Where a caller-supplied query's two whole-query-only clauses are written, from the
/// parse that also decides whether the text is a query at all, and whether it is the
/// one form this seam can read as rows.
///
/// One parse, four answers, and the first two are why the other two can be afforded:
/// the wrapping [`supplied_text`] writes needs both positions, so reading the text is
/// not an extra check bolted onto the seam but the thing the seam already had to do.
/// The two refusals are not a policy about which queries are welcome here. A text that
/// is not a query is refused with the parser's own words ([`UnitError::NotAQuery`]); a
/// query that is not a `SELECT` is refused by its form name
/// ([`UnitError::NotASelect`]), because the wrapper is a sub-`SELECT` and the grammar
/// admits no other form inside one — an `ASK`, `CONSTRUCT` or `DESCRIBE` could never
/// have run through this seam, so nothing valid is lost by saying so at the
/// constructor. Every `SELECT` the parser accepts is admitted, whatever its dataset
/// clause, `VALUES` or `VERSION` carries, and everything about the *relations* in it is
/// left to the registry-aware parse at execution.
///
/// Parsed with [`ParserOptions::default`], i.e. with no relation namespaces and no
/// registered relation IRIs, for two reasons. There is no registry at this
/// constructor — a bundle a caller assembles names one only at
/// [`execute`](crate::execute) — and the parse is about grammar rather than about the
/// seam: recognizing a predicate as a relation call changes what the query *means*, not
/// whether it parses, and a call written with argument lists is ordinary collection
/// syntax to a parser that has not been told otherwise. The registry-aware parse stays
/// the authority, and is strictly the narrower of the two, so this refusal cannot
/// close the seam on a text execution would have accepted.
fn hoistable_clauses(text: &str) -> Result<(usize, Option<Range<usize>>), UnitError> {
    let split = SparqlParser::new()
        .parse_query_split(text, &ParserOptions::default())
        .map_err(|error| UnitError::NotAQuery {
            reason: error.to_string(),
        })?;
    let form = match split.query {
        Query::Select { .. } => return Ok((split.body_at, split.dataset_at)),
        Query::Ask { .. } => "ASK",
        Query::Construct { .. } => "CONSTRUCT",
        Query::Describe { .. } => "DESCRIBE",
    };
    // The other three forms carry a dataset clause too, and none of them reaches the
    // wrap: each is refused here, by name, before the position could matter.
    Err(UnitError::NotASelect { form })
}

/// Whether some property-function call of a caller's `text` could be a source of its
/// `?candidate` column under any registry — every value the column takes a value that
/// call emitted — or the reason none could.
///
/// [`StratumUnit::new`] holds no registry, so it reads `text` with **every** bare
/// predicate IRI taken as a call: an empty namespace claims them all. That is the
/// widest reading a registry could give the text: a registry can only read some of
/// those calls as triple patterns instead, and a triple pattern is never a source
/// ([`CallReadShape::sources_of`]). So whatever column this reading draws from no
/// call, every registry's reading draws from none too, and refusing it here refuses
/// nothing a registry would admit. What it admits is decided again, against the
/// registry the unit runs under, by [`StratumUnit::exclusion_sparql`].
fn candidate_drawn_from_calls(text: &str) -> Result<(), String> {
    let every_predicate_a_call = ParserOptions {
        property_fn_namespaces: vec![String::new()],
        ..ParserOptions::default()
    };
    let query = SparqlParser::new()
        .parse_query_with(text, &every_predicate_a_call)
        .map_err(|error| format!("it does not parse with its predicates read as calls: {error}"))?;
    CallReadShape::of(&query)
        .and_then(|shape| shape.sources_of(CANDIDATE_NAME).map(|_| ()))
        .map_err(|refusal| refusal.reason().to_owned())
}

/// One exclusion lookup: its text, the parameter the candidate is bound through, and
/// whether one candidate asks it one invocation or several.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LookupText {
    /// The lookup's whole text.
    pub(crate) text: String,
    /// The one variable the text leaves for a caller to bind: the candidate.
    pub(crate) parameter: String,
    /// Whether the call is driven by patterns of the caller's text (see
    /// [`supplied_exclusion_texts`]): then the text projects the call's driven inputs
    /// beside the candidate, invokes the call once per distinct binding of them, and is
    /// read whole — a conforming producer answers each invocation with at most one
    /// row, so two identical rows are a breach of that bound. An undriven lookup is one
    /// invocation read under [`EXCLUSION_LIMIT`].
    pub(crate) driven: bool,
}

impl LookupText {
    /// An undriven lookup whose parameter is [`CANDIDATE_NAME`].
    fn point(text: String) -> Self {
        Self {
            text,
            parameter: CANDIDATE_NAME.to_owned(),
            driven: false,
        }
    }
}

/// The exclusion lookups of a caller's text, derived from `ranking` — that text as
/// prepared against `registry` — for a unit whose contract declares `basis`: for each
/// alternative its `?candidate` column is drawn from (see
/// [`CallReadShape::sources_of`]), one lookup per qualifying call of that alternative,
/// in the order the shape description names them.
///
/// A candidate is excluded when **every** alternative excludes it, and an alternative
/// excludes it when **any** of its lookups does: every solution naming a candidate is
/// one some alternative's calls all emitted it in. So every alternative must keep a
/// qualifying call, or no verdict could be proof — an alternative none of whose calls
/// qualifies refuses the whole derivation, naming each call's reason. Within an
/// alternative, a call that does not qualify is simply not asked: asking fewer of an
/// alternative's calls can only turn an exclusion into a `Possible`, never the other
/// way.
///
/// See [`StratumUnit::exclusion_sparql`]. Each lookup is written as algebra and
/// serialised by the algebra crate's own writer, so the caller's constants reach it
/// as the terms the parse made of them rather than as a second spelling of the
/// caller's bytes.
fn supplied_exclusion_texts(
    ranking: &PreparedQuery,
    registry: &PropertyFunctionRegistry,
    basis: ExclusionBasis,
) -> Result<Vec<Vec<LookupText>>, String> {
    let shape = ranking
        .call_read_shape()
        .map_err(|refusal| refusal.reason().to_owned())?;
    let alternatives = shape
        .sources_of(CANDIDATE_NAME)
        .map_err(|refusal| refusal.reason().to_owned())?;
    let several = alternatives.len() > 1;
    let mut derived = Vec::with_capacity(alternatives.len());
    for (at, alternative) in alternatives.iter().enumerate() {
        let mut lookups = Vec::with_capacity(alternative.len());
        let mut unqualified = Vec::new();
        for source in alternative {
            match qualifying_lookup(source, registry, basis) {
                Ok(lookup) => lookups.push(lookup),
                Err(reason) => unqualified.push(reason),
            }
        }
        if lookups.is_empty() {
            let reasons = unqualified.join("; ");
            return Err(if several {
                format!(
                    "its ?{CANDIDATE_NAME} column is drawn from {} alternatives, one per branch \
                     that binds it, and a candidate is excluded only when every alternative \
                     excludes it; no call of alternative {} qualifies: {reasons}",
                    alternatives.len(),
                    at + 1
                )
            } else {
                reasons
            });
        }
        derived.push(lookups);
    }
    Ok(derived)
}

/// The exclusion lookup of `source`'s call, whose variable carries the text's
/// `?candidate` column — or why the registry does not make that call one a lookup of
/// `basis` may be asked of.
///
/// # A call its text drives
///
/// Where patterns of the text bind the call's other positions before it is invoked —
/// a needle read out of the data, a `VALUES` row, a `BIND` —
/// ([`ColumnSource::driving_pattern`]), the lookup keeps them: the call cannot be
/// invoked without those inputs, and blanking them asks it in a mode it never
/// declared. The lookup is
///
/// ```text
/// SELECT ?candidate ?in… [the text's dataset clause] WHERE {
///   { SELECT DISTINCT ?in… WHERE { driving pattern } }
///   call, the candidate the parameter, the inputs by name, every other position blank
/// }
/// ```
///
/// and it answers `Excluded` only when no row comes back. That is sound: every
/// solution of the text naming a candidate `c` is one in which the call, invoked with
/// the inputs the text bound, emitted `c`; the driving pattern is implied by the text
/// — every solution of the text, restricted to the inputs, is a solution of it
/// restricted to them, read under the same dataset clause (the rule and its proof
/// are [`ColumnSource::driving_pattern`]'s) — so the inputs that solution bound are
/// among the rows of the sub-`SELECT`. So if the call, invoked once per such row with
/// `c` bound, emits `c` for none of them, no solution of the text names `c`. Every
/// row of the sub-`SELECT` must fail, not one: a candidate one needle binding excludes
/// and another names is a candidate the text names. And where the sub-`SELECT` has no
/// row at all, neither does the text invoke the call: no candidate is one it names,
/// and the lookup answers `Excluded` having invoked nothing. The sub-`SELECT` projects
/// the inputs only, so a pattern that also constrains the candidate is read as the
/// wider set of inputs it binds for any candidate — a looser question, never a wrong
/// one — and it is `DISTINCT`, so each binding of the inputs is one invocation and the
/// declared point bound holds per row of it: a conforming producer answers each with
/// at most one row, the text projects the candidate and the inputs, and two identical
/// rows are one invocation answering twice.
///
/// # The mode the lookup asks
///
/// A lookup, driven or not, is derived only where the relation declares a mode that
/// serves the access pattern it invokes the call in — the candidate, the constants and
/// the driven inputs bound, every other position free
/// ([`lookup_mode_is_declared`]). Otherwise the call does not qualify, by name: a
/// point lookup is never a stand-in for inputs the text drives and the lookup does
/// not.
fn qualifying_lookup(
    source: &ColumnSource<'_>,
    registry: &PropertyFunctionRegistry,
    basis: ExclusionBasis,
) -> Result<LookupText, String> {
    let call = source.call();
    let source_var = source.variable();
    let declaration = registry.ranked_declaration(&call.iri).ok_or_else(|| {
        format!(
            "the registry holds no ranked declaration for <{}>, so nothing declares the {} basis \
             the unit's contract claims",
            call.iri,
            basis.as_str()
        )
    })?;
    if declaration.exclusion != basis {
        return Err(format!(
            "the unit's contract declares an exclusion basis of {} and the registry declares {} \
             for <{}>",
            basis.as_str(),
            declaration.exclusion.as_str(),
            call.iri
        ));
    }
    // The basis was admitted at registration against one position: the declared
    // candidate position, bound, under a mode whose row bound is a point bound. A
    // lookup binding the candidate anywhere else asks a call that admission never
    // saw, so the text must draw its candidate from that very position.
    let arguments: Vec<&TermPattern> = call.subject_args.iter().chain(&call.object_args).collect();
    if !matches!(
        arguments.get(declaration.candidate_position),
        Some(TermPattern::Variable(variable)) if variable == source_var
    ) {
        return Err(format!(
            "its ?{CANDIDATE_NAME} column reads ?{} of the call to <{}>, and the registry declared \
             the {} basis for the candidate at argument position {}, which the text does not fill \
             with that variable",
            source_var.as_str(),
            call.iri,
            basis.as_str(),
            declaration.candidate_position
        ));
    }
    // The depth position, where the producer declared one, is freed whatever the
    // caller wrote there: a depth is an offer of how many rows to rank, and a lookup
    // handed one asks "is this candidate among your best n", whose absences are not
    // exclusions. It is the one constant of the caller's the lookup does not keep —
    // for the reason the rendered lookup renders no number there either
    // (`RenderedQuery::exclusion_text`).
    let depth_position = declaration
        .depth_placement
        .as_ref()
        .map(|placement| placement.position);
    // The inputs the text drives: the driven variables written at some position the
    // lookup keeps — a variable seen only at the freed depth position drives nothing
    // the lookup asks.
    let inputs: Vec<&Variable> = source
        .driven_variables()
        .iter()
        .copied()
        .filter(|variable| {
            arguments.iter().enumerate().any(|(position, term)| {
                depth_position != Some(position) && term_mentions(term, variable)
            })
        })
        .collect();
    lookup_mode_is_declared(call, registry, declaration, source_var, &arguments, &inputs)?;
    if inputs.is_empty() {
        return Ok(LookupText::point(point_lookup(
            call,
            source_var,
            &arguments,
            depth_position,
        )));
    }
    Ok(driven_lookup(
        call,
        source,
        &arguments,
        depth_position,
        &inputs,
    ))
}

/// Whether the lookup of `call` invokes it in a mode its relation declares — or the
/// refusal naming the call, the mode it would be asked in and each position left
/// free.
///
/// The lookup binds the candidate, the constants the text wrote and the `inputs` its
/// driving pattern binds; every other position it leaves free: a variable nothing
/// before the call binds, a blank node, the freed depth. A relation that declares no
/// mode serving that access pattern cannot be asked the lookup at all, so the call
/// does not qualify, and it is named here rather than when the lookup fails to
/// prepare. It is never asked in a mode it did not declare instead.
///
/// Beside the depth, which every lookup frees on purpose (see [`qualifying_lookup`]),
/// this refuses only what the text itself cannot run: the text invokes the call with
/// its inputs bound by the patterns the driving pattern is built from
/// ([`ColumnSource::driving_pattern`]) — the frames before it in its own scope, and
/// through a sub-`SELECT`'s projection the scopes around — so where a declared mode
/// needs an input the lookup leaves free, the planner could not have bound it in the
/// text either.
fn lookup_mode_is_declared(
    call: &PropertyFunctionCall,
    registry: &PropertyFunctionRegistry,
    declaration: &RankedDeclaration,
    candidate: &Variable,
    arguments: &[&TermPattern],
    inputs: &[&Variable],
) -> Result<(), String> {
    let bound_variable = |variable: &Variable| variable == candidate || inputs.contains(&variable);
    // The shape is the declaration's own derivation, the one registration admitted
    // the basis against: what the text supplies is this call's to say, and the
    // candidate bound and the depth freed are the contract's.
    let mode = declaration.exclusion_lookup_mode(arguments.len(), |position| {
        arguments
            .get(position)
            .is_some_and(|term| lookup_binds(term, &bound_variable))
    });
    let depth_position = declaration
        .depth_placement
        .as_ref()
        .map(|placement| placement.position);
    let bound: Vec<bool> = (0..arguments.len())
        .map(|position| mode.is_bound(position))
        .collect();
    let relation = registry.resolve(&call.iri).ok_or_else(|| {
        format!(
            "the registry registers no relation <{}>, so nothing serves its lookup",
            call.iri
        )
    })?;
    let declared =
        purrdf_sparql_eval::property_fn::declaration_contained(&call.iri, "declared modes", || {
            relation.modes().to_vec()
        })
        .map_err(|error| error.to_string())?;
    if declared.iter().any(|declared| declared.subsumes(mode)) {
        return Ok(());
    }
    // The positions some declared mode binds and the lookup leaves free: what the
    // text would have had to drive for any declared mode to serve the lookup.
    let needed: Vec<String> = arguments
        .iter()
        .zip(&bound)
        .enumerate()
        .filter(|(position, (_, bound))| {
            !**bound && declared.iter().any(|declared| declared.is_bound(*position))
        })
        .map(|(position, (term, _))| {
            if depth_position == Some(position) {
                format!("the depth at position {position}, which a lookup always frees")
            } else {
                match term {
                    TermPattern::Variable(variable) => {
                        format!("?{} at position {position}", variable.as_str())
                    }
                    TermPattern::BlankNode(_) => format!("a blank node at position {position}"),
                    _ => format!("a quoted triple not wholly bound at position {position}"),
                }
            }
        })
        .collect();
    Err(format!(
        "its lookup would invoke <{}> as `{}` and the relation declares [{}], none serving it: \
         no pattern the text evaluates before the call binds {}, so no lookup is derived for \
         that call",
        call.iri,
        mode.code(),
        declared
            .iter()
            .map(|mode| mode.code())
            .collect::<Vec<_>>()
            .join(", "),
        needed.join(" or ")
    ))
}

/// Whether the lookup binds `term`'s position: a constant, a variable
/// `bound_variable` holds, or a quoted triple each of whose parts is one of those.
fn lookup_binds(term: &TermPattern, bound_variable: &dyn Fn(&Variable) -> bool) -> bool {
    match term {
        TermPattern::NamedNode(_) | TermPattern::Literal(_) => true,
        TermPattern::BlankNode(_) => false,
        TermPattern::Variable(variable) => bound_variable(variable),
        TermPattern::Triple(triple) => {
            lookup_binds(&triple.subject, bound_variable)
                && match &triple.predicate {
                    NamedNodePattern::NamedNode(_) => true,
                    NamedNodePattern::Variable(variable) => bound_variable(variable),
                }
                && lookup_binds(&triple.object, bound_variable)
        }
    }
}

/// Whether `term` writes `variable`, at its top level or inside a quoted triple.
fn term_mentions(term: &TermPattern, variable: &Variable) -> bool {
    match term {
        TermPattern::Variable(written) => written == variable,
        TermPattern::Triple(triple) => {
            matches!(&triple.predicate, NamedNodePattern::Variable(written) if written == variable)
                || term_mentions(&triple.subject, variable)
                || term_mentions(&triple.object, variable)
        }
        TermPattern::NamedNode(_) | TermPattern::Literal(_) | TermPattern::BlankNode(_) => false,
    }
}

/// The lookup of a call nothing in its text drives: the call alone, one invocation.
fn point_lookup(
    call: &PropertyFunctionCall,
    source: &Variable,
    arguments: &[&TermPattern],
    depth_position: Option<usize>,
) -> String {
    // Each position the candidate's variable fills is the lookup's one parameter.
    // Every other variable and blank node becomes a blank labelled by the first
    // position it fills outside the depth's — so two positions the caller made equal stay equal — which
    // is SPARQL's spelling of "nothing reads this" and what the engine reports to the
    // producer as an unobserved position. Constants are the caller's own terms.
    // A variable or blank node that also occurs inside a quoted-triple argument keeps
    // a name of its own everywhere, so the equality the caller wrote between it and
    // the triple's inside survives; every other one is a top-level blank.
    let mut nested: Vec<(bool, &str)> = Vec::new();
    for term in arguments {
        if let TermPattern::Triple(triple) = term {
            nested_names(triple, &mut nested);
        }
    }
    let mut labels: Vec<(&TermPattern, usize)> = Vec::new();
    let mut rewritten: Vec<TermPattern> = Vec::with_capacity(arguments.len());
    for (position, &term) in arguments.iter().enumerate() {
        rewritten.push(match term {
            TermPattern::Variable(variable) if variable == source => {
                TermPattern::Variable(Variable::new(CANDIDATE_NAME))
            }
            _ if depth_position == Some(position) => {
                TermPattern::BlankNode(BlankNode::new(format!("c{position}")))
            }
            TermPattern::Variable(_) | TermPattern::BlankNode(_)
                if free_name(term).is_some_and(|name| nested.contains(&name)) =>
            {
                kept_name(term)
            }
            TermPattern::Variable(_) | TermPattern::BlankNode(_) => {
                let first = match labels.iter().find(|(seen, _)| *seen == term) {
                    Some(&(_, first)) => first,
                    None => {
                        labels.push((term, position));
                        position
                    }
                };
                TermPattern::BlankNode(BlankNode::new(format!("c{first}")))
            }
            TermPattern::Triple(triple) => {
                TermPattern::Triple(Box::new(nested_rewrite(triple, source)))
            }
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => term.clone(),
        });
    }
    let object_args = rewritten.split_off(call.subject_args.len());
    let subject_args = rewritten;
    let lookup = GraphPattern::Slice {
        start: 0,
        length: Some(usize::try_from(EXCLUSION_LIMIT).unwrap_or(usize::MAX)),
        inner: Box::new(GraphPattern::Project {
            inner: Box::new(GraphPattern::PropertyFunction(PropertyFunctionCall {
                iri: call.iri.clone(),
                subject_args,
                object_args,
            })),
            variables: vec![Variable::new(CANDIDATE_NAME)],
        }),
    };
    pattern_to_select_query(&lookup)
}

/// The lookup of a call its text drives — see [`qualifying_lookup`] for the text and
/// why it is sound.
///
/// Every name is chosen so nothing the lookup writes can meet a name the driving
/// patterns use: the driving patterns sit inside a sub-`SELECT` that exports only the
/// inputs, so beside the inputs the only names the rest of the lookup shares a scope
/// with are its own. The candidate's variable is renamed to a parameter no input and
/// no quoted-triple variable of the call is called; the call's other variables keep
/// their names — an input so, to be joined to the sub-`SELECT`, a variable inside a
/// quoted triple so its equalities survive; and every blank the lookup writes carries
/// a label prefix the driving patterns' text never writes.
fn driven_lookup(
    call: &PropertyFunctionCall,
    source: &ColumnSource<'_>,
    arguments: &[&TermPattern],
    depth_position: Option<usize>,
    inputs: &[&Variable],
) -> LookupText {
    let source_var = source.variable();
    // Built by the shape, which knows how the text evaluates each pattern before the
    // call — which conjuncts are independent and which are evaluated with earlier
    // rows in hand — and rebuilds them the same way.
    let driving = source
        .driving_pattern()
        .cloned()
        .unwrap_or(GraphPattern::Bgp { patterns: vec![] });
    let driving_text = pattern_to_select_query(&driving);
    let mut nested: Vec<(bool, &str)> = Vec::new();
    for term in arguments {
        if let TermPattern::Triple(triple) = term {
            nested_names(triple, &mut nested);
        }
    }
    // The parameter shares its scope with the inputs and the call's quoted-triple
    // variables only; the blank labels with the driving patterns' text, which a
    // sub-`SELECT` does not hide from a blank label.
    let mut parameter = CANDIDATE_NAME.to_owned();
    while inputs.iter().any(|input| input.as_str() == parameter)
        || nested.contains(&(false, parameter.as_str()))
    {
        parameter.push('_');
    }
    let unwritten = |base: &str| {
        let mut prefix = base.to_owned();
        while driving_text.contains(&format!("_:{prefix}")) {
            prefix.push('_');
        }
        prefix
    };
    // Distinct first characters, so no label under one prefix is one under the other.
    let blank = unwritten("c");
    let kept_blank = unwritten("b_");
    let rename = |term: &TermPattern| -> TermPattern {
        match term {
            TermPattern::BlankNode(label) => {
                TermPattern::BlankNode(BlankNode::new(format!("{kept_blank}{}", label.as_str())))
            }
            other => other.clone(),
        }
    };
    let mut labels: Vec<(&TermPattern, usize)> = Vec::new();
    let mut rewritten: Vec<TermPattern> = Vec::with_capacity(arguments.len());
    for (position, &term) in arguments.iter().enumerate() {
        rewritten.push(match term {
            TermPattern::Variable(variable) if variable == source_var => {
                TermPattern::Variable(Variable::new(parameter.as_str()))
            }
            _ if depth_position == Some(position) => {
                TermPattern::BlankNode(BlankNode::new(format!("{blank}{position}")))
            }
            TermPattern::Variable(variable) if inputs.contains(&variable) => term.clone(),
            TermPattern::Variable(_) | TermPattern::BlankNode(_)
                if free_name(term).is_some_and(|name| nested.contains(&name)) =>
            {
                rename(term)
            }
            TermPattern::Variable(_) | TermPattern::BlankNode(_) => {
                let first = match labels.iter().find(|(seen, _)| *seen == term) {
                    Some(&(_, first)) => first,
                    None => {
                        labels.push((term, position));
                        position
                    }
                };
                TermPattern::BlankNode(BlankNode::new(format!("{blank}{first}")))
            }
            TermPattern::Triple(triple) => TermPattern::Triple(Box::new(driven_nested_rewrite(
                triple,
                source_var,
                &parameter,
                &kept_blank,
            ))),
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => term.clone(),
        });
    }
    let object_args = rewritten.split_off(call.subject_args.len());
    let subject_args = rewritten;
    let input_vars: Vec<Variable> = inputs.iter().map(|input| (*input).clone()).collect();
    let projected = GraphPattern::Project {
        // The sub-`SELECT` and then the call, written in one group as a caller
        // writes a needle pattern before its call: the planner orders the group's
        // atoms and drives the call with the sub-`SELECT`'s rows — the parameter
        // counted bound there too, which a `LATERAL` block's inside is not.
        inner: Box::new(GraphPattern::Join {
            left: Box::new(GraphPattern::Distinct {
                inner: Box::new(GraphPattern::Project {
                    inner: Box::new(driving),
                    variables: input_vars.clone(),
                }),
            }),
            right: Box::new(GraphPattern::Lateral {
                left: Box::new(GraphPattern::Bgp { patterns: vec![] }),
                right: Box::new(GraphPattern::PropertyFunction(PropertyFunctionCall {
                    iri: call.iri.clone(),
                    subject_args,
                    object_args,
                })),
            }),
        }),
        variables: std::iter::once(Variable::new(parameter.as_str()))
            .chain(input_vars)
            .collect(),
    };
    // Written as the query's own top-level `SELECT`, where the prepare's parameter
    // pushdown reaches the call, rather than as a sub-`SELECT` of a `SELECT *` — a
    // scope a parameter is not pushed into, which would leave the call's candidate
    // position free when its modes are checked. A slice with no length and no offset
    // is the writer's spelling of exactly that, and bounds nothing: the lookup is read
    // whole.
    let lookup = GraphPattern::Slice {
        start: 0,
        length: None,
        inner: Box::new(projected),
    };
    LookupText {
        text: with_dataset(pattern_to_select_query(&lookup), source.dataset()),
        parameter,
        driven: true,
    }
}

/// `text`, a `SELECT` this module wrote, reading `dataset`: the clause written between
/// its projection and its `WHERE`, where the grammar puts it.
///
/// A driven lookup reads its driving patterns out of the data, and the text read them
/// under its own dataset clause: read under any other dataset they bind other inputs.
/// The call itself is handed no graph, so an undriven lookup needs none. The
/// projection this module writes is variables alone, so the first ` WHERE {` is the
/// query's own.
fn with_dataset(text: String, dataset: &QueryDataset) -> String {
    if dataset.default.is_empty() && dataset.named.is_empty() {
        return text;
    }
    text.replacen(" WHERE {", &format!(" {dataset}WHERE {{"), 1)
}

/// `triple` as a driven lookup writes it: the candidate's variable is the parameter
/// wherever it occurs, every other variable keeps its name, and every blank node its
/// label behind `kept_blank`.
fn driven_nested_rewrite(
    triple: &TriplePattern,
    source: &Variable,
    parameter: &str,
    kept_blank: &str,
) -> TriplePattern {
    let term = |term: &TermPattern| match term {
        TermPattern::Variable(variable) if variable == source => {
            TermPattern::Variable(Variable::new(parameter))
        }
        TermPattern::Triple(inner) => TermPattern::Triple(Box::new(driven_nested_rewrite(
            inner, source, parameter, kept_blank,
        ))),
        TermPattern::BlankNode(label) => {
            TermPattern::BlankNode(BlankNode::new(format!("{kept_blank}{}", label.as_str())))
        }
        other => other.clone(),
    };
    TriplePattern {
        subject: term(&triple.subject),
        predicate: match &triple.predicate {
            NamedNodePattern::Variable(variable) if variable == source => {
                NamedNodePattern::Variable(Variable::new(parameter))
            }
            other => other.clone(),
        },
        object: term(&triple.object),
    }
}

/// Every variable and blank node written inside the quoted triple `triple`, its
/// nested triples and its predicate included, recorded once each by its kind (`true`
/// for a blank node) and its name.
fn nested_names<'q>(triple: &'q TriplePattern, names: &mut Vec<(bool, &'q str)>) {
    let mut record = |name: (bool, &'q str)| {
        if !names.contains(&name) {
            names.push(name);
        }
    };
    if let NamedNodePattern::Variable(variable) = &triple.predicate {
        record((false, variable.as_str()));
    }
    for term in [&triple.subject, &triple.object] {
        match term {
            TermPattern::Variable(variable) => record((false, variable.as_str())),
            TermPattern::BlankNode(blank) => record((true, blank.as_str())),
            TermPattern::Triple(_) | TermPattern::NamedNode(_) | TermPattern::Literal(_) => {}
        }
    }
    for term in [&triple.subject, &triple.object] {
        if let TermPattern::Triple(inner) = term {
            nested_names(inner, names);
        }
    }
}

/// The kind (`true` for a blank node) and name of a variable or blank-node term.
fn free_name(term: &TermPattern) -> Option<(bool, &str)> {
    match term {
        TermPattern::Variable(variable) => Some((false, variable.as_str())),
        TermPattern::BlankNode(blank) => Some((true, blank.as_str())),
        TermPattern::NamedNode(_) | TermPattern::Literal(_) | TermPattern::Triple(_) => None,
    }
}

/// The name a variable or blank node that occurs inside a quoted-triple argument
/// keeps in a lookup: its own, prefixed, so it can collide neither with the lookup's
/// `?candidate` nor with the `_:c{n}` blanks the lookup writes for its other free
/// positions.
fn kept_name(term: &TermPattern) -> TermPattern {
    match term {
        TermPattern::Variable(variable) => {
            TermPattern::Variable(Variable::new(format!("v_{}", variable.as_str())))
        }
        TermPattern::BlankNode(blank) => {
            TermPattern::BlankNode(BlankNode::new(format!("b_{}", blank.as_str())))
        }
        TermPattern::NamedNode(_) | TermPattern::Literal(_) | TermPattern::Triple(_) => {
            term.clone()
        }
    }
}

/// `triple` as a lookup writes it: the candidate's variable is the lookup's
/// parameter wherever it occurs, and every other variable and blank node keeps its
/// own prefixed name ([`kept_name`]) so every equality the caller wrote survives.
fn nested_rewrite(triple: &TriplePattern, source: &Variable) -> TriplePattern {
    let term = |term: &TermPattern| match term {
        TermPattern::Variable(variable) if variable == source => {
            TermPattern::Variable(Variable::new(CANDIDATE_NAME))
        }
        TermPattern::Triple(inner) => TermPattern::Triple(Box::new(nested_rewrite(inner, source))),
        other => kept_name(other),
    };
    TriplePattern {
        subject: term(&triple.subject),
        predicate: match &triple.predicate {
            NamedNodePattern::Variable(variable) if variable == source => {
                NamedNodePattern::Variable(Variable::new(CANDIDATE_NAME))
            }
            NamedNodePattern::Variable(variable) => {
                NamedNodePattern::Variable(Variable::new(format!("v_{}", variable.as_str())))
            }
            NamedNodePattern::NamedNode(node) => NamedNodePattern::NamedNode(node.clone()),
        },
        object: term(&triple.object),
    }
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
/// emission, where [`place`](crate::matching::place) needs the value to occupy the
/// argument slot and derive the access mode it satisfies, and once per read of the
/// unit's text, where [`RenderedQuery::text`] renders it into that slot. Same
/// function, same two arguments, both of them facts the unit holds.
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
        //
        // The refusal is about the STREAMING unit, and only about it. Binding the
        // candidate is not forbidden as such — the exclusion lookup
        // ([`RenderedQuery::exclusion_text`]) binds it deliberately, which is the
        // whole of what makes it a lookup rather than a scan. What a streaming
        // unit cannot do is project a ranking out of a position the invocation
        // already filled with a request value: there would be no column left to
        // read the candidate back from. The registry's own refusal at
        // registration is the narrower one and stays narrower — it forbids the
        // candidate position as a *placement* target, which is a statement about
        // the declaration rather than about any one invocation.
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
/// [`place`](crate::matching::place) proves every slot it fills renders, so reaching
/// this means the registry moved between the two calls; it is reported on the same
/// dimension because it is the same claim.
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_sparql_eval::{
        AcceptedTerm, BindingPattern, CandidateDomains, DepthPlacement, DuplicatePolicy, EvalError,
        ExclusionBasis, ExtensionEnv, NativeSparqlEngine, PfArgs, PfArity, PfCursor,
        PropertyFunction, PropertyFunctionRegistry, QueryOptions, RankFidelity, RankedDeclaration,
        RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
    };

    use super::{
        CANDIDATE_NAME, LookupText, RenderedQuery, UnitArgument, supplied_exclusion_texts,
    };
    use crate::admission::ProbedDepth;

    const NEIGHBOURS: &str = "https://example.org/pf/neighbours";
    const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

    /// A self-bounding producer's declaration surface, and nothing behind it: the
    /// candidate at 0, a seed at 1, the depth at 2, a score at 3. Never opened — the
    /// tests below read texts and prepare plans, and run nothing.
    struct Neighbours {
        modes: Vec<BindingPattern>,
    }

    impl PropertyFunction for Neighbours {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            PfArity::new(1, 3)
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
            if mode.is_bound(0) { 1 } else { 64 }
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            Err(EvalError::function("never opened by these tests"))
        }
    }

    fn neighbours_registry(exclusion: ExclusionBasis) -> PropertyFunctionRegistry {
        neighbours_declaring(
            exclusion,
            vec![
                BindingPattern::from_bound_positions(4, [1, 2]),
                BindingPattern::from_bound_positions(4, [0, 1]),
                BindingPattern::from_bound_positions(4, [2]),
                BindingPattern::from_bound_positions(4, [0]),
            ],
        )
    }

    /// [`neighbours_registry`], the relation declaring `modes`.
    fn neighbours_declaring(
        exclusion: ExclusionBasis,
        modes: Vec<BindingPattern>,
    ) -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            NEIGHBOURS,
            Arc::new(Neighbours { modes }),
            RankedDeclaration {
                stratum: purrdf_core::parse_iri("https://example.org/stratum/near")
                    .expect("a fixture IRI"),
                accepted_terms: vec![AcceptedTerm {
                    pattern: TermPattern {
                        kind: TermKind::Iri,
                        datatype: None,
                        language: None,
                        predicate: None,
                    },
                    placements: vec![TermPlacement {
                        facet: RequestFacet::Value,
                        position: 1,
                        datatype: None,
                    }],
                }],
                depth_placement: Some(DepthPlacement {
                    position: 2,
                    datatype: INTEGER.to_owned(),
                }),
                candidate_position: 0,
                duplicates: DuplicatePolicy::Unique,
                fidelity: RankFidelity::EXACT,
                domains: CandidateDomains::Unrestricted,
                block_position: None,
                exclusion,
                mandatory: false,
            },
        );
        registry
    }

    /// The lookup a caller's `text` derives under `registry`, and whether that lookup
    /// prepares with the candidate as its one parameter.
    fn supplied_lookup(text: &str, registry: &PropertyFunctionRegistry) -> Result<String, String> {
        let env = ExtensionEnv::over_relations(registry.clone()).expect("the fixture env");
        let options = || QueryOptions {
            env: &env,
            ..QueryOptions::EMPTY
        };
        let engine = NativeSparqlEngine::new();
        let prepared = engine
            .prepare_query_with_options(text, None, options())
            .expect("the caller's text prepares");
        let lookups = supplied_exclusion_texts(&prepared, registry, ExclusionBasis::Membership)?;
        let [lookup] = <[LookupText; 1]>::try_from(lookups.concat())
            .unwrap_or_else(|lookups| panic!("one call, one lookup: {lookups:?}"));
        assert_eq!(lookup.parameter, CANDIDATE_NAME);
        assert!(!lookup.driven, "nothing in the text drives the call");
        engine
            .prepare_execution(&lookup.text, None, &[CANDIDATE_NAME], options())
            .unwrap_or_else(|error| {
                panic!("the derived lookup prepares: {error}\n{}", lookup.text)
            });
        Ok(lookup.text)
    }

    /// **A caller's one-call text derives the lookup the rendered unit would: the
    /// candidate a parameter, the depth and every other free position a blank, the
    /// caller's other constants kept.**
    ///
    /// The caller wrote a depth of five into the call, through a renaming nested
    /// `SELECT` with a `LIMIT`. Keeping that constant would ask *is this candidate
    /// among your best five*, whose absences are not exclusions, so the derived lookup
    /// frees it; the seed is the caller's and stays. The neighbour — the candidate
    /// read from a position the registry did not declare the basis at — is refused
    /// with the position named.
    #[test]
    fn a_supplied_call_derives_the_rendered_lookup_and_frees_its_depth() {
        let registry = neighbours_registry(ExclusionBasis::Membership);
        let lookup = supplied_lookup(
            &format!(
                "SELECT ?candidate WHERE {{ {{ SELECT (?hit AS ?candidate) WHERE {{ ?hit \
                 <{NEIGHBOURS}> ( <https://example.org/d/seed> \"5\"^^<{INTEGER}> ?score ) }} \
                 LIMIT 3 }} }}"
            ),
            &registry,
        )
        .expect("a one-call text derives its lookup");
        assert!(
            lookup.contains(&format!("?{CANDIDATE_NAME} <{NEIGHBOURS}>")),
            "the candidate is the call's candidate position, as the parameter: {lookup}"
        );
        assert!(
            lookup.contains("<https://example.org/d/seed>"),
            "the caller's seed is kept: {lookup}"
        );
        assert!(
            !lookup.contains(INTEGER) && lookup.contains("_:c2") && lookup.contains("_:c3"),
            "the depth and the score are free blanks, no number in the depth position: \
             {lookup}"
        );
        assert!(
            !lookup
                .replace(&format!("?{CANDIDATE_NAME}"), "")
                .contains('?'),
            "no variable but the candidate: {lookup}"
        );
        assert!(
            lookup.ends_with("LIMIT 2"),
            "read under the lookup's own ceiling, not the caller's: {lookup}"
        );

        let refused = supplied_lookup(
            &format!(
                "SELECT ?candidate WHERE {{ ?c <{NEIGHBOURS}> ( <https://example.org/d/seed> \
                 \"5\"^^<{INTEGER}> ?candidate ) }}"
            ),
            &registry,
        )
        .expect_err("the candidate is read from the score position");
        assert!(
            refused.contains("argument position 0"),
            "naming the position the basis was declared at: {refused}"
        );
    }

    /// **Inside a quoted-triple argument the candidate is the parameter too, and every
    /// equality the caller wrote survives.**
    ///
    /// The caller's seed is a triple term naming the candidate and a variable `?w`
    /// that is also the score position. A lookup that bound only the top-level
    /// candidate, or blanked the score while the triple kept `?w`, would ask a looser
    /// question than the caller's call; the derived lookup binds the candidate at both
    /// places and keeps `?w` one variable at both.
    #[test]
    fn a_quoted_triple_argument_keeps_the_callers_equalities() {
        let registry = neighbours_registry(ExclusionBasis::Membership);
        let lookup = supplied_lookup(
            &format!(
                "SELECT ?candidate WHERE {{ ?candidate <{NEIGHBOURS}> ( <<( ?candidate \
                 <https://example.org/p> ?w )>> \"5\"^^<{INTEGER}> ?w ) }}"
            ),
            &registry,
        )
        .expect("a one-call text derives its lookup");
        assert!(
            lookup.contains(&format!(
                "<<( ?{CANDIDATE_NAME} <https://example.org/p> ?v_w )>> _:c2 ?v_w )"
            )),
            "the candidate bound inside the triple, the depth freed, ?w one variable: \
             {lookup}"
        );
    }

    /// **A call whose seed the text reads out of the data keeps the pattern that binds
    /// it: a `DISTINCT` sub-`SELECT` exporting the seed, the call invoked with the
    /// candidate and the seed bound, the depth and the score free, and no row
    /// ceiling.**
    ///
    /// A lookup that blanked the seed would ask the producer in a mode it never
    /// declared — the candidate alone bound beside a free seed is one of its modes
    /// here only because the fixture declares `[0]`, so the prepared lookup is also
    /// checked to be the seed-bound question. The data triple unrelated to the call
    /// is not kept.
    #[test]
    fn a_driven_call_keeps_the_pattern_binding_its_seed() {
        let registry = neighbours_registry(ExclusionBasis::Membership);
        let env = ExtensionEnv::over_relations(registry.clone()).expect("the fixture env");
        let options = || QueryOptions {
            env: &env,
            ..QueryOptions::EMPTY
        };
        let engine = NativeSparqlEngine::new();
        let prepared = engine
            .prepare_query_with_options(
                &format!(
                    "SELECT ?candidate WHERE {{ <https://example.org/d/cfg> \
                     <https://example.org/d/seedOf> ?seed . ?other <https://example.org/d/p> ?o . \
                     ?candidate <{NEIGHBOURS}> ( ?seed \"5\"^^<{INTEGER}> ?score ) }}"
                ),
                None,
                options(),
            )
            .expect("the caller's text prepares");
        let lookups = supplied_exclusion_texts(&prepared, &registry, ExclusionBasis::Membership)
            .expect("the driven call derives its lookup");
        let [lookup] = <[LookupText; 1]>::try_from(lookups.concat())
            .unwrap_or_else(|lookups| panic!("one call, one lookup: {lookups:?}"));
        assert!(lookup.driven, "the seed drives the call");
        assert_eq!(lookup.parameter, CANDIDATE_NAME);
        let text = &lookup.text;
        assert!(
            text.contains("SELECT DISTINCT ?seed")
                && text.contains("<https://example.org/d/seedOf>"),
            "the seed's pattern, distinct, exporting the seed: {text}"
        );
        assert!(
            !text.contains("<https://example.org/d/p>"),
            "the unrelated pattern is not kept: {text}"
        );
        assert!(
            text.contains(&format!(
                "?{CANDIDATE_NAME} <{NEIGHBOURS}> ( ?seed _:c2 _:c3 )"
            )),
            "candidate and seed bound, depth and score free: {text}"
        );
        assert!(
            !text.contains("LIMIT"),
            "read whole, one row per seed: {text}"
        );
        engine
            .prepare_execution(text, None, &[CANDIDATE_NAME], options())
            .unwrap_or_else(|error| panic!("the driven lookup prepares: {error}\n{text}"));
    }

    /// A four-position call shaped like the two shipped vector producers': the candidate
    /// at 0, a constant seed at 1, the producer's depth at 2, and a free projection at 3.
    fn self_bounding() -> RenderedQuery {
        RenderedQuery {
            producer: "https://example.org/pf/neighbours".to_owned(),
            subject: vec![UnitArgument::Free],
            object: vec![
                UnitArgument::Placed("<https://example.org/d/seed>".to_owned()),
                UnitArgument::Depth {
                    datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                },
                UnitArgument::Free,
            ],
            candidate: 0,
            block: None,
        }
    }

    /// **The streaming text renders the depth and the exclusion text does not.**
    ///
    /// The pair is asserted together because either half alone is satisfied by a defect
    /// the other catches. A lookup that carried a depth would ask the producer *is this
    /// candidate among your best n*, whose absences are not exclusions; a streaming read
    /// that stopped carrying one would stop bounding itself. The two texts are the two
    /// questions, and the depth is the whole of what distinguishes them.
    #[test]
    fn only_the_streaming_text_renders_a_depth() {
        let rendered = self_bounding();
        let depth = ProbedDepth::checked(7).expect("seven is a probeable depth");
        let streaming = rendered.text(depth, Some(64));
        assert!(
            streaming.contains("\"8\"^^<http://www.w3.org/2001/XMLSchema#integer>"),
            "the streaming read is handed the depth plus its probe row: {streaming}"
        );

        let lookup = rendered.exclusion_text();
        assert!(
            !lookup.contains("XMLSchema#integer"),
            "no number belongs in the depth position of a lookup: {lookup}"
        );
        assert!(
            lookup.contains("_:c2") && lookup.contains("_:c3"),
            "the depth position is left free, as the blank node every free position of a \
             lookup other than the candidate's is written as: {lookup}"
        );
        assert!(
            !lookup
                .replace(&format!("?{CANDIDATE_NAME}"), "")
                .contains('?'),
            "a lookup names no variable but the candidate, so nothing it does not read is \
             reported observed to the producer: {lookup}"
        );
        assert!(
            streaming.contains("( ?c0 )") && streaming.contains("?c3"),
            "while the streaming read keeps its free positions variables, because it projects \
             the candidate out of one: {streaming}"
        );
        assert!(
            lookup.contains(&format!("?{CANDIDATE_NAME}")),
            "and the candidate is the one variable a caller binds per lookup: {lookup}"
        );
    }

    /// **A call whose input nothing in the text binds, under a relation that cannot be
    /// asked without it, derives no lookup and is refused by name — and only where the
    /// text itself cannot bind that input either.**
    ///
    /// The relation declares only modes binding the seed: `bbff` and `fbff`. The text
    /// `?candidate <neighbours> ( ?seed ?depth ?score )` binds `?seed` nowhere, and
    /// against that relation it does not even prepare — the planner finds no order that
    /// binds the seed, so no run of the text invokes the call. The text prepared
    /// against a relation that also serves the free mode is handed the seed-bound
    /// relation's declarations: its lookup would ask `bfff`, which that relation does
    /// not declare, and the derivation says so — the call, the mode, `?seed` at
    /// position 1, the declared modes — rather than deriving a point lookup that
    /// fails to prepare, or one asking a mode nothing declared.
    ///
    /// The neighbours: the same text under a relation that also declares the all-free
    /// mode, which serves `bfff`, derives its point lookup; and the text binding its
    /// seed from
    /// `VALUES` before the call, under the seed-bound relation, derives a driven
    /// lookup that prepares.
    #[test]
    fn a_call_whose_input_nothing_binds_is_refused_by_name_where_its_relation_needs_it() {
        let seed_bound = || {
            vec![
                BindingPattern::from_bound_positions(4, [0, 1]),
                BindingPattern::from_bound_positions(4, [1]),
            ]
        };
        let strict = neighbours_declaring(ExclusionBasis::Membership, seed_bound());
        let wide = neighbours_declaring(
            ExclusionBasis::Membership,
            std::iter::once(BindingPattern::from_bound_positions(4, []))
                .chain(seed_bound())
                .collect(),
        );
        let text = format!(
            "SELECT ?candidate WHERE {{ ?candidate <{NEIGHBOURS}> ( ?seed ?depth ?score ) }}"
        );
        let engine = NativeSparqlEngine::new();
        let prepared_under = |registry: &PropertyFunctionRegistry, text: &str| {
            let env = ExtensionEnv::over_relations(registry.clone()).expect("the fixture env");
            engine
                .prepare_query_with_options(
                    text,
                    None,
                    QueryOptions {
                        env: &env,
                        ..QueryOptions::EMPTY
                    },
                )
                .map_err(|error| error.to_string())
        };

        let unplanned = prepared_under(&strict, &text)
            .expect_err("no order binds the seed, so the text itself cannot run");
        assert!(
            unplanned.contains("no feasible evaluation order"),
            "the planner's own refusal: {unplanned}"
        );

        let prepared = prepared_under(&wide, &text).expect("the free mode serves the text");
        let refused = supplied_exclusion_texts(&prepared, &strict, ExclusionBasis::Membership)
            .expect_err("no declared mode serves the lookup");
        assert!(
            refused.contains(&format!(
                "<{NEIGHBOURS}> as `bfff` and the relation declares [bbff, fbff], none serving \
                 it: no pattern the text evaluates before the call binds ?seed at position 1,"
            )),
            "the call, the mode, the undriven input and the declared modes named: {refused}"
        );

        let lookups = supplied_exclusion_texts(&prepared, &wide, ExclusionBasis::Membership)
            .expect("the free-seed mode the wide relation declares serves a point lookup");
        let [lookup] = <[LookupText; 1]>::try_from(lookups.concat())
            .unwrap_or_else(|lookups| panic!("one call, one lookup: {lookups:?}"));
        assert!(!lookup.driven, "nothing drives the call");

        let driven_text = format!(
            "SELECT ?candidate WHERE {{ VALUES ?seed {{ <https://example.org/d/seed> }} \
             ?candidate <{NEIGHBOURS}> ( ?seed ?depth ?score ) }}"
        );
        let prepared =
            prepared_under(&strict, &driven_text).expect("the VALUES row binds the seed");
        let lookups = supplied_exclusion_texts(&prepared, &strict, ExclusionBasis::Membership)
            .expect("the driven lookup asks `bbff`, which the relation declares");
        let [lookup] = <[LookupText; 1]>::try_from(lookups.concat())
            .unwrap_or_else(|lookups| panic!("one call, one lookup: {lookups:?}"));
        assert!(lookup.driven, "the VALUES row drives the call");
        let env = ExtensionEnv::over_relations(strict).expect("the fixture env");
        engine
            .prepare_execution(
                &lookup.text,
                None,
                &[CANDIDATE_NAME],
                QueryOptions {
                    env: &env,
                    ..QueryOptions::EMPTY
                },
            )
            .unwrap_or_else(|error| panic!("the driven lookup prepares: {error}\n{}", lookup.text));
    }
}
