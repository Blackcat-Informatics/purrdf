// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The guard every recursive evaluator entry passes through, refusing when the running
//! evaluation has too little stack left.
//!
//! The parser admits a request of any depth — it keeps what encloses the cursor on
//! heap-allocated stacks — but the evaluator recurses: one written level of
//! `FILTER NOT EXISTS` or `FILTER EXISTS` costs the evaluator about 5.3 KB of wasm32
//! shadow stack, one of `BIND` about 2.4 KB, and one of `LATERAL`, `OPTIONAL` or
//! `MINUS` about 1.2 to 1.4 KB — far more than the parser spent on it — while the flat
//! and shallow requests need a fraction of any stack. (Measured on the shipped npm
//! artifact as the growth of a job's deepest poll between 20 and 60 levels. The
//! figures hold only while `eval::eval_node` stays a thin dispatcher, since its frame
//! is paid twice per level of a negation.) Exhausting the stack is not an
//! error anywhere: natively the process aborts, and on `wasm32-unknown-unknown` the
//! shadow stack runs below its floor and traps with the instance's memory in an unknown
//! state.
//!
//! So every evaluator entry that can deepen the stack measures the stack actually left
//! and refuses with [`EvalError::StackExhausted`] — naming the construct — when it is
//! below [`purrdf_stack::MARGIN_BYTES`]. Nothing traps, and the thread (or the wasm
//! instance) goes on to answer the next request. Exactly these entries open a scope:
//!
//! * **Evaluation proper** checks and returns the error: every algebra node
//!   (`eval::eval_evaluated`, through [`is_low`]), every `EXISTS` (`expr::exists`),
//!   every correlated evaluation
//!   (a `LATERAL` right side or a correlated `EXISTS`, per outer row, in `binop`) and
//!   every user-defined function call (`user_fn`) — each through [`check`].
//! * **Correlated substitution** copies and rewrites bodies through an iterative
//!   native walk. Its work buffers are admitted before growth and a refused walk
//!   returns its original typed failure before publishing the body.
//! * **The height admission** ([`height`], through
//!   `governor::soundness::validate_graph_pattern_depth` and `engine`'s preparation)
//!   measures a whole plan, iteratively, against the stack the evaluation starts on at a
//!   per-level charge, before the recursive entries above walk it, and refuses the plan,
//!   typed, where it does not fit.
//!
//! Everything else the evaluator does over a request's structure needs no stack per
//! level, so it is neither checked nor bounded: the algebra's own copy, comparison,
//! hashing, formatting, serialization and drop; a term's copy, comparison, order,
//! hashing, formatting and drop; every walk over a triple term — its dataset lookup and
//! interning, its matching against a pattern, its instantiation from a template, its
//! conversion to and from the algebra and the results — however deeply it nests, wherever
//! the evaluation stands and on whichever thread; and every analysis that runs once over
//! the whole plan before its first operator (blank-node scoping, the endpoint scan, the
//! soundness and parallel-safety analyses, the plan survey, the normal-form and
//! pre-binding rewrites). Each of those walks a work list, and a triple term or a plan is
//! bounded by memory alone, identically on every host.
//!
//! # The measurement
//!
//! How much stack is left — the per-thread floor it is measured against, natively the
//! operating system's thread limit and on `wasm32` the shadow stack's low end or the floor
//! a host installed for a stack it switched onto — and the margin a check refuses at are
//! [`purrdf_stack`]'s, the one measurement the wasm host shares with this guard. See [`purrdf_stack::MARGIN_BYTES`] for the measured derivation of the margin.
//! The walk scopes are [`purrdf_stack`]'s too ([`purrdf_stack::walk`]): their state is
//! part of the per-computation [`purrdf_stack::Context`] a host swaps whenever it
//! suspends an evaluation and runs something else on the thread.

use crate::error::EvalError;

pub(crate) mod height;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        let refused =
            purrdf_stack::on_stack(1024 * 1024, || (descend(0), check("after"))).expect("spawn");
        assert_eq!(
            refused.0,
            Err(EvalError::StackExhausted {
                construct: "test recursion"
            })
        );
        assert_eq!(refused.1, Ok(()), "the unwound thread passes a check again");
    }
}
