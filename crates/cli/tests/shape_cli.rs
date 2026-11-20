// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! End-to-end `shape` coverage that drives the BUILT `purrdf` binary.
//!
//! The verb's contract is that it prints aggregate figures and nothing that identifies
//! content. Each test names the part of that contract it protects:
//!
//! * recognizable fixture content never reaches stdout or stderr;
//! * a malformed or missing input is refused without quoting its text or its name;
//! * the figures of a known dataset are exact;
//! * one dataset read from Turtle, N-Quads and GTS gives one document;
//! * a count below the small-count threshold prints as a bucket, a count at it exactly.

mod support;
use support::{code, run, stderr, stdout, write_file};

use purrdf_core::RdfLookaside;

/// One object of a fixture statement.
enum Object {
    Blank(&'static str),
    Plain(&'static str),
    Typed(&'static str, &'static str),
    Tagged(&'static str, &'static str),
    Triple(&'static str, &'static str, &'static str),
}

/// A fixture statement; subjects and predicates are IRI local names, except a blank subject.
struct Row {
    subject: &'static str,
    predicate: &'static str,
    object: Object,
}

/// Strings that must never appear in an output.
const NEEDLES: [&str; 9] = [
    "needle-subject",
    "needle-pred",
    "zebra-lit",
    "zq-needlex",
    "marker-parent",
    "marker-kid",
    "bonjour-",
    "same-secret",
    "example.org",
];

fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    let subjects = [
        "needle-subject-1",
        "needle-subject-2",
        "needle-subject-3",
        "needle-subject-4",
        "needle-subject-5",
        "needle-subject-6",
    ];
    let strings = [
        "zebra-lit-1",
        "zebra-lit-2",
        "zebra-lit-3",
        "zebra-lit-4",
        "zebra-lit-5",
        "zebra-lit-6",
    ];
    for (subject, text) in subjects.iter().zip(strings) {
        rows.push(Row {
            subject,
            predicate: "needle-pred-string",
            object: Object::Plain(text),
        });
    }
    for (subject, number) in subjects.iter().zip(["1", "2", "3", "4", "5"]) {
        rows.push(Row {
            subject,
            predicate: "needle-pred-int",
            object: Object::Typed(number, "http://www.w3.org/2001/XMLSchema#integer"),
        });
    }
    for (subject, text) in subjects.iter().zip([
        "bonjour-1",
        "bonjour-2",
        "bonjour-3",
        "bonjour-4",
        "bonjour-5",
    ]) {
        rows.push(Row {
            subject,
            predicate: "needle-pred-lang",
            object: Object::Tagged(text, "zq-needlex"),
        });
    }
    rows.push(Row {
        subject: "needle-subject-1",
        predicate: "needle-pred-has",
        object: Object::Blank("marker-parent"),
    });
    for kid in [
        "marker-kid-1",
        "marker-kid-2",
        "marker-kid-3",
        "marker-kid-4",
        "marker-kid-5",
    ] {
        rows.push(Row {
            subject: "_:marker-parent",
            predicate: "needle-pred-link",
            object: Object::Blank(kid),
        });
        rows.push(Row {
            subject: Box::leak(format!("_:{kid}").into_boxed_str()),
            predicate: "needle-pred-q",
            object: Object::Plain("same-secret"),
        });
    }
    rows.push(Row {
        subject: "needle-subject-1",
        predicate: "needle-pred-triple",
        object: Object::Triple("needle-subject-2", "needle-pred-string", "zebra-lit-2"),
    });
    rows
}

fn node(name: &str) -> String {
    if name.starts_with("_:") {
        name.to_owned()
    } else {
        format!("<http://example.org/{name}>")
    }
}

fn prefixed(name: &str) -> String {
    if name.starts_with("_:") {
        name.to_owned()
    } else {
        format!("e:{name}")
    }
}

/// The fixture as N-Quads (the default graph only), or as Turtle written with a prefix.
fn document(rows: &[Row], turtle: bool) -> String {
    let name = |value: &str| {
        if turtle { prefixed(value) } else { node(value) }
    };
    let mut out = String::new();
    if turtle {
        out.push_str("@prefix e: <http://example.org/> .\n");
    }
    for row in rows {
        let object = match &row.object {
            Object::Blank(label) => format!("_:{label}"),
            Object::Plain(text) => format!("\"{text}\""),
            Object::Typed(text, datatype) => format!("\"{text}\"^^<{datatype}>"),
            Object::Tagged(text, tag) => format!("\"{text}\"@{tag}"),
            Object::Triple(s, p, o) => format!("<<( {} {} \"{o}\" )>>", name(s), name(p)),
        };
        out += &[
            name(row.subject),
            name(row.predicate),
            object,
            ".\n".to_owned(),
        ]
        .join(" ");
    }
    out
}

/// Append one line to a fixture.
fn push(out: &mut String, line: &str) {
    out.push_str(line);
}

fn shape(args: &[&str]) -> std::process::Output {
    let mut all = vec!["shape"];
    all.extend_from_slice(args);
    run(&all)
}

fn fixture_files(dir: &std::path::Path) -> (String, String, String) {
    let rows = rows();
    let nquads = document(&rows, false);
    let nq = write_file(dir, "data.nq", &nquads);
    let ttl = write_file(dir, "data.ttl", &document(&rows, true));
    let dataset = purrdf_rdf::parse_dataset(nquads.as_bytes(), "application/n-quads", None)
        .expect("the fixture parses");
    let bytes = purrdf_rdf::gts_write::to_gts(&dataset, &RdfLookaside::default(), "purrdf.gts")
        .expect("the fixture encodes as GTS");
    let gts = support::path(dir, "data.gts");
    std::fs::write(&gts, bytes).expect("write the GTS fixture");
    (ttl, nq, gts)
}

/// Protects: no IRI, literal, tag or blank label of the input reaches either stream, and the
/// run that proves it succeeded on every source syntax.
#[test]
fn recognizable_content_never_reaches_stdout_or_stderr() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let (ttl, nq, gts) = fixture_files(dir.path());
    for input in [&ttl, &nq, &gts] {
        let out = shape(&[input]);
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        let printed = format!("{}{}", stdout(&out), stderr(&out));
        for needle in NEEDLES {
            assert!(!printed.contains(needle), "`{needle}` leaked");
        }
        for name in ["data.ttl", "data.nq", "data.gts"] {
            assert!(!printed.contains(name), "a file name leaked");
        }
        assert_eq!(stderr(&out), "", "nothing is written but stdout");
        assert!(stdout(&out).starts_with("{\"schema_version\":1,"));
    }
}

/// Protects: a malformed, missing or unclassifiable input is refused with a fixed sentence,
/// so neither its text nor its name can leak through an error. The neighbouring well-formed
/// input of the same name pattern succeeds.
#[test]
fn a_refused_input_quotes_neither_its_text_nor_its_name() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let broken = write_file(
        dir.path(),
        "secret-name-qq.ttl",
        "<http://example.org/needle-bad> <http://example.org/p \"oops-secret\" .\n",
    );
    let out = shape(&[&broken]);
    assert_eq!(code(&out), 1);
    assert_eq!(stdout(&out), "");
    for text in ["needle-bad", "oops-secret", "secret-name-qq", "example.org"] {
        assert!(!stderr(&out).contains(text), "`{text}` leaked into stderr");
    }
    assert!(stderr(&out).contains("could not be read as"));

    let missing = support::path(dir.path(), "secret-missing-qq.nq");
    let out = shape(&[&missing]);
    assert_eq!(code(&out), 1);
    assert_eq!(stdout(&out), "");
    assert!(!stderr(&out).contains("secret-missing-qq"));

    let unknown = write_file(dir.path(), "secret-ext-qq.zzz", "x");
    let out = shape(&[&unknown]);
    assert_eq!(code(&out), 2);
    assert_eq!(stdout(&out), "");
    assert!(!stderr(&out).contains("secret-ext-qq"));

    let valid = write_file(
        dir.path(),
        "secret-name-qq-ok.nq",
        "<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n",
    );
    let out = shape(&["--no-labelling", &valid]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
}

/// Protects: every figure of a known dataset, including bin bounds and datatype counts.
#[test]
fn the_figures_of_a_known_dataset_are_exact() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let (_, nq, _) = fixture_files(dir.path());
    let out = shape(&["--no-labelling", &nq]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        concat!(
            "{\"schema_version\":1,\"small_count_threshold\":5,",
            "\"counts\":{\"quads\":28,\"distinct_iris\":13,\"distinct_literals\":17,",
            "\"blank_nodes\":6,\"named_graphs\":0,\"triple_terms\":\"<5\",",
            "\"reifiers\":0,\"annotations\":0},\"max_triple_term_depth\":1,",
            "\"blank_siblings\":{\"parents\":\"<5\",\"max_largest_like_group\":5,",
            "\"largest_like_group_histogram\":[{\"min\":4,\"max\":7,\"count\":\"<5\"}]},",
            "\"literal_length_bytes_histogram\":[{\"min\":1,\"max\":1,\"count\":5},",
            "{\"min\":8,\"max\":15,\"count\":12}],",
            "\"datatypes\":{\"named_xsd\":{\"integer\":5,\"string\":7},\"other\":0},",
            "\"language_tags\":{\"tagged_literals\":5,\"distinct_tags\":\"<5\",",
            "\"case_forms\":{\"status\":\"omitted\",\"reason\":\"reader-folds-before-figure\"}},",
            "\"canonical_labelling\":{\"status\":\"omitted\",\"reason\":\"disabled\"}}\n"
        )
    );
}

/// Protects: the canonical labelling figure is taken by default, and a guaranteed input is
/// never refused by a stated limit.
#[test]
fn the_labelling_figure_is_taken_and_a_limit_spares_a_guaranteed_input() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let (_, nq, _) = fixture_files(dir.path());
    let out = shape(&[&nq]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("\"canonical_labelling\":{\"status\":\"labelled\",\"lines\":28,"),
        "{}",
        stdout(&out)
    );
    // A guaranteed input is never refused, whatever limit is stated.
    let limited = shape(&["--labelling-limit", "0", &nq]);
    assert_eq!(code(&limited), 0, "{}", stderr(&limited));
    assert_eq!(stdout(&limited), stdout(&out));
}

/// The document with its `case_forms` object removed: only a reader that sees the tag as
/// written can report it, so that one figure differs by syntax and the rest must not.
fn without_case_forms(document: &str) -> String {
    let start = document
        .find("\"case_forms\":")
        .expect("a case_forms figure");
    let end = start + document[start..].find('}').expect("its closing brace") + 1;
    format!("{}{}", &document[..start], &document[end..])
}

/// Protects: the figures belong to the dataset, not to the syntax it arrived in. A pack is
/// read under the caller's canonical control, which only a pack accepts.
#[test]
fn one_dataset_gives_one_document_from_turtle_nquads_gts_and_pack() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let (ttl, nq, gts) = fixture_files(dir.path());
    let pack = support::path(dir.path(), "data.purrpck");
    let receipts = support::path(dir.path(), "convert.json");
    let out = run(&[
        "convert",
        &nq,
        &pack,
        "--canonical-search",
        "unlimited",
        "--canonicalization-receipts",
        &receipts,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let from_nquads = shape(&[&nq]);
    assert_eq!(code(&from_nquads), 0, "{}", stderr(&from_nquads));
    for other in [&ttl, &gts] {
        let out = shape(&[other]);
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        assert_eq!(
            without_case_forms(&stdout(&out)),
            without_case_forms(&stdout(&from_nquads))
        );
    }
    let shape_receipts = support::path(dir.path(), "shape.json");
    let out = shape(&[
        "--canonical-search",
        "unlimited",
        "--canonicalization-receipts",
        &shape_receipts,
        &pack,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert_eq!(stdout(&out), stdout(&from_nquads));
}

/// Protects: the verb distinguishes a pack read without its canonical control from a bad pack
/// and from canonical flags on an input that never canonicalizes, by fixed sentence.
#[test]
fn a_pack_needs_canonical_control_and_only_a_pack_accepts_it() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let (ttl, nq, _) = fixture_files(dir.path());
    let pack = support::path(dir.path(), "data.purrpck");
    let receipts = support::path(dir.path(), "convert.json");
    let out = run(&[
        "convert",
        &nq,
        &pack,
        "--canonical-search",
        "unlimited",
        "--canonicalization-receipts",
        &receipts,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));

    let missing = shape(&[&pack]);
    assert_eq!(code(&missing), 2);
    assert_eq!(stdout(&missing), "");
    assert!(stderr(&missing).contains("a pack input is verified by canonicalizing it"));
    assert!(!stderr(&missing).contains("could not be read"));

    let not_pack = write_file(dir.path(), "broken.purrpck", "not a pack");
    let flagged = |input: &str| {
        let destination = support::path(dir.path(), "refused.json");
        shape(&[
            "--canonical-search",
            "unlimited",
            "--canonicalization-receipts",
            &destination,
            input,
        ])
    };
    let broken = flagged(&not_pack);
    assert_eq!(code(&broken), 1);
    assert!(stderr(&broken).contains("could not be read as"));
    assert!(!stderr(&broken).contains("broken.purrpck"));

    // `--base` has nothing to resolve in a pack, and still resolves a text input.
    let based = shape(&[
        "--base",
        "http://example.org/",
        "--canonical-search",
        "unlimited",
        "--canonicalization-receipts",
        &support::path(dir.path(), "based.json"),
        &pack,
    ]);
    assert_eq!(code(&based), 2);
    assert!(stderr(&based).contains("--base has no effect"));
    let text_based = shape(&["--base", "http://example.org/", "--no-labelling", &ttl]);
    assert_eq!(code(&text_based), 0, "{}", stderr(&text_based));

    let wrapped = flagged(&ttl);
    assert_eq!(code(&wrapped), 2);
    assert!(stderr(&wrapped).contains("only a pack input canonicalizes"));
    assert_eq!(stdout(&wrapped), "");
}

/// Protects: a named graph is one figure whatever the syntax.
#[test]
fn a_named_graph_gives_one_document_from_trig_nquads_and_gts() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let quads = (0..6)
        .map(|index| {
            format!(
                "<http://example.org/s{index}> <http://example.org/p> <http://example.org/o> <http://example.org/g> .\n"
            )
        })
        .collect::<Vec<_>>().concat();
    let trig_body = (0..6)
        .map(|index| format!("e:s{index} e:p e:o .\n"))
        .collect::<Vec<_>>()
        .concat();
    let nq = write_file(dir.path(), "named.nq", &quads);
    let trig = write_file(
        dir.path(),
        "named.trig",
        &format!("@prefix e: <http://example.org/> .\ne:g {{\n{trig_body}}}\n"),
    );
    let dataset = purrdf_rdf::parse_dataset(quads.as_bytes(), "application/n-quads", None)
        .expect("the fixture parses");
    let bytes = purrdf_rdf::gts_write::to_gts(&dataset, &RdfLookaside::default(), "purrdf.gts")
        .expect("the fixture encodes as GTS");
    let gts = support::path(dir.path(), "named.gts");
    std::fs::write(&gts, bytes).expect("write the GTS fixture");
    let from_nquads = shape(&[&nq]);
    assert_eq!(code(&from_nquads), 0, "{}", stderr(&from_nquads));
    assert!(stdout(&from_nquads).contains("\"named_graphs\":\"<5\""));
    for other in [&trig, &gts] {
        let out = shape(&[other]);
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        assert_eq!(
            without_case_forms(&stdout(&out)),
            without_case_forms(&stdout(&from_nquads))
        );
    }
}

/// Protects: the small-count threshold. Four quads print as a bucket and five exactly.
#[test]
fn a_count_below_the_threshold_prints_as_a_bucket_and_one_at_it_exactly() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let line = |index: usize| {
        format!("<http://example.org/s{index}> <http://example.org/p> <http://example.org/o> .\n")
    };
    let four: String = (0..4).map(line).collect();
    let five: String = (0..5).map(line).collect();
    let four = write_file(dir.path(), "four.nq", &four);
    let five = write_file(dir.path(), "five.nq", &five);
    let below = shape(&["--no-labelling", &four]);
    assert_eq!(code(&below), 0, "{}", stderr(&below));
    assert!(
        stdout(&below).contains("\"quads\":\"<5\""),
        "{}",
        stdout(&below)
    );
    let at = shape(&["--no-labelling", &five]);
    assert_eq!(code(&at), 0, "{}", stderr(&at));
    assert!(stdout(&at).contains("\"quads\":5,"), "{}", stdout(&at));
}

/// Protects: the RDF 1.2 statement layer is counted, one reifier and one annotation per
/// annotated statement.
#[test]
fn reifiers_and_annotations_are_counted() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let statements = (0..5)
        .map(|index| format!("e:s{index} e:p e:o {{| e:q \"v{index}\" |}} .\n"))
        .collect::<Vec<_>>()
        .concat();
    let text = format!("@prefix e: <http://example.org/> .\n{statements}");
    let input = write_file(dir.path(), "annotated.ttl", &text);
    let out = shape(&["--no-labelling", &input]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("\"reifiers\":5,\"annotations\":5}"),
        "{}",
        stdout(&out)
    );
}

/// N-Quads of typed and plain literals: `plain` strings and `typed` integers, each one
/// distinct, and one empty string when `empty` holds.
fn literal_fixture(plain: &[&str], typed: &[&str], empty: bool) -> String {
    let mut out = String::new();
    let mut add = |object: String| {
        push(
            &mut out,
            &format!("<http://example.org/s> <http://example.org/p> {object} .\n"),
        );
    };
    for text in plain {
        add(format!("\"{text}\""));
    }
    for text in typed {
        add(format!(
            "\"{text}\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        ));
    }
    if empty {
        add("\"\"".to_owned());
    }
    out
}

/// The unsigned integers a document prints exactly, and whether each given path is exact.
fn exact(document: &purrdf_lex::json::Value, path: &[&str]) -> Option<u64> {
    path.iter()
        .fold(document, |value, key| &value[*key])
        .as_u64()
}

/// The interval a printed count allows: exact, `"<5"`, `"withheld"` or `">=N"`.
fn interval(value: &purrdf_lex::json::Value) -> (u64, u64) {
    if let Some(n) = value.as_u64() {
        return (n, n);
    }
    match value.as_str().expect("a count or a suppressed form") {
        "<5" => (1, 4),
        "withheld" => (5, u64::MAX),
        other => (
            other
                .strip_prefix(">=")
                .expect("a lower bound")
                .parse()
                .expect("its number"),
            u64::MAX,
        ),
    }
}

/// What a reader who knows every printed form learns about each cell of the distinct
/// literals: the cells of the two breakdowns and the total, tightened by the two equations
/// until nothing changes. Returns each cell's interval beside its printed one.
fn learned(document: &purrdf_lex::json::Value) -> Vec<((u64, u64), (u64, u64))> {
    let mut cells: Vec<(u64, u64)> = document["literal_length_bytes_histogram"]
        .as_array()
        .expect("bins")
        .iter()
        .map(|bin| interval(&bin["count"]))
        .collect();
    let lengths = cells.len();
    for name in ["integer", "string"] {
        cells.push(interval(&document["datatypes"]["named_xsd"][name]));
    }
    cells.push(interval(&document["datatypes"]["other"]));
    cells.push(interval(&document["language_tags"]["tagged_literals"]));
    let printed = cells.clone();
    let mut total = interval(&document["counts"]["distinct_literals"]);
    let partitions = [0..lengths, lengths..cells.len()];
    for _ in 0..16 {
        for part in &partitions {
            let low: u64 = cells[part.clone()].iter().map(|c| c.0).sum();
            let high = cells[part.clone()]
                .iter()
                .fold(0_u64, |acc, c| acc.saturating_add(c.1));
            total.0 = total.0.max(low);
            if high != u64::MAX {
                total.1 = total.1.min(high);
            }
            for index in part.clone() {
                let others_low = low - cells[index].0;
                let others_high = cells[part.clone()]
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| part.start + i != index)
                    .fold(0_u64, |acc, (_, c)| acc.saturating_add(c.1));
                if others_high != u64::MAX {
                    cells[index].0 = cells[index].0.max(total.0.saturating_sub(others_high));
                }
                if total.1 != u64::MAX {
                    cells[index].1 = cells[index].1.min(total.1.saturating_sub(others_low));
                }
            }
        }
    }
    cells.into_iter().zip(printed).collect()
}

/// Protects: small-count suppression cannot be undone by subtraction, even through two
/// breakdowns of one total. Each fixture has one hidden length cell; a reader who combines
/// the length breakdown, the datatype breakdown and the total learns nothing about any
/// hidden cell beyond its printed form. The neighbour, with every cell at or above the
/// threshold, prints all of them and the cells add up.
#[test]
fn hidden_cells_are_not_recoverable_from_two_breakdowns_of_one_total() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    // Four one-byte strings (the hidden bin), one two-byte string, five two-byte integers.
    let first = write_file(
        dir.path(),
        "first.nq",
        &literal_fixture(
            &["a", "b", "c", "d", "xy"],
            &["10", "11", "12", "13", "14"],
            false,
        ),
    );
    // Five one-byte strings, three two-byte strings (a hidden string count), one integer.
    let second = write_file(
        dir.path(),
        "second.nq",
        &literal_fixture(&["a", "b", "c", "d", "e", "aa", "bb", "cc"], &["10"], false),
    );
    for input in [&first, &second] {
        let out = shape(&["--no-labelling", input]);
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        let document = purrdf_lex::json::read(&stdout(&out)).expect("one JSON document");
        let mut hidden = 0;
        for (learned, printed) in learned(&document) {
            if printed.1 != 5 - 1 && printed.1 != u64::MAX {
                continue; // exact
            }
            hidden += 1;
            if printed.1 == 4 {
                // A "<5" cell still ranges over one to four.
                assert_eq!(learned, printed, "{}", stdout(&out));
            } else {
                // A withheld or lower-bounded cell still has no upper bound.
                assert_eq!(learned.1, u64::MAX, "{}", stdout(&out));
            }
        }
        assert!(hidden >= 1, "the fixture hides a cell: {}", stdout(&out));
        assert!(document["counts"]["distinct_literals"].as_u64().is_none());
    }

    let all_exact = write_file(
        dir.path(),
        "all-exact.nq",
        &literal_fixture(
            &["a", "b", "c", "d", "e"],
            &["10", "11", "12", "13", "14"],
            false,
        ),
    );
    let out = shape(&["--no-labelling", &all_exact]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let document = purrdf_lex::json::read(&stdout(&out)).expect("one JSON document");
    let bins = document["literal_length_bytes_histogram"]
        .as_array()
        .expect("bins");
    let sum: u64 = bins
        .iter()
        .map(|bin| bin["count"].as_u64().expect("exact"))
        .sum();
    assert_eq!(
        exact(&document, &["counts", "distinct_literals"]),
        Some(sum)
    );
    assert_eq!(
        exact(&document, &["datatypes", "named_xsd", "string"]),
        Some(5)
    );
    assert_eq!(
        exact(&document, &["datatypes", "named_xsd", "integer"]),
        Some(5)
    );
}

/// Protects: the four XSD datatypes RDF treats as compatible but `purrdf_xsd` does not model
/// are named, an unlisted XSD name is not printed, and all of them are counted.
#[test]
fn the_rdf_compatible_xsd_names_are_named_and_unknown_names_are_not_printed() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let mut text = String::new();
    for datatype in ["language", "Name", "NCName", "NMTOKEN"] {
        for index in 0..5 {
            push(
                &mut text,
                &format!(
                    "<http://example.org/s> <http://example.org/p> \"v{index}\"^^<http://www.w3.org/2001/XMLSchema#{datatype}> .\n"
                ),
            );
        }
    }
    for index in 0..5 {
        push(
            &mut text,
            &format!(
                "<http://example.org/s> <http://example.org/p> \"w{index}\"^^<http://www.w3.org/2001/XMLSchema#secretname> .\n"
            ),
        );
    }
    let input = write_file(dir.path(), "names.nq", &text);
    let out = shape(&["--no-labelling", &input]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let document = purrdf_lex::json::read(&stdout(&out)).expect("one JSON document");
    for name in ["language", "Name", "NCName", "NMTOKEN"] {
        assert_eq!(
            exact(&document, &["datatypes", "named_xsd", name]),
            Some(5),
            "{}",
            stdout(&out)
        );
    }
    assert_eq!(exact(&document, &["datatypes", "other"]), Some(5));
    assert!(!stdout(&out).contains("secretname"));
}

/// The 13-node quadratic-residue circulant over blank nodes: a labelling input that is not
/// guaranteed, so a limit of zero refuses it.
fn circulant() -> String {
    let mut out = String::new();
    for node in 0..13_u32 {
        for residue in [1_u32, 3, 4, 9, 10, 12] {
            push(
                &mut out,
                &format!(
                    "_:n{node} <http://example.org/p> _:n{} .\n",
                    (node + residue) % 13
                ),
            );
        }
    }
    out
}

/// Protects: the labelling refusal is typed and content free, and the same input labels under
/// the engine's limit; a guaranteed input is not refused by the same limit.
#[test]
fn the_labelling_refusal_is_typed_and_its_neighbour_labels() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let input = write_file(dir.path(), "circulant.nq", &circulant());
    let refused = shape(&["--labelling-limit", "0", &input]);
    assert_eq!(code(&refused), 0, "{}", stderr(&refused));
    assert!(
        stdout(&refused).contains(
            "\"canonical_labelling\":{\"status\":\"refused\",\"refusal\":\"limit\",\"code\":3}"
        ),
        "{}",
        stdout(&refused)
    );
    let labelled = shape(&[&input]);
    assert_eq!(code(&labelled), 0, "{}", stderr(&labelled));
    assert!(
        stdout(&labelled)
            .contains("\"canonical_labelling\":{\"status\":\"labelled\",\"lines\":78,"),
        "{}",
        stdout(&labelled)
    );
}

/// Protects: the written case of a language tag is reported where the reader sees it written
/// (GTS), omitted with a typed reason where the reader folds it first, and a tag count that a
/// case cell would determine is not printed exactly.
#[test]
fn language_tag_case_forms_are_reported_from_gts_and_omitted_elsewhere() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let tags = [
        "en", "EN", "En", "en-us", "EN-US", "En-Us", "fr", "FR", "Fr", "de", "it", "es",
    ];
    let mut writer = purrdf_gts::writer::Writer::new("purrdf.gts");
    let mut terms = vec![
        purrdf_gts::model::Term::iri("http://example.org/s"),
        purrdf_gts::model::Term::iri("http://example.org/p"),
    ];
    let mut quads = Vec::new();
    for (index, tag) in tags.iter().enumerate() {
        terms.push(purrdf_gts::model::Term::lang_literal(
            format!("v{index}"),
            *tag,
        ));
        quads.push((0, 1, 2 + index, None));
    }
    writer.add_terms(&terms);
    writer.add_quads(&quads);
    let gts = support::path(dir.path(), "tags.gts");
    std::fs::write(&gts, writer.into_bytes()).expect("write the GTS fixture");
    let out = shape(&["--no-labelling", &gts]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let document = purrdf_lex::json::read(&stdout(&out)).expect("one JSON document");
    let forms = &document["language_tags"]["case_forms"];
    assert_eq!(forms["status"].as_str(), Some("reported"));
    // Six lower, three upper, three mixed: the upper and mixed cells are small, so the lower
    // cell is withheld with the total of the tagged literals, which would give them back.
    assert_eq!(
        forms["lower"].as_str(),
        Some("withheld"),
        "{}",
        stdout(&out)
    );
    assert_eq!(forms["upper"].as_str(), Some("<5"));
    assert_eq!(forms["mixed"].as_str(), Some("<5"));
    // The tagged-literal total would give a suppressed form cell by subtraction.
    assert_eq!(
        exact(&document, &["language_tags", "tagged_literals"]),
        None
    );

    let text = tags
        .iter()
        .enumerate()
        .map(|(index, tag)| {
            format!("<http://example.org/s> <http://example.org/p> \"v{index}\"@{tag} .\n")
        })
        .collect::<Vec<_>>()
        .concat();
    let nq = write_file(dir.path(), "tags.nq", &text);
    let out = shape(&["--no-labelling", &nq]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(stdout(&out).contains(
        "\"case_forms\":{\"status\":\"omitted\",\"reason\":\"reader-folds-before-figure\"}"
    ));
}

/// A GTS file of `terms` language-tagged literals, one segment per slice of `segments`.
fn gts_with_tags(segments: &[&[(&str, &str)]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for rows in segments {
        let mut writer = purrdf_gts::writer::Writer::new("purrdf.gts");
        let mut terms = vec![
            purrdf_gts::model::Term::iri("http://example.org/s"),
            purrdf_gts::model::Term::iri("http://example.org/p"),
        ];
        let mut quads = Vec::new();
        for (index, (lexical, tag)) in rows.iter().enumerate() {
            terms.push(purrdf_gts::model::Term::lang_literal(*lexical, *tag));
            quads.push((0, 1, 2 + index, None));
        }
        writer.add_terms(&terms);
        writer.add_quads(&quads);
        bytes.extend_from_slice(&writer.into_bytes());
    }
    bytes
}

/// Protects: the case-form figure counts distinct literals, the unit of `tagged_literals`. A
/// literal repeated across two segments, and a literal written under two case variants of its
/// tag, each count once, so the forms add up to the tagged literals.
#[test]
fn case_forms_count_distinct_literals_across_segments_and_case_variants() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let words = ["v0", "v1", "v2", "v3", "v4", "v5"];
    let lower: Vec<(&str, &str)> = words.iter().map(|word| (*word, "en")).collect();
    let upper: Vec<(&str, &str)> = words.iter().map(|word| (*word, "EN")).collect();
    let both: Vec<(&str, &str)> = lower.iter().chain(&upper).copied().collect();
    let cases = [
        ("repeated", gts_with_tags(&[&lower, &lower]), (6, 0, 0)),
        ("variants", gts_with_tags(&[&both]), (0, 6, 0)),
        (
            "variants across segments",
            gts_with_tags(&[&lower, &upper]),
            (0, 6, 0),
        ),
    ];
    for (name, bytes, (want_lower, want_upper, want_mixed)) in cases {
        let path = support::path(dir.path(), &format!("{}.gts", name.replace(' ', "-")));
        std::fs::write(&path, bytes).expect("write the GTS fixture");
        let out = shape(&["--no-labelling", &path]);
        assert_eq!(code(&out), 0, "{name}: {}", stderr(&out));
        let document = purrdf_lex::json::read(&stdout(&out)).expect("one JSON document");
        let forms = &document["language_tags"]["case_forms"];
        let counted = (
            forms["lower"].as_u64(),
            forms["upper"].as_u64(),
            forms["mixed"].as_u64(),
        );
        assert_eq!(
            counted,
            (Some(want_lower), Some(want_upper), Some(want_mixed)),
            "{name}: {}",
            stdout(&out)
        );
        assert_eq!(
            exact(&document, &["language_tags", "tagged_literals"]),
            Some(6),
            "{name}: the forms add up to the tagged literals"
        );
    }
}
