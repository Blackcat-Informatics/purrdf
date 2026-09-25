// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The guard every recursive evaluator entry passes through, refusing when the running
//! evaluation has too little stack left.
//!
//! The parser bounds how deeply a request may nest
//! ([`purrdf_sparql_algebra::MAX_NESTING_DEPTH`]), but a level-count bound cannot bound
//! the stack *evaluating* that request needs: one written level of `FILTER NOT EXISTS`
//! costs the evaluator about 16.7 KB of wasm32 shadow stack, one of `LATERAL` about
//! 9.5 KB, and any other algebra level (`OPTIONAL`, `MINUS`, `BIND`, a sibling spine)
//! about 4.8 KB, so the deepest admitted requests need more than the synchronous wasm
//! lane's whole 1 MiB stack, while the flat and shallow ones a lower count would refuse
//! need a fraction of it. Exhausting the stack is not an error anywhere: natively the
//! process aborts, and on `wasm32-unknown-unknown` the shadow stack runs below its floor
//! and traps with the instance's memory in an unknown state.
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
//! * **Walks with no error channel** that run while evaluating — over a subtree whose
//!   height only the parser's budgets bound — run inside a `walk` scope. Each level asks
//!   `walk_is_low`; the first that finds the margin gone latches the refusal and returns a
//!   placeholder, and the scope discards whatever the walk built and returns the error.
//!   These are an `EXISTS` site's preparation (normalization, its source map, its
//!   structural analysis) and its scope-collision check, a correlated evaluation's
//!   per-row substitution copy, a property path's traversal, a `SERVICE` body's analysis,
//!   copy and serialization, an in-process `SERVICE`'s blank-node rewrite, a function
//!   body's copy and pre-binding rewrite, and template instantiation (`CONSTRUCT`, and
//!   an update's `DELETE`/`INSERT` and `DATA` templates). Their copies of algebra trees
//!   go through the `clone` submodule rather than the derived `Clone`, so they can refuse too.
//! * **Walks whose answer has a safe side** answer it when the stack is low, with no
//!   scope: the parallel-safety classification answers "unsafe" (the loop runs
//!   sequentially), `EXISTS` probe admissibility answers "not admissible" (the per-row
//!   definition runs), and the `GRAPH` emptiness proofs answer "not proven" (the graph
//!   is evaluated). Each is always correct, and the evaluation that follows refuses at
//!   its own next check.
//!
//! What remains unchecked is bounded by the parser's budgets (128 levels of graph
//! pattern, 512 of expression), at tens to a few hundred bytes a level: the derived copy
//! and drop of terms and of the per-row copies, the algebra serializer, the one-level
//! visitors — all of which the margin holds wherever they run — and the infallible walks
//! that run once over the whole plan before its first operator (planning, blank-node
//! scoping, the parallel and admission analyses). Those start from the top of the stack
//! the evaluation starts on; a thread too small even for them would need far more to
//! evaluate the first levels of the query, and is outside what this guard can make
//! safe. The parser guards its own recursion against the same measurement, so the
//! re-parse of a body an in-process `SERVICE` forwards — which runs at whatever depth
//! the `SERVICE` sits — refuses rather than overflows too.
//!
//! # The measurement
//!
//! How much stack is left — the per-thread floor it is measured against, natively the
//! operating system's thread limit and on `wasm32` the shadow stack's low end or the floor
//! a host installed for a stack it switched onto — and the margin a check refuses at are
//! [`purrdf_stack`]'s, the one measurement the parser and the wasm host share with this
//! guard. See [`purrdf_stack::MARGIN_BYTES`] for the measured derivation of the margin.

use core::cell::Cell;

use crate::error::EvalError;

pub(crate) mod clone;

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

/// Where the current thread stands with respect to [`walk`] scopes.
#[derive(Clone, Copy)]
enum WalkState {
    /// No scope is open: a walk that runs low has nobody to hand a refusal to, so it
    /// carries on exactly as it did before the guard existed.
    Unscoped,
    /// A scope is open and nothing has refused yet.
    Scoped,
    /// A walk inside the open scope refused at `construct`; the floor it replaced with
    /// [`purrdf_stack::EXHAUSTED`] is `floor`.
    Refused {
        /// What refused.
        construct: &'static str,
        /// The floor to put back when the scope closes.
        floor: usize,
    },
}

thread_local! {
    /// This thread's [`WalkState`].
    static WALK: Cell<WalkState> = const { Cell::new(WalkState::Unscoped) };
}

/// Run `body` — an infallible recursive walk over part of the plan (an analysis, a
/// substitution, a copy) — so that it can refuse when the stack runs low.
///
/// Such a walk has no error channel, so a level of it that finds less than
/// [`purrdf_stack::MARGIN_BYTES`] left calls [`walk_is_low`], which latches the refusal
/// on this thread and tells the walk to stop descending and return a placeholder. This
/// scope then discards whatever `body` produced and returns [`EvalError::StackExhausted`]
/// naming the construct that refused: a placeholder never escapes, because the only way
/// to reach `body`'s value is through the `Ok` this returns only when nothing refused.
/// Anything the walk wrote through a reference must be discarded with it, which is why
/// every caller hands its walk state it owns (a fresh table, a fresh map) and drops it on
/// the error.
///
/// Once a level has refused, the rest of the walk must not go on computing over the
/// placeholder it left — a later level could trip an internal consistency assertion on a
/// tree it half-built — so the refusal also replaces the thread's floor with
/// [`purrdf_stack::EXHAUSTED`]: every check on the thread, in every walk, in the
/// evaluator and in the parser alike, then refuses at once, and the walk unwinds to this
/// scope level by level without doing any more work.
/// The real floor is put back when the scope closes.
///
/// Scopes nest: the enclosing scope's state (and, after a refusal, the floor) is put back
/// when this one closes, even when `body` unwinds.
///
/// # Errors
///
/// [`EvalError::StackExhausted`] when any level of the walk refused.
pub(crate) fn walk<T>(body: impl FnOnce() -> T) -> Result<T, EvalError> {
    /// Closes the scope when the walk returns or unwinds: puts the real floor back if a
    /// level refused, then the enclosing scope's state.
    struct Close(WalkState);
    impl Drop for Close {
        fn drop(&mut self) {
            let inner = WALK.with(|state| state.replace(self.0));
            if let WalkState::Refused { floor, .. } = inner {
                purrdf_stack::replace_floor(floor);
            }
        }
    }
    let close = Close(WALK.with(|state| state.replace(WalkState::Scoped)));
    let value = body();
    let outcome = WALK.with(Cell::get);
    drop(close);
    match outcome {
        WalkState::Refused { construct, .. } => Err(EvalError::StackExhausted { construct }),
        WalkState::Unscoped | WalkState::Scoped => Ok(value),
    }
}

/// Whether a level of an infallible walk must stop descending: inside a [`walk`] scope,
/// less than [`purrdf_stack::MARGIN_BYTES`] of stack are left (the refusal is latched
/// for the scope to report), or a level of the same walk already refused. Outside any
/// scope this is always `false`. `construct` names the walk, for the error.
#[inline]
pub(crate) fn walk_is_low(construct: &'static str) -> bool {
    if !is_low() {
        return false;
    }
    walk_refuse(construct)
}

/// The cold half of [`walk_is_low`]: latch the refusal in the open scope, if any.
#[cold]
#[inline(never)]
fn walk_refuse(construct: &'static str) -> bool {
    WALK.with(|state| match state.get() {
        WalkState::Unscoped => false,
        WalkState::Scoped => {
            let floor = purrdf_stack::replace_floor(purrdf_stack::EXHAUSTED);
            state.set(WalkState::Refused { construct, floor });
            true
        }
        WalkState::Refused { .. } => true,
    })
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
