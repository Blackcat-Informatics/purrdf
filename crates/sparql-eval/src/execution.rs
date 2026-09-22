// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A **prepared, parameterized execution**: prepare once, bind and run many times.
//!
//! [`PlanCache`](crate::engine::PlanCache)'s own documentation already tells callers
//! to *"pass changing data as substitutions to a prepared plan instead of splicing it
//! into query text"* — because splicing a value into text mints a new query, and a
//! new query misses the plan cache, is re-parsed, and is re-admitted. Until now there
//! was no object to follow that advice with: every entry point took the query as a
//! `&str`, so a caller running one query per row paid a cache probe per row to be
//! handed back the same plan each time.
//!
//! A [`PreparedExecution`] is that object. It holds the admitted plan, so the text is
//! parsed and admitted once; it holds its parameters as already-interned
//! [`Variable`]s, so a name is never rebuilt from a borrow; and it holds their
//! current values in a vector it overwrites, so binding a new row writes cells rather
//! than building a list.
//!
//! # Why `&mut self` on the run, and why that is a guarantee rather than a limitation
//!
//! [`NativeSparqlEngine::execute`](crate::NativeSparqlEngine::execute) takes the
//! execution by unique reference. That is deliberate and load-bearing: it makes an
//! execution that is already in flight **impossible to reach again**, as a
//! borrow-check error rather than as a runtime hazard.
//!
//! The hazard is real and not hypothetical. A SHACL-AF function body can be a query
//! whose evaluation re-enters validation and reaches the same evaluator — the shapes
//! crate keeps a call-depth counter precisely because "a recursion can LEAVE this
//! evaluator and come back". A handle reachable from an ambient cache would hand that
//! inner call the very tree the outer call is mid-evaluation over. `&mut` means the
//! compiler refuses that program.
//!
//! The consequence for callers is that a handle belongs to one worker at a time. That
//! is the natural shape anyway: the engine it was prepared against is itself `!Sync`.

use std::sync::Arc;

use purrdf_core::{DatasetView, RdfDiagnostic, TermValue};
use purrdf_sparql_algebra::{GroundTerm, Query, Variable};

use crate::engine::{PreparedQuery, ShaclPrebinding};
use crate::prebind_memo::{PrebindMemo, ValueShape};
use crate::substitute::ParameterValue;

/// Per-thread switch for `PreparedExecution::substituted`'s debug-only
/// differential oracle (see the `#[cfg(debug_assertions)]` block inside it).
///
/// # Why this exists
///
/// The oracle re-runs the full, un-memoized rewrite on every memo hit and compares
/// it against the memo's answer, so its own cost — a whole clone-and-walk of the
/// admitted algebra — IS the allocation the memo exists to remove. In an ordinary
/// debug test that is the right trade: the check turns every existing fixture that
/// runs a prepared execution twice with matching value shapes into a differential
/// test, for the price of allocations nobody there is counting. But an allocation
/// harness that counts them (`crates/shapes/tests/sparql_path_alloc.rs`) is a debug
/// build too — `cargo test` always compiles with `debug_assertions` on — so without
/// a way to turn the check off, the harness would measure the oracle's cost instead
/// of the memo's, and the memo's saving would be structurally invisible to the one
/// instrument built to see it.
///
/// # Scope, and what turning it off does NOT do
///
/// This is a thread-local, not a process-global: setting it on one thread leaves
/// every other thread's oracle running. Disabling it narrows the oracle to exactly
/// the window it is off for, on exactly the threads it was set on, and nothing
/// else — every run outside that window, and every run on a thread that never
/// called [`set_memo_verification_enabled`], is still checked. It changes nothing
/// about what a memo hit ANSWERS, only whether that answer is re-derived and
/// compared; the memo itself is unconditional and identical in both states.
///
/// SHACL validation fans focus nodes over `rayon`'s worker pool, so a caller
/// measuring that path must set this on every worker, not on the calling thread —
/// `rayon::broadcast` is the tool for that, and
/// `crates/shapes/tests/sparql_path_alloc.rs` already carries one for warming the
/// per-worker plan cache; the same mechanism sets this flag.
///
/// # Debug-only
///
/// `#[cfg(debug_assertions)]`, matching the oracle it controls: in a release build
/// neither the flag, this function, nor the check it gates exists, so the switch
/// costs nothing there and cannot be mistaken for a knob a production caller might
/// reach for.
///
/// # `pub`, not `pub(crate)`
///
/// Nothing in this crate's own `src/` calls this function; every caller is a test,
/// and at least one of them cannot be granted access any narrower than `pub` at
/// all. `crates/shapes/tests/sparql_path_alloc.rs` calls it directly
/// (`purrdf_sparql_eval::set_memo_verification_enabled(false)`, broadcast to every
/// worker) — that file belongs to the `purrdf-shapes` PACKAGE, a different crate
/// entirely, so `pub(crate)` here would refuse it outright, not merely discourage
/// it. This crate's own `tests/prepared_execution.rs` also calls it, and even
/// though that file ships in the SAME package, `cargo` still compiles every file
/// under `tests/` as an independent crate linking this one as an external
/// dependency — `pub(crate)` does not reach across that boundary either, so this
/// function would be unreachable from its closest caller too. `pub` is therefore
/// the minimum visibility either caller can compile against, not a looser grant
/// made for convenience.
#[cfg(debug_assertions)]
pub fn set_memo_verification_enabled(enabled: bool) {
    MEMO_VERIFICATION_ENABLED.with(|flag| flag.set(enabled));
}

#[cfg(debug_assertions)]
std::thread_local! {
    /// Backing storage for [`set_memo_verification_enabled`]. Defaults to `true` on
    /// every thread, so a thread this crate's caller never touches — every ordinary
    /// test, and every worker the allocation harness does not broadcast to — keeps
    /// the oracle on.
    static MEMO_VERIFICATION_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Whether the calling thread currently runs `PreparedExecution::substituted`'s
/// differential oracle. `pub(crate)` rather than re-exported: a caller sets the
/// switch through [`set_memo_verification_enabled`] and has no need to read it
/// back.
#[cfg(debug_assertions)]
fn memo_verification_enabled() -> bool {
    MEMO_VERIFICATION_ENABLED.with(std::cell::Cell::get)
}

/// A plan prepared once, with named parameters bound and re-bound per execution.
///
/// Build one with
/// [`NativeSparqlEngine::prepare_execution`](crate::NativeSparqlEngine::prepare_execution)
/// and run it with
/// [`NativeSparqlEngine::execute`](crate::NativeSparqlEngine::execute).
#[derive(Debug)]
pub struct PreparedExecution {
    /// The admitted plan. Held by `Arc`, so it stays valid for this execution's
    /// lifetime even if the engine's cache evicts it.
    pub(crate) prepared: Arc<PreparedQuery>,
    /// The declared parameters, interned once, in declaration order.
    pub(crate) parameters: Box<[Variable]>,
    /// The current value of each parameter, positionally. `None` until bound;
    /// running with any parameter still `None` is refused rather than defaulted.
    ///
    /// A [`ParameterValue`] rather than a bare [`TermValue`] because a slot may have
    /// been filled through either door — see that type for what each one holds and
    /// why the id door stores a resolved term rather than the id it was given.
    pub(crate) values: Vec<Option<ParameterValue>>,
    /// This run's bindings as the rewrite consumes them, in a buffer this execution
    /// keeps. The list has the same length on every run and only its cells change,
    /// so refilling it costs nothing where a fresh `Vec` cost one allocation per run.
    probes: Vec<(Variable, GroundTerm)>,
    /// The substituted algebra, once one has been built and checked — see
    /// [`crate::prebind_memo`].
    memo: Option<PrebindMemo>,
    /// The lane and value shapes of the most recent run that did not come from
    /// [`Self::memo`].
    pending: Option<PendingShape>,
    /// The per-run tables this execution retains between runs, emptied but not
    /// given back. See [`ExecutionWorkspace`].
    workspace: ExecutionWorkspace,
}

/// The evaluation tables a prepared execution keeps across its runs: **emptied
/// between runs, but not given back to the allocator**.
///
/// # What this is for
///
/// [`crate::eval::EvalCtx`] is built fresh for every run and dropped at the end of
/// it, and everything lazy on it therefore grows from zero again on the next run.
/// A handle that runs the same query once per focus node pays that growth once per
/// focus node for tables whose SIZE is a property of the query rather than of the
/// focus node. Holding them on the handle — which already outlives the run — and
/// clearing them instead of dropping them keeps the capacity and pays the growth
/// once.
///
/// # Why it holds ONE table and not seven
///
/// [`crate::eval::EvalCtx`] carries seven lazy per-evaluation maps beside the
/// scratch interner — the `BNODE(strExpr)` memo, three `EXISTS` caches, the regex
/// cache, the constant-atom cache and the XSD parse cache. Every one of them was
/// measured before this type was written, and every one of them allocates
/// **nothing** on the paths this workspace exists for: a `DetHashMap` is a
/// `HashMap::default()`, which builds no table until its first insert, and none of
/// those seven takes an insert on a query that does not use the feature it
/// memoizes. Pre-reserving all seven adds exactly seven allocations per evaluation
/// context to `crates/sparql-eval/tests/prepared_execution.rs`'s pin and ten to
/// `crates/shapes/tests/sparql_path_alloc.rs`'s per-focus-node figures — seven for
/// the tables themselves and three more where a forked `FILTER` worker clones the
/// three `EXISTS` caches, which is free while they are empty and is one allocation
/// each once they are not.
///
/// So retaining them would buy nothing and cost a clear. They are deliberately
/// absent, and the reason is a measurement rather than an opinion. A later change
/// that gives one of them a per-run cost belongs here, and the clear below is
/// written so that adding it cannot be done without clearing it.
///
/// # Correctness
///
/// Everything here is FOCUS-NODE-DEPENDENT and must be cleared between runs — the
/// scratch interner holds the values one run computed, and a `SolutionTerm::Computed`
/// id is an index into it, so a table carried forward uncleared would answer one
/// run's id with another run's value. That is a silently wrong answer, which is why
/// [`crate::scratch::ScratchInterner::clear`] is written as a destructuring `let`
/// over every field with no rest pattern: a field added and not cleared fails to
/// compile.
///
/// Nothing here memoizes a pure function of the query, which is the only category
/// that could legitimately be RETAINED rather than cleared. The plan-shaped memos
/// this execution keeps — [`PreparedExecution::memo`] and the interned parameter
/// names — are exactly that category and live outside this type, because their
/// lifecycle is "build once and keep", not "clear every run".
#[derive(Debug, Default)]
struct ExecutionWorkspace {
    /// The interner for terms a run computes. Cleared between runs; its tables are
    /// kept.
    scratch: crate::scratch::ScratchInterner,
    /// This workspace's retained capacity, as charged to
    /// [`crate::plan_memory::interner_memory_observer`].
    ///
    /// Retained capacity is retained memory, and a host that can read a plan's
    /// bytes off [`crate::PlanMemoryObserver`] should be able to read these too —
    /// otherwise this type's whole saving is memory that grew and became
    /// invisible. Recharged only when the retained figure actually MOVES, which
    /// after the first few runs it stops doing: re-charging on every run would put
    /// a mutex acquisition on the hot path to restate a number that did not
    /// change.
    charge: crate::plan_memory::InternerCharge,
    /// The bytes [`Self::charge`] currently stands for, so a recharge can be
    /// skipped when the capacity has not moved.
    charged_bytes: usize,
}

impl ExecutionWorkspace {
    /// Hand this run the retained tables, leaving an empty pair behind.
    ///
    /// The placeholder costs nothing: a fresh [`crate::scratch::ScratchInterner`]
    /// allocates no table until something is minted into it, which is the same
    /// property that makes a run whose evaluation fails cost only its own capacity
    /// rather than corrupting the next one's.
    fn check_out(&mut self) -> crate::scratch::ScratchInterner {
        std::mem::take(&mut self.scratch)
    }

    /// Take the tables back from a finished run, empty them, and re-charge the
    /// capacity they kept.
    ///
    /// Destructured with no rest pattern, for the same reason
    /// [`crate::scratch::ScratchInterner::clear`] is: this is the clearing seam, and
    /// a table added to this type and not given a line here would be retained
    /// across runs UNCLEARED — the one failure on this path that produces a wrong
    /// answer rather than an error. It does not compile instead.
    fn check_in(&mut self, used: &mut crate::scratch::ScratchInterner) {
        let Self {
            scratch,
            charge,
            charged_bytes,
        } = self;
        *scratch = std::mem::take(used);
        scratch.clear();
        let bytes = scratch.retained_capacity_bytes();
        if bytes != *charged_bytes {
            charge.clear();
            if bytes > 0 {
                charge.add(bytes);
            }
            *charged_bytes = bytes;
        }
    }

    /// Every byte this workspace is retaining, across every table in it.
    ///
    /// Destructured for the third reason the clear is: a table added and left out
    /// of this sum would be memory that grew and became invisible, which is the
    /// failure the charge exists to prevent. `charge` and `charged_bytes` are the
    /// accounting OF this sum rather than part of it, so they are named and
    /// skipped rather than added.
    fn retained_bytes(&self) -> usize {
        let Self {
            scratch,
            charge: _,
            charged_bytes: _,
        } = self;
        scratch.retained_capacity_bytes()
    }
}

/// What the last un-memoized run looked like, and whether a memo for it was tried.
///
/// Building a memo costs several whole rewrites, so it is not paid for a run that
/// may be the only one of its shape. It is paid on the SECOND consecutive sighting
/// of one lane and shape list, which is the point at which "this execution runs the
/// same query over changing values" has actually been observed rather than assumed.
/// A caller that runs a prepared execution once therefore pays nothing for the memo
/// at all, and a focus set alternating between shapes — an IRI node and a blank one,
/// the case [`ValueShape::SeedOnly`] exists for — never reaches a second consecutive
/// sighting and so never pays either.
#[derive(Debug)]
struct PendingShape {
    /// Which rewrite that run took.
    lane: ShaclPrebinding,
    /// The [`ValueShape`] of each of its bindings.
    shapes: Box<[ValueShape]>,
    /// Whether a memo was attempted for this lane and shape list and declined.
    ///
    /// A build is a pure function of the plan, the lane and the shapes, so a decline
    /// is final for all three: without this the attempt — and its several rewrites —
    /// would be repeated on every subsequent run, making the decline cost more than
    /// the memo would have saved.
    refused: bool,
}

/// The algebra one run evaluates.
///
/// Two variants because a run either reads the tree its execution retains or builds
/// one for itself, and the difference must be visible to the borrow checker: the
/// retained tree is borrowed from the execution, which is what stops a second run
/// from starting while the first is reading it.
pub(crate) enum Substituted<'a> {
    /// The retained substituted tree, or the admitted plan itself when this
    /// execution declares no parameters and there is nothing to substitute.
    Retained(&'a Query),
    /// A tree rewritten for this run alone, exactly as the `&str` doors build one.
    ///
    /// Boxed so this variant is one pointer wide instead of a whole [`Query`]:
    /// `Retained` is a single reference, and an enum's size is its LARGEST
    /// variant's, so an unboxed `Query` here would make every `Retained` — the
    /// hot, memoized path — carry room for a tree it never holds.
    Fresh(Box<Query>),
}

impl Substituted<'_> {
    /// The algebra to evaluate.
    pub(crate) fn query(&self) -> &Query {
        match self {
            Self::Retained(query) => query,
            Self::Fresh(query) => query.as_ref(),
        }
    }
}

impl PreparedExecution {
    /// An execution over `prepared`, declaring `parameters` and binding none of them.
    ///
    /// Private to the crate and taking already-interned [`Variable`]s, because
    /// [`NativeSparqlEngine::prepare_execution`](crate::NativeSparqlEngine::prepare_execution)
    /// is the only door: it is what refuses a repeated parameter name and what runs
    /// the admission whose result this value then carries.
    pub(crate) fn new(prepared: Arc<PreparedQuery>, parameters: Box<[Variable]>) -> Self {
        let values = vec![None; parameters.len()];
        Self {
            prepared,
            parameters,
            values,
            probes: Vec::new(),
            memo: None,
            pending: None,
            workspace: ExecutionWorkspace::default(),
        }
    }

    /// Hand this run the retained evaluation tables. See [`ExecutionWorkspace`].
    pub(crate) fn check_out_workspace(&mut self) -> crate::scratch::ScratchInterner {
        self.workspace.check_out()
    }

    /// Take the retained evaluation tables back from a finished run and empty them.
    ///
    /// Called on the failing path too, not only the answering one: an evaluation
    /// that errored still grew the tables, and giving that capacity back would make
    /// a handle that sees an occasional failure pay the growth again every time.
    pub(crate) fn check_in_workspace(&mut self, used: &mut crate::scratch::ScratchInterner) {
        self.workspace.check_in(used);
    }

    /// The bytes this execution's retained evaluation tables are holding — the
    /// capacity they keep between runs, not the values of any one run.
    ///
    /// The same figure charged to
    /// [`PlanMemoryObserver`](crate::PlanMemoryObserver), but not readable back out
    /// of one: an observer's own [`stats`](crate::PlanMemoryObserver::stats) is an
    /// engine- or thread-wide AGGREGATE across every execution charging into it, with
    /// no way to isolate one handle's share. This accessor is what makes the figure
    /// readable per handle instead. A host that pools executions is the caller this
    /// is for: the pool's footprint is the sum of these, and it is a number that grew
    /// because the handle stopped giving its tables back.
    ///
    /// It is also the one place the retention invariant is OBSERVABLE. Keeping the
    /// tables while failing to empty them would still answer correctly — the
    /// interner de-duplicates by value, so a stale entry is found rather than
    /// misread — and would grow this figure without bound, one run's values at a
    /// time. "The workspace does not grow with the number of runs" is therefore a
    /// claim only a reader of this can make, and
    /// `tests/prepared_execution.rs`'s
    /// `a_reused_handle_answers_and_charges_exactly_as_a_fresh_one_does` makes it.
    ///
    /// # `pub`, not `pub(crate)`
    ///
    /// That one caller is a Rust integration test — a file under this crate's own
    /// `tests/`, which `cargo` compiles as its OWN crate, linking `purrdf-sparql-eval`
    /// as an external dependency rather than as a module inside it. `pub(crate)`
    /// grants visibility inside the defining crate; an integration test is outside it
    /// by construction, so `pub(crate)` would make this UNREACHABLE from the one place
    /// that reads it, not merely discouraged there. `pub` is therefore not a looser
    /// grant made for convenience — it is the minimum visibility the test can compile
    /// against at all. A future reader auditing this crate's public surface for items
    /// with no production caller should read this section rather than re-litigate the
    /// accessor as unexplained dead surface: the earlier removal of this method's two
    /// test-only siblings (`plan()` and `bind_named_id`, see the commit that also
    /// extended this doc block) turned on the SAME question, decided the opposite way,
    /// for reasons specific to each — a redundant internal accessor and an unadopted
    /// suggestion, respectively, neither of which applies here.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> usize {
        self.workspace.retained_bytes()
    }

    /// The declared parameters, in declaration order.
    #[must_use]
    pub fn parameters(&self) -> &[Variable] {
        &self.parameters
    }

    /// The slot of the parameter named `name`, if it was declared.
    ///
    /// # Resolve once, bind many: the point of taking a slot at all
    ///
    /// [`Self::bind`] and [`Self::bind_id`] take a `usize` rather than a name so
    /// that a caller running the same execution once per row can pay the cost of
    /// finding a parameter's position ONCE, outside the row loop, instead of on
    /// every row — [`Self::bind_named`] exists for the caller who does not loop,
    /// and internally does exactly the linear scan this method does, once per
    /// call. `slot` is the other half of that trade: resolve a name to its slot
    /// before the loop starts, then bind by slot inside it.
    ///
    /// ```
    /// # use purrdf_core::TermValue;
    /// # use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
    /// let engine = NativeSparqlEngine::new();
    /// let mut execution = engine.prepare_execution(
    ///     "SELECT ?o WHERE { ?this <http://example.org/p> ?o }",
    ///     None,
    ///     &["this"],
    ///     QueryOptions::EMPTY,
    /// )?;
    ///
    /// // Resolved once, before the loop.
    /// let this_slot = execution.slot("this").expect("declared above");
    ///
    /// for subject in ["http://example.org/a", "http://example.org/b"] {
    ///     // Bound by index inside the loop: no per-row name lookup.
    ///     execution.bind(this_slot, TermValue::Iri(subject.to_owned()))?;
    ///     // ... run `execution` here ...
    /// }
    /// # Ok::<(), purrdf_core::RdfDiagnostic>(())
    /// ```
    #[must_use]
    pub fn slot(&self, name: &str) -> Option<usize> {
        self.parameters
            .iter()
            .position(|parameter| parameter.as_str() == name)
    }

    /// Bind the parameter in `slot` to `value`.
    ///
    /// # Errors
    ///
    /// [`RdfDiagnostic`] if `slot` is not a declared parameter. An out-of-range slot
    /// is a caller mistake about the query's own shape, not a value this execution
    /// could reasonably answer for, so it is refused rather than ignored — the same
    /// rule the pre-binding rewrite applies to a value it cannot ground.
    pub fn bind(&mut self, slot: usize, value: TermValue) -> Result<(), RdfDiagnostic> {
        self.write(slot, ParameterValue::Value(value))
    }

    /// Bind the parameter in `slot` to the term `dataset` interns at `id` — the **id
    /// door**.
    ///
    /// The same binding as [`Self::bind`], reached by a caller that already holds the
    /// dataset's own identity for the term instead of an owned spelling of it. A
    /// SHACL focus node is exactly that caller: target resolution produces term ids,
    /// and handing one to the value door meant materializing the term as a
    /// [`TermValue`] purely so the pre-binding rewrite could re-own the same bytes
    /// into the algebra and the BGP compiler could then hash them back to the id they
    /// came from. This door resolves the id straight into the algebra term
    /// (`crate::substitute::ground_term_from_id`), so that middle materialization —
    /// one owned `String` per term component, per focus node — does not happen.
    ///
    /// It is an ADDITIONAL door, not a replacement: [`Self::bind`] is unchanged and
    /// remains the door for a caller with a term and no dataset. The two agree by
    /// construction, because they converge on the same [`GroundTerm`] before anything
    /// reads them — `crates/sparql-eval/tests/prepared_execution.rs` pins that
    /// agreement on the substituted plan itself rather than asserting it.
    ///
    /// # Cross-dataset binding
    ///
    /// A dataset-local id is meaningless against any other dataset: an id from one
    /// view used against another is in range, resolves, and denotes a DIFFERENT term,
    /// with nothing to say so. This door makes that **impossible rather than
    /// refused**, and it does so structurally: the id and the view that interprets it
    /// are arguments of ONE call, the id is consumed inside it, and what the slot
    /// keeps afterwards is the resolved term — which carries no dataset-local
    /// identity at all. A handle therefore never holds an id, so there is no later
    /// moment at which an id could be paired with a dataset, and no way for a handle
    /// reused across datasets (which the SHACL handle cache does) to reinterpret one.
    /// `D::Id` is the view's OWN associated id type, so an id minted by a view of
    /// another kind does not type-check either.
    ///
    /// What remains is the caller passing this call an id one view minted and a
    /// different view of the same kind to read it with. That is
    /// [`DatasetView::resolve`]'s own precondition, unchanged and not widened by this
    /// door; the SHACL wiring satisfies it by construction, resolving against the
    /// very view it then executes against and falling back to [`Self::bind`] wherever
    /// the id space it holds is not the one the query will run in.
    ///
    /// # Errors
    ///
    /// [`RdfDiagnostic`] if `slot` is not a declared parameter — the identical
    /// refusal, with the identical diagnostic code and message, that [`Self::bind`]
    /// gives for the identical mistake — or if the term `dataset` holds at `id`
    /// cannot become an algebra term (an IRI a [`NamedNode`](purrdf_sparql_algebra::NamedNode)
    /// would refuse, a language tag this profile does not lex). That second refusal
    /// arrives EARLIER than the value door's, which grounds at run time rather than
    /// at bind time; it is the same judgement on the same components, made as soon as
    /// there is something to judge.
    pub fn bind_id<D: DatasetView>(
        &mut self,
        slot: usize,
        dataset: &D,
        id: D::Id,
    ) -> Result<(), RdfDiagnostic> {
        // The slot is checked BEFORE the id is resolved, so a caller who got the slot
        // wrong reads the same diagnostic here as at the value door rather than a
        // grounding failure from a term they never meant to bind.
        if slot >= self.values.len() {
            return Err(self.no_such_slot(slot));
        }
        let ground = crate::substitute::ground_term_from_id(dataset, id)?;
        self.write(slot, ParameterValue::Ground(ground))
    }

    /// Bind the parameter called `name` to `value`.
    ///
    /// # Errors
    ///
    /// [`RdfDiagnostic`] if no parameter of that name was declared. Silently dropping
    /// an unknown name would leave the parameter it was meant for at its previous
    /// value and answer a query nobody asked, which is the shape of a silent wrong
    /// answer rather than of a lenient API.
    pub fn bind_named(&mut self, name: &str, value: TermValue) -> Result<(), RdfDiagnostic> {
        let slot = self.declared_slot(name)?;
        self.write(slot, ParameterValue::Value(value))
    }

    /// Write `value` into `slot`, or refuse the slot.
    ///
    /// The single writing seam, so the two doors cannot grow two different notions of
    /// what an out-of-range slot means. `bind_id` checks the slot separately BEFORE
    /// resolving, for the ordering reason given there; this is what it checks against.
    fn write(&mut self, slot: usize, value: ParameterValue) -> Result<(), RdfDiagnostic> {
        let Some(cell) = self.values.get_mut(slot) else {
            return Err(self.no_such_slot(slot));
        };
        *cell = Some(value);
        Ok(())
    }

    /// The slot named `name`, or the undeclared-name refusal both name doors give.
    ///
    /// # Errors
    ///
    /// [`RdfDiagnostic`] if no parameter of that name was declared.
    fn declared_slot(&self, name: &str) -> Result<usize, RdfDiagnostic> {
        self.slot(name).ok_or_else(|| {
            RdfDiagnostic::error(
                "native-sparql-execution-parameter",
                format!(
                    "no parameter named {name:?}: this execution declares {}",
                    self.parameter_list()
                ),
            )
        })
    }

    /// The out-of-range-slot refusal both slot doors give.
    fn no_such_slot(&self, slot: usize) -> RdfDiagnostic {
        RdfDiagnostic::error(
            "native-sparql-execution-parameter",
            format!(
                "no parameter in slot {slot}: this execution declares {}",
                self.parameter_list()
            ),
        )
    }

    /// Return every parameter to unbound.
    ///
    /// `bind` and `bind_named` only ever write `Some`, so without this there is no
    /// way to make a slot forget a value it was once given. That is exactly the gap
    /// a caller whose bindings must be TOTAL per call falls into: a Python
    /// `PreparedQuery.run(**bindings)`, say, where each call's keyword arguments are
    /// meant to be the whole story. Such a caller applies only the parameters a
    /// given call actually names, so a parameter one call bound and a LATER call
    /// omits would otherwise keep answering with that earlier call's value forever
    /// — not a crash, not a wrong-looking result, just a query silently answering a
    /// question one fewer (or one different) argument than the caller actually
    /// asked. That is the same shape of failure an unbound parameter is already
    /// refused for, except here the parameter only *looks* bound, because this
    /// execution's own bookkeeping cannot tell "bound this call" from "bound three
    /// calls ago and never touched since".
    ///
    /// The fix is to call `unbind_all` at the top of every such call, before
    /// applying that call's own bindings: every slot goes back to `None`, so a
    /// parameter the call does not (re)bind is `None` when `execute` checks, and is
    /// refused by the engine's own unbound check exactly as if this execution had
    /// just been prepared. There is deliberately no partial `unbind(slot)` — nothing
    /// in this crate needs to forget one parameter while keeping the rest, and
    /// adding one would only invite a second, narrower way to get the total-call
    /// semantics wrong.
    pub fn unbind_all(&mut self) {
        self.values.fill(None);
    }

    /// The declared parameters, for a diagnostic.
    fn parameter_list(&self) -> String {
        if self.parameters.is_empty() {
            return "none".to_owned();
        }
        self.parameters
            .iter()
            .map(|parameter| format!("?{}", parameter.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The names of every parameter still unbound.
    pub(crate) fn unbound(&self) -> Vec<&str> {
        self.parameters
            .iter()
            .zip(&self.values)
            .filter(|(_, value)| value.is_none())
            .map(|(parameter, _)| parameter.as_str())
            .collect()
    }

    /// The algebra this run evaluates: the current bindings, pre-bound into the plan
    /// by whichever rewrite `lane` names.
    ///
    /// This is where a prepared execution stops re-deriving the substituted plan and
    /// starts re-binding one. The first sighting of a lane and value-shape list takes
    /// the ordinary rewrite; the second builds a [`PrebindMemo`] for it, which is
    /// accepted only if replaying values into it reproduces the real rewrite exactly;
    /// every run after that writes its values into the retained tree. A run whose
    /// values have a different SHAPE — a blank-node focus node where the memo was
    /// built for an IRI one, say — takes the ordinary rewrite and leaves the memo
    /// alone, so a mixed focus set costs what it always did and corrupts nothing.
    ///
    /// Only valid once every parameter is bound, which
    /// [`NativeSparqlEngine::execute`](crate::NativeSparqlEngine::execute) checks
    /// before it calls this.
    ///
    /// # Errors
    ///
    /// [`RdfDiagnostic`] if a bound value cannot become an algebra term — a datatype
    /// IRI that is not a valid IRI, or a language tag the concrete syntaxes would not
    /// have lexed. That refusal is the rewrite's own and is unchanged by the memo: it
    /// happens while the values are being grounded, before any tree is touched.
    pub(crate) fn substituted(
        &mut self,
        lane: ShaclPrebinding,
    ) -> Result<Substituted<'_>, RdfDiagnostic> {
        // Destructured so the probe buffer, the memo and the parameter slices are
        // three disjoint borrows rather than three borrows of one `self`.
        let Self {
            prepared,
            parameters,
            values,
            probes,
            memo,
            pending,
            // Named and ignored rather than covered by a `..` rest: the rewrite has
            // no business in the run's evaluation tables, and spelling that out
            // keeps this pattern exhaustive, so a field added later still has to be
            // decided about here.
            workspace: _,
        } = self;
        crate::substitute::build_probes_into(
            probes,
            crate::substitute::Prebindings::Paired(parameters, values),
        )?;
        if probes.is_empty() {
            return Ok(Substituted::Retained(prepared.query()));
        }
        if let Some(memo) = memo.as_mut()
            && memo.matches(lane, probes)
        {
            let bound = memo.bind(probes);
            // The differential oracle: in a debug build, every memo hit is checked
            // against the rewrite it stands in for, not just trusted because
            // `matches` said yes. `PrebindMemo::build` already proves this once, at
            // build time, against a MOVED probe set; this is the same comparison,
            // repeated on every later run's REAL probes, which is the situation
            // `build`'s own check cannot see — a shape that recurs correctly for the
            // value it was built from but diverges for some later value of the same
            // shape. Compiled out of a release build entirely, so it costs nothing
            // there; in a debug build it turns every existing test that runs a
            // prepared execution twice with matching shapes into a case of this
            // check, which is far more coverage than any bespoke fixture list here
            // could hold on its own.
            //
            // Gated additionally by `memo_verification_enabled`, the one escape hatch
            // from that check within a debug build: see
            // [`set_memo_verification_enabled`] for why it exists (this comparison's
            // own cost IS the allocation a memo hit exists to avoid, which makes it
            // indistinguishable from what it is checking to any instrument that counts
            // allocations) and for the scope it is deliberately narrow about.
            #[cfg(debug_assertions)]
            if memo_verification_enabled() {
                let fresh =
                    crate::prebind_memo::rewrite(prepared.query().clone(), lane, probes.clone());
                assert_eq!(
                    bound, &fresh,
                    "a prepared execution's memoized substituted plan disagrees with the \
                     rewrite it stands in for, for lane {lane:?}: the memo would have answered \
                     a query nobody asked for"
                );
            }
            return Ok(Substituted::Retained(bound));
        }
        let seen_before = pending.as_ref().is_some_and(|seen| {
            seen.lane == lane && PrebindMemo::shapes_match(&seen.shapes, probes)
        });
        if !seen_before {
            *pending = Some(PendingShape {
                lane,
                shapes: PrebindMemo::shapes_of(probes),
                refused: false,
            });
        } else if memo.is_none() && pending.as_ref().is_some_and(|seen| !seen.refused) {
            // A second consecutive sighting of one lane and shape list, and no memo
            // yet: this is the run that pays for one. (A memo that already exists for
            // ANOTHER shape list is left alone — one memo per execution, so the other
            // shapes keep the ordinary rewrite rather than evicting a tree that is
            // answering for the shape this execution mostly sees.)
            if let Some(built) = PrebindMemo::build(prepared.query(), lane, probes) {
                *pending = None;
                return Ok(Substituted::Retained(memo.insert(built).query()));
            }
            if let Some(seen) = pending.as_mut() {
                seen.refused = true;
            }
        }
        Ok(Substituted::Fresh(Box::new(crate::prebind_memo::rewrite(
            prepared.query().clone(),
            lane,
            probes.clone(),
        ))))
    }
}
