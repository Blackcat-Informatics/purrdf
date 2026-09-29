// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A bench binary's command line.

use std::collections::VecDeque;
use std::fmt;

use crate::harness::platform;

/// What the invocation asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Measure every selected benchmark (`cargo bench` passes `--bench`).
    Benchmark,
    /// Run every selected benchmark's routine once and report whether it ran:
    /// `--test`, and every invocation without `--bench` (`cargo test
    /// --benches`).
    Test,
    /// `--list`: print the selected benchmarks' ids.
    List,
    /// `-h` / `--help`: print [`USAGE`].
    Help,
}

/// Which saved record the measurements are compared against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    /// `--baseline <name>`: a benchmark with no record `<name>` fails the run.
    Strict(String),
    /// `--baseline-lenient <name>`: a benchmark with no record `<name>` is
    /// measured and recorded without a comparison.
    Lenient(String),
}

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arguments {
    /// What to do.
    pub mode: Mode,
    /// Id filters; a benchmark is selected when its id contains any of them
    /// (or equals one, under `--exact`). Empty selects every benchmark.
    pub filters: Vec<String>,
    /// `--exact`.
    pub exact: bool,
    /// `--quick`: at most 10 samples, 0.5 s of warm-up and 1 s of measurement
    /// per benchmark, with the same statistics over them.
    pub quick: bool,
    /// `--save-baseline <name>`: also write each benchmark's estimates as the
    /// record `<name>`, after comparing against that record's previous
    /// contents when it has any.
    pub save_baseline: Option<String>,
    /// `--baseline <name>` or `--baseline-lenient <name>`.
    pub baseline: Option<Comparison>,
}

impl Default for Arguments {
    /// What `cargo bench` with no further arguments asks for.
    fn default() -> Self {
        Self {
            mode: Mode::Benchmark,
            filters: Vec::new(),
            exact: false,
            quick: false,
            save_baseline: None,
            baseline: None,
        }
    }
}

/// A refused command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentError(String);

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ArgumentError {}

/// The record every run writes its latest estimates to; no baseline may take
/// its name.
pub const LATEST_RECORD: &str = "new";

/// The usage text `--help` prints.
pub const USAGE: &str = "\
Usage: <bench binary> [OPTIONS] [FILTERS...]

Measures every benchmark whose id (group/function/parameter) contains any
FILTER, and prints its median time per iteration with a 95% bootstrap
interval. Estimates are written under the benchmark store (PURRDF_BENCH_HOME,
or purrdf-bench/ beside the build's CARGO_TARGET_TMPDIR).

Options:
        --bench         Measure (cargo bench passes this); without it, or
                        with --test, each routine runs once instead
        --test          Run each selected routine once and report whether it
                        ran; nothing is measured or written
        --list          List the selected benchmarks' ids
        --exact         Select ids equal to a filter rather than containing it
        --quick         At most 10 samples, 0.5 s warm-up, 1 s measurement
        --save-baseline NAME
                        Also record the estimates as baseline NAME, compared
                        first against its previous contents
        --baseline NAME Compare against baseline NAME; a benchmark without it
                        fails the run
        --baseline-lenient NAME
                        Compare against baseline NAME where it exists
        --noplot        Accepted: this harness draws no plots
    -h, --help          Display this message
";

impl Arguments {
    /// Parse the process's arguments (after the program name). On wasm32 they
    /// come from the test runner's host.
    pub fn from_env() -> Result<Self, ArgumentError> {
        Self::parse(platform::args())
    }

    /// Parse `args` (without the program name).
    pub fn parse<I, S>(args: I) -> Result<Self, ArgumentError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut parsed = Self::default();
        let mut bench = false;
        let mut test = false;
        let mut list = false;
        let mut help = false;
        let mut rest = args
            .into_iter()
            .map(Into::into)
            .collect::<VecDeque<String>>();
        while let Some(argument) = rest.pop_front() {
            if argument == "--" {
                parsed.filters.extend(rest);
                break;
            }
            if argument == "-h" {
                help = true;
                continue;
            }
            let Some(long) = argument.strip_prefix("--") else {
                if argument.starts_with('-') {
                    return Err(ArgumentError(format!("unrecognized option `{argument}`")));
                }
                parsed.filters.push(argument);
                continue;
            };
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value.to_owned())),
                None => (long, None),
            };
            let flag = |set: &mut bool| {
                if inline.is_some() {
                    return Err(ArgumentError(format!(
                        "option `--{name}` does not take a value"
                    )));
                }
                *set = true;
                Ok(())
            };
            match name {
                "bench" => flag(&mut bench)?,
                "test" => flag(&mut test)?,
                "list" => flag(&mut list)?,
                "help" => flag(&mut help)?,
                "exact" => flag(&mut parsed.exact)?,
                "quick" => flag(&mut parsed.quick)?,
                // No plot is ever drawn, so the request is already met.
                "noplot" => flag(&mut bool::default())?,
                "save-baseline" => {
                    let value = baseline_name(name, inline, &mut rest)?;
                    if parsed.save_baseline.replace(value).is_some() {
                        return Err(ArgumentError(
                            "option `--save-baseline` is given twice".to_owned(),
                        ));
                    }
                }
                "baseline" | "baseline-lenient" => {
                    let value = baseline_name(name, inline, &mut rest)?;
                    let comparison = if name == "baseline" {
                        Comparison::Strict(value)
                    } else {
                        Comparison::Lenient(value)
                    };
                    if parsed.baseline.replace(comparison).is_some() {
                        return Err(ArgumentError(
                            "options `--baseline` and `--baseline-lenient` are given more than once between them".to_owned(),
                        ));
                    }
                }
                _ => {
                    return Err(ArgumentError(format!("unrecognized option `--{name}`")));
                }
            }
        }
        if parsed.save_baseline.is_some() && parsed.baseline.is_some() {
            return Err(ArgumentError(
                "option `--save-baseline` cannot be combined with `--baseline` or `--baseline-lenient`: a run either records a baseline or compares against one".to_owned(),
            ));
        }
        if list && test {
            return Err(ArgumentError(
                "options `--list` and `--test` cannot be combined".to_owned(),
            ));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let store_flag = if parsed.save_baseline.is_some() {
                Some("--save-baseline")
            } else {
                match parsed.baseline {
                    Some(Comparison::Strict(_)) => Some("--baseline"),
                    Some(Comparison::Lenient(_)) => Some("--baseline-lenient"),
                    None => None,
                }
            };
            if let Some(flag) = store_flag {
                return Err(ArgumentError(format!(
                    "option `{flag}` reads or writes the benchmark store, which needs a file system, and wasm32-unknown-unknown has none"
                )));
            }
        }
        parsed.mode = if help {
            Mode::Help
        } else if list {
            Mode::List
        } else if test || !bench {
            Mode::Test
        } else {
            Mode::Benchmark
        };
        Ok(parsed)
    }

    /// Whether the benchmark `id` is selected.
    pub fn selects(&self, id: &str) -> bool {
        self.filters.is_empty()
            || self.filters.iter().any(|filter| {
                if self.exact {
                    id == filter
                } else {
                    id.contains(filter.as_str())
                }
            })
    }
}

/// The value of the baseline option `--name`, checked to be a usable record
/// name: one path component of ASCII letters, digits, `-`, `_` and `.`, not
/// starting with `.`, and not the latest-run record's name.
fn baseline_name(
    name: &str,
    inline: Option<String>,
    rest: &mut VecDeque<String>,
) -> Result<String, ArgumentError> {
    let Some(value) = inline.or_else(|| rest.pop_front()) else {
        return Err(ArgumentError(format!(
            "option `--{name}` requires a baseline name"
        )));
    };
    let well_formed = !value.is_empty()
        && !value.starts_with('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if !well_formed {
        return Err(ArgumentError(format!(
            "option `--{name}`: baseline name `{value}` must be ASCII letters, digits, `-`, `_` and `.`, not starting with `.`"
        )));
    }
    if value == LATEST_RECORD {
        return Err(ArgumentError(format!(
            "option `--{name}`: `{LATEST_RECORD}` is the latest run's record, not a baseline name"
        )));
    }
    Ok(value)
}
