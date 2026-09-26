// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's shared test support, written once instead of per crate.
//!
//! * [`golden`] — checked-in golden files compared byte for byte, CRLF and
//!   trailing whitespace included, rewritten from the produced output when
//!   `PURRDF_REGENERATE_GOLDEN=1`. Call it through [`assert_golden!`], which
//!   resolves `tests/golden/` against the *calling* crate.
//! * [`TempDir`] and [`NamedTempFile`] — scratch space under the cargo target
//!   directory, never the system temporary directory, on every target with a
//!   file system (so not on `wasm32-unknown-unknown`). Integration tests and
//!   benches create them through [`temp_dir!`] and [`temp_file!`], which read
//!   `CARGO_TARGET_TMPDIR` in the calling target; a crate's `src/` unit tests,
//!   where cargo does not set that variable, use [`TempDir::for_unit_test`]
//!   and [`NamedTempFile::for_unit_test`].
//! * [`vectors`] — frozen differential vectors: a line format whose header
//!   carries a SHA-256 of its own body, a recorder that writes it, and a replay
//!   that names the first record an implementation disagrees with.
//! * [`harness`] — a libtest-compatible runner for `harness = false` targets:
//!   the same console lines, the same tally line, the same flags, parallel
//!   execution and per-case panic isolation, natively and on
//!   `wasm32-unknown-unknown` in Node under `scripts/wasm-test-runner.sh`.
//!   [`harness_main!`] writes a target's `main` from plain case functions.
//! * [`prop`] — property-based testing on a recorded choice sequence:
//!   strategies, [`prop_test!`], shrinking by replaying edited choice
//!   sequences, regex-driven string generators, stateful model-based testing,
//!   and a deterministic seed per property.
//! * [`rng`] — the one deterministic SplitMix64 / xoshiro256** stream every
//!   crate's fixed-seed tests draw from, including [`prop`] itself.
//!
//! The crate depends on no `purrdf-*` crate, and must not: every crate in the
//! workspace may take it as a dev-dependency, so a first-party edge from here
//! would close a cycle through that crate's tests. It is never published and
//! appears only in `[dev-dependencies]`.

#![forbid(unsafe_code)]

pub mod golden;
pub mod harness;
mod host;
pub mod prop;
pub mod rng;
// Scratch space on a file system. wasm32-unknown-unknown has none — its
// `std::fs::File` is an uninhabited type — so the module exists only where a
// test can create a path.
#[cfg(not(target_arch = "wasm32"))]
mod temp;
pub mod vectors;

#[cfg(not(target_arch = "wasm32"))]
pub use temp::{NamedTempFile, TempDir};

/// A fresh [`TempDir`] under the calling test's `CARGO_TARGET_TMPDIR`.
///
/// `temp_dir!()` uses the default name prefix; `temp_dir!("prefix-")` names
/// the directory `prefix-<pid>-<counter>-<nanos>`. The variable is read with
/// `env!` where the macro is *expanded*, so it resolves in the integration test
/// or bench that calls it — cargo sets it for exactly those targets, and using
/// the macro anywhere else is a compile error rather than a silent fallback to
/// the system temporary directory. Evaluates to
/// `std::io::Result<TempDir>`.
#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! temp_dir {
    () => {
        $crate::TempDir::new_in(::core::env!("CARGO_TARGET_TMPDIR"))
    };
    ($prefix:expr $(,)?) => {
        $crate::TempDir::with_prefix_in($prefix, ::core::env!("CARGO_TARGET_TMPDIR"))
    };
}

/// A fresh [`NamedTempFile`] under the calling test's `CARGO_TARGET_TMPDIR`.
///
/// The file counterpart of [`temp_dir!`], with the same prefix form and the
/// same call-site resolution. Evaluates to `std::io::Result<NamedTempFile>`.
#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! temp_file {
    () => {
        $crate::NamedTempFile::new_in(::core::env!("CARGO_TARGET_TMPDIR"))
    };
    ($prefix:expr $(,)?) => {
        $crate::NamedTempFile::with_prefix_in($prefix, ::core::env!("CARGO_TARGET_TMPDIR"))
    };
}

/// Compare `actual` against the calling crate's `tests/golden/<name>`, or
/// rewrite that file when `PURRDF_REGENERATE_GOLDEN=1`.
///
/// `CARGO_MANIFEST_DIR` is read where the macro is expanded, so the golden
/// directory is the calling crate's. See [`golden`] for the comparison rules.
#[macro_export]
macro_rules! assert_golden {
    ($name:expr, $actual:expr $(,)?) => {
        $crate::golden::assert_golden_in(
            ::std::path::Path::new(::core::env!("CARGO_MANIFEST_DIR")),
            $name,
            $actual,
        )
    };
}

/// A `harness = false` target's `main`, running the named functions as its
/// cases on [`harness::main`].
///
/// Each function takes no argument and fails by panicking, as a `#[test]`
/// function does, and its case is named by the function's identifier — the
/// name libtest would give the same function — so a target moved off libtest
/// keeps its case names. Cases run in the order written. A case may carry
/// attributes, such as a `#[cfg(...)]` that registers it on some targets only.
///
/// ```
/// fn arithmetic_holds() {
///     assert_eq!(1 + 1, 2);
/// }
///
/// purrdf_testkit::harness_main!(arithmetic_holds);
/// ```
#[macro_export]
macro_rules! harness_main {
    ($($(#[$attribute:meta])* $case:ident),+ $(,)?) => {
        fn main() -> ::std::process::ExitCode {
            $crate::harness::main([
                $(
                    $(#[$attribute])*
                    $crate::harness::Trial::test(::core::stringify!($case), || {
                        $case();
                        ::core::result::Result::Ok(())
                    }),
                )+
            ])
        }
    };
}
