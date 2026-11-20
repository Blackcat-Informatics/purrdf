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
            "\"case_forms\":{\"status\":\"omitted\",\"reason\":\"text-reader-records-no-census\"}},",
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

/// The `case_forms` object of a document.
fn case_forms(document: &str) -> &str {
    let start = document
        .find("\"case_forms\":")
        .expect("a case_forms figure");
    let end = start + document[start..].find('}').expect("its closing brace") + 1;
    &document[start..end]
}

/// The reader a document came from.
#[derive(Clone, Copy)]
enum Reader {
    Text,
    Gts,
    Pack,
}

/// The `case_forms` object a text reader prints.
const TEXT_OMITTED: &str =
    "\"case_forms\":{\"status\":\"omitted\",\"reason\":\"text-reader-records-no-census\"}";

/// Assert that `document` equals the text-syntax `reference` in every figure but
/// `case_forms`, and that `case_forms` is what its reader gives: a reported figure for GTS,
/// and a typed omission with its own reason for a text syntax and for a pack.
fn assert_same_but_case_forms(document: &str, reference: &str, reader: Reader) {
    assert_eq!(case_forms(reference), TEXT_OMITTED, "{reference}");
    match reader {
        Reader::Gts => assert!(
            case_forms(document).starts_with("\"case_forms\":{\"status\":\"reported\",")
                || case_forms(document)
                    == "\"case_forms\":{\"status\":\"omitted\",\"reason\":\"small-counts-would-be-exposed\"}",
            "{document}"
        ),
        Reader::Text => assert_eq!(case_forms(document), TEXT_OMITTED, "{document}"),
        Reader::Pack => assert_eq!(
            case_forms(document),
            "\"case_forms\":{\"status\":\"omitted\",\"reason\":\"pack-holds-only-folded-tag\"}",
            "{document}"
        ),
    }
    assert_eq!(
        document.replace(case_forms(document), ""),
        reference.replace(TEXT_OMITTED, ""),
        "every other figure is equal"
    );
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
    for (other, reader) in [(&ttl, Reader::Text), (&gts, Reader::Gts)] {
        let out = shape(&[other]);
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        assert_same_but_case_forms(&stdout(&out), &stdout(&from_nquads), reader);
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
    assert_same_but_case_forms(&stdout(&out), &stdout(&from_nquads), Reader::Pack);
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
    for (other, reader) in [(&trig, Reader::Text), (&gts, Reader::Gts)] {
        let out = shape(&[other]);
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        assert_same_but_case_forms(&stdout(&out), &stdout(&from_nquads), reader);
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
fn interval(value: &purrdf_lex::json::Value) -> Option<(i128, i128)> {
    const OPEN: i128 = i128::MAX / 4;
    if let Some(n) = value.as_u64() {
        return Some((i128::from(n), i128::from(n)));
    }
    Some(match value.as_str()? {
        "<5" => (1, 4),
        "withheld" => (5, OPEN),
        other => (other.strip_prefix(">=")?.parse().ok()?, OPEN),
    })
}

/// A tie between printed counts: the sum of `lesser` plus `constant` is at most the sum of
/// `greater`, each a list of `(coefficient, variable)`.
type Tie = (Vec<(i128, usize)>, i128, Vec<(i128, usize)>);

/// What a reader who knows the structure of an RDF 1.2 dataset learns from a document: every
/// printed count as an interval, tightened by the sums that tie a total to its breakdowns and by
/// the facts that bound one count by others, until nothing changes. A count is recoverable
/// when it is hidden and fewer values remain possible for it than its printed form leaves: three
/// for `"<5"`, two for a withheld or lower-bounded count. The facts are those of the verb's
/// documentation: a literal is the object of a statement, an annotation or a triple term; a
/// parent is a blank node with a statement of its own; a group of like children needs that
/// many blank nodes besides its parent; a triple term nests as deep as it has terms; an
/// identifier occupies at most the positions that name it.
fn nothing_recoverable(document: &purrdf_lex::json::Value) -> bool {
    const OPEN: i128 = i128::MAX / 4;
    let mut names: Vec<String> = Vec::new();
    let mut bounds: Vec<(i128, i128)> = Vec::new();
    let mut add = |name: &str, value: &purrdf_lex::json::Value| -> Option<usize> {
        let found = interval(value).or_else(|| value["status"].as_str().map(|_| (0, 0)))?;
        names.push(name.to_owned());
        bounds.push(found);
        Some(bounds.len() - 1)
    };
    let counts = &document["counts"];
    let quads = add("quads", &counts["quads"]).expect("quads");
    let iris = add("iris", &counts["distinct_iris"]).expect("iris");
    let literals = add("literals", &counts["distinct_literals"]).expect("literals");
    let blanks = add("blanks", &counts["blank_nodes"]).expect("blanks");
    let graphs = add("graphs", &counts["named_graphs"]).expect("graphs");
    let triples = add("triples", &counts["triple_terms"]).expect("triples");
    let reifiers = add("reifiers", &counts["reifiers"]).expect("reifiers");
    let annotations = add("annotations", &counts["annotations"]).expect("annotations");
    let depth = add("depth", &document["max_triple_term_depth"]).expect("depth");
    let siblings = &document["blank_siblings"];
    let parents = add("parents", &siblings["parents"]).expect("parents");
    let group = add("group", &siblings["max_largest_like_group"]).expect("group");
    let tags_doc = &document["language_tags"];
    let tags = add("tags", &tags_doc["distinct_tags"]).expect("tags");
    let tagged = add("tagged", &tags_doc["tagged_literals"]).expect("tagged");
    let datatypes = &document["datatypes"];
    let other = add("other", &datatypes["other"]).expect("other");
    let mut sums: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut by_length = Vec::new();
    for bin in document["literal_length_bytes_histogram"]
        .as_array()
        .map_or(&[][..], |bins| &bins[..])
    {
        by_length.push(add("length", &bin["count"]).expect("bin"));
    }
    if !by_length.is_empty() {
        sums.push((literals, by_length));
    }
    let mut by_datatype = vec![other, tagged];
    for name in ["integer", "string"] {
        if let Some(index) = add(name, &datatypes["named_xsd"][name]) {
            by_datatype.push(index);
        }
    }
    if datatypes["named_xsd"].as_object().is_some()
        && datatypes["named_xsd"]["status"].as_str().is_none()
    {
        sums.push((literals, by_datatype));
    }
    let mut by_group = Vec::new();
    for bin in siblings["largest_like_group_histogram"]
        .as_array()
        .map_or(&[][..], |bins| &bins[..])
    {
        by_group.push(add("group bin", &bin["count"]).expect("bin"));
    }
    if !by_group.is_empty() {
        sums.push((parents, by_group));
    }
    let forms = &tags_doc["case_forms"];
    if forms["status"].as_str() == Some("reported") {
        let by_form: Vec<usize> = ["lower", "upper", "mixed"]
            .iter()
            .map(|key| add(key, &forms[*key]).expect("form"))
            .collect();
        sums.push((tagged, by_form));
    }
    let mut ties: Vec<Tie> = Vec::new();
    let one = |v: usize| (1_i128, v);
    ties.push((
        vec![one(literals)],
        0,
        vec![one(quads), one(annotations), one(triples)],
    ));
    ties.push((vec![one(tagged)], 0, vec![one(literals)]));
    ties.push((vec![one(tags)], 0, vec![one(tagged)]));
    ties.push((vec![one(parents)], 0, vec![one(blanks)]));
    ties.push((vec![one(parents)], 0, vec![one(quads)]));
    ties.push((vec![one(depth)], 0, vec![one(triples)]));
    ties.push((
        vec![one(blanks)],
        0,
        vec![
            (3, quads),
            (2, annotations),
            (3, triples),
            one(reifiers),
            one(graphs),
        ],
    ));
    ties.push((
        vec![one(iris)],
        0,
        vec![
            (4, quads),
            (3, annotations),
            (3, triples),
            (2, reifiers),
            one(graphs),
        ],
    ));
    if bounds[group].0 > 0 {
        ties.push((vec![one(group)], 1, vec![one(blanks)]));
        ties.push((vec![one(group)], 0, vec![one(quads)]));
    }
    for (total, cells) in &sums {
        let cells: Vec<(i128, usize)> = cells.iter().map(|c| (1, *c)).collect();
        ties.push((cells.clone(), 0, vec![(1, *total)]));
        ties.push((vec![(1, *total)], 0, cells));
    }
    let printed = bounds.clone();
    for _ in 0..200 {
        let mut changed = false;
        for (lesser, constant, greater) in &ties {
            // sum(lesser) - sum(greater) <= -constant
            let terms: Vec<(i128, usize)> = lesser
                .iter()
                .copied()
                .chain(greater.iter().map(|(c, v)| (-c, *v)))
                .collect();
            for (k, (coefficient, variable)) in terms.iter().enumerate() {
                let mut rest = 0_i128;
                let mut open = false;
                for (j, (a, v)) in terms.iter().enumerate() {
                    if j == k {
                        continue;
                    }
                    if *a > 0 {
                        rest += a * bounds[*v].0;
                    } else if bounds[*v].1 >= OPEN {
                        open = true;
                    } else {
                        rest += a * bounds[*v].1;
                    }
                }
                if open {
                    continue;
                }
                let room = -constant - rest;
                if *coefficient > 0 {
                    let high = room.div_euclid(*coefficient);
                    if high < bounds[*variable].1 {
                        bounds[*variable].1 = high;
                        changed = true;
                    }
                } else {
                    let low = -room.div_euclid(-*coefficient);
                    if low > bounds[*variable].0 {
                        bounds[*variable].0 = low;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    printed
        .iter()
        .zip(&bounds)
        .zip(&names)
        .all(|((print, learned), name)| {
            let values = learned.1 - learned.0 + 1;
            let ok = if print.0 == print.1 {
                true
            } else if print.1 == 4 {
                learned.1 == 4 && values >= 3
            } else {
                values >= 2
            };
            assert!(ok || values > 0, "{name}: inconsistent");
            ok
        })
}

/// Protects: small-count suppression cannot be undone by subtraction, even through two
/// breakdowns of one total. Each fixture hides a count; a reader who combines every printed
/// count and every structural tie between them cannot narrow a hidden one. The neighbour, with
/// every count at or above the threshold, prints all of them and they add up.
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
        let printed = stdout(&out);
        let document = purrdf_lex::json::read(&printed).expect("one JSON document");
        assert!(nothing_recoverable(&document), "{printed}");
        assert!(
            printed.contains("\"<5\"") || printed.contains("withheld"),
            "the fixture hides a count: {printed}"
        );
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
    assert!(nothing_recoverable(&document));
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

/// Protects: a small dataset of integers prints every figure group or a typed omission of the
/// group, and never refuses the document or lets the statement count bound a hidden literal
/// count. The refused or pinned shapes are `1 10 11 12 13`, `1 2 10 11 12`, `10..13 1000..1003`
/// and `1 10..14` (six statements, so six literals); each is beside a dataset whose counts are
/// all at or above the threshold and print exactly.
#[test]
fn small_integer_datasets_print_without_pinning_or_refusing() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let shapes: [(&str, &[&str]); 4] = [
        ("j", &["1", "10", "11", "12", "13"]),
        ("k", &["1", "2", "10", "11", "12"]),
        (
            "m",
            &["10", "11", "12", "13", "1000", "1001", "1002", "1003"],
        ),
        ("l", &["1", "10", "11", "12", "13", "14"]),
    ];
    for (name, integers) in shapes {
        let input = write_file(
            dir.path(),
            &format!("{name}.nq"),
            &literal_fixture(&[], integers, false),
        );
        let out = shape(&["--no-labelling", &input]);
        assert_eq!(code(&out), 0, "{name}: {}", stderr(&out));
        let printed = stdout(&out);
        let document = purrdf_lex::json::read(&printed).expect("one JSON document");
        assert!(nothing_recoverable(&document), "{name}: {printed}");
    }
    let exact_neighbour = write_file(
        dir.path(),
        "neighbour.nq",
        &literal_fixture(
            &[],
            &["1", "2", "3", "4", "5", "10", "11", "12", "13", "14"],
            false,
        ),
    );
    let out = shape(&["--no-labelling", &exact_neighbour]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let document = purrdf_lex::json::read(&stdout(&out)).expect("one JSON document");
    assert!(nothing_recoverable(&document));
    assert_eq!(exact(&document, &["counts", "quads"]), Some(10));
    assert_eq!(exact(&document, &["counts", "distinct_literals"]), Some(10));
    assert_eq!(
        exact(&document, &["datatypes", "named_xsd", "integer"]),
        Some(10)
    );
}

/// Protects: the parent count and the largest like group do not pin the blank-node count. One
/// parent with three like children needs four blank nodes, so a hidden blank-node count and an
/// exact group size would give it exactly; the neighbour has every count at or above the
/// threshold and prints them exactly.
#[test]
fn a_like_group_does_not_pin_the_blank_node_count() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let mut small = String::new();
    for child in 0..3 {
        push(
            &mut small,
            &format!(
                "_:parent <http://example.org/has> _:child{child} .\n_:child{child} <http://example.org/v> \"same\" .\n"
            ),
        );
    }
    let small = write_file(dir.path(), "small.nq", &small);
    let mut large = String::new();
    for parent in 0..5 {
        for child in 0..5 {
            push(
                &mut large,
                &format!(
                    "_:parent{parent} <http://example.org/has> _:child{parent}x{child} .\n_:child{parent}x{child} <http://example.org/v> \"same\" .\n"
                ),
            );
        }
    }
    let large = write_file(dir.path(), "large.nq", &large);
    for (input, exact_counts) in [(&small, false), (&large, true)] {
        let out = shape(&["--no-labelling", input]);
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        let printed = stdout(&out);
        let document = purrdf_lex::json::read(&printed).expect("one JSON document");
        assert!(nothing_recoverable(&document), "{printed}");
        assert_eq!(
            exact(&document, &["counts", "blank_nodes"]).is_some(),
            exact_counts,
            "{printed}"
        );
    }
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

/// A GTS file of `integers` xsd:integer literals followed by `tagged` language-tagged
/// literals, in one segment.
fn gts_with_literals(integers: &[&str], tagged: &[(&str, &str)]) -> Vec<u8> {
    let mut writer = purrdf_gts::writer::Writer::new("purrdf.gts");
    let mut terms = vec![
        purrdf_gts::model::Term::iri("http://example.org/s"),
        purrdf_gts::model::Term::iri("http://example.org/p"),
        purrdf_gts::model::Term::iri("http://www.w3.org/2001/XMLSchema#integer"),
    ];
    for lexical in integers {
        terms.push(purrdf_gts::model::Term::literal(*lexical, Some(2)));
    }
    for (lexical, tag) in tagged {
        terms.push(purrdf_gts::model::Term::lang_literal(*lexical, *tag));
    }
    let quads: Vec<_> = (3..terms.len())
        .map(|object| (0, 1, object, None))
        .collect();
    writer.add_terms(&terms);
    writer.add_quads(&quads);
    writer.into_bytes()
}

/// Protects: the case forms never let a reader recover a small count. A GTS document either
/// reports them with nothing hidden that the figures pin, or omits them for suppression. Case A
/// is five `en` tags and one `EN`; case C is four integers beside eight lower-case tags, where
/// the main figures already withhold the tagged-literal count. Each is beside a neighbour that
/// still reports its forms exactly.
#[test]
fn case_forms_never_expose_a_small_count() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let words = [
        "v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8", "v9", "v10", "v11", "v12", "v13",
        "v14", "v15", "v16",
    ];
    let tagged = |tags: &[&'static str]| -> Vec<(&'static str, &'static str)> {
        tags.iter()
            .enumerate()
            .map(|(index, tag)| (words[index], *tag))
            .collect()
    };
    let exposed = "{\"status\":\"omitted\",\"reason\":\"small-counts-would-be-exposed\"}";
    // What each case must print: the forms omitted for suppression, reported exactly, or never
    // as the exact count the other figures would otherwise pin.
    let cases: [(&str, Vec<u8>, &str); 4] = [
        (
            "a-five-en-one-EN",
            gts_with_literals(&[], &tagged(&["en", "en", "en", "en", "en", "EN"])),
            "omitted",
        ),
        (
            "a-neighbour",
            gts_with_literals(
                &[],
                &tagged(&[
                    "en", "en", "en", "en", "en", "en", "EN", "EN", "EN", "EN", "EN", "EN", "En",
                    "En", "En", "En", "En",
                ]),
            ),
            "reported",
        ),
        (
            "c-four-integers-eight-tagged",
            gts_with_literals(
                &["10", "11", "12", "13"],
                &tagged(&["en", "en", "en", "en", "en", "en", "en", "en"]),
            ),
            "hidden",
        ),
        (
            "c-neighbour",
            gts_with_literals(
                &["10", "11", "12", "13", "14"],
                &tagged(&["en", "en", "en", "en", "en", "en", "en", "en"]),
            ),
            "reported",
        ),
    ];
    for (name, bytes, expected) in cases {
        let path = support::path(dir.path(), &format!("{name}.gts"));
        std::fs::write(&path, bytes).expect("write the GTS fixture");
        let out = shape(&["--no-labelling", &path]);
        assert_eq!(code(&out), 0, "{name}: {}", stderr(&out));
        let printed = stdout(&out);
        let document = purrdf_lex::json::read(&printed).expect("one JSON document");
        assert!(nothing_recoverable(&document), "{name}: {printed}");
        let forms = &document["language_tags"]["case_forms"];
        match expected {
            "reported" => {
                assert_eq!(
                    forms["status"].as_str(),
                    Some("reported"),
                    "{name}: {printed}"
                );
                assert!(forms["lower"].as_u64().is_some(), "{name}: {printed}");
                if name == "a-neighbour" {
                    // Different upper and mixed counts, so each cell renders in its own slot.
                    assert_eq!(
                        (
                            forms["lower"].as_u64(),
                            forms["upper"].as_u64(),
                            forms["mixed"].as_u64()
                        ),
                        (Some(6), Some(6), Some(5)),
                        "{printed}"
                    );
                }
            }
            "omitted" => assert!(printed.contains(exposed), "{name}: {printed}"),
            _ => assert!(
                forms["lower"].as_u64().is_none(),
                "{name}: the lower count is not printed exactly: {printed}"
            ),
        }
    }
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

/// Protects: reporting the case forms changes no other figure. A GTS file with upper-case and
/// mixed-case tags, whose small form cells print as `"<5"` beside a withheld lower cell, gives
/// the document of its text twin in every figure but `case_forms`.
#[test]
fn upper_and_mixed_case_gts_matches_its_text_twin_but_for_case_forms() {
    let dir = purrdf_testkit::temp_dir!().expect("temporary inputs");
    let rows: Vec<(String, &str)> = [
        "en", "en", "en", "en", "en", "en", "EN", "EN", "En-US", "En-US", "en-us", "fr",
    ]
    .iter()
    .enumerate()
    .map(|(index, tag)| (format!("v{index}"), *tag))
    .collect();
    let borrowed: Vec<(&str, &str)> = rows.iter().map(|(l, t)| (l.as_str(), *t)).collect();
    let gts = support::path(dir.path(), "cased.gts");
    std::fs::write(&gts, gts_with_tags(&[&borrowed])).expect("write the GTS fixture");
    let text: String = rows
        .iter()
        .map(|(lexical, tag)| {
            format!("<http://example.org/s> <http://example.org/p> \"{lexical}\"@{tag} .\n")
        })
        .collect::<Vec<_>>()
        .concat();
    let nq = write_file(dir.path(), "cased.nq", &text);
    let from_text = shape(&["--no-labelling", &nq]);
    assert_eq!(code(&from_text), 0, "{}", stderr(&from_text));
    let from_gts = shape(&["--no-labelling", &gts]);
    assert_eq!(code(&from_gts), 0, "{}", stderr(&from_gts));
    assert_same_but_case_forms(&stdout(&from_gts), &stdout(&from_text), Reader::Gts);
    // The small upper and mixed cells keep their range beside the exact tagged-literal count
    // because the lower cell is withheld, so the forms print and no other figure moves.
    let forms = case_forms(&stdout(&from_gts)).to_owned();
    assert!(forms.contains("\"lower\":\"withheld\""), "{forms}");
    assert!(forms.contains("\"upper\":\"<5\""), "{forms}");
    assert!(forms.contains("\"mixed\":\"<5\""), "{forms}");
}
