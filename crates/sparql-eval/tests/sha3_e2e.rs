// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SEP-0008 SHA-3 built-ins, end to end, from the vantage a host has: real
//! query text with the HYPHENATED keyword in it, parsed and evaluated through
//! the PUBLIC [`NativeSparqlEngine`] entry point.
//!
//! The unit tests in `purrdf-sparql-eval`'s `expr` module already pin the four
//! digests against their published NIST FIPS 202 vectors, and the algebra
//! crate's parser tests pin the single-token reading of `SHA3-224`. Neither
//! proves the two halves MEET: a lexer/parser that read the name correctly and
//! an evaluator that computed the digest correctly could still be joined by a
//! dispatch table that sent `SHA3-384` to the 256-bit arm. This file is the
//! join, exercised the way a caller reaches it — from query text.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::NativeSparqlEngine;

/// The fixture namespace (AGENTS.md: fixtures live under `example.org`).
const EX: &str = "https://example.org/sha3#";

/// `:s :message "abc"` — the message every published SHA-3 example table starts
/// from, so each expected digest below is a citable value rather than a
/// recorded one.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{EX}s"));
    let message = b.intern_iri(&format!("{EX}message"));
    let abc = b.intern_literal(RdfLiteral {
        lexical_form: "abc".to_owned(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_owned()),
        language: None,
        direction: None,
    });
    b.push_quad(s, message, abc, None);
    b.freeze().expect("freeze the fixture")
}

/// Evaluate `SELECT (<call> AS ?h) WHERE { ?s :message ?m }` and return the one
/// bound lexical form.
fn digest_of(call: &str) -> String {
    let dataset = dataset();
    let query = format!("PREFIX : <{EX}> SELECT ({call} AS ?h) WHERE {{ ?s :message ?m }}");
    let result = NativeSparqlEngine::new()
        .query(
            &dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .unwrap_or_else(|e| panic!("evaluate `{query}`: {e}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    assert_eq!(rows.len(), 1, "the fixture binds exactly one row");
    match rows[0].first() {
        Some(Some(TermValue::Literal { lexical_form, .. })) => lexical_form.clone(),
        other => panic!("expected a bound literal digest, got {other:?}"),
    }
}

/// Each hyphenated name must reach ITS OWN digest arm — the four published
/// `"abc"` vectors from NIST FIPS 202, in size order.
#[test]
fn each_sha3_name_reaches_its_own_digest_from_query_text() {
    assert_eq!(
        digest_of("SHA3-224(?m)"),
        "e642824c3f8cf24ad09234ee7d3c766fc9a3a5168d0c94ad73b46fdf"
    );
    assert_eq!(
        digest_of("SHA3-256(?m)"),
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
    );
    assert_eq!(
        digest_of("SHA3-384(?m)"),
        "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b2\
         98d88cea927ac7f539f1edf228376d25"
    );
    assert_eq!(
        digest_of("SHA3-512(?m)"),
        "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e\
         10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0"
    );
}

/// SHA-3 is NOT SHA-2: the four SHA-3 names must not collide with the four
/// SHA-1/SHA-2 names the engine already had. A dispatch table that routed
/// `SHA3-256` to `SHA256` would pass a "does it return 64 hex chars" check.
#[test]
fn sha3_does_not_collide_with_the_sha1_sha2_builtins() {
    assert_ne!(digest_of("SHA3-256(?m)"), digest_of("SHA256(?m)"));
    assert_ne!(digest_of("SHA3-384(?m)"), digest_of("SHA384(?m)"));
    assert_ne!(digest_of("SHA3-512(?m)"), digest_of("SHA512(?m)"));
    assert_ne!(digest_of("SHA3-224(?m)"), digest_of("SHA1(?m)"));
}

/// A FILTER over the hyphenated call proves the name survives in a position
/// where the `-` sits between two expression-shaped operands, which is the
/// place a lexer that split the token would produce a *different valid parse*
/// (a subtraction) rather than an error.
#[test]
fn a_hyphenated_sha3_call_works_inside_a_filter() {
    let dataset = dataset();
    let query = format!(
        "PREFIX : <{EX}> ASK {{ ?s :message ?m \
         FILTER(SHA3-256(?m) = \
         \"3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532\") }}"
    );
    let result = NativeSparqlEngine::new()
        .query(
            &dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("evaluate the ASK");
    assert!(
        matches!(result, SparqlResult::Boolean(true)),
        "the FILTER over SHA3-256(?m) must hold"
    );
}
