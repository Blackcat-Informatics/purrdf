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

    // The negative control: with no base the document is REFUSED rather than parsed with
    // `<alice>` left unresolved. Silently interning the relative reference and then
    // describing nothing is the defect this behaviour replaces — an empty description is
    // indistinguishable from "the subject genuinely has no triples", so the user was told
    // nothing was there instead of that their document could not be resolved. stdin has no
    // retrieval IRI, so there is no base to fall back to.
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
    assert_eq!(
        code(&unbased),
        1,
        "a relative IRI with no base must be refused, not silently unresolved:\n{}",
        stdout(&unbased)
    );
    assert!(
        stderr(&unbased).contains("iri-relative-no-base"),
        "the refusal carries the code for the condition a base fixes:\n{}",
        stderr(&unbased)
    );
    assert!(
        stdout(&unbased).trim().is_empty(),
        "a refused parse emits no description:\n{}",
        stdout(&unbased)
    );
}

/// A RELATIVE `--iri` resolves against the base in force instead of silently matching
/// nothing.
///
/// `--iri` is command-line text with no retrieval IRI of its own, and it is compared against
/// graph terms that are absolute by the time the parser is done. So a relative selector used
/// to match nothing at all, and `Describer::describe_iris` drops a term the dataset does not
/// contain: the command exited 0 with an EMPTY document and said nothing about a required
/// argument that denoted no resource.
#[test]
fn a_relative_iri_selector_resolves_against_the_base_in_force() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "rel.ttl", "<alice> <../knows> <bob> .\n");

    // The reproduction, now answered: `--iri alice` denotes what `<alice>` in the document
    // denotes, because both resolve against the same base.
    let based = run(&[
        "describe",
        "--iri",
        "alice",
        "--from",
        "turtle",
        "--to",
        "ntriples",
        "--base",
        "http://example.org/dir/",
        &data,
    ]);
    assert_eq!(code(&based), 0, "{}", stderr(&based));
    assert_eq!(
        stdout(&based).trim_end(),
        "<http://example.org/dir/alice> <http://example.org/knows> \
         <http://example.org/dir/bob> .",
        "the selector must resolve against --base, not match nothing"
    );

    // The absolute spelling of the same request is byte-for-byte the same answer — the
    // resolution adds a spelling, it does not change what an absolute selector denotes.
    let absolute = run(&[
        "describe",
        "--iri",
        "http://example.org/dir/alice",
        "--from",
        "turtle",
        "--to",
        "ntriples",
        "--base",
        "http://example.org/dir/",
        &data,
    ]);
    assert_eq!(
        stdout(&based),
        stdout(&absolute),
        "the relative and absolute spellings must name the same resource"
    );

    // With NO `--base`, a filesystem input still has its own derived `file://` retrieval
    // IRI, and the selector resolves against THAT — the same base the document parsed under,
    // which is what makes the selector able to match at all.
    let derived = run(&[
        "describe", "--iri", "alice", "--from", "turtle", "--to", "ntriples", &data,
    ]);
    assert_eq!(code(&derived), 0, "{}", stderr(&derived));
    let retrieval =
        purrdf_cli::file_retrieval_iri(&data).expect("the fixture has a file:// retrieval IRI");
    let resolved = retrieval.replace("/rel.ttl", "/alice");
    assert!(
        stdout(&derived).contains(&format!("<{resolved}>")),
        "the derived retrieval IRI must resolve the selector too:\n{}",
        stdout(&derived)
    );
}

/// A relative `--iri` with NOTHING in scope is REFUSED, and an absolute one the graph does
/// not mention is still a legitimate empty description. The two are deliberately not
/// conflated.
///
/// A relative reference denotes no resource until it is resolved, so it is a malformed
/// request — a usage error decided before the source is read. An absolute IRI that is simply
/// absent is a well-formed question whose true answer is empty, exactly as `DESCRIBE` defines
/// it; refusing that would break "describe each of these IRIs" and contradict the library.
#[test]
fn an_unresolvable_selector_is_refused_and_an_absent_one_is_an_empty_answer() {
    let refused = pipe(
        &[
            "describe", "--iri", "alice", "--from", "ntriples", "--to", "ntriples", "-",
        ],
        "<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n",
    );
    assert_eq!(
        code(&refused),
        2,
        "a selector that denotes nothing is a usage error:\n{}",
        stderr(&refused)
    );
    assert!(
        stderr(&refused).contains("iri-relative-no-base"),
        "the refusal carries the shared code:\n{}",
        stderr(&refused)
    );
    assert!(
        stderr(&refused).contains("--iri") && stderr(&refused).contains("--base"),
        "the refusal names the argument and the remedy that applies to it:\n{}",
        stderr(&refused)
    );
    // The remedy must be one the operator can actually apply to a command-line argument —
    // the library's own `@base`/`xml:base` hint names document directives `--iri` has none of.
    assert!(
        !stderr(&refused).contains("@base"),
        "a command-line argument must not be told to add a document directive:\n{}",
        stderr(&refused)
    );
    assert!(
        stdout(&refused).trim().is_empty(),
        "a refused request emits no description:\n{}",
        stdout(&refused)
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let absent = run(&[
        "describe",
        "--iri",
        "http://example.org/nobody",
        "--to",
        "ntriples",
        &data,
    ]);
    assert_eq!(
        code(&absent),
        0,
        "an absolute IRI absent from the graph is a true empty answer:\n{}",
        stderr(&absent)
    );
    assert!(
        stdout(&absent).trim().is_empty(),
        "and the answer is empty:\n{}",
        stdout(&absent)
    );
}

/// `--base` is NEVER refused by `describe`, because `--iri` always spends it.
///
/// This combination used to be refused on the format rows alone — a pack source and an
/// N-Triples target can spend a base on neither the parse nor the serialize leg — and that
/// refusal stood on the old wiring, in which `--iri` was compared verbatim. Now that the
/// selector resolves, `describe` has a leg no format row can describe (the same shape as a
/// ShEx shape map), so refusing here would reject a base doing the one job that makes the
/// selector denote anything.
#[test]
fn base_is_never_refused_because_the_iri_selector_always_spends_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "rel.ttl", "<alice> <../knows> <bob> .\n");
    let pack = dir
        .path()
        .join("data.purrpck")
        .to_str()
        .expect("temp path")
        .to_owned();
    let packed = run(&[
        "convert",
        "--from",
        "turtle",
        "--to",
        "pack",
        "--base",
        "http://example.org/dir/",
        &data,
        &pack,
    ]);
    assert!(packed.status.success(), "{}", stderr(&packed));

    // A pack SOURCE stores fully-resolved terms and an N-Triples TARGET can express no base,
    // so neither format leg can spend it — and it is still honoured, by `--iri`.
    let from_pack = run(&[
        "describe",
        "--iri",
        "alice",
        "--to",
        "ntriples",
        "--base",
        "http://example.org/dir/",
        &pack,
    ]);
    assert_eq!(
        code(&from_pack),
        0,
        "the pack lane's base is spent by --iri: {}",
        stderr(&from_pack)
    );
    assert!(
        stdout(&from_pack).contains("<http://example.org/dir/alice>"),
        "and it really resolved the selector:\n{}",
        stdout(&from_pack)
    );

    // The same for an absolute-only source AND an absolute-only target: the combination the
    // format rows call inert is exactly the one `--iri` makes live.
    let ntriples = write_file(
        dir.path(),
        "data.nt",
        "<http://example.org/dir/alice> <http://example.org/p> <http://example.org/bob> .\n",
    );
    let both_inert = run(&[
        "describe",
        "--iri",
        "alice",
        "--from",
        "ntriples",
        "--to",
        "ntriples",
        "--base",
        "http://example.org/dir/",
        &ntriples,
    ]);
    assert_eq!(
        code(&both_inert),
        0,
        "neither format leg can spend it, but --iri can: {}",
        stderr(&both_inert)
    );
    assert!(
        stdout(&both_inert).contains("<http://example.org/dir/alice>"),
        "and the selector resolved:\n{}",
        stdout(&both_inert)
    );
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
