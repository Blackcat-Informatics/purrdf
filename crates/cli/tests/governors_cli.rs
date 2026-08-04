// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end coverage of `query`'s execution governors, `--explain`, and the exit-3
//! contract, driving the BUILT `purrdf` binary (`env!("CARGO_BIN_EXE_purrdf")`) rather than
//! the library — so every assertion pins what a shell actually sees. All fixtures use
//! `example.org`.
//!
//! ## The three channels a tripped run writes, and why they are tested separately
//!
//! A governor trip is a SUCCESSFUL run stopped by the caller's own policy, so it is not an
//! error and does not exit 1. It writes:
//!
//! * **stdout** — the answers the run certified, in the requested `--results-format`, as a
//!   well-formed document of that format. Tested by parsing it back.
//! * **stderr** — the deterministic governor report: which governor stopped the run, what
//!   the rows in hand bound, and the whole consumption/ceiling vector.
//! * **exit 3** — the only thing a shell can test to learn that the document on stdout is a
//!   partial answer.
//!
//! Nothing about the trip appears on stdout, which is what lets `purrdf query … | jq` keep
//! working across a trip instead of choking on an interleaved diagnostic. The tests below
//! assert both halves of that: stdout parses, and stdout carries none of the report.
//!
//! ## RDF 1.2 is first class here
//!
//! The answer cap denominates each query form's own answer sequence, which for a graph form
//! is *output statements* — ordinary triples plus RDF 1.2 reifier bindings and annotation
//! triples. [`construct_answer_cap_counts_rdf12_statements_not_solution_rows`] is where that
//! is pinned, over an annotation-syntax template; [`a_reifier_query_certifies_its_partial_rows`]
//! governs a SELECT whose pattern matches through a triple term.

use std::process::{Command, Output};

/// The path to the built `purrdf` binary this integration test target links against.
const PURRDF: &str = env!("CARGO_BIN_EXE_purrdf");

/// Three `ex:knows` edges and one name, so an answer cap has something to cut.
const DATA_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice ex:knows ex:bob .\n",
    "ex:alice ex:knows ex:carol .\n",
    "ex:alice ex:knows ex:dave .\n",
    "ex:alice ex:name \"Alice\" .\n",
);

/// The same edges carrying RDF 1.2 annotations, so a governed query can match through a
/// triple term and a governed CONSTRUCT can mint reifiers.
const STAR_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice ex:knows ex:bob {| ex:certainty \"0.9\" |} .\n",
    "ex:alice ex:knows ex:carol {| ex:certainty \"0.4\" |} .\n",
);

/// The SELECT the governed lanes below answer: three rows, no ORDER BY, no modifier.
const KNOWS: &str = "SELECT ?o WHERE { ?s <http://example.org/knows> ?o }";

/// A SELECT whose pattern matches an RDF 1.2 reifier and the triple term it reifies.
const REIFIER_QUERY: &str = concat!(
    "SELECT ?o ?c WHERE { ",
    "?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
    "<<( ?s <http://example.org/knows> ?o )>> . ",
    "?r <http://example.org/certainty> ?c }",
);

/// Run `purrdf` with `args`, returning the captured [`Output`].
fn run(args: &[&str]) -> Output {
    Command::new(PURRDF)
        .args(args)
        .output()
        .expect("spawn purrdf")
}

/// stdout of an [`Output`] as a `String`.
fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("utf-8 stdout")
}

/// stderr of an [`Output`] as a `String`.
fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).expect("utf-8 stderr")
}

/// The exit code of an [`Output`].
fn code(out: &Output) -> i32 {
    out.status.code().expect("the process exited normally")
}

/// Write `contents` to `dir/name` and return the path as an owned string.
fn write_file(dir: &std::path::Path, name: &str, contents: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, contents).expect("write fixture");
    path.to_str().expect("utf-8 path").to_owned()
}

/// EVERY CEILING REACHES THE ENGINE'S LIMIT VECTOR, including the one nothing can charge.
///
/// The five numeric flags are passed together with distinct values and the run is tripped
/// on fuel; the report echoes the whole ceiling vector, so every flag is observed at the
/// place it would have to be in force. `--max-remote-requests` is checked exactly this way
/// and no other: this binary configures no federation source, so nothing charges that
/// dimension, and the honest evidence that the flag arrived is that the engine is enforcing
/// the number.
///
/// A dimension no flag named must read `unbounded` — a flag that quietly bounded its
/// neighbours would pass every trip test and still be wrong.
#[test]
fn every_governor_flag_reaches_the_engines_ceiling_vector() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);
    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--fuel",
        "1",
        "--max-answers",
        "11",
        "--max-intermediate-cells",
        "12",
        "--max-scratch-bytes",
        "13",
        "--max-remote-requests",
        "14",
        KNOWS,
    ]);
    assert_eq!(code(&out), 3, "stderr:\n{}", stderr(&out));
    let report = stderr(&out);
    for line in [
        "limit fuel 1",
        "limit answer-rows 11",
        "limit intermediate-cells 12",
        "limit scratch-bytes 13",
        "limit remote-requests 14",
    ] {
        assert!(
            report.contains(&format!("\n{line}\n")),
            "the report must echo `{line}`; got:\n{report}"
        );
    }

    // One flag alone bounds one dimension alone.
    let out = run(&["query", "--data", &ttl, "--fuel", "1", KNOWS]);
    let report = stderr(&out);
    assert!(report.contains("\nlimit fuel 1\n"), "{report}");
    for dimension in [
        "answer-rows",
        "intermediate-cells",
        "scratch-bytes",
        "remote-requests",
    ] {
        assert!(
            report.contains(&format!("\nlimit {dimension} unbounded\n")),
            "an unnamed dimension must stay unbounded; got:\n{report}"
        );
    }
}

/// EACH NUMERIC CEILING TRIPS ON ITS OWN DIMENSION, and the report names which.
///
/// The labels are the kernel's pinned discriminants, so a consumer may match on them. The
/// intermediate-cell ceiling is refused at ADMISSION rather than charged, which is a
/// different label on purpose: nothing ran, so nothing was consumed.
#[test]
fn each_numeric_ceiling_trips_under_its_own_governor() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);
    let scratch_query =
        "SELECT (CONCAT(STR(?o), \"!\") AS ?x) WHERE { ?s <http://example.org/knows> ?o }";
    for (flag, value, query, expected) in [
        ("--fuel", "0", KNOWS, "tripped fuel-exhausted"),
        ("--max-answers", "1", KNOWS, "tripped answer-cap-exhausted"),
        (
            "--max-intermediate-cells",
            "0",
            KNOWS,
            "tripped cardinality-admission-refused",
        ),
        (
            "--max-scratch-bytes",
            "0",
            scratch_query,
            "tripped scratch-exhausted",
        ),
    ] {
        let out = run(&["query", "--data", &ttl, flag, value, query]);
        assert_eq!(
            code(&out),
            3,
            "`{flag} {value}` must trip and exit 3; stderr:\n{}",
            stderr(&out)
        );
        assert!(
            stderr(&out).contains(&format!("\n{expected}\n")),
            "`{flag} {value}` must report `{expected}`; got:\n{}",
            stderr(&out)
        );
    }
}

/// `--deadline` REACHES EVALUATION as a stop signal, and its spelling is a real grammar.
///
/// A zero budget expires on the first poll, which the evaluator performs when it enters an
/// algebra node — so the trip is deterministic rather than a race, and the reported governor
/// is the stop cause rather than a ceiling. A budget no query could exhaust completes
/// normally, which is what proves the flag is not simply always tripping.
#[test]
fn a_deadline_stops_the_query_and_names_the_stop_cause() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);

    let expired = run(&["query", "--data", &ttl, "--deadline", "0ms", KNOWS]);
    assert_eq!(code(&expired), 3, "stderr:\n{}", stderr(&expired));
    assert!(
        stderr(&expired).contains("\ntripped deadline-exceeded\n"),
        "{}",
        stderr(&expired)
    );

    let ample = run(&["query", "--data", &ttl, "--deadline", "1m30s", KNOWS]);
    assert_eq!(
        code(&ample),
        0,
        "a budget no query can exhaust must complete; stderr:\n{}",
        stderr(&ample)
    );
    assert!(stdout(&ample).contains("http://example.org/dave"));
    assert!(
        stderr(&ample).is_empty(),
        "a complete run prints no governor report; got:\n{}",
        stderr(&ample)
    );
}

/// A UNITLESS DEADLINE IS REFUSED rather than assumed to be seconds, and the refusal teaches
/// the spelling. Exit 2, the usage code, because the command line is what is wrong.
#[test]
fn a_deadline_without_a_unit_is_a_usage_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);
    for spelling in ["30", "30x", "1m30"] {
        let out = run(&["query", "--data", &ttl, "--deadline", spelling, KNOWS]);
        assert_eq!(code(&out), 2, "`{spelling}` must be a usage error");
        assert!(
            stderr(&out).contains("`1m30s`"),
            "the refusal of `{spelling}` must teach the spelling; got:\n{}",
            stderr(&out)
        );
    }
}

/// A TRIP EXITS 3 AND STILL PRINTS THE ROWS IT CERTIFIED.
///
/// The certified rows are the true answer's first rows, in order (`positional-prefix true`),
/// so they are asserted against the ungoverned answer's own prefix rather than against a
/// hand-written expectation — a check that cannot pass by accident and cannot drift.
#[test]
fn a_trip_exits_3_and_prints_the_certified_partial_answers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);

    let whole = run(&["query", "--data", &ttl, "--results-format", "tsv", KNOWS]);
    assert_eq!(code(&whole), 0, "stderr:\n{}", stderr(&whole));
    let whole_body = stdout(&whole);
    let whole_lines: Vec<&str> = whole_body.lines().collect();
    assert_eq!(whole_lines.len(), 4, "header plus three rows: {whole_body}");

    let capped = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "tsv",
        "--max-answers",
        "1",
        KNOWS,
    ]);
    assert_eq!(code(&capped), 3, "stderr:\n{}", stderr(&capped));
    let capped_body = stdout(&capped);
    assert_eq!(
        capped_body.lines().collect::<Vec<_>>(),
        whole_lines[..2].to_vec(),
        "the certified rows must be the whole answer's first rows, in order"
    );

    let report = stderr(&capped);
    assert!(report.starts_with("purrdf-governor-report 1\n"), "{report}");
    assert!(report.contains("\noutcome budget-exhausted\n"), "{report}");
    assert!(
        report.contains("\ntripped answer-cap-exhausted\n"),
        "{report}"
    );
    assert!(
        report.contains("\ndetail answer-rows budget exceeded: consumed 2, limit 1\n"),
        "{report}"
    );
    assert!(
        report.contains("\nanswers certain\n") && report.contains("\npositional-prefix true\n"),
        "the rows are a certified lower bound and a positional prefix; got:\n{report}"
    );
}

/// THE PARTIAL ANSWER IS A WELL-FORMED DOCUMENT OF THE REQUESTED FORMAT.
///
/// This is the whole reason the governor report is on stderr. A caller piping SPARQL-Results
/// JSON or XML into a parser gets a document that parses across a trip; the partial status
/// is carried by the exit code and the report, never by an in-band marker that would corrupt
/// the stream (or require inventing a non-W3C extension to four serializations).
#[test]
fn a_partial_answer_does_not_corrupt_the_machine_readable_stream() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);
    let capped = |format: &str| {
        run(&[
            "query",
            "--data",
            &ttl,
            "--results-format",
            format,
            "--max-answers",
            "1",
            KNOWS,
        ])
    };

    let json = capped("json");
    assert_eq!(code(&json), 3, "stderr:\n{}", stderr(&json));
    let body = stdout(&json);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).expect("the truncated stream is still valid JSON");
    assert_eq!(
        parsed["results"]["bindings"]
            .as_array()
            .expect("bindings array")
            .len(),
        1,
        "exactly the certified row: {body}"
    );
    assert_eq!(
        parsed["head"]["vars"].as_array().expect("vars array").len(),
        1,
        "the header still describes the projection: {body}"
    );

    let xml = capped("xml");
    assert_eq!(code(&xml), 3);
    let body = stdout(&xml);
    assert!(body.starts_with("<?xml"), "{body}");
    assert!(
        body.trim_end().ends_with("</sparql>"),
        "the document is closed rather than cut off: {body}"
    );
    assert_eq!(
        body.matches("<result>").count(),
        1,
        "exactly the certified row: {body}"
    );

    // Whichever format was asked for, not one byte of the governor report is on stdout.
    for format in ["json", "xml", "csv", "tsv"] {
        let out = capped(format);
        assert!(
            !stdout(&out).contains("purrdf-governor-report"),
            "the report leaked into the {format} result stream:\n{}",
            stdout(&out)
        );
        assert!(
            stderr(&out).contains("purrdf-governor-report 1"),
            "the {format} run must still report the trip on stderr"
        );
    }
}

/// WHEN NO BOUND SURVIVES, NO ROW CROSSES — and the report names the operator that
/// withheld them.
///
/// An aggregate is not monotone in its input, so a truncated input bounds its output on
/// neither side. Printing a serialized empty result here would be an "there are no answers"
/// claim the run cannot make, so stdout stays empty and the actionable half — which operator
/// turned a partial result into no result — is on stderr.
#[test]
fn withheld_answers_print_no_rows_and_name_the_barrier() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);
    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--fuel",
        "3",
        "SELECT (COUNT(?o) AS ?n) WHERE { ?s <http://example.org/knows> ?o }",
    ]);
    assert_eq!(code(&out), 3, "stderr:\n{}", stderr(&out));
    assert!(
        stdout(&out).is_empty(),
        "rows that bound the answer on neither side must not be printed; got:\n{}",
        stdout(&out)
    );
    let report = stderr(&out);
    assert!(report.contains("\nanswers withheld\n"), "{report}");
    assert!(report.contains("\nbarrier Group\n"), "{report}");
    assert!(
        !report.contains("positional-prefix"),
        "there are no rows to be a prefix of anything: {report}"
    );
}

/// A GOVERNED RUN THAT COMPLETES IS INDISTINGUISHABLE FROM AN UNGOVERNED ONE, except that
/// it exits 0 with an empty stderr.
///
/// Falsifiable in the direction that matters: a governed lane that always reported a trip,
/// or that reordered or dropped a row, would fail here rather than only under a ceiling
/// nobody set.
#[test]
fn a_complete_governed_run_exits_0_and_answers_identically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);
    let ungoverned = run(&["query", "--data", &ttl, "--results-format", "json", KNOWS]);
    assert_eq!(code(&ungoverned), 0, "stderr:\n{}", stderr(&ungoverned));

    let governed = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "json",
        "--fuel",
        "1000000",
        "--max-answers",
        "1000",
        "--max-intermediate-cells",
        "1000000",
        "--max-scratch-bytes",
        "1000000",
        "--max-remote-requests",
        "1000",
        "--deadline",
        "2h",
        KNOWS,
    ]);
    assert_eq!(code(&governed), 0, "stderr:\n{}", stderr(&governed));
    assert_eq!(
        governed.stdout, ungoverned.stdout,
        "a ceiling nothing reaches must not change one byte of the answer"
    );
    assert!(
        stderr(&governed).is_empty(),
        "a complete run says nothing about governors; got:\n{}",
        stderr(&governed)
    );
}

/// AN UNGOVERNED INVOCATION IS UNCHANGED: exit 0, the answers on stdout, nothing on stderr,
/// and no governor vocabulary anywhere.
#[test]
fn an_ungoverned_invocation_is_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);
    let out = run(&["query", "--data", &ttl, "--results-format", "tsv", KNOWS]);
    assert_eq!(code(&out), 0, "stderr:\n{}", stderr(&out));
    assert!(stderr(&out).is_empty(), "stderr:\n{}", stderr(&out));
    let body = stdout(&out);
    assert_eq!(body.lines().count(), 4, "header plus three rows: {body}");
    for row in ["bob", "carol", "dave"] {
        assert!(body.contains(row), "{body}");
    }
    assert!(!body.contains("governor"), "{body}");
}

/// `--explain` RENDERS THE CHARGE LEDGER, and it is byte-identical across two runs.
///
/// Determinism is the load-bearing property — the rendering is what a frozen corpus can
/// pin — and it holds because node identity is positional, the per-node totals are sums, and
/// nothing in the ledger reads a clock. The section assertions are what stop a rendering
/// that is stably empty from passing a stability test.
#[test]
fn explain_renders_the_ledger_and_is_byte_identical_across_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);
    let query = "SELECT ?o WHERE { ?s <http://example.org/knows> ?o . \
                 ?s <http://example.org/name> ?n }";

    let first = run(&["query", "--data", &ttl, "--explain", query]);
    assert_eq!(code(&first), 0, "stderr:\n{}", stderr(&first));
    let second = run(&["query", "--data", &ttl, "--explain", query]);
    assert_eq!(
        first.stdout, second.stdout,
        "two explanations of the same query over the same data must be byte-identical"
    );

    let body = stdout(&first);
    assert!(
        body.starts_with("profile purrdf-sparql-governors v"),
        "the explanation names the schedule it was priced under: {body}"
    );
    assert!(body.contains("stop-poll-fuel "), "{body}");
    for section in [
        "\nschedule\n",
        "\nledger\n",
        "\njoin-orders\n",
        "\nconsumed\n",
    ] {
        assert!(
            body.contains(section),
            "the explanation must carry the `{}` section; got:\n{body}",
            section.trim()
        );
    }
    // A charge line, a per-node line with the planner's prediction beside the count that
    // materialized, and the whole-execution consumption the ledger decomposes.
    assert!(body.contains("  algebra-node-entry\t"), "{body}");
    assert!(
        body.contains("estimated-rows=") && body.contains("actual-rows="),
        "the planner's error is what an EXPLAIN is for: {body}"
    );
    assert!(body.contains("\n  fuel\t"), "{body}");
    // The answers are replaced, not accompanied: an EXPLAIN returns a plan, not rows.
    assert!(
        !body.contains("http://example.org/bob"),
        "--explain prints the explanation INSTEAD of the answers: {body}"
    );
}

/// `--explain` REFUSES WHAT IT CANNOT HONOR — and that is now the WHOLE list.
///
/// Each refusal is a usage error (exit 2) naming the flag to drop, because the alternative
/// is a ceiling an operator believes is in force and that nothing enforces — the most
/// dangerous shape a silent no-op can take.
///
/// `--entailment` beside a governor flag used to be the third refusal and is deliberately
/// absent: the entailment-aware query lane takes governors now, so the combination is
/// exercised as a WORKING one by the tests below rather than pinned as a refusal here.
#[test]
fn unenforceable_flag_combinations_are_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);

    let explain_and_ceiling = run(&["query", "--data", &ttl, "--explain", "--fuel", "100", KNOWS]);
    assert_eq!(code(&explain_and_ceiling), 2);
    let message = stderr(&explain_and_ceiling);
    assert!(message.contains("--fuel"), "{message}");
    assert!(message.contains("never enforced"), "{message}");

    let explain_and_entailment = run(&[
        "query",
        "--data",
        &ttl,
        "--explain",
        "--entailment",
        "rdfs",
        KNOWS,
    ]);
    assert_eq!(code(&explain_and_entailment), 2);
    assert!(
        stderr(&explain_and_entailment).contains("--entailment"),
        "{}",
        stderr(&explain_and_entailment)
    );
}

/// `--entailment REGIME --fuel N` TRIPS OVER THE CLOSURE AND EXITS 3.
///
/// The combination used to exit 2 by name. It now runs: the RDFS closure is materialized,
/// the query is evaluated over it under the ceiling, and the ceiling is reached — so the
/// contract a shell tests is the ordinary governed one (report on stderr, exit 3), reached
/// through a lane that previously refused to start.
///
/// `--fuel 1` is enough to trip because the closure is large: RDFS asserts the axiomatic
/// triples and re-types every term, so a scan over it costs far more than one charge.
#[test]
fn an_entailment_query_under_a_fuel_ceiling_trips_and_exits_three() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "rdfs",
        "--fuel",
        "1",
        KNOWS,
    ]);
    assert_eq!(
        code(&out),
        3,
        "a fuel ceiling over an entailed closure must TRIP, not be refused and not be \
         ignored; stderr:\n{}",
        stderr(&out)
    );
    let report = stderr(&out);
    assert!(report.contains("purrdf-governor-report 1"), "{report}");
    assert!(report.contains("tripped fuel-exhausted"), "{report}");
    assert!(report.contains("limit fuel 1"), "{report}");
    // The trip report is on stderr and nothing of it is interleaved into the document.
    assert!(!stdout(&out).contains("purrdf-governor-report"), "{report}");
}

/// The SAME query WITHOUT a ceiling still answers over the closure, so the trip above is a
/// governor doing its job rather than the entailment lane having broken.
#[test]
fn an_ungoverned_entailment_query_is_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "rdfs",
        "--results-format",
        "tsv",
        KNOWS,
    ]);
    assert_eq!(code(&out), 0, "stderr:\n{}", stderr(&out));
    let body = stdout(&out);
    assert_eq!(body.lines().count(), 4, "header plus three rows: {body}");

    // …and a ceiling above the query's cost is not a trip either: the governed lane over a
    // closure completes exactly like the ungoverned one when nothing is reached.
    let governed = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "rdfs",
        "--results-format",
        "tsv",
        "--max-answers",
        "100",
        KNOWS,
    ]);
    assert_eq!(code(&governed), 0, "stderr:\n{}", stderr(&governed));
    assert_eq!(
        stdout(&governed),
        body,
        "an unreached ceiling must not change a byte of the answer"
    );
}

/// AN ANSWER CAP REACHES THE EVALUATION OVER THE CLOSURE and cuts it there.
///
/// The cap is 1 over a query with three answers, so the run trips, prints one certified row,
/// and exits 3 — the ceiling is enforced against the entailed closure rather than against
/// the raw view or against nothing.
#[test]
fn an_answer_cap_cuts_a_query_over_an_entailed_closure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "rdfs",
        "--results-format",
        "tsv",
        "--max-answers",
        "1",
        KNOWS,
    ]);
    assert_eq!(code(&out), 3, "stderr:\n{}", stderr(&out));
    let report = stderr(&out);
    assert!(report.contains("tripped answer-cap-exhausted"), "{report}");
    assert!(report.contains("answers certain"), "{report}");
    let body = stdout(&out);
    assert_eq!(
        body.lines().count(),
        2,
        "header plus the one certified row: {body}"
    );
}

/// A DEADLINE THAT HAS ALREADY EXPIRED STOPS THE CLOSURE ITSELF, and claims nothing.
///
/// `--deadline 0ms` is expired the moment it is built — before the first fixpoint round — so
/// the run stops in phase one, where there is no query result to truncate. The report says
/// exactly that: `deadline-exceeded`, `answers withheld`, and a barrier naming the closure
/// rather than an algebra operator. stdout is EMPTY, because an empty result set there would
/// be the claim "this query has no answers" and this run never asked it.
///
/// This is the half of the contract a stop signal buys that a charge schedule could not: a
/// wall deadline over an entailment-regime query is honest even when the closure is the
/// expensive half.
#[test]
fn an_expired_deadline_stops_the_closure_and_claims_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);

    let out = run(&[
        "query",
        "--data",
        &ttl,
        "--entailment",
        "rdfs",
        "--deadline",
        "0ms",
        KNOWS,
    ]);
    assert_eq!(code(&out), 3, "stderr:\n{}", stderr(&out));
    let report = stderr(&out);
    assert!(report.contains("purrdf-governor-report 1"), "{report}");
    assert!(report.contains("tripped deadline-exceeded"), "{report}");
    assert!(report.contains("answers withheld"), "{report}");
    assert!(report.contains("barrier entailment-closure"), "{report}");
    assert_eq!(
        stdout(&out),
        "",
        "a run that computed no closure must claim no answers"
    );
}

/// A GOVERNED QUERY THAT MATCHES THROUGH AN RDF 1.2 TRIPLE TERM certifies its partial rows
/// exactly like any other.
///
/// The pattern binds a reifier to the triple term it reifies and reads the annotation off
/// it, so the governed lane is exercised over the RDF 1.2 statement layer rather than only
/// over base triples.
#[test]
fn a_reifier_query_certifies_its_partial_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "star.ttl", STAR_TTL);

    let whole = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "tsv",
        REIFIER_QUERY,
    ]);
    assert_eq!(code(&whole), 0, "stderr:\n{}", stderr(&whole));
    let whole_body = stdout(&whole);
    let whole_lines: Vec<&str> = whole_body.lines().collect();
    assert_eq!(whole_lines.len(), 3, "header plus two rows: {whole_body}");
    assert!(whole_body.contains("\"0.9\""), "{whole_body}");

    let capped = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "tsv",
        "--max-answers",
        "1",
        REIFIER_QUERY,
    ]);
    assert_eq!(code(&capped), 3, "stderr:\n{}", stderr(&capped));
    assert_eq!(
        stdout(&capped).lines().collect::<Vec<_>>(),
        whole_lines[..2].to_vec(),
        "the certified rows are the whole answer's first rows"
    );
    assert!(
        stderr(&capped).contains("\ntripped answer-cap-exhausted\n"),
        "{}",
        stderr(&capped)
    );
}

/// THE ANSWER CAP COUNTS RDF 1.2 STATEMENTS FOR A GRAPH FORM, not solution rows.
///
/// The template mints one base triple, one reifier binding and one annotation triple per
/// solution, so a cap of 2 over a two-solution query cuts the graph mid-way. A cap that
/// counted solution rows would have let all six statements through, which is precisely the
/// governor that governs nothing this denomination exists to prevent.
#[test]
fn construct_answer_cap_counts_rdf12_statements_not_solution_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "star.ttl", STAR_TTL);
    let construct = "CONSTRUCT { ?s <http://example.org/friend> ?o \
                     {| <http://example.org/via> \"star\" |} } \
                     WHERE { ?s <http://example.org/knows> ?o }";

    let whole = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "ntriples",
        construct,
    ]);
    assert_eq!(code(&whole), 0, "stderr:\n{}", stderr(&whole));
    let whole_body = stdout(&whole);
    assert_eq!(
        whole_body.lines().count(),
        6,
        "two base triples, two reifier bindings, two annotations: {whole_body}"
    );
    assert_eq!(
        whole_body
            .matches("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies")
            .count(),
        2,
        "{whole_body}"
    );

    let capped = run(&[
        "query",
        "--data",
        &ttl,
        "--results-format",
        "ntriples",
        "--max-answers",
        "2",
        construct,
    ]);
    assert_eq!(code(&capped), 3, "stderr:\n{}", stderr(&capped));
    let capped_body = stdout(&capped);
    assert_eq!(
        capped_body.lines().count(),
        2,
        "the cap denominates OUTPUT STATEMENTS: {capped_body}"
    );
    assert!(
        !capped_body.contains("reifies"),
        "the statements past the cap must not be emitted: {capped_body}"
    );
    assert!(
        stderr(&capped).contains("\ndetail answer-rows budget exceeded"),
        "{}",
        stderr(&capped)
    );
}

/// A GOVERNED QUERY OVER A PACK IS THE SAME QUERY, zero-copy and all.
///
/// The governed lane runs over whichever concrete view the source resolved to, so a pack is
/// still mmap'd and queried without materialization — the trip, the certified rows and the
/// report are byte-identical to the text source's.
#[test]
fn a_governed_query_over_a_pack_trips_identically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(dir, "data.ttl", DATA_TTL);
    let pack = write_file(dir, "data.purrpck", "");
    let build = run(&["convert", "--from", "turtle", "--to", "pack", &ttl, &pack]);
    assert_eq!(code(&build), 0, "stderr:\n{}", stderr(&build));

    let capped = |data: &str| {
        run(&[
            "query",
            "--data",
            data,
            "--results-format",
            "json",
            "--max-answers",
            "2",
            KNOWS,
        ])
    };
    let text = capped(&ttl);
    let packed = capped(&pack);
    assert_eq!(code(&text), 3, "stderr:\n{}", stderr(&text));
    assert_eq!(code(&packed), 3, "stderr:\n{}", stderr(&packed));
    assert_eq!(text.stdout, packed.stdout, "the certified rows must match");
    assert_eq!(text.stderr, packed.stderr, "the governor report must match");
}

/// THE GOVERNOR REPORT IS BYTE-DETERMINISTIC across two identical runs.
///
/// Every value in it is a counter, a pinned label or an algebra variant name, and none of
/// them is a clock reading or a hash iteration — which is what makes the report something a
/// frozen corpus can pin rather than merely something a human can read.
#[test]
fn the_governor_report_is_byte_identical_across_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ttl = write_file(dir.path(), "data.ttl", DATA_TTL);
    let capped = || {
        run(&[
            "query",
            "--data",
            &ttl,
            "--max-answers",
            "1",
            "--max-intermediate-cells",
            "64",
            KNOWS,
        ])
    };
    let first = capped();
    let second = capped();
    assert_eq!(code(&first), 3, "stderr:\n{}", stderr(&first));
    assert_eq!(first.stderr, second.stderr, "the report must be stable");
    assert_eq!(first.stdout, second.stdout, "the rows must be stable");
}
