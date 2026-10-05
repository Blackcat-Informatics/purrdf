// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A libtest-compatible runner for `harness = false` test targets.
//!
//! A target that generates its cases at run time — one per manifest entry, one
//! per corpus file — declares `harness = false` and hands its cases to
//! [`main`]:
//!
//! ```no_run
//! use purrdf_testkit::harness::{self, Failed, Trial};
//!
//! fn main() -> std::process::ExitCode {
//!     let cases = vec![
//!         Trial::test("arithmetic", || {
//!             if 1 + 1 == 2 { Ok(()) } else { Err(Failed::from("arithmetic broke")) }
//!         }),
//!         Trial::test("slow_case", || Ok(())).with_ignored_flag(true),
//!     ];
//!     harness::main(cases)
//! }
//! ```
//!
//! # What is the same as libtest
//!
//! * **Console output**, line for line: `running N tests`, one
//!   `test <name> ... ok|FAILED|ignored` line per case (or libtest's terse
//!   `.`/`F`/`i` characters, wrapped at 88 with a `done/total` count), the
//!   `failures:` section with a `---- <name> stdout ----` block per failure,
//!   the sorted failure list, and the tally line
//!   `test result: ok|FAILED. P passed; F failed; I ignored; 0 measured; X filtered out; finished in S.SSs`
//!   that cargo, CI log scrapers and `scripts/conformance-matrix.py` read.
//! * **Flags**: name filters (substring, any of several), `--exact`,
//!   `--skip` (repeatable), `--list`, `--format pretty|terse`, `-q`/`--quiet`,
//!   `--ignored`, `--include-ignored`, `--nocapture` (and its newer spelling
//!   `--no-capture`), `--show-output`, `--color auto|always|never`,
//!   `--test-threads N`, `-Z unstable-options`, and `-h`/`--help`; plus the
//!   `RUST_TEST_THREADS` and `RUST_TEST_NOCAPTURE` environment variables.
//!   Every other flag is refused with libtest's message and exit status 101 —
//!   a flag this runner does not implement is never silently ignored.
//! * **Isolation**: each case runs under `catch_unwind`, so a panicking case
//!   is `FAILED` with its panic message and every other case still runs.
//! * **Parallelism**: cases run on `--test-threads` worker threads (default:
//!   the available parallelism); with one thread each case's
//!   `test <name> ... ` is printed before it starts, as libtest does.
//! * **Exit status**: 0 when nothing failed, 101 otherwise.
//!
//! # What differs, and why
//!
//! * **Order**: cases run and list in the order given, not sorted by name.
//!   A generated suite's order is part of its contract (a directory walk's
//!   order, for instance), and the caller can sort if it wants libtest's.
//! * **Captured output**: libtest captures a case's standard output through an
//!   interface that is not stable Rust. Here a case's own prints go straight
//!   to the process's output; only its panic message is captured, and shown
//!   in the `failures:` section (or immediately, under `--nocapture`).
//! * **Names are unique**: two cases with one name are refused before
//!   anything runs, since a filter or a failure report naming one of them
//!   would be ambiguous.
//! * **Console locking**: the standard output lock is taken per write, never
//!   for the whole run, so a case that prints from a worker thread or from a
//!   thread it spawned does not block behind the reporter.
//!
//! # On `wasm32-unknown-unknown`
//!
//! The same target runs in Node under `scripts/wasm-test-runner.sh`, the cargo
//! runner `make wasm-test` sets. The standard library has no command line,
//! environment, console or clock there, so the harness takes all four from the
//! runner's host, and the output is the same console lines and tally.
//!
//! * Cases run serially: there are no threads. `--test-threads` is accepted
//!   and has no effect.
//! * A panic aborts a wasm32 module (the module traps; nothing unwinds), so a
//!   panicking case ends the run. It is still reported `FAILED` with its
//!   message, followed by the `failures:` section, a note giving the number of
//!   cases that did not run, and the tally. The runner then exits 101. A
//!   JavaScript exception that escapes a case, such as a read of a host clock
//!   that [`without_host_clock_or_entropy`] withdrew, is reported the same
//!   way with the exception's text. Because the run ends there, the message
//!   is shown in the `failures:` section even under `--nocapture`.
//! * The exit status is reported to the host by [`main`]. A module that
//!   returns without reporting one is refused by the runner.
//!
//! A `harness = false` target written as plain functions uses
//! [`harness_main!`](crate::harness_main) instead of writing `main` itself.

use std::any::Any;
use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};
use std::fmt::{self, Write as _};
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;
use std::sync::mpsc;
use std::sync::{Mutex, Once};

#[cfg(target_arch = "wasm32")]
use crate::host;

/// The exit status of a failed run or a refused command line, as libtest's.
pub const ERROR_EXIT_CODE: u8 = 101;

/// libtest's terse-format line width.
const TERSE_COLUMNS: usize = 88;

/// A case's failure, with the message shown in the `failures:` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    message: Option<String>,
}

impl Failed {
    /// A failure with no message.
    pub const fn without_message() -> Self {
        Self { message: None }
    }

    /// The failure's message, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl<M: fmt::Display> From<M> for Failed {
    fn from(message: M) -> Self {
        Self {
            message: Some(message.to_string()),
        }
    }
}

type Body = Box<dyn FnOnce() -> Result<(), Failed> + Send + 'static>;

/// One named case.
pub struct Trial {
    name: String,
    ignored: bool,
    body: Body,
}

purrdf_hash::debug_non_exhaustive!(Trial { name, ignored });

impl Trial {
    /// A case named `name` that runs `body`. A panic in `body` fails the case.
    pub fn test<F>(name: impl Into<String>, body: F) -> Self
    where
        F: FnOnce() -> Result<(), Failed> + Send + 'static,
    {
        Self {
            name: name.into(),
            ignored: false,
            body: Box::new(body),
        }
    }

    /// Mark the case ignored (`#[ignore]`): skipped unless `--ignored` or
    /// `--include-ignored` is given.
    #[must_use]
    pub fn with_ignored_flag(mut self, ignored: bool) -> Self {
        self.ignored = ignored;
        self
    }

    /// The case's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the case is marked ignored.
    pub const fn is_ignored(&self) -> bool {
        self.ignored
    }
}

/// libtest's console formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// One `test <name> ... <result>` line per case.
    Pretty,
    /// One character per case.
    Terse,
}

/// libtest's `--color` choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    /// Colour when standard output is a terminal.
    Auto,
    /// Always colour.
    Always,
    /// Never colour.
    Never,
}

/// What the invocation asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Run the selected cases (the default).
    Run,
    /// `--list`: print the selected cases' names instead of running them.
    List,
    /// `-h` / `--help`: print [`USAGE`]; takes precedence over `--list`.
    Help,
}

/// Which cases run with respect to their ignored flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunIgnored {
    /// Ignored cases are reported `ignored` (the default).
    No,
    /// Only ignored cases run (`--ignored`).
    Only,
    /// Every case runs (`--include-ignored`).
    Yes,
}

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arguments {
    /// Name filters; a case runs when its name contains any of them (or
    /// equals one, under `--exact`). Empty means every case.
    pub filters: Vec<String>,
    /// `--exact`.
    pub exact: bool,
    /// `--skip` filters, with the same matching rule as `filters`.
    pub skip: Vec<String>,
    /// `--list`, `--help`, or neither.
    pub action: Action,
    /// `--format` / `-q`.
    pub format: Format,
    /// `--ignored` / `--include-ignored`.
    pub run_ignored: RunIgnored,
    /// `--nocapture` or `RUST_TEST_NOCAPTURE`.
    pub nocapture: bool,
    /// `--show-output`.
    pub show_output: bool,
    /// `--color`.
    pub color: ColorChoice,
    /// `--test-threads` or `RUST_TEST_THREADS`; `None` means the available
    /// parallelism.
    pub test_threads: Option<usize>,
}

impl Default for Arguments {
    fn default() -> Self {
        Self {
            filters: Vec::new(),
            exact: false,
            skip: Vec::new(),
            action: Action::Run,
            format: Format::Pretty,
            run_ignored: RunIgnored::No,
            nocapture: false,
            show_output: false,
            color: ColorChoice::Auto,
            test_threads: None,
        }
    }
}

/// A refused command line. Its `Display` is libtest's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentError(String);

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ArgumentError {}

/// The usage text `--help` prints.
pub const USAGE: &str = "\
Usage: <test binary> [OPTIONS] [FILTERS...]

Options:
        --include-ignored
                        Run ignored and not ignored tests
        --ignored       Run only ignored tests
        --test-threads n_threads
                        Number of threads used for running tests in parallel
        --skip FILTER   Skip tests whose names contain FILTER (this flag can
                        be used multiple times)
        --exact         Exactly match filters rather than by substring
        --list          List all tests
    -q, --quiet         Display one character per test instead of one line.
                        Alias to --format=terse
        --nocapture     Show a failing test's panic message as it happens
                        (alias: --no-capture)
        --show-output   Show the successes section
        --color auto|always|never
                        Configure coloring of output
        --format pretty|terse
                        Configure formatting of output
    -Z unstable-options Accepted for libtest compatibility
    -h, --help          Display this message
";

impl Arguments {
    /// Parse the process's arguments (after the program name) and the
    /// `RUST_TEST_THREADS` / `RUST_TEST_NOCAPTURE` environment variables.
    ///
    /// On wasm32 both come from the runner's host: Node's command line after
    /// the module path, and Node's environment.
    pub fn from_env() -> Result<Self, ArgumentError> {
        let mut arguments = Self::parse(platform::args())?;
        if !arguments.nocapture {
            arguments.nocapture =
                platform::env_var("RUST_TEST_NOCAPTURE").is_some_and(|value| value != "0");
        }
        if arguments.test_threads.is_none()
            && let Some(value) = platform::env_var("RUST_TEST_THREADS")
        {
            arguments.test_threads = Some(parse_threads(&value).map_err(|_| {
                ArgumentError(format!(
                    "RUST_TEST_THREADS is `{value}`, should be a positive integer."
                ))
            })?);
        }
        Ok(arguments)
    }

    /// Parse `args` (without the program name), ignoring the environment.
    pub fn parse<I, S>(args: I) -> Result<Self, ArgumentError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut parsed = Self::default();
        let mut list = false;
        let mut help = false;
        let mut include_ignored = false;
        let mut only_ignored = false;
        let mut format: Option<String> = None;
        let mut quiet = false;
        let mut color: Option<String> = None;
        let mut threads: Option<String> = None;
        let mut rest = args
            .into_iter()
            .map(Into::into)
            .collect::<VecDeque<String>>();
        while let Some(argument) = rest.pop_front() {
            if argument == "--" {
                parsed.filters.extend(rest);
                break;
            }
            if let Some(long) = argument.strip_prefix("--") {
                let (name, inline) = match long.split_once('=') {
                    Some((name, value)) => (name, Some(value.to_owned())),
                    None => (long, None),
                };
                let flag = |set: &mut bool| {
                    if inline.is_some() {
                        return Err(ArgumentError(format!(
                            "Option '{name}' does not take an argument"
                        )));
                    }
                    *set = true;
                    Ok(())
                };
                match name {
                    "exact" => flag(&mut parsed.exact)?,
                    "list" => flag(&mut list)?,
                    "ignored" => flag(&mut only_ignored)?,
                    "include-ignored" => flag(&mut include_ignored)?,
                    "nocapture" | "no-capture" => flag(&mut parsed.nocapture)?,
                    "show-output" => flag(&mut parsed.show_output)?,
                    "quiet" => flag(&mut quiet)?,
                    "help" => flag(&mut help)?,
                    "skip" => parsed.skip.push(value_of(name, inline, &mut rest)?),
                    "format" => format = Some(value_of(name, inline, &mut rest)?),
                    "color" => color = Some(value_of(name, inline, &mut rest)?),
                    "test-threads" => threads = Some(value_of(name, inline, &mut rest)?),
                    _ => {
                        return Err(ArgumentError(format!("Unrecognized option: '{name}'")));
                    }
                }
            } else if let Some(short) = argument.strip_prefix('-').filter(|s| !s.is_empty()) {
                match short {
                    "q" => quiet = true,
                    "h" => help = true,
                    "Z" => unstable(&value_of("Z", None, &mut rest)?)?,
                    _ => {
                        if let Some(option) = short.strip_prefix('Z') {
                            unstable(option)?;
                            continue;
                        }
                        let first = short.chars().next().unwrap_or('-');
                        return Err(ArgumentError(format!("Unrecognized option: '{first}'")));
                    }
                }
            } else {
                parsed.filters.push(argument);
            }
        }

        parsed.action = if help {
            Action::Help
        } else if list {
            Action::List
        } else {
            Action::Run
        };
        if include_ignored && only_ignored {
            return Err(ArgumentError(
                "the options --include-ignored and --ignored are mutually exclusive".to_owned(),
            ));
        }
        parsed.run_ignored = if include_ignored {
            RunIgnored::Yes
        } else if only_ignored {
            RunIgnored::Only
        } else {
            RunIgnored::No
        };
        parsed.format = match (format.as_deref(), quiet) {
            (None | Some("pretty"), false) => Format::Pretty,
            (None | Some("terse"), true) | (Some("terse"), false) => Format::Terse,
            (Some("pretty"), true) => {
                return Err(ArgumentError(
                    "the options --format=pretty and -q/--quiet are mutually exclusive".to_owned(),
                ));
            }
            (Some(unsupported @ ("json" | "junit")), _) => {
                return Err(ArgumentError(format!(
                    "--format {unsupported} is not implemented by this harness \
                     (expected pretty or terse)"
                )));
            }
            (Some(other), _) => {
                return Err(ArgumentError(format!(
                    "argument for --format must be pretty, terse, json or junit (was {other})"
                )));
            }
        };
        parsed.color = match color.as_deref() {
            None | Some("auto") => ColorChoice::Auto,
            Some("always") => ColorChoice::Always,
            Some("never") => ColorChoice::Never,
            Some(other) => {
                return Err(ArgumentError(format!(
                    "argument for --color must be auto, always, or never (was {other})"
                )));
            }
        };
        if let Some(threads) = threads {
            parsed.test_threads = Some(parse_threads(&threads).map_err(|reason| {
                ArgumentError(format!("argument for --test-threads {reason}"))
            })?);
        }
        Ok(parsed)
    }
}

/// The value of an option taking one: inline after `=`, or the next argument.
fn value_of(
    name: &str,
    inline: Option<String>,
    rest: &mut VecDeque<String>,
) -> Result<String, ArgumentError> {
    inline
        .or_else(|| rest.pop_front())
        .ok_or_else(|| ArgumentError(format!("Argument to option '{name}' missing")))
}

/// `-Z <option>`: only `unstable-options` exists.
fn unstable(option: &str) -> Result<(), ArgumentError> {
    if option == "unstable-options" {
        Ok(())
    } else {
        Err(ArgumentError(format!("unknown -Z option: '{option}'")))
    }
}

/// A positive thread count.
fn parse_threads(value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(0) => Err("must not be 0".to_owned()),
        Ok(threads) => Ok(threads),
        Err(error) => Err(format!("must be a number > 0 (error: {error})")),
    }
}

/// What a run produced. `measured` is always 0: this runner has no benches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Conclusion {
    /// Cases that passed.
    pub passed: usize,
    /// Cases that failed.
    pub failed: usize,
    /// Cases reported `ignored`.
    pub ignored: usize,
    /// Cases removed by filters, `--skip` or `--ignored`.
    pub filtered_out: usize,
}

impl Conclusion {
    /// Whether any case failed.
    pub const fn has_failed(&self) -> bool {
        self.failed > 0
    }

    /// 0, or 101 when a case failed.
    pub fn exit_code(&self) -> ExitCode {
        if self.has_failed() {
            ExitCode::from(ERROR_EXIT_CODE)
        } else {
            ExitCode::SUCCESS
        }
    }
}

/// Parse the command line, run `trials`, print libtest's output to standard
/// output, and return the exit status. A refused command line prints
/// `error: <message>` to standard error and returns 101.
///
/// On wasm32 the status is also reported to the runner's host, which is where
/// the runner reads it; see the [module documentation](self).
///
/// `PURRDF_TEST_REQUIRE_EXACT=1` requires a nonempty exact selection and refuses
/// a run unless every requested filter executes one case. The focused WASM lane
/// sets this after its runner preflight. Unset or `0` keeps libtest's defaults.
pub fn main(trials: impl IntoIterator<Item = Trial>) -> ExitCode {
    let status = main_status(trials);
    platform::report_exit(status);
    ExitCode::from(status)
}

/// [`main`]'s status as a number, so that it can be reported before it
/// becomes an opaque [`ExitCode`].
fn main_status(trials: impl IntoIterator<Item = Trial>) -> u8 {
    let arguments = match Arguments::from_env() {
        Ok(arguments) => arguments,
        Err(error) => {
            platform::print_error(&format!("error: {error}\n"));
            return ERROR_EXIT_CODE;
        }
    };
    let expected = match platform::env_var("PURRDF_TEST_REQUIRE_EXACT").as_deref() {
        None | Some("0") => None,
        Some("1")
            if arguments.action == Action::Run
                && arguments.exact
                && !arguments.filters.is_empty() =>
        {
            Some(arguments.filters.len())
        }
        Some("1") => {
            platform::print_error(
                "error: PURRDF_TEST_REQUIRE_EXACT requires --exact and nonempty filters to run\n",
            );
            return ERROR_EXIT_CODE;
        }
        Some(value) => {
            platform::print_error(&format!(
                "error: PURRDF_TEST_REQUIRE_EXACT is `{value}`, expected 0 or 1\n"
            ));
            return ERROR_EXIT_CODE;
        }
    };
    if arguments.action == Action::Help {
        let mut out = Console::new();
        return match out.write_all(USAGE.as_bytes()).and_then(|()| out.flush()) {
            Ok(()) => 0,
            Err(error) => {
                platform::print_error(&format!("error: {error}\n"));
                ERROR_EXIT_CODE
            }
        };
    }
    let trials: Vec<Trial> = trials.into_iter().collect();
    let color = match arguments.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => platform::stdout_is_terminal(),
    };
    let mut out = Console::new();
    match run_to(&arguments, trials, &mut out, color) {
        Ok(conclusion) if conclusion.has_failed() => ERROR_EXIT_CODE,
        Ok(conclusion) => {
            let executed = conclusion.passed + conclusion.failed;
            if let Some(expected) = expected
                && executed != expected
            {
                platform::print_error(&format!(
                    "error: required {expected} exact cases to execute, but {executed} executed\n"
                ));
                return ERROR_EXIT_CODE;
            }
            0
        }
        Err(error) => {
            platform::print_error(&format!("error: {error}\n"));
            ERROR_EXIT_CODE
        }
    }
}

/// Write `line` and a newline to standard output as one write.
///
/// Natively this goes straight to the process's standard output, bypassing
/// the `print!` capture libtest installs, so the line reaches the console
/// under a plain `cargo test` without `--nocapture` as well as under this
/// runner. On wasm32 it goes to the runner's host console, where `println!`
/// reaches nothing. Panics if standard output cannot be written, as
/// `println!` does.
pub fn print_line(line: &str) {
    let mut text = String::with_capacity(line.len() + 1);
    text.push_str(line);
    text.push('\n');
    platform::print_raw(&text);
}

/// A determinism digest: `compute` run with every host clock and entropy source
/// withdrawn ([`without_host_clock_or_entropy`]), reported on one
/// `determinism-digest case=<case> digest=<16 hex digits> corpus_len=<n>` line
/// ([`print_line`]) for the cross-target determinism scripts to compare, and
/// returned. `corpus_len` is the size of the corpus the digest covers.
pub fn report_digest(case: &str, corpus_len: usize, compute: impl FnOnce() -> u64) -> u64 {
    let value = without_host_clock_or_entropy(compute);
    print_line(&format!(
        "determinism-digest case={case} digest={value:016x} corpus_len={corpus_len}"
    ));
    value
}

/// Run `computation` with every host clock and entropy source withdrawn, and
/// return its value.
///
/// On wasm32 each of the host's clocks (`Date`, `performance.now`,
/// `process.hrtime`, `process.uptime`) and entropy sources (`Math.random`,
/// Web Crypto's `getRandomValues` and `randomUUID`, and `node:crypto`'s random
/// functions) throws an error that names it for the duration. A computation
/// that reaches one fails its case by that source's name, rather than
/// producing an answer that agrees across targets by accident. Seals nest.
///
/// Natively there is no host to withdraw, and `computation` simply runs. The
/// wasm32 run of the same case is the one that observes the seal.
pub fn without_host_clock_or_entropy<T>(computation: impl FnOnce() -> T) -> T {
    #[cfg(target_arch = "wasm32")]
    {
        host::seal();
        let value = computation();
        host::unseal();
        value
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        computation()
    }
}

/// Run `trials` under `arguments`, writing libtest's console output to `out`
/// (uncoloured). Returns what the run concluded, or an error when the output
/// could not be written or two cases share a name.
///
/// On wasm32 a panicking case ends the module, and its report goes to the
/// runner's host console rather than to `out`.
pub fn run(
    arguments: &Arguments,
    trials: Vec<Trial>,
    out: &mut dyn Write,
) -> io::Result<Conclusion> {
    run_to(arguments, trials, out, false)
}

fn run_to(
    arguments: &Arguments,
    trials: Vec<Trial>,
    out: &mut dyn Write,
    color: bool,
) -> io::Result<Conclusion> {
    let mut names = BTreeSet::new();
    for trial in &trials {
        if !names.insert(trial.name.as_str()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "two cases are named `{}`; case names must be unique",
                    trial.name
                ),
            ));
        }
    }
    let total = trials.len();
    let selected: Vec<Trial> = trials
        .into_iter()
        .filter(|trial| is_selected(arguments, trial))
        .collect();
    let filtered_out = total - selected.len();

    if arguments.action == Action::List {
        for trial in &selected {
            writeln!(out, "{}: test", trial.name)?;
        }
        if arguments.format == Format::Pretty {
            if !selected.is_empty() {
                writeln!(out)?;
            }
            let noun = if selected.len() == 1 { "test" } else { "tests" };
            writeln!(out, "{} {noun}, 0 benchmarks", selected.len())?;
        }
        out.flush()?;
        return Ok(Conclusion {
            filtered_out,
            ..Conclusion::default()
        });
    }

    let started = platform::Stopwatch::start();
    let noun = if selected.len() == 1 { "test" } else { "tests" };
    out.write_all(format!("\nrunning {} {noun}\n", selected.len()).as_bytes())?;
    out.flush()?;

    let threads = platform::threads(arguments.test_threads);
    let mut report = Report {
        format: arguments.format,
        color,
        serial: threads == 1,
        show_output: arguments.show_output,
        total: selected.len(),
        done: 0,
        conclusion: Conclusion {
            filtered_out,
            ..Conclusion::default()
        },
        failures: Vec::new(),
        successes: Vec::new(),
    };

    install_panic_hook();
    let runs = |trial: &Trial| !trial.ignored || arguments.run_ignored != RunIgnored::No;
    let running = selected.iter().filter(|trial| runs(trial)).count();
    if threads == 1 || running <= 1 {
        let count = selected.len();
        for (index, trial) in selected.into_iter().enumerate() {
            if runs(&trial) {
                report.start(out, &trial.name)?;
                let armed = platform::arm(&report, &trial.name, count - index - 1, &started);
                let outcome = execute(&trial.name, trial.body, arguments.nocapture);
                if armed {
                    platform::disarm();
                }
                report.finish(out, &trial.name, &outcome)?;
            } else {
                report.finish(out, &trial.name, &Outcome::Ignored)?;
            }
        }
    } else {
        let mut queue = VecDeque::with_capacity(running);
        for trial in selected {
            if runs(&trial) {
                queue.push_back(trial);
            } else {
                report.finish(out, &trial.name, &Outcome::Ignored)?;
            }
        }
        let queue = Mutex::new(queue);
        let (sender, receiver) = mpsc::channel::<(String, Outcome)>();
        let nocapture = arguments.nocapture;
        std::thread::scope(|scope| -> io::Result<()> {
            for index in 0..threads.min(running) {
                let sender = sender.clone();
                let queue = &queue;
                std::thread::Builder::new()
                    .name(format!("purrdf-testkit-worker-{index}"))
                    .spawn_scoped(scope, move || {
                        loop {
                            let next = queue
                                .lock()
                                .expect(
                                    "the case queue is never poisoned: cases run outside the lock",
                                )
                                .pop_front();
                            let Some(trial) = next else { break };
                            let outcome = execute(&trial.name, trial.body, nocapture);
                            if sender.send((trial.name, outcome)).is_err() {
                                break;
                            }
                        }
                    })?;
            }
            drop(sender);
            for (name, outcome) in receiver {
                report.finish(out, &name, &outcome)?;
            }
            Ok(())
        })?;
    }

    let elapsed = format!("{:.2}", started.seconds());
    report.conclude(out, &elapsed, 0)?;
    Ok(report.conclusion)
}

/// Whether `trial` survives the name filters, `--skip` and `--ignored`.
fn is_selected(arguments: &Arguments, trial: &Trial) -> bool {
    let matches = |pattern: &String| {
        if arguments.exact {
            trial.name == *pattern
        } else {
            trial.name.contains(pattern.as_str())
        }
    };
    (arguments.filters.is_empty() || arguments.filters.iter().any(matches))
        && !arguments.skip.iter().any(matches)
        && (arguments.run_ignored != RunIgnored::Only || trial.ignored)
}

/// How one case ended.
#[derive(Debug)]
enum Outcome {
    Passed,
    Failed(String),
    Ignored,
}

thread_local! {
    /// The panic message of the case running on this thread, while one runs.
    static CAPTURE: RefCell<Option<Capture>> = const { RefCell::new(None) };
}

struct Capture {
    name: String,
    message: String,
    echo: bool,
}

/// Route panics on a thread running a case into that case's capture; every
/// other panic goes to the hook that was installed before.
///
/// On wasm32 the panic ends the module, so a case's panic is handed to the
/// host's armed report at once, and any other panic's message is written to
/// the host's standard error (the standard library's hook has no console
/// there).
fn install_panic_hook() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let location = info
                .location()
                .map_or_else(String::new, |location| format!(" at {location}"));
            let payload = panic_message(info.payload()).unwrap_or("Box<dyn Any>");
            let handled = CAPTURE.with(|slot| {
                let Ok(mut slot) = slot.try_borrow_mut() else {
                    return false;
                };
                let Some(capture) = slot.as_mut() else {
                    return false;
                };
                let text = format!("thread '{}' panicked{location}:\n{payload}\n", capture.name);
                if cfg!(target_arch = "wasm32") {
                    platform::fail_armed(&text);
                } else if capture.echo {
                    eprint!("{text}");
                }
                capture.message.push_str(&text);
                true
            });
            if !handled {
                if cfg!(target_arch = "wasm32") {
                    platform::print_error(&format!(
                        "thread 'main' panicked{location}:\n{payload}\n"
                    ));
                }
                previous(info);
            }
        }));
    });
}

/// The message a panic carried: its `&str` or `String` payload, the two types
/// `panic!` produces; `None` for any other payload. Every runner here that
/// reports a caught panic reads it through this one function.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

/// Run one case body under `catch_unwind`, capturing its panic message.
fn execute(name: &str, body: Body, echo: bool) -> Outcome {
    CAPTURE.with(|slot| {
        *slot.borrow_mut() = Some(Capture {
            name: name.to_owned(),
            message: String::new(),
            echo,
        });
    });
    let result = panic::catch_unwind(AssertUnwindSafe(body));
    let captured = CAPTURE
        .with(|slot| slot.borrow_mut().take())
        .map_or_default(|capture| capture.message);
    match result {
        Ok(Ok(())) => Outcome::Passed,
        Ok(Err(failed)) => Outcome::Failed(failed.message.unwrap_or_default()),
        Err(_) if echo => Outcome::Failed(String::new()),
        Err(_) => Outcome::Failed(captured),
    }
}

/// The console reporter: libtest's pretty and terse formatters.
///
/// It holds the run's state, not its output, so that on wasm32 a copy can
/// render the report a case would end the run with before that case starts.
#[derive(Clone)]
struct Report {
    format: Format,
    color: bool,
    serial: bool,
    show_output: bool,
    total: usize,
    done: usize,
    conclusion: Conclusion,
    failures: Vec<(String, String)>,
    successes: Vec<String>,
}

impl Report {
    /// Before a case runs: serial pretty output names it first, as libtest.
    fn start(&self, out: &mut dyn Write, name: &str) -> io::Result<()> {
        if self.format == Format::Pretty && self.serial {
            out.write_all(format!("test {name} ... ").as_bytes())?;
            out.flush()?;
        }
        Ok(())
    }

    /// After a case ends: its result, written as one piece.
    fn finish(&mut self, out: &mut dyn Write, name: &str, outcome: &Outcome) -> io::Result<()> {
        let (word, short, colour) = match outcome {
            Outcome::Passed => {
                self.conclusion.passed += 1;
                self.successes.push(name.to_owned());
                ("ok", ".", "32")
            }
            Outcome::Failed(message) => {
                self.conclusion.failed += 1;
                self.failures.push((name.to_owned(), message.clone()));
                ("FAILED", "F", "31")
            }
            Outcome::Ignored => {
                self.conclusion.ignored += 1;
                ("ignored", "i", "33")
            }
        };
        let mut text = String::new();
        match self.format {
            Format::Pretty => {
                let printed_name = self.serial && !matches!(outcome, Outcome::Ignored);
                if !printed_name {
                    let _ = write!(text, "test {name} ... ");
                }
                self.paint(&mut text, word, colour);
                text.push('\n');
            }
            Format::Terse => {
                self.paint(&mut text, short, colour);
                if self.done % TERSE_COLUMNS == TERSE_COLUMNS - 1 {
                    let _ = writeln!(text, " {}/{}", self.done + 1, self.total);
                }
            }
        }
        self.done += 1;
        out.write_all(text.as_bytes())?;
        out.flush()
    }

    fn paint(&self, text: &mut String, word: &str, colour: &str) {
        if self.color {
            let _ = write!(text, "\u{1b}[{colour}m{word}\u{1b}[0m");
        } else {
            text.push_str(word);
        }
    }

    /// The `successes:` section (under `--show-output`), the `failures:`
    /// section and the tally line, with
    /// `elapsed` as the duration. `not_run` counts the cases a wasm32 panic
    /// kept from running; a note names them when it is not zero.
    fn conclude(&mut self, out: &mut dyn Write, elapsed: &str, not_run: usize) -> io::Result<()> {
        let mut text = String::new();
        if self.show_output {
            text.push_str("\nsuccesses:\n");
            text.push_str("\nsuccesses:\n");
            self.successes.sort();
            for name in &self.successes {
                let _ = writeln!(text, "    {name}");
            }
        }
        let ok = self.conclusion.failed == 0;
        if !ok {
            text.push_str("\nfailures:\n");
            let blocks = self
                .failures
                .iter()
                .filter(|(_, message)| !message.is_empty())
                .fold(String::new(), |mut blocks, (name, message)| {
                    let _ = write!(blocks, "---- {name} stdout ----\n{message}\n");
                    blocks
                });
            if !blocks.is_empty() {
                text.push('\n');
                text.push_str(&blocks);
            }
            text.push_str("\nfailures:\n");
            let mut names: Vec<&str> = self
                .failures
                .iter()
                .map(|(name, _)| name.as_str())
                .collect();
            names.sort_unstable();
            for name in names {
                let _ = writeln!(text, "    {name}");
            }
        }
        if not_run > 0 {
            let noun = if not_run == 1 { "case" } else { "cases" };
            let _ = write!(
                text,
                "\nnote: {not_run} {noun} not run: a panic ends a wasm32 run at the case that panicked\n"
            );
        }
        text.push_str("\ntest result: ");
        if ok {
            self.paint(&mut text, "ok", "32");
        } else {
            self.paint(&mut text, "FAILED", "31");
        }
        let Conclusion {
            passed,
            failed,
            ignored,
            filtered_out,
        } = self.conclusion;
        let _ = write!(
            text,
            ". {passed} passed; {failed} failed; {ignored} ignored; 0 measured; \
             {filtered_out} filtered out; finished in {elapsed}s\n\n"
        );
        out.write_all(text.as_bytes())?;
        out.flush()
    }
}

use platform::Console;

/// The command line, environment, console and clock: the process's natively,
/// the runner's host on wasm32.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod platform {
    use std::io::{self, IsTerminal as _, Write};
    use std::time::Instant;

    use super::Report;

    /// The arguments after the program name.
    pub(crate) fn args() -> Vec<String> {
        std::env::args().skip(1).collect()
    }

    /// An environment variable, lossily decoded.
    pub(crate) fn env_var(name: &str) -> Option<String> {
        std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
    }

    /// Write `text` to standard error.
    pub(crate) fn print_error(text: &str) {
        eprint!("{text}");
    }

    /// Write `text` to standard output under one lock, and flush it.
    pub(crate) fn print_raw(text: &str) {
        let mut out = io::stdout().lock();
        if let Err(error) = out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
            panic!("failed printing to stdout: {error}");
        }
    }

    pub(super) fn stdout_is_terminal() -> bool {
        io::stdout().is_terminal()
    }

    /// The worker count: the request, or the available parallelism.
    pub(super) fn threads(requested: Option<usize>) -> usize {
        requested.unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
    }

    /// Nothing to report: the process's exit status is `main`'s.
    pub(crate) const fn report_exit(_status: u8) {}

    /// Natively a panicking case unwinds and the run goes on: nothing to arm.
    pub(super) const fn arm(
        _report: &Report,
        _name: &str,
        _not_run: usize,
        _started: &Stopwatch,
    ) -> bool {
        false
    }

    pub(super) const fn disarm() {}

    pub(super) const fn fail_armed(_message: &str) {}

    pub(crate) struct Stopwatch(Instant);

    impl Stopwatch {
        pub(crate) fn start() -> Self {
            Self(Instant::now())
        }

        pub(crate) fn seconds(&self) -> f64 {
            self.0.elapsed().as_secs_f64()
        }
    }

    /// Standard output, locked per write rather than per run, so a case
    /// printing from another thread never waits on the reporter.
    pub(super) struct Console;

    impl Console {
        pub(super) const fn new() -> Self {
            Self
        }
    }

    impl Write for Console {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            io::stdout().lock().write(buf)
        }

        fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
            io::stdout().lock().write_all(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            io::stdout().lock().flush()
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod platform {
    use std::io::{self, Write};

    use super::{Outcome, Report};
    use crate::host;

    pub(crate) fn args() -> Vec<String> {
        (0..host::arg_count()).map(host::arg).collect()
    }

    pub(crate) fn env_var(name: &str) -> Option<String> {
        host::env_var(name)
    }

    pub(crate) fn print_error(text: &str) {
        host::write_stderr(text);
    }

    pub(crate) fn print_raw(text: &str) {
        host::write_stdout(text);
    }

    pub(super) fn stdout_is_terminal() -> bool {
        host::stdout_is_terminal()
    }

    /// One: cases run serially, whatever was asked for.
    pub(super) const fn threads(_requested: Option<usize>) -> usize {
        1
    }

    pub(crate) fn report_exit(status: u8) {
        host::exit(status);
    }

    /// Hand the host the rest of the run's output as it will read if the case
    /// `name` ends the module: its `FAILED` result, the failure sections, the
    /// count of the `not_run` cases after it, and the tally.
    pub(super) fn arm(report: &Report, name: &str, not_run: usize, started: &Stopwatch) -> bool {
        let mut scratch = report.clone();
        let mut text = Vec::new();
        let rendered = scratch
            .finish(
                &mut text,
                name,
                &Outcome::Failed(host::MESSAGE_SLOT.to_owned()),
            )
            .and_then(|()| scratch.conclude(&mut text, host::ELAPSED_SLOT, not_run));
        if let Err(error) = rendered {
            // Rendering into memory cannot fail; say so loudly if it ever does.
            panic!("the abort report could not be rendered: {error}");
        }
        host::arm(name, &String::from_utf8_lossy(&text), started.0);
        true
    }

    pub(super) fn disarm() {
        host::disarm();
    }

    pub(super) fn fail_armed(message: &str) {
        host::fail(message);
    }

    /// The run's start, in the host's milliseconds.
    pub(crate) struct Stopwatch(f64);

    impl Stopwatch {
        pub(crate) fn start() -> Self {
            Self(host::now_millis())
        }

        pub(crate) fn seconds(&self) -> f64 {
            (host::now_millis() - self.0) / 1000.0
        }
    }

    /// The host's standard output. Text is forwarded as soon as it is complete
    /// UTF-8; an incomplete trailing sequence waits for the next write.
    pub(super) struct Console {
        pending: Vec<u8>,
    }

    impl Console {
        pub(super) const fn new() -> Self {
            Self {
                pending: Vec::new(),
            }
        }
    }

    impl Write for Console {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.pending.extend_from_slice(buf);
            let complete = match std::str::from_utf8(&self.pending) {
                Ok(_) => self.pending.len(),
                Err(error) if error.error_len().is_some() => self.pending.len(),
                Err(error) => error.valid_up_to(),
            };
            if complete > 0 {
                host::write_stdout(&String::from_utf8_lossy(&self.pending[..complete]));
                self.pending.drain(..complete);
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
