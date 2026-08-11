// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `consistency` coverage that drives the BUILT `purrdf` binary
//! (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so every assertion pins the
//! shipped executable's OWL-Direct consistency decision, byte for byte.
//!
//! ## The fixtures
//!
//! [`ORDINARY_ONTOLOGY`] is the seventeen-triple ontology
//! `crates/validate/tests/dl_consistency_search_budget.rs` pins: one `owl:equivalentClass`
//! over two anonymous restrictions, an `owl:inverseOf`, an `rdfs:range`, one type assertion
//! — a document that once exhausted the DL search's step budget outright before the
//! clausification and search-refinement fixes this subcommand exists to make one-command
//! reproducible. Kept verbatim, including the anonymous restrictions carrying no
//! `rdf:type owl:Restriction` (legal OWL 2 RDF; the reverse mapping recognizes the shape
//! structurally), so this pins the actual reported document rather than a retyped one.
//!
//! [`INCONSISTENT_ONTOLOGY`] is `A ⊑ B`, `A ⊑ ¬B`, `a : A` — no model, the same shape
//! `crates/entail/tests/reasoner.rs` decides `Verdict::False` for the reasoner facade.
//!
//! ## What each test pins
//!
//! * the verdict line AND the certificate are both on stdout, unconditionally (no
//!   `--report` gate exists for this subcommand — see `consistency.rs`'s module doc);
//! * `true` and `false` both exit 0 — DECIDED verdicts, neither a failure;
//! * `--step-cap 1` narrows the ordinary ontology into `unknown` / `completeness
//!   budget-exhausted`, exiting 3 exactly like a governed `query` cut short;
//! * `--work-cap 1` does the same through the OTHER budget, and the certificate's
//!   `work`/`work-budget` lines say which one it was — a round is a pass rather than a
//!   unit of cost, so the two caps bound different quantities and a run can reach either;
//! * `--loss-ledger`/`--jsonld-options` are refused rather than silently ignored, since
//!   clap makes both global and this subcommand produces neither a ledger nor RDF.

use std::path::Path;
use std::process::{Command, Output};

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

/// Write `contents` to `dir/name`, returning the path.
fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, contents).expect("write fixture file");
    p.to_str().expect("temp path is valid UTF-8").to_owned()
}

/// The reported seventeen-triple ontology, verbatim — see
/// `crates/validate/tests/dl_consistency_search_budget.rs`.
const ORDINARY_ONTOLOGY: &str = r"
@prefix : <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:r owl:inverseOf :ri .
:ri rdfs:range :S .

:A owl:equivalentClass
        [
            owl:onProperty :r ;
            owl:allValuesFrom [
                owl:intersectionOf (
                    :S
                    [
                        owl:onProperty :p ;
                        owl:allValuesFrom :D
                    ]
                )
            ]
        ] ,
        [
            owl:onProperty :c ;
            owl:cardinality 1
        ] ;
    rdfs:subClassOf :S .

:a a :A .
";

/// `A ⊑ B`, `A ⊑ ¬B`, `a : A` — no model.
const INCONSISTENT_ONTOLOGY: &str = concat!(
    "@prefix : <http://example.org/> .\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    ":A a owl:Class .\n",
    ":B a owl:Class .\n",
    ":A rdfs:subClassOf :B .\n",
    ":A rdfs:subClassOf [ owl:complementOf :B ] .\n",
    ":a a :A .\n",
);

/// The same shape as [`INCONSISTENT_ONTOLOGY`] — `A ⊑ B`, `A ⊑ ¬B`, `a : A` — but every
/// class/individual term is a RELATIVE IRI (no `@prefix`, no scheme), so the document only
/// parses, and only decides `false` rather than erroring, when `--base` resolves `<A>`/`<B>`/
/// `<a>` to the SAME absolute IRIs everywhere they occur.
const RELATIVE_INCONSISTENT_ONTOLOGY: &str = concat!(
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    "<A> a owl:Class .\n",
    "<B> a owl:Class .\n",
    "<A> rdfs:subClassOf <B> .\n",
    "<A> rdfs:subClassOf [ owl:complementOf <B> ] .\n",
    "<a> a <A> .\n",
);

/// A consistent ontology decides `consistency true`, `completeness decided`, and exits 0.
///
/// Both the verdict AND the full certificate — including the peak-nodes/disjunctions/
/// peak-depth search-cost counters — are on stdout with no flag required: this is the
/// one-command reproduction the subcommand exists to give an operator.
#[test]
fn a_consistent_ontology_decides_true_and_is_fully_decided() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_file(dir.path(), "ontology.ttl", ORDINARY_ONTOLOGY);

    let out = run(&["consistency", &input]);
    assert!(out.status.success(), "consistency failed: {}", stderr(&out));
    assert_eq!(code(&out), 0, "a decided `true` exits 0");

    let text = stdout(&out);
    assert!(
        text.starts_with("consistency true\n"),
        "the verdict line leads stdout:\n{text}"
    );
    assert!(
        text.contains("\npurrdf-dl-certificate 1\n"),
        "the certificate banner follows the verdict, unconditionally:\n{text}"
    );
    assert!(
        text.contains("\ncompleteness decided\n"),
        "a decided verdict, not a truncated search:\n{text}"
    );
    assert!(
        text.contains("\npeak-nodes "),
        "peak-nodes counter:\n{text}"
    );
    assert!(
        text.contains("\ndisjunctions "),
        "disjunctions counter:\n{text}"
    );
    assert!(
        text.contains("\npeak-depth "),
        "peak-depth counter:\n{text}"
    );
    // Nothing about the trip machinery leaks to stderr on a normal decided run.
    assert!(
        stderr(&out).is_empty(),
        "unexpected stderr: {}",
        stderr(&out)
    );
}

/// An inconsistent ontology decides `consistency false` and STILL exits 0: a decided
/// `false` is not a failure of this command any more than a `false` ASK answer fails
/// `query`.
#[test]
fn an_inconsistent_ontology_decides_false_and_still_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_file(dir.path(), "inconsistent.ttl", INCONSISTENT_ONTOLOGY);

    let out = run(&["consistency", &input]);
    assert!(
        out.status.success(),
        "a decided `false` must exit 0, not fail: {}",
        stderr(&out)
    );
    assert_eq!(
        code(&out),
        0,
        "a decided `false` exits 0, exactly as `true` does"
    );

    let text = stdout(&out);
    assert!(
        text.starts_with("consistency false\n"),
        "the verdict line:\n{text}"
    );
    assert!(
        text.contains("\ncompleteness decided\n")
            || text.contains("\ncompleteness decided-within-boundaries\n"),
        "a decided refutation, not a truncated search:\n{text}"
    );
}

/// `--step-cap 1` narrows the ordinary ontology's own derived round cap so tight the
/// search cannot saturate: the verdict becomes `unknown`, the certificate says
/// `completeness budget-exhausted`, and the process exits 3 — the same code a governed
/// `query` a caller-set ceiling cut short.
#[test]
fn a_narrow_step_cap_reports_unknown_and_exits_three() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_file(dir.path(), "ontology.ttl", ORDINARY_ONTOLOGY);

    let out = run(&["consistency", "--step-cap", "1", &input]);
    assert!(
        !out.status.success(),
        "exit 3 is not process success, and this run trips the round cap"
    );
    assert_eq!(
        code(&out),
        3,
        "an unknown verdict exits 3, like a governed query cut short"
    );

    let text = stdout(&out);
    assert!(
        text.starts_with("consistency unknown\n"),
        "the verdict line:\n{text}"
    );
    assert!(
        text.contains("\ncompleteness budget-exhausted\n"),
        "the certificate names the exhausted budget in its own words:\n{text}"
    );
}

/// `--work-cap 1` narrows the OTHER budget to the same effect, and the certificate says
/// which one ran out.
///
/// Not a duplicate of the test above. The round cap bounds derivation PASSES and the work
/// cap bounds the matcher, scan, closure and clone work done inside them — an ontology can
/// make each pass enormously more expensive without taking more passes, which is the class
/// `dl_work_budget` demonstrates. Both reach the same honest three-valued answer, and the
/// four rendered budget lines are what distinguish them.
#[test]
fn a_narrow_work_cap_reports_unknown_and_exits_three() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_file(dir.path(), "ontology.ttl", ORDINARY_ONTOLOGY);

    let out = run(&["consistency", "--work-cap", "1", &input]);
    assert_eq!(
        code(&out),
        3,
        "an unknown verdict exits 3, whichever budget produced it"
    );

    let text = stdout(&out);
    assert!(
        text.starts_with(
            "consistency unknown
"
        ),
        "the verdict line:\n{text}"
    );
    assert!(
        text.contains("\ncompleteness budget-exhausted\n"),
        "the certificate names the exhausted budget in its own words:\n{text}"
    );
    assert!(
        text.contains("\nwork 1\n") && text.contains("\nwork-budget 1\n"),
        "the two work lines say WHICH cap ended the run — an exhausted search has its work \
         figure at its work budget:\n{text}"
    );
}

/// Without `--step-cap` the same ontology decides comfortably inside its own derived
/// budget (see `dl_consistency_search_budget.rs`), so narrowing is what changed the
/// answer above rather than an ontology that was always undecidable.
#[test]
fn the_same_ontology_decides_true_without_narrowing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_file(dir.path(), "ontology.ttl", ORDINARY_ONTOLOGY);

    let out = run(&["consistency", "--step-cap", "0", "--work-cap", "0", &input]);
    assert!(out.status.success(), "unnarrowed: {}", stderr(&out));
    assert_eq!(code(&out), 0);
    let text = stdout(&out);
    assert!(text.starts_with("consistency true\n"));
    assert!(
        text.contains("\nwork ") && text.contains("\nwork-budget "),
        "an unnarrowed run still reports both work figures:\n{text}"
    );
}

/// `--from`/`--base` resolve exactly as they do for `reason`/`entails`: a `.ttl`
/// extension needs no override, and `-` (stdin) requires one.
#[test]
fn stdin_requires_an_explicit_from_format() {
    use std::io::Write as _;
    use std::process::Stdio;

    let mut child = purrdf()
        .args(["consistency", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn purrdf");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(INCONSISTENT_ONTOLOGY.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait for purrdf");

    assert_eq!(code(&out), 2, "a usage error: stdin has no extension");
    assert!(
        stderr(&out).contains("--from"),
        "the refusal names the missing flag: {}",
        stderr(&out)
    );
}

/// `--from` alone resolves stdin: an ABSOLUTE-IRI document needs no `--base` to decide.
#[test]
fn from_resolves_stdin_turtle() {
    use std::io::Write as _;
    use std::process::Stdio;

    let mut child = purrdf()
        .args(["consistency", "--from", "turtle", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn purrdf");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(ORDINARY_ONTOLOGY.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait for purrdf");

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).starts_with("consistency true\n"));
}

/// `--base` resolves RELATIVE IRIs in a stdin-piped Turtle document before deciding it: without
/// it the same bytes cannot even parse (`<A>` names no scheme), and with it
/// [`RELATIVE_INCONSISTENT_ONTOLOGY`] resolves `<A>`/`<B>`/`<a>` to the SAME absolute IRIs
/// everywhere they recur and decides `false` — the one answer that is only reachable if every
/// occurrence resolved identically rather than, say, each parse call minting its own blank
/// term for an unresolved relative reference.
#[test]
fn base_resolves_relative_iris_piped_via_stdin() {
    use std::io::Write as _;
    use std::process::Stdio;

    let pipe = |args: &[&str]| -> Output {
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
            .write_all(RELATIVE_INCONSISTENT_ONTOLOGY.as_bytes())
            .expect("write stdin");
        child.wait_with_output().expect("wait for purrdf")
    };

    // Without `--base`, a relative IRI has no scheme to be absolute with — the negative
    // control that keeps the assertion below from passing by accident.
    let unbased = pipe(&["consistency", "--from", "turtle", "-"]);
    assert_eq!(
        code(&unbased),
        1,
        "a relative IRI with no --base must fail to parse: {}",
        stderr(&unbased)
    );
    assert!(
        stderr(&unbased).contains("absolute"),
        "the refusal names what a relative IRI is missing: {}",
        stderr(&unbased)
    );

    let based = pipe(&[
        "consistency",
        "--from",
        "turtle",
        "--base",
        "https://example.org/",
        "-",
    ]);
    assert!(based.status.success(), "{}", stderr(&based));
    assert!(
        stdout(&based).starts_with("consistency false\n"),
        "resolved relative IRIs must decide the same inconsistency as \
         INCONSISTENT_ONTOLOGY's absolute ones: {}",
        stdout(&based)
    );
}

/// N-Triples needs no `--from` override either — the sibling verbs' default format
/// coverage, over the same ontology re-expressed as one graph.
#[test]
fn ntriples_input_is_inferred_from_the_extension() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_file(
        dir.path(),
        "a.nt",
        concat!(
            "<http://example.org/A> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ",
            "<http://www.w3.org/2002/07/owl#Class> .\n",
        ),
    );

    let out = run(&["consistency", &input]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).starts_with("consistency true\n"));
}

/// `--loss-ledger` is refused rather than silently ignored: it is a GLOBAL clap flag, so
/// without a refusal it would be accepted and do nothing, the no-op this repository
/// refuses everywhere else.
#[test]
fn loss_ledger_is_refused_rather_than_silently_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_file(dir.path(), "ontology.ttl", ORDINARY_ONTOLOGY);

    let out = run(&["--loss-ledger", "consistency", &input]);
    assert_eq!(code(&out), 2, "a usage error");
    assert!(
        stderr(&out).contains("--loss-ledger"),
        "the refusal names the flag: {}",
        stderr(&out)
    );
}

/// `--jsonld-options` is refused for the same reason: this subcommand runs no serializer.
#[test]
fn jsonld_options_is_refused_rather_than_silently_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_file(dir.path(), "ontology.ttl", ORDINARY_ONTOLOGY);
    let options = write_file(
        dir.path(),
        "jsonld-options.json",
        r#"{"version":1,"mode":"context","prefixes":{"ex":"http://example.org/"}}"#,
    );

    let out = run(&["--jsonld-options", &options, "consistency", &input]);
    assert_eq!(code(&out), 2, "a usage error");
    assert!(
        stderr(&out).contains("--jsonld-options"),
        "the refusal names the flag: {}",
        stderr(&out)
    );
}
