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
//!   accepted and ignored.

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
