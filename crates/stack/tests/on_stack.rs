// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `on_stack` on every target, through one runner.
//!
//! The target is `harness = false` and hands its named cases to
//! `purrdf_testkit::harness`, so they run natively under `cargo test` and on
//! `wasm32-unknown-unknown` in Node:
//!
//! ```text
//! CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=scripts/wasm-test-runner.sh \
//!     cargo test -p purrdf-stack --target wasm32-unknown-unknown --test on_stack
//! ```
//!
//! On `wasm32` there is no second thread, so `on_stack` runs its computation inline
//! against the shadow stack's floor: a request larger than what is left above it is
//! refused, typed, and its neighbour inside the floor runs on a stack of the size asked
//! for. Natively every request gets a thread of its own and none is refused.

use std::process::ExitCode;

use purrdf_stack::{MARGIN_BYTES, StackError, is_low, on_stack, on_stack_scoped, remaining};
use purrdf_testkit::harness::{self, Failed, Trial};

/// A request inside the floor runs, returns its value, and measures a stack of the size
/// it asked for: half a margin is low, four margins are not.
fn an_in_floor_request_runs() -> Result<(), Failed> {
    const BYTES: usize = 4 * MARGIN_BYTES;
    let left = on_stack(BYTES, remaining).map_err(|error| error.to_string())?;
    // Never less than asked for, less the frames already live. Natively the C library
    // may hand a thread a cached stack several times the request, so only wasm32, whose
    // floor is installed exactly `BYTES` below the calling frame, bounds it from above.
    if left + MARGIN_BYTES < BYTES || (cfg!(target_arch = "wasm32") && left > BYTES) {
        return Err(format!("{left} bytes left on a {BYTES}-byte stack").into());
    }
    if on_stack(BYTES, is_low).map_err(|error| error.to_string())? {
        return Err("four margins of stack are reported low".into());
    }
    if !on_stack(MARGIN_BYTES / 2, is_low).map_err(|error| error.to_string())? {
        return Err("half a margin of stack is not reported low".into());
    }
    Ok(())
}

/// A request larger than the stack left above the floor is refused with the typed error,
/// and the computation never runs.
fn an_over_floor_request_is_refused() -> Result<(), Failed> {
    let requested = remaining().saturating_add(1024 * 1024);
    match on_stack(requested, || ()) {
        Err(StackError::ExceedsFloor {
            requested: reported,
            available,
        }) if reported == requested && available < requested => Ok(()),
        other => Err(format!("an over-floor request was not refused: {other:?}").into()),
    }
}

/// A scoped request inside the floor runs a computation that borrows the
/// caller's locals — reading one and writing another — and measures a stack of
/// the size it asked for.
fn a_scoped_in_floor_request_borrows_the_caller_s_locals() -> Result<(), Failed> {
    const BYTES: usize = 4 * MARGIN_BYTES;
    let members = [3_u64, 5, 7];
    let mut visits = 0_u32;
    let (sum, left) = on_stack_scoped(BYTES, || {
        visits += 1;
        (members.iter().sum::<u64>(), remaining())
    })
    .map_err(|error| error.to_string())?;
    if sum != 15 || visits != 1 {
        return Err(format!("the borrowed computation saw {sum} and ran {visits} times").into());
    }
    if left + MARGIN_BYTES < BYTES || (cfg!(target_arch = "wasm32") && left > BYTES) {
        return Err(format!("{left} bytes left on a {BYTES}-byte scoped stack").into());
    }
    Ok(())
}

/// A scoped request larger than the stack left above the floor is refused with
/// the typed error, and the borrowing computation never runs.
fn an_over_floor_scoped_request_is_refused() -> Result<(), Failed> {
    let requested = remaining().saturating_add(1024 * 1024);
    let mut ran = false;
    let outcome = on_stack_scoped(requested, || ran = true);
    match outcome {
        Err(StackError::ExceedsFloor {
            requested: reported,
            available,
        }) if reported == requested && available < requested && !ran => Ok(()),
        other => Err(format!("an over-floor scoped request was not refused: {other:?}").into()),
    }
}

/// A panic in a scoped computation resumes on the caller with its own payload.
fn a_scoped_panic_resumes_with_its_payload() -> Result<(), Failed> {
    let message = String::from("the borrowed computation failed");
    let caught = std::panic::catch_unwind(|| {
        on_stack_scoped(4 * MARGIN_BYTES, || -> () {
            std::panic::panic_any(message.clone())
        })
    })
    .expect_err("the panic resumes");
    match caught.downcast_ref::<String>() {
        Some(payload) if *payload == message => Ok(()),
        _ => Err("the resumed panic lost its payload".into()),
    }
}

fn main() -> ExitCode {
    let mut trials = vec![
        Trial::test("an_in_floor_request_runs", an_in_floor_request_runs),
        Trial::test(
            "a_scoped_in_floor_request_borrows_the_caller_s_locals",
            a_scoped_in_floor_request_borrows_the_caller_s_locals,
        ),
    ];
    if cfg!(target_arch = "wasm32") {
        trials.push(Trial::test(
            "an_over_floor_request_is_refused",
            an_over_floor_request_is_refused,
        ));
        trials.push(Trial::test(
            "an_over_floor_scoped_request_is_refused",
            an_over_floor_scoped_request_is_refused,
        ));
    } else {
        // wasm32-unknown-unknown aborts on a panic, so there is nothing to resume.
        trials.push(Trial::test(
            "a_scoped_panic_resumes_with_its_payload",
            a_scoped_panic_resumes_with_its_payload,
        ));
    }
    harness::main(trials)
}
