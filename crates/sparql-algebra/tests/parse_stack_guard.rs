// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The parser's stack guard: a query inside the nesting limit, parsed where the thread has
//! too little stack left for it, is the typed [`ParseError::StackExhausted`] — never an
//! aborted process — and the same query parses on a thread with room for it.
//!
//! Each parse runs beneath frames that eat the thread's stack down to a chosen amount,
//! measured with the guard's own [`purrdf_stack::remaining`]: a thread's real stack can be
//! larger than it asked for (the C library reuses a cached stack of up to several times
//! the requested size), so a test that trusted the request would pass or fail with the
//! order the harness ran it in.

use purrdf_sparql_algebra::{MAX_NESTING_DEPTH, ParseError, SparqlParser};

const EX: &str = "http://example.org/";

/// Left for the parse: 64 KiB above the margin. Every deep form below needs more than
/// that, and a shallow query needs a few KiB of it.
const SMALL: usize = purrdf_stack::MARGIN_BYTES + 64 * 1024;

/// Left for the parse: 64 MiB, far more than any admitted query needs.
const LARGE: usize = 64 * 1024 * 1024;

/// Run `body` with exactly `bytes` of stack left below it (to within one 4 KiB frame).
fn on_stack<T: Send + 'static>(bytes: usize, body: impl FnOnce() -> T + Send + 'static) -> T {
    fn descend<T>(bytes: usize, body: impl FnOnce() -> T) -> T {
        if purrdf_stack::remaining() <= bytes {
            return body();
        }
        let frame = core::hint::black_box([0u8; 4096]);
        let value = descend(bytes, body);
        core::hint::black_box(&frame);
        value
    }
    std::thread::Builder::new()
        .stack_size(bytes + 1024 * 1024)
        .spawn(move || descend(bytes, body))
        .expect("spawn")
        .join()
        .expect("the parsing thread returned rather than aborting")
}

/// Parse `query` with `bytes` of stack left: `Ok(())`, or the refusal.
fn parse_on(query: &str, bytes: usize) -> Result<(), ParseError> {
    let query = query.to_owned();
    on_stack(bytes, move || {
        SparqlParser::new().parse_query(&query).map(|_| ())
    })
}

/// `open` written `n` times around `core`, closed by `close` written `n` times.
fn nested(open: &str, core: &str, close: &str, n: usize) -> String {
    format!("{}{core}{}", open.repeat(n), close.repeat(n))
}

/// The deepest forms of every recursive production family the parser admits: each is
/// inside [`MAX_NESTING_DEPTH`] (the `SELECT`'s own `WHERE` group is the first level).
fn deep_forms() -> Vec<(&'static str, String)> {
    let depth = MAX_NESTING_DEPTH - 1;
    let triple = format!("?s <{EX}p> ?o");
    vec![
        (
            "127 nested groups",
            format!(
                "SELECT * WHERE {{ {} }}",
                nested("{ ", &triple, " }", depth)
            ),
        ),
        (
            "126 nested built-in calls",
            format!(
                "SELECT * WHERE {{ {triple} FILTER({}) }}",
                nested("STR(", "?o", ")", depth - 1)
            ),
        ),
        (
            "127 nested brackets",
            format!(
                "SELECT * WHERE {{ {triple} FILTER({}) }}",
                nested("(", "?o", ")", depth)
            ),
        ),
        (
            "63 nested sub-SELECTs",
            format!(
                "SELECT * WHERE {{ {} }}",
                nested("{ SELECT * WHERE { ", &triple, " } }", depth / 2)
            ),
        ),
        (
            "63 nested FILTER NOT EXISTS",
            format!(
                "SELECT * WHERE {{ {} }}",
                nested(
                    &format!("{triple} FILTER NOT EXISTS {{ "),
                    &triple,
                    " }",
                    depth / 2
                )
            ),
        ),
        (
            "127 nested path groups",
            format!(
                "SELECT * WHERE {{ ?s {} ?o }}",
                nested("(", &format!("<{EX}p>"), ")/<http://example.org/q>", depth)
            ),
        ),
    ]
}

/// Assert `refused` is the stack refusal, not a syntax or limit error.
fn assert_stack_refusal(refused: &Result<(), ParseError>, what: &str) {
    match refused {
        Err(ParseError::StackExhausted { construct, .. }) => {
            assert!(
                !construct.is_empty(),
                "{what}: the refusal names a construct"
            );
            let message = refused.as_ref().expect_err(what).to_string();
            assert!(
                message.contains("stack exhausted") && message.contains(construct),
                "{what}: {message}"
            );
        }
        other => panic!("{what}: expected the stack refusal, got {other:?}"),
    }
}

#[test]
fn a_deep_query_on_a_small_stack_is_the_typed_refusal_and_parses_on_a_large_one() {
    for (what, query) in deep_forms() {
        assert_stack_refusal(&parse_on(&query, SMALL), what);
        // The valid neighbour: the very same text, with room for it.
        parse_on(&query, LARGE).unwrap_or_else(|e| panic!("{what} parses on a large stack: {e}"));
    }
}

#[test]
fn the_small_stack_still_parses_a_shallow_query() {
    // The refusal is about the stack this query needs, not about the thread: the same
    // small stack parses the one-level twin of every deep form.
    let triple = format!("?s <{EX}p> ?o");
    for query in [
        format!("SELECT * WHERE {{ {{ {triple} }} }}"),
        format!("SELECT * WHERE {{ {triple} FILTER(STR(?o)) }}"),
        format!("SELECT * WHERE {{ {{ SELECT * WHERE {{ {triple} }} }} }}"),
        format!("SELECT * WHERE {{ {triple} FILTER NOT EXISTS {{ {triple} }} }}"),
        format!("SELECT * WHERE {{ ?s (<{EX}p>)/<{EX}q> ?o }}"),
        // A long flat chain costs the parser no stack: its loop builds it.
        format!(
            "SELECT * WHERE {{ {triple} FILTER(?o{}) }}",
            " + ?o".repeat(400)
        ),
    ] {
        parse_on(&query, SMALL).unwrap_or_else(|e| panic!("`{query}` on a small stack: {e}"));
    }
}

#[test]
fn a_query_beyond_the_nesting_limit_is_refused_by_the_limit_where_the_stack_reaches_it() {
    let query = format!(
        "SELECT * WHERE {{ {} }}",
        nested("{ ", &format!("?s <{EX}p> ?o"), " }", MAX_NESTING_DEPTH)
    );
    // With room to reach the limit, the limit is what refuses it: the stack guard adds no
    // second reason to a query the count already refuses.
    match parse_on(&query, LARGE) {
        Err(ParseError::Syntax { reason, .. }) => {
            assert!(reason.contains("safety limit"), "{reason}");
        }
        other => panic!("expected the nesting-limit refusal, got {other:?}"),
    }
    // Without it, the stack runs out first, and that is the refusal.
    assert_stack_refusal(&parse_on(&query, SMALL), "129 nested groups");
}

#[test]
fn every_stack_between_the_margin_and_the_need_parses_or_refuses_typed() {
    // Wherever in the descent the stack runs out, the parse returns: it parses (a stack
    // with room) or it is the stack refusal (one without) — never an abort, which would
    // take the whole test process down, and never any other error.
    let floor = purrdf_stack::MARGIN_BYTES;
    for (what, query) in deep_forms() {
        let mut parsed_at = None;
        for step in 0..24 {
            let bytes = floor + step * 32 * 1024;
            match parse_on(&query, bytes) {
                Ok(()) => {
                    parsed_at.get_or_insert(bytes);
                }
                refused => {
                    assert_stack_refusal(&refused, what);
                    assert!(
                        parsed_at.is_none(),
                        "{what}: refused at {bytes} bytes after parsing with less"
                    );
                }
            }
        }
        assert!(
            parsed_at.is_some(),
            "{what}: parses within {} KiB",
            (floor + 23 * 32 * 1024) / 1024
        );
    }
}
