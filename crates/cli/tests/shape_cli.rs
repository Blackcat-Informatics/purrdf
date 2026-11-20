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
            "\"case_forms\":{\"status\":\"omitted\",\"reason\":\"language-tag-case-not-retained\"}},",
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

/// Protects: the figures belong to the dataset, not to the syntax it arrived in.
#[test]
fn one_dataset_gives_one_document_from_turtle_nquads_and_gts() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let (ttl, nq, gts) = fixture_files(dir.path());
    let from_nquads = shape(&[&nq]);
    assert_eq!(code(&from_nquads), 0, "{}", stderr(&from_nquads));
    for other in [&ttl, &gts] {
        let out = shape(&[other]);
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        assert_eq!(stdout(&out), stdout(&from_nquads));
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
