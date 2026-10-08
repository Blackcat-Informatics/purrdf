// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One fixed-work collector and production call boundary for both allocator modes.

use std::{ffi::OsString, hint::black_box, path::Path, sync::Arc};

use purrdf_alloc_probe::{Measurement, WholeProcessWindow};
use purrdf_core::{
    GovernorEvidence, RdfDataset, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    GovernedOutcome, NativeSparqlEngine, PreparedQuery, QueryGovernors, QueryOptions,
};
use purrdf_testkit::bench::counter::{
    CountContext, CountReport, CountUnit, CounterCommand, CounterControl, collect_instructions,
    save_report,
};

use super::governor_workloads::{
    AnswerGuard, GROUPED, LOOP_QUERIES, LOOP_ROWS, loop_dataset, loop_governors, numeric_dataset,
};

const SAMPLES: usize = 10;
const CALLS: u64 = 5;
const WORKER_ITERATIONS: usize = 1_000_000;

/// Executable-level allocator boundary; these modes live in separate binaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountMode {
    /// Normal-allocator retired user-mode instructions.
    Instructions,
    /// Separate whole-process allocation windows.
    Allocations,
}

// Retain the actual production return carriers. Boxing the governed outcome
// would introduce an allocation into the query window solely for this tool.
#[allow(clippy::large_enum_variant)]
enum Product {
    Plain(SparqlResult),
    Governed(GovernedOutcome),
}

impl Product {
    fn result(&self) -> &SparqlResult {
        match self {
            Self::Plain(result) | Self::Governed(GovernedOutcome::Complete { result, .. }) => {
                result
            }
            Self::Governed(other) => panic!("the count workload must complete: {other:?}"),
        }
    }

    fn receipt(&self) -> Option<GovernorEvidence> {
        match self {
            Self::Plain(_) => None,
            Self::Governed(outcome) => {
                let evidence = outcome.evidence();
                assert!(evidence.tripped.is_none());
                assert!(
                    evidence.expression_errors.is_empty(),
                    "the frozen workload has no expression errors"
                );
                assert_eq!(
                    evidence.silenced,
                    [] as [purrdf_core::SilencedInvocation; 0]
                );
                Some(evidence.clone())
            }
        }
    }
}

struct Workload {
    dataset: Arc<RdfDataset>,
    engine: NativeSparqlEngine,
    prepared: Option<Arc<PreparedQuery>>,
    governors: Option<QueryGovernors>,
    guard: AnswerGuard,
    fixture: String,
}

impl Workload {
    fn new(case: &str, lane: &str) -> Self {
        let engine = NativeSparqlEngine::new();
        let (dataset, prepared, guard, text) = match case {
            "numeric" => (numeric_dataset(), None, AnswerGuard::grouped(), GROUPED),
            "filter" | "bind" => {
                let text = LOOP_QUERIES
                    .iter()
                    .find(|(shape, _, _)| *shape == case)
                    .expect("row query")
                    .1;
                let prepared = engine
                    .prepare_query(text, None)
                    .expect("prepare original row workload");
                (
                    loop_dataset(LOOP_ROWS),
                    Some(prepared),
                    AnswerGuard::row_loop(case),
                    text,
                )
            }
            _ => panic!("unknown count fixture {case}"),
        };
        let governors = if case == "numeric" {
            match lane {
                "ungoverned" => None,
                "fuel" => Some(QueryGovernors::UNBOUNDED.with_fuel(1_000_000_000)),
                "scratch" => Some(QueryGovernors::UNBOUNDED.with_max_scratch_bytes(1 << 32)),
                "metered" => Some(QueryGovernors::METERED),
                _ => panic!("unknown numeric lane {lane}"),
            }
        } else {
            loop_governors()
                .into_iter()
                .find(|(name, _)| *name == lane)
                .expect("original row governor")
                .1
        };
        let mut hash = purrdf_hash::blake3::Hasher::new();
        hash.update(text.as_bytes());
        for quad in dataset.quad_refs() {
            hash.update(format!("{quad:?}\n").as_bytes());
        }
        hash.update(guard.signature().as_bytes());
        let fixture = purrdf_hash::hex::encode(hash.finalize().as_bytes());
        Self {
            dataset,
            engine,
            prepared,
            governors,
            guard,
            fixture,
        }
    }

    fn run(&self) -> Product {
        match (&self.prepared, &self.governors) {
            (None, None) => Product::Plain(
                self.engine
                    .query(
                        &self.dataset,
                        SparqlRequest {
                            query: GROUPED,
                            base_iri: None,
                            substitutions: &[],
                        },
                    )
                    .expect("original public numeric query"),
            ),
            (None, Some(governors)) => Product::Governed(
                self.engine
                    .query_governed(
                        &self.dataset,
                        SparqlRequest {
                            query: GROUPED,
                            base_iri: None,
                            substitutions: &[],
                        },
                        QueryOptions::EMPTY,
                        governors,
                    )
                    .expect("original public governed numeric query"),
            ),
            (Some(prepared), None) => Product::Plain(
                self.engine
                    .query_prepared(&self.dataset, prepared, &[], QueryOptions::EMPTY)
                    .expect("original public prepared row query"),
            ),
            (Some(prepared), Some(governors)) => Product::Governed(
                self.engine
                    .query_prepared_governed_view(
                        &*self.dataset,
                        prepared,
                        &[] as &[(String, TermValue)],
                        QueryOptions::EMPTY,
                        governors,
                    )
                    .expect("original public governed prepared row query"),
            ),
        }
    }
}

fn pool(threads: usize) -> rayon::ThreadPool {
    assert!(threads > 0, "worker count must be positive");
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("count worker pool");
    let mut initialized = pool.broadcast(|context| context.index());
    initialized.sort_unstable();
    assert_eq!(
        initialized,
        (0..threads).collect::<Vec<_>>(),
        "all workers exist before enable"
    );
    pool
}

fn sample(mode: CountMode, case: &str, lane: &str, threads: usize) {
    let pool = pool(threads);
    // The engine's private plan cache is caller-local. Keep fixture construction,
    // warm-up, measured calls and guards in one installed caller closure rather
    // than making the production engine Sync for a benchmark.
    pool.install(|| {
        let workload = (case != "empty").then(|| Workload::new(case, lane));
        let mut expected_receipt = None;
        if let Some(workload) = &workload {
            for _ in 0..3 {
                let product = workload.run();
                workload.guard.check(product.result());
                let receipt = product.receipt();
                if expected_receipt.is_none() {
                    expected_receipt = receipt;
                } else {
                    assert_eq!(receipt, expected_receipt, "warm receipts are deterministic");
                }
            }
        }
        match mode {
            CountMode::Instructions => {
                let mut results = Vec::with_capacity(usize::try_from(CALLS).unwrap());
                let mut control = CounterControl::required().expect("admitted instruction counter");
                control
                    .command(CounterCommand::Enable)
                    .expect("enable instructions");
                for _ in 0..CALLS {
                    if let Some(workload) = &workload {
                        results.push(black_box(workload.run()));
                    } else {
                        results.push(black_box(Product::Plain(SparqlResult::Boolean(false))));
                    }
                }
                control
                    .command(CounterCommand::Disable)
                    .expect("disable instructions before validation/drop");
                assert_eq!(results.len(), usize::try_from(CALLS).unwrap());
                if let Some(workload) = &workload {
                    for result in &results {
                        workload.guard.check(result.result());
                        assert_eq!(
                            result.receipt(),
                            expected_receipt,
                            "every measured receipt agrees"
                        );
                    }
                    report_guard(workload, expected_receipt.as_ref());
                } else {
                    assert!(
                        results
                            .iter()
                            .all(|result| matches!(result.result(), SparqlResult::Boolean(false)))
                    );
                    println!(
                        "EMPTY iterations={CALLS} retained_dummy_products={CALLS} query_calls=0"
                    );
                }
            }
            CountMode::Allocations => {
                let workload = workload.expect("allocation mode requires a production fixture");
                let window = WholeProcessWindow::open();
                let product = black_box(workload.run());
                let measurement = window.close();
                workload.guard.check(product.result());
                assert_eq!(
                    product.receipt(),
                    expected_receipt,
                    "allocation result receipt agrees"
                );
                println!(
                    "ALLOC {} {} {} {} {}",
                    measurement.allocations,
                    measurement.requested_bytes,
                    measurement.retained_bytes,
                    measurement.peak_working_bytes,
                    workload.guard.signature()
                );
                report_guard(&workload, expected_receipt.as_ref());
            }
        }
    });
}

fn report_guard(workload: &Workload, receipt: Option<&GovernorEvidence>) {
    println!(
        "GUARD {} fixture={} receipt={receipt:?}",
        workload.guard.signature(),
        workload.fixture
    );
}

fn worker_work() -> u64 {
    let mut value = 1_u64;
    for _ in 0..WORKER_ITERATIONS {
        value = black_box(value);
        value = purrdf_hash::mix::lcg64_next(&mut value, 1);
    }
    value
}

fn worker_sample(threads: usize, target: usize) {
    use std::sync::{Condvar, Mutex};

    #[derive(Default)]
    struct Gate {
        arrived: usize,
        released: bool,
    }

    // A failed native command must wake the other callbacks before Rayon joins
    // the panic. Cleanup recovers the lock solely to release those waiters.
    struct Release<'a>(&'a (Mutex<Gate>, Condvar));
    impl Drop for Release<'_> {
        fn drop(&mut self) {
            self.0
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .released = true;
            self.0.1.notify_all();
        }
    }

    let pool = pool(threads);
    assert!(target < threads);
    assert!(
        rayon::current_thread_index().is_none(),
        "the external caller must block on broadcast, not steal worker tasks"
    );
    let caller = std::thread::current().id();
    let expected = worker_work();
    let gate = (Mutex::new(Gate::default()), Condvar::new());
    let output = pool.broadcast(|context| {
        // Declare cleanup before the lock guard, so a panic drops the guard
        // before the selected callback tries to release the parked callbacks.
        let selected = context.index() == target;
        let release = selected.then(|| Release(&gate));
        let (state, _) = gate
            .1
            .wait_timeout_while(
                {
                    let mut arrival = gate.0.lock().expect("worker coverage rendezvous");
                    arrival.arrived += 1;
                    gate.1.notify_all();
                    arrival
                },
                std::time::Duration::from_secs(10),
                |state| {
                    if selected {
                        state.arrived != threads
                    } else {
                        !state.released
                    }
                },
            )
            .expect("bounded selected/parked worker rendezvous");
        if !selected {
            assert!(
                state.released,
                "selected worker did not release the witness"
            );
            drop(state);
            return None;
        }
        assert_eq!(
            state.arrived, threads,
            "worker coverage rendezvous incomplete"
        );
        drop(state);
        assert_ne!(std::thread::current().id(), caller);
        let mut control = CounterControl::required().expect("worker coverage counter");
        control
            .command(CounterCommand::Enable)
            .expect("enable selected worker witness");
        let value = worker_work();
        control
            .command(CounterCommand::Disable)
            .expect("disable before releasing other workers or the caller");
        drop(release);
        Some((context.index(), value))
    });
    let results: Vec<_> = output.into_iter().flatten().collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, target);
    assert_eq!(
        results[0].1, expected,
        "the selected existing worker completes every step"
    );
    println!(
        "WORKER target={target} existing_workers={threads} iterations={WORKER_ITERATIONS} signature={}",
        results[0].1
    );
}

fn context(case: &str, threads: usize, mode: CountMode, unit: CountUnit) -> CountContext {
    let (fixture, seam) = match case {
        "numeric" => (
            Workload::new(case, "ungoverned").fixture,
            "public-query-materialized",
        ),
        "filter" | "bind" => (
            Workload::new(case, "ungoverned").fixture,
            "public-prepared-query-materialized",
        ),
        "empty" => (
            "empty-control-v1".to_owned(),
            "five-dummy-products-retained-zero-query-calls",
        ),
        _ => panic!("unknown report fixture {case}"),
    };
    CountContext {
        unit,
        work: if mode == CountMode::Instructions {
            CALLS
        } else {
            1
        },
        threads: u64::try_from(threads).unwrap(),
        fixture,
        boundary: if case == "empty" {
            seam.to_owned()
        } else {
            format!(
                "{seam}-{}-results-retained",
                if mode == CountMode::Instructions {
                    "five"
                } else {
                    "one"
                }
            )
        },
    }
}

fn allocation_sample(text: &str) -> Measurement {
    let records: Vec<_> = text
        .lines()
        .filter_map(|line| line.strip_prefix("ALLOC "))
        .collect();
    assert_eq!(records.len(), 1, "exactly one guarded allocation receipt");
    let fields: Vec<_> = records[0].split_whitespace().collect();
    assert_eq!(fields.len(), 5);
    Measurement {
        allocations: fields[0].parse().expect("allocation calls"),
        requested_bytes: fields[1].parse().expect("requested bytes"),
        retained_bytes: fields[2].parse().expect("retained bytes"),
        peak_working_bytes: fields[3].parse().expect("peak bytes"),
    }
}

fn collect(mode: CountMode, case: &str, lane: &str, threads: usize, record: &str) {
    let executable = std::env::current_exe().expect("count executable");
    let home =
        std::env::var_os("PURRDF_BENCH_HOME").expect("PURRDF_BENCH_HOME must place count evidence");
    if mode == CountMode::Instructions {
        let proof = Path::new(&home)
            .join("worker-coverage")
            .join(record)
            .join(threads.to_string())
            .join("complete.txt");
        let identity = executable_identity(&executable);
        let expected = format!(
            "executable={identity}\nthreads={threads}\nworkers={threads}\nevent=instructions:u\nscale=none\nexact_coverage=all\n"
        );
        assert_eq!(
            std::fs::read_to_string(&proof).expect(
                "run --coverage THREADS RECORD on this exact executable before collecting queries"
            ),
            expected,
            "worker proof must bind this executable and all initialized workers"
        );
    }
    let components = ["governor_counts", case, lane];
    let raw = Path::new(&home)
        .join("raw-counts")
        .join(record)
        .join(format!("{case}-{lane}-{threads}-{}", std::process::id()));
    std::fs::create_dir_all(&raw).expect("raw count directory");
    let args: Vec<OsString> = [
        "--sample".to_owned(),
        case.to_owned(),
        lane.to_owned(),
        threads.to_string(),
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    let mut instructions = Vec::new();
    let mut allocations = Vec::new();
    let mut empty = Vec::new();
    if mode == CountMode::Instructions && case != "empty" {
        let empty_args: Vec<OsString> = [
            "--sample".to_owned(),
            "empty".to_owned(),
            "ungoverned".to_owned(),
            threads.to_string(),
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        for sample in 0..SAMPLES {
            let count = collect_instructions(
                &executable,
                &empty_args,
                &raw.join(format!("empty-{sample}")),
            )
            .expect("complete empty control sample");
            empty.push(i64::try_from(count.instructions).expect("empty count range"));
        }
    }
    for sample in 0..SAMPLES {
        let directory = raw.join(format!("sample-{sample}"));
        if mode == CountMode::Instructions {
            let count = collect_instructions(&executable, &args, &directory)
                .expect("complete fresh instruction sample");
            instructions.push(
                i64::try_from(count.instructions).expect("count fits exact signed report range"),
            );
        } else {
            std::fs::create_dir(&directory).expect("new allocation sample directory");
            let output = std::process::Command::new(&executable)
                .args(&args)
                .output()
                .expect("allocation child process");
            std::fs::write(directory.join("workload.stdout"), &output.stdout)
                .expect("allocation output receipt");
            std::fs::write(directory.join("workload.stderr"), &output.stderr)
                .expect("allocation error receipt");
            assert!(
                output.status.success(),
                "allocation child failed: {}; inspect {}",
                output.status,
                directory.display()
            );
            allocations.push(allocation_sample(
                std::str::from_utf8(&output.stdout).expect("allocation receipt UTF-8"),
            ));
        }
    }
    if !empty.is_empty() {
        let lowest = *instructions.iter().min().unwrap();
        let highest_empty = *empty.iter().max().unwrap();
        assert!(
            highest_empty <= lowest / 100,
            "empty control exceeds one percent of lowest workload sample: empty={highest_empty}, workload={lowest}"
        );
        let report = CountReport::from_samples(
            context("empty", threads, mode, CountUnit::Instructions),
            empty,
            478,
        )
        .expect("empty control report");
        let thread_component = threads.to_string();
        let path = save_report(
            &report,
            &[
                components[0],
                case,
                lane,
                &thread_component,
                "empty-instructions",
            ],
            record,
        )
        .expect("empty report store");
        println!(
            "EMPTY-CONTROL raw={:?} maximum={highest_empty} lowest_workload={lowest} limit_percent=1 report={}",
            report.raw_samples(),
            path.display()
        );
    }
    let units = if mode == CountMode::Instructions {
        vec![(CountUnit::Instructions, instructions)]
    } else {
        vec![
            (
                CountUnit::AllocationCalls,
                allocations
                    .iter()
                    .map(|value| i64::try_from(value.allocations).unwrap())
                    .collect(),
            ),
            (
                CountUnit::RequestedBytes,
                allocations
                    .iter()
                    .map(|value| i64::try_from(value.requested_bytes).unwrap())
                    .collect(),
            ),
            (
                CountUnit::RetainedBytes,
                allocations
                    .iter()
                    .map(|value| value.retained_bytes)
                    .collect(),
            ),
            (
                CountUnit::PeakBytes,
                allocations
                    .iter()
                    .map(|value| value.peak_working_bytes)
                    .collect(),
            ),
        ]
    };
    for (unit, samples) in units {
        let report = CountReport::from_samples(context(case, threads, mode, unit), samples, 478)
            .expect("fixed-work report");
        let thread_component = threads.to_string();
        let components = [
            components[0],
            components[1],
            components[2],
            &thread_component,
            unit.name(),
        ];
        let path = save_report(&report, &components, record).expect("count record store");
        println!(
            "{case}/{lane}/{threads} unit={} median={} mad={} ci=[{},{}] raw={:?} report={}",
            unit.name(),
            report.estimates().median,
            report.estimates().mad,
            report.estimates().ci_low,
            report.estimates().ci_high,
            report.raw_samples(),
            path.display()
        );
    }
}

fn coverage(threads: usize, record: &str) {
    let executable = std::env::current_exe().expect("coverage executable");
    let home =
        std::env::var_os("PURRDF_BENCH_HOME").expect("PURRDF_BENCH_HOME places coverage receipts");
    let raw = Path::new(&home)
        .join("worker-coverage")
        .join(record)
        .join(format!("{threads}-{}", std::process::id()));
    std::fs::create_dir_all(&raw).expect("coverage receipt directory");
    for target in 0..threads {
        let args: Vec<OsString> = [
            "--worker".to_owned(),
            threads.to_string(),
            target.to_string(),
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        let count = collect_instructions(&executable, &args, &raw.join(format!("worker-{target}")))
            .expect("exact worker counter coverage");
        assert!(
            count.instructions >= u64::try_from(WORKER_ITERATIONS).unwrap(),
            "pre-existing worker {target} was not counted: {} instructions",
            count.instructions
        );
        println!(
            "WORKER-COVERAGE threads={threads} target={target} instructions={} enabled={} running={} displayed={} receipt={}",
            count.instructions,
            count.enabled,
            count.running,
            count.displayed_running_percent,
            raw.display()
        );
    }
    let proof = Path::new(&home)
        .join("worker-coverage")
        .join(record)
        .join(threads.to_string());
    std::fs::create_dir_all(&proof).expect("coverage proof directory");
    let identity = executable_identity(&executable);
    std::fs::write(proof.join("complete.txt"), format!("executable={identity}\nthreads={threads}\nworkers={threads}\nevent=instructions:u\nscale=none\nexact_coverage=all\n")).expect("complete artifact-bound worker proof");
}

fn executable_identity(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("actual counter executable identity");
    purrdf_hash::hex::encode(purrdf_hash::blake3::hash(&bytes).as_bytes())
}

fn allocation_coverage(threads: usize) {
    let pool = pool(threads);
    let window = WholeProcessWindow::open();
    let buffers =
        pool.broadcast(|context| vec![u8::try_from(context.index() % 256).unwrap(); 4096]);
    let measurement = window.close();
    assert_eq!(buffers.len(), threads);
    for (index, buffer) in buffers.iter().enumerate() {
        assert_eq!(buffer.len(), 4096);
        assert!(buffer.iter().all(|byte| usize::from(*byte) == index % 256));
    }
    let payload = i64::try_from(4096 * threads).unwrap();
    assert!(measurement.allocations >= u64::try_from(threads).unwrap());
    assert!(measurement.requested_bytes >= u64::try_from(payload).unwrap());
    assert!(measurement.retained_bytes >= payload);
    assert!(measurement.peak_working_bytes >= payload);
    println!(
        "ALLOCATION-COVERAGE workers={threads} payload={payload} calls={} requested={} retained={} peak={}",
        measurement.allocations,
        measurement.requested_bytes,
        measurement.retained_bytes,
        measurement.peak_working_bytes
    );
}

/// Fixed-work CLI. `--test` executes full guards without claiming any counts.
pub fn entry(mode: CountMode) {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--bench")
        .collect();
    match args.first().map(String::as_str) {
        None | Some("--test") => {
            assert!(args.len() <= 1, "--test takes no additional arguments");
            let pool = pool(4);
            pool.install(|| {
                for (case, lanes) in [
                    ("numeric", &["ungoverned", "fuel", "scratch", "metered"][..]),
                    (
                        "filter",
                        &["ungoverned", "stop_signal", "fuel_and_stop_signal"][..],
                    ),
                    (
                        "bind",
                        &["ungoverned", "stop_signal", "fuel_and_stop_signal"][..],
                    ),
                ] {
                    for lane in lanes {
                        let workload = Workload::new(case, lane);
                        let product = workload.run();
                        workload.guard.check(product.result());
                        println!("FUNCTIONAL {case}/{lane} {}", workload.guard.signature());
                    }
                }
            });
        }
        Some("--sample") => {
            assert_eq!(args.len(), 4);
            sample(
                mode,
                &args[1],
                &args[2],
                args[3].parse().expect("worker count"),
            );
        }
        Some("--worker") => {
            assert_eq!(mode, CountMode::Instructions);
            assert_eq!(args.len(), 3);
            worker_sample(args[1].parse().unwrap(), args[2].parse().unwrap());
        }
        Some("--coverage") => {
            assert_eq!(mode, CountMode::Instructions);
            assert_eq!(args.len(), 3);
            coverage(args[1].parse().unwrap(), &args[2]);
        }
        Some("--allocation-coverage") => {
            assert_eq!(mode, CountMode::Allocations);
            assert_eq!(args.len(), 2);
            allocation_coverage(args[1].parse().unwrap());
        }
        Some("--collect") => {
            assert_eq!(args.len(), 5);
            collect(mode, &args[1], &args[2], args[3].parse().unwrap(), &args[4]);
        }
        _ => panic!(
            "expected --test, --sample CASE LANE THREADS, --collect CASE LANE THREADS RECORD, --coverage THREADS RECORD, or --worker THREADS TARGET"
        ),
    }
}
