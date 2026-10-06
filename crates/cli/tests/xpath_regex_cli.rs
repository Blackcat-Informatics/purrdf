// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! End-to-end `--xpath-regex` coverage over the BUILT `purrdf` binary, on every subcommand
//! that decides a pattern: `query` and `update` (SPARQL `REGEX`/`REPLACE`), `validate`
//! (`sh:pattern` and SHACL-SPARQL `REGEX`), `shex` (string-facet `PATTERN`), `rules`
//! (a SHACL rule's `REGEX` and condition `sh:pattern`, a SPARQL 1.2 RL filter's `REGEX`)
//! and `node-expr` (a filter shape's `sh:pattern`, a `sparql:regex` call).
//!
//! Each law is shown to be the one APPLIED, not merely accepted, by two patterns whose
//! verdicts separate the three engines:
//!
//! * `(?:a)b` — a non-capturing group, defined by XPath F&O 3.1 and not by 2.0. The
//!   compatibility engine and the 3.1 law match `"ab"`; under 2.0 the pattern is invalid.
//! * `^(a)\1$` — a backreference, defined by both dated laws. Both match `"aa"` and not
//!   `"ab"`; the compatibility engine refuses backreferences outright.
//!
//! A pattern longer than the production source bound (64 KiB) is the resource refusal: it
//! must fail the run as an operational error, never read as an unbound value, a `false`
//! filter or a finding. All fixtures use `example.org`.

mod support;
use std::fmt::Write as _;

use support::{code, run, stderr, stdout, write_file};

/// The two stable dated-law names, oldest first.
const XPATH_20: &str = "xpath-2.0-2010-12-14";
const XPATH_31: &str = "xpath-3.1-2017-03-21";

/// `ex:a` carries `"aa"` and `ex:b` carries `"ab"`.
const DATA_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:a ex:v \"aa\" .\n",
    "ex:b ex:v \"ab\" .\n",
);

const EX_A: &str = "http://example.org/a";
const EX_B: &str = "http://example.org/b";

/// The answer of a pattern that matches nothing.
const NO_MATCH: [&str; 0] = [];

/// One byte past the production source bound every native law admits.
const OVERSIZED: usize = 64 * 1024 + 1;

/// The pattern-law arguments for `law`: none for the compatibility engine.
fn law_args(law: Option<&'static str>) -> Vec<&'static str> {
    law.map_or_else(Vec::new, |name| vec!["--xpath-regex", name])
}

/// Run `purrdf` with `head`, the `law` arguments, then `tail`.
fn run_under(head: &[&str], law: Option<&'static str>, tail: &[&str]) -> std::process::Output {
    let mut args = head.to_vec();
    args.extend(law_args(law));
    args.extend_from_slice(tail);
    run(&args)
}

/// Assert a successful exit, naming the stderr otherwise.
fn assert_ok(out: &std::process::Output, what: &str) {
    assert_eq!(code(out), 0, "{what} must exit 0; stderr:\n{}", stderr(out));
}

/// Assert the operational refusal shape: exit 1, nothing on stdout, and the refused
/// resource's stable code on stderr.
fn assert_refused(out: &std::process::Output, what: &str) {
    assert_eq!(
        code(out),
        1,
        "{what} must fail as a runtime error; stdout:\n{}\nstderr:\n{}",
        stdout(out),
        stderr(out)
    );
    assert!(
        out.stdout.is_empty(),
        "{what} must write no answer; got:\n{}",
        stdout(out)
    );
    assert!(
        stderr(out).contains("xpath-pattern-bytes"),
        "{what} must name the refused resource; stderr:\n{}",
        stderr(out)
    );
}

// --- query -------------------------------------------------------------------------------

/// The subjects `FILTER(REGEX(?v, pattern))` keeps, as the CSV rows of the answer.
fn query_matches(data: &str, law: Option<&'static str>, pattern: &str) -> Vec<String> {
    let query = format!(
        "SELECT ?s WHERE {{ ?s <http://example.org/v> ?v FILTER(REGEX(?v, \"{pattern}\")) }} \
         ORDER BY ?s"
    );
    let out = run_under(
        &["query", "--data", data, "--results-format", "csv"],
        law,
        &[&query],
    );
    assert_ok(&out, &format!("query under {law:?} with {pattern}"));
    stdout(&out)
        .lines()
        .skip(1)
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect()
}

#[test]
fn query_regex_runs_under_the_selected_law() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);

    // A non-capturing group: 3.1 defines it, 2.0 does not, so under 2.0 REGEX is an
    // expression error and the FILTER keeps nothing.
    assert_eq!(query_matches(&data, None, "(?:a)b"), [EX_B]);
    assert_eq!(query_matches(&data, Some(XPATH_31), "(?:a)b"), [EX_B]);
    assert_eq!(query_matches(&data, Some(XPATH_20), "(?:a)b"), NO_MATCH);

    // A backreference (`\\` is the SPARQL escape of one backslash): both dated laws
    // match "aa" alone, and the compatibility engine refuses the construct.
    assert_eq!(query_matches(&data, Some(XPATH_20), r"^(a)\\1$"), [EX_A]);
    assert_eq!(query_matches(&data, Some(XPATH_31), r"^(a)\\1$"), [EX_A]);
    assert_eq!(query_matches(&data, None, r"^(a)\\1$"), NO_MATCH);
}

#[test]
fn query_replace_runs_under_the_selected_law() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    let query = r#"SELECT ?r WHERE { <http://example.org/a> <http://example.org/v> ?v BIND(REPLACE(?v, "(a)\\1", "[$1]") AS ?r) }"#;
    for law in [XPATH_20, XPATH_31] {
        let out = run_under(
            &["query", "--data", &data, "--results-format", "csv"],
            Some(law),
            &[query],
        );
        assert_ok(&out, law);
        assert_eq!(stdout(&out).lines().nth(1), Some("[a]"), "under {law}");
    }
    // The compatibility engine refuses the backreference, so REPLACE leaves ?r unbound.
    let out = run_under(
        &["query", "--data", &data, "--results-format", "csv"],
        None,
        &[query],
    );
    assert_ok(&out, "compatibility REPLACE");
    assert_eq!(stdout(&out).lines().nth(1).map(str::trim), Some(""));
}

#[test]
fn query_native_resource_refusal_fails_the_query() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    let query_with = |len: usize| {
        format!(
            "SELECT ?s WHERE {{ ?s <http://example.org/v> ?v FILTER(REGEX(?v, \"{}\")) }}",
            "a".repeat(len)
        )
    };
    for law in [XPATH_20, XPATH_31] {
        let refused = run_under(
            &["query", "--data", &data, "--results-format", "csv"],
            Some(law),
            &[&query_with(OVERSIZED)],
        );
        assert_refused(&refused, &format!("an oversized pattern under {law}"));

        // The neighbour at the bound itself is admitted, evaluated, and matches nothing.
        let admitted = run_under(
            &["query", "--data", &data, "--results-format", "csv"],
            Some(law),
            &[&query_with(OVERSIZED - 1)],
        );
        assert_ok(&admitted, &format!("a pattern at the bound under {law}"));
        assert_eq!(stdout(&admitted).trim(), "s");
    }
}

// --- update ------------------------------------------------------------------------------

/// The N-Triples dataset an `INSERT … WHERE { FILTER(REGEX(?v, pattern)) }` writes.
fn update_output(data: &str, law: Option<&'static str>, pattern: &str) -> String {
    let update = format!(
        "INSERT {{ ?s <http://example.org/matched> true }} WHERE {{ ?s <http://example.org/v> \
         ?v FILTER(REGEX(?v, \"{pattern}\")) }}"
    );
    let out = run_under(
        &["update", "--data", data, "--to", "ntriples"],
        law,
        &[&update],
    );
    assert_ok(&out, &format!("update under {law:?} with {pattern}"));
    stdout(&out)
}

/// The subjects an update output marks `ex:matched`, sorted.
fn matched(ntriples: &str) -> Vec<&str> {
    let mut subjects: Vec<&str> = ntriples
        .lines()
        .filter(|line| line.contains("<http://example.org/matched>"))
        .filter_map(|line| line.split_whitespace().next())
        .map(|subject| subject.trim_start_matches('<').trim_end_matches('>'))
        .collect();
    subjects.sort_unstable();
    subjects
}

#[test]
fn update_where_runs_under_the_selected_law() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);

    assert_eq!(matched(&update_output(&data, None, "(?:a)b")), [EX_B]);
    assert_eq!(
        matched(&update_output(&data, Some(XPATH_31), "(?:a)b")),
        [EX_B]
    );
    assert_eq!(
        matched(&update_output(&data, Some(XPATH_20), "(?:a)b")),
        NO_MATCH
    );

    assert_eq!(
        matched(&update_output(&data, Some(XPATH_20), r"^(a)\\1$")),
        [EX_A]
    );
    assert_eq!(
        matched(&update_output(&data, Some(XPATH_31), r"^(a)\\1$")),
        [EX_A]
    );
    assert_eq!(matched(&update_output(&data, None, r"^(a)\\1$")), NO_MATCH);
}

#[test]
fn update_native_resource_refusal_applies_nothing() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    let target = dir.path().join("out.nt");
    let target = target.to_str().expect("utf-8 path");
    let update = format!(
        "INSERT {{ ?s <http://example.org/matched> true }} WHERE {{ ?s <http://example.org/v> \
         ?v FILTER(REGEX(?v, \"{}\")) }}",
        "a".repeat(OVERSIZED)
    );
    let out = run_under(
        &[
            "update", "--data", &data, "--output", target, "--to", "ntriples",
        ],
        Some(XPATH_31),
        &[&update],
    );
    assert_refused(&out, "an oversized UPDATE pattern");
    assert!(
        !std::path::Path::new(target).exists(),
        "a refused request must write no dataset"
    );
}

// --- validate ----------------------------------------------------------------------------

/// A shapes graph whose one property shape constrains `ex:v` by `sh:pattern`, given as a
/// Turtle string body (already escaped for Turtle).
fn pattern_shapes(turtle_pattern: &str) -> String {
    format!(
        "@prefix ex: <http://example.org/> .\n\
         @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:v ;\n  \
         sh:property [ sh:path ex:v ; sh:pattern \"{turtle_pattern}\" ] .\n"
    )
}

/// The focus nodes of a validation report's results, sorted, and the report text.
fn report_focus_nodes(out: &std::process::Output) -> (Vec<String>, String) {
    let report = stdout(out);
    let mut nodes: Vec<String> = report
        .lines()
        .filter(|line| line.contains("<http://www.w3.org/ns/shacl#focusNode>"))
        .filter_map(|line| line.split_whitespace().nth(2))
        .map(|node| {
            node.trim_start_matches('<')
                .trim_end_matches('>')
                .to_owned()
        })
        .collect();
    nodes.sort_unstable();
    (nodes, report)
}

/// Validate `data` against `shapes` with `extra` arguments under `law`.
fn validate(
    shapes_flag: &str,
    shapes: &str,
    data: &str,
    law: Option<&'static str>,
    extra: &[&str],
) -> (Vec<String>, String) {
    let mut tail = extra.to_vec();
    tail.push(data);
    let out = run_under(&["validate", shapes_flag, shapes], law, &tail);
    assert_ok(&out, &format!("validate under {law:?}"));
    report_focus_nodes(&out)
}

#[test]
fn validate_sh_pattern_runs_under_the_selected_law() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    let group = write_file(dir.path(), "group.ttl", &pattern_shapes("(?:a)b"));
    // `\\1` is Turtle's escape of the pattern's one backslash.
    let backref = write_file(dir.path(), "backref.ttl", &pattern_shapes(r"^(a)\\1$"));

    // The non-capturing group: only "aa" violates under compatibility and 3.1; under 2.0
    // the pattern is invalid and both values are reported against it, naming the law.
    for law in [None, Some(XPATH_31)] {
        let (nodes, report) = validate("--shapes", &group, &data, law, &[]);
        assert_eq!(nodes, [EX_A], "under {law:?}:\n{report}");
    }
    let (nodes, report) = validate("--shapes", &group, &data, Some(XPATH_20), &[]);
    assert_eq!(nodes, [EX_A, EX_B], "{report}");
    assert!(
        report.contains("non-capturing groups require XPath 3.1"),
        "{report}"
    );

    // The backreference: both laws reject "ab" alone; the compatibility engine reports
    // the pattern itself as unsupported, against both values.
    for law in [XPATH_20, XPATH_31] {
        let (nodes, report) = validate("--shapes", &backref, &data, Some(law), &[]);
        assert_eq!(nodes, [EX_B], "under {law}:\n{report}");
    }
    let (nodes, report) = validate("--shapes", &backref, &data, None, &[]);
    assert_eq!(nodes, [EX_A, EX_B], "{report}");
    assert!(report.contains("backreference"), "{report}");
}

#[test]
fn validate_applies_the_law_on_every_route() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    let backref = write_file(dir.path(), "backref.ttl", &pattern_shapes(r"^(a)\\1$"));
    let product = dir.path().join("backref.purrshp");
    let product = product.to_str().expect("utf-8 path");
    let packed = run(&["shacl", "pack", "--shapes", &backref, "--out", product]);
    assert_ok(&packed, "shacl pack");
    // The incremental lane: `ex:c "aa"` joins the graph.
    let changes = write_file(
        dir.path(),
        "changes.ttl",
        "@prefix ex: <http://example.org/> .\nex:c ex:v \"aa\" .\n",
    );

    for law in [XPATH_20, XPATH_31] {
        // A restored prepared product.
        let (nodes, report) = validate("--shapes-product", product, &data, Some(law), &[]);
        assert_eq!(nodes, [EX_B], "product under {law}:\n{report}");
        // The governed route.
        let (nodes, report) = validate("--shapes", &backref, &data, Some(law), &["--fuel", "1000"]);
        assert_eq!(nodes, [EX_B], "governed under {law}:\n{report}");
        // The change path: the affected focus node conforms under the law.
        let (nodes, report) = validate(
            "--shapes",
            &backref,
            &data,
            Some(law),
            &["--changes", &changes],
        );
        assert!(nodes.is_empty(), "change path under {law}:\n{report}");
    }
    // The same three routes on the compatibility engine still refuse the backreference.
    let (nodes, _) = validate("--shapes-product", product, &data, None, &[]);
    assert_eq!(nodes, [EX_A, EX_B]);
    let (nodes, _) = validate("--shapes", &backref, &data, None, &["--fuel", "1000"]);
    assert_eq!(nodes, [EX_A, EX_B]);
    let (nodes, _) = validate("--shapes", &backref, &data, None, &["--changes", &changes]);
    assert_eq!(nodes, ["http://example.org/c"]);
}

#[test]
fn validate_sparql_regex_runs_under_the_selected_law() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    // A SHACL-SPARQL constraint reporting every value its backreference REGEX matches.
    // The Turtle long string carries the SPARQL escape `\\1` as `\\\\1`.
    let shapes = write_file(
        dir.path(),
        "sparql.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
            "ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:v ;\n",
            "  sh:sparql [ sh:select \"\"\"SELECT $this WHERE { $this <http://example.org/v> ?v ",
            "FILTER(REGEX(?v, \"^(a)\\\\\\\\1$\")) }\"\"\" ] .\n",
        ),
    );
    for law in [XPATH_20, XPATH_31] {
        let (nodes, report) = validate("--shapes", &shapes, &data, Some(law), &[]);
        assert_eq!(nodes, [EX_A], "under {law}:\n{report}");
    }
    let (nodes, report) = validate("--shapes", &shapes, &data, None, &[]);
    assert!(nodes.is_empty(), "{report}");
}

#[test]
fn validate_native_resource_refusal_writes_no_report() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    let oversized = write_file(
        dir.path(),
        "oversized.ttl",
        &pattern_shapes(&"a".repeat(OVERSIZED)),
    );
    for extra in [&[][..], &["--fuel", "1000"][..]] {
        let mut tail = extra.to_vec();
        tail.push(&data);
        let out = run_under(&["validate", "--shapes", &oversized], Some(XPATH_31), &tail);
        assert_refused(&out, &format!("an oversized sh:pattern with {extra:?}"));
    }
    // The neighbour at the bound is admitted and decided as an ordinary pattern.
    let at_bound = write_file(
        dir.path(),
        "at_bound.ttl",
        &pattern_shapes(&"a".repeat(OVERSIZED - 1)),
    );
    let (nodes, report) = validate("--shapes", &at_bound, &data, Some(XPATH_31), &[]);
    assert_eq!(nodes, [EX_A, EX_B], "{report}");
}

// --- shex --------------------------------------------------------------------------------

/// The `(node, status)` pairs of a result shape map, in its own order.
fn shex_statuses(schema: &str, data: &str, law: Option<&'static str>) -> Vec<(String, String)> {
    let map = "<http://example.org/a>@<http://example.org/S>,<http://example.org/b>@<http://example.org/S>";
    let out = run_under(&["shex", "--schema", schema, "--data", data], law, &[map]);
    assert_ok(&out, &format!("shex under {law:?}"));
    let json = purrdf_lex::json::read(&stdout(&out)).expect("result shape map JSON");
    json.as_array()
        .expect("result shape map array")
        .iter()
        .map(|entry| {
            let field = |name: &str| {
                entry
                    .get(name)
                    .and_then(purrdf_lex::json::Value::as_str)
                    .expect("string field")
                    .to_owned()
            };
            (field("node"), field("status"))
        })
        .collect()
}

/// The expected `(node, status)` pairs, `ex:a` first.
fn statuses(a: &str, b: &str) -> Vec<(String, String)> {
    vec![
        (format!("<{EX_A}>"), a.to_owned()),
        (format!("<{EX_B}>"), b.to_owned()),
    ]
}

/// A ShExJ schema whose one shape constrains `ex:v` by `pattern` (JSON-escaped).
fn shexj_schema(json_pattern: &str) -> String {
    format!(
        "{{\"@context\":\"http://www.w3.org/ns/shex.jsonld\",\"type\":\"Schema\",\"shapes\":[\
         {{\"type\":\"Shape\",\"id\":\"http://example.org/S\",\"expression\":\
         {{\"type\":\"TripleConstraint\",\"predicate\":\"http://example.org/v\",\"valueExpr\":\
         {{\"type\":\"NodeConstraint\",\"pattern\":\"{json_pattern}\"}}}}}}]}}"
    )
}

#[test]
fn shex_pattern_runs_under_the_selected_law() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    let group = write_file(
        dir.path(),
        "group.shex",
        "PREFIX ex: <http://example.org/>\n<http://example.org/S> { ex:v /(?:a)b/ }\n",
    );
    // ShExC's own escape grammar has no `\1`, so the backreference travels in ShExJ.
    let backref = write_file(dir.path(), "backref.json", &shexj_schema(r"^(a)\\1$"));

    for law in [None, Some(XPATH_31)] {
        assert_eq!(
            shex_statuses(&group, &data, law),
            statuses("nonconformant", "conformant"),
            "under {law:?}"
        );
    }
    assert_eq!(
        shex_statuses(&group, &data, Some(XPATH_20)),
        statuses("nonconformant", "nonconformant")
    );

    for law in [XPATH_20, XPATH_31] {
        assert_eq!(
            shex_statuses(&backref, &data, Some(law)),
            statuses("conformant", "nonconformant"),
            "under {law}"
        );
    }
    assert_eq!(
        shex_statuses(&backref, &data, None),
        statuses("nonconformant", "nonconformant")
    );
}

#[test]
fn shex_native_resource_refusal_writes_no_shape_map() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    let oversized = write_file(
        dir.path(),
        "oversized.json",
        &shexj_schema(&"a".repeat(OVERSIZED)),
    );
    let map = "<http://example.org/a>@<http://example.org/S>";
    let out = run_under(
        &["shex", "--schema", &oversized, "--data", &data],
        Some(XPATH_31),
        &[map],
    );
    assert_refused(&out, "an oversized ShEx pattern");

    // The neighbour at the bound is an ordinary facet that "aa" fails.
    let at_bound = write_file(
        dir.path(),
        "at_bound.json",
        &shexj_schema(&"a".repeat(OVERSIZED - 1)),
    );
    let out = run_under(
        &["shex", "--schema", &at_bound, "--data", &data],
        Some(XPATH_31),
        &[map],
    );
    assert_ok(&out, "a ShEx pattern at the bound");
    assert!(
        stdout(&out).contains("\"nonconformant\""),
        "{}",
        stdout(&out)
    );
}

// --- rules -------------------------------------------------------------------------------

/// A shapes graph whose rule infers `?this ex:hit ?v` for each `ex:v` value `pattern`
/// matches: through a SPARQL rule's `REGEX`, or a triple rule's condition `sh:pattern`.
fn rule_shapes(pattern: &str, sparql: bool) -> String {
    let rule = if sparql {
        let construct = format!(
            "CONSTRUCT {{ $this <http://example.org/hit> ?v }} \
             WHERE {{ $this <http://example.org/v> ?v FILTER(REGEX(?v, {pattern:?})) }}"
        );
        format!("[ a sh:SPARQLRule ; sh:construct {construct:?} ]")
    } else {
        format!(
            "[ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:hit ; \
               sh:object [ sh:path ex:v ] ; \
               sh:condition [ sh:property [ sh:path ex:v ; sh:pattern {pattern:?} ] ] ]"
        )
    };
    format!(
        "@prefix ex: <http://example.org/> .\n\
         @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:v ; sh:rule {rule} .\n"
    )
}

/// A SPARQL 1.2 RL rule set inferring `?s :hit ?v` for each `:v` value `pattern` matches.
fn rule_set(pattern: &str) -> String {
    format!(
        "PREFIX : <http://example.org/>\n\
         RULE {{ ?s :hit ?v }} WHERE {{ ?s :v ?v FILTER(REGEX(?v, {pattern:?})) }}\n"
    )
}

/// The three rule routes over `pattern`: `(name, rule-source flag, file)`.
fn rule_sources(dir: &std::path::Path, pattern: &str) -> [(&'static str, &'static str, String); 3] {
    [
        (
            "SPARQL rule",
            "--shapes",
            write_file(dir, "sparql-rule.ttl", &rule_shapes(pattern, true)),
        ),
        (
            "triple rule",
            "--shapes",
            write_file(dir, "triple-rule.ttl", &rule_shapes(pattern, false)),
        ),
        (
            "SPARQL 1.2 RL",
            "--srl",
            write_file(dir, "rules.srl", &rule_set(pattern)),
        ),
    ]
}

/// `purrdf rules` over `data` under `law`, with the subjects of its inferred triples.
fn rules_run(
    flag: &str,
    source: &str,
    data: &str,
    law: Option<&'static str>,
) -> std::process::Output {
    run_under(&["rules", flag, source, "--to", "ntriples"], law, &[data])
}

fn inferred_subjects(out: &std::process::Output) -> Vec<String> {
    let mut subjects: Vec<String> = stdout(out)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(|node| {
            node.trim_start_matches('<')
                .trim_end_matches('>')
                .to_owned()
        })
        .collect();
    subjects.sort_unstable();
    subjects
}

#[test]
fn rules_run_under_the_selected_law() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    for (pattern, xpath20, xpath31) in [
        // The non-capturing group: under 2.0 it is ill-formed, so nothing is inferred.
        ("(?:a)b", &NO_MATCH[..], &[EX_B][..]),
        // The backreference: both laws match "aa" alone.
        (r"^(a)\1$", &[EX_A][..], &[EX_A][..]),
    ] {
        for (route, flag, source) in rule_sources(dir.path(), pattern) {
            for (law, expected) in [(XPATH_20, xpath20), (XPATH_31, xpath31)] {
                let out = rules_run(flag, &source, &data, Some(law));
                assert_ok(&out, &format!("{route} {pattern} under {law}"));
                assert_eq!(
                    inferred_subjects(&out),
                    expected,
                    "{route} {pattern} under {law}; stderr:\n{}",
                    stderr(&out)
                );
            }
        }
    }
    // Unselected, every route keeps the compatibility engine, which refuses the
    // backreference both laws match.
    for (route, flag, source) in rule_sources(dir.path(), r"^(a)\1$") {
        let out = rules_run(flag, &source, &data, None);
        assert_ok(&out, route);
        assert!(
            inferred_subjects(&out).is_empty(),
            "{route}: {}",
            stdout(&out)
        );
    }
}

#[test]
fn rules_native_resource_refusal_writes_no_graph() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    for law in [XPATH_20, XPATH_31] {
        for (route, flag, source) in rule_sources(dir.path(), &"a".repeat(OVERSIZED)) {
            let out = rules_run(flag, &source, &data, Some(law));
            assert_refused(&out, &format!("an oversized {route} pattern under {law}"));
        }
        // The neighbour at the bound runs, and matches nothing.
        for (route, flag, source) in rule_sources(dir.path(), &"a".repeat(OVERSIZED - 1)) {
            let out = rules_run(flag, &source, &data, Some(law));
            assert_ok(&out, &format!("a {route} pattern at the bound under {law}"));
            assert_eq!(inferred_subjects(&out), NO_MATCH);
        }
    }
}

#[test]
fn rules_check_refuses_a_law_it_would_not_apply() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let rules = write_file(dir.path(), "rules.srl", &rule_set("a"));
    let out = run(&[
        "rules",
        "--srl",
        &rules,
        "--check",
        "--xpath-regex",
        XPATH_31,
    ]);
    assert_eq!(code(&out), 2, "stderr:\n{}", stderr(&out));
    // The neighbour: the same check without the flag passes.
    assert_ok(
        &run(&["rules", "--srl", &rules, "--check"]),
        "rules --check",
    );
}

// --- node-expr ---------------------------------------------------------------------------

/// A shapes graph whose `_:e` filters the focus node by `sh:pattern`, and whose `_:r` is
/// `sparql:regex` of the focus node.
fn expression_shapes(pattern: &str) -> String {
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n\
         @prefix sparql: <http://www.w3.org/ns/sparql#> .\n\
         _:e shnex:filterShape [ sh:pattern {pattern} ] ; shnex:nodes [ shnex:var \"focusNode\" ] .\n\
         _:r sparql:regex ( [ shnex:var \"focusNode\" ] {pattern} ) .\n",
        pattern = format!("{pattern:?}")
    )
}

/// `purrdf node-expr` of `expr` at the literal focus `value`, under `law`.
fn node_expr(
    shapes: &str,
    data: &str,
    expr: &str,
    value: &str,
    law: Option<&'static str>,
) -> std::process::Output {
    let focus = format!("{value:?}");
    run_under(
        &[
            "node-expr",
            "--shapes",
            shapes,
            "--expr",
            expr,
            "--focus",
            &focus,
        ],
        law,
        &[data],
    )
}

#[test]
fn node_expr_runs_under_the_selected_law() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    let truth = "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>";
    for (pattern, value, xpath20, xpath31) in [
        ("(?:a)b", "ab", false, true),
        (r"^(a)\1$", "aa", true, true),
    ] {
        let shapes = write_file(dir.path(), "expr.ttl", &expression_shapes(pattern));
        for (law, matches) in [(XPATH_20, xpath20), (XPATH_31, xpath31)] {
            let filtered = node_expr(&shapes, &data, "_:e", value, Some(law));
            assert_ok(&filtered, &format!("filter {pattern} under {law}"));
            let expected = if matches {
                format!("{value:?}\n")
            } else {
                String::new()
            };
            assert_eq!(stdout(&filtered), expected, "filter {pattern} under {law}");
            let called = node_expr(&shapes, &data, "_:r", value, Some(law));
            assert_ok(&called, &format!("sparql:regex {pattern} under {law}"));
            assert_eq!(
                stdout(&called).trim() == truth,
                matches,
                "sparql:regex {pattern} under {law}: {}",
                stdout(&called)
            );
        }
    }
    // Unselected, the compatibility engine refuses the backreference.
    let shapes = write_file(dir.path(), "expr.ttl", &expression_shapes(r"^(a)\1$"));
    let filtered = node_expr(&shapes, &data, "_:e", "aa", None);
    assert_ok(&filtered, "compatibility filter");
    assert_eq!(stdout(&filtered), "");
}

#[test]
fn node_expr_native_resource_refusal_writes_no_output() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA_TTL);
    for law in [XPATH_20, XPATH_31] {
        let over = write_file(
            dir.path(),
            "over.ttl",
            &expression_shapes(&"a".repeat(OVERSIZED)),
        );
        for expr in ["_:e", "_:r"] {
            let out = node_expr(&over, &data, expr, "ab", Some(law));
            assert_refused(&out, &format!("an oversized {expr} pattern under {law}"));
        }
        let at = write_file(
            dir.path(),
            "at.ttl",
            &expression_shapes(&"a".repeat(OVERSIZED - 1)),
        );
        let out = node_expr(&at, &data, "_:e", "ab", Some(law));
        assert_ok(&out, &format!("a filter pattern at the bound under {law}"));
        assert_eq!(stdout(&out), "");
    }
}

// --- the value boundary ------------------------------------------------------------------

/// One minimal successful invocation of each covered subcommand, with the law argument
/// spliced in at `law`.
fn invocation(dir: &std::path::Path, subcommand: &str, law: &'static str) -> std::process::Output {
    let data = write_file(dir, "data.ttl", DATA_TTL);
    match subcommand {
        "query" => run(&["query", "--data", &data, "--xpath-regex", law, "ASK {}"]),
        "update" => run(&[
            "update",
            "--data",
            &data,
            "--to",
            "ntriples",
            "--xpath-regex",
            law,
            "CLEAR DEFAULT",
        ]),
        "validate" => {
            let shapes = write_file(dir, "shapes.ttl", &pattern_shapes("a"));
            run(&["validate", "--shapes", &shapes, "--xpath-regex", law, &data])
        }
        "shex" => {
            let schema = write_file(
                dir,
                "schema.shex",
                "<http://example.org/S> { <http://example.org/v> /a/ }\n",
            );
            run(&[
                "shex",
                "--schema",
                &schema,
                "--data",
                &data,
                "--xpath-regex",
                law,
                "<http://example.org/a>@<http://example.org/S>",
            ])
        }
        "rules" => {
            let shapes = write_file(dir, "shapes.ttl", &pattern_shapes("a"));
            run(&[
                "rules",
                "--shapes",
                &shapes,
                "--to",
                "ntriples",
                "--xpath-regex",
                law,
                &data,
            ])
        }
        "node-expr" => {
            let shapes = write_file(dir, "shapes.ttl", &pattern_shapes("a"));
            run(&[
                "node-expr",
                "--shapes",
                &shapes,
                "--expr",
                EX_A,
                "--focus",
                EX_A,
                "--xpath-regex",
                law,
                &data,
            ])
        }
        other => unreachable!("not a covered subcommand: {other}"),
    }
}

const COVERED: [&str; 6] = ["query", "update", "validate", "shex", "rules", "node-expr"];

#[test]
fn an_unknown_law_name_is_a_usage_error_and_the_exact_names_are_accepted() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    for subcommand in COVERED {
        for unknown in [
            "xpath-3.1",
            "XPATH-3.1-2017-03-21",
            "xpath-2.0",
            " xpath-3.1-2017-03-21",
            "",
        ] {
            let out = invocation(dir.path(), subcommand, unknown);
            assert_eq!(
                code(&out),
                2,
                "`{subcommand} --xpath-regex {unknown:?}` must be a usage error; stderr:\n{}",
                stderr(&out)
            );
            let err = stderr(&out);
            assert!(
                err.contains(XPATH_20) && err.contains(XPATH_31),
                "the refusal must list the accepted names; stderr:\n{err}"
            );
        }
        for exact in [XPATH_20, XPATH_31] {
            let out = invocation(dir.path(), subcommand, exact);
            assert_eq!(
                code(&out),
                0,
                "`{subcommand} --xpath-regex {exact}` must be accepted; stderr:\n{}",
                stderr(&out)
            );
        }
    }
}

#[test]
fn every_covered_subcommand_documents_the_flag_and_its_names() {
    for subcommand in COVERED {
        let out = run(&[subcommand, "--help"]);
        assert_ok(&out, &format!("{subcommand} --help"));
        let help = stdout(&out);
        for needle in ["--xpath-regex", XPATH_20, XPATH_31] {
            assert!(
                help.contains(needle),
                "`{subcommand} --help` must mention {needle}; got:\n{help}"
            );
        }
    }
}

// --- production-default scale ----------------------------------------------------------

#[test]
fn query_answers_the_adversary_shapes_at_the_production_defaults_like_the_compatibility_engine() {
    // Each shape was refused by the native matcher at a fraction of these sizes:
    // multiple unbounded runs at 8 KB, group repetition from 44 KB, a word repetition
    // at 159 KB, and the nested nullable repetition at 41 bytes.
    let text = purrdf_testkit::text::word_prose(1 << 20);
    let pairs = "ab".repeat(1 << 19);
    let forty = "a".repeat(40);
    let values = [
        ("prose", text.clone()),
        ("zzz", format!("{text} zzz")),
        ("bang", format!("{text}!")),
        ("spaced", format!("{text} ")),
        ("pairs", pairs.clone()),
        ("pairs_c", format!("{pairs}c")),
        ("forty_b", format!("{forty}b")),
        ("forty_c", format!("{forty}c")),
    ];
    let mut turtle = String::from("@prefix ex: <http://example.org/> .\n");
    for (subject, value) in &values {
        writeln!(turtle, "ex:{subject} ex:v \"{value}\" .").unwrap();
    }
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", &turtle);
    let subjects = |names: &[&str]| -> Vec<String> {
        names
            .iter()
            .map(|name| format!("http://example.org/{name}"))
            .collect()
    };
    // `\\` is the SPARQL escape of one backslash.
    for (pattern, matching) in [
        ("node.*graph.*zzz", subjects(&["zzz"])),
        ("alpha.*zzz", subjects(&["zzz"])),
        (
            "^([a-z]+ ?)+$",
            subjects(&[
                "forty_b", "forty_c", "pairs", "pairs_c", "prose", "spaced", "zzz",
            ]),
        ),
        (
            r"^(\\w+\\s)*\\w+$",
            subjects(&["forty_b", "forty_c", "pairs", "pairs_c", "prose", "zzz"]),
        ),
        ("^(a|b)*$", subjects(&["forty_b", "pairs"])),
        ("^(?:ab)*$", subjects(&["pairs"])),
        ("^(a|aa)*$|^(a*)*b$", subjects(&["forty_b"])),
    ] {
        assert_eq!(
            query_matches(&data, None, pattern),
            matching,
            "compatibility {pattern}"
        );
        for law in [XPATH_20, XPATH_31] {
            if law == XPATH_20 && pattern.contains("(?:") {
                continue;
            }
            assert_eq!(
                query_matches(&data, Some(law), pattern),
                matching,
                "{law} {pattern}"
            );
        }
    }
}

#[test]
fn query_backreference_blowup_fails_the_query_and_the_neighbour_answers() {
    let forty = "a".repeat(40);
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let refused = write_file(
        dir.path(),
        "refused.ttl",
        &format!("<http://example.org/a> <http://example.org/v> \"{forty}\" .\n"),
    );
    let neighbour = write_file(
        dir.path(),
        "neighbour.ttl",
        &format!("<http://example.org/a> <http://example.org/v> \"{forty}ca\" .\n"),
    );
    let pattern = r"^(a|aa)*c\\1$";
    for law in [XPATH_20, XPATH_31] {
        let query = format!(
            "SELECT ?s WHERE {{ ?s <http://example.org/v> ?v FILTER(REGEX(?v, \"{pattern}\")) }}"
        );
        let out = run_under(
            &["query", "--data", &refused, "--results-format", "csv"],
            Some(law),
            &[&query],
        );
        assert_eq!(code(&out), 1, "{law}: stderr:\n{}", stderr(&out));
        assert!(out.stdout.is_empty(), "{law}: {}", stdout(&out));
        assert!(
            [
                "xpath-match-steps",
                "xpath-match-states",
                "xpath-match-slots"
            ]
            .iter()
            .any(|resource| stderr(&out).contains(resource)),
            "{law}: {}",
            stderr(&out)
        );
        assert_eq!(
            query_matches(&neighbour, Some(law), pattern),
            [EX_A],
            "{law}"
        );
    }
}

#[test]
fn query_answers_the_counted_repetition_shapes_at_the_production_defaults_like_the_compatibility_engine()
 {
    // Each shape was refused by the native matcher at these sizes: a counted group
    // repeated from every start kept one thread per distinct count.
    let pairs = "ab".repeat(1 << 19);
    let mixed = "abbaab".repeat((4 << 20) / 6);
    let prose = purrdf_testkit::text::word_prose(8 << 20);
    let plain = [
        ("pairs_128k", &pairs[..128 << 10], "c"),
        ("pairs", pairs.as_str(), "c"),
        ("quads", &"abcd".repeat(1 << 18), "e"),
        ("mixed_400k", &mixed[..400 << 10], "c"),
        ("mixed_800k", &mixed[..800 << 10], "c"),
        ("mixed", mixed.as_str(), "c"),
        (
            "prose_4m",
            &prose[..prose[..4 << 20].rfind(' ').unwrap()],
            " zzz",
        ),
        ("prose", prose.as_str(), " zzz"),
        ("run", &"a".repeat(1 << 20), "b"),
    ];
    let mut turtle = String::from("@prefix ex: <http://example.org/> .\nex:empty ex:v \"\" .\n");
    for (subject, value, suffix) in &plain {
        writeln!(turtle, "ex:{subject} ex:v \"{value}\" .").unwrap();
        writeln!(turtle, "ex:{subject}_completed ex:v \"{value}{suffix}\" .").unwrap();
    }
    // A mebibyte of short runs, after a long one, that one replacement
    // rewrites thousands of times.
    let mut runs = "a".repeat(100_000);
    let mut index = 0_usize;
    while runs.len() < 1 << 20 {
        runs.push_str(&"a".repeat(1 + (index * 7 + index / 5) % 13));
        runs.push(if index.is_multiple_of(3) { 'c' } else { 'b' });
        index += 1;
    }
    writeln!(turtle, "ex:runs ex:v \"{runs}\" .").unwrap();
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", &turtle);
    // A count the compatibility engine refuses to build is compared through the
    // pattern with the same matches: a match exists exactly when the shorter one's
    // does. `\\` is the SPARQL escape of one backslash.
    let cases = [
        ("(ab){1,1000}c", "(ab){1,1000}c", "pairs_128k"),
        ("(ab){2,50}c", "(ab){2,50}c", "pairs"),
        ("(ab){1,100}c", "(ab){1,100}c", "pairs"),
        ("(ab|cd){1,20}e", "(ab|cd){1,20}e", "quads"),
        ("((a|b){3}){5,9}c", "((a|b){3}){5,9}c", "mixed_400k"),
        ("((a|b){2}){2,5}c", "((a|b){2}){2,5}c", "mixed_800k"),
        ("(a|b){1,30}c", "(a|b){1,30}c", "mixed"),
        ("(a|b){3,9}c", "(a|b){3,9}c", "mixed"),
        (r"(\\w+\\s){3,5}zzz", r"(\\w+\\s){3,5}zzz", "prose_4m"),
        ("node.*graph.*zzz", "node.*graph.*zzz", "prose"),
        ("(a|aa){1,1000}b", "(a|aa){1,1000}b", "run"),
        ("(ab){1,100000}c", "abc", "pairs_128k"),
        ("(a?){18446744073709551616}", "", "pairs"),
        ("^(a?){18446744073709551616}$", "^a*$", "empty"),
    ];
    // One query decides each pattern on its plain and completed subjects; a
    // nullable whole is also decided on another subject.
    let table = |law: Option<&'static str>, native: bool| -> Vec<String> {
        let mut rows = String::new();
        for (index, (pattern, compatible, subject)) in cases.iter().enumerate() {
            let pattern = if native { pattern } else { compatible };
            let others = if *subject == "empty" {
                vec!["empty".to_owned(), "pairs".to_owned()]
            } else {
                vec![(*subject).to_owned(), format!("{subject}_completed")]
            };
            for other in others {
                write!(
                    rows,
                    " ({index} <http://example.org/{other}> \"{pattern}\")"
                )
                .unwrap();
            }
        }
        let query = format!(
            "SELECT ?i ?s ?r WHERE {{ VALUES (?i ?s ?p) {{{rows} }} \
             ?s <http://example.org/v> ?v BIND(REGEX(?v, ?p) AS ?r) }} ORDER BY ?i ?s"
        );
        let out = run_under(
            &["query", "--data", &data, "--results-format", "csv"],
            law,
            &[&query],
        );
        assert_ok(&out, &format!("query under {law:?}"));
        stdout(&out)
            .lines()
            .map(|line| line.trim().to_owned())
            .collect()
    };
    let expected = table(None, false);
    assert_eq!(expected.len(), 1 + cases.len() * 2, "{expected:?}");
    for (index, (_, compatible, subject)) in cases.iter().enumerate() {
        let answer = |other: &str| {
            let prefix = format!("{index},http://example.org/{other},");
            let row = expected
                .iter()
                .find(|row| row.starts_with(&prefix))
                .unwrap_or_else(|| panic!("{prefix}: {expected:?}"));
            row[prefix.len()..].to_owned()
        };
        // Every pattern decides its completed subject; every pattern without a
        // nullable whole leaves its plain subject unmatched.
        if *subject == "empty" {
            assert_eq!(answer("empty"), "true", "{compatible}");
            assert_eq!(answer("pairs"), "false", "{compatible}");
        } else if compatible.is_empty() {
            assert_eq!(answer(subject), "true", "{compatible}");
        } else {
            assert_eq!(
                answer(&format!("{subject}_completed")),
                "true",
                "{compatible}"
            );
            assert_eq!(answer(subject), "false", "{compatible}");
        }
    }
    for law in [XPATH_20, XPATH_31] {
        assert_eq!(table(Some(law), true), expected, "{law}");
    }
    // Every match of an ambiguous counted body over the mebibyte of runs,
    // replaced with its last iteration's capture.
    for pattern in ["(a|aa){1,1000}b", "((a|aa){1,4})(b|c)"] {
        let query = format!(
            "SELECT ?r WHERE {{ <http://example.org/runs> <http://example.org/v> ?v \
             BIND(REPLACE(?v, \"{pattern}\", \"[$1]\") AS ?r) }}"
        );
        let replaced = |law| {
            let out = run_under(
                &["query", "--data", &data, "--results-format", "csv"],
                law,
                &[&query],
            );
            assert_ok(&out, &format!("REPLACE under {law:?}"));
            stdout(&out)
        };
        let expected = replaced(None);
        assert!(expected.contains("[a]"), "{pattern}");
        for law in [XPATH_20, XPATH_31] {
            assert_eq!(replaced(Some(law)), expected, "{law} {pattern}");
        }
    }
}
