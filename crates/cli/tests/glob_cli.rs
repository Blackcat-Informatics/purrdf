// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The real binary consumes the public compiled matcher through the facade.
mod support;
use support::{run, stderr, stdout_utf8};

#[test]
fn explicit_surfaces_decide_and_refusals_publish_no_answer() {
    let matched = run(&[
        "glob",
        "**/a*b/c",
        "axxx/y/ab/c",
        "--mode",
        "path",
        "--surface",
        "literal",
        "--max-work",
        "1000",
    ]);
    assert!(matched.status.success(), "{}", stderr(&matched));
    assert!(stdout_utf8(&matched).starts_with("matched true\nwork "));
    let negative = run(&[
        "glob",
        "a/*",
        "https://example.org/a/b/c",
        "--mode",
        "path",
        "--surface",
        "iri-path",
        "--max-work",
        "1000",
    ]);
    assert!(negative.status.success(), "{}", stderr(&negative));
    assert!(stdout_utf8(&negative).starts_with("matched false\nwork "));
    let iri = run(&[
        "glob",
        "/a/%2F",
        "https://example.org/a/%2F?x#y",
        "--mode",
        "path",
        "--surface",
        "iri-path",
        "--max-work",
        "1000",
    ]);
    assert!(iri.status.success(), "{}", stderr(&iri));
    assert!(stdout_utf8(&iri).starts_with("matched true\nwork "));
    for (value, expected) in [
        ("https://example.org/", "false"),
        ("https://example.org/?", "true"),
    ] {
        let query = run(&[
            "glob",
            "*",
            value,
            "--mode",
            "text",
            "--surface",
            "iri-query",
            "--max-work",
            "100",
        ]);
        assert!(query.status.success(), "{}", stderr(&query));
        assert!(stdout_utf8(&query).starts_with(&format!("matched {expected}\nwork ")));
    }
    for (pattern, value, surface, work, message) in [
        ("*", "anything", "literal", "0", "glob work refused"),
        ("[z-a]", "a", "literal", "100", "glob syntax at byte 1"),
        ("*", "http://[", "iri", "100", "malformed authority"),
    ] {
        let refused = run(&[
            "glob",
            pattern,
            value,
            "--mode",
            "text",
            "--surface",
            surface,
            "--max-work",
            work,
        ]);
        assert_eq!(refused.status.code(), Some(1), "{}", stderr(&refused));
        assert_eq!(refused.stdout, [] as [u8; 0]);
        assert!(stderr(&refused).contains(message), "{}", stderr(&refused));
    }
    let usage = run(&["glob", "*", "a"]);
    assert_eq!(usage.status.code(), Some(2));
    let help = run(&["glob", "--help"]);
    assert!(help.status.success());
    for flag in ["--mode", "--surface", "--max-work"] {
        assert!(stdout_utf8(&help).contains(flag));
    }
}
