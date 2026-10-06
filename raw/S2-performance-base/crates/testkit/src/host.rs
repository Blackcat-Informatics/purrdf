// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The harness's host on `wasm32-unknown-unknown`.
//!
//! The standard library has no command line, environment, console or clock on
//! that target, so a test binary built for it runs in Node under
//! `scripts/wasm-test-runner.sh`, whose driver installs the global object
//! `__purrdf_testkit_host` before the module starts. Every function here is
//! one of that object's methods. The object is looked up when a function is
//! called, so a module run anywhere else fails at its first host call rather
//! than running with no console.

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    /// The number of command-line arguments after the module path.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = argCount)]
    pub(crate) fn arg_count() -> u32;

    /// The command-line argument at `index` (after the module path).
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = arg)]
    pub(crate) fn arg(index: u32) -> String;

    /// The environment variable `name`, if it is set.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = envVar)]
    pub(crate) fn env_var(name: &str) -> Option<String>;

    /// Write `text` to standard output, byte for byte (no newline is added).
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = writeStdout)]
    pub(crate) fn write_stdout(text: &str);

    /// Write `text` to standard error, byte for byte.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = writeStderr)]
    pub(crate) fn write_stderr(text: &str);

    /// Whether standard output is a terminal.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = stdoutIsTerminal)]
    pub(crate) fn stdout_is_terminal() -> bool;

    /// Milliseconds on a monotonic clock that the seal never withdraws: the
    /// harness's own timing, not a source a computation under test may read.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = nowMillis)]
    pub(crate) fn now_millis() -> f64;

    /// Withdraw every host clock and entropy source until the matching
    /// [`unseal`]. Seals nest.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = seal)]
    pub(crate) fn seal();

    /// Undo one [`seal`].
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = unseal)]
    pub(crate) fn unseal();

    /// Record the run's exit status. The runner refuses a module that never
    /// calls this.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = exit)]
    pub(crate) fn exit(status: u8);

    /// Arm the report printed if case `name` ends the module: `report` is the
    /// rest of the run's console output with the failure message and the
    /// elapsed seconds left as [`MESSAGE_SLOT`] and [`ELAPSED_SLOT`], and
    /// `started_millis` is the run's start on [`now_millis`]'s clock.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = arm)]
    pub(crate) fn arm(name: &str, report: &str, started_millis: f64);

    /// The armed case finished; nothing is printed for it here.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = disarm)]
    pub(crate) fn disarm();

    /// The armed case failed with `message`: print the armed report with the
    /// message and the elapsed time filled in. The module traps next.
    #[wasm_bindgen(js_namespace = __purrdf_testkit_host, js_name = fail)]
    pub(crate) fn fail(message: &str);
}

/// Where the failure message goes in an armed report.
pub(crate) const MESSAGE_SLOT: &str = "\u{0}purrdf-testkit-message\u{0}";

/// Where the elapsed seconds, to two decimals, go in an armed report.
pub(crate) const ELAPSED_SLOT: &str = "\u{0}purrdf-testkit-elapsed\u{0}";
