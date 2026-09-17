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
