// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The evaluator's observable trace over every query-evaluation case, pinned.
//!
//! For every `QueryEvaluationTest` under `suite/` — the W3C SPARQL 1.1/1.2 manifests and
//! the first-party ones — evaluated exactly as the conformance harness evaluates it (the
//! same dataset, `SERVICE` source, engine and extension environment, through the
//! `run::query_eval_*` helpers), one committed golden records:
//!
//! 1. **the answer**, in the canonical form the conformance comparer judges it in: a
//!    solution sequence as the RDFC-1.0 canonical N-Quads of its comparer encoding
//!    (ordered only when the query's top-level `ORDER BY` makes order observable), a
//!    graph as its canonical N-Quads, a boolean as itself — or the diagnostic code of a
//!    refusal;
//! 2. **the receipt**: the consumption vector of the same query run under
//!    [`QueryGovernors::METERED`], and the per-node charge ledger of its explanation —
//!    every algebra node in pre-order, with the fuel it charged at each charge point, the
//!    rows it committed and the largest bag it held;
//! 3. **a fuel sweep**: the query run under a fuel ceiling of every value from `0` to
//!    the metered total (and at least to `64`). A query whose total exceeds
//!    [`EXHAUSTIVE_SWEEP_MAX`] is swept over `0..=64`, every power of two from 128 below
//!    the total, and the total less one and the total itself; no case in the suite comes
//!    near that bound today. Each point records which governor tripped and the
//!    consumption it had reached when it did, the certificate the partial rows carry
//!    (class, positional prefix, barrier), how many rows crossed, and a digest of those
//!    rows in the comparer's canonical form. Adjacent points that observed the same thing
//!    are folded into one line.
//!
//! The sweep is what pins charge **order**: a ceiling of `k` trips at the first charge
//! that takes consumption past `k`, so sweeping every unit reads the whole charge
//! sequence one charge at a time, together with the rows each prefix of it had
//! committed. An evaluator that
//! charged the same total in a different order, or truncated at a different row, moves
//! this golden.
//!
//! # Updates
//!
//! Every `UpdateEvaluationTest` is traced into a second golden, `update.trace`, applied
//! exactly as the conformance run applies it (the same pre-state, extension environment
//! and offline `LOAD` source, through `run::update_eval_*`). Each records the post-state
//! graph store — default and named graphs — as its RDFC-1.0 canonical N-Quads, the form
//! the comparer judges an update in (or the refusal's code); the consumption of the
//! request under `update_governed` with [`QueryGovernors::METERED`]; and the governed
//! request under the same fuel sweep. A tripped update applies nothing, so each tripped
//! point also records that the store it left is the pre-state.
//!
//! # Nondeterminism
//!
//! Blank nodes are relabelled away by the canonical forms. `NOW()` reads the wall clock
//! and `RAND()`/`UUID()`/`STRUUID()` an entropy seed, so a case whose query calls one
//! (the same textual test the SPARQL golden capture uses) is recorded by result SHAPE
//! only — form, variables and row count — and its sweep carries row counts but no
//! digest.
//!
//! # Regenerating
//!
//! ```text
//! cargo test -p purrdf-sparql-conformance --test evaluator_trace -- --ignored regenerate_evaluator_trace
//! cargo test -p purrdf-sparql-conformance --test evaluator_trace -- --ignored regenerate_update_trace
//! ```

mod support;

use purrdf::viz::stable_hash_hex;
use support::suite_root;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use purrdf_core::{
    GovernorEvidence, RdfDataset, ResourceDimension, SparqlRequest, SparqlResult, TrippedGovernor,
};
use purrdf_sparql_conformance::compare::canonical_solutions;
use purrdf_sparql_conformance::manifest::{SparqlTestCase, TestKind};
use purrdf_sparql_conformance::run::{
    query_eval_dataset, query_eval_engine, query_eval_env, query_eval_is_ordered, update_eval_env,
    update_eval_options,
};
use purrdf_sparql_eval::governor::NodeCharges;
use purrdf_sparql_eval::{
    ChargePoint, ExtensionEnv, GovernedOutcome, InProcessServiceResolver, PartialAnswers,
    QueryGovernors, QueryOptions,
};

/// The every-unit prefix every sweep covers, whatever the query's total.
const DENSE_SWEEP_MAX: u64 = 64;

/// The largest metered total swept at every unit; above it the sweep turns geometric.
const EXHAUSTIVE_SWEEP_MAX: u64 = 4096;

/// A floor on the number of update-evaluation cases traced.
const MIN_UPDATE_CASES: usize = 100;

/// A floor on the number of query-evaluation cases traced, so a corpus walk that stopped
/// finding cases cannot leave the trace passing over nothing.
const MIN_CASES: usize = 300;

/// Every `manifest.ttl` under `root`, sorted.
fn discover_manifests(root: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(root).unwrap_or_else(|error| panic!("read {}: {error}", root.display()));
    for entry in entries {
        let path = entry.expect("suite directory entry").path();
        if path.is_dir() {
            discover_manifests(&path, out);
        } else if path.file_name().and_then(|name| name.to_str()) == Some("manifest.ttl") {
            out.push(path);
        }
    }
}

/// Whether a query reads the wall clock or the entropy seed.
fn is_volatile(query_text: &str) -> bool {
    let lower = query_text.to_ascii_lowercase();
    ["now(", "rand(", "uuid("]
        .iter()
        .any(|call| lower.contains(call))
}

/// Everything one case's evaluations share.
struct Prepared<'a> {
    case: &'a SparqlTestCase,
    query: String,
    dataset: Arc<RdfDataset>,
    remote: InProcessServiceResolver,
    env: ExtensionEnv,
    ordered: bool,
    volatile: bool,
}

impl Prepared<'_> {
    fn request(&self) -> SparqlRequest<'_> {
        SparqlRequest {
            query: &self.query,
            base_iri: Some(&self.case.base),
            substitutions: &[],
        }
    }

    fn options(&self) -> QueryOptions<'_> {
        QueryOptions::new()
            .with_env(&self.env)
            .with_remote(Some(&self.remote))
    }

    /// One governed evaluation, on a fresh engine so no run shares a plan with another.
    fn governed(
        &self,
        governors: &QueryGovernors,
    ) -> Result<GovernedOutcome, purrdf_core::RdfDiagnostic> {
        query_eval_engine().query_governed(&self.dataset, self.request(), self.options(), governors)
    }
}

/// The canonical rendering of a result, one line per canonical N-Quad under a `|`
/// margin. Shape only when `volatile`.
fn render_result(result: &SparqlResult, ordered: bool, volatile: bool, out: &mut String) {
    match result {
        SparqlResult::Boolean(value) => {
            if volatile {
                out.push_str("boolean (volatile)\n");
            } else {
                writeln!(out, "boolean {value}").expect("write");
            }
        }
        SparqlResult::Graph(graph) => {
            let nquads = purrdf_core::canonicalize(graph).nquads;
            writeln!(out, "graph quads={}", nquads.lines().count()).expect("write");
            if !volatile {
                margin(&nquads, out);
            }
        }
        SparqlResult::Solutions {
            variables,
            rows,
            aux,
        } => {
            writeln!(
                out,
                "solutions vars={variables:?} rows={} ordered={ordered}",
                rows.len()
            )
            .expect("write");
            if volatile {
                return;
            }
            match canonical_solutions(variables, rows, ordered) {
                Ok(nquads) => margin(&nquads, out),
                Err(_) => out.push_str("unencodable\n"),
            }
            if aux.quad_count() > 0 {
                out.push_str("aux\n");
                margin(&purrdf_core::canonicalize(aux).nquads, out);
            }
        }
    }
}

fn margin(text: &str, out: &mut String) {
    for line in text.lines() {
        out.push_str("| ");
        out.push_str(line);
        out.push('\n');
    }
}

/// Every nonzero consumption, in [`ResourceDimension::ALL`] order.
///
/// A volatile case's scratch bytes are left out: they count the bytes of the values the
/// evaluation minted, and a random double or a clock reading has no fixed length.
fn render_consumption(evidence: &GovernorEvidence, volatile: bool) -> String {
    let mut parts = Vec::new();
    for dimension in ResourceDimension::ALL {
        if volatile && dimension == ResourceDimension::ScratchBytes {
            continue;
        }
        let consumed = evidence.consumed_in(dimension);
        if consumed > 0 {
            parts.push(format!("{}={consumed}", dimension.label()));
        }
    }
    if parts.is_empty() {
        "nothing".to_owned()
    } else {
        parts.join(" ")
    }
}

fn render_ledger(ledger: &[NodeCharges], out: &mut String) {
    for node in ledger {
        write!(
            out,
            "node {} depth={} {} rows={} cells={}",
            node.ordinal, node.depth, node.label, node.rows, node.cells
        )
        .expect("write");
        for point in ChargePoint::ALL {
            let fuel = node.fuel_at(point);
            if fuel > 0 {
                write!(out, " {point}={fuel}").expect("write");
            }
        }
        out.push('\n');
    }
}

/// One sweep point's observation, without the ceiling itself, so adjacent points that
/// observed the same thing render identically.
fn render_point(prepared: &Prepared<'_>, fuel: u64) -> String {
    let outcome = match prepared.governed(&QueryGovernors::UNBOUNDED.with_fuel(fuel)) {
        Ok(outcome) => outcome,
        Err(error) => return format!("error {}", error.code),
    };
    let mut out = String::new();
    match &outcome {
        GovernedOutcome::Complete { result, .. } => {
            out.push_str("complete");
            write_rows(result, prepared, &mut out);
        }
        GovernedOutcome::BudgetExhausted(exhausted) => {
            write!(out, "tripped {}", render_tripped(exhausted.tripped)).expect("write");
            match &exhausted.partial {
                PartialAnswers::Certain(partial) | PartialAnswers::AtMost(partial) => {
                    let class = if exhausted.partial.is_certain() {
                        "certain"
                    } else {
                        "at-most"
                    };
                    write!(out, " {class} prefix={}", partial.is_positional_prefix())
                        .expect("write");
                    write_rows(partial.result(), prepared, &mut out);
                }
                PartialAnswers::Unknown(barrier) => {
                    write!(out, " unknown barrier={}", barrier.operator()).expect("write");
                }
            }
        }
    }
    write!(
        out,
        " | {}",
        render_consumption(outcome.evidence(), prepared.volatile)
    )
    .expect("write");
    out
}

/// The governor that tripped and the number it tripped on, without the ceiling (which
/// is the sweep point itself).
fn render_tripped(tripped: TrippedGovernor) -> String {
    match tripped {
        TrippedGovernor::Budget { consumed, .. } => {
            format!("{} consumed={consumed}", tripped.label())
        }
        TrippedGovernor::Refused { estimate, .. } => {
            format!("{} estimate={estimate}", tripped.label())
        }
        other => other.label().to_owned(),
    }
}

/// The row count and — unless the case is volatile — the digest of the canonical form.
fn write_rows(result: &SparqlResult, prepared: &Prepared<'_>, out: &mut String) {
    let count = match result {
        SparqlResult::Boolean(_) => 1,
        SparqlResult::Graph(graph) => graph.quad_count(),
        SparqlResult::Solutions { rows, .. } => rows.len(),
    };
    write!(out, " rows={count}").expect("write");
    if !prepared.volatile {
        let mut canonical = String::new();
        render_result(result, prepared.ordered, false, &mut canonical);
        write!(out, " digest={}", stable_hash_hex(&canonical)).expect("write");
    }
}

/// The ceilings the sweep evaluates at, for a query whose metered fuel is `total`.
fn sweep_points(total: u64) -> Vec<u64> {
    if total <= EXHAUSTIVE_SWEEP_MAX {
        return (0..=total.max(DENSE_SWEEP_MAX)).collect();
    }
    let mut points: BTreeSet<u64> = (0..=DENSE_SWEEP_MAX).collect();
    let mut geometric = DENSE_SWEEP_MAX.saturating_mul(2);
    while geometric < total {
        points.insert(geometric);
        geometric = geometric.saturating_mul(2);
    }
    points.insert(total.saturating_sub(1));
    points.insert(total);
    points.into_iter().collect()
}

/// Append one sweep point to the current run of identical observations, flushing the run
/// when the observation changes.
fn fold_point(
    run: &mut Option<(u64, u64, usize, String)>,
    fuel: u64,
    observed: String,
    out: &mut String,
) {
    match run {
        Some((_, last, count, previous)) if *previous == observed => {
            *last = fuel;
            *count += 1;
        }
        _ => {
            flush_run(run, out);
            *run = Some((fuel, fuel, 1, observed));
        }
    }
}

fn flush_run(run: &mut Option<(u64, u64, usize, String)>, out: &mut String) {
    if let Some((first, last, count, observed)) = run.take() {
        if count == 1 {
            writeln!(out, "fuel {first} {observed}").expect("write");
        } else {
            writeln!(out, "fuel {first}..={last} x{count} {observed}").expect("write");
        }
    }
}

fn render_sweep(prepared: &Prepared<'_>, total: u64, out: &mut String) {
    let mut run = None;
    for fuel in sweep_points(total) {
        fold_point(&mut run, fuel, render_point(prepared, fuel), out);
    }
    flush_run(&mut run, out);
}

fn render_case(case: &SparqlTestCase, header: &str, out: &mut String) {
    writeln!(out, "== {header}").expect("write");
    let query = std::fs::read_to_string(&case.query)
        .unwrap_or_else(|error| panic!("read query {}: {error}", case.query.display()));
    let Ok(dataset) = query_eval_dataset(case, &query) else {
        out.push_str("dataset-unloadable\n");
        return;
    };
    let Ok(remote) = purrdf_sparql_conformance::service::build(case) else {
        out.push_str("service-unloadable\n");
        return;
    };
    let Ok(env) = query_eval_env(case) else {
        out.push_str("environment-unreadable\n");
        return;
    };
    let prepared = Prepared {
        case,
        ordered: query_eval_is_ordered(&query),
        volatile: is_volatile(&query),
        query,
        dataset,
        remote,
        env,
    };
    if prepared.volatile {
        out.push_str("volatile\n");
    }

    // 1. The answer, through the ungoverned entry the conformance run uses.
    match query_eval_engine().query_with_source(
        &prepared.dataset,
        prepared.request(),
        &prepared.remote,
        QueryOptions::new().with_env(&prepared.env),
    ) {
        Ok(result) => render_result(&result, prepared.ordered, prepared.volatile, out),
        Err(error) => writeln!(out, "error {}", error.code).expect("write"),
    }

    // 2. The receipt: the metered consumption and the per-node ledger.
    let total = match prepared.governed(&QueryGovernors::METERED) {
        Ok(outcome) => {
            let evidence = outcome.evidence();
            writeln!(
                out,
                "metered {} {}",
                outcome
                    .tripped()
                    .map_or("complete", |tripped| tripped.label()),
                render_consumption(evidence, prepared.volatile)
            )
            .expect("write");
            evidence.consumed_in(ResourceDimension::Fuel)
        }
        Err(error) => {
            writeln!(out, "metered error {}", error.code).expect("write");
            return;
        }
    };
    match query_eval_engine().explain_query_with_options(
        &prepared.dataset,
        &prepared.query,
        Some(&case.base),
        prepared.options(),
    ) {
        Ok(explanation) => {
            writeln!(
                out,
                "explain {}",
                render_consumption(explanation.evidence(), prepared.volatile)
            )
            .expect("write");
            render_ledger(explanation.ledger(), out);
        }
        Err(error) => writeln!(out, "explain error {}", error.code).expect("write"),
    }

    // 3. The fuel sweep.
    render_sweep(&prepared, total, out);
}

/// The canonical rendering of a graph store — default and named graphs — in the form
/// the conformance comparer compares update post-states in: RDFC-1.0 canonical N-Quads.
fn render_store(store: &RdfDataset, volatile: bool, out: &mut String) {
    let nquads = purrdf_core::canonicalize(store).nquads;
    writeln!(out, "store quads={}", nquads.lines().count()).expect("write");
    if !volatile {
        margin(&nquads, out);
    }
}

/// The store's quad count and — unless volatile — the digest of its canonical form.
fn store_digest(store: &RdfDataset, volatile: bool) -> String {
    let nquads = purrdf_core::canonicalize(store).nquads;
    let quads = nquads.lines().count();
    if volatile {
        format!("quads={quads}")
    } else {
        format!("quads={quads} digest={}", stable_hash_hex(&nquads))
    }
}

/// One `UpdateEvaluationTest`: the post-state store (or the refusal's code), the metered
/// governed application's consumption, and the governed application under the same fuel
/// sweep a query gets. A governed update that trips applies nothing, so a tripped point
/// records whether the store it left is the pre-state.
fn render_update_case(case: &SparqlTestCase, header: &str, out: &mut String) {
    writeln!(out, "== {header}").expect("write");
    let request_text = std::fs::read_to_string(&case.query)
        .unwrap_or_else(|error| panic!("read update {}: {error}", case.query.display()));
    let Ok(pre) =
        purrdf_sparql_conformance::run::build_dataset(&case.base, &case.data, &case.graph_data)
    else {
        out.push_str("dataset-unloadable\n");
        return;
    };
    let Ok(env) = update_eval_env(case) else {
        out.push_str("environment-unreadable\n");
        return;
    };
    let volatile = is_volatile(&request_text);
    if volatile {
        out.push_str("volatile\n");
    }
    let request = || SparqlRequest {
        query: &request_text,
        base_iri: Some(&case.base),
        substitutions: &[],
    };
    let pre_digest = store_digest(&pre, volatile);

    // 1. The post-state, through the ungoverned entry the conformance run uses.
    let mut store = Arc::clone(&pre);
    match purrdf_sparql_eval::NativeSparqlEngine::new().update_with_options(
        &mut store,
        request(),
        update_eval_options(&env),
    ) {
        Ok(()) => render_store(&store, volatile, out),
        Err(error) => writeln!(out, "error {}", error.code).expect("write"),
    }

    let governed = |governors: &QueryGovernors| {
        let mut store = Arc::clone(&pre);
        purrdf_sparql_eval::NativeSparqlEngine::new()
            .update_governed(&mut store, request(), update_eval_options(&env), governors)
            .map(|outcome| (outcome, store))
    };

    // 2. The metered consumption.
    let total = match governed(&QueryGovernors::METERED) {
        Ok((outcome, _)) => {
            writeln!(
                out,
                "metered {} {}",
                outcome
                    .tripped()
                    .map_or("applied", |tripped| tripped.label()),
                render_consumption(outcome.evidence(), volatile)
            )
            .expect("write");
            outcome.evidence().consumed_in(ResourceDimension::Fuel)
        }
        Err(error) => {
            writeln!(out, "metered error {}", error.code).expect("write");
            return;
        }
    };

    // 3. The fuel sweep.
    let mut run: Option<(u64, u64, usize, String)> = None;
    for fuel in sweep_points(total) {
        let observed = match governed(&QueryGovernors::UNBOUNDED.with_fuel(fuel)) {
            Err(error) => format!("error {}", error.code),
            Ok((outcome, store)) => {
                let after = store_digest(&store, volatile);
                let head = match outcome.tripped() {
                    None => format!("applied {after}"),
                    Some(tripped) if after == pre_digest => {
                        format!("tripped {} store=pre-state", render_tripped(tripped))
                    }
                    Some(tripped) => {
                        format!("tripped {} store=MOVED {after}", render_tripped(tripped))
                    }
                };
                format!(
                    "{head} | {}",
                    render_consumption(outcome.evidence(), volatile)
                )
            }
        };
        fold_point(&mut run, fuel, observed, out);
    }
    flush_run(&mut run, out);
}

/// Which cases a trace walks, and how it renders each.
#[derive(Clone, Copy)]
enum Corpus {
    /// Every `QueryEvaluationTest`.
    Query,
    /// Every `UpdateEvaluationTest`.
    Update,
}

impl Corpus {
    const fn admits(self, kind: TestKind) -> bool {
        match self {
            Self::Query => matches!(kind, TestKind::QueryEval),
            Self::Update => matches!(kind, TestKind::UpdateEval),
        }
    }

    const fn min_cases(self) -> usize {
        match self {
            Self::Query => MIN_CASES,
            Self::Update => MIN_UPDATE_CASES,
        }
    }

    fn golden_path(self) -> PathBuf {
        let name = match self {
            Self::Query => "suite.trace",
            Self::Update => "update.trace",
        };
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/goldens/evaluator-trace")
            .join(name)
    }

    const fn regenerate_test(self) -> &'static str {
        match self {
            Self::Query => "regenerate_evaluator_trace",
            Self::Update => "regenerate_update_trace",
        }
    }
}

fn render_trace(corpus: Corpus) -> String {
    let root = suite_root();
    let mut manifests = Vec::new();
    discover_manifests(&root, &mut manifests);
    manifests.sort();
    let mut out = String::new();
    let mut traced = 0_usize;
    let mut headers = BTreeSet::new();
    for manifest in &manifests {
        let relative = manifest
            .strip_prefix(&root)
            .expect("a manifest lies under suite/")
            .parent()
            .expect("a manifest has a directory")
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let cases = purrdf_sparql_conformance::manifest::load(manifest)
            .unwrap_or_else(|error| panic!("load {}: {error}", manifest.display()));
        for case in &cases {
            if !corpus.admits(case.kind) {
                continue;
            }
            traced += 1;
            let header = format!("{relative} <{}> {}", case.iri, case.name);
            assert!(
                headers.insert(header.clone()),
                "two cases share the trace header {header:?}, so a difference in one could be \
                 reported against the other"
            );
            match corpus {
                Corpus::Query => render_case(case, &header, &mut out),
                Corpus::Update => render_update_case(case, &header, &mut out),
            }
        }
    }
    assert!(
        traced >= corpus.min_cases(),
        "the evaluation corpus shrank: only {traced} cases were traced"
    );
    out
}

/// Each case's block, keyed by its header line.
fn blocks(trace: &str) -> Vec<(&str, String)> {
    let mut out: Vec<(&str, String)> = Vec::new();
    for line in trace.lines() {
        if let Some(header) = line.strip_prefix("== ") {
            out.push((header, String::new()));
        } else if let Some((_, body)) = out.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    out
}

/// The first line on which two blocks differ, for a failure message.
fn first_difference(golden: &str, actual: &str) -> String {
    let mut golden_lines = golden.lines();
    let mut actual_lines = actual.lines();
    loop {
        match (golden_lines.next(), actual_lines.next()) {
            (Some(g), Some(a)) if g == a => {}
            (g, a) => {
                return format!(
                    "    golden: {}\n    actual: {}",
                    g.unwrap_or("<end>"),
                    a.unwrap_or("<end>")
                );
            }
        }
    }
}

/// Compare a freshly rendered trace against its committed golden, reporting the first
/// cases whose block moved.
fn assert_matches_golden(corpus: Corpus) {
    let actual = render_trace(corpus);
    let path = corpus.golden_path();
    let regenerate = format!(
        "cargo test -p purrdf-sparql-conformance --test evaluator_trace -- --ignored {}",
        corpus.regenerate_test()
    );
    let golden = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "read {}: {error}; regenerate with `{regenerate}`",
            path.display()
        )
    });
    if actual == golden {
        return;
    }
    let golden_blocks = blocks(&golden);
    let actual_blocks = blocks(&actual);
    let mut report = String::new();
    let mut differing = 0_usize;
    let golden_map: std::collections::BTreeMap<&str, &String> =
        golden_blocks.iter().map(|(h, b)| (*h, b)).collect();
    let actual_map: std::collections::BTreeMap<&str, &String> =
        actual_blocks.iter().map(|(h, b)| (*h, b)).collect();
    for (header, body) in &golden_blocks {
        let moved = match actual_map.get(header) {
            None => Some("    missing now".to_owned()),
            Some(now) if *now != body => Some(first_difference(body, now)),
            Some(_) => None,
        };
        if let Some(moved) = moved {
            differing += 1;
            if differing <= 10 {
                writeln!(report, "  {header}\n{moved}").expect("write");
            }
        }
    }
    for (header, _) in &actual_blocks {
        if !golden_map.contains_key(header) {
            differing += 1;
            if differing <= 10 {
                writeln!(report, "  new case: {header}").expect("write");
            }
        }
    }
    panic!(
        "the evaluator's trace moved on {differing} case(s) (first ten shown):\n{report}\nif \
         the change is intended, regenerate with `{regenerate}` and review the diff"
    );
}

fn write_golden(corpus: Corpus) {
    let path = corpus.golden_path();
    std::fs::create_dir_all(path.parent().expect("the golden has a parent directory"))
        .expect("create the golden directory");
    std::fs::write(&path, render_trace(corpus)).expect("write the golden");
    println!("wrote {}", path.display());
}

#[test]
fn the_evaluator_trace_over_every_query_evaluation_case_matches_the_golden() {
    assert_matches_golden(Corpus::Query);
}

#[test]
fn the_evaluator_trace_over_every_update_evaluation_case_matches_the_golden() {
    assert_matches_golden(Corpus::Update);
}

/// Regeneration path for the query trace. Ignored by default because it WRITES the
/// committed golden.
#[test]
#[ignore = "regeneration path: writes the committed golden"]
fn regenerate_evaluator_trace() {
    write_golden(Corpus::Query);
}

/// Regeneration path for the update trace. Ignored by default because it WRITES the
/// committed golden.
#[test]
#[ignore = "regeneration path: writes the committed golden"]
fn regenerate_update_trace() {
    write_golden(Corpus::Update);
}
