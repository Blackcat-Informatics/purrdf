// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Crate-local SPARQL 1.1 Update corpus harness.
//!
//! Reads every `*.ru` request under `tests/update/`, parses it with
//! [`SparqlParser::parse_update`], and asserts each parses `Ok`. The corpus
//! covers one representative request per operation kind (INSERT/DELETE DATA,
//! DELETE WHERE, DELETE/INSERT modify, WITH, USING, LOAD, CLEAR/DROP/CREATE,
//! ADD/MOVE/COPY, an RDF-1.2 quoted-triple insert, and a multi-operation
//! sequence).
//!
//! This is deliberately crate-local: it must NOT touch the repo-root `queries/`
//! tree nor the `.rq` count gate in `tests/corpus.rs`.

use std::fs;
use std::path::Path;

use purrdf_sparql_algebra::SparqlParser;

#[test]
fn update_corpus_parses() {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/update"));
    let mut count = 0usize;
    for entry in fs::read_dir(dir).expect("read tests/update dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("ru") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read .ru fixture");
        let parsed = SparqlParser::new().parse_update(&text);
        assert!(
            parsed.is_ok(),
            "fixture {} must parse, got {:?}",
            path.display(),
            parsed.err()
        );
        count += 1;
    }
    // Guard against a silently-empty corpus (e.g. a glob/path regression).
    assert!(
        count >= 8,
        "expected at least 8 .ru fixtures, found {count}"
    );
}

/// An Update request's display re-parses to the same request, with every literal
/// in the canonical term form: a control character or C1 scalar the Rust debug
/// form would spell `\u{…}` is written as SPARQL reads it.
#[test]
fn update_display_writes_literals_in_term_syntax_and_round_trips() {
    let parser = SparqlParser::new();
    for (text, expected) in [
        (
            "INSERT DATA { <http://example.org/s> <http://example.org/p> \"a\\u0001b\u{85}\\\"\" }",
            "INSERT DATA { <http://example.org/s> <http://example.org/p> \"a\\u0001b\u{85}\\\"\" . }",
        ),
        (
            "INSERT DATA { <http://example.org/s> <http://example.org/p> \"x\"@ar--rtl }",
            "INSERT DATA { <http://example.org/s> <http://example.org/p> \"x\"@ar--rtl . }",
        ),
        (
            "INSERT DATA { <http://example.org/s> <http://example.org/p> 1 }",
            "INSERT DATA { <http://example.org/s> <http://example.org/p> \
             \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> . }",
        ),
        (
            "INSERT DATA { <http://example.org/s> <http://example.org/p> \"plain\" }",
            "INSERT DATA { <http://example.org/s> <http://example.org/p> \"plain\" . }",
        ),
    ] {
        let update = parser.parse_update(text).expect("the request parses");
        let written = update.to_string();
        assert_eq!(written, expected);
        let again = parser
            .parse_update(&written)
            .expect("the display re-parses");
        assert_eq!(again.to_string(), written);
    }
}
