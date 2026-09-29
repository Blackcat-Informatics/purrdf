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

use purrdf_stack::{MARGIN_BYTES, StackError, is_low, on_stack, remaining};
use purrdf_testkit::harness::{self, Failed, Trial};

/// A request inside the floor runs, returns its value, and measures a stack of the size
/// it asked for: half a margin is low, four margins are not.
fn an_in_floor_request_runs() -> Result<(), Failed> {
    const BYTES: usize = 4 * MARGIN_BYTES;
    let left = on_stack(BYTES, remaining).map_err(|error| error.to_string())?;
    if left > BYTES + MARGIN_BYTES {
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

fn main() -> ExitCode {
    let mut trials = vec![Trial::test(
        "an_in_floor_request_runs",
        an_in_floor_request_runs,
    )];
    if cfg!(target_arch = "wasm32") {
        trials.push(Trial::test(
            "an_over_floor_request_is_refused",
            an_over_floor_request_is_refused,
        ));
    }
    harness::main(trials)
}
