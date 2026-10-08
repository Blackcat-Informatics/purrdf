// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Process-isolated governed row-workspace measurement. This ordinary integration
//! binary has one test and its own allocator, so the normal workspace gate cannot
//! overlap its whole-process measurement window with unrelated test cases.

#![cfg(not(target_arch = "wasm32"))]

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_core::{ResourceDimension, SparqlRequest, SparqlResult, TermValue, TrippedGovernor};
use purrdf_sparql_eval::{
    Arity, ExtensionEnv, GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions,
    UserFunctionRegistry, Volatility,
};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn high_cost_dataset(rows: usize) -> std::sync::Arc<purrdf_core::RdfDataset> {
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://example.org/v");
    for index in 0..rows {
        let s = b.intern_iri(&format!("http://example.org/s{index}"));
        let value = b.intern_literal(RdfLiteral::typed(
            format!("1{index:0width$}", width = 3_125 - 1),
            purrdf_xsd::datatype::XSD_INTEGER,
        ));
        b.push_quad(s, p, value, None);
    }
    b.freeze().expect("numeric fixture")
}

fn high_cost_query() -> String {
    let mut power = "?v".to_owned();
    for _ in 0..5 {
        power = format!("({power} * {power})");
    }
    // IF keeps the visit and expensive arm in one row evaluation, including when
    // the planner separates ordinary AND conjuncts into distinct FILTERs.
    format!(
        "SELECT ?s WHERE {{ ?s <http://example.org/v> ?v FILTER(IF(<http://example.org/tick>(?v), STRLEN(STR({power})) > 0, false)) }}"
    )
}

fn high_cost_calibration(
    request: SparqlRequest<'_>,
    options: QueryOptions<'_>,
) -> (u64, u64, purrdf_alloc_probe::Measurement) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("pool");
    let single = high_cost_dataset(1);
    let window = WholeProcessWindow::open();
    let one = pool
        .install(|| {
            NativeSparqlEngine::new().query_governed(
                &single,
                request,
                options,
                &QueryGovernors::METERED,
            )
        })
        .expect("single heavy row");
    let allocation = window.close();
    let GovernedOutcome::Complete {
        result: SparqlResult::Solutions { rows, .. },
        ..
    } = &one
    else {
        panic!("complete single-row calibration: {one:?}");
    };
    assert_eq!(rows, &[vec![Some(TermValue::iri("http://example.org/s0"))]]);
    let row_scratch = one.evidence().consumed_in(ResourceDimension::ScratchBytes);
    assert!(
        row_scratch > 128 << 10,
        "the row must be expensive: {row_scratch}"
    );
    let pair = high_cost_dataset(2);
    let two = pool
        .install(|| {
            NativeSparqlEngine::new().query_governed(
                &pair,
                request,
                options,
                &QueryGovernors::METERED,
            )
        })
        .expect("two heavy rows");
    let GovernedOutcome::Complete {
        result: SparqlResult::Solutions { rows, .. },
        ..
    } = &two
    else {
        panic!("complete two-row calibration: {two:?}");
    };
    assert_eq!(
        rows,
        &[
            vec![Some(TermValue::iri("http://example.org/s0"))],
            vec![Some(TermValue::iri("http://example.org/s1"))],
        ]
    );
    let per_row = two.evidence().consumed_in(ResourceDimension::ScratchBytes) - row_scratch;
    assert!(per_row > 128 << 10, "each distinct row has expensive mints");
    (row_scratch, per_row, allocation)
}

fn high_cost_input_peak(ds: &std::sync::Arc<purrdf_core::RdfDataset>, rows: usize) -> u64 {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("pool");
    let window = WholeProcessWindow::open();
    let input = pool
        .install(|| {
            NativeSparqlEngine::new().query_governed(
                ds,
                SparqlRequest {
                    query: "SELECT ?s WHERE { ?s <http://example.org/v> ?v }",
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED,
            )
        })
        .expect("input relation control");
    let allocation = window.close();
    let GovernedOutcome::Complete {
        result: SparqlResult::Solutions { rows: actual, .. },
        ..
    } = &input
    else {
        panic!("complete input control: {input:?}");
    };
    assert_eq!(actual.len(), rows);
    eprintln!(
        "input control: rows={rows} peak={} retained={} requested={} allocations={}; fixture/pool excluded",
        allocation.peak_working_bytes,
        allocation.retained_bytes,
        allocation.requested_bytes,
        allocation.allocations
    );
    u64::try_from(allocation.peak_working_bytes).expect("nonnegative input peak")
}

/// This binary's sole test owns each process-wide allocator window automatically. Its
/// data and pools are built before each window; retained output and transient workspace
/// are reported separately. The stable callback counts source visits and returns true.
#[test]
fn forked_high_cost_rows_stop_with_bounded_workspace_and_work() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

    const ROWS: usize = 16_384;
    const EX: &str = "http://example.org/";
    let visits = Arc::new(AtomicUsize::new(0));
    // Fixed fixture state is outside the allocation windows. Source visits prove that
    // prefix continuation does not evaluate a speculative source row twice.
    let source_visits: Arc<Vec<_>> = Arc::new((0..ROWS).map(|_| AtomicU8::new(0)).collect());
    let mut functions = UserFunctionRegistry::default();
    let counted = Arc::clone(&visits);
    let recorded = Arc::clone(&source_visits);
    functions.register_native(
        format!("{EX}tick"),
        Arity::Exact(1),
        Volatility::Stable,
        Arc::new(move |args: &[&TermValue]| {
            counted.fetch_add(1, Ordering::Relaxed);
            if let TermValue::Literal { lexical_form, .. } = args[0] {
                let index: usize = lexical_form[lexical_form.len() - 5..]
                    .parse()
                    .expect("fixture row ordinal");
                recorded[index].fetch_add(1, Ordering::Relaxed);
            }
            Ok(Some(TermValue::boolean(true)))
        }),
    );
    let engine = NativeSparqlEngine::new();
    let functions = engine
        .bind_functions(functions, ExtensionEnv::empty())
        .expect("stable function");
    let options = QueryOptions::new().with_functions(&functions);
    let query = high_cost_query();
    let request = SparqlRequest {
        query: &query,
        base_iri: None,
        substitutions: &[],
    };
    let (row_scratch, per_row, row_allocation) = high_cost_calibration(request, options);
    // The five squared integers and their STR have payload lengths 6,249 + 12,497 +
    // 24,993 + 49,985 + 99,969 + 99,969, plus five integer (40 + 32) and one string
    // (39 + 32) term charges. Every distinct fixture ordinal has these same lengths.
    assert_eq!(per_row, 294_093);
    let ds = high_cost_dataset(ROWS);
    let input_peak = high_cost_input_peak(&ds, ROWS);
    for ceiling in [128, 8_192, row_scratch * 3] {
        let governors = QueryGovernors::UNBOUNDED.with_max_scratch_bytes(ceiling);
        let mut reference = None;
        for threads in [1, 4, 32] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("pool");
            visits.store(0, Ordering::Relaxed);
            for count in source_visits.iter() {
                count.store(0, Ordering::Relaxed);
            }
            let window = WholeProcessWindow::open();
            let outcome = pool
                .install(|| {
                    NativeSparqlEngine::new().query_governed(&ds, request, options, &governors)
                })
                .expect("bounded heavy rows");
            let allocation = window.close();
            let visited = visits.load(Ordering::Relaxed);
            let observation = (
                outcome.tripped(),
                ResourceDimension::ALL.map(|dimension| outcome.evidence().consumed_in(dimension)),
                format!("{outcome:?}"),
            );
            assert!(
                matches!(&outcome, GovernedOutcome::BudgetExhausted(exhausted)
                if matches!(exhausted.tripped, TrippedGovernor::Budget { dimension: ResourceDimension::ScratchBytes, .. })),
                "{outcome:?}"
            );
            if let Some(expected) = &reference {
                assert_eq!(&observation, expected, "{threads} workers at {ceiling}");
            } else {
                reference = Some(observation);
            }
            // The independent BGP/project control bounds input materialization and its
            // small IRI output. Single-row evaluation still initializes, defers and
            // commits a loop worker, so its peak includes the fork-only ledger/mints.
            let row_peak =
                u64::try_from(row_allocation.peak_working_bytes).expect("nonnegative peak");
            let peak_bound = input_peak + ceiling + (threads as u64) * row_peak;
            // Fork work has one shared headroom and one in-flight row per worker.
            // Conservative stopping can leave a short committed prefix: its ordered
            // continuation separately has at most one retained headroom of completed
            // rows and a first refusal. The row_checkpoint phase witness checks these
            // two allowances separately, using actual retained and pending growth.
            // Here the largest multiply claims pending growth and admits 355,456
            // transient bytes after 94,012 bytes of this row's smaller powers, before
            // minting its final product/STR. Those 449,468 bytes exceed its full
            // 294,093 retained increment, so every completed continuation row fits
            // the retained ceiling, even though its final tail is charged later.
            let admitted = usize::try_from(ceiling / per_row).expect("small ceiling");
            let fork_bound = admitted + threads;
            let continuation_bound = admitted + 1;
            let work_bound = fork_bound + continuation_bound;
            eprintln!(
                "bounded heavy rows: workers={threads} scratch={ceiling} visits={visited}/{work_bound} peak={}/{} retained={} fixture/pool excluded; row scratch={row_scratch} unique row increment={per_row} row peak={row_peak} input peak={input_peak}",
                allocation.peak_working_bytes, peak_bound, allocation.retained_bytes,
            );
            assert!(
                source_visits
                    .iter()
                    .all(|count| count.load(Ordering::Relaxed) <= 1),
                "a source row must not be evaluated twice"
            );
            assert!(
                u64::try_from(allocation.peak_working_bytes).expect("nonnegative peak")
                    <= peak_bound,
                "bounded workspace: threads={threads} ceiling={ceiling} row={row_allocation:?} actual={allocation:?} bound={peak_bound}"
            );
            assert!(
                visited <= work_bound,
                "bounded row work: {visited} > {work_bound} at {threads} workers, ceiling {ceiling}"
            );
            assert!(
                allocation.retained_bytes < 1 << 20,
                "refused output is separate: {allocation:?}"
            );
        }
    }
}
