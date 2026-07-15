// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `reason` coverage that drives the BUILT `purrdf` binary
//! (`env!("CARGO_BIN_EXE_purrdf")`) — never the library — so every assertion pins the
//! shipped executable's entailment behavior.
//!
//! ## What each regime asserts
//!
//! Every supported regime gets a fixture with a KNOWN entailment, and the test asserts
//! the SPECIFIC inferred N-Triples line is present in the output (the closure is written
//! to a `.nt` file so the output is deterministic, line-based text and substring
//! assertions are robust). The exact inferred lines below were confirmed by driving the
//! binary directly, not assumed:
//!
//! * **simple** — a faithful copy: the output quad SET equals the input (nothing added).
//! * **rdf** — predicate-typing: every resource used in predicate position is asserted an
//!   `rdf:Property`.
//! * **rdfs** — the RDFS rule set: `subClassOf` type propagation, `domain`/`range` typing,
//!   and `subPropertyOf` triple propagation.
//! * **owl-rl** — OWL 2 RL beyond RDFS: `owl:SymmetricProperty` and
//!   `owl:TransitiveProperty` closure.
//!
//! The three non-materializable regimes (`owl-direct`, `rif`, `d`) are the exit-3
//! boundary (they need query class expressions or a rule set the CLI cannot supply), and
//! a `.purrpck` pack source exercises the pack→dataset reconstruction path inside
//! `reason`.

use std::path::Path;
use std::process::{Command, Output};

/// The rdf:type IRI, spelled out (the inferred-triple assertions key on it).
const RDF_TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";

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

/// stderr of an [`Output`] as a `String`, for diagnostics + boundary assertions.
fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Join a name onto `dir`, returning it as an owned `String` (the shape [`run`] wants).
fn path(dir: &Path, name: &str) -> String {
    dir.join(name)
        .to_str()
        .expect("temp path is valid UTF-8")
        .to_owned()
}

/// Write `contents` to `dir/name`, returning the path.
fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let p = path(dir, name);
    std::fs::write(&p, contents).expect("write fixture file");
    p
}

/// The non-empty, trimmed lines of a file as a sorted `Vec` (a set-equality helper for
/// line-based N-Triples output).
fn sorted_lines(p: &str) -> Vec<String> {
    let text = std::fs::read_to_string(p).expect("read output file");
    let mut lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    lines.sort();
    lines
}

/// `reason --regime simple` is a faithful copy: the output quad SET equals the input.
/// Simple entailment adds nothing beyond a faithful reproduction of the source.
#[test]
fn simple_is_identity_closure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let input = concat!(
        "<http://example.org/a> <http://example.org/knows> <http://example.org/b> .\n",
        "<http://example.org/b> <http://example.org/name> \"Bob\" .\n",
    );
    let seed = write_file(dir, "simple.nt", input);
    let out = path(dir, "out.nt");

    let o = run(&["reason", "--regime", "simple", &seed, &out]);
    assert!(o.status.success(), "simple reason failed: {}", stderr(&o));

    // Every input triple is present, and NOTHING was added: the set is identical.
    assert_eq!(
        sorted_lines(&out),
        sorted_lines(&seed),
        "simple entailment must be a faithful identity copy (set-equal to the input)"
    );
}

/// `reason --regime rdf` asserts predicate-typing: every resource used in predicate
/// position is inferred to be an `rdf:Property`. From `ex:a ex:knows ex:b .` the closure
/// contains `ex:knows a rdf:Property`.
#[test]
fn rdf_infers_predicate_is_property() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(
        dir,
        "rdf.nt",
        "<http://example.org/a> <http://example.org/knows> <http://example.org/b> .\n",
    );
    let out = path(dir, "out.nt");

    let o = run(&["reason", "--regime", "rdf", &seed, &out]);
    assert!(o.status.success(), "rdf reason failed: {}", stderr(&o));

    let inferred = format!(
        "<http://example.org/knows> {RDF_TYPE} \
         <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property>"
    );
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(
        text.contains(&inferred),
        "rdf entailment must type the predicate as rdf:Property; got: {text}"
    );
    // The original triple survives too.
    assert!(
        text.contains("<http://example.org/a> <http://example.org/knows> <http://example.org/b>"),
        "the original triple must be preserved; got: {text}"
    );
}

/// `reason --regime rdfs` runs the RDFS rule set. A `subClassOf` chain propagates
/// `rdf:type`; `rdfs:domain` / `rdfs:range` type the subject / object of a use of the
/// property; and `rdfs:subPropertyOf` propagates the base triple onto the super-property.
#[test]
fn rdfs_infers_subclass_domain_range_and_subproperty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(
        dir,
        "rdfs.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            // subClassOf → type propagation.
            "ex:Dog rdfs:subClassOf ex:Animal .\n",
            "ex:rex a ex:Dog .\n",
            // domain + range → subject/object typing.
            "ex:knows rdfs:domain ex:Person .\n",
            "ex:knows rdfs:range ex:Person .\n",
            "ex:a ex:knows ex:b .\n",
            // subPropertyOf → triple propagation.
            "ex:loves rdfs:subPropertyOf ex:knows .\n",
            "ex:c ex:loves ex:d .\n",
        ),
    );
    let out = path(dir, "out.nt");

    let o = run(&["reason", "--regime", "rdfs", &seed, &out]);
    assert!(o.status.success(), "rdfs reason failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");

    // subClassOf: ex:rex a ex:Animal.
    assert!(
        text.contains(&format!(
            "<http://example.org/rex> {RDF_TYPE} <http://example.org/Animal>"
        )),
        "rdfs must infer `ex:rex a ex:Animal` via subClassOf; got: {text}"
    );
    // domain: ex:a a ex:Person.
    assert!(
        text.contains(&format!(
            "<http://example.org/a> {RDF_TYPE} <http://example.org/Person>"
        )),
        "rdfs must infer `ex:a a ex:Person` via rdfs:domain; got: {text}"
    );
    // range: ex:b a ex:Person.
    assert!(
        text.contains(&format!(
            "<http://example.org/b> {RDF_TYPE} <http://example.org/Person>"
        )),
        "rdfs must infer `ex:b a ex:Person` via rdfs:range; got: {text}"
    );
    // subPropertyOf: ex:c ex:knows ex:d (propagated from ex:c ex:loves ex:d).
    assert!(
        text.contains("<http://example.org/c> <http://example.org/knows> <http://example.org/d>"),
        "rdfs must propagate `ex:c ex:knows ex:d` via subPropertyOf; got: {text}"
    );
}

/// `reason --regime owl-rl` runs OWL 2 RL — strictly beyond RDFS. An
/// `owl:SymmetricProperty` yields the reversed triple, and an `owl:TransitiveProperty`
/// closes a two-hop chain.
#[test]
fn owl_rl_infers_symmetric_and_transitive() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(
        dir,
        "owlrl.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
            // Symmetric: ex:a ex:knows ex:b ⇒ ex:b ex:knows ex:a.
            "ex:knows a owl:SymmetricProperty .\n",
            "ex:a ex:knows ex:b .\n",
            // Transitive: ex:x ex:before ex:y, ex:y ex:before ex:z ⇒ ex:x ex:before ex:z.
            "ex:before a owl:TransitiveProperty .\n",
            "ex:x ex:before ex:y .\n",
            "ex:y ex:before ex:z .\n",
        ),
    );
    let out = path(dir, "out.nt");

    let o = run(&["reason", "--regime", "owl-rl", &seed, &out]);
    assert!(o.status.success(), "owl-rl reason failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");

    // Symmetry.
    assert!(
        text.contains("<http://example.org/b> <http://example.org/knows> <http://example.org/a>"),
        "owl-rl must infer the symmetric `ex:b ex:knows ex:a`; got: {text}"
    );
    // Transitivity.
    assert!(
        text.contains("<http://example.org/x> <http://example.org/before> <http://example.org/z>"),
        "owl-rl must infer the transitive `ex:x ex:before ex:z`; got: {text}"
    );
}

/// The three non-materializable regimes (`owl-direct`, `rif`, `d`) each exit with code 3
/// and print a diagnostic to stderr naming why: they need query class expressions or a
/// rule set the CLI has no way to supply.
#[test]
fn boundary_regimes_exit_three_with_diagnostic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(
        dir,
        "seed.nt",
        "<http://example.org/a> <http://example.org/knows> <http://example.org/b> .\n",
    );
    let out = path(dir, "out.nt");

    // Each boundary regime is unsupported for a DISTINCT spec-inherent reason; the
    // diagnostic must name that specific reason, not a generic catch-all.
    for (regime, expected_reason) in [
        ("owl-direct", "class expressions"),
        ("rif", "rule set"),
        ("d", "datatype"),
    ] {
        let o = run(&["reason", "--regime", regime, &seed, &out]);
        assert_eq!(
            o.status.code(),
            Some(3),
            "regime {regime} must exit 3 (unsupported boundary); stderr: {}",
            stderr(&o)
        );
        let err = stderr(&o);
        assert!(
            err.contains("cannot be materialized"),
            "regime {regime} must explain it cannot be materialized; got: {err}"
        );
        assert!(
            err.contains(expected_reason),
            "regime {regime} must name its specific reason ({expected_reason:?}); got: {err}"
        );
    }
}

/// `reason` over a `.purrpck` PACK source reconstructs the dataset and materializes the
/// closure — exercising the pack→dataset reconstruction path inside `reason`. The
/// inferred RDFS type appears just as it does from a text source.
#[test]
fn pack_input_is_reconstructed_and_reasoned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let ttl = write_file(
        dir,
        "sub.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "ex:Dog rdfs:subClassOf ex:Animal .\n",
            "ex:rex a ex:Dog .\n",
        ),
    );

    // Build a pack from the RDFS fixture, then reason over the pack.
    let pack = path(dir, "sub.purrpck");
    let o = run(&["convert", "--from", "turtle", "--to", "pack", &ttl, &pack]);
    assert!(
        o.status.success(),
        "building the pack failed: {}",
        stderr(&o)
    );

    let out = path(dir, "out.nt");
    let o = run(&["reason", "--regime", "rdfs", &pack, &out]);
    assert!(
        o.status.success(),
        "reason over a pack failed: {}",
        stderr(&o)
    );
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(
        text.contains(&format!(
            "<http://example.org/rex> {RDF_TYPE} <http://example.org/Animal>"
        )),
        "reasoning over a reconstructed pack must yield `ex:rex a ex:Animal`; got: {text}"
    );
}

/// A `reason` run twice produces byte-identical output: the closure and serializer are
/// deterministic.
#[test]
fn reason_output_is_deterministic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(
        dir,
        "rdfs.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "ex:Dog rdfs:subClassOf ex:Animal .\n",
            "ex:rex a ex:Dog .\n",
        ),
    );
    let a = path(dir, "a.nt");
    let b = path(dir, "b.nt");

    assert!(
        run(&["reason", "--regime", "rdfs", &seed, &a])
            .status
            .success()
    );
    assert!(
        run(&["reason", "--regime", "rdfs", &seed, &b])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read(&a).expect("read a"),
        std::fs::read(&b).expect("read b"),
        "a reason run twice must be byte-identical"
    );
}

/// `--base` resolves relative IRIs in the source before reasoning: a Turtle input with
/// relative IRI terms, reasoned under `--base`, yields absolute IRIs in the output.
#[test]
fn base_resolves_relative_iris_before_reasoning() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    // Turtle carries relative IRIs (N-Triples would reject them); `--base` resolves them.
    let seed = write_file(
        dir,
        "rel.ttl",
        "<thing> <http://example.org/knows> <other> .\n",
    );
    let out = path(dir, "out.nt");

    let o = run(&[
        "reason",
        "--regime",
        "rdf",
        "--base",
        "http://example.org/base/",
        &seed,
        &out,
    ]);
    assert!(o.status.success(), "--base reason failed: {}", stderr(&o));
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(
        text.contains("http://example.org/base/thing"),
        "relative subject must resolve against --base; got: {text}"
    );
    assert!(
        text.contains("http://example.org/base/other"),
        "relative object must resolve against --base; got: {text}"
    );
}

/// `reason` to a `.purrpck` OUTPUT produces a valid pack whose reconstructed contents
/// carry the inferred triple (re-converting the pack back to text shows the inference).
#[test]
fn pack_output_is_a_valid_pack_carrying_the_inference() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir = dir.path();
    let seed = write_file(
        dir,
        "rdfs.ttl",
        concat!(
            "@prefix ex: <http://example.org/> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "ex:Dog rdfs:subClassOf ex:Animal .\n",
            "ex:rex a ex:Dog .\n",
        ),
    );

    // Reason directly INTO a pack.
    let pack = path(dir, "closure.purrpck");
    let o = run(&["reason", "--regime", "rdfs", &seed, &pack]);
    assert!(
        o.status.success(),
        "reason to a pack failed: {}",
        stderr(&o)
    );

    // Re-convert the pack to N-Triples: a valid pack whose contents carry the inference.
    let back = path(dir, "back.nt");
    let o = run(&[
        "convert", "--from", "pack", "--to", "ntriples", &pack, &back,
    ]);
    assert!(
        o.status.success(),
        "re-converting the closure pack failed: {}",
        stderr(&o)
    );
    let text = std::fs::read_to_string(&back).expect("read reconverted output");
    assert!(
        text.contains(&format!(
            "<http://example.org/rex> {RDF_TYPE} <http://example.org/Animal>"
        )),
        "the closure pack must carry the inferred `ex:rex a ex:Animal`; got: {text}"
    );
}
