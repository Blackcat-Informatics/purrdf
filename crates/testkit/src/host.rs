// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The process facilities the harness reads and writes, on each target.
//!
//! Natively they are the standard library's: the command line, the
//! environment, standard output and error, and a monotonic clock.
//!
//! On `wasm32-unknown-unknown` the standard library has none of them. Its
//! command line and environment are empty, its standard output and error
//! discard every byte, `Instant::now` panics, and a thread cannot be spawned.
//! There the test binary runs in Node under `scripts/wasm-test-runner.sh`, and
//! each facility is Node's instead: `process.argv` after the module path,
//! `process.env`, `console.log` and `console.error` one line at a time, and
//! `process.uptime`. Cases run serially, since there is no second thread to
//! run one on.
//!
//! The clock is `process.uptime` rather than `Date.now` or `performance.now`
//! because the runner replaces those, `Math.random` and
//! `crypto.getRandomValues` with functions that throw: code under test that
//! reaches a host clock or entropy source fails by name, and the harness's own
//! timing must not be what trips it.

pub(crate) use imp::{
    Stopwatch, args, forward_panic, parallelism, print_error, seal_host, set_exit_status, stdout,
    unseal_host, var,
};

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::io::{self, IsTerminal as _, Write};
    use std::time::Instant;

    /// The arguments after the program name.
    pub(crate) fn args() -> Vec<String> {
        std::env::args().skip(1).collect()
    }

    /// An environment variable, `None` when unset; a value that is not UTF-8
    /// is read lossily.
    pub(crate) fn var(name: &str) -> Option<String> {
        std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
    }

    /// Standard output and whether it is a terminal. It is not held locked:
    /// each write takes the lock, so a case printing from a worker thread
    /// while the reporter writes on the main thread waits for one write, not
    /// for the whole run.
    pub(crate) fn stdout() -> (Box<dyn Write>, bool) {
        let stdout = io::stdout();
        let terminal = stdout.is_terminal();
        (Box::new(stdout), terminal)
    }

    /// Write `text` to standard error as it is.
    pub(crate) fn print_error(text: &str) {
        eprint!("{text}");
    }

    /// Natively the default panic hook already writes the message to standard
    /// error, so there is nothing to forward.
    pub(crate) const fn forward_panic(_text: &str) -> bool {
        false
    }

    /// Natively the exit status is `main`'s return value, so there is
    /// nothing to hand over.
    pub(crate) const fn set_exit_status(_status: u8) {}

    /// Natively the host's clock and entropy cannot be withdrawn from code
    /// under test, so sealing is the identity; the wasm32 run is where the
    /// seal is enforced.
    pub(crate) const fn seal_host() {}

    /// See [`seal_host`].
    pub(crate) const fn unseal_host() {}

    /// The default worker count: the available parallelism.
    pub(crate) fn parallelism(requested: Option<usize>) -> usize {
        requested.unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
    }

    /// A monotonic start time.
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct Stopwatch(Instant);

    impl Stopwatch {
        /// Start now.
        pub(crate) fn start() -> Self {
            Self(Instant::now())
        }

        /// Seconds since the start.
        pub(crate) fn seconds(self) -> f64 {
            self.0.elapsed().as_secs_f64()
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::RefCell;
    use std::io::{self, Write};

    use js_sys::{Array, Function, Object, Reflect};
    use wasm_bindgen::prelude::wasm_bindgen;
    use wasm_bindgen::{JsCast as _, JsValue};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console, js_name = log)]
        fn console_log(line: &str);

        #[wasm_bindgen(js_namespace = console, js_name = error)]
        fn console_error(line: &str);
    }

    /// Node's `process`, which the runner guarantees exists.
    fn process() -> Object {
        Reflect::get(&js_sys::global(), &JsValue::from_str("process"))
            .ok()
            .and_then(|value| value.dyn_into::<Object>().ok())
            .expect(
                "the wasm32 test binary runs in Node under scripts/wasm-test-runner.sh, \
                 which provides `process`",
            )
    }

    /// `process.argv` after the Node executable and the module path, which is
    /// what the runner forwards of cargo's arguments.
    pub(crate) fn args() -> Vec<String> {
        let argv = Reflect::get(&process(), &JsValue::from_str("argv"))
            .ok()
            .and_then(|value| value.dyn_into::<Array>().ok())
            .expect("Node's process.argv is an array");
        argv.iter()
            .skip(2)
            .map(|value| {
                value
                    .as_string()
                    .expect("every process.argv entry is a string")
            })
            .collect()
    }

    /// `process.env[name]`, `None` when unset.
    pub(crate) fn var(name: &str) -> Option<String> {
        let env = Reflect::get(&process(), &JsValue::from_str("env")).ok()?;
        Reflect::get(&env, &JsValue::from_str(name))
            .ok()
            .and_then(|value| value.as_string())
    }

    thread_local! {
        /// Bytes written to standard output that do not yet end a line.
        /// `console.log` always ends a line, so a line is logged only once its
        /// newline arrives: libtest's `test <name> ... ` is completed by the
        /// `ok` written after the case, and the two must reach one line.
        static PENDING: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }

    /// Standard output as `console.log`, one line per call.
    struct Console;

    impl Write for Console {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            PENDING.with(|pending| {
                let mut pending = pending.borrow_mut();
                pending.extend_from_slice(bytes);
                while let Some(end) = pending.iter().position(|&byte| byte == b'\n') {
                    let line: Vec<u8> = pending.drain(..=end).collect();
                    console_log(&String::from_utf8_lossy(&line[..end]));
                }
            });
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Drop for Console {
        /// A trailing partial line is logged rather than lost.
        fn drop(&mut self) {
            PENDING.with(|pending| {
                let mut pending = pending.borrow_mut();
                if !pending.is_empty() {
                    console_log(&String::from_utf8_lossy(&pending));
                    pending.clear();
                }
            });
        }
    }

    /// Standard output as `console.log`; never a terminal, since Node's
    /// console output is what the runner hands back to cargo.
    pub(crate) fn stdout() -> (Box<dyn Write>, bool) {
        (Box::new(Console), false)
    }

    /// Write `text` to `console.error`, one call per line.
    pub(crate) fn print_error(text: &str) {
        for line in text.strip_suffix('\n').unwrap_or(text).split('\n') {
            console_error(line);
        }
    }

    /// The standard library's panic hook writes to a standard error that
    /// discards every byte here, so a panic's message is forwarded to
    /// `console.error` instead, or it would never be seen.
    pub(crate) fn forward_panic(text: &str) -> bool {
        print_error(text);
        true
    }

    /// `process.exitCode`. The generated bindings call `main` from the
    /// module's start function and drop what it returns, so the status is
    /// handed to Node here instead. The runner refuses a module that ends
    /// without setting it: that module never ran this harness.
    pub(crate) fn set_exit_status(status: u8) {
        Reflect::set(
            &process(),
            &JsValue::from_str("exitCode"),
            &JsValue::from(status),
        )
        .expect("Node's process.exitCode is writable");
    }

    /// Call `purrdfWasmTestRunner.<method>()`, the seal the runner provides.
    fn runner_call(method: &str) {
        let runner = Reflect::get(
            &js_sys::global(),
            &JsValue::from_str("purrdfWasmTestRunner"),
        )
        .ok()
        .filter(JsValue::is_object)
        .expect(
            "globalThis.purrdfWasmTestRunner is absent: a wasm32 test binary runs under \
                 scripts/wasm-test-runner.sh, which provides the host seal",
        );
        let function = Reflect::get(&runner, &JsValue::from_str(method))
            .ok()
            .and_then(|value| value.dyn_into::<Function>().ok())
            .unwrap_or_else(|| panic!("purrdfWasmTestRunner.{method} is not a function"));
        if let Err(error) = function.call0(&runner) {
            panic!("purrdfWasmTestRunner.{method}() refused: {error:?}");
        }
    }

    /// Replace every host clock and entropy source with one that throws.
    pub(crate) fn seal_host() {
        runner_call("sealHost");
    }

    /// Restore what [`seal_host`] replaced.
    pub(crate) fn unseal_host() {
        runner_call("unsealHost");
    }

    /// One: there is no second thread to run a case on.
    pub(crate) const fn parallelism(_requested: Option<usize>) -> usize {
        1
    }

    /// `process.uptime()`, in seconds.
    fn uptime() -> f64 {
        let process = process();
        Reflect::get(&process, &JsValue::from_str("uptime"))
            .ok()
            .and_then(|value| value.dyn_into::<Function>().ok())
            .and_then(|uptime| uptime.call0(&process).ok())
            .and_then(|seconds| seconds.as_f64())
            .expect("Node's process.uptime() returns a number of seconds")
    }

    /// A start time from `process.uptime`, since `Instant::now` panics here.
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct Stopwatch(f64);

    impl Stopwatch {
        /// Start now.
        pub(crate) fn start() -> Self {
            Self(uptime())
        }

        /// Seconds since the start.
        pub(crate) fn seconds(self) -> f64 {
            uptime() - self.0
        }
    }
}
