// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parsing distinct blank-node labels is INJECTIVE, as the SPARQL evaluator sees
//! it: `COUNT(DISTINCT ?s)` over a document of five distinct `_:` tokens is 5.
//!
//! This is the query-visible half of the blank-label codec's contract; the
//! document-level half (parse identity, byte fixpoint, RDFC canonicalization,
//! scope envelopes, standardize-apart, and the injectivity property test) is
//! pinned in `crates/rdf/tests/blank_label_injectivity.rs`. Both drive the same
//! probe, because a conflation on the parse path is not a formatting detail: it
//! changes the answers a query returns.
//!
//! The regression class it exists for: an egress transform that maps the legal
//! label alphabet onto a PROPER SUBSET of itself cannot be injective, so no
//! ingress decode can undo it. The old encoding doubled raw dots (`a.b` → `a..b`,
//! making the token `a..b` ambiguous) and decoded the reserved marker without an
//! image check (`purrdfesc_abc` → `abc`), which merged FIVE distinct legal labels
//! into three nodes with no diagnostic — silently changing what the data means,
//! including what it canonicalizes to.

use std::sync::Arc;

use purrdf::sparql::NativeSparqlEngine;
use purrdf::{RdfDataset, SparqlEngine, SparqlRequest, SparqlResult, TermValue, parse_dataset};

const NTRIPLES: &str = "application/n-triples";

/// The adversary's probe, verbatim: five DISTINCT legal `BLANK_NODE_LABEL`s, one
/// per predicate so a merge cannot hide behind quad deduplication.
const PROBE: &str = "_:a.b <https://example.org/p1> \"1\" .\n\
                     _:a..b <https://example.org/p2> \"2\" .\n\
                     _:a...b <https://example.org/p3> \"3\" .\n\
                     _:purrdfesc_abc <https://example.org/p4> \"4\" .\n\
                     _:abc <https://example.org/p5> \"5\" .\n";

/// The same five triples with only THREE distinct subjects — what the defective
/// encoding turned the probe into. Kept as an explicit control, so the assertion
/// on the probe is discriminating rather than vacuous.
const MERGED: &str = "_:a.b <https://example.org/p1> \"1\" .\n\
                      _:a.b <https://example.org/p2> \"2\" .\n\
                      _:a...b <https://example.org/p3> \"3\" .\n\
                      _:abc <https://example.org/p4> \"4\" .\n\
                      _:abc <https://example.org/p5> \"5\" .\n";

fn parse(text: &str) -> Arc<RdfDataset> {
    parse_dataset(text.as_bytes(), NTRIPLES, None)
        .unwrap_or_else(|e| panic!("N-Triples must parse: {e}\n{text}"))
}

/// `SELECT (COUNT(DISTINCT ?s) AS ?n) WHERE { ?s ?p ?o }` through the public
/// engine surface — the same one the CLI's `query` subcommand drives.
fn count_distinct_subjects(dataset: &Arc<RdfDataset>) -> i64 {
    let result = NativeSparqlEngine::new()
        .query(
            dataset,
            SparqlRequest {
                query: "SELECT (COUNT(DISTINCT ?s) AS ?n) WHERE { ?s ?p ?o }",
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("the aggregate query evaluates");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("SELECT must yield Solutions");
    };
    assert_eq!(rows.len(), 1, "an aggregate yields exactly one row");
    match rows[0][0] {
        Some(TermValue::Literal {
            ref lexical_form, ..
        }) => lexical_form.parse().expect("COUNT is an integer"),
        ref other => panic!("COUNT must bind a literal, got {other:?}"),
    }
}

#[test]
fn count_distinct_over_the_five_label_probe_is_five() {
    assert_eq!(count_distinct_subjects(&parse(PROBE)), 5);
    assert_eq!(count_distinct_subjects(&parse(MERGED)), 3);
}
