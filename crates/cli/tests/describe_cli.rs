// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `describe` coverage that drives the BUILT `purrdf` binary
//! (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so every assertion pins the shipped
//! executable's Symmetric Concise Bounded Description surface.
//!
//! ## What is pinned here
//!
//! * the description is SYMMETRIC (incoming links, not only outgoing), closes blank nodes
//!   transitively, and does NOT expand named-node neighbours;
//! * it carries the RDF 1.2 statement layer — the reifiers about the subject and their
//!   annotations — and a target syntax that cannot hold them records the drop in the loss
//!   ledger rather than losing it silently;
//! * `describe --iri X` is BYTE-IDENTICAL to `query 'DESCRIBE <X>'`, which is the assertion
//!   that this verb reaches the shared `Describer` rather than re-deriving a second walk;
//! * every native source syntax and the pack container reach the same description;
//! * an absent subject is an empty description and exit **0**, not a failure;
//! * `--base`, `--from`/`--to`, stdin/stdout and the RDF-emitting global flags behave exactly
//!   as they do for `convert`/`reason`.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// A `Command` for the built `purrdf` binary.
fn purrdf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_purrdf"))
}

/// Run `purrdf` with `args`, returning the captured [`Output`].
fn run(args: &[&str]) -> Output {
    purrdf()
        .args(args)
        .output()
        .expect("spawn the built purrdf binary")
}

/// Run `purrdf` with `args`, writing `stdin_bytes` to its standard input.
fn pipe(args: &[&str], stdin_bytes: &str) -> Output {
    let mut child = purrdf()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn purrdf");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(stdin_bytes.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait for purrdf")
}

/// stdout of an [`Output`] as a `String`.
fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// stderr of an [`Output`] as a `String`.
fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The exit code of an [`Output`].
fn code(out: &Output) -> i32 {
    out.status.code().expect("the process exited normally")
}

/// Write `contents` to `dir/name`, returning the path as a `String`.
fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, contents).expect("write fixture file");
    p.to_str().expect("temp path is valid UTF-8").to_owned()
}

/// A graph exercising every SCBD rule at once:
///
/// * `alice → bob` is OUTGOING from the subject;
/// * `carol → alice` is INCOMING (a plain CBD would miss it);
/// * `alice → _:addr → "Springfield"` is a blank-node closure that must ride along in full;
/// * `bob → dave` hangs off a NAMED neighbour and must NOT be pulled in.
const DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice ex:knows ex:bob ; ex:address [ ex:city \"Springfield\" ] .\n",
    "ex:carol ex:knows ex:alice .\n",
    "ex:bob ex:knows ex:dave .\n",
    "ex:erin ex:knows ex:frank .\n",
);

/// A statement-layer document: one annotated triple about `alice`.
const STAR_DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice ex:knows ex:bob {| ex:certainty 0.9 ; ex:source ex:census |} .\n",
    "ex:erin ex:knows ex:frank .\n",
);

/// The Symmetric CBD keeps outgoing AND incoming arcs, closes blank nodes, and stops at named
/// neighbours.
#[test]
fn the_description_is_a_symmetric_concise_bounded_description() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&[
        "describe",
        "--iri",
        "http://example.org/alice",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));

    let body = stdout(&out);
    assert!(
        body.contains(
            "<http://example.org/alice> <http://example.org/knows> \
                       <http://example.org/bob> ."
        ),
        "the outgoing arc:\n{body}"
    );
    assert!(
        body.contains(
            "<http://example.org/carol> <http://example.org/knows> \
                       <http://example.org/alice> ."
        ),
        "the INCOMING arc — a forward-only CBD would drop it:\n{body}"
    );
    assert!(
        body.contains("\"Springfield\""),
        "the blank-node closure rides along in full:\n{body}"
    );
    assert!(
        !body.contains("<http://example.org/dave>"),
        "a NAMED neighbour does not expand — that would drag in the graph:\n{body}"
    );
    assert!(
        !body.contains("<http://example.org/erin>"),
        "an unrelated subject is not described:\n{body}"
    );
}

/// `describe --iri X` and `query 'DESCRIBE <X>'` produce BYTE-IDENTICAL output.
///
/// This is the assertion that matters most: it says the verb reaches the shared
/// `purrdf_core::describe::Describer` that SPARQL `DESCRIBE` evaluates to, rather than being a
/// second walk that happens to agree today. A divergence here would mean two definitions of
/// "describe" in one binary.
#[test]
fn describe_agrees_byte_for_byte_with_sparql_describe() {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, source) in [("plain.ttl", DATA), ("star.ttl", STAR_DATA)] {
        let data = write_file(dir.path(), name, source);

        let verb = run(&[
            "describe",
            "--iri",
            "http://example.org/alice",
            "--to",
            "ntriples",
            &data,
        ]);
        assert_eq!(code(&verb), 0, "`{name}`: {}", stderr(&verb));

        let sparql = run(&[
            "query",
            "--data",
            &data,
            "--results-format",
            "ntriples",
            "DESCRIBE <http://example.org/alice>",
        ]);
        assert!(sparql.status.success(), "`{name}`: {}", stderr(&sparql));

        assert_eq!(
            stdout(&verb),
            stdout(&sparql),
            "`{name}`: one authority, one description"
        );
        assert!(
            !stdout(&verb).is_empty(),
            "`{name}`: the agreement must not be two empty outputs"
        );
    }
}

/// The obvious SPARQL route FAILS without an explicit `--results-format`, and `describe` does
/// not — which is one of the three reasons the verb exists rather than being sugar.
#[test]
fn the_verb_needs_no_serialization_incantation_the_query_route_does() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let out_path = dir
        .path()
        .join("alice.ttl")
        .to_str()
        .expect("temp path")
        .to_owned();

    // `query`'s `--results-format` defaults to `json`, which is a SPARQL-results
    // serialization; a DESCRIBE produces a GRAPH, so the natural invocation is an error.
    let sparql = run(&[
        "query",
        "--data",
        &data,
        "DESCRIBE <http://example.org/alice>",
    ]);
    assert_ne!(
        code(&sparql),
        0,
        "the un-hinted SPARQL route must still fail, or this verb's premise is stale:\n{}",
        stdout(&sparql)
    );

    // `describe` resolves the target format from `OUT`'s extension, exactly as `convert` does.
    let verb = run(&[
        "describe",
        "--iri",
        "http://example.org/alice",
        &data,
        &out_path,
    ]);
    assert_eq!(code(&verb), 0, "{}", stderr(&verb));
    let written = std::fs::read_to_string(&out_path).expect("description written");
    assert!(written.contains("http://example.org/bob"), "{written}");
}

/// Several `--iri` values are described as ONE union subgraph.
#[test]
fn several_subjects_are_described_as_one_union() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&[
        "describe",
        "--iri",
        "http://example.org/alice",
        "--iri",
        "http://example.org/erin",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let body = stdout(&out);
    assert!(
        body.contains("<http://example.org/carol>"),
        "alice's half:\n{body}"
    );
    assert!(
        body.contains("<http://example.org/frank>"),
        "erin's half:\n{body}"
    );

    // And it is the union of the two, not a concatenation of two documents.
    assert_eq!(
        body.matches(
            "<http://example.org/erin> <http://example.org/knows> \
                      <http://example.org/frank> ."
        )
        .count(),
        1,
        "each quad appears once:\n{body}"
    );
}

/// An absent subject is an empty description and exit 0 — a term may legitimately carry no
/// asserted or incoming triples, and "nothing describes it" is a true answer.
#[test]
fn an_absent_subject_is_an_empty_description() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&[
        "describe",
        "--iri",
        "http://example.org/nobody",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(
        code(&out),
        0,
        "an absent subject is not a failure: {}",
        stderr(&out)
    );
    assert!(
        stdout(&out).trim().is_empty(),
        "the description is empty:\n{}",
        stdout(&out)
    );
}

/// EVERY native source syntax, plus the pack container, reaches the same description.
#[test]
fn every_native_source_format_reaches_the_same_description() {
    let dir = tempfile::tempdir().expect("tempdir");
    let seed = write_file(dir.path(), "seed.ttl", DATA);
    let baseline = run(&[
        "describe",
        "--iri",
        "http://example.org/alice",
        "--to",
        "ntriples",
        &seed,
    ]);
    assert_eq!(code(&baseline), 0, "{}", stderr(&baseline));
    let expected = stdout(&baseline);
    assert!(!expected.is_empty());

    for (token, extension) in [
        ("turtle", "ttl"),
        ("trig", "trig"),
        ("ntriples", "nt"),
        ("nquads", "nq"),
        ("rdfxml", "rdf"),
        ("trix", "trix"),
        ("hextuples", "hext"),
        ("jsonld", "jsonld"),
        ("yamlld", "yamlld"),
        ("pack", "purrpck"),
    ] {
        let target = dir
            .path()
            .join(format!("source.{extension}"))
            .to_str()
            .expect("temp path is valid UTF-8")
            .to_owned();
        let converted = run(&["convert", "--from", "turtle", "--to", token, &seed, &target]);
        assert!(
            converted.status.success(),
            "seeding the `{token}` source failed: {}",
            stderr(&converted)
        );

        let out = run(&[
            "describe",
            "--iri",
            "http://example.org/alice",
            "--to",
            "ntriples",
            &target,
        ]);
        assert_eq!(code(&out), 0, "`{token}`: {}", stderr(&out));
        assert_eq!(
            stdout(&out),
            expected,
            "`{token}` must describe the same subgraph"
        );
    }
}

/// The RDF 1.2 statement layer is part of the description: the reifiers whose reified triple
/// touches the closure ride along with their annotations.
#[test]
fn the_rdf12_statement_layer_is_described() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "star.ttl", STAR_DATA);

    let out = run(&[
        "describe",
        "--iri",
        "http://example.org/alice",
        "--to",
        "turtle",
        &data,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let body = stdout(&out);
    assert!(
        body.contains("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"),
        "the reifier binding about the subject:\n{body}"
    );
    assert!(
        body.contains("<http://example.org/certainty>") && body.contains("\"0.9\""),
        "its first annotation:\n{body}"
    );
    assert!(
        body.contains("<http://example.org/source>")
            && body.contains("<http://example.org/census>"),
        "its second annotation:\n{body}"
    );
    assert!(
        !body.contains("<http://example.org/erin>"),
        "and the unrelated subject is still excluded:\n{body}"
    );
}

/// The same description written into a star-INCAPABLE syntax records the drop in the loss
/// ledger rather than losing it silently — the concrete payoff of `describe` being an
/// RDF-emitting verb that shares `convert`'s sink.
#[test]
fn a_star_incapable_target_records_the_dropped_statement_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "star.ttl", STAR_DATA);
    let ledger_path = dir
        .path()
        .join("describe.loss.json")
        .to_str()
        .expect("temp path")
        .to_owned();

    let out = run(&[
        &format!("--loss-ledger={ledger_path}"),
        "describe",
        "--iri",
        "http://example.org/alice",
        "--to",
        "rdfxml",
        &data,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));

    let ledger: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&ledger_path).expect("ledger written"))
            .expect("the ledger is JSON");
    assert_eq!(ledger["schema_version"], 1);
    let codes: Vec<&str> = ledger["losses"]
        .as_array()
        .expect("losses is an array")
        .iter()
        .filter_map(|entry| entry["code"].as_str())
        .collect();
    assert!(
        codes.contains(&"statement-rows-dropped"),
        "the REALIZED drop is recorded, not only the contract: {ledger}"
    );
    assert!(
        codes.contains(&"rdf12-star-unrepresentable"),
        "and the contract loss for the pair: {ledger}"
    );

    // The bare form renders the same ledger to stderr.
    let to_stderr = run(&[
        "--loss-ledger",
        "describe",
        "--iri",
        "http://example.org/alice",
        "--to",
        "rdfxml",
        &data,
    ]);
    assert_eq!(code(&to_stderr), 0, "{}", stderr(&to_stderr));
    assert!(
        stderr(&to_stderr).contains("statement-rows-dropped"),
        "{}",
        stderr(&to_stderr)
    );
}

/// stdin in, stdout out: both `-` ends require their explicit format, exactly as for
/// `convert`.
#[test]
fn stdin_and_stdout_require_explicit_formats() {
    let no_from = pipe(
        &["describe", "--iri", "http://example.org/alice", "-"],
        DATA,
    );
    assert_eq!(code(&no_from), 2, "a usage error: {}", stderr(&no_from));
    assert!(stderr(&no_from).contains("--from"), "{}", stderr(&no_from));

    let no_to = pipe(
        &[
            "describe",
            "--iri",
            "http://example.org/alice",
            "--from",
            "turtle",
            "-",
        ],
        DATA,
    );
    assert_eq!(code(&no_to), 2, "a usage error: {}", stderr(&no_to));
    assert!(stderr(&no_to).contains("--to"), "{}", stderr(&no_to));

    let piped = pipe(
        &[
            "describe",
            "--iri",
            "http://example.org/alice",
            "--from",
            "turtle",
            "--to",
            "ntriples",
            "-",
            "-",
        ],
        DATA,
    );
    assert_eq!(code(&piped), 0, "{}", stderr(&piped));
    assert!(
        stdout(&piped).contains("<http://example.org/carol>"),
        "the piped description is complete:\n{}",
        stdout(&piped)
    );
}

/// `--base` resolves relative IRIs while parsing, so a relative subject is describable under
/// its resolved absolute name.
#[test]
fn base_resolves_relative_iris_while_parsing() {
    let relative = concat!(
        "@prefix ex: <http://example.org/> .\n",
        "<alice> ex:knows <bob> .\n",
    );

    let based = pipe(
        &[
            "describe",
            "--iri",
            "http://example.org/alice",
            "--from",
            "turtle",
            "--to",
            "ntriples",
            "--base",
            "http://example.org/",
            "-",
        ],
        relative,
    );
    assert_eq!(code(&based), 0, "{}", stderr(&based));
    assert!(
        stdout(&based).contains(
            "<http://example.org/alice> <http://example.org/knows> \
                                 <http://example.org/bob> ."
        ),
        "the relative subject resolved and was found:\n{}",
        stdout(&based)
    );

    // The negative control: unresolved, the subject is a different term and describes nothing.
    let unbased = pipe(
        &[
            "describe",
            "--iri",
            "http://example.org/alice",
            "--from",
            "turtle",
            "--to",
            "ntriples",
            "-",
        ],
        relative,
    );
    assert_eq!(code(&unbased), 0, "{}", stderr(&unbased));
    assert!(
        stdout(&unbased).trim().is_empty(),
        "an unresolved relative IRI is not the absolute one:\n{}",
        stdout(&unbased)
    );
}

/// `--base` with a pack source or a pack target is refused by name, exactly as for
/// `convert`/`reason`.
#[test]
fn base_with_a_pack_end_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let pack = dir
        .path()
        .join("data.purrpck")
        .to_str()
        .expect("temp path")
        .to_owned();
    let packed = run(&["convert", "--from", "turtle", "--to", "pack", &data, &pack]);
    assert!(packed.status.success(), "{}", stderr(&packed));

    let from_pack = run(&[
        "describe",
        "--iri",
        "http://example.org/alice",
        "--to",
        "ntriples",
        "--base",
        "http://example.org/",
        &pack,
    ]);
    assert_eq!(code(&from_pack), 2, "{}", stderr(&from_pack));
    assert!(
        stderr(&from_pack).contains("--base"),
        "{}",
        stderr(&from_pack)
    );

    let out_pack = dir
        .path()
        .join("out.purrpck")
        .to_str()
        .expect("temp path")
        .to_owned();
    let to_pack = run(&[
        "describe",
        "--iri",
        "http://example.org/alice",
        "--base",
        "http://example.org/",
        &data,
        &out_pack,
    ]);
    assert_eq!(code(&to_pack), 2, "{}", stderr(&to_pack));
    assert!(stderr(&to_pack).contains("--base"), "{}", stderr(&to_pack));
}

/// A description can be written into the lossless pack container and read back.
#[test]
fn a_description_can_be_written_to_a_pack() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "star.ttl", STAR_DATA);
    let pack = dir
        .path()
        .join("alice.purrpck")
        .to_str()
        .expect("temp path")
        .to_owned();

    let out = run(&[
        "describe",
        "--iri",
        "http://example.org/alice",
        &data,
        &pack,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));

    let verified = run(&["pack", "verify", &pack]);
    assert!(verified.status.success(), "{}", stderr(&verified));

    let back = run(&["convert", "--from", "pack", "--to", "turtle", &pack, "-"]);
    assert!(back.status.success(), "{}", stderr(&back));
    assert!(
        stdout(&back).contains("rdf-syntax-ns#reifies"),
        "a pack carries the described statement layer losslessly:\n{}",
        stdout(&back)
    );
}

/// `--jsonld-options` requires a JSON-LD/YAML-LD `--to`, and is refused otherwise rather than
/// silently ignored.
#[test]
fn jsonld_options_require_a_jsonld_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let options = write_file(
        dir.path(),
        "jsonld-options.json",
        r#"{"version":1,"mode":"context","prefixes":{"ex":"http://example.org/"}}"#,
    );

    let refused = run(&[
        "--jsonld-options",
        &options,
        "describe",
        "--iri",
        "http://example.org/alice",
        "--to",
        "turtle",
        &data,
    ]);
    assert_eq!(code(&refused), 2, "{}", stderr(&refused));
    assert!(
        stderr(&refused).contains("--jsonld-options"),
        "{}",
        stderr(&refused)
    );

    let accepted = run(&[
        "--jsonld-options",
        &options,
        "describe",
        "--iri",
        "http://example.org/alice",
        "--to",
        "jsonld",
        &data,
    ]);
    assert_eq!(code(&accepted), 0, "{}", stderr(&accepted));
    assert!(
        stdout(&accepted).contains("@context"),
        "the configured serializer really ran:\n{}",
        stdout(&accepted)
    );
}

/// At least one `--iri` is required: `describe` never describes "everything" by default.
#[test]
fn at_least_one_iri_is_required() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);

    let out = run(&["describe", "--to", "ntriples", &data]);
    assert_eq!(code(&out), 2, "a missing required flag is a usage error");
    assert!(stderr(&out).contains("--iri"), "{}", stderr(&out));
}

/// A malformed source is a runtime failure (exit 1) that writes nothing.
#[test]
fn a_malformed_source_is_a_runtime_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "broken.ttl", "<a> <b> .");

    let out = run(&[
        "describe",
        "--iri",
        "http://example.org/alice",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(stdout(&out).is_empty(), "no description is invented");
}
