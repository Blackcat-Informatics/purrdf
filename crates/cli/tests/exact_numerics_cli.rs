// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `xsd:integer` and `xsd:decimal` values of any size through the built `purrdf`
//! binary: SPARQL arithmetic and casts, the `--division` policy, SHACL range
//! constraints, ShEx numeric facets and OWL 2 DL consistency. Every refusal runs
//! beside a neighbouring command line that answers. Fixtures use `example.org`.

use std::fmt::Write as _;

mod support;
use support::{code, run, stderr_utf8 as stderr, stdout_utf8 as stdout, write_file};

/// `2^127`, one past `i128::MAX`.
const PAST_I128: &str = "170141183460469231731687303715884105728";

/// `10^42 + 1` and `10^42 + 2`.
fn ten_42(plus: u8) -> String {
    format!("1{}{plus}", "0".repeat(41))
}

/// A dataset holding one value for `ex:v`.
fn data_with(dir: &std::path::Path, value: &str) -> String {
    write_file(
        dir,
        "data.ttl",
        &format!(
            "@prefix ex: <http://example.org/> .\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
             ex:s ex:v \"{value}\"^^xsd:integer .\n"
        ),
    )
}

/// `purrdf query` over an empty-ish dataset, returning the TSV body.
fn query(dir: &std::path::Path, extra: &[&str], text: &str) -> std::process::Output {
    let data = write_file(dir, "empty.ttl", "@prefix ex: <http://example.org/> .\n");
    let mut args = vec!["query", "--data", &data, "--results-format", "tsv"];
    args.extend_from_slice(extra);
    args.push(text);
    run(&args)
}

#[test]
fn query_answers_past_machine_words_exactly() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let max = i128::MAX.to_string();
    let out = query(
        dir.path(),
        &[],
        &format!(
            "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> SELECT ?a ?b ?c WHERE {{ \
             BIND({max} + 1 AS ?a) BIND(0.1000000000000000000000000001 + 0 AS ?b) \
             BIND(xsd:integer(\"1e300\"^^xsd:double) AS ?c) }}"
        ),
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let body = stdout(&out);
    assert!(body.contains(PAST_I128), "{body}");
    assert!(body.contains("0.1000000000000000000000000001"), "{body}");
    // The double nearest 1e300, digit for digit (Python's `int(1e300)`).
    assert!(
        body.contains("1000000000000000052504760255204420248704468581108159154915854115511802457988908195786371375080447864043704443832883878176942523235360430575644792184786706982848387200926575803737830233794788090059368953234970799945081119038967640880074652742780142494579258788820056842838115669472196386865459400540160"),
        "{body}"
    );
}

#[test]
fn division_policy_is_selectable_and_leaves_only_a_non_terminating_quotient_unbound() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    // The default: eighteen digits, truncated.
    let out = query(dir.path(), &[], "SELECT (1 / 3 AS ?q) WHERE {}");
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("0.333333333333333333"),
        "{}",
        stdout(&out)
    );
    // `exact` answers a terminating quotient exactly…
    let out = query(
        dir.path(),
        &["--division", "exact"],
        "SELECT (1 / 8 AS ?q) WHERE {}",
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("0.125"), "{}", stdout(&out));
    // …and leaves a non-terminating one unbound, an expression error (SPARQL §17.2):
    // the query answers, and a governed run names the F&O code on stderr.
    let out = query(
        dir.path(),
        &["--division", "exact", "--fuel", "1000000000"],
        "SELECT (1 / 3 AS ?q) (COALESCE(1 / 3, \"caught\") AS ?c) WHERE {}",
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("\"caught\""), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("purrdf-expression-errors 1\nabsorbed err:FOAR0002 2\n"),
        "{}",
        stderr(&out)
    );
    // A scale and rounding.
    let out = query(
        dir.path(),
        &["--division", "4:half-even"],
        "SELECT (2 / 3 AS ?q) WHERE {}",
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("0.6667"), "{}", stdout(&out));
    // A malformed policy is a usage error; its neighbour parses.
    let out = query(
        dir.path(),
        &["--division", "4:sideways"],
        "SELECT (2 / 3 AS ?q) WHERE {}",
    );
    assert_eq!(code(&out), 2, "{}", stderr(&out));
    let out = query(
        dir.path(),
        &["--division", "4:floor"],
        "SELECT (2 / 3 AS ?q) WHERE {}",
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains("0.6666"), "{}", stdout(&out));
}

/// A governed query reports the XPath F&O errors it absorbed into unbound values on
/// stderr, one `absorbed CODE COUNT` line per code, while stdout keeps the rows; the
/// same query without the failing expression writes no block.
#[test]
fn a_governed_query_names_the_errors_it_absorbed() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let governed = ["--fuel", "1000000000"];
    let out = query(
        dir.path(),
        &governed,
        "SELECT (1 / 0 AS ?q) (\"x\" + 1 AS ?s) (1 / 0 AS ?r) (2 AS ?k) WHERE {}",
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let report = stderr(&out);
    assert!(report.contains("purrdf-expression-errors 1\n"), "{report}");
    assert!(report.contains("absorbed err:FOAR0001 2\n"), "{report}");
    assert!(report.contains("absorbed err:XPTY0004 1\n"), "{report}");
    assert!(stdout(&out).contains('2'), "{}", stdout(&out));
    // Neighbour: nothing fails, nothing is reported.
    let out = query(
        dir.path(),
        &governed,
        "SELECT (1 / 2 AS ?q) (2 AS ?k) WHERE {}",
    );
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        !stderr(&out).contains("purrdf-expression-errors"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_fuel_ceiling_refuses_a_squaring_blow_up_and_admits_its_neighbour() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let chain = |steps: usize| {
        let mut binds = format!("BIND({} AS ?x0)", "7".repeat(1000));
        for step in 1..=steps {
            let previous = step - 1;
            let _ = write!(binds, " BIND(?x{previous} * ?x{previous} AS ?x{step})");
        }
        format!("SELECT (STRLEN(STR(?x{steps})) AS ?len) WHERE {{ {binds} }}")
    };
    let out = query(dir.path(), &["--fuel", "2000000"], &chain(20));
    assert_eq!(
        code(&out),
        3,
        "the blow-up trips the ceiling: {}",
        stderr(&out)
    );
    let out = query(dir.path(), &["--fuel", "2000000"], &chain(3));
    assert_eq!(code(&out), 0, "the neighbour answers: {}", stderr(&out));
}

#[test]
fn shacl_range_constraints_compare_every_digit() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let shapes = write_file(
        dir.path(),
        "shapes.ttl",
        &format!(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <http://example.org/> .\n\
             @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
             ex:S a sh:NodeShape ; sh:targetNode ex:s ;\n\
               sh:property [ sh:path ex:v ; sh:maxInclusive \"{}\"^^xsd:integer ] .\n",
            ten_42(1)
        ),
    );
    let over = data_with(dir.path(), &ten_42(2));
    let out = run(&["validate", "--shapes", &shapes, &over]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("shacl conforms false\n"),
        "10^42+2 violates: {}",
        stderr(&out)
    );
    let at = data_with(dir.path(), &ten_42(1));
    let out = run(&["validate", "--shapes", &shapes, &at]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("shacl conforms true\n"),
        "10^42+1 conforms: {}",
        stderr(&out)
    );
}

#[test]
fn shex_numeric_facets_compare_every_digit() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let schema = write_file(
        dir.path(),
        "schema.shex",
        &format!(
            "PREFIX ex: <http://example.org/>\nPREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
             ex:S {{ ex:v xsd:integer MAXINCLUSIVE {} }}\n",
            ten_42(1)
        ),
    );
    let map = "<http://example.org/s>@<http://example.org/S>";
    let over = data_with(dir.path(), &ten_42(2));
    let out = run(&["shex", "--schema", &schema, "--data", &over, map]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("\"status\":\"nonconformant\""),
        "10^42+2 is past MAXINCLUSIVE 10^42+1: {}",
        stdout(&out)
    );
    let at = data_with(dir.path(), &ten_42(1));
    let out = run(&["shex", "--schema", &schema, "--data", &at, map]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("\"status\":\"conformant\""),
        "10^42+1 is at the bound: {}",
        stdout(&out)
    );
    // A value of any size is a well-formed xsd:integer.
    let huge = data_with(dir.path(), &format!("-{}", "9".repeat(80)));
    let out = run(&["shex", "--schema", &schema, "--data", &huge, map]);
    assert!(
        stdout(&out).contains("\"status\":\"conformant\""),
        "{}",
        stdout(&out)
    );
}

#[test]
fn owl_dl_tells_two_sixty_digit_values_apart() {
    let dir = purrdf_testkit::temp_dir!().expect("tempdir");
    let ontology = |second: &str| {
        format!(
            "@prefix : <http://example.org/> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
             :v a owl:DatatypeProperty , owl:FunctionalProperty .\n\
             :a :v \"1{zeros}\"^^xsd:integer .\n:a :v \"{second}\"^^xsd:integer .\n",
            zeros = "0".repeat(59)
        )
    };
    // Two different values of a functional property: inconsistent.
    let differ = write_file(
        dir.path(),
        "differ.ttl",
        &ontology(&format!("1{}1", "0".repeat(58))),
    );
    let out = run(&["consistency", &differ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).starts_with("consistency false\n"),
        "{}",
        stdout(&out)
    );
    // The same value written twice (one with a leading zero): consistent.
    let same = write_file(
        dir.path(),
        "same.ttl",
        &ontology(&format!("01{}", "0".repeat(59))),
    );
    let out = run(&["consistency", &same]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).starts_with("consistency true\n"),
        "{}",
        stdout(&out)
    );
}
