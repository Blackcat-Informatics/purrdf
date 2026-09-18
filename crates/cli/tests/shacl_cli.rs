// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end `shacl pack|verify|explain` and `validate --shapes-product` coverage that
//! drives the BUILT `purrdf` binary (`env!("CARGO_BIN_EXE_purrdf")`), never the library.
//!
//! ## What is pinned here
//!
//! * a packed product ROUND-TRIPS: it verifies, it explains, and validating through it
//!   reaches the byte-identical report `--shapes` reaches over the same document — which
//!   is the only property that makes a prepared product safe to cache;
//! * a CORRUPT product is refused with its DIMENSION on stderr as a `shacl dimension
//!   <label>` line, and the neighbouring UNMODIFIED product still succeeds;
//! * the shapes-PARSE flags are refused by name against `--shapes-product` rather than
//!   accepted and ignored;
//! * `--expect-identity` binds a restore to the product the operator NAMED: the wrong
//!   product is refused non-zero with its dimension on stderr, the right one produces a
//!   report byte-identical to the unbound run, and the digest `shacl explain` prints is
//!   accepted back verbatim;
//! * `--rebuild` rescues a product whose stage id this build does not know — refused on
//!   `stage-id` without the flag, restored with it — reaches the byte-identical report on
//!   a CURRENT product too, and still honours `--expect-identity` rather than bypassing it;
//! * `shacl pack --shapes-graph` records the same absolute IRI `validate --shapes
//!   --shapes-graph` resolves, so a SHACL-SPARQL body reading `$shapesGraph` reaches the
//!   byte-identical verdict through either lane;
//! * every `validate` run prints `shacl shapes-provenance <token>`, and a restored product
//!   names the ARTIFACT by the identity digest `shacl explain` publishes for that same file
//!   — so a verdict in a log is attributable to the bytes that produced it.

use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;

use purrdf_core::artifact::{ArtifactBuilder, ArtifactSpec, ArtifactView};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::product::ShapesProfile;

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

/// Write `contents` to `dir/name`, returning the path as a `String`.
fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, contents).expect("write fixture file");
    p.to_str().expect("temp path is valid UTF-8").to_owned()
}

/// The same minimal shapes graph `validate_cli` uses: one violation to find.
const SHAPES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "ex:PersonShape a sh:NodeShape ;\n",
    "  sh:targetClass ex:Person ;\n",
    "  sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n",
);

/// Two people: `alice`'s age is a string (one violation), `bob`'s is an integer.
const DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice a ex:Person ; ex:age \"nope\" .\n",
    "ex:bob a ex:Person ; ex:age 42 .\n",
);

// ── `shacl pack --import` and the owl:imports closure ───────────────────────────
//
// `shacl pack` used to read raw Turtle text through a route with no import table and no
// stderr to warn on, so an `owl:imports` it could not see was silently dropped: the product
// carried FEWER shapes than the document it was packed from, and validating through it
// reported a decided, well-formed, WRONG verdict with nothing printed to say so. These tests
// pin the two lanes to the same answer, which is what the shared seam exists to guarantee.

/// A root shapes document that is nothing but an ontology header importing `lib` — every
/// shape it has lives in the imported document, not here.
const PACK_IMPORT_ROOT: &str = concat!(
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
    "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
    "<http://example.org/root> rdf:type owl:Ontology ;\n",
    "    owl:imports <http://example.org/lib> .\n",
);

/// The imported document: the one shape `PACK_IMPORT_ROOT` depends on entirely.
const PACK_IMPORT_LIB: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;\n",
    "  sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n",
);

/// One `ex:Person` violating `PACK_IMPORT_LIB`'s `ex:age` datatype constraint.
const PACK_IMPORT_DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
    "ex:alice rdf:type ex:Person ; ex:age \"nope\" .\n",
);

/// THE FALSIFIABLE CORE: `shacl pack --import` folds the closure exactly as
/// `validate --shapes --import` does, so a product packed with the import resolved and a
/// document validated with the same import reach the byte-IDENTICAL report. Before the fix,
/// `shacl pack` had no `--import` flag at all and no way to see the closure, so this test
/// could not even be expressed the same way the bug reports it — pack silently produced a
/// product that validated `conforms true / results 0` against no shapes.
#[test]
fn packing_with_import_agrees_byte_for_byte_with_validating_the_document_with_import() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = write_file(dir.path(), "root.ttl", PACK_IMPORT_ROOT);
    let lib = write_file(dir.path(), "lib.ttl", PACK_IMPORT_LIB);
    let data = write_file(dir.path(), "data.ttl", PACK_IMPORT_DATA);
    let product = dir.path().join("root.purrshp");
    let product_path = product.to_str().expect("utf8 path");
    let import_pair = format!("http://example.org/lib={lib}");

    let packed = run(&[
        "shacl",
        "pack",
        "--shapes",
        &root,
        "--import",
        &import_pair,
        "--out",
        product_path,
    ]);
    assert_eq!(
        code(&packed),
        0,
        "pack with --import failed: {}",
        stderr(&packed)
    );

    let via_product = run(&[
        "validate",
        "--shapes-product",
        product_path,
        "--format",
        "sarif",
        &data,
    ]);
    let via_document = run(&[
        "validate",
        "--shapes",
        &root,
        "--import",
        &import_pair,
        "--format",
        "sarif",
        &data,
    ]);
    assert_eq!(code(&via_product), 0, "{}", stderr(&via_product));
    assert_eq!(code(&via_document), 0, "{}", stderr(&via_document));
    assert_eq!(
        stdout(&via_product),
        stdout(&via_document),
        "a product packed with --import and a document validated with the same --import must \
         reach the byte-identical report"
    );
    for out in [&via_product, &via_document] {
        assert!(
            stderr(out).contains("shacl conforms false\n")
                && stderr(out).contains("shacl results 1\n"),
            "the imported shape must actually be enforced, not silently dropped: {}",
            stderr(out)
        );
    }
}

/// WITHOUT `--import`, `shacl pack` reports the unresolved `owl:imports` on stderr and still
/// packs the root graph alone — the same warn-not-refuse asymmetry `validate --shapes`
/// carries, and for the same reason (see `crate::shapes_source`'s module documentation, or
/// its mirror in `validate --shapes`'s doc comment): a shapes document may legitimately carry
/// an inert `owl:Ontology` header, and refusing every one of them would reject input that is
/// valid.
#[test]
fn packing_an_unresolved_import_emits_a_warning_and_still_packs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = write_file(dir.path(), "root.ttl", PACK_IMPORT_ROOT);
    let data = write_file(dir.path(), "data.ttl", PACK_IMPORT_DATA);
    let product = dir.path().join("root.purrshp");
    let product_path = product.to_str().expect("utf8 path");

    let packed = run(&["shacl", "pack", "--shapes", &root, "--out", product_path]);
    let err = stderr(&packed);
    assert_eq!(code(&packed), 0, "an unresolved import still packs: {err}");
    assert!(
        err.contains("shacl warning") && err.contains("http://example.org/lib"),
        "the unresolved import is reported, not silently dropped: {err}"
    );
    assert!(
        err.contains("--import"),
        "the warning names the remedy: {err}"
    );
    assert!(
        err.contains("shacl product bytes "),
        "the product is still written: {err}"
    );

    // The root graph alone carries no shapes, so validating through the product decides
    // `conforms true` — the SAME verdict `validate --shapes root.ttl` (no `--import`) reaches,
    // never a report that pretends the imported shape ran.
    let via_product = run(&["validate", "--shapes-product", product_path, &data]);
    assert_eq!(code(&via_product), 0, "{}", stderr(&via_product));
    assert!(
        stderr(&via_product).contains("shacl conforms true\n")
            && stderr(&via_product).contains("shacl results 0\n"),
        "validating against the root alone still decides, honestly: {}",
        stderr(&via_product)
    );
}

/// An `--import` pair that resolves nothing the shapes graph imports is a usage error, at
/// pack time exactly as at validate time: folding it in would pack a shapes graph the
/// document never described.
#[test]
fn an_import_pair_that_resolves_nothing_is_a_usage_error_at_pack_time() {
    let dir = tempfile::tempdir().expect("tempdir");
    // `SHAPES` has no `owl:imports` at all.
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let lib = write_file(dir.path(), "lib.ttl", PACK_IMPORT_LIB);
    let product = dir.path().join("shapes.purrshp");
    let product_path = product.to_str().expect("utf8 path");

    let out = run(&[
        "shacl",
        "pack",
        "--shapes",
        &shapes,
        "--import",
        &format!("http://example.org/lib={lib}"),
        "--out",
        product_path,
    ]);
    let err = stderr(&out);
    assert_eq!(code(&out), 2, "a usage error, not a runtime one: {err}");
    assert!(
        err.contains("no owl:imports at all") && err.contains("--import"),
        "the refusal says why the pair cannot be used: {err}"
    );

    // The neighbouring VALID case: the very same shapes graph, with no --import at all,
    // still packs — the refusal above is triggered by the UNUSED pair, never by the
    // shapes graph simply having no owl:imports.
    let plain = run(&["shacl", "pack", "--shapes", &shapes, "--out", product_path]);
    assert_eq!(
        code(&plain),
        0,
        "a shapes graph with no owl:imports and no --import still packs: {}",
        stderr(&plain)
    );
}

/// THE NEIGHBOURING VALID CASES for the tests above. Over-refusal is the mirror image of
/// the silent drop this file exists to close, and every refusal added above is paired here
/// with a case that must still succeed.
#[test]
fn valid_import_configurations_still_pack_and_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");

    // 1. A shapes graph with NO imports at all packs exactly as it always did — `--import`
    //    existing as a flag must not change the zero-import path.
    let plain = write_file(dir.path(), "plain.ttl", SHAPES);
    let plain_product = dir.path().join("plain.purrshp");
    let plain_product_path = plain_product.to_str().expect("utf8 path");
    assert_eq!(
        code(&run(&[
            "shacl",
            "pack",
            "--shapes",
            &plain,
            "--out",
            plain_product_path
        ])),
        0,
        "a graph with no imports still packs"
    );

    // 2. A shapes graph whose imports are ALL resolved packs, and the product round-trips:
    //    it verifies, it explains, and it validates the same report `validate --shapes
    //    --import` reaches.
    let root = write_file(dir.path(), "root.ttl", PACK_IMPORT_ROOT);
    let lib = write_file(dir.path(), "lib.ttl", PACK_IMPORT_LIB);
    let data = write_file(dir.path(), "data.ttl", PACK_IMPORT_DATA);
    let product = dir.path().join("root.purrshp");
    let product_path = product.to_str().expect("utf8 path");
    let import_pair = format!("http://example.org/lib={lib}");

    assert_eq!(
        code(&run(&[
            "shacl",
            "pack",
            "--shapes",
            &root,
            "--import",
            &import_pair,
            "--out",
            product_path
        ])),
        0,
        "a fully-resolved import table still packs"
    );
    assert_eq!(
        code(&run(&["shacl", "verify", product_path])),
        0,
        "the packed product verifies"
    );
    assert_eq!(
        code(&run(&["shacl", "explain", product_path])),
        0,
        "the packed product explains"
    );
    let validated = run(&["validate", "--shapes-product", product_path, &data]);
    assert_eq!(code(&validated), 0, "{}", stderr(&validated));
    assert!(
        stderr(&validated).contains("shacl conforms false\n")
            && stderr(&validated).contains("shacl results 1\n"),
        "the imported shape still fires through the restored product: {}",
        stderr(&validated)
    );
}

// ── `shacl pack` and a SHACL-AF `sh:SPARQLFunction` ────────────────────────────
//
// A shapes graph declaring a SPARQL function used to be refused outright by `shacl pack`:
// the declaration is parse output rather than model, so nothing in the encoded model
// carried it. It is now re-derived at restore from the shapes dataset the product already
// carries, which is where it was stated in the first place. These fixtures put the function
// on the critical path of the verdict, so a restore that lost it could not produce the same
// report — it would report nothing at all.

/// A shapes graph declaring `ex:double` as a `sh:SPARQLFunction` and calling it from the
/// `sh:sparql` body that decides whether a node conforms.
const SPARQL_FN_SHAPES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "ex:double a sh:SPARQLFunction ;\n",
    "  sh:parameter [ sh:path ex:arg ; sh:datatype xsd:integer ] ;\n",
    "  sh:returnType xsd:integer ;\n",
    "  sh:select \"SELECT ?result WHERE { BIND(?arg * 2 AS ?result) }\" .\n",
    "ex:CapShape a sh:NodeShape ;\n",
    "  sh:targetClass ex:Thing ;\n",
    "  sh:sparql [ a sh:SPARQLConstraint ;\n",
    "    sh:message \"the doubled value exceeds the cap\" ;\n",
    "    sh:select \"SELECT $this ?value WHERE { $this ex:n ?value . FILTER (ex:double(?value) > 10) }\" ] .\n",
);

/// Two `ex:Thing`s: `ex:high` doubles past the cap, `ex:low` does not.
const SPARQL_FN_DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "ex:low  a ex:Thing ; ex:n \"3\"^^xsd:integer .\n",
    "ex:high a ex:Thing ; ex:n \"7\"^^xsd:integer .\n",
);

/// THE NEGATIVE CONTROL for [`SPARQL_FN_DATA`]: both values moved across the cap, so the
/// verdict flips to the other node.
const SPARQL_FN_DATA_FLIPPED: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "ex:low  a ex:Thing ; ex:n \"9\"^^xsd:integer .\n",
    "ex:high a ex:Thing ; ex:n \"2\"^^xsd:integer .\n",
);

/// THE FALSIFIABLE CORE: `shacl pack` SUCCEEDS on a shapes graph declaring a
/// `sh:SPARQLFunction`, and validating through that product reaches the byte-identical
/// report `validate --shapes` reaches over the same document — on data where the function
/// decides the verdict, and on data where it decides it the other way.
#[test]
fn packing_a_sparql_function_agrees_byte_for_byte_with_validating_the_document() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SPARQL_FN_SHAPES);
    let data = write_file(dir.path(), "data.ttl", SPARQL_FN_DATA);
    let flipped = write_file(dir.path(), "flipped.ttl", SPARQL_FN_DATA_FLIPPED);
    let product = dir.path().join("shapes.purrshp");
    let product_path = product.to_str().expect("utf8 path");

    let packed = run(&["shacl", "pack", "--shapes", &shapes, "--out", product_path]);
    assert_eq!(
        code(&packed),
        0,
        "a sh:SPARQLFunction declaration must pack: {}",
        stderr(&packed)
    );
    assert_eq!(
        code(&run(&["shacl", "verify", product_path])),
        0,
        "the packed product verifies"
    );

    for (graph, condemned, spared) in [(&data, "high", "low"), (&flipped, "low", "high")] {
        let via_product = run(&[
            "validate",
            "--shapes-product",
            product_path,
            "--format",
            "sarif",
            graph,
        ]);
        let via_document = run(&["validate", "--shapes", &shapes, "--format", "sarif", graph]);
        assert_eq!(code(&via_product), 0, "{}", stderr(&via_product));
        assert_eq!(code(&via_document), 0, "{}", stderr(&via_document));
        assert_eq!(
            stdout(&via_product),
            stdout(&via_document),
            "a product carrying a declared SPARQL function must reach the byte-identical report"
        );
        // Not vacuous: the function has to have FIRED, on exactly the node whose doubled
        // value clears the cap. Two routes that both reported nothing would agree too.
        for out in [&via_product, &via_document] {
            assert!(
                stderr(out).contains("shacl conforms false\n")
                    && stderr(out).contains("shacl results 1\n"),
                "the declared function must decide the verdict: {}",
                stderr(out)
            );
            assert!(
                stdout(out).contains(&format!("http://example.org/{condemned}"))
                    && !stdout(out).contains(&format!("http://example.org/{spared}")),
                "the report must name ex:{condemned} and not ex:{spared}: {}",
                stdout(out)
            );
        }
    }
}

#[test]
fn a_packed_product_verifies_explains_and_validates_identically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = dir.path().join("shapes.ttl");
    let data = dir.path().join("data.ttl");
    let product = dir.path().join("shapes.purrshp");
    std::fs::write(&shapes, SHAPES).expect("write shapes");
    std::fs::write(&data, DATA).expect("write data");

    let shapes_path = shapes.to_str().expect("utf8 path");
    let data_path = data.to_str().expect("utf8 path");
    let product_path = product.to_str().expect("utf8 path");

    let packed = run(&[
        "shacl",
        "pack",
        "--shapes",
        shapes_path,
        "--out",
        product_path,
    ]);
    assert_eq!(code(&packed), 0, "pack failed: {}", stderr(&packed));
    assert!(
        stderr(&packed).contains("shacl product bytes "),
        "pack reports the byte count it wrote: {}",
        stderr(&packed)
    );

    // The writer is byte-deterministic, so packing twice is the same artifact.
    let again = dir.path().join("again.purrshp");
    let again_path = again.to_str().expect("utf8 path");
    assert_eq!(
        code(&run(&[
            "shacl",
            "pack",
            "--shapes",
            shapes_path,
            "--out",
            again_path
        ])),
        0
    );
    assert_eq!(
        std::fs::read(&product).expect("read product"),
        std::fs::read(&again).expect("read product"),
        "two packs of one document produce identical bytes",
    );

    let verified = run(&["shacl", "verify", product_path]);
    assert_eq!(code(&verified), 0, "verify failed: {}", stderr(&verified));
    assert_eq!(
        stdout(&verified).trim().len(),
        64,
        "verify prints the 64-hex identity digest, got {:?}",
        stdout(&verified)
    );

    let explained = run(&["shacl", "explain", product_path]);
    assert_eq!(
        code(&explained),
        0,
        "explain failed: {}",
        stderr(&explained)
    );
    let text = stdout(&explained);
    assert!(text.starts_with("format-version 1\n"), "got {text:?}");
    assert!(text.contains("\nstage-known true\n"), "got {text:?}");
    assert!(
        text.contains("\nidentity profile \"purrdf-shacl-core-v1\"\n"),
        "explain decodes the identity components: {text:?}"
    );
    assert!(
        text.contains("\nparse-base file:"),
        "explain reports the base the product was packed under: {text:?}"
    );

    // THE property a cache is only allowed to have: restoring the product and parsing
    // the document reach the identical report.
    let via_product = run(&[
        "validate",
        "--shapes-product",
        product_path,
        "--format",
        "sarif",
        data_path,
    ]);
    let via_document = run(&[
        "validate",
        "--shapes",
        shapes_path,
        "--format",
        "sarif",
        data_path,
    ]);
    assert_eq!(code(&via_product), 0, "{}", stderr(&via_product));
    assert_eq!(code(&via_document), 0, "{}", stderr(&via_document));
    assert_eq!(stdout(&via_product), stdout(&via_document));
    assert!(stderr(&via_product).contains("shacl conforms false"));
}

#[test]
fn a_corrupt_product_is_refused_with_its_dimension() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = dir.path().join("shapes.ttl");
    let data = dir.path().join("data.ttl");
    let product = dir.path().join("shapes.purrshp");
    std::fs::write(&shapes, SHAPES).expect("write shapes");
    std::fs::write(&data, DATA).expect("write data");
    let shapes_path = shapes.to_str().expect("utf8 path");
    let data_path = data.to_str().expect("utf8 path");
    let product_path = product.to_str().expect("utf8 path");
    assert_eq!(
        code(&run(&[
            "shacl",
            "pack",
            "--shapes",
            shapes_path,
            "--out",
            product_path
        ])),
        0
    );

    // Overwrite the magic: these bytes were never a prepared shapes product.
    let mut bytes = std::fs::read(&product).expect("read product");
    bytes[0] = b'X';
    let corrupt = dir.path().join("corrupt.purrshp");
    std::fs::write(&corrupt, &bytes).expect("write corrupt product");
    let corrupt_path = corrupt.to_str().expect("utf8 path");

    for args in [
        vec!["shacl", "verify", corrupt_path],
        vec!["shacl", "explain", corrupt_path],
        vec!["validate", "--shapes-product", corrupt_path, data_path],
    ] {
        let out = run(&args);
        assert_eq!(code(&out), 1, "{args:?} should be refused");
        assert!(
            stderr(&out).contains("shacl dimension magic\n"),
            "{args:?} names the refused dimension on its own line: {}",
            stderr(&out)
        );
    }

    // The neighbouring VALID case still succeeds — a refusal is a claim too.
    assert_eq!(code(&run(&["shacl", "verify", product_path])), 0);
    assert_eq!(
        code(&run(&[
            "validate",
            "--shapes-product",
            product_path,
            data_path
        ])),
        0
    );
}

#[test]
fn the_shapes_parse_flags_are_refused_against_a_product() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = dir.path().join("shapes.ttl");
    let data = dir.path().join("data.ttl");
    let product = dir.path().join("shapes.purrshp");
    std::fs::write(&shapes, SHAPES).expect("write shapes");
    std::fs::write(&data, DATA).expect("write data");
    let shapes_path = shapes.to_str().expect("utf8 path");
    let data_path = data.to_str().expect("utf8 path");
    let product_path = product.to_str().expect("utf8 path");
    assert_eq!(
        code(&run(&[
            "shacl",
            "pack",
            "--shapes",
            shapes_path,
            "--out",
            product_path
        ])),
        0
    );

    for (flag, value) in [
        ("--shapes-from", "turtle"),
        ("--shapes-graph", "http://example.org/sg"),
        ("--import", "http://example.org/o=x.ttl"),
    ] {
        let out = run(&[
            "validate",
            "--shapes-product",
            product_path,
            flag,
            value,
            data_path,
        ]);
        assert_eq!(code(&out), 2, "{flag} should be a usage error");
        assert!(
            stderr(&out).contains(flag),
            "{flag} is refused BY NAME: {}",
            stderr(&out)
        );
    }

    // …and naming both spellings of the shapes input at once is refused by clap.
    let both = run(&[
        "validate",
        "--shapes",
        shapes_path,
        "--shapes-product",
        product_path,
        data_path,
    ]);
    assert_eq!(code(&both), 2);

    // The neighbouring VALID case still succeeds: no parse flag, one spelling.
    assert_eq!(
        code(&run(&[
            "validate",
            "--shapes-product",
            product_path,
            data_path
        ])),
        0
    );
}

// ── `validate --shapes-product --expect-identity` ──────────────────────────────────
//
// Every other check `--shapes-product` runs asks about THIS PROCESS. None of them asks
// whether the file named on the command line is the product the operator wanted,
// because nothing in a product states which product was meant — so before this flag
// existed, `validate --shapes-product WRONG.product data.ttl` validated against
// whatever shapes that product happened to carry and exited 0.

/// A second shapes graph over different classes, so the two products genuinely carry
/// two input bindings and neither's shapes say anything about the other's data.
const OTHER_SHAPES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/> .\n",
    "ex:WidgetShape a sh:NodeShape ;\n",
    "  sh:targetClass ex:Widget ;\n",
    "  sh:property [ sh:path ex:maker ; sh:minCount 1 ] .\n",
);

/// Pack `shapes` into `dir/name`, returning the product path.
fn pack(dir: &Path, name: &str, shapes: &str) -> String {
    let source = write_file(dir, &format!("{name}.ttl"), shapes);
    let product = dir.join(name);
    let product_path = product.to_str().expect("utf8 path").to_owned();
    let out = run(&["shacl", "pack", "--shapes", &source, "--out", &product_path]);
    assert_eq!(code(&out), 0, "pack failed: {}", stderr(&out));
    product_path
}

/// The `identity-digest` line `shacl explain` prints for `product` — read exactly the
/// way an operator reads it, out of the command's own stdout.
fn explained_identity(product: &str) -> String {
    let out = run(&["shacl", "explain", product]);
    assert_eq!(code(&out), 0, "explain failed: {}", stderr(&out));
    stdout(&out)
        .lines()
        .find_map(|line| line.strip_prefix("identity-digest ").map(ToOwned::to_owned))
        .expect("explain prints an identity-digest line")
}

/// THE FALSIFIABLE CORE, on the production surface: a valid product that is NOT the one
/// named is refused, non-zero, with its dimension on stderr. Without `--expect-identity`
/// this command line is indistinguishable from the right one, and exits 0 with a report.
#[test]
fn validating_a_product_that_is_not_the_expected_one_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let a = pack(dir.path(), "a.purrshp", SHAPES);
    let b = pack(dir.path(), "b.purrshp", OTHER_SHAPES);

    let wanted = explained_identity(&b);
    assert_ne!(
        wanted,
        explained_identity(&a),
        "the two fixtures must be two products, or the expectation is vacuous",
    );

    let out = run(&[
        "validate",
        "--shapes-product",
        &a,
        "--expect-identity",
        &wanted,
        &data,
    ]);
    assert_ne!(code(&out), 0, "the wrong product must not validate");
    assert!(
        stderr(&out).contains("shacl dimension shapes-graph\n"),
        "the refusal names its dimension on its own line: {}",
        stderr(&out)
    );
    assert!(
        stdout(&out).is_empty(),
        "a refused restore writes no report: {}",
        stdout(&out)
    );

    // The gap this closes, stated as a passing assertion: without the flag, the very
    // same command line validates against whatever that product happens to carry.
    let unbound = run(&["validate", "--shapes-product", &a, &data]);
    assert_eq!(code(&unbound), 0, "{}", stderr(&unbound));
}

/// The PAIRED NEIGHBOUR. A matrix of refusals alone is satisfied by refusing
/// everything, and an expectation nobody can satisfy would send every operator back to
/// the unbound spelling it exists to replace. So `--expect-identity <A's own digest>`
/// against `A.product` must succeed and produce a report byte-identical to the one with
/// no `--expect-identity` at all: stating which product you meant changes the door, not
/// the answer.
#[test]
fn validating_a_product_against_its_own_identity_changes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let a = pack(dir.path(), "a.purrshp", SHAPES);
    let own = explained_identity(&a);

    let bound = run(&[
        "validate",
        "--shapes-product",
        &a,
        "--expect-identity",
        &own,
        &data,
    ]);
    let unbound = run(&["validate", "--shapes-product", &a, &data]);
    assert_eq!(code(&bound), 0, "{}", stderr(&bound));
    assert_eq!(code(&unbound), 0, "{}", stderr(&unbound));
    assert_eq!(
        stdout(&bound),
        stdout(&unbound),
        "a satisfied expectation must not move a single byte of the report",
    );
    assert_eq!(
        stderr(&bound),
        stderr(&unbound),
        "…nor a single byte of the verdict lines",
    );
    assert!(
        stderr(&bound).contains("shacl conforms false\n"),
        "the fixture must actually find its violation, or the comparison is vacuous: {}",
        stderr(&bound)
    );
}

/// The mechanism is USABLE end to end, not merely present: the digest `shacl explain`
/// prints, fed straight back into `--expect-identity` with nothing edited, is accepted.
/// A selector nobody can read off the artifact is not a mechanism.
#[test]
fn the_identity_explain_prints_is_the_identity_expect_identity_accepts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let a = pack(dir.path(), "a.purrshp", SHAPES);

    let explained = run(&["shacl", "explain", &a]);
    assert_eq!(code(&explained), 0, "{}", stderr(&explained));
    let printed = stdout(&explained)
        .lines()
        .find_map(|line| line.strip_prefix("identity-digest ").map(ToOwned::to_owned))
        .expect("explain prints an identity-digest line");
    assert_eq!(printed.len(), 64, "the printed selector is 64 hex digits");

    let out = run(&[
        "validate",
        "--shapes-product",
        &a,
        "--expect-identity",
        &printed,
        &data,
    ]);
    assert_eq!(
        code(&out),
        0,
        "the digest `shacl explain` prints must be accepted verbatim: {}",
        stderr(&out)
    );

    // `shacl verify` prints the same one fact about the same product, so a manifest
    // built from either verb names the same artifact.
    let verified = run(&["shacl", "verify", &a]);
    assert_eq!(code(&verified), 0, "{}", stderr(&verified));
    assert_eq!(stdout(&verified).trim_end(), printed);
}

/// A mis-typed selector is the OPERATOR's command line, not the artifact's fault: it
/// exits 2 as a usage error and names no dimension, because no product was inspected.
/// And `--expect-identity` against `--shapes` is refused rather than accepted and
/// ignored — a flag whose whole job is to fail closed must never be the flag that
/// silently did nothing.
#[test]
fn a_selector_that_names_nothing_is_a_usage_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = write_file(dir.path(), "data.ttl", DATA);
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES);
    let a = pack(dir.path(), "a.purrshp", SHAPES);
    let own = explained_identity(&a);

    let mistyped = run(&[
        "validate",
        "--shapes-product",
        &a,
        "--expect-identity",
        "not-a-digest",
        &data,
    ]);
    assert_eq!(code(&mistyped), 2, "{}", stderr(&mistyped));
    assert!(
        stderr(&mistyped).contains("--expect-identity")
            && !stderr(&mistyped).contains("shacl dimension"),
        "no product was inspected, so no dimension is named: {}",
        stderr(&mistyped)
    );

    let against_a_document = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--expect-identity",
        &own,
        &data,
    ]);
    assert_eq!(
        code(&against_a_document),
        2,
        "{}",
        stderr(&against_a_document)
    );
    assert!(
        stderr(&against_a_document).contains("--expect-identity"),
        "the flag is refused BY NAME against a shapes document: {}",
        stderr(&against_a_document)
    );

    // The neighbouring VALID case still succeeds, with the same product and the same
    // digest: only the two spellings above are refused.
    let ok = run(&[
        "validate",
        "--shapes-product",
        &a,
        "--expect-identity",
        &own,
        &data,
    ]);
    assert_eq!(code(&ok), 0, "{}", stderr(&ok));
}

// ── `shacl pack --shapes-graph` and SHACL-SPARQL's `$shapesGraph` ──────────────────
//
// `--shapes-graph IRI` can be given to `validate --shapes` but, until now, had no
// equivalent on `shacl pack`: every product's identity carried `shapes_graph: None` no
// matter what the operator wanted `$shapesGraph` bound to, so a SHACL-SPARQL body reading
// `GRAPH $shapesGraph { … }` decided one verdict through `--shapes --shapes-graph` and a
// DIFFERENT one through the packed product, with no flag able to close the gap. These
// tests pin the two lanes to the same answer.

/// A shapes graph whose SPARQL constraint fires — for every targeted focus node — exactly
/// when `$shapesGraph` is BOUND to a graph containing this document's own triples. This is
/// the fixture that makes the two lanes' divergence expressible at all: with no way to
/// bind `$shapesGraph` on the pack lane, `FILTER bound($shapesGraph)` was always false
/// through a restored product, no matter what `validate --shapes --shapes-graph` decided
/// over the identical document.
const SHAPES_GRAPH_SHAPES: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "ex:PersonShape a sh:NodeShape ;\n",
    "  sh:targetClass ex:Person ;\n",
    "  sh:sparql ex:Constraint .\n",
    "ex:Constraint\n",
    "  sh:message \"the shapes graph is exposed as $shapesGraph\" ;\n",
    "  sh:select \"\"\"\n",
    "    SELECT $this\n",
    "    WHERE {\n",
    "        FILTER bound($shapesGraph)\n",
    "        GRAPH $shapesGraph {\n",
    "            ex:PersonShape a <http://www.w3.org/ns/shacl#NodeShape> .\n",
    "        }\n",
    "    }\n",
    "  \"\"\" .\n",
);

/// One `ex:Person`, targeted by `ex:PersonShape` regardless of `$shapesGraph`.
const SHAPES_GRAPH_DATA: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:alice a ex:Person .\n",
);

const SHAPES_GRAPH_IRI: &str = "http://example.org/sg";

/// THE FALSIFIABLE CORE: `shacl pack --shapes-graph` records the same absolute IRI
/// `validate --shapes --shapes-graph` resolves and binds, so packing with the flag and
/// validating the document with the same flag reach the byte-identical report. Before
/// `shacl pack` had a `--shapes-graph` flag at all, this test could not even be attempted:
/// there was no way to ask the pack lane to bind `$shapesGraph` to anything, so a
/// product's SPARQL constraint could never fire the way `validate --shapes --shapes-graph`
/// fires it.
#[test]
fn packing_with_shapes_graph_agrees_byte_for_byte_with_validating_the_document_with_shapes_graph() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES_GRAPH_SHAPES);
    let data = write_file(dir.path(), "data.ttl", SHAPES_GRAPH_DATA);
    let product = dir.path().join("shapes.purrshp");
    let product_path = product.to_str().expect("utf8 path");

    let packed = run(&[
        "shacl",
        "pack",
        "--shapes",
        &shapes,
        "--shapes-graph",
        SHAPES_GRAPH_IRI,
        "--out",
        product_path,
    ]);
    assert_eq!(
        code(&packed),
        0,
        "pack with --shapes-graph failed: {}",
        stderr(&packed)
    );

    let via_product = run(&[
        "validate",
        "--shapes-product",
        product_path,
        "--format",
        "sarif",
        &data,
    ]);
    let via_document = run(&[
        "validate",
        "--shapes",
        &shapes,
        "--shapes-graph",
        SHAPES_GRAPH_IRI,
        "--format",
        "sarif",
        &data,
    ]);
    assert_eq!(code(&via_product), 0, "{}", stderr(&via_product));
    assert_eq!(code(&via_document), 0, "{}", stderr(&via_document));
    assert_eq!(
        stdout(&via_product),
        stdout(&via_document),
        "a product packed with --shapes-graph and a document validated with the same \
         --shapes-graph must reach the byte-identical report"
    );
    for out in [&via_product, &via_document] {
        assert!(
            stderr(out).contains("shacl conforms false\n")
                && stderr(out).contains("shacl results 1\n"),
            "the SPARQL constraint must actually see $shapesGraph bound and fire, not \
             silently see it unbound: {}",
            stderr(out)
        );
    }

    // Without the flag on EITHER lane, $shapesGraph is unbound and the constraint never
    // fires — that verdict must still be reachable by NOT asking for the graph.
    let via_document_unbound = run(&["validate", "--shapes", &shapes, &data]);
    assert_eq!(
        code(&via_document_unbound),
        0,
        "{}",
        stderr(&via_document_unbound)
    );
    assert!(
        stderr(&via_document_unbound).contains("shacl conforms true\n")
            && stderr(&via_document_unbound).contains("shacl results 0\n"),
        "with no --shapes-graph at all, $shapesGraph is unbound and the constraint does not \
         fire: {}",
        stderr(&via_document_unbound)
    );
}

/// `shacl explain` on a product packed WITH `--shapes-graph` prints the `shapes-graph`
/// identity row as PRESENT, carrying exactly the IRI it was packed with — the identity row
/// that no shipped tool could ever populate before this flag existed.
#[test]
fn explain_reports_the_shapes_graph_pack_recorded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES_GRAPH_SHAPES);
    let product = dir.path().join("shapes.purrshp");
    let product_path = product.to_str().expect("utf8 path");

    assert_eq!(
        code(&run(&[
            "shacl",
            "pack",
            "--shapes",
            &shapes,
            "--shapes-graph",
            SHAPES_GRAPH_IRI,
            "--out",
            product_path,
        ])),
        0
    );

    let explained = run(&["shacl", "explain", product_path]);
    assert_eq!(code(&explained), 0, "{}", stderr(&explained));
    let text = stdout(&explained);
    assert!(
        text.contains(&format!(
            "\nidentity shapes-graph \"(present) {SHAPES_GRAPH_IRI}\"\n"
        )),
        "explain reports the recorded shapes-graph IRI as present: {text:?}"
    );
    assert!(
        text.contains(&format!("\nparse-shapes-graph {SHAPES_GRAPH_IRI}\n")),
        "explain reports the recorded shapes-graph IRI in its parse inputs too: {text:?}"
    );
}

/// THE NEIGHBOURING VALID CASE: a shapes graph packed with NO `--shapes-graph` at all
/// packs exactly as it always did — the flag existing must not change the no-flag path.
/// `explain` still reports the row absent, and the restored report is unchanged from what
/// it was before this flag existed.
#[test]
fn accepts_pack_without_shapes_graph_neighbour() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes = write_file(dir.path(), "shapes.ttl", SHAPES_GRAPH_SHAPES);
    let data = write_file(dir.path(), "data.ttl", SHAPES_GRAPH_DATA);
    let product = dir.path().join("shapes.purrshp");
    let product_path = product.to_str().expect("utf8 path");

    let packed = run(&["shacl", "pack", "--shapes", &shapes, "--out", product_path]);
    assert_eq!(code(&packed), 0, "{}", stderr(&packed));

    let explained = run(&["shacl", "explain", product_path]);
    assert_eq!(code(&explained), 0, "{}", stderr(&explained));
    let text = stdout(&explained);
    assert!(
        text.contains("\nidentity shapes-graph \"(absent)\"\n"),
        "with no --shapes-graph flag, the identity row stays absent: {text:?}"
    );
    assert!(
        text.contains("\nparse-shapes-graph none\n"),
        "with no shapes-graph recorded, the parse-inputs line says so explicitly: {text:?}"
    );

    // $shapesGraph is unbound, so the constraint never fires — the same verdict a document
    // with no sh:shapesGraph declaration and no --shapes-graph flag has always reached.
    let via_product = run(&["validate", "--shapes-product", product_path, &data]);
    assert_eq!(code(&via_product), 0, "{}", stderr(&via_product));
    assert!(
        stderr(&via_product).contains("shacl conforms true\n")
            && stderr(&via_product).contains("shacl results 0\n"),
        "no --shapes-graph at pack time still restores a validator with $shapesGraph \
         unbound: {}",
        stderr(&via_product)
    );
}

// ── `validate --shapes-product --rebuild` ──────────────────────────────────────────
//
// `rebuild` is the prepared-product design's entire forward-compatibility answer: a
// reader that meets a product whose stage id it does not know refuses `admit` and
// re-derives the preparation from the shapes DATASET the product carries instead —
// no RDF text is parsed and no file is read. Until now that path had no command-line
// spelling: an operator meeting a `stage-id` refusal was told to "restore with a
// rebuild" by a verb this binary did not have.

/// The envelope constants a reader observes from any product this build writes —
/// the header magic, the format version, and the section count — re-declared here so
/// the tamper below can reframe a real product's sections into a new, fully
/// self-consistent container. This is the SAME technique
/// `crates/shapes/tests/product_refusal.rs` uses to build its `stage-id` fixture:
/// repacking recomputes every digest, so the result is a product this build's own
/// envelope check accepts — a tamper the envelope itself would refuse would prove
/// nothing about `--rebuild`.
const PRODUCT_MAGIC: [u8; 8] = *b"PURRSHP1";
const PRODUCT_FORMAT_VERSION: u32 = 1;
const PRODUCT_SECTION_COUNT: usize = 3;
const PRODUCT_SPEC: ArtifactSpec =
    ArtifactSpec::new(PRODUCT_MAGIC, PRODUCT_FORMAT_VERSION, PRODUCT_SECTION_COUNT);
const SECTION_IDENTITY: u32 = 0;
const SECTION_DATASET: u32 = 1;
const SECTION_AST: u32 = 2;

/// Parse `shapes_ttl`, prepare and pack it exactly as `shacl pack` does, then splice
/// a stage id NO build ever wrote into its identity section and reframe the
/// container so every digest still checks out.
///
/// This is the one state `admit` cannot reach and `--rebuild` exists for: a product
/// whose memo describes a preparation stage this build does not recognize. The
/// dataset section is untouched, so the shapes graph it carries is still exactly the
/// one `shapes_ttl` describes — only the memo's stage id is foreign.
fn foreign_stage_product(shapes_ttl: &str) -> Vec<u8> {
    let shapes = parse_shapes(shapes_ttl, None).expect("the fixture shapes parse");
    let genuine = PreparedShapes::new(Arc::new(shapes))
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture packs under this build");

    let view = ArtifactView::from_bytes(PRODUCT_SPEC, &genuine).expect("the genuine product opens");
    let mut identity_section = view
        .section(SECTION_IDENTITY)
        .expect("the identity section is present")
        .to_vec();
    // The stage id is the identity section's first 32 bytes — see
    // `purrdf_shapes::product`'s `encode_preamble`/`decode_preamble`.
    identity_section[..32].copy_from_slice(&[0xAB; 32]);

    let mut builder = ArtifactBuilder::new(PRODUCT_SPEC);
    builder
        .identity(view.identity().clone())
        .section(SECTION_IDENTITY, &identity_section)
        .section(
            SECTION_DATASET,
            view.section(SECTION_DATASET)
                .expect("the dataset section is present"),
        )
        .section(
            SECTION_AST,
            view.section(SECTION_AST)
                .expect("the ast section is present"),
        );
    builder
        .build_bytes()
        .expect("the repack frames a well-formed product")
}

/// THE FALSIFIABLE CORE: `--shapes-product` alone refuses a product whose stage id
/// this build does not know on `stage-id`, and `--rebuild` is the remedy that
/// production surface names — not merely a capability that exists somewhere in
/// Rust. The rebuilt report is byte-identical to validating the ORIGINAL shapes
/// document directly, which is the property that makes rescuing the product safe
/// rather than merely non-crashing.
#[test]
fn rebuild_rescues_a_product_whose_stage_id_this_build_does_not_know() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes_path = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data_path = write_file(dir.path(), "data.ttl", DATA);
    let product = dir.path().join("foreign.purrshp");
    std::fs::write(&product, foreign_stage_product(SHAPES)).expect("write foreign product");
    let product_path = product.to_str().expect("utf8 path");

    // Without --rebuild: refused on stage-id, non-zero, no report on stdout.
    let unrebuilt = run(&["validate", "--shapes-product", product_path, &data_path]);
    assert_eq!(
        code(&unrebuilt),
        1,
        "a foreign stage id must not admit: {}",
        stderr(&unrebuilt)
    );
    assert!(
        stderr(&unrebuilt).contains("shacl dimension stage-id\n"),
        "the refusal names its dimension on its own line: {}",
        stderr(&unrebuilt)
    );
    assert!(
        stdout(&unrebuilt).is_empty(),
        "a refused restore writes no report: {}",
        stdout(&unrebuilt)
    );

    // With --rebuild: the same bytes restore and validate.
    let rebuilt = run(&[
        "validate",
        "--shapes-product",
        product_path,
        "--rebuild",
        &data_path,
    ]);
    assert_eq!(code(&rebuilt), 0, "{}", stderr(&rebuilt));
    assert!(
        stderr(&rebuilt).contains("shacl conforms false\n"),
        "the fixture must actually find its violation, or the comparison is vacuous: {}",
        stderr(&rebuilt)
    );

    // …and the rescued report is BYTE-IDENTICAL to validating the original document
    // directly: rebuilding must not merely succeed, it must answer correctly.
    let via_document = run(&["validate", "--shapes", &shapes_path, &data_path]);
    assert_eq!(code(&via_document), 0, "{}", stderr(&via_document));
    assert_eq!(
        stdout(&rebuilt),
        stdout(&via_document),
        "a rescued product must answer exactly what the original document does",
    );
}

/// THE PAIRED NEIGHBOUR: `--rebuild` on a CURRENT product (one whose stage id this
/// build already knows) must ALSO succeed, and must reach the byte-identical report
/// plain `admit` does. `--rebuild` is a second door onto one product, never a
/// second, divergent answer — over-refusal's mirror image is a second success that
/// silently disagrees with the first.
#[test]
fn rebuild_on_a_current_product_matches_admit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data_path = write_file(dir.path(), "data.ttl", DATA);
    let product_path = pack(dir.path(), "shapes.purrshp", SHAPES);

    let admitted = run(&["validate", "--shapes-product", &product_path, &data_path]);
    let rebuilt = run(&[
        "validate",
        "--shapes-product",
        &product_path,
        "--rebuild",
        &data_path,
    ]);
    assert_eq!(code(&admitted), 0, "{}", stderr(&admitted));
    assert_eq!(code(&rebuilt), 0, "{}", stderr(&rebuilt));
    assert_eq!(
        stdout(&admitted),
        stdout(&rebuilt),
        "rebuilding a current product must not move a single byte of the report",
    );
    // …nor a single byte of the verdict lines. The `shacl shapes-provenance` receipt is
    // deliberately excluded and is the ONE line that legitimately differs: it names the
    // seam the preparation came through, which is exactly what these two runs differ in,
    // and flattening it would report "checked against this process" for a rebuild that was
    // not. Everything that describes the ANSWER must still match byte for byte.
    let verdict = |out: &Output| {
        stderr(out)
            .lines()
            .filter(|line| !line.starts_with("shacl shapes-provenance "))
            .fold(String::new(), |mut kept, line| {
                kept.push_str(line);
                kept.push('\n');
                kept
            })
    };
    assert_eq!(verdict(&admitted), verdict(&rebuilt));
    assert!(
        verdict(&admitted).contains("shacl conforms false\n"),
        "the fixture must actually find its violation, or the comparison is vacuous: {}",
        stderr(&admitted)
    );

    // The excluded line is excluded because it DIFFERS, not because it is absent — a
    // filter that silently matched nothing would make the comparison above weaker than it
    // looks.
    assert!(
        stderr(&admitted).contains("shacl shapes-provenance restored-admitted ")
            && stderr(&rebuilt).contains("shacl shapes-provenance restored-rebuilt "),
        "each run must name its own seam: {} / {}",
        stderr(&admitted),
        stderr(&rebuilt)
    );
}

/// `--rebuild` composes with `--expect-identity` rather than escaping it: the
/// expectation is checked FIRST regardless of which repair strategy is chosen, so a
/// product that is not the one required is refused on `shapes-graph` — never
/// silently rebuilt into a report about a shapes graph nobody asked about.
#[test]
fn rebuild_still_honours_expect_identity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data_path = write_file(dir.path(), "data.ttl", DATA);
    let a = pack(dir.path(), "a.purrshp", SHAPES);
    let b = pack(dir.path(), "b.purrshp", OTHER_SHAPES);
    let own = explained_identity(&a);
    let other = explained_identity(&b);
    assert_ne!(own, other, "the two fixtures must be two products");

    // Mismatched digest: --rebuild must not become an escape hatch around the
    // identity binding.
    let mismatched = run(&[
        "validate",
        "--shapes-product",
        &a,
        "--rebuild",
        "--expect-identity",
        &other,
        &data_path,
    ]);
    assert_ne!(code(&mismatched), 0, "the wrong product must not rebuild");
    assert!(
        stderr(&mismatched).contains("shacl dimension shapes-graph\n"),
        "the mismatch is refused on its dimension even under --rebuild: {}",
        stderr(&mismatched)
    );
    assert!(
        stdout(&mismatched).is_empty(),
        "a refused restore writes no report: {}",
        stdout(&mismatched)
    );

    // Matching digest: --rebuild still succeeds, byte-identical to the unbound
    // rebuild and to the bound admit over the same product.
    let matched = run(&[
        "validate",
        "--shapes-product",
        &a,
        "--rebuild",
        "--expect-identity",
        &own,
        &data_path,
    ]);
    assert_eq!(code(&matched), 0, "{}", stderr(&matched));
    let unbound_rebuild = run(&["validate", "--shapes-product", &a, "--rebuild", &data_path]);
    assert_eq!(code(&unbound_rebuild), 0, "{}", stderr(&unbound_rebuild));
    assert_eq!(
        stdout(&matched),
        stdout(&unbound_rebuild),
        "a satisfied expectation must not move a single byte of the rebuilt report",
    );
}

/// `--rebuild` names a repair strategy for restoring a PRODUCT, and a shapes
/// DOCUMENT has no memo to skip and no carried dataset to re-derive from — it is
/// refused BY NAME against `--shapes` rather than accepted and silently ignored,
/// the same posture `--expect-identity` takes. The neighbouring VALID case — the
/// same flag against `--shapes-product` — still succeeds.
#[test]
fn rebuild_is_refused_against_a_shapes_document() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes_path = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data_path = write_file(dir.path(), "data.ttl", DATA);
    let product_path = pack(dir.path(), "shapes.purrshp", SHAPES);

    let against_a_document = run(&[
        "validate",
        "--shapes",
        &shapes_path,
        "--rebuild",
        &data_path,
    ]);
    assert_eq!(
        code(&against_a_document),
        2,
        "{}",
        stderr(&against_a_document)
    );
    assert!(
        stderr(&against_a_document).contains("--rebuild"),
        "the flag is refused BY NAME against a shapes document: {}",
        stderr(&against_a_document)
    );

    let against_a_product = run(&[
        "validate",
        "--shapes-product",
        &product_path,
        "--rebuild",
        &data_path,
    ]);
    assert_eq!(
        code(&against_a_product),
        0,
        "{}",
        stderr(&against_a_product)
    );
}

/// Every `validate` run says where its shapes came from, and a restored product names the
/// ARTIFACT — with the identity digest `shacl explain` prints for that same file.
///
/// This is the half of the question admission does not answer. `--shapes-product` establishes
/// that this build MAY execute a product; nothing in the verdict it prints says WHICH product
/// produced it, so an operator reading `shacl conforms true` in a log has no way to attribute
/// it to a file. The digest is read off `shacl explain` here rather than restated, because a
/// second spelling of one artifact's name is a second thing to drift: the point of the
/// receipt is that the value on it can be handed straight back to `--expect-identity`.
#[test]
fn a_validate_run_says_where_its_shapes_came_from() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shapes_path = write_file(dir.path(), "shapes.ttl", SHAPES);
    let data_path = write_file(dir.path(), "data.ttl", DATA);
    let product = dir.path().join("shapes.purrshp");
    let product_path = product.to_str().expect("utf8 path").to_owned();

    let packed = run(&[
        "shacl",
        "pack",
        "--shapes",
        &shapes_path,
        "--out",
        &product_path,
    ]);
    assert_eq!(code(&packed), 0, "{}", stderr(&packed));

    // The artifact's own name, read off the verb that publishes it.
    let explained = run(&["shacl", "explain", &product_path]);
    assert_eq!(code(&explained), 0, "{}", stderr(&explained));
    let digest = stdout(&explained)
        .lines()
        .find_map(|line| line.strip_prefix("identity-digest ").map(ToOwned::to_owned))
        .expect("`shacl explain` prints an identity digest");
    assert_eq!(digest.len(), 64, "the digest is 64 hexadecimal digits");

    let parsed = run(&["validate", "--shapes", &shapes_path, &data_path]);
    assert_eq!(code(&parsed), 0, "{}", stderr(&parsed));
    assert!(
        stderr(&parsed).contains("shacl shapes-provenance parsed\n"),
        "a document lane names no artifact, and must not invent one: {}",
        stderr(&parsed)
    );

    let admitted = run(&["validate", "--shapes-product", &product_path, &data_path]);
    assert_eq!(code(&admitted), 0, "{}", stderr(&admitted));
    assert!(
        stderr(&admitted).contains(&format!(
            "shacl shapes-provenance restored-admitted {digest}\n"
        )),
        "the admitted lane must name the artifact by the digest `shacl explain` prints: {}",
        stderr(&admitted)
    );

    let rebuilt = run(&[
        "validate",
        "--shapes-product",
        &product_path,
        "--rebuild",
        &data_path,
    ]);
    assert_eq!(code(&rebuilt), 0, "{}", stderr(&rebuilt));
    assert!(
        stderr(&rebuilt).contains(&format!(
            "shacl shapes-provenance restored-rebuilt {digest}\n"
        )),
        "the rebuild lane names the same artifact under its own token, because the digest it \
         carries was recorded rather than checked: {}",
        stderr(&rebuilt)
    );

    // The two restore tokens really are distinguishable — a rendering that flattened them
    // would report the stronger claim (checked against this process) for both.
    assert!(
        !stderr(&rebuilt).contains("restored-admitted"),
        "the rebuild lane must not claim the admitted token: {}",
        stderr(&rebuilt)
    );

    // …and the receipt is a receipt, not a replacement for the verdict.
    for out in [&parsed, &admitted, &rebuilt] {
        assert!(
            stderr(out).contains("shacl conforms false\n")
                && stderr(out).contains("shacl results 1\n"),
            "every lane still decides: {}",
            stderr(out)
        );
    }
}
