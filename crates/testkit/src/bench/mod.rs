// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's micro-benchmark harness, for `harness = false` bench
//! targets.
//!
//! ```no_run
//! use purrdf_testkit::bench::{Bench, BenchmarkId, Throughput, bench_group, bench_main, black_box};
//!
//! fn sum(c: &mut Bench) {
//!     let mut group = c.benchmark_group("sum");
//!     for len in [64_u64, 4096] {
//!         let data: Vec<u64> = (0..len).collect();
//!         group.throughput(Throughput::Elements(len));
//!         group.bench_with_input(BenchmarkId::new("iter", len), &data, |b, data| {
//!             b.iter(|| black_box(data).iter().sum::<u64>());
//!         });
//!     }
//!     group.finish();
//! }
//!
//! bench_group!(benches, sum);
//! bench_main!(benches);
//! ```
//!
//! # Method
//!
//! Each selected benchmark is measured in three phases:
//!
//! 1. **Warm-up.** The routine runs 1, 2, 4, … iterations at a time until the
//!    warm-up time (3 s by default) has passed, which also estimates its time
//!    per iteration.
//! 2. **Sampling.** Every sample times the same iteration count, chosen so the
//!    samples together last the measurement time (5 s by default); there are
//!    100 samples by default and never fewer than 10. A routine too slow for
//!    that budget still gets its samples, with a printed warning naming the
//!    time they will take.
//! 3. **Analysis** ([`stats`]). The median time per iteration, its median
//!    absolute deviation, and a 95% bootstrap percentile interval of the median
//!    over 10 000 resamples seeded from the benchmark's id through
//!    `purrdf_hash::mix`, so the same samples always give the same interval.
//!    Outliers are counted by Tukey's fences and kept. A declared
//!    [`Throughput`] is reported as a rate with the same interval.
//!
//! A comparison against a saved record bootstraps the relative change of the
//! median from both records' samples. The change is reported as
//! `Performance has regressed.` when its whole interval lies above +1%,
//! `Performance has improved.` when it lies below −1%, `Change within noise
//! threshold.` when it excludes zero but not the ±1% band, and `No change in
//! performance detected.` otherwise.
//!
//! # Command line
//!
//! See [`USAGE`] and [`Arguments`]: a filter, `--exact`, `--bench`, `--test`,
//! `--list`, `--quick`, `--save-baseline`, `--baseline`, `--baseline-lenient`,
//! `--noplot` and `--help`. Every other option is refused by name, with exit
//! status 101.
//!
//! # Output
//!
//! One summary line per measured benchmark, followed by its throughput, its
//! change against a record and its outliers when it has them, and one
//! estimates file per benchmark and record (see [`estimates`] for the schema,
//! and [`store`] for where the files live).
//!
//! # On `wasm32-unknown-unknown`
//!
//! A bench binary runs in Node under `scripts/wasm-test-runner.sh`, like a
//! `harness = false` test: the command line, the console, the exit status and
//! the clock (`performance.now`) all come from the runner's host. There is no
//! file system, so the store options are refused and a measured run prints its
//! estimates without writing them, saying so once.
//!
//! # Matching and sampling
//!
//! Filters match ids as substrings, like libtest's, not as regular
//! expressions. Every sample times the same iteration count (flat sampling),
//! which is what makes the median of per-iteration times meaningful.

mod args;
pub mod estimates;
pub mod stats;
pub mod store;

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};
use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Duration;

pub use std::hint::black_box;

pub use self::args::{ArgumentError, Arguments, Comparison, LATEST_RECORD, Mode, USAGE};
pub use self::estimates::{Estimates, EstimatesError};
pub use self::stats::Outliers;
#[cfg(not(target_arch = "wasm32"))]
use self::store::Store;
pub use crate::{bench_group, bench_main};

use crate::harness::{ERROR_EXIT_CODE, platform};

/// The relative change inside which a difference is reported as noise.
pub const NOISE_THRESHOLD: f64 = 0.01;

/// The fewest samples a benchmark may take.
pub const MIN_SAMPLE_SIZE: usize = 10;

/// The work one iteration of a benchmark does, for a rate beside its time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Throughput {
    /// Bytes processed per iteration.
    Bytes(u64),
    /// Elements processed per iteration.
    Elements(u64),
}

/// How many inputs [`Bencher::iter_batched`] and
/// [`Bencher::iter_batched_ref`] set up before timing a batch of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchSize {
    /// Inputs are cheap to hold: each sample's iterations run in 10 batches.
    SmallInput,
    /// Inputs are large: each sample's iterations run in 1000 batches.
    LargeInput,
}

impl BatchSize {
    /// The iterations per batch for a sample of `iterations`.
    const fn per_batch(self, iterations: u64) -> u64 {
        let batches = match self {
            Self::SmallInput => 10,
            Self::LargeInput => 1000,
        };
        iterations.div_ceil(batches)
    }
}

/// A benchmark's name within its group: a function name, a parameter, or
/// both, joined by `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkId {
    function: Option<String>,
    parameter: Option<String>,
}

impl BenchmarkId {
    /// The benchmark `function` at `parameter`.
    pub fn new(function: impl Into<String>, parameter: impl fmt::Display) -> Self {
        Self {
            function: Some(function.into()),
            parameter: Some(parameter.to_string()),
        }
    }

    /// A benchmark named by its parameter alone.
    pub fn from_parameter(parameter: impl fmt::Display) -> Self {
        Self {
            function: None,
            parameter: Some(parameter.to_string()),
        }
    }
}

/// What a group's benchmark may be named by: a [`BenchmarkId`], or a string
/// naming its function.
pub trait IntoBenchmarkId {
    /// The id.
    fn into_benchmark_id(self) -> BenchmarkId;
}

impl IntoBenchmarkId for BenchmarkId {
    fn into_benchmark_id(self) -> BenchmarkId {
        self
    }
}

impl<S: Into<String>> IntoBenchmarkId for S {
    fn into_benchmark_id(self) -> BenchmarkId {
        BenchmarkId {
            function: Some(self.into()),
            parameter: None,
        }
    }
}

/// A benchmark's full id: its group, then its function and parameter when it
/// has them.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FullId {
    components: Vec<String>,
}

impl FullId {
    fn new(group: &str, id: BenchmarkId) -> Self {
        let components: Vec<String> = std::iter::once(group.to_owned())
            .chain(id.function)
            .chain(id.parameter)
            .collect();
        for component in &components {
            assert!(
                !component.is_empty(),
                "benchmark id `{}` has an empty component",
                components.join("/")
            );
        }
        Self { components }
    }

    fn text(&self) -> String {
        self.components.join("/")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parts(&self) -> Vec<&str> {
        self.components.iter().map(String::as_str).collect()
    }
}

/// Timing parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Config {
    sample_size: usize,
    warm_up_time: Duration,
    measurement_time: Duration,
}

impl Config {
    const DEFAULT: Self = Self {
        sample_size: 100,
        warm_up_time: Duration::from_secs(3),
        measurement_time: Duration::from_secs(5),
    };

    /// `--quick`'s caps.
    fn quick(self) -> Self {
        Self {
            sample_size: MIN_SAMPLE_SIZE,
            warm_up_time: self.warm_up_time.min(Duration::from_millis(500)),
            measurement_time: self.measurement_time.min(Duration::from_secs(1)),
        }
    }
}

fn checked_sample_size(n: usize) -> usize {
    assert!(
        n >= MIN_SAMPLE_SIZE,
        "sample size {n} is below the minimum of {MIN_SAMPLE_SIZE}"
    );
    n
}

fn checked_duration(what: &str, duration: Duration) -> Duration {
    assert!(!duration.is_zero(), "the {what} must be longer than zero");
    duration
}

/// A comparison of a benchmark's new estimates with a saved record.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// The record compared against.
    pub record: String,
    /// `median(new) / median(record) - 1`.
    pub point: f64,
    /// The lower bound of the change's bootstrap interval.
    pub ci_low: f64,
    /// The upper bound of the change's bootstrap interval.
    pub ci_high: f64,
    /// What the interval says.
    pub verdict: Verdict,
}

/// What a change's interval says against [`NOISE_THRESHOLD`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The whole interval is above `+NOISE_THRESHOLD`.
    Regressed,
    /// The whole interval is below `-NOISE_THRESHOLD`.
    Improved,
    /// The interval excludes zero but reaches into the noise band.
    WithinNoise,
    /// The interval contains zero.
    NoChange,
}

impl Verdict {
    /// The verdict of the interval `[low, high]`.
    pub fn of(low: f64, high: f64) -> Self {
        if low > NOISE_THRESHOLD {
            Self::Regressed
        } else if high < -NOISE_THRESHOLD {
            Self::Improved
        } else if low > 0.0 || high < 0.0 {
            Self::WithinNoise
        } else {
            Self::NoChange
        }
    }

    /// The sentence the console prints.
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::Regressed => "Performance has regressed.",
            Self::Improved => "Performance has improved.",
            Self::WithinNoise => "Change within noise threshold.",
            Self::NoChange => "No change in performance detected.",
        }
    }
}

impl Change {
    /// The change from `base` to `new`, bootstrapped with `seed`.
    pub fn between(record: &str, new: &Estimates, base: &Estimates, seed: u64) -> Self {
        let (point, interval) = stats::bootstrap_change(
            &new.samples,
            &base.samples,
            stats::RESAMPLES,
            stats::CONFIDENCE,
            seed,
        );
        Self {
            record: record.to_owned(),
            point,
            ci_low: interval.low,
            ci_high: interval.high,
            verdict: Verdict::of(interval.low, interval.high),
        }
    }
}

/// One benchmark's outcome in a run.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The full id, `group[/function][/parameter]`.
    pub id: String,
    /// The estimates, when the benchmark was measured.
    pub estimates: Option<Estimates>,
    /// The comparison, when there was a record to compare against.
    pub change: Option<Change>,
    /// Why the benchmark failed, when it did.
    pub failure: Option<String>,
}

/// Every benchmark a run has seen.
#[derive(Debug, Default)]
struct Tally {
    ids: BTreeSet<String>,
    outcomes: Vec<Outcome>,
    /// Whether the note that nothing is written has been printed.
    #[cfg(target_arch = "wasm32")]
    store_note_printed: bool,
}

/// The process-wide run [`bench_main!`] sets up.
#[derive(Debug)]
struct Global {
    arguments: Arguments,
    #[cfg(not(target_arch = "wasm32"))]
    store: Store,
}

static GLOBAL: OnceLock<Global> = OnceLock::new();
static GLOBAL_TALLY: Mutex<Tally> = Mutex::new(Tally {
    ids: BTreeSet::new(),
    outcomes: Vec::new(),
    #[cfg(target_arch = "wasm32")]
    store_note_printed: false,
});

fn global_tally() -> MutexGuard<'static, Tally> {
    GLOBAL_TALLY.lock().unwrap_or_else(PoisonError::into_inner)
}

fn global() -> &'static Global {
    GLOBAL.get_or_init(|| match Arguments::from_env() {
        Ok(arguments) => Global {
            arguments,
            #[cfg(not(target_arch = "wasm32"))]
            store: Store::resolve(platform::env_var(store::HOME_VARIABLE), None),
        },
        Err(error) => panic!("error: {error}"),
    })
}

/// Where a run's tally and console go.
#[derive(Debug)]
enum Sink {
    /// The process's run: the global tally, the console.
    Process,
    /// A run of its own, with its console text kept for inspection.
    Own { tally: Tally, console: String },
}

/// A benchmark run's driver: its timing configuration, its arguments, and
/// where its results go.
#[derive(Debug)]
pub struct Bench {
    config: Config,
    arguments: Arguments,
    #[cfg(not(target_arch = "wasm32"))]
    store: Store,
    sink: Sink,
}

impl Default for Bench {
    /// The default timing (100 samples, 3 s warm-up, 5 s measurement) with
    /// `cargo bench`'s default arguments, reporting to the process's run.
    fn default() -> Self {
        Self {
            config: Config::DEFAULT,
            arguments: Arguments::default(),
            #[cfg(not(target_arch = "wasm32"))]
            store: Store::resolve(platform::env_var(store::HOME_VARIABLE), None),
            sink: Sink::Process,
        }
    }
}

impl Bench {
    /// A run with `arguments` of its own: its outcomes are kept apart from the
    /// process's and its console text is returned by [`Self::console`] rather
    /// than printed. Its store is `PURRDF_BENCH_HOME` until
    /// [`Self::with_store`] places it.
    pub fn with_arguments(arguments: Arguments) -> Self {
        Self {
            arguments,
            sink: Sink::Own {
                tally: Tally::default(),
                console: String::new(),
            },
            ..Self::default()
        }
    }

    /// Place the store at `root`.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn with_store(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.store = Store::Directory(root.into());
        self
    }

    /// Take the process's arguments (and store) as [`bench_main!`] parsed
    /// them, parsing them here when no `bench_main!` did.
    #[must_use]
    pub fn configure_from_args(mut self) -> Self {
        let global = global();
        self.arguments = global.arguments.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store = global.store.clone();
        }
        self.sink = Sink::Process;
        self
    }

    /// Take `n` samples per benchmark (at least [`MIN_SAMPLE_SIZE`]).
    #[must_use]
    pub fn sample_size(mut self, n: usize) -> Self {
        self.config.sample_size = checked_sample_size(n);
        self
    }

    /// Warm each benchmark up for `duration` (longer than zero).
    #[must_use]
    pub fn warm_up_time(mut self, duration: Duration) -> Self {
        self.config.warm_up_time = checked_duration("warm-up time", duration);
        self
    }

    /// Spread each benchmark's samples over `duration` (longer than zero).
    #[must_use]
    pub fn measurement_time(mut self, duration: Duration) -> Self {
        self.config.measurement_time = checked_duration("measurement time", duration);
        self
    }

    /// A group of benchmarks named `group`, sharing this run's configuration
    /// until the group changes its own.
    pub fn benchmark_group(&mut self, group: impl Into<String>) -> BenchmarkGroup<'_> {
        let name = group.into();
        assert!(!name.is_empty(), "a benchmark group needs a name");
        BenchmarkGroup {
            config: self.config,
            bench: self,
            name,
            throughput: None,
        }
    }

    /// Measure `routine` as the benchmark `id` (a group of one).
    pub fn bench_function<F>(&mut self, id: &str, mut routine: F) -> &mut Self
    where
        F: FnMut(&mut Bencher),
    {
        let full = FullId::new(
            id,
            BenchmarkId {
                function: None,
                parameter: None,
            },
        );
        let config = self.config;
        self.run(&full, config, None, &mut routine);
        self
    }

    /// This run's console text, for a run made by [`Self::with_arguments`];
    /// empty for the process's run, which prints as it goes.
    pub fn console(&self) -> &str {
        match &self.sink {
            Sink::Process => "",
            Sink::Own { console, .. } => console,
        }
    }

    /// Every benchmark this run has seen, in order.
    pub fn outcomes(&self) -> Vec<Outcome> {
        match &self.sink {
            Sink::Process => global_tally().outcomes.clone(),
            Sink::Own { tally, .. } => tally.outcomes.clone(),
        }
    }

    fn emit(&mut self, text: &str) {
        match &mut self.sink {
            Sink::Process => platform::print_raw(text),
            Sink::Own { console, .. } => console.push_str(text),
        }
    }

    fn with_tally<R>(&mut self, body: impl FnOnce(&mut Tally) -> R) -> R {
        match &mut self.sink {
            Sink::Process => body(&mut global_tally()),
            Sink::Own { tally, .. } => body(tally),
        }
    }

    fn record(&mut self, outcome: Outcome) {
        self.with_tally(|tally| tally.outcomes.push(outcome));
    }

    /// Run one benchmark as the arguments ask.
    fn run(
        &mut self,
        id: &FullId,
        config: Config,
        throughput: Option<Throughput>,
        routine: &mut dyn FnMut(&mut Bencher),
    ) {
        let text = id.text();
        if !self.arguments.selects(&text) {
            return;
        }
        let fresh = self.with_tally(|tally| tally.ids.insert(text.clone()));
        assert!(fresh, "benchmark id `{text}` is used twice in one run");
        match self.arguments.mode {
            Mode::Help => {}
            Mode::List => self.emit(&format!("{text}: benchmark\n")),
            Mode::Test => self.test(text, routine),
            Mode::Benchmark => self.measure(id, text, config, throughput, routine),
        }
    }

    fn test(&mut self, text: String, routine: &mut dyn FnMut(&mut Bencher)) {
        self.emit(&format!("Testing {text}\n"));
        let result = panic::catch_unwind(AssertUnwindSafe(|| time_iterations(routine, 1)));
        let failure = result.err().map(|payload| panic_text(&*payload));
        match &failure {
            None => self.emit("Success\n"),
            Some(message) => self.emit(&format!("FAILED: {message}\n")),
        }
        self.record(Outcome {
            id: text,
            estimates: None,
            change: None,
            failure,
        });
    }

    fn fail(&mut self, text: String, message: String) {
        self.emit(&format!("{text}: FAILED: {message}\n"));
        self.record(Outcome {
            id: text,
            estimates: None,
            change: None,
            failure: Some(message),
        });
    }

    fn measure(
        &mut self,
        id: &FullId,
        text: String,
        config: Config,
        throughput: Option<Throughput>,
        routine: &mut dyn FnMut(&mut Bencher),
    ) {
        let config = if self.arguments.quick {
            config.quick()
        } else {
            config
        };
        let baseline = match self.load_baseline(id) {
            Ok(baseline) => baseline,
            Err(message) => return self.fail(text, message),
        };
        let sampled = match panic::catch_unwind(AssertUnwindSafe(|| sample(config, routine))) {
            Ok(sampled) => sampled,
            Err(payload) => {
                return self.fail(text, panic_text(&*payload));
            }
        };
        let seed = purrdf_hash::fnv::fnv1a64(text.as_bytes());
        let estimates = Estimates::from_samples(
            sampled.samples,
            sampled.iterations_per_sample,
            throughput,
            seed,
        );
        let change = baseline.map(|(record, base)| {
            Change::between(
                &record,
                &estimates,
                &base,
                purrdf_hash::mix::splitmix64_finalize(seed),
            )
        });
        let mut report = String::new();
        if let Some(warning) = sampled.warning {
            let _ = writeln!(report, "warning: {text}: {warning}");
        }
        write_summary(&mut report, &text, &estimates, change.as_ref());
        self.emit(&report);
        let failure = self.save(id, &estimates).err();
        if let Some(message) = &failure {
            self.emit(&format!("{text}: FAILED: {message}\n"));
        }
        self.record(Outcome {
            id: text,
            estimates: Some(estimates),
            change,
            failure,
        });
    }

    /// The record to compare against and its estimates: the explicit
    /// baseline, else the baseline being saved, else the previous run. Only a
    /// strict baseline's absence is an error.
    #[cfg(not(target_arch = "wasm32"))]
    fn load_baseline(&self, id: &FullId) -> Result<Option<(String, Estimates)>, String> {
        let (record, strict) = match (&self.arguments.baseline, &self.arguments.save_baseline) {
            (Some(Comparison::Strict(name)), _) => (name.as_str(), true),
            (Some(Comparison::Lenient(name)), _) | (None, Some(name)) => (name.as_str(), false),
            (None, None) => (LATEST_RECORD, false),
        };
        let parts = id.parts();
        let path = || {
            self.store
                .record_path(&parts, record)
                .map_or_else(|error| error, |path| path.display().to_string())
        };
        match self.store.read(&parts, record) {
            Ok(Some(estimates)) => Ok(Some((record.to_owned(), estimates))),
            Ok(None) if strict => Err(format!("no baseline `{record}` at {}", path())),
            Ok(None) => Ok(None),
            // With nothing to compare against and nowhere to write, the save
            // reports the missing store; a missing store is not a missing record.
            Err(_) if !strict && matches!(self.store, Store::Unplaced) => Ok(None),
            Err(error) => Err(error),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn load_baseline(&self, _id: &FullId) -> Result<Option<(String, Estimates)>, String> {
        Ok(None)
    }

    /// Write the latest record, and the baseline being saved.
    #[cfg(not(target_arch = "wasm32"))]
    fn save(&self, id: &FullId, estimates: &Estimates) -> Result<(), String> {
        let parts = id.parts();
        self.store.write(&parts, LATEST_RECORD, estimates)?;
        if let Some(name) = &self.arguments.save_baseline {
            self.store.write(&parts, name, estimates)?;
        }
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    fn save(&mut self, _id: &FullId, _estimates: &Estimates) -> Result<(), String> {
        let first =
            self.with_tally(|tally| !std::mem::replace(&mut tally.store_note_printed, true));
        if first {
            self.emit(
                "note: wasm32-unknown-unknown has no file system: estimates are printed, not written\n",
            );
        }
        Ok(())
    }
}

/// A group of benchmarks sharing a name prefix, a timing configuration and a
/// throughput.
#[derive(Debug)]
pub struct BenchmarkGroup<'a> {
    bench: &'a mut Bench,
    name: String,
    config: Config,
    throughput: Option<Throughput>,
}

impl BenchmarkGroup<'_> {
    /// Take `n` samples per benchmark in this group (at least
    /// [`MIN_SAMPLE_SIZE`]).
    pub fn sample_size(&mut self, n: usize) -> &mut Self {
        self.config.sample_size = checked_sample_size(n);
        self
    }

    /// Warm each benchmark in this group up for `duration`.
    pub fn warm_up_time(&mut self, duration: Duration) -> &mut Self {
        self.config.warm_up_time = checked_duration("warm-up time", duration);
        self
    }

    /// Spread each benchmark's samples in this group over `duration`.
    pub fn measurement_time(&mut self, duration: Duration) -> &mut Self {
        self.config.measurement_time = checked_duration("measurement time", duration);
        self
    }

    /// Report the benchmarks that follow with `throughput` per iteration.
    pub fn throughput(&mut self, throughput: Throughput) -> &mut Self {
        self.throughput = Some(throughput);
        self
    }

    /// Measure `routine` as the benchmark `id` of this group.
    pub fn bench_function<Id, F>(&mut self, id: Id, mut routine: F) -> &mut Self
    where
        Id: IntoBenchmarkId,
        F: FnMut(&mut Bencher),
    {
        let full = FullId::new(&self.name, id.into_benchmark_id());
        self.bench
            .run(&full, self.config, self.throughput, &mut routine);
        self
    }

    /// Measure `routine` over `input` as the benchmark `id` of this group.
    pub fn bench_with_input<Id, F, I>(&mut self, id: Id, input: &I, mut routine: F) -> &mut Self
    where
        Id: IntoBenchmarkId,
        F: FnMut(&mut Bencher, &I),
        I: ?Sized,
    {
        let full = FullId::new(&self.name, id.into_benchmark_id());
        self.bench.run(
            &full,
            self.config,
            self.throughput,
            &mut |bencher: &mut Bencher| routine(bencher, input),
        );
        self
    }

    /// End the group.
    pub fn finish(self) {}
}

/// Times a routine over the iteration count the harness asks for. A
/// benchmark's closure must call exactly one of its `iter` methods.
#[derive(Debug)]
pub struct Bencher {
    iterations: u64,
    elapsed_nanos: f64,
    timed: bool,
}

impl Bencher {
    const fn new(iterations: u64) -> Self {
        Self {
            iterations,
            elapsed_nanos: 0.0,
            timed: false,
        }
    }

    fn mark_timed(&mut self) {
        assert!(
            !self.timed,
            "a benchmark's closure called Bencher's `iter` methods more than once"
        );
        self.timed = true;
    }

    /// Time `routine`; each output is passed through [`black_box`] and dropped
    /// inside the timed region.
    #[expect(
        clippy::iter_not_returning_iterator,
        reason = "`iter` is the name every bench in the workspace times its routine with"
    )]
    pub fn iter<O, R>(&mut self, mut routine: R)
    where
        R: FnMut() -> O,
    {
        self.mark_timed();
        let clock = platform::Stopwatch::start();
        for _ in 0..self.iterations {
            black_box(routine());
        }
        self.elapsed_nanos = clock.seconds() * 1e9;
    }

    /// Time `routine`, dropping its outputs after the timed region.
    pub fn iter_with_large_drop<O, R>(&mut self, mut routine: R)
    where
        R: FnMut() -> O,
    {
        self.mark_timed();
        let mut outputs = Vec::with_capacity(self.iterations as usize);
        let clock = platform::Stopwatch::start();
        for _ in 0..self.iterations {
            outputs.push(black_box(routine()));
        }
        self.elapsed_nanos = clock.seconds() * 1e9;
        drop(black_box(outputs));
    }

    /// Time `routine` over inputs made by `setup`, which runs outside the
    /// timed region, in batches of `size`; outputs are dropped after it.
    pub fn iter_batched<I, O, S, R>(&mut self, mut setup: S, mut routine: R, size: BatchSize)
    where
        S: FnMut() -> I,
        R: FnMut(I) -> O,
    {
        self.mark_timed();
        let per_batch = size.per_batch(self.iterations);
        let mut remaining = self.iterations;
        let mut elapsed = 0.0;
        while remaining > 0 {
            let batch = per_batch.min(remaining);
            let inputs = black_box((0..batch).map(|_| setup()).collect::<Vec<I>>());
            let mut outputs = Vec::with_capacity(batch as usize);
            let clock = platform::Stopwatch::start();
            outputs.extend(inputs.into_iter().map(&mut routine));
            elapsed += clock.seconds();
            drop(black_box(outputs));
            remaining -= batch;
        }
        self.elapsed_nanos = elapsed * 1e9;
    }

    /// Time `routine` over mutable references to inputs made by `setup`,
    /// which runs outside the timed region, in batches of `size`; inputs and
    /// outputs are dropped after it.
    pub fn iter_batched_ref<I, O, S, R>(&mut self, mut setup: S, mut routine: R, size: BatchSize)
    where
        S: FnMut() -> I,
        R: FnMut(&mut I) -> O,
    {
        self.mark_timed();
        let per_batch = size.per_batch(self.iterations);
        let mut remaining = self.iterations;
        let mut elapsed = 0.0;
        while remaining > 0 {
            let batch = per_batch.min(remaining);
            let mut inputs = black_box((0..batch).map(|_| setup()).collect::<Vec<I>>());
            let mut outputs = Vec::with_capacity(batch as usize);
            let clock = platform::Stopwatch::start();
            outputs.extend(inputs.iter_mut().map(&mut routine));
            elapsed += clock.seconds();
            drop(black_box(outputs));
            drop(inputs);
            remaining -= batch;
        }
        self.elapsed_nanos = elapsed * 1e9;
    }
}

/// Run `routine` over `iterations` and return the time it timed, in
/// nanoseconds.
fn time_iterations(routine: &mut dyn FnMut(&mut Bencher), iterations: u64) -> f64 {
    let mut bencher = Bencher::new(iterations);
    routine(&mut bencher);
    assert!(
        bencher.timed,
        "a benchmark's closure must call one of Bencher's `iter` methods"
    );
    bencher.elapsed_nanos
}

/// A benchmark's raw measurements.
struct Sampled {
    samples: Vec<f64>,
    iterations_per_sample: u64,
    warning: Option<String>,
}

/// Warm `routine` up, then take `config.sample_size` samples of it.
fn sample(config: Config, routine: &mut dyn FnMut(&mut Bencher)) -> Sampled {
    let warm_up = platform::Stopwatch::start();
    let warm_up_seconds = config.warm_up_time.as_secs_f64();
    let mut iterations = 1_u64;
    let mut total_iterations = 0_u64;
    let mut total_nanos = 0.0;
    loop {
        total_nanos += time_iterations(routine, iterations);
        total_iterations += iterations;
        if warm_up.seconds() >= warm_up_seconds {
            break;
        }
        iterations = iterations.saturating_mul(2);
    }
    // The timed estimate, or the wall clock's when the routine timed below
    // the clock's resolution.
    let per_iteration = if total_nanos > 0.0 {
        total_nanos / total_iterations as f64
    } else {
        warm_up.seconds() * 1e9 / total_iterations as f64
    };
    let measurement_nanos = config.measurement_time.as_secs_f64() * 1e9;
    let sample_count = config.sample_size;
    let iterations_per_sample =
        ((measurement_nanos / sample_count as f64 / per_iteration).ceil() as u64).max(1);
    let expected_seconds = iterations_per_sample as f64 * per_iteration * sample_count as f64 / 1e9;
    let warning = (expected_seconds > config.measurement_time.as_secs_f64() * 1.5).then(|| {
        format!(
            "{sample_count} samples cannot finish in {:.1} s; they will take about {expected_seconds:.1} s",
            config.measurement_time.as_secs_f64()
        )
    });
    let samples = (0..sample_count)
        .map(|_| time_iterations(routine, iterations_per_sample) / iterations_per_sample as f64)
        .collect();
    Sampled {
        samples,
        iterations_per_sample,
        warning,
    }
}

/// A caught benchmark panic's message.
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    crate::harness::panic_message(payload)
        .unwrap_or("the benchmark panicked")
        .to_owned()
}

/// `nanos` with four significant digits and a unit from ps to s.
pub fn format_time(nanos: f64) -> String {
    let (value, unit) = if nanos < 1.0 {
        (nanos * 1e3, "ps")
    } else if nanos < 1e3 {
        (nanos, "ns")
    } else if nanos < 1e6 {
        (nanos / 1e3, "µs")
    } else if nanos < 1e9 {
        (nanos / 1e6, "ms")
    } else {
        (nanos / 1e9, "s")
    };
    format!("{} {unit}", significant(value))
}

/// `per_second` of `throughput`'s kind with four significant digits and a
/// binary (bytes) or decimal (elements) prefix.
pub fn format_rate(throughput: Throughput, per_second: f64) -> String {
    let (value, unit) = match throughput {
        Throughput::Bytes(_) => {
            let units = ["B/s", "KiB/s", "MiB/s", "GiB/s", "TiB/s"];
            scaled(per_second, 1024.0, &units)
        }
        Throughput::Elements(_) => {
            let units = ["elem/s", "Kelem/s", "Melem/s", "Gelem/s", "Telem/s"];
            scaled(per_second, 1000.0, &units)
        }
    };
    format!("{} {unit}", significant(value))
}

fn scaled(mut value: f64, step: f64, units: &[&'static str]) -> (f64, &'static str) {
    let mut unit = 0;
    while value >= step && unit + 1 < units.len() {
        value /= step;
        unit += 1;
    }
    (value, units[unit])
}

/// Four significant digits (at least one decimal place below 1000).
fn significant(value: f64) -> String {
    let decimals = if value >= 1000.0 {
        0
    } else if value >= 100.0 {
        1
    } else if value >= 10.0 {
        2
    } else {
        3
    };
    format!("{value:.decimals$}")
}

fn signed_percent(fraction: f64) -> String {
    format!("{:+.4}%", fraction * 100.0)
}

/// The console report of one measured benchmark.
fn write_summary(out: &mut String, id: &str, estimates: &Estimates, change: Option<&Change>) {
    const PAD: usize = 24;
    let _ = writeln!(
        out,
        "{id:<PAD$} time:   [{} {} {}]  mad: {}  ({} samples × {} iterations)",
        format_time(estimates.ci_low),
        format_time(estimates.median),
        format_time(estimates.ci_high),
        format_time(estimates.mad),
        estimates.samples.len(),
        estimates.iterations_per_sample,
    );
    if let Some(throughput) = estimates.throughput {
        let amount = match throughput {
            Throughput::Bytes(amount) | Throughput::Elements(amount) => amount as f64,
        };
        let rate = |nanos: f64| format_rate(throughput, amount / (nanos / 1e9));
        // A longer time is a lower rate: the interval's ends swap.
        let _ = writeln!(
            out,
            "{:PAD$} thrpt:  [{} {} {}]",
            "",
            rate(estimates.ci_high),
            rate(estimates.median),
            rate(estimates.ci_low),
        );
    }
    if let Some(change) = change {
        let _ = writeln!(
            out,
            "{:PAD$} change: [{} {} {}] against `{}`: {}",
            "",
            signed_percent(change.ci_low),
            signed_percent(change.point),
            signed_percent(change.ci_high),
            change.record,
            change.verdict.sentence(),
        );
    }
    let outliers = &estimates.outliers;
    if outliers.total() > 0 {
        let _ = writeln!(
            out,
            "{:PAD$} outliers: {} of {} ({} low severe, {} low mild, {} high mild, {} high severe)",
            "",
            outliers.total(),
            estimates.samples.len(),
            outliers.low_severe,
            outliers.low_mild,
            outliers.high_mild,
            outliers.high_severe,
        );
    }
}

/// The `main` of a bench target: parse the command line, run every group, and
/// report. `target_tmpdir` is the `CARGO_TARGET_TMPDIR` the target was
/// compiled with, which places the store; [`bench_main!`] passes it.
///
/// Exit status 0 when every selected benchmark ran and every record was read
/// and written, 101 otherwise (and for a refused command line). On wasm32 the
/// status is also reported to the test runner's host.
pub fn main(target_tmpdir: Option<&str>, groups: &[fn()]) -> ExitCode {
    let status = run_main(target_tmpdir, groups);
    platform::report_exit(status);
    ExitCode::from(status)
}

fn run_main(target_tmpdir: Option<&str>, groups: &[fn()]) -> u8 {
    let arguments = match Arguments::from_env() {
        Ok(arguments) => arguments,
        Err(error) => {
            platform::print_error(&format!(
                "error: {error}\nRun with --help for the options.\n"
            ));
            return ERROR_EXIT_CODE;
        }
    };
    if arguments.mode == Mode::Help {
        platform::print_raw(USAGE);
        return 0;
    }
    #[cfg(target_arch = "wasm32")]
    let _ = target_tmpdir;
    let mode = arguments.mode;
    let installed = GLOBAL.set(Global {
        arguments,
        #[cfg(not(target_arch = "wasm32"))]
        store: Store::resolve(platform::env_var(store::HOME_VARIABLE), target_tmpdir),
    });
    assert!(installed.is_ok(), "bench::main runs once per process");
    for group in groups {
        group();
    }
    let outcomes = std::mem::take(&mut global_tally().outcomes);
    let failures: Vec<&Outcome> = outcomes
        .iter()
        .filter(|outcome| outcome.failure.is_some())
        .collect();
    let count = outcomes.len();
    let verb = match mode {
        Mode::Test => "tested",
        Mode::List => "listed",
        Mode::Benchmark | Mode::Help => "measured",
    };
    let noun = if count == 1 {
        "benchmark"
    } else {
        "benchmarks"
    };
    let mut summary = format!("\n{count} {noun} {verb}; {} failed\n", failures.len());
    if !failures.is_empty() {
        summary.push_str("failures:\n");
        for outcome in &failures {
            let _ = writeln!(
                summary,
                "    {}: {}",
                outcome.id,
                outcome.failure.as_deref().unwrap_or_default()
            );
        }
    }
    platform::print_raw(&summary);
    if failures.is_empty() {
        0
    } else {
        ERROR_EXIT_CODE
    }
}

/// Declare a benchmark group: a `pub fn` that runs each target over one
/// [`Bench`] configured from the command line.
///
/// `bench_group!(name, target_a, target_b)` uses the default timing;
/// `bench_group! { name = name; config = <Bench expression>; targets = target_a, target_b }`
/// starts from the given configuration. Each target is a
/// `fn(&mut Bench)`.
#[macro_export]
macro_rules! bench_group {
    (name = $name:ident; config = $config:expr; targets = $($target:path),+ $(,)?) => {
        /// A benchmark group.
        pub fn $name() {
            let mut bench = $crate::bench::Bench::configure_from_args($config);
            $( $target(&mut bench); )+
        }
    };
    ($name:ident, $($target:path),+ $(,)?) => {
        $crate::bench_group! {
            name = $name;
            config = $crate::bench::Bench::default();
            targets = $($target),+
        }
    };
}

/// Write a bench target's `main` over its groups: `bench_main!(group_a,
/// group_b)` runs each group (any `fn()`, usually one [`bench_group!`]
/// declared) through [`bench::main`](crate::bench::main).
#[macro_export]
macro_rules! bench_main {
    ($($group:path),+ $(,)?) => {
        fn main() -> ::std::process::ExitCode {
            $crate::bench::main(
                ::core::option_env!("CARGO_TARGET_TMPDIR"),
                &[$($group),+],
            )
        }
    };
}
