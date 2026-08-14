// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The property-function **dispatch**: evaluating a
//! [`GraphPattern::PropertyFunction`] node against the injected registry.
//!
//! A call is a row source driven **per input row**. The rows it is driven over come
//! from the enclosing `Lateral` — the shape the parser builds for a triples block
//! containing a call — and the call's arguments are read DIRECTLY from that row:
//!
//! ```text
//! ?doc ex:contains ("needle" ?score)
//! ```
//!
//! with `?doc` bound by a sibling data pattern is one invocation per document, each
//! with the document and the needle BOUND and only `?score` free.
//!
//! # Why the arguments are read from the row rather than from a substituted node
//!
//! The generic `Lateral` path substitutes the outer row into its right operand and
//! evaluates the rewritten pattern. That substitution is IRI-only by doctrine
//! ([`crate::expr::substitute_pattern`]): a literal, blank-node or quoted-triple
//! binding stays a variable in the rewritten tree and is reconciled afterwards by the
//! lateral join's compatibility test. For an ordinary pattern that is merely a
//! late filter. For a relation it would be a **wrong access pattern**: the position
//! would be reported free, the invocation's [`BindingPattern`] would lose a bound bit,
//! and a relation that declares only `bf` would be refused an invocation the engine
//! can perfectly well make — or, worse, an unbounded generator would be opened
//! wide-open and filtered afterwards. So the call reads the row itself and every
//! binding, whatever its term kind, becomes a bound argument.
//!
//! # What the engine guarantees about what a relation returns
//!
//! A relation is arbitrary host Rust, so nothing it emits is trusted:
//!
//! * **Row width** is checked against the declared arity before a row is used.
//! * **Bound positions are equality-filtered.** A relation may echo the input value
//!   (the usual thing) or emit any candidate it likes; a row disagreeing at a bound
//!   position is dropped. This is what makes `PropertyFunction::admits`'s subsumption
//!   rule sound — a relation serving `bf` answers a `bb` invocation by generating and
//!   letting this filter cut.
//! * **Repeated variables** — within one side or across both — are enforced by the
//!   same mechanism: the first occurrence binds, later occurrences compare. A row
//!   violating consistency is FILTERED, not an error: it is an ordinary non-match,
//!   exactly as `?s ex:p ?s` is for a triple pattern.
//! * **Blank-node arguments** are non-distinguished variables: they bind, they enforce
//!   consistency across their occurrences within the call, and they are projected away
//!   before the node's rows leave — the same treatment `crate::bgp` gives a blank in a
//!   triple pattern, using the same synthetic-slot machinery.
//!
//! # Where a call's row ceiling comes from
//!
//! A call stops producing once the node whose output its rows are can use no more of
//! them — the answer-cap / `LIMIT` pushdown's verdict. Which node that is depends on the
//! shape, and the two are genuinely different nodes:
//!
//! * A call written with nothing before it in its group is a plan node evaluated at its
//!   own address, and the ceiling recorded there is its own.
//! * A call written after a data pattern is the right operand of a `Lateral`, and
//!   [`crate::binop::eval_lateral`] FUSES the pair: the rows produced below are already
//!   joined with the left row, so they are the `Lateral`'s output rows and the ceiling
//!   that bounds them is the `Lateral`'s. That node's certificate is what licensed it;
//!   nothing new is licensed by the fusion, and the call node itself carries no ceiling
//!   (see [`crate::governor::soundness::child_row_ceiling`]).
//!
//! The ceiling is consumed twice over: this dispatch stops accumulating at it, and — for
//! a call whose positions leave the engine nothing to filter — it is passed on to the
//! relation as the licence to stop generating. The intermediate-cell ceiling is a
//! separate, live governor and is applied to the same bag through the shared
//! [`crate::row_ingest::GovernedRowIngest`], exactly as `crate::bgp` applies both to a
//! join order's last stage.
//!
//! # Emission order is preserved verbatim
//!
//! Rows leave in the order the relation emitted them, per input row in input-row
//! order. Nothing is sorted, and nothing is de-duplicated: a relation's declared
//! emission order is part of its contract precisely so the query's answer is
//! reproducible, and re-ordering it here would throw that away.

use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{DatasetView, TermValue, TrippedGovernor};
use purrdf_sparql_algebra::{
    AggregateFunction, Function, GraphPattern, NamedNodePattern, PropertyFunctionCall, TermPattern,
    TriplePattern, Variable,
};

use crate::DetHashMap;
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::governor::ChargePoint;
use crate::governor::lift::{Evaluated, Truncation};
use crate::property_fn::{PfArgs, PfArity, PropertyFunction, next_contained, open_contained};
use crate::row_ingest::{GovernedRowIngest, RowAdmission};
use crate::solution::{Solution, SolutionSeq, VarSchema};

/// Evaluate a property-function node that is NOT the right operand of a `Lateral` —
/// a call with nothing written before it in its group.
///
/// The input bag is then the identity table `Z` (one row binding nothing), which is
/// exactly what makes this the same code path as the lateral one with an empty left
/// side rather than a second implementation of it.
///
/// # Errors
///
/// Propagates every failure [`eval_call_over`] raises.
pub(crate) fn eval_property_function<D: DatasetView + Sync>(
    call: &PropertyFunctionCall,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let unit = SolutionSeq::unit();
    // This node IS the plan node being evaluated, so the ceiling that bounds its output
    // is its own.
    let ceiling = ctx.row_ceiling();
    finish(eval_call_over(call, &unit, ceiling, ctx)?)
}

/// Evaluate `Lateral(left, PropertyFunction(call))` given `left`'s already-evaluated
/// rows: one invocation per left row, output rows in left-row order.
///
/// `ceiling` is the row ceiling of the **`Lateral`**, not of the call node: the rows
/// produced here are already joined with `left`, so they are the fused operator's — that
/// is, the `Lateral`'s — output rows. [`crate::binop::eval_lateral`] reads it there and
/// hands it over; see [`crate::governor::soundness::child_row_ceiling`] for why the
/// licence lives at that node and is not pushed across the edge.
///
/// # Errors
///
/// Propagates every failure [`eval_call_over`] raises.
pub(crate) fn eval_lateral_property_function<D: DatasetView + Sync>(
    call: &PropertyFunctionCall,
    left: &SolutionSeq<D::Id>,
    ceiling: Option<usize>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    finish(eval_call_over(call, left, ceiling, ctx)?)
}

/// Wrap a driven call's bag in the governed outcome channel: a complete bag, or the
/// truncation this node ORIGINATES (its rows are a positional prefix of what the whole
/// invocation sequence would have produced, because both loops — input rows and emitted
/// rows — run in order and stop at the first refusal).
fn finish<I: purrdf_core::ViewTermId>(
    (seq, tripped): (SolutionSeq<I>, Option<TrippedGovernor>),
) -> Result<Evaluated<I>, EvalError> {
    Ok(match tripped {
        None => Evaluated::Complete(seq),
        Some(tripped) => Evaluated::Truncated(Truncation::origin(seq, tripped)),
    })
}

// ---------------------------------------------------------------------------
// The per-node plan
// ---------------------------------------------------------------------------

/// One argument position's compiled form.
#[derive(Debug)]
enum Arg {
    /// A constant written at the call site: the same value on every row.
    Constant(TermValue),
    /// A variable or blank node, as an index into the call's slot space (see
    /// [`CallPlan`]).
    Slot(usize),
    /// A quoted triple carrying at least one variable/blank: matched structurally,
    /// exactly as a `Bgp` matches a quoted-triple position that binds.
    Triple(Box<[Self; 3]>),
}

/// The per-node compilation of a call: what each flattened position is, which slots
/// its variables occupy, and which output column each slot lands in.
///
/// Computed once per node evaluation rather than per input row: none of it depends on
/// the data.
#[derive(Debug)]
struct CallPlan {
    /// The flattened argument positions, subject side then object side.
    args: Vec<Arg>,
    /// The declared subject-side width, so [`PfArgs`] can be split at it.
    subject_len: usize,
    /// Slot → output column, or `None` for a blank-node slot (projected away).
    slot_cols: Vec<Option<usize>>,
    /// The output schema: the input schema's columns, then the call's variables in
    /// first-seen flattened order. Blank-node slots are NOT columns.
    schema: Arc<VarSchema>,
    /// The output columns a successful row must fill — the call's variable slots'
    /// columns, in slot order.
    bound_cols: Vec<(usize, usize)>,
    /// Slot → the input schema column it reads its seed value from, when the input
    /// schema already carries that variable.
    slot_seed: Vec<Option<usize>>,
    /// Whether a row ceiling may be handed to the relation for this call — see
    /// [`args_are_admission_transparent`].
    ceiling_is_offerable: bool,
}

impl CallPlan {
    /// Compile `call` against the schema of the rows it will be driven over.
    fn compile(call: &PropertyFunctionCall, input: &VarSchema) -> Result<Self, EvalError> {
        let mut slots: DetHashMap<Variable, usize> = DetHashMap::default();
        let mut schema = input.clone();
        let mut slot_cols: Vec<Option<usize>> = Vec::new();
        let mut slot_seed: Vec<Option<usize>> = Vec::new();
        let mut args = Vec::with_capacity(call.subject_args.len() + call.object_args.len());
        for term in call.subject_args.iter().chain(&call.object_args) {
            args.push(compile_arg(
                term,
                &mut slots,
                &mut schema,
                &mut slot_cols,
                &mut slot_seed,
                input,
            )?);
        }
        let bound_cols = slot_cols
            .iter()
            .enumerate()
            .filter_map(|(slot, col)| col.map(|col| (slot, col)))
            .collect();
        let ceiling_is_offerable = args_are_admission_transparent(&args, slot_cols.len());
        Ok(Self {
            args,
            subject_len: call.subject_args.len(),
            slot_cols,
            schema: Arc::new(schema),
            bound_cols,
            slot_seed,
            ceiling_is_offerable,
        })
    }

    /// The number of distinct variable/blank slots the call carries.
    fn slot_count(&self) -> usize {
        self.slot_cols.len()
    }
}

/// Whether this call's positions are **admission-transparent**: every row the relation
/// emits that agrees with the invocation's bound positions is admitted by
/// [`unify_row`], with nothing left for the engine to drop.
///
/// This is the precondition for offering the relation a row ceiling at all, and it is a
/// question about what the relation can *see*. A ceiling says "you may stop after this
/// many rows"; a relation can only honour it against rows it knows the engine wants, and
/// the invocation's bound values are the whole of what it is told. Two things the engine
/// filters on are invisible from there:
///
/// * **A repeated slot.** `?x <rel> ?x` hands the relation two FREE positions; it cannot
///   know they must be equal, and the engine drops the rows where they are not. A
///   relation stopping at `k` emitted rows could then hand back none at all — a short
///   answer reported as complete. A slot seeded from the input row is not affected: both
///   of its occurrences arrive bound, so agreement at the bound positions already implies
///   unification. The test below is nonetheless purely syntactic, because seeding is a
///   per-row fact (an `OPTIONAL` can leave the column unbound on one row and not the
///   next) and withholding a ceiling only costs an optimization.
/// * **A quoted-triple position that is not fully bound.** It reaches the relation as a
///   free position, so the relation may put any term there, and [`unify_row`] then drops
///   whatever fails to match structurally.
///
/// Everything else — constants, and slots that occur once — is either bound (and so
/// visible to the relation as the value the engine will compare against) or unconstrained
/// (and so always unifies).
fn args_are_admission_transparent(args: &[Arg], slot_count: usize) -> bool {
    fn walk(arg: &Arg, seen: &mut [bool]) -> bool {
        match arg {
            Arg::Constant(_) => true,
            Arg::Slot(slot) => !std::mem::replace(&mut seen[*slot], true),
            Arg::Triple(_) => false,
        }
    }
    let mut seen = vec![false; slot_count];
    args.iter().all(|arg| walk(arg, &mut seen))
}

/// Compile one argument position, registering any variable/blank it introduces.
fn compile_arg(
    term: &TermPattern,
    slots: &mut DetHashMap<Variable, usize>,
    schema: &mut VarSchema,
    slot_cols: &mut Vec<Option<usize>>,
    slot_seed: &mut Vec<Option<usize>>,
    input: &VarSchema,
) -> Result<Arg, EvalError> {
    match term {
        TermPattern::NamedNode(_) | TermPattern::Literal(_) => Ok(Arg::Constant(
            crate::convert::ground_term_pattern_to_value(term)?,
        )),
        TermPattern::Variable(variable) => Ok(Arg::Slot(slot_for(
            variable.clone(),
            true,
            slots,
            schema,
            slot_cols,
            slot_seed,
            input,
        ))),
        // A blank node is a non-distinguished variable. It gets a slot (so its
        // occurrences must agree) under the same NUL-prefixed synthetic name
        // `crate::bgp` uses, and no output column (so it is projected away).
        TermPattern::BlankNode(blank) => Ok(Arg::Slot(slot_for(
            crate::bgp::blank_var(blank.as_str()),
            false,
            slots,
            schema,
            slot_cols,
            slot_seed,
            input,
        ))),
        TermPattern::Triple(triple) => {
            if triple_is_ground(triple) {
                return Ok(Arg::Constant(
                    crate::convert::ground_triple_pattern_to_value(triple)?,
                ));
            }
            let predicate = match &triple.predicate {
                NamedNodePattern::NamedNode(node) => {
                    Arg::Constant(crate::convert::named_node_to_value(node))
                }
                NamedNodePattern::Variable(variable) => Arg::Slot(slot_for(
                    variable.clone(),
                    true,
                    slots,
                    schema,
                    slot_cols,
                    slot_seed,
                    input,
                )),
            };
            let subject = compile_arg(&triple.subject, slots, schema, slot_cols, slot_seed, input)?;
            let object = compile_arg(&triple.object, slots, schema, slot_cols, slot_seed, input)?;
            Ok(Arg::Triple(Box::new([subject, predicate, object])))
        }
    }
}

/// The slot of `variable`, registering it (and its output column, when it is a real
/// variable rather than a synthetic blank slot) on first sight.
fn slot_for(
    variable: Variable,
    projected: bool,
    slots: &mut DetHashMap<Variable, usize>,
    schema: &mut VarSchema,
    slot_cols: &mut Vec<Option<usize>>,
    slot_seed: &mut Vec<Option<usize>>,
    input: &VarSchema,
) -> usize {
    if let Some(&slot) = slots.get(&variable) {
        return slot;
    }
    let slot = slot_cols.len();
    slot_cols.push(projected.then(|| schema.push(variable.clone())));
    slot_seed.push(projected.then(|| input.index_of(&variable)).flatten());
    slots.insert(variable, slot);
    slot
}

/// Whether a quoted-triple pattern is variable-free (and so a plain constant).
fn triple_is_ground(triple: &TriplePattern) -> bool {
    fn term_is_ground(term: &TermPattern) -> bool {
        match term {
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => true,
            TermPattern::Variable(_) | TermPattern::BlankNode(_) => false,
            TermPattern::Triple(triple) => triple_is_ground(triple),
        }
    }
    term_is_ground(&triple.subject)
        && matches!(triple.predicate, NamedNodePattern::NamedNode(_))
        && term_is_ground(&triple.object)
}

// ---------------------------------------------------------------------------
// The driver
// ---------------------------------------------------------------------------

/// Drive `call` over every row of `input`, in order.
///
/// `ceiling` is the answer-cap / `LIMIT` row ceiling of the node whose OUTPUT this bag
/// is — the call node itself when the call stands alone, the enclosing `Lateral` when
/// this is the fused correlated shape. Both callers read it from that node; it is a
/// parameter rather than a second `ctx.row_ceiling()` here precisely because the two
/// shapes read it at different nodes.
///
/// It is applied exactly as `crate::bgp` applies its ceiling to the last stage of a join
/// order: the rows accumulated below are that node's output rows, produced in order, so
/// stopping at `k` of them yields the first `k` — a genuine positional prefix.
///
/// Returns the output bag together with the governor that stopped it, if one did.
fn eval_call_over<D: DatasetView + Sync>(
    call: &PropertyFunctionCall,
    input: &SolutionSeq<D::Id>,
    ceiling: Option<usize>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<(SolutionSeq<D::Id>, Option<TrippedGovernor>), EvalError> {
    let relation = resolve(call, ctx)?;
    let plan = CallPlan::compile(call, &input.schema)?;
    let declared =
        crate::property_fn::declaration_contained(&call.iri, "arity", || relation.arity())?;
    let supplied = PfArity::new(plan.subject_len, plan.args.len() - plan.subject_len);
    if declared != supplied {
        return Err(EvalError::function(format!(
            "property function <{}> is declared with {declared} argument(s); the call site \
             supplies {supplied}",
            call.iri
        )));
    }
    // Read once per node evaluation rather than once per input row: the declaration
    // cannot change between rows of the SAME invocation loop, so the previous per-row
    // `modes()`/`admits()` call was repeated, uncontained host-code work for an answer
    // that could not change. `admit_mode` below checks each row's own access pattern
    // against this cached list rather than asking the relation again.
    let modes = crate::property_fn::declaration_contained(&call.iri, "declared modes", || {
        relation.modes().to_vec()
    })?;

    let width = plan.schema.len();
    let left_len = input.schema.len();
    // A call's own per-row point. It rides the shared governed ingest core exactly as
    // `SERVICE`'s does, because a relation's output is the other bag in this evaluator
    // whose size an outside party picks: the generic per-node accounting every algebra
    // node pays would price a relation that emits a million rows from one invocation
    // exactly as it prices one that emits ten.
    let ingest = GovernedRowIngest::new(ctx, width, Some(ChargePoint::PropertyFunctionRow));
    let mut rows: Vec<Solution<D::Id>> = Vec::new();
    let mut tripped: Option<TrippedGovernor> = None;

    let mut seed: Vec<Option<TermValue>> = vec![None; plan.slot_count()];
    let mut values: Vec<Option<TermValue>> = vec![None; plan.slot_count()];
    // The per-invocation argument buffer: fixed length (`plan.args.len()`), hoisted out
    // of the row loop and refilled in place, so every row reuses the same allocation
    // instead of paying for a fresh `Vec` per input row. Owned `TermValue`s, so nothing
    // borrows from it across a mutation and reuse is unconditionally sound.
    let mut args: Vec<Option<TermValue>> = vec![None; plan.args.len()];

    'input: for mu in &input.rows {
        // The invocation's inputs, read straight off this row: a slot seeded from the
        // input schema is BOUND whatever term kind it carries.
        for (slot, column) in plan.slot_seed.iter().enumerate() {
            seed[slot] = column
                .and_then(|column| mu.get(column).copied().flatten())
                .map(|term| ctx.scratch.value_of(ctx.dataset, term));
        }
        for (dst, arg) in args.iter_mut().zip(&plan.args) {
            *dst = arg_value(arg, &seed);
        }
        // The borrowed view `PfArgs` needs. This one is NOT hoisted the way `args` is
        // above: it borrows `args`, which this loop mutates every row, and reusing a
        // `Vec<Option<&TermValue>>` across a mutation of its own referent is exactly the
        // reborrow shape today's (non-Polonius) borrow checker cannot verify sound
        // across a loop back-edge. `SmallVec` sidesteps the problem rather than fighting
        // it: built fresh each row, but on the STACK for every relation of `Solution`'s
        // own usual width (inline capacity 4, matching `crate::solution::Solution`) —
        // so the common case pays no heap allocation at all, and only an arity wider
        // than that spills, exactly as the plain `Vec` it replaces always did.
        let refs: smallvec::SmallVec<[Option<&TermValue>; 4]> =
            args.iter().map(Option::as_ref).collect();
        let (subject, object) = refs.split_at(plan.subject_len);
        let pf_args = PfArgs::new(subject, object);
        let mode = pf_args.mode();
        admit_mode(&modes, &call.iri, mode)?;

        // The `property-function-invocation` charge point. Charged once per invocation
        // that actually reaches host code — the arity and access-pattern refusals above
        // entered no relation, and charging for work that never happened would make the
        // schedule a description of the query text rather than of the execution. This is
        // the placement doctrine `crate::user_fn`'s invocation point already follows.
        if let Err(governor) = ctx.charge(ChargePoint::PropertyFunctionInvocation) {
            tripped = Some(governor);
            break 'input;
        }

        // Poll before entering host code: a relation may block for the whole of one
        // `open`, so the poll immediately before it is the last one that can prevent
        // the wait rather than merely notice it afterwards.
        if let Some(governor) = ctx.stop_check() {
            tripped = Some(governor);
            break 'input;
        }
        // The relation's own licence to stop early: what is LEFT of the node's ceiling
        // once the invocations already driven have contributed. Remaining rather than the
        // whole ceiling because the accumulated bag is the node's output — the tightest
        // honest number is what this invocation could still add to it.
        //
        // Withheld entirely unless the call is admission-transparent: a ceiling honoured
        // against rows this engine then drops for a reason the relation cannot see would
        // turn a short bag into a "complete" answer. See
        // [`args_are_admission_transparent`].
        let invocation_ceiling = ceiling
            .filter(|_| plan.ceiling_is_offerable)
            .map(|ceiling| u64::try_from(ceiling.saturating_sub(rows.len())).unwrap_or(u64::MAX));
        let mut cursor =
            open_contained(relation.as_ref(), &call.iri, &pf_args, invocation_ceiling)?;

        loop {
            if let Some(governor) = ctx.stop_check() {
                tripped = Some(governor);
                break 'input;
            }
            // The semantic ceiling: rows past it cannot reach the query's answer, so
            // this node stops producing. It is a plan licence, not a governor, so it
            // ends the work without certifying a truncation.
            if ceiling.is_some_and(|ceiling| rows.len() >= ceiling) {
                break 'input;
            }
            let Some(emitted) = next_contained(&mut *cursor, &call.iri)? else {
                break;
            };
            if emitted.len() != declared.total() {
                return Err(EvalError::function(format!(
                    "property function <{}> emitted a row of {} value(s); its declared arity \
                     ({declared}) requires {}",
                    call.iri,
                    emitted.len(),
                    declared.total()
                )));
            }
            values.clone_from(&seed);
            if !unify_row(&plan.args, &emitted, &mut values) {
                // A row that disagrees with a bound position, or with an earlier
                // occurrence of a repeated variable. An ordinary non-match: filtered,
                // never an error.
                continue;
            }
            match ingest.admit(ctx, rows.len()) {
                RowAdmission::Abandoned(governor) => {
                    tripped = governor;
                    break 'input;
                }
                RowAdmission::Admitted => {}
            }
            let mut row: Solution<D::Id> = smallvec::smallvec![None; width];
            row[..left_len].copy_from_slice(mu);
            for &(slot, column) in &plan.bound_cols {
                let value = values[slot]
                    .clone()
                    .ok_or_else(|| unbound_slot_internal(&call.iri))?;
                row[column] = Some(ctx.scratch.intern(ctx.dataset, value));
            }
            rows.push(row);
        }
    }

    // The node's own materialized bag, measured against the intermediate-cell ceiling
    // exactly as every other producer measures its output.
    if tripped.is_none() {
        tripped = ctx.observe_cells(rows.len(), width).err();
    }
    Ok((
        SolutionSeq {
            schema: plan.schema,
            rows,
        },
        tripped,
    ))
}

/// Resolve a call's predicate IRI to its registered relation.
///
/// An unregistered IRI is a hard [`EvalError::Function`], and an absent registry is the
/// same failure spelled differently: the parser only ever mints this node under a
/// caller-configured namespace, so reaching it with nothing to resolve against is a
/// host misconfiguration, never an empty relation. The engine's prepare step raises
/// this before any governor charge; this is the backstop for a directly-built
/// [`EvalCtx`], which never runs that step.
fn resolve<D: DatasetView + Sync>(
    call: &PropertyFunctionCall,
    ctx: &EvalCtx<'_, D>,
) -> Result<Arc<dyn PropertyFunction>, EvalError> {
    ctx.property_functions
        .and_then(|registry| registry.resolve(&call.iri))
        .map(Arc::clone)
        .ok_or_else(|| {
            EvalError::function(format!(
                "no property function is registered for <{}>",
                call.iri
            ))
        })
}

/// Check that a relation whose declared modes are `modes` can serve an invocation whose
/// access pattern is `mode`.
///
/// The prepare-time feasibility ordering
/// ([`crate::property_fn_plan`]) already proved a feasible order exists for the
/// *statically* bound positions. This is the per-row check, and it can genuinely fail
/// where the static one passed: a variable the plan saw bound by an earlier atom may be
/// UNBOUND in a particular row (an `OPTIONAL` left it so), which makes this invocation
/// strictly more general than the one the plan admitted. That is a real infeasibility,
/// not an engine bug, so it is a typed failure naming both patterns — and it is a
/// failure rather than zero rows, because a relation that cannot compute an answer has
/// not established that there is none.
///
/// Takes the ALREADY-READ declared-mode list rather than the relation itself: this runs
/// once per input row, and re-reading [`PropertyFunction::modes`] from the relation on
/// every row would repeat host-code work — and an uncontained host call — for an answer
/// that cannot change within one invocation loop. [`eval_call_over`] reads it once,
/// contained, before the row loop starts. This is the same lattice rule
/// [`PropertyFunction::admits`] states; it is restated here rather than called because
/// the cached list, not a live relation reference, is what this check has to hand.
fn admit_mode(modes: &[BindingPattern], iri: &str, mode: BindingPattern) -> Result<(), EvalError> {
    if modes.iter().any(|declared| declared.subsumes(mode)) {
        return Ok(());
    }
    let declared: Vec<String> = modes.iter().copied().map(BindingPattern::code).collect();
    Err(EvalError::function(format!(
        "property function <{iri}> cannot serve the invocation `{}`; it declares [{}] — a \
         position the plan expected to be bound is unbound in this row",
        mode.code(),
        declared.join(", ")
    )))
}

/// The failure of the invariant "a row that unified fills every slot".
fn unbound_slot_internal(iri: &str) -> EvalError {
    EvalError::internal(format!(
        "property function <{iri}>: a unified row left an argument slot unbound"
    ))
}

/// The value of one argument position for this invocation, or `None` when the position
/// is free.
fn arg_value(arg: &Arg, seed: &[Option<TermValue>]) -> Option<TermValue> {
    match arg {
        Arg::Constant(value) => Some(value.clone()),
        Arg::Slot(slot) => seed[*slot].clone(),
        // A quoted-triple argument is bound only when every component is: a partly-bound
        // triple term denotes no single value, so the position is free and the relation
        // is asked to produce one (which `unify_term` then matches structurally).
        Arg::Triple(parts) => {
            let s = arg_value(&parts[0], seed)?;
            let p = arg_value(&parts[1], seed)?;
            let o = arg_value(&parts[2], seed)?;
            Some(TermValue::Triple {
                s: Box::new(s),
                p: Box::new(p),
                o: Box::new(o),
            })
        }
    }
}

/// Match one emitted row against the call's argument positions, binding free slots and
/// comparing everything already fixed.
///
/// `false` means the row does not agree — with a constant, with a bound input, or with
/// an earlier occurrence of the same variable. `values` may have been partly written
/// when that happens; the caller discards it (it is re-seeded per row).
fn unify_row(args: &[Arg], emitted: &[TermValue], values: &mut [Option<TermValue>]) -> bool {
    args.iter()
        .zip(emitted)
        .all(|(arg, value)| unify_term(arg, value, values))
}

/// [`unify_row`] for one position (recursing through quoted triples).
fn unify_term(arg: &Arg, value: &TermValue, values: &mut [Option<TermValue>]) -> bool {
    match arg {
        Arg::Constant(constant) => constant == value,
        Arg::Slot(slot) => match &values[*slot] {
            Some(existing) => existing == value,
            None => {
                values[*slot] = Some(value.clone());
                true
            }
        },
        Arg::Triple(parts) => match value {
            TermValue::Triple { s, p, o } => {
                unify_term(&parts[0], s, values)
                    && unify_term(&parts[1], p, values)
                    && unify_term(&parts[2], o, values)
            }
            _ => false,
        },
    }
}

// ---------------------------------------------------------------------------
// The forwarding refusal
// ---------------------------------------------------------------------------

/// Whether `pattern` reaches a property-function call anywhere — including inside an
/// expression-embedded `EXISTS`, which the algebra crate's own serialization-facing
/// walk deliberately does not descend into.
///
/// Used at the `SERVICE` forwarding boundary: a call serializes as an ordinary triple,
/// so a remote endpoint would silently match it against ITS data and return rows that
/// are not the relation's. That is a wrong answer with no symptom, so the forwarding
/// refuses instead.
pub(crate) fn pattern_reaches_property_function(pattern: &GraphPattern) -> bool {
    if matches!(pattern, GraphPattern::PropertyFunction(_)) {
        return true;
    }
    let mut found = false;
    crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
        found |= match part {
            crate::governor::soundness::PatternPart::Child(child, _edge) => {
                pattern_reaches_property_function(child)
            }
            crate::governor::soundness::PatternPart::Expression(expr) => {
                expression_reaches_property_function(expr)
            }
        };
        found
    });
    found
}

/// [`pattern_reaches_property_function`] through an expression's embedded patterns.
pub(crate) fn expression_reaches_property_function(
    expr: &purrdf_sparql_algebra::Expression,
) -> bool {
    let mut found = false;
    crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
        found |= match part {
            crate::governor::soundness::ExpressionPart::Exists(pattern) => {
                pattern_reaches_property_function(pattern)
            }
            crate::governor::soundness::ExpressionPart::Sub(inner) => {
                expression_reaches_property_function(inner)
            }
            crate::governor::soundness::ExpressionPart::Call(_) => false,
        };
        found
    });
    found
}

/// Whether `pattern` reaches a `GROUP BY` with an [`AggregateFunction::Custom`]
/// aggregate anywhere — including inside an expression-embedded `EXISTS`.
///
/// Shares the soundness-visitor idiom [`pattern_reaches_property_function`]
/// establishes, but for a different reason at each of its two call sites:
///
/// * `crate::property_fn_plan::plan_query`'s prepare-time walk uses it to decide
///   whether a query needs planning AT ALL when it carries no property-function
///   call either — without this check, a query with a `Custom` aggregate and NO
///   property function would skip `plan_query`'s whole walk via its
///   `pattern_reaches_property_function`-only short-circuit, and the unregistered-
///   IRI/arity admission [`crate::property_fn_plan::plan_aggregate`] performs
///   would never run.
/// * `crate::remote::eval_service` uses it for the SAME reason
///   [`pattern_reaches_property_function`] exists: a `Custom` aggregate inside a
///   `SERVICE` body would serialize as an ordinary `AGG`-shaped call the endpoint
///   cannot know, or worse, an `AGG(<iri>, …)` textual form it silently mishandles
///   — either way a wrong answer with no local symptom.
pub(crate) fn pattern_reaches_custom_aggregate(pattern: &GraphPattern) -> bool {
    if let GraphPattern::Group { aggregates, .. } = pattern
        && aggregates
            .iter()
            .any(|(_, aggregate)| matches!(aggregate.function, AggregateFunction::Custom(_)))
    {
        return true;
    }
    let mut found = false;
    crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
        found |= match part {
            crate::governor::soundness::PatternPart::Child(child, _edge) => {
                pattern_reaches_custom_aggregate(child)
            }
            crate::governor::soundness::PatternPart::Expression(expr) => {
                expression_reaches_custom_aggregate(expr)
            }
        };
        found
    });
    found
}

/// [`pattern_reaches_custom_aggregate`] through an expression's embedded patterns
/// (an `EXISTS`'s inner `GROUP BY`, e.g. `FILTER EXISTS { SELECT (AGG(<iri>,?x)
/// AS ?v) WHERE {...} GROUP BY ?g }`).
///
/// `pub(crate)`: also read directly by `crate::property_fn_plan::plan_expression`'s
/// own short-circuit, for the identical reason `plan_where_pattern` reads
/// [`pattern_reaches_custom_aggregate`] — an expression containing an `EXISTS`
/// whose inner pattern has a `Custom` aggregate but no property-function call
/// must not skip the walk that reaches that aggregate's prepare-time admission.
pub(crate) fn expression_reaches_custom_aggregate(
    expr: &purrdf_sparql_algebra::Expression,
) -> bool {
    let mut found = false;
    crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
        found |= match part {
            crate::governor::soundness::ExpressionPart::Exists(pattern) => {
                pattern_reaches_custom_aggregate(pattern)
            }
            crate::governor::soundness::ExpressionPart::Sub(inner) => {
                expression_reaches_custom_aggregate(inner)
            }
            crate::governor::soundness::ExpressionPart::Call(_) => false,
        };
        found
    });
    found
}

/// Whether `pattern` reaches a [`Function::Custom`] scalar-function call anywhere
/// — including inside an expression-embedded `EXISTS`.
///
/// Used ONLY at the `SERVICE` forwarding boundary
/// ([`crate::remote::eval_service`]): a `Custom` call serializes as an ordinary
/// function-call syntax the remote endpoint does not define, so `SILENT` would
/// launder a request that could never mean what it meant locally into an
/// endpoint-side syntax error (best case) or a same-spelled-but-different builtin
/// on the remote engine (worst case, and silent). Unlike
/// [`pattern_reaches_custom_aggregate`], prepare-time admission has no analogous
/// need for this walk: an unresolved [`Function::Custom`] IRI already fails
/// LOUDLY at evaluation time (an XSD-cast attempt or a typed "undefined function"
/// error — see `crate::expr`), so there is no silent-empty-answer hazard for
/// `crate::property_fn_plan` to close the way there is for a relation's predicate
/// or a `Custom` aggregate's registry mismatch.
pub(crate) fn pattern_reaches_custom_function(pattern: &GraphPattern) -> bool {
    let mut found = false;
    crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
        found |= match part {
            crate::governor::soundness::PatternPart::Child(child, _edge) => {
                pattern_reaches_custom_function(child)
            }
            crate::governor::soundness::PatternPart::Expression(expr) => {
                expression_reaches_custom_function(expr)
            }
        };
        found
    });
    found
}

/// [`pattern_reaches_custom_function`] through an expression's embedded patterns.
fn expression_reaches_custom_function(expr: &purrdf_sparql_algebra::Expression) -> bool {
    let mut found = false;
    crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
        found |= match part {
            crate::governor::soundness::ExpressionPart::Sub(inner) => {
                expression_reaches_custom_function(inner)
            }
            crate::governor::soundness::ExpressionPart::Exists(pattern) => {
                pattern_reaches_custom_function(pattern)
            }
            crate::governor::soundness::ExpressionPart::Call(f) => matches!(f, Function::Custom(_)),
        };
        found
    });
    found
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use purrdf_core::{
        RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult,
        TermValue,
    };

    use super::*;
    use crate::engine::NativeSparqlEngine;
    use crate::error::EvalError;
    use crate::property_fn::{MemoryRelation, PfCursor, PfRow, PropertyFunctionRegistry};
    use crate::user_fn::Volatility;

    const EX: &str = "http://example.org/";
    const PF_SPLIT: &str = "http://example.org/pf/split";
    const PF_CONTAINS: &str = "http://example.org/pf/contains";
    const PF_LOOKUP: &str = "http://example.org/pf/lookup";
    const PF_TAG: &str = "http://example.org/pf/tag";

    // ---- fixtures ---------------------------------------------------------

    /// `ex:d1 ex:section ex:intro`, `ex:d2 ex:section ex:appendix`,
    /// `ex:d1 ex:kind "report"`.
    fn documents() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let section = b.intern_iri(&format!("{EX}section"));
        let kind = b.intern_iri(&format!("{EX}kind"));
        let d1 = b.intern_iri(&format!("{EX}d1"));
        let d2 = b.intern_iri(&format!("{EX}d2"));
        let intro = b.intern_iri(&format!("{EX}intro"));
        let appendix = b.intern_iri(&format!("{EX}appendix"));
        b.push_quad(d1, section, intro, None);
        b.push_quad(d2, section, appendix, None);
        let report = b.intern_literal(RdfLiteral::simple("report".to_owned()));
        b.push_quad(d1, kind, report, None);
        b.freeze().expect("freeze")
    }

    fn iri(local: &str) -> TermValue {
        TermValue::iri(format!("{EX}{local}"))
    }

    fn text(value: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: value.to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            language: None,
            direction: None,
        }
    }

    /// The two-row reference table `( ex:a ex:1 ) ( ex:b ex:2 )`, all modes served.
    fn split_table() -> MemoryRelation {
        MemoryRelation::new(
            1,
            1,
            vec![vec![iri("a"), iri("1")], vec![iri("b"), iri("2")]],
        )
        .expect("uniform rows")
    }

    fn registry_of(entries: Vec<(&str, Arc<dyn PropertyFunction>)>) -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        for (iri, relation) in entries {
            registry.register(iri, relation);
        }
        registry
    }

    fn split_registry() -> PropertyFunctionRegistry {
        registry_of(vec![(PF_SPLIT, Arc::new(split_table()))])
    }

    /// Render one result cell: `<iri>` for an IRI, the lexical form for a literal,
    /// `_:label` for a blank, `UNBOUND` for an absent binding.
    fn cell(value: Option<&TermValue>) -> String {
        match value {
            None => "UNBOUND".to_owned(),
            Some(TermValue::Iri(i)) => format!("<{i}>"),
            Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
            Some(TermValue::Blank { label, .. }) => format!("_:{label}"),
            Some(other) => format!("{other:?}"),
        }
    }

    /// Evaluate `query` over `dataset` with `registry` injected, in RESULT ORDER (never
    /// sorted — emission order is the property under test).
    fn run_on(
        dataset: &Arc<RdfDataset>,
        query: &str,
        registry: &PropertyFunctionRegistry,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
        let engine = NativeSparqlEngine::new();
        let result = engine
            .query_with_property_functions(
                dataset,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                registry,
            )
            .map_err(|e| e.to_string())?;
        match result {
            SparqlResult::Solutions {
                variables, rows, ..
            } => Ok((
                variables,
                rows.iter()
                    .map(|row| row.iter().map(|c| cell(c.as_ref())).collect())
                    .collect(),
            )),
            SparqlResult::Boolean(b) => Ok((vec![], vec![vec![b.to_string()]])),
            SparqlResult::Graph(_) => Ok((vec![], vec![])),
        }
    }

    fn rows_of(query: &str, registry: &PropertyFunctionRegistry) -> Vec<Vec<String>> {
        run_on(&documents(), query, registry)
            .expect("query evaluates")
            .1
    }

    fn error_of(query: &str, registry: &PropertyFunctionRegistry) -> String {
        run_on(&documents(), query, registry).expect_err("query must be refused")
    }

    // ---- test relations ---------------------------------------------------

    /// A relation whose declared modes are fixed by the constructor, recording the
    /// access pattern of every invocation it is opened with.
    #[derive(Debug)]
    struct RecordingRelation {
        arity: PfArity,
        modes: Vec<BindingPattern>,
        rows: Vec<PfRow>,
        /// Rows emitted per invocation under each declared mode, for tie-breaking.
        row_bound: u64,
        invocations: Mutex<Vec<String>>,
        volatility: Volatility,
    }

    impl RecordingRelation {
        fn new(subject: usize, object: usize, modes: &[&str], rows: Vec<PfRow>) -> Self {
            let row_bound = rows.len() as u64;
            Self {
                arity: PfArity::new(subject, object),
                modes: modes.iter().map(|m| BindingPattern::from_code(m)).collect(),
                rows,
                row_bound,
                invocations: Mutex::new(Vec::new()),
                volatility: Volatility::Stable,
            }
        }

        /// Override the declared row bound, which is the ORDERING pass's first
        /// tie-break — so two registries can declare different costs for identical
        /// tables.
        fn with_row_bound(mut self, bound: u64) -> Self {
            self.row_bound = bound;
            self
        }

        fn calls(&self) -> Vec<String> {
            self.invocations.lock().expect("uncontended").clone()
        }
    }

    impl PropertyFunction for RecordingRelation {
        fn volatility(&self) -> Volatility {
            self.volatility
        }

        fn arity(&self) -> PfArity {
            self.arity
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            self.row_bound
        }

        fn open(
            &self,
            args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            self.invocations
                .lock()
                .expect("uncontended")
                .push(args.mode().code());
            // The table is emitted VERBATIM, bound positions and all: a relation is
            // entitled to emit candidates and let the engine's equality filter cut the
            // ones that disagree, and this fixture exercises exactly that.
            Ok(Box::new(VecCursor {
                rows: self.rows.clone(),
                next: 0,
            }))
        }
    }

    /// A cursor over a fixed row vector.
    #[derive(Debug)]
    struct VecCursor {
        rows: Vec<PfRow>,
        next: usize,
    }

    impl PfCursor for VecCursor {
        fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
            let row = self.rows.get(self.next).cloned();
            self.next += 1;
            Ok(row)
        }
    }

    /// A relation that records the row ceiling of every invocation it is opened with.
    ///
    /// It emits its fixture rows with every BOUND position echoed back, so nothing it
    /// produces is dropped for disagreeing with an input — which keeps these tests about
    /// the ceiling and not about the equality filter.
    #[derive(Debug)]
    struct CeilingSpy {
        modes: [BindingPattern; 1],
        rows: Vec<PfRow>,
        echo_bound: bool,
        ceilings: Mutex<Vec<Option<u64>>>,
    }

    impl CeilingSpy {
        /// A 1/1 relation emitting `rows`, echoing bound positions.
        fn new(rows: Vec<PfRow>) -> Self {
            Self {
                modes: [PfArity::new(1, 1).all_free_mode()],
                rows,
                echo_bound: true,
                ceilings: Mutex::new(Vec::new()),
            }
        }

        /// The same, emitting its rows VERBATIM — for the repeated-variable call, where
        /// the point is that the engine's own filter cuts some of them.
        fn verbatim(rows: Vec<PfRow>) -> Self {
            Self {
                echo_bound: false,
                ..Self::new(rows)
            }
        }

        /// One row per invocation, `ex:r0`, for the shape tests.
        fn single() -> Self {
            Self::new(vec![vec![iri("w0"), iri("r0")]])
        }

        fn ceilings(&self) -> Vec<Option<u64>> {
            self.ceilings.lock().expect("uncontended").clone()
        }
    }

    impl PropertyFunction for CeilingSpy {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            PfArity::new(1, 1)
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            self.rows.len() as u64
        }

        fn open(
            &self,
            args: &PfArgs<'_>,
            ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            self.ceilings.lock().expect("uncontended").push(ceiling);
            let rows = self
                .rows
                .iter()
                .map(|row| {
                    row.iter()
                        .enumerate()
                        .map(|(position, value)| match args.get(position) {
                            Some(bound) if self.echo_bound => bound.clone(),
                            Some(_) | None => value.clone(),
                        })
                        .collect()
                })
                .collect();
            Ok(Box::new(VecCursor { rows, next: 0 }))
        }
    }

    /// A relation that MINTS a term absent from the dataset: `?doc ex:tag ?label`
    /// emits the document's own IRI paired with a freshly-built literal.
    #[derive(Debug)]
    struct TaggingRelation {
        modes: [BindingPattern; 1],
    }

    impl TaggingRelation {
        fn new() -> Self {
            Self {
                modes: [PfArity::new(1, 1).all_free_mode()],
            }
        }
    }

    impl PropertyFunction for TaggingRelation {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            PfArity::new(1, 1)
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            1
        }

        fn open(
            &self,
            args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            let Some(TermValue::Iri(subject)) = args.get(0).cloned() else {
                // Nothing to tag: an unbound (or non-IRI) subject yields no rows.
                return Ok(Box::new(VecCursor {
                    rows: Vec::new(),
                    next: 0,
                }));
            };
            let label = text(&format!("tag:{subject}"));
            Ok(Box::new(VecCursor {
                rows: vec![vec![TermValue::Iri(subject), label]],
                next: 0,
            }))
        }
    }

    // ---- adornments -------------------------------------------------------

    #[test]
    fn ff_enumerates_the_whole_relation_in_emission_order() {
        let rows = rows_of(
            &format!("SELECT ?w ?p WHERE {{ ?w <{PF_SPLIT}> ?p }}"),
            &split_registry(),
        );
        assert_eq!(
            rows,
            vec![
                vec![
                    "<http://example.org/a>".to_owned(),
                    "<http://example.org/1>".to_owned()
                ],
                vec![
                    "<http://example.org/b>".to_owned(),
                    "<http://example.org/2>".to_owned()
                ],
            ]
        );
    }

    #[test]
    fn bf_looks_up_from_a_bound_subject() {
        let rows = rows_of(
            &format!("SELECT ?p WHERE {{ <{EX}b> <{PF_SPLIT}> ?p }}"),
            &split_registry(),
        );
        assert_eq!(rows, vec![vec!["<http://example.org/2>".to_owned()]]);
    }

    #[test]
    fn fb_looks_up_from_a_bound_object() {
        let rows = rows_of(
            &format!("SELECT ?w WHERE {{ ?w <{PF_SPLIT}> <{EX}1> }}"),
            &split_registry(),
        );
        assert_eq!(rows, vec![vec!["<http://example.org/a>".to_owned()]]);
    }

    #[test]
    fn bb_is_a_membership_test() {
        let registry = split_registry();
        assert_eq!(
            rows_of(
                &format!("ASK {{ <{EX}a> <{PF_SPLIT}> <{EX}1> }}"),
                &registry
            ),
            vec![vec!["true".to_owned()]]
        );
        assert_eq!(
            rows_of(
                &format!("ASK {{ <{EX}a> <{PF_SPLIT}> <{EX}2> }}"),
                &registry
            ),
            vec![vec!["false".to_owned()]]
        );
    }

    // ---- the FTS needle shape ---------------------------------------------

    /// `?doc ex:contains ("needle" ?score)` written BEFORE the data pattern that binds
    /// `?doc`, against a relation that can only be invoked with both the document and
    /// the needle bound. Textual order is infeasible; the prepare-time ordering makes
    /// it feasible, and the relation is invoked once per document with the needle bound.
    #[test]
    fn the_needle_shape_is_ordered_so_the_relation_is_invoked_bound() {
        let relation = Arc::new(RecordingRelation::new(
            1,
            2,
            // Subject (the document) and the needle must be bound; only the score is
            // computed.
            &["bbf"],
            vec![vec![iri("d1"), text("needle"), text("0.75")]],
        ));
        let registry = registry_of(vec![(PF_CONTAINS, relation.clone())]);
        let rows = rows_of(
            &format!(
                "SELECT ?doc ?score WHERE {{ \
                 ?doc <{PF_CONTAINS}> (\"needle\" ?score) . \
                 ?doc <{EX}section> <{EX}intro> }}"
            ),
            &registry,
        );
        assert_eq!(
            rows,
            vec![vec![
                "<http://example.org/d1>".to_owned(),
                "0.75".to_owned()
            ]],
            "the score the relation computed reaches the answer"
        );
        assert_eq!(
            relation.calls(),
            vec!["bbf".to_owned()],
            "exactly one invocation, per document the data pattern bound, with the \
             document AND the needle bound"
        );
    }

    #[test]
    fn a_needle_relation_that_can_enumerate_is_still_invoked_once_per_document() {
        // The same query against a relation that ALSO declares the document-free mode:
        // the ordering still schedules the data pattern first, because a data atom is
        // always at least as bound-making as a call and can never be infeasible.
        let relation = Arc::new(RecordingRelation::new(
            1,
            2,
            &["fbf"],
            vec![vec![iri("d1"), text("needle"), text("0.75")]],
        ));
        let registry = registry_of(vec![(PF_CONTAINS, relation.clone())]);
        let rows = rows_of(
            &format!(
                "SELECT ?doc ?score WHERE {{ \
                 ?doc <{PF_CONTAINS}> (\"needle\" ?score) . \
                 ?doc <{EX}section> <{EX}intro> }}"
            ),
            &registry,
        );
        assert_eq!(
            rows,
            vec![vec![
                "<http://example.org/d1>".to_owned(),
                "0.75".to_owned()
            ]]
        );
        assert_eq!(relation.calls(), vec!["bbf".to_owned()]);
    }

    // ---- the row ceiling --------------------------------------------------

    #[test]
    fn a_correlated_call_is_opened_with_the_lateral_s_row_ceiling() {
        // The shape the parser builds for a call written after a data pattern:
        // `Lateral(Bgp, PropertyFunction)`. The ceiling belongs to the Lateral — the
        // dispatch emits that node's output rows — and reaches the relation through the
        // fusion. Before this it never arrived at all, and the "tell the index that ten
        // rows matter" licence fired only for a call standing on its own.
        let spy = Arc::new(CeilingSpy::single());
        let registry = registry_of(vec![(PF_SPLIT, spy.clone())]);
        let rows = rows_of(
            &format!("SELECT ?s ?x WHERE {{ ?s <{EX}section> ?o . ?s <{PF_SPLIT}> ?x }} LIMIT 2"),
            &registry,
        );
        assert_eq!(rows.len(), 2, "two documents, one relation row each");
        assert_eq!(
            spy.ceilings(),
            vec![Some(2), Some(1)],
            "the first invocation may serve the whole LIMIT; the second is offered what \
             the first left, because the accumulated bag is the node's output"
        );
    }

    #[test]
    fn a_standalone_call_is_opened_with_its_own_row_ceiling() {
        // The other shape: nothing written before the call, so the node IS the plan node
        // the ceiling was recorded at. It is driven over the identity table, hence one
        // invocation.
        let spy = Arc::new(CeilingSpy::new(vec![
            vec![iri("w0"), iri("r0")],
            vec![iri("w1"), iri("r1")],
            vec![iri("w2"), iri("r2")],
        ]));
        let registry = registry_of(vec![(PF_SPLIT, spy.clone())]);
        let rows = rows_of(
            &format!("SELECT ?w ?p WHERE {{ ?w <{PF_SPLIT}> ?p }} LIMIT 2"),
            &registry,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(spy.ceilings(), vec![Some(2)]);
    }

    #[test]
    fn a_call_under_a_sort_is_offered_no_ceiling() {
        // `ORDER BY` + `LIMIT` is a top-k problem: the row that sorts first can be
        // produced last, so no prefix of this node's output is the answer's prefix and
        // the plan licenses nothing. The guard that withholds it is the same certificate
        // that grants it above — see `crate::governor::soundness`.
        let spy = Arc::new(CeilingSpy::single());
        let registry = registry_of(vec![(PF_SPLIT, spy.clone())]);
        let rows = rows_of(
            &format!(
                "SELECT ?s ?x WHERE {{ ?s <{EX}section> ?o . ?s <{PF_SPLIT}> ?x }} \
                 ORDER BY ?s LIMIT 2"
            ),
            &registry,
        );
        assert_eq!(
            rows.len(),
            2,
            "the answer is unchanged; only the licence is"
        );
        assert_eq!(
            spy.ceilings(),
            vec![None, None],
            "a ceiling under a sort would let a relation stop before producing the row \
             that sorts first"
        );
    }

    #[test]
    fn a_repeated_variable_withholds_the_ceiling_from_the_relation() {
        // `?x <rel> ?x` hands the relation two FREE positions. It cannot know they must
        // agree, so a licence to stop after `k` rows would be counted against rows this
        // engine then drops — and a stop at the first, dropped, row would report an empty
        // answer for a query that has one.
        let spy = Arc::new(CeilingSpy::verbatim(vec![
            vec![iri("a"), iri("1")],
            vec![iri("c"), iri("c")],
        ]));
        let registry = registry_of(vec![(PF_SPLIT, spy.clone())]);
        let rows = rows_of(
            &format!("SELECT ?x WHERE {{ ?x <{PF_SPLIT}> ?x }} LIMIT 1"),
            &registry,
        );
        assert_eq!(rows, vec![vec!["<http://example.org/c>".to_owned()]]);
        assert_eq!(
            spy.ceilings(),
            vec![None],
            "the engine withholds a ceiling it would have to trust the relation to \
             account for against something the relation was never told"
        );

        // The same call against the reference relation, which DOES honour a ceiling: the
        // answer is the same, which it would not be if the ceiling had been offered.
        let table = MemoryRelation::new(
            1,
            1,
            vec![vec![iri("a"), iri("1")], vec![iri("c"), iri("c")]],
        )
        .expect("uniform rows");
        let rows = rows_of(
            &format!("SELECT ?x WHERE {{ ?x <{PF_SPLIT}> ?x }} LIMIT 1"),
            &registry_of(vec![(PF_SPLIT, Arc::new(table))]),
        );
        assert_eq!(
            rows,
            vec![vec!["<http://example.org/c>".to_owned()]],
            "a relation that stopped after one emitted row would have emitted only the \
             row the repeated variable rejects, and the answer would be empty"
        );
    }

    #[test]
    fn a_ceiling_honouring_relation_answers_the_same_query() {
        // The end-to-end oracle for the licence: a hundred-row table under a LIMIT 3,
        // correlated so the ceiling arrives through the fused Lateral. The reference
        // relation stops generating at the ceiling; the answer must still be the first
        // three rows the unbounded scan would have produced.
        let rows: Vec<PfRow> = (0..100)
            .map(|i| vec![iri("d1"), iri(&format!("r{i:03}"))])
            .collect();
        let table = MemoryRelation::new(1, 1, rows).expect("uniform rows");
        let registry = registry_of(vec![(PF_SPLIT, Arc::new(table))]);
        let query = format!("SELECT ?x WHERE {{ ?s <{EX}kind> ?k . ?s <{PF_SPLIT}> ?x }}");

        let all = rows_of(&query, &registry);
        assert_eq!(all.len(), 100, "the unbounded answer is the whole table");

        let limited = rows_of(&format!("{query} LIMIT 3"), &registry);
        assert_eq!(
            limited,
            all[..3].to_vec(),
            "the ceiling changes how much of the table is scanned, never the answer"
        );
    }

    // ---- filtering, order, consistency ------------------------------------

    #[test]
    fn a_bound_output_position_filters_the_relation_s_rows() {
        // The relation ignores its bound positions and emits both table rows every
        // time; the engine's equality filter is what cuts the mismatch.
        let relation = Arc::new(RecordingRelation::new(
            1,
            1,
            &["ff"],
            vec![vec![iri("a"), iri("1")], vec![iri("b"), iri("2")]],
        ));
        let registry = registry_of(vec![(PF_SPLIT, relation)]);
        let rows = rows_of(
            &format!("SELECT ?w WHERE {{ ?w <{PF_SPLIT}> <{EX}2> }}"),
            &registry,
        );
        assert_eq!(
            rows,
            vec![vec!["<http://example.org/b>".to_owned()]],
            "the row whose object is ex:1 disagrees with the bound object and is dropped"
        );
    }

    #[test]
    fn multi_row_output_keeps_the_relation_s_emission_order() {
        let relation = Arc::new(RecordingRelation::new(
            0,
            1,
            &["f"],
            vec![vec![iri("z")], vec![iri("a")], vec![iri("m")]],
        ));
        let registry = registry_of(vec![(PF_SPLIT, relation)]);
        let rows = rows_of(
            &format!("SELECT ?x WHERE {{ () <{PF_SPLIT}> ?x }}"),
            &registry,
        );
        assert_eq!(
            rows,
            vec![
                vec!["<http://example.org/z>".to_owned()],
                vec!["<http://example.org/a>".to_owned()],
                vec!["<http://example.org/m>".to_owned()],
            ],
            "emitted order is preserved verbatim, never sorted"
        );
    }

    #[test]
    fn a_repeated_variable_enforces_equality_and_filters_rather_than_errors() {
        let relation = Arc::new(RecordingRelation::new(
            1,
            1,
            &["ff"],
            vec![
                vec![iri("a"), iri("a")],
                vec![iri("a"), iri("b")],
                vec![iri("c"), iri("c")],
            ],
        ));
        let registry = registry_of(vec![(PF_SPLIT, relation)]);
        let rows = rows_of(
            &format!("SELECT ?x WHERE {{ ?x <{PF_SPLIT}> ?x }}"),
            &registry,
        );
        assert_eq!(
            rows,
            vec![
                vec!["<http://example.org/a>".to_owned()],
                vec!["<http://example.org/c>".to_owned()],
            ],
            "the inconsistent row is an ordinary non-match, not a failure"
        );
    }

    #[test]
    fn a_blank_node_argument_binds_consistently_and_is_projected_away() {
        let relation = Arc::new(RecordingRelation::new(
            1,
            2,
            &["fff"],
            vec![
                // The blank appears twice, so only a row agreeing with itself survives.
                vec![iri("a"), iri("a"), iri("1")],
                vec![iri("b"), iri("c"), iri("2")],
            ],
        ));
        let registry = registry_of(vec![(PF_SPLIT, relation)]);
        let (variables, rows) = run_on(
            &documents(),
            &format!("SELECT * WHERE {{ _:shared <{PF_SPLIT}> (_:shared ?out) }}"),
            &registry,
        )
        .expect("query evaluates");
        assert_eq!(
            variables,
            vec!["out".to_owned()],
            "a blank-node argument is a non-distinguished variable: it is never a column"
        );
        assert_eq!(rows, vec![vec!["<http://example.org/1>".to_owned()]]);
    }

    // ---- admission --------------------------------------------------------

    #[test]
    fn an_unregistered_iri_under_a_configured_namespace_is_refused_before_evaluation() {
        // A CALLER-DECLARED namespace (not the registry-derived exact-IRI set) still
        // claims every IRI under it: a call whose specific IRI nothing supplies is
        // refused before evaluation.
        let engine = NativeSparqlEngine::new().with_parser_options(crate::ParserOptions {
            extension_fn_namespaces: vec![],
            property_fn_namespaces: vec![format!("{EX}pf/")],
            property_fn_iris: Vec::new(),
        });
        let error = engine
            .query(
                &documents(),
                SparqlRequest {
                    query: &format!("SELECT ?w WHERE {{ ?w <{PF_SPLIT}x> ?p }}"),
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect_err("nothing is registered under the configured namespace");
        assert_eq!(error.code, "native-sparql-property-function");
        let message = error.to_string();
        assert!(
            message.contains("no property function is registered")
                && message.contains(&format!("{PF_SPLIT}x")),
            "got {message}"
        );
    }

    /// THE GAP-3 regression, at the registry-injection layer this module owns:
    /// registering `PF_SPLIT` must not hijack the DIFFERENT, merely
    /// prefix-sharing IRI `PF_SPLIT`+`x` into a call. `query_with_property_functions`
    /// (unlike the test above) configures NO caller namespace — the only seam in
    /// scope is the registry-derived exact-IRI set — so `PF_SPLIT`+`x` stays an
    /// ordinary triple pattern and the query evaluates against the graph, which
    /// holds no such predicate, rather than hard-erroring as unregistered.
    #[test]
    fn a_sibling_iri_that_merely_shares_a_registered_iri_s_prefix_is_ordinary_data() {
        let rows = rows_of(
            &format!("SELECT ?w ?p WHERE {{ ?w <{PF_SPLIT}x> ?p }}"),
            &split_registry(),
        );
        assert!(
            rows.is_empty(),
            "the dataset holds no <{PF_SPLIT}x> triple, and the call is never made: {rows:?}"
        );
    }

    #[test]
    fn a_call_with_no_registry_at_all_is_the_unregistered_case() {
        let engine = NativeSparqlEngine::new().with_parser_options(crate::ParserOptions {
            extension_fn_namespaces: vec![],
            property_fn_namespaces: vec![format!("{EX}pf/")],
            property_fn_iris: Vec::new(),
        });
        let error = engine
            .query(
                &documents(),
                SparqlRequest {
                    query: &format!("SELECT ?w WHERE {{ ?w <{PF_SPLIT}> ?p }}"),
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect_err("a configured namespace with nothing registered cannot evaluate");
        assert_eq!(error.code, "native-sparql-property-function");
        let message = error.to_string();
        assert!(
            message.contains("no property function is registered"),
            "got {message}"
        );
    }

    #[test]
    fn an_arity_mismatch_names_both_arities() {
        let message = error_of(
            &format!("SELECT ?w WHERE {{ ?w <{PF_SPLIT}> (?p ?q) }}"),
            &split_registry(),
        );
        assert!(
            message.contains("declared with 1 subject / 1 object")
                && message.contains("supplies 1 subject / 2 object"),
            "got {message}"
        );
    }

    #[test]
    fn a_chain_with_no_feasible_order_names_the_stuck_call() {
        // The relation can only be invoked with its subject bound, and nothing in the
        // group can ever bind it.
        let relation = Arc::new(RecordingRelation::new(
            1,
            1,
            &["bf"],
            vec![vec![iri("a"), iri("1")]],
        ));
        let registry = registry_of(vec![(PF_LOOKUP, relation)]);
        let message = error_of(
            &format!("SELECT ?w ?p WHERE {{ ?w <{PF_LOOKUP}> ?p }}"),
            &registry,
        );
        assert!(
            message.contains("no feasible evaluation order"),
            "got {message}"
        );
        assert!(message.contains(PF_LOOKUP), "got {message}");
        assert!(message.contains("`ff`"), "got {message}");
        assert!(message.contains("free position(s) 0, 1"), "got {message}");
        assert!(message.contains("[bf]"), "got {message}");
    }

    #[test]
    fn a_sibling_pattern_makes_the_same_call_feasible() {
        // The identical relation and query, with one data pattern that binds `?w`: the
        // order exists, so the query runs. This is what proves the refusal above is
        // about feasibility rather than about the relation.
        let relation = Arc::new(RecordingRelation::new(
            1,
            1,
            &["bf"],
            vec![vec![iri("d1"), iri("1")]],
        ));
        let registry = registry_of(vec![(PF_LOOKUP, relation.clone())]);
        let rows = rows_of(
            &format!("SELECT ?p WHERE {{ ?w <{PF_LOOKUP}> ?p . ?w <{EX}section> <{EX}intro> }}"),
            &registry,
        );
        assert_eq!(rows, vec![vec!["<http://example.org/1>".to_owned()]]);
        assert_eq!(relation.calls(), vec!["bf".to_owned()]);
    }

    /// THE GAP-4 regression: a call NESTED inside an earlier atom's own subtree (here,
    /// a `UNION` arm's own one-call chain) must be planned against the bound set THAT
    /// ATOM was itself CHOSEN against — never the fully-accumulated set left behind
    /// once every atom in the enclosing chain has committed.
    ///
    /// The outer chain has three atoms, in this TEXTUAL order: an unrelated, always-
    /// feasible call to `PF_SPLIT` (written first so its own `Lateral` anchors the
    /// chain the parser assembles — this is what makes the group a `collect_chain`
    /// chain at all, rather than an opaque node `map_children` recurses into
    /// structurally, which is already correct and does not exercise this bug); a
    /// `UNION` whose each arm is its own nested call to `PF_LOOKUP` (subject-bound
    /// only, `bf`); and a `Bgp` that binds `?x`. The two non-call atoms (`UNION`,
    /// `Bgp`) tie on the ordering key's first two components — a data atom always
    /// sorts ahead of a call — so the tie-break falls to TEXTUAL position and the
    /// `UNION` is scheduled BEFORE the `Bgp` that binds `?x`. The nested call inside
    /// the `UNION`'s arms therefore genuinely cannot see `?x` bound: nothing the outer
    /// chain has committed by the time it runs supplies it.
    ///
    /// Before the fix, the rebuild loop planned every atom — including the `UNION` —
    /// against the FINAL bound set (which, by the time the selection loop finished,
    /// held `?x` from the `Bgp` atom committed AFTER it). The nested call was wrongly
    /// admitted at prepare time as feasible, an admission the evaluator cannot honor:
    /// `?x` is unbound when the `UNION`'s `Lateral` actually runs, and the failure
    /// surfaced later as a per-row `admit_mode` hard error instead of a prepare-time
    /// refusal. After the fix, each atom is planned against the snapshot `bound` held
    /// BEFORE it was chosen, so the nested call is (correctly) found infeasible and the
    /// whole query is refused HERE, at prepare time, naming `PF_LOOKUP` — never opening
    /// a cursor.
    #[test]
    fn a_call_nested_inside_an_earlier_atom_is_planned_against_its_own_bound_set() {
        let lookup = Arc::new(RecordingRelation::new(
            1,
            1,
            &["bf"],
            vec![vec![iri("d1"), iri("1")]],
        ));
        let trigger = Arc::new(RecordingRelation::new(0, 1, &["f"], vec![vec![iri("t1")]]));
        let registry = registry_of(vec![
            (PF_LOOKUP, lookup.clone() as Arc<dyn PropertyFunction>),
            (PF_SPLIT, trigger as Arc<dyn PropertyFunction>),
        ]);
        let message = error_of(
            &format!(
                "SELECT ?x ?p WHERE {{ \
                 () <{PF_SPLIT}> ?dummy . \
                 {{ ?x <{PF_LOOKUP}> ?p }} UNION {{ ?x <{PF_LOOKUP}> ?p }} . \
                 ?x <{EX}kind> \"report\" }}"
            ),
            &registry,
        );
        assert!(
            message.contains("no feasible evaluation order"),
            "the nested call must be refused at PREPARE time, not left to a per-row \
             `admit_mode` failure: got {message}"
        );
        assert!(message.contains(PF_LOOKUP), "got {message}");
        assert!(
            !message.contains("cannot serve the invocation"),
            "this must be the prepare-time ordering diagnostic, never the per-row \
             admit_mode message that a wrongly-admitted plan would surface instead: \
             got {message}"
        );
        assert!(
            lookup.calls().is_empty(),
            "a query refused at prepare time never opens a cursor: {:?}",
            lookup.calls()
        );
    }

    // ---- the seam is off by default ---------------------------------------

    #[test]
    fn with_no_registry_the_same_triple_is_an_ordinary_bgp_pattern() {
        // `ex:d1 ex:kind "report"` is data. Spelled with no property-function
        // configuration anywhere, it matches data exactly as it always did — and the
        // query text is identical to one that would be a call under a registry.
        let engine = NativeSparqlEngine::new();
        let query = format!("SELECT ?k WHERE {{ <{EX}d1> <{EX}kind> ?k }}");
        let plain = engine
            .query(
                &documents(),
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("an unconfigured engine evaluates it as data");
        let empty_registry = PropertyFunctionRegistry::new();
        let with_empty = run_on(&documents(), &query, &empty_registry).expect("evaluates");
        let plain_rows = match plain {
            SparqlResult::Solutions { rows, .. } => rows
                .iter()
                .map(|row| row.iter().map(|c| cell(c.as_ref())).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            other => panic!("expected solutions, got {other:?}"),
        };
        assert_eq!(plain_rows, vec![vec!["report".to_owned()]]);
        assert_eq!(
            with_empty.1, plain_rows,
            "an empty registry is indistinguishable from none"
        );
    }

    // ---- correlated EXISTS ------------------------------------------------

    #[test]
    fn a_call_inside_a_correlated_exists_sees_the_outer_binding() {
        let relation = Arc::new(RecordingRelation::new(
            1,
            1,
            &["bf"],
            vec![vec![iri("d1"), iri("1")]],
        ));
        let registry = registry_of(vec![(PF_LOOKUP, relation.clone())]);
        let rows = rows_of(
            &format!(
                "SELECT ?doc WHERE {{ ?doc <{EX}section> ?s . \
                 FILTER EXISTS {{ ?doc <{PF_LOOKUP}> ?p }} }}"
            ),
            &registry,
        );
        assert_eq!(
            rows,
            vec![vec!["<http://example.org/d1>".to_owned()]],
            "only the document the relation has a row for survives the EXISTS"
        );
        assert!(
            relation.calls().iter().all(|mode| mode == "bf"),
            "every invocation was made with the outer document bound: {:?}",
            relation.calls()
        );
    }

    // ---- SERVICE forwarding -----------------------------------------------

    #[test]
    fn a_call_inside_a_service_body_is_refused_at_the_forwarding_boundary() {
        let registry = split_registry();
        let message = error_of(
            &format!("SELECT ?w WHERE {{ SERVICE <{EX}endpoint> {{ ?w <{PF_SPLIT}> ?p }} }}"),
            &registry,
        );
        assert!(
            message.contains("property-function call inside a SERVICE body"),
            "got {message}"
        );
    }

    #[test]
    fn a_call_inside_a_silent_service_body_is_refused_too() {
        let registry = split_registry();
        let message = error_of(
            &format!(
                "SELECT ?w WHERE {{ SERVICE SILENT <{EX}endpoint> {{ ?w <{PF_SPLIT}> ?p }} }}"
            ),
            &registry,
        );
        assert!(
            message.contains("property-function call inside a SERVICE body"),
            "SILENT promises an empty result from a failed endpoint, never a wrong one \
             from a misread query: {message}"
        );
    }

    // ---- minting under a parallel arm -------------------------------------

    #[test]
    fn a_minted_term_survives_reinterning_under_a_forced_parallel_union() {
        // The relation mints a literal absent from the dataset, so the row that carries
        // it is a `Computed` scratch id. Under a forced-parallel `UNION` each arm runs
        // against its own forked context, and the minted cell must be re-interned into
        // the parent's scratch space (`parallel::portable_row` /
        // `reintern_minted_row`) — otherwise the id would resolve to the wrong value.
        let registry = registry_of(vec![(PF_TAG, Arc::new(TaggingRelation::new()))]);
        let query = format!(
            "SELECT ?doc ?label WHERE {{ \
             {{ ?doc <{EX}section> <{EX}intro> . ?doc <{PF_TAG}> ?label }} \
             UNION \
             {{ ?doc <{EX}section> <{EX}appendix> . ?doc <{PF_TAG}> ?label }} }}"
        );
        let dataset = documents();
        let parallel = {
            let _guard = crate::parallel::force_parallel_for_test(true);
            run_on(&dataset, &query, &registry).expect("evaluates")
        };
        let sequential = {
            let _guard = crate::parallel::force_parallel_for_test(false);
            run_on(&dataset, &query, &registry).expect("evaluates")
        };
        assert_eq!(
            parallel.1,
            vec![
                vec![
                    "<http://example.org/d1>".to_owned(),
                    "tag:http://example.org/d1".to_owned(),
                ],
                vec![
                    "<http://example.org/d2>".to_owned(),
                    "tag:http://example.org/d2".to_owned(),
                ],
            ],
            "each arm's minted label resolves to the value that arm produced"
        );
        assert_eq!(
            parallel, sequential,
            "forced-parallel and forced-sequential evaluation are byte-identical"
        );
    }

    // ---- truncation -------------------------------------------------------

    /// Evaluate `query` under `governors` with `registry` injected.
    fn run_governed(
        query: &str,
        registry: &PropertyFunctionRegistry,
        governors: &crate::governor::QueryGovernors,
    ) -> crate::GovernedOutcome {
        let engine = NativeSparqlEngine::new();
        let dataset = documents();
        let state = Arc::new(crate::governor::GovernorState::new(governors));
        engine
            .query_governed_in_operation(
                &*dataset,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                crate::QueryOptions {
                    property_functions: Some(registry),
                    ..crate::QueryOptions::EMPTY
                },
                &state,
            )
            .expect("a governor trip is an outcome, never an error")
    }

    #[test]
    fn a_row_budget_below_the_relation_s_output_truncates_deterministically() {
        let relation = Arc::new(RecordingRelation::new(
            0,
            1,
            &["f"],
            vec![vec![iri("r1")], vec![iri("r2")], vec![iri("r3")]],
        ));
        let registry = registry_of(vec![(PF_SPLIT, relation)]);
        let query = format!("SELECT ?x WHERE {{ () <{PF_SPLIT}> ?x }}");

        let complete = run_governed(&query, &registry, &crate::governor::QueryGovernors::METERED);
        assert!(complete.tripped().is_none(), "the metered run completes");

        // An answer cap below the relation's output: the answer is the FIRST two rows,
        // in emission order, and it is certified as a positional prefix.
        let capped = run_governed(
            &query,
            &registry,
            &crate::governor::QueryGovernors::UNBOUNDED.with_max_answers(2),
        );
        let crate::GovernedOutcome::BudgetExhausted(exhausted) = capped else {
            panic!("an answer cap below the relation's output must truncate");
        };
        let crate::PartialAnswers::Certain(partial) = &exhausted.partial else {
            panic!("a truncated leaf's rows are a certified lower bound: {exhausted:?}");
        };
        assert!(partial.is_positional_prefix());
        let SparqlResult::Solutions { rows, .. } = partial.result() else {
            panic!("expected solutions");
        };
        let rendered: Vec<Vec<String>> = rows
            .iter()
            .map(|row| row.iter().map(|c| cell(c.as_ref())).collect())
            .collect();
        assert_eq!(
            rendered,
            vec![
                vec!["<http://example.org/r1>".to_owned()],
                vec!["<http://example.org/r2>".to_owned()],
            ],
            "the prefix is the relation's first rows in its own emission order"
        );

        // Deterministic: the same budget over the same query is the same answer.
        let again = run_governed(
            &query,
            &registry,
            &crate::governor::QueryGovernors::UNBOUNDED.with_max_answers(2),
        );
        assert_eq!(
            again.tripped(),
            exhausted.evidence.tripped(),
            "the same budget trips the same governor"
        );
    }

    #[test]
    fn a_correlated_call_under_an_answer_cap_reports_the_same_prefix_as_an_uncapped_run() {
        // The answer cap reaches the fused dispatch as a row ceiling of cap + 1 — one row
        // past what the answer can use, which is what tells "exactly full" from
        // "overflowed". The differential statement is the one that matters: capping must
        // change how much of the relation runs, never WHICH rows come back.
        let relation = Arc::new(CeilingSpy::new(vec![
            vec![iri("d"), iri("r0")],
            vec![iri("d"), iri("r1")],
            vec![iri("d"), iri("r2")],
        ]));
        let registry = registry_of(vec![(PF_SPLIT, relation.clone())]);
        let query = format!("SELECT ?x WHERE {{ ?s <{EX}section> ?o . ?s <{PF_SPLIT}> ?x }}");

        let complete = rows_of(&query, &registry);
        assert_eq!(complete.len(), 6, "two documents, three relation rows each");
        assert_eq!(
            relation.ceilings(),
            vec![None, None],
            "the uncapped run drives both documents under no ceiling at all"
        );

        let capped = run_governed(
            &query,
            &registry,
            &crate::governor::QueryGovernors::UNBOUNDED.with_max_answers(2),
        );
        let crate::GovernedOutcome::BudgetExhausted(exhausted) = capped else {
            panic!("an answer cap below the correlated output must truncate");
        };
        let crate::PartialAnswers::Certain(partial) = &exhausted.partial else {
            panic!("the surviving rows are a certified lower bound: {exhausted:?}");
        };
        let SparqlResult::Solutions { rows, .. } = partial.result() else {
            panic!("expected solutions");
        };
        let rendered: Vec<Vec<String>> = rows
            .iter()
            .map(|row| row.iter().map(|c| cell(c.as_ref())).collect())
            .collect();
        assert_eq!(
            rendered,
            complete[..2].to_vec(),
            "the capped answer is the uncapped answer's first rows, verbatim"
        );
        assert_eq!(
            relation.ceilings(),
            vec![None, None, Some(3)],
            "the capped run added ONE invocation, opened at cap + 1: the ceiling stopped \
             the drive inside the first document's block instead of driving the second"
        );
    }

    // ---- admission --------------------------------------------------------

    #[test]
    fn a_relation_that_declares_more_rows_than_the_cell_ceiling_is_refused_before_it_runs() {
        // The cell ceiling is the one governor that must act BEFORE the work rather than
        // after it: by the time a meter can report a materialized bag, the bag is in
        // memory. A relation's bag is not predicted from any index, so the only prediction
        // there is is the relation's own declaration — which is exactly why the
        // declaration is held to an upper-bound honesty contract.
        let declared = 1_000_000_u64;
        let relation = Arc::new(
            RecordingRelation::new(0, 1, &["f"], vec![vec![iri("r1")], vec![iri("r2")]])
                .with_row_bound(declared),
        );
        let registry = registry_of(vec![(PF_SPLIT, relation.clone())]);
        let query = format!("SELECT ?x WHERE {{ () <{PF_SPLIT}> ?x }}");

        let refused = run_governed(
            &query,
            &registry,
            &crate::governor::QueryGovernors::UNBOUNDED.with_max_intermediate_cells(10),
        );
        assert_eq!(
            refused.tripped(),
            Some(TrippedGovernor::Refused {
                dimension: purrdf_core::ResourceDimension::IntermediateCells,
                limit: 10,
                // One flattened argument position, invoked once against the identity
                // table: the composition rule's leaf case, stated exactly.
                estimate: declared,
            }),
            "a declared bound above the ceiling is a refusal carrying the ESTIMATE, never a \
             consumption nothing measured"
        );
        assert!(
            relation.calls().is_empty(),
            "a refused plan opens no cursor: admission acts before the allocation, not after"
        );

        // The same query, the same ceiling, an honest small declaration: admitted, and the
        // relation runs. An estimate is what decides admission, so a truthful one is what
        // gets a caller their answer.
        let modest = Arc::new(
            RecordingRelation::new(0, 1, &["f"], vec![vec![iri("r1")], vec![iri("r2")]])
                .with_row_bound(2),
        );
        let admitted = run_governed(
            &query,
            &registry_of(vec![(PF_SPLIT, modest.clone())]),
            &crate::governor::QueryGovernors::UNBOUNDED.with_max_intermediate_cells(10),
        );
        assert_eq!(
            admitted.tripped(),
            None,
            "2 cells sit inside a 10-cell ceiling"
        );
        assert_eq!(
            modest.calls(),
            vec!["f".to_owned()],
            "the relation really ran"
        );
    }

    // ---- the plan cache ---------------------------------------------------

    #[test]
    fn two_registries_do_not_share_a_plan_cache_entry() {
        // Two calls, both feasible from the start, so the ORDER between them is decided
        // purely by the first tie-break: the lowest declared `rows_per_invocation`. The
        // two registries below carry identical tables and declare opposite costs, so the
        // plan differs while the query text does not — which is exactly the case a cache
        // key without the registry's fingerprint would get wrong.
        let engine = NativeSparqlEngine::new();
        let dataset = documents();
        let query = format!("SELECT ?x ?y WHERE {{ () <{PF_SPLIT}> ?x . () <{PF_LOOKUP}> ?y }}");
        fn request(query: &str) -> SparqlRequest<'_> {
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            }
        }
        let table = |first: &str, second: &str| vec![vec![iri(first)], vec![iri(second)]];
        let registry_with = |split_bound: u64, lookup_bound: u64| {
            registry_of(vec![
                (
                    PF_SPLIT,
                    Arc::new(
                        RecordingRelation::new(0, 1, &["f"], table("x1", "x2"))
                            .with_row_bound(split_bound),
                    ) as Arc<dyn PropertyFunction>,
                ),
                (
                    PF_LOOKUP,
                    Arc::new(
                        RecordingRelation::new(0, 1, &["f"], table("y1", "y2"))
                            .with_row_bound(lookup_bound),
                    ) as Arc<dyn PropertyFunction>,
                ),
            ])
        };
        let rows_under = |registry: &PropertyFunctionRegistry| {
            let result = engine
                .query_with_property_functions(&dataset, request(&query), registry)
                .expect("evaluates");
            let SparqlResult::Solutions { rows, .. } = result else {
                panic!("expected solutions");
            };
            rows.iter()
                .map(|row| row.iter().map(|c| cell(c.as_ref())).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };

        // `split` is the cheaper call, so it is the outer one: its rows vary slowest.
        let split_first = rows_under(&registry_with(1, 2));
        assert_eq!(
            split_first,
            vec![
                vec![
                    "<http://example.org/x1>".to_owned(),
                    "<http://example.org/y1>".to_owned()
                ],
                vec![
                    "<http://example.org/x1>".to_owned(),
                    "<http://example.org/y2>".to_owned()
                ],
                vec![
                    "<http://example.org/x2>".to_owned(),
                    "<http://example.org/y1>".to_owned()
                ],
                vec![
                    "<http://example.org/x2>".to_owned(),
                    "<http://example.org/y2>".to_owned()
                ],
            ]
        );

        // The SAME engine and the SAME query text, with the costs reversed: `lookup` is
        // now the outer call, so the row order is transposed. A stale plan would repeat
        // the order above.
        let lookup_first = rows_under(&registry_with(2, 1));
        assert_eq!(
            lookup_first,
            vec![
                vec![
                    "<http://example.org/x1>".to_owned(),
                    "<http://example.org/y1>".to_owned()
                ],
                vec![
                    "<http://example.org/x2>".to_owned(),
                    "<http://example.org/y1>".to_owned()
                ],
                vec![
                    "<http://example.org/x1>".to_owned(),
                    "<http://example.org/y2>".to_owned()
                ],
                vec![
                    "<http://example.org/x2>".to_owned(),
                    "<http://example.org/y2>".to_owned()
                ],
            ],
            "the second registry's declarations produced its own order"
        );

        // Re-running the first configuration reproduces its own order, so the two are
        // cached independently rather than the last one winning.
        assert_eq!(rows_under(&registry_with(1, 2)), split_first);
    }
}
