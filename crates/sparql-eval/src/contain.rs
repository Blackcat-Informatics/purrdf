// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared panic-containment helpers for host-injected extension seams.
//!
//! Every extension point the evaluator hands control to arbitrary host Rust —
//! property functions ([`crate::property_fn`]), native/SPARQL-bodied user
//! functions ([`crate::user_fn`]), and custom aggregates ([`crate::agg_fn`]) —
//! needs the same two guarantees before and around that call: a **declaration**
//! read (an infallible, argument-free query of a registered extension's static
//! metadata — arity, volatility, modes, algebraic class, a state bound, …) must
//! not be able to abort the caller if it panics, and neither must a **fallible
//! host call** (`open`, `step`, `finish`, …). Both wrappers live here once so
//! every seam reads the same contract instead of re-deriving it, and so a panic
//! from any of them renders in the same fixed, payload-free shape.
//!
//! # Why payload-free
//!
//! The panic payload is deliberately never interpolated into the returned
//! error: rendering it would make the message depend on which worker thread
//! panicked (a rayon-parallel host call can be reached from any of them), and a
//! query's result text must not depend on scheduling.
//!
//! # wasm32 note
//!
//! `catch_unwind` requires `panic = "unwind"`. A `wasm32-unknown-unknown`
//! target commonly builds with `panic = "abort"`, in which case a host
//! extension's panic aborts the process rather than being contained here —
//! this module cannot change that policy, only use the standard containment
//! primitive when unwinding is configured. Every extension seam's doc carries
//! this same note; it is not repeated per call site.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::error::EvalError;

/// Read a host **declaration** — an infallible, argument-free query of a
/// registered extension's static metadata (arity, volatility, modes, algebraic
/// class, a declared state bound, …) — with the panic contained.
///
/// `kind` names the extension seam ("property function", "custom aggregate", …)
/// and `what` names the declaration being read, so the message identifies both
/// without leaking the panic's own payload (see the module docs).
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise `Ok` of `read`'s result.
pub(crate) fn declaration_contained<T>(
    kind: &str,
    iri: &str,
    what: &str,
    read: impl FnOnce() -> T,
) -> Result<T, EvalError> {
    match catch_unwind(AssertUnwindSafe(read)) {
        Ok(value) => Ok(value),
        Err(_) => Err(EvalError::function(format!(
            "{kind} <{iri}> panicked while reporting its {what}"
        ))),
    }
}

/// Run a fallible host **call** (`open`, `step`, `finish`, …) with the panic
/// contained, producing a fixed, payload-free error naming what panicked.
///
/// The [`declaration_contained`] twin for a call that returns its own
/// [`Result`] rather than an infallible value.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise `call`'s own result,
/// propagated unchanged.
pub(crate) fn call_contained<T>(
    kind: &str,
    iri: &str,
    what: &str,
    call: impl FnOnce() -> Result<T, EvalError>,
) -> Result<T, EvalError> {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(result) => result,
        Err(_) => Err(EvalError::function(format!(
            "{kind} <{iri}> panicked while {what}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Suppress the default panic-hook stderr dump for an *expected*, caught panic
    /// (mirrors every other seam's test-only helper of the same shape).
    fn without_panic_output<R>(body: impl FnOnce() -> R) -> R {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let out = body();
        std::panic::set_hook(default_hook);
        out
    }

    #[test]
    fn declaration_contained_returns_the_value_on_success() {
        let value = declaration_contained("kind", "iri", "thing", || 42).expect("no panic");
        assert_eq!(value, 42);
    }

    #[test]
    fn declaration_contained_catches_a_panic_cleanly() {
        let error = without_panic_output(|| {
            declaration_contained("custom aggregate", "http://ex/agg", "arity", || {
                panic!("payload");
                #[allow(unreachable_code)]
                0
            })
            .expect_err("a panicking read must not escape")
        });
        let message = error.to_string();
        assert!(
            message.contains("custom aggregate <http://ex/agg> panicked while reporting its arity"),
            "got {message}"
        );
        assert!(!message.contains("payload"), "got {message}");
    }

    #[test]
    fn call_contained_returns_the_result_on_success() {
        let value: Result<i32, EvalError> =
            call_contained("kind", "iri", "doing", || Ok::<i32, EvalError>(7));
        assert_eq!(value.expect("ok"), 7);
    }

    #[test]
    fn call_contained_propagates_an_ordinary_error() {
        let error = call_contained("kind", "iri", "doing", || {
            Err::<i32, EvalError>(EvalError::function("boom"))
        })
        .expect_err("ordinary error propagates");
        assert!(error.to_string().contains("boom"), "got {error}");
    }

    #[test]
    fn call_contained_catches_a_panic_cleanly() {
        let error = without_panic_output(|| {
            call_contained("custom aggregate", "http://ex/agg", "finishing", || {
                panic!("payload");
                #[allow(unreachable_code)]
                Ok::<i32, EvalError>(0)
            })
            .expect_err("a panicking call must not escape")
        });
        let message = error.to_string();
        assert!(
            message.contains("custom aggregate <http://ex/agg> panicked while finishing"),
            "got {message}"
        );
        assert!(!message.contains("payload"), "got {message}");
    }
}
