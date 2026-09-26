// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The guard every recursive evaluator entry passes through, refusing when the running
//! evaluation has too little stack left.
//!
//! The parser admits a request as deep as the stack of the thread parsing it holds, but
//! that does not bound the stack *evaluating* it needs: one written level of
//! `FILTER NOT EXISTS` costs the evaluator about 16.7 KB of wasm32 shadow stack, one of
//! `LATERAL` about 9.5 KB, and any other algebra level (`OPTIONAL`, `MINUS`, `BIND`, a
//! sibling spine) about 4.8 KB — far more than the parser spent on it — while the flat
//! and shallow requests need a fraction of any stack. Exhausting the stack is not an
//! error anywhere: natively the process aborts, and on `wasm32-unknown-unknown` the
//! shadow stack runs below its floor and traps with the instance's memory in an unknown
//! state.
//!
//! So every evaluator entry that can deepen the stack measures the stack actually left
//! and refuses with [`EvalError::StackExhausted`] — naming the construct — when it is
//! below [`purrdf_stack::MARGIN_BYTES`]. Nothing traps, and the thread (or the wasm
//! instance) goes on to answer the next request. There are three kinds of entry:
//!
//! * **Evaluation proper** checks and returns the error: every algebra node
//!   (`eval::eval_evaluated`), every expression node (`expr::eval_expr`), every `EXISTS`,
//!   every correlated evaluation (a `LATERAL` right side or a correlated `EXISTS`, per
//!   outer row) and every user-defined function call — and so do the fallible walks a
//!   plan passes through before its first operator: a governed evaluation's plan survey
//!   and the `1.2-basic` profile's admission.
//! * **Walks with no error channel** that run while evaluating — over a subtree as tall
//!   as the stack that parsed it held — run inside a `walk` scope. Each level asks
//!   `walk_is_low`; the first that finds the margin gone latches the refusal and returns a
//!   placeholder, and the scope discards whatever the walk built and returns the error.
//!   These are an `EXISTS` site's preparation (normalization, its source map, its
//!   structural analysis) and its scope-collision check, a correlated evaluation's
//!   per-row substitution copy, a property path's traversal, a `SERVICE` body's analysis,
//!   copy and serialization (the serializer's own levels ask too), an in-process
//!   `SERVICE`'s blank-node rewrite, a function body's copy and pre-binding rewrite, and
//!   template instantiation (`CONSTRUCT`, and an update's `DELETE`/`INSERT` and `DATA`
//!   templates). Their copies of algebra trees go through the `clone` submodule rather
//!   than the derived `Clone`, so they can refuse too.
//! * **Walks whose answer has a safe side** answer it when the stack is low, with no
//!   scope: the parallel-safety classification answers "unsafe" (the loop runs
//!   sequentially), `EXISTS` probe admissibility answers "not admissible" (the per-row
//!   definition runs), and the `GRAPH` emptiness proofs answer "not proven" (the graph
//!   is evaluated). Each is always correct, and the evaluation that follows refuses at
//!   its own next check.
//!
//! What remains unchecked is bounded otherwise:
//!
//! * The infallible walks that run once over the whole plan before its first operator
//!   (planning, blank-node scoping, the endpoint, parallel and admission analyses), and
//!   the derived copy and drop of the whole plan, start from the top of the stack the
//!   evaluation starts on. Every evaluation first measures the whole plan's height
//!   against that stack at the parser's per-level charge
//!   (`governor::soundness::validate_graph_pattern_depth`), which holds the costliest of
//!   those walks with room to spare, and refuses the plan, typed, where it does not fit.
//! * The drop of a per-row copy runs where the copy was made, by a guarded walk whose
//!   levels cost more than the drop's.
//! * The derived copy, comparison, hashing and matching of terms recurse once per
//!   triple-term level, wherever an evaluation stands. The margin holds those walks for
//!   terms up to [`MARGIN_TERM_LEVELS`] deep — every term a dataset holds (a frozen
//!   dataset at most 16, a PACK at most 128). A request whose own triple terms nest
//!   deeper — in a pattern, a `VALUES` block, a template, or a chain of `TRIPLE` calls —
//!   is evaluated under a [`purrdf_stack::reserve`] of [`TERM_LEVEL_BYTES`] a level past
//!   that ([`reserve_terms`]), so every check the evaluation passes leaves room below it
//!   for a walk over the deepest of them. A term built at run time — `TRIPLE` calls
//!   feeding each other through `BIND`, `LATERAL` or a function, or a value a user
//!   function, a `SERVICE` or property-function row or an aggregate hands back — is
//!   admitted as it enters the scratch interner, before anything walks it
//!   ([`admit_term`]): one deeper than the evaluation covers widens its reserve
//!   ([`purrdf_stack::widen`]) or is refused, typed. What bounds such a term is the
//!   stack, measured, and never a count. A fork-join worker keeps no reserve, so an
//!   evaluation holding deep terms never forks, and a worker that builds one refuses it
//!   and the work runs on the evaluating thread instead (see `crate::parallel`). A pattern position whose triple terms nest deeper than any the
//!   dataset holds matches nothing, and is answered so before it is walked (see
//!   `bgp::compile_term`).
//! * The one-level visitors do not recurse.
//!
//! The parser guards its own recursion against the same measurement, so the re-parse of
//! a body an in-process `SERVICE` forwards — which runs at whatever depth the `SERVICE`
//! sits — refuses rather than overflows too.
//!
//! # The measurement
//!
//! How much stack is left — the per-thread floor it is measured against, natively the
//! operating system's thread limit and on `wasm32` the shadow stack's low end or the floor
//! a host installed for a stack it switched onto — and the margin a check refuses at are
//! [`purrdf_stack`]'s, the one measurement the parser and the wasm host share with this
//! guard. See [`purrdf_stack::MARGIN_BYTES`] for the measured derivation of the margin.
//! The walk scopes are [`purrdf_stack`]'s too ([`purrdf_stack::walk`]): their state is
//! part of the per-computation [`purrdf_stack::Context`] a host swaps whenever it
//! suspends an evaluation and runs something else on the thread.

use crate::error::EvalError;

pub(crate) mod clone;

/// How many triple-term levels [`purrdf_stack::MARGIN_BYTES`] holds walks over, with no
/// reserve: 128, at [`TERM_LEVEL_BYTES`] a level half the margin natively and a quarter
/// of it on `wasm32` (see the margin's derivation).
pub(crate) const MARGIN_TERM_LEVELS: usize = 128;

/// What one triple-term level costs the walks over a term, in bytes: 512 natively, 256 on
/// `wasm32`.
///
/// Measured natively (x86_64, the workspace's opt-level-3 profile) on chains of triple
/// terms thousands of levels deep, the costliest walk a level takes is the derived
/// `Debug` of a pattern triple term, 496 bytes (its `Clone` is 258); a `wasm32` level
/// costs 0.37 to 0.45 of its native size on the shadow stack the guard measures. It is the
/// figure the parser charges a level of any tree it builds for the walks over it.
pub(crate) const TERM_LEVEL_BYTES: usize = if cfg!(target_arch = "wasm32") {
    256
} else {
    512
};

/// Reserve the stack walks over triple terms nested `levels` deep need past what the
/// margin already holds ([`MARGIN_TERM_LEVELS`]), for as long as the returned reserve
/// lives, then check that the stack left still clears the margin.
///
/// Nothing is reserved for a request whose triple terms nest [`MARGIN_TERM_LEVELS`] deep
/// or less — every request a count used to admit — so the common case pays nothing but
/// the scope the reserve opens, which [`admit_term`] widens for a deeper term the
/// evaluation builds.
///
/// # Errors
///
/// [`EvalError::StackExhausted`] naming triple terms when the thread has too little stack
/// left for the reserve and the margin both.
pub(crate) fn reserve_terms(levels: usize) -> Result<purrdf_stack::Reserve, EvalError> {
    let reserve = purrdf_stack::reserve(
        levels
            .saturating_sub(MARGIN_TERM_LEVELS)
            .saturating_mul(TERM_LEVEL_BYTES),
    );
    check("triple term")?;
    Ok(reserve)
}

/// The construct a triple term deeper than the margin holds is refused as on a thread
/// with no evaluation scope open — a fork-join worker (see `crate::parallel`), which
/// cannot keep stack for it past its own work. The fork-join primitives read this refusal
/// as "run the work sequentially" rather than reporting it.
pub(crate) const UNSCOPED_TRIPLE_TERM: &str = "triple term built off the evaluating thread";

/// Admit `value` into the running evaluation before anything walks it: every value the
/// evaluator builds or receives at run time — a `TRIPLE` call's result, a user function's,
/// a `SERVICE` or property-function row, an aggregate's — enters through the scratch
/// interner, which calls this first.
///
/// A value that is no triple term costs one discriminant test. A triple term's nesting is
/// counted without recursion; up to what the running evaluation already covers — the
/// [`MARGIN_TERM_LEVELS`] the margin holds, plus whatever it reserved for the request's
/// written terms or widened for terms it built before — nothing else happens. A deeper
/// one widens the evaluation's reserve by [`TERM_LEVEL_BYTES`] a level past that
/// ([`purrdf_stack::widen`]), so every check the evaluation passes from here on leaves
/// room for walks over it, and every frame up to where the evaluation started has that
/// room too: the term is bounded by the stack, measured, not by a count.
///
/// # Errors
///
/// [`EvalError::StackExhausted`] naming triple terms when the thread has too little stack
/// left for the widening and the margin both, or [`UNSCOPED_TRIPLE_TERM`] on a thread
/// with no evaluation scope open.
#[inline]
pub(crate) fn admit_term(value: &purrdf_core::TermValue) -> Result<(), EvalError> {
    if !matches!(value, purrdf_core::TermValue::Triple { .. }) {
        return Ok(());
    }
    admit_levels(term_nesting(value))
}

/// The triple-term half of [`admit_term`].
fn admit_levels(levels: usize) -> Result<(), EvalError> {
    let covered = MARGIN_TERM_LEVELS + purrdf_stack::reserved() / TERM_LEVEL_BYTES;
    if levels <= covered {
        return Ok(());
    }
    if purrdf_stack::widen((levels - covered).saturating_mul(TERM_LEVEL_BYTES)) {
        return Ok(());
    }
    Err(EvalError::StackExhausted {
        construct: if purrdf_stack::scoped() {
            "triple term"
        } else {
            UNSCOPED_TRIPLE_TERM
        },
    })
}

/// How many triple terms `value`'s longest chain holds, the outermost included, counted
/// without recursion (and without allocating unless a triple term's subject is itself
/// one).
pub(crate) fn term_nesting(value: &purrdf_core::TermValue) -> usize {
    use purrdf_core::TermValue;
    let mut deepest = 0;
    let mut pending: Vec<(&TermValue, usize)> = Vec::new();
    let mut next = Some((value, 0));
    while let Some((term, above)) = next.take().or_else(|| pending.pop()) {
        if let TermValue::Triple { s, p, o } = term {
            let depth = above + 1;
            deepest = deepest.max(depth);
            for part in [s, p] {
                if matches!(**part, TermValue::Triple { .. }) {
                    pending.push((part, depth));
                }
            }
            next = Some((o, depth));
        }
    }
    deepest
}

/// Drop `value` without recursing, so a triple term refused for being too deep for the
/// stack is released without the walk it was refused for.
pub(crate) fn drop_term(value: purrdf_core::TermValue) {
    use purrdf_core::TermValue;
    let mut pending = vec![value];
    while let Some(term) = pending.pop() {
        if let TermValue::Triple { s, p, o } = term {
            pending.extend([*s, *p, *o]);
        }
    }
}

/// How deeply the triple terms of `template` nest: the deepest of its quads' subjects
/// and objects, counted without recursion.
pub(crate) fn template_nesting(template: &[purrdf_sparql_algebra::QuadPattern]) -> usize {
    template
        .iter()
        .map(|quad| {
            quad.triple
                .subject
                .triple_term_nesting()
                .max(quad.triple.object.triple_term_nesting())
        })
        .max()
        .unwrap_or(0)
}

/// Refuse to go deeper when less than [`purrdf_stack::MARGIN_BYTES`] of stack are left.
///
/// `construct` names what was about to be evaluated, for the error.
///
/// # Errors
///
/// [`EvalError::StackExhausted`] naming `construct` when the stack left is below the
/// margin.
#[inline]
pub(crate) fn check(construct: &'static str) -> Result<(), EvalError> {
    if is_low() {
        return Err(EvalError::StackExhausted { construct });
    }
    Ok(())
}

/// Whether less than [`purrdf_stack::MARGIN_BYTES`] of stack are left.
///
/// [`check`] is this with the error attached; a caller that names its construct only
/// once it knows it is refusing (so the name is never computed on the hot path), or a
/// walk that cannot return an [`EvalError`] itself and latches the refusal for its
/// caller, calls this directly. The hot path is [`purrdf_stack::is_low`]'s: one
/// thread-local load, one subtraction and one comparison.
#[inline]
pub(crate) fn is_low() -> bool {
    purrdf_stack::is_low()
}

/// Run `body` — an infallible recursive walk over part of the plan (an analysis, a
/// substitution, a copy) — so that it can refuse when the stack runs low.
///
/// This is [`purrdf_stack::walk`], with its refusal typed as the evaluator's: a level of
/// the walk that finds less than [`purrdf_stack::MARGIN_BYTES`] left calls
/// [`walk_is_low`], which latches the refusal in this scope and tells the walk to stop
/// descending and return a placeholder; the scope then discards whatever `body` produced,
/// so a placeholder never escapes. Anything the walk wrote through a reference must be
/// discarded with it, which is why every caller hands its walk state it owns (a fresh
/// table, a fresh map) and drops it on the error. Once a level has refused, every check
/// in the running context refuses at once until the scope closes, so the walk unwinds
/// without computing over the placeholder it left.
///
/// The scope state lives in [`purrdf_stack`] beside the stack floor, as one
/// [`purrdf_stack::Context`] per running computation: a host that suspends an evaluation
/// inside a scope — a property path's traversal polls the stop signal, and the wasm
/// package's asynchronous lane suspends there — swaps the context out, so whatever runs
/// while it waits neither sees the open scope nor latches its own refusals in it.
///
/// # Errors
///
/// [`EvalError::StackExhausted`] naming the construct that refused, when any level of the
/// walk did.
pub(crate) fn walk<T>(body: impl FnOnce() -> T) -> Result<T, EvalError> {
    purrdf_stack::walk(body).map_err(|construct| EvalError::StackExhausted { construct })
}

/// Whether a level of an infallible walk must stop descending: inside a [`walk`] scope,
/// less than [`purrdf_stack::MARGIN_BYTES`] of stack are left (the refusal is latched
/// for the scope to report), or a level of the same walk already refused. Outside any
/// scope this is always `false`. `construct` names the walk, for the error.
///
/// [`purrdf_stack::walk_is_low`]; its hot path is [`is_low`]'s.
#[inline]
pub(crate) fn walk_is_low(construct: &'static str) -> bool {
    purrdf_stack::walk_is_low(construct)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `body` on a fresh thread with `bytes` of stack, so the thread-local floor is
    /// read for that thread alone.
    fn on_thread<T: Send + 'static>(bytes: usize, body: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(bytes)
            .spawn(body)
            .expect("spawn")
            .join()
            .expect("join")
    }

    /// A check passes on a roomy stack and refuses, typed, once a recursion has eaten
    /// into the margin — and the thread keeps running afterwards.
    #[test]
    fn check_refuses_inside_the_margin_and_not_before() {
        fn descend(level: usize) -> Result<usize, EvalError> {
            check("test recursion")?;
            // A frame big enough that the recursion reaches the margin quickly.
            let frame = core::hint::black_box([0u8; 4096]);
            descend(level + 1).map(|deepest| deepest.max(level + usize::from(frame[0])))
        }
        let refused = on_thread(1024 * 1024, || (descend(0), check("after")));
        assert_eq!(
            refused.0,
            Err(EvalError::StackExhausted {
                construct: "test recursion"
            })
        );
        assert_eq!(refused.1, Ok(()), "the unwound thread passes a check again");
    }
}
