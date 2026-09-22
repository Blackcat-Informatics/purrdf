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

use purrdf_core::{RdfDiagnostic, TermValue};
use purrdf_sparql_algebra::{GroundTerm, Query, Variable};

use crate::engine::{PreparedQuery, ShaclPrebinding};
use crate::prebind_memo::{PrebindMemo, ValueShape};

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
    pub(crate) values: Vec<Option<TermValue>>,
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
        }
    }

    /// The declared parameters, in declaration order.
    #[must_use]
    pub fn parameters(&self) -> &[Variable] {
        &self.parameters
    }

    /// The plan this execution runs.
    #[must_use]
    pub fn plan(&self) -> &PreparedQuery {
        &self.prepared
    }

    /// The slot of the parameter named `name`, if it was declared.
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
        let Some(cell) = self.values.get_mut(slot) else {
            return Err(RdfDiagnostic::error(
                "native-sparql-execution-parameter",
                format!(
                    "no parameter in slot {slot}: this execution declares {}",
                    self.parameter_list()
                ),
            ));
        };
        *cell = Some(value);
        Ok(())
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
        let Some(slot) = self.slot(name) else {
            return Err(RdfDiagnostic::error(
                "native-sparql-execution-parameter",
                format!(
                    "no parameter named {name:?}: this execution declares {}",
                    self.parameter_list()
                ),
            ));
        };
        self.values[slot] = Some(value);
        Ok(())
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
