// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's shared test support, written once instead of per crate.
//!
//! * [`golden`] — checked-in golden files compared byte for byte, CRLF and
//!   trailing whitespace included, rewritten from the produced output when
//!   `PURRDF_REGENERATE_GOLDEN=1`. Call it through [`assert_golden!`], which
//!   resolves `tests/golden/` against the *calling* crate, or
//!   [`assert_golden_bytes!`] when the output is bytes rather than text.
//! * [`TempDir`] and [`NamedTempFile`] — scratch space under the cargo target
//!   directory, including when that directory is placed under `/tmp`. Integration tests and
//!   benches create them through [`temp_dir!`] and [`temp_file!`], which read
//!   `CARGO_TARGET_TMPDIR` in the calling target; a crate's `src/` unit tests,
//!   where cargo does not set that variable, use [`TempDir::for_unit_test`] and
//!   [`NamedTempFile::for_unit_test`]. None of these exist on
//!   `wasm32-unknown-unknown`, which has no file system.
//! * [`paths`] — the workspace root above the calling crate, through
//!   [`workspace_root!`], and a sorted sweep of every file of one extension
//!   under a directory, for the hygiene tests that read the whole tree. Also
//!   absent on `wasm32-unknown-unknown`.
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
//! * [`rng`] — the one deterministic SplitMix64 / xoshiro256** stream every
//!   crate's fixed-seed tests draw from, including [`prop`] itself, and the
//!   Fisher–Yates, xorshift64 and MMIX-LCG forms fixtures were frozen on.
//! * [`env_flag`] — the one rule by which a switch is read from the
//!   environment: set to exactly `1`.
//!
//! The crate depends on no `purrdf-*` crate, and must not: every crate in the
//! workspace may take it as a dev-dependency, so a first-party edge from here
//! would close a cycle through that crate's tests. It is never published and
//! appears only in `[dev-dependencies]`.

// The wasm32 host's imports are `#[wasm_bindgen]` declarations, whose
// expansion is `unsafe`; `host` is the one module allowed it, and only there.
#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]
#![cfg_attr(target_arch = "wasm32", deny(unsafe_code))]

pub mod golden;
pub mod harness;
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code, reason = "the expansion of `#[wasm_bindgen]` imports")]
mod host;
pub mod jsonschema_metaschemas;
#[cfg(not(target_arch = "wasm32"))]
pub mod paths;
pub mod prop;
pub mod rng;
#[cfg(not(target_arch = "wasm32"))]
mod temp;
pub mod vectors;

#[cfg(not(target_arch = "wasm32"))]
pub use temp::{NamedTempFile, TempDir};

/// Whether the environment variable `name` is set to exactly `1`.
///
/// The one truthiness rule this crate reads a switch by —
/// `PURRDF_REGENERATE_GOLDEN`, through [`golden::Mode::from_env`] — stated
/// once so that every `PURRDF_UPDATE_*` and `PURRDF_RECORD_*` switch a crate's
/// tests read can adopt it, and a reader never has to ask whether `0`, `true`,
/// `yes` or an empty value switches anything: none of them do. `1` switches;
/// `1` with whitespace around it, `01`, any other value, a value that is not
/// Unicode, and absence do not.
pub fn env_flag(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| value == "1")
}

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

/// Compare the bytes `actual` against the calling crate's
/// `tests/golden/<name>`, or rewrite that file when
/// `PURRDF_REGENERATE_GOLDEN=1`: [`assert_golden!`] for output that is bytes
/// rather than text.
///
/// `CARGO_MANIFEST_DIR` is read where the macro is expanded, so the golden
/// directory is the calling crate's. See [`golden`] for the comparison rules.
#[macro_export]
macro_rules! assert_golden_bytes {
    ($name:expr, $actual:expr $(,)?) => {
        $crate::golden::assert_golden_bytes_in(
            ::std::path::Path::new(::core::env!("CARGO_MANIFEST_DIR")),
            $name,
            $actual,
        )
    };
}

/// The workspace root above the calling crate: the nearest ancestor of its
/// manifest directory whose `Cargo.toml` declares a workspace.
///
/// `CARGO_MANIFEST_DIR` is read where the macro is expanded, so the walk
/// starts at the calling crate, and the result is that directory's ancestor
/// as written — never canonicalized. Evaluates to `std::path::PathBuf`; see
/// [`paths::workspace_root_from`] for the rules.
#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! workspace_root {
    () => {
        $crate::paths::workspace_root_from(::std::path::Path::new(::core::env!(
            "CARGO_MANIFEST_DIR"
        )))
    };
}
