// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's shared test support, written once instead of per crate.
//!
//! * [`golden`] — checked-in golden files compared byte for byte, CRLF and
//!   trailing whitespace included, rewritten from the produced output when
//!   `PURRDF_REGENERATE_GOLDEN=1`. Call it through [`assert_golden!`], which
//!   resolves `tests/golden/` against the *calling* crate.
//! * [`TempDir`] and [`NamedTempFile`] — scratch space under the cargo target
//!   directory, including when that directory is placed under `/tmp`. Integration tests and
//!   benches create them through [`temp_dir!`] and [`temp_file!`], which read
//!   `CARGO_TARGET_TMPDIR` in the calling target; a crate's `src/` unit tests,
//!   where cargo does not set that variable, use [`TempDir::for_unit_test`] and
//!   [`NamedTempFile::for_unit_test`]. None of these exist on
//!   `wasm32-unknown-unknown`, which has no file system.
//! * [`vectors`] — frozen differential vectors: a line format whose header
//!   carries a SHA-256 of its own body, a recorder that writes it, and a replay
//!   that names the first record an implementation disagrees with.
//! * [`harness`] — a libtest-compatible runner for `harness = false` targets:
//!   the same console lines, the same tally line, the same flags, parallel
//!   execution and per-case panic isolation, with [`harness_main!`] for a
//!   target written as plain functions. The same targets run on
//!   `wasm32-unknown-unknown` in Node under `scripts/wasm-test-runner.sh`.
//! * [`prop`] — property-based testing on a recorded choice sequence:
//!   strategies, [`prop_test!`], shrinking by replaying edited choice
//!   sequences, regex-driven string generators, stateful model-based testing,
//!   and a deterministic seed per property.
//! * [`jsonschema_metaschemas`] — the published JSON Schema meta-schemas as
//!   test data, for every test and example that registers them with
//!   `purrdf-jsonschema` (which carries none).
//! * [`bench`] — the micro-benchmark harness for `harness = false` bench
//!   targets: warm-up, flat sampling, the median with its MAD and a seeded
//!   bootstrap interval, throughput, saved baselines compared with a
//!   bootstrapped change, and a fixed-schema JSON estimates file per
//!   benchmark, written with [`bench_group!`] and [`bench_main!`]. The same
//!   targets run on `wasm32-unknown-unknown` under the test runner.
//! * [`rng`] — the one deterministic SplitMix64 / xoshiro256** stream every
//!   crate's fixed-seed tests draw from, including [`prop`] itself, and the
//!   xorshift64 and 64-bit LCG recurrences pinned test fixtures are built on.
//! * [`paths`] — the workspace root, resolved from the `[workspace]` manifest
//!   rather than by counting `..`, and the Rust sources under a directory.
//!
//! Its one first-party dependency is `purrdf-hash`, the zero-dependency root,
//! whose own tests and benches do not use this crate; `layers.toml` allows no
//! other. Every crate in the workspace may take this one as a dev-dependency,
//! so any further first-party edge from here would close a cycle through that
//! crate's tests. It is never published and appears only in
//! `[dev-dependencies]` (and in `purrdf-hash-conformance`, the unpublished
//! home of the root's suites).

// The wasm32 host's imports are `#[wasm_bindgen]` declarations, whose
// expansion is `unsafe`; `host` is the one module allowed it, and only there.
#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]
#![cfg_attr(target_arch = "wasm32", deny(unsafe_code))]

pub mod bench;
pub mod golden;
pub mod harness;
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code, reason = "the expansion of `#[wasm_bindgen]` imports")]
mod host;
pub mod jsonschema_metaschemas;
pub mod paths;
pub mod prop;
pub mod rng;
#[cfg(not(target_arch = "wasm32"))]
mod temp;
pub mod vectors;

#[cfg(not(target_arch = "wasm32"))]
pub use temp::{NamedTempFile, TempDir};

/// Write a `harness = false` target's `main` over plain functions.
///
/// `harness_main!(case_a, case_b)` expands to a `fn main() -> ExitCode` that
/// hands one [`harness::Trial`] per function to [`harness::main`], in the
/// order written. Each case is named by its function's identifier. It passes
/// when the function returns, and fails when it panics. Attributes written
/// before an identifier apply to that case's registration, so
/// `#[cfg(not(target_arch = "wasm32"))] native_only_case` registers the case
/// only where the function exists.
///
/// ```no_run
/// fn arithmetic() {
///     assert_eq!(1 + 1, 2);
/// }
///
/// #[cfg(not(target_arch = "wasm32"))]
/// fn native_only() {}
///
/// purrdf_testkit::harness_main!(
///     arithmetic,
///     #[cfg(not(target_arch = "wasm32"))]
///     native_only,
/// );
/// ```
#[macro_export]
macro_rules! harness_main {
    ($( $(#[$meta:meta])* $case:ident ),+ $(,)?) => {
        fn main() -> ::std::process::ExitCode {
            // `mut` is unused when every registration is configured away.
            #[allow(unused_mut, clippy::vec_init_then_push)]
            let mut trials: ::std::vec::Vec<$crate::harness::Trial> = ::std::vec::Vec::new();
            $(
                $(#[$meta])*
                trials.push($crate::harness::Trial::test(
                    ::core::stringify!($case),
                    || {
                        $case();
                        ::core::result::Result::Ok(())
                    },
                ));
            )+
            $crate::harness::main(trials)
        }
    };
}

/// A fresh [`TempDir`] under the calling test's `CARGO_TARGET_TMPDIR`.
///
/// `temp_dir!()` uses the default name prefix; `temp_dir!("prefix-")` names
/// the directory `prefix-<pid>-<counter>-<nanos>`. The variable is read with
/// `env!` where the macro is *expanded*, so it resolves in the integration test
/// or bench that calls it — cargo sets it for exactly those targets, and using
/// the macro anywhere else is a compile error rather than a silent fallback to
/// a different directory. Evaluates to
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
