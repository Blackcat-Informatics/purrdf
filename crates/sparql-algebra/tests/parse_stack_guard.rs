// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The parser's stack guard: how deep a query may nest is the stack of the thread
//! parsing it, not a count. A query parsed where the thread has too little stack left
//! for it is the typed [`ParseError::StackExhausted`] — never an aborted process — and
//! the same query parses on a thread with room for it. A tree the parser returns can be
//! copied, compared, hashed, formatted, serialized and dropped from the frame that parsed
//! it, however close to the end of the stack that parse ran.
//!
//! Each parse runs beneath frames that eat the thread's stack down to a chosen amount,
//! measured with the guard's own [`purrdf_stack::remaining`]: a thread's real stack can be
//! larger than it asked for (the C library reuses a cached stack of up to several times
//! the requested size), so a test that trusted the request would pass or fail with the
//! order the harness ran it in. A stack overflow aborts the whole test process, so every
//! assertion reached is itself the proof that nothing overflowed.

use std::hash::{Hash, Hasher};

use purrdf_sparql_algebra::{
    GraphPattern, ParseError, Query, SparqlParser, TermPattern, pattern_to_select_query,
};

const EX: &str = "http://example.org/";

/// Left for the parse: 64 KiB above the margin. Every deep form below needs more than
/// that, and a shallow query needs a few KiB of it.
const SMALL: usize = purrdf_stack::MARGIN_BYTES + 64 * 1024;

/// Left for the parse: 256 MiB, far more than any form below needs.
const LARGE: usize = 256 * 1024 * 1024;

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

/// Every walk a caller makes over a parsed tree, from the frame that parsed it: a copy,
/// the comparison of the copy with the original, a hash, the `Debug` form, the SPARQL
/// rendering of the pattern, and both drops. Returns the `Debug` form's length, so the
/// walk cannot be optimized away.
fn walk_everything(query: Query) -> usize {
    let copy = query.clone();
    assert_eq!(copy, query, "a copy compares equal to its original");
    let mut hasher = std::hash::DefaultHasher::new();
    copy.hash(&mut hasher);
    let debug = format!("{copy:?}").len();
    let (Query::Select { pattern, .. }
    | Query::Ask { pattern, .. }
    | Query::Construct { pattern, .. }
    | Query::Describe { pattern, .. }) = &copy;
    let rendered = pattern_to_select_query(pattern).len();
    drop(copy);
    drop(query);
    core::hint::black_box(hasher.finish());
    debug + rendered
}

/// Parse `query` with `bytes` of stack left and walk the tree it built from the same
/// frame: `Ok(())`, or the refusal.
fn parse_on(query: &str, bytes: usize) -> Result<(), ParseError> {
    let query = query.to_owned();
    on_stack(bytes, move || {
        SparqlParser::new().parse_query(&query).map(|parsed| {
            core::hint::black_box(walk_everything(parsed));
        })
    })
}

/// `open` written `n` times around `core`, closed by `close` written `n` times.
fn nested(open: &str, core: &str, close: &str, n: usize) -> String {
    format!("{}{core}{}", open.repeat(n), close.repeat(n))
}

/// A form of every kind of tree the parser builds, written `n` levels deep.
struct Form {
    name: &'static str,
    text: fn(usize) -> String,
}

/// The tree-building forms: each recursive production family, the cheapest recursion per
/// tree level (`!`), operators a loop builds under brackets, a sibling spine under
/// nested groups, and nested triple terms.
fn forms() -> Vec<Form> {
    vec![
        Form {
            name: "nested groups",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    nested("{ ", &format!("?s <{EX}p> ?o"), " }", n)
                )
            },
        },
        Form {
            name: "nested OPTIONAL",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    nested(
                        &format!("?s <{EX}p> ?o OPTIONAL {{ "),
                        &format!("?s <{EX}q> ?o"),
                        " }",
                        n
                    )
                )
            },
        },
        Form {
            name: "nested built-in calls",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s <{EX}p> ?o FILTER({}) }}",
                    nested("ABS(", "?o", ")", n)
                )
            },
        },
        Form {
            name: "nested unary minus",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s <{EX}p> ?o FILTER({} = 1) }}",
                    nested("-(", "?o", ")", n)
                )
            },
        },
        Form {
            name: "negation chain",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s <{EX}p> ?o FILTER({}?o) }}",
                    "!".repeat(n)
                )
            },
        },
        Form {
            name: "nested brackets",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s <{EX}p> ?o FILTER({} = 1) }}",
                    nested("(", "?o", ")", n)
                )
            },
        },
        Form {
            name: "operator levels under brackets",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s <{EX}p> ?o FILTER({}) }}",
                    nested("(?o || ?o && ?o != ?o + ?o * ", "?o", ")", n)
                )
            },
        },
        Form {
            name: "nested sub-SELECTs",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    nested("{ SELECT * WHERE { ", &format!("?s <{EX}p> ?o"), " } }", n)
                )
            },
        },
        Form {
            name: "nested FILTER NOT EXISTS",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    nested(
                        &format!("?s <{EX}p> ?o FILTER NOT EXISTS {{ "),
                        &format!("?s <{EX}q> ?o"),
                        " }",
                        n
                    )
                )
            },
        },
        Form {
            name: "nested inverse paths",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s {} ?o }}",
                    nested("^(", &format!("<{EX}p>"), ")", n)
                )
            },
        },
        Form {
            name: "nested path groups in chains",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s {} ?o }}",
                    nested(
                        &format!("<{EX}p>/("),
                        &format!("<{EX}q>"),
                        &format!(")|<{EX}r>"),
                        n
                    )
                )
            },
        },
        Form {
            name: "nested triple terms",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} <{EX}q> ?z }}",
                    nested(&format!("<<( ?s <{EX}p> "), "?o", " )>>", n)
                )
            },
        },
        Form {
            name: "nested VALUES triple terms",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ VALUES ?x {{ {} }} }}",
                    nested(&format!("<<( <{EX}s> <{EX}p> "), "1", " )>>", n)
                )
            },
        },
        Form {
            name: "a spine of 1 500 OPTIONAL siblings under nested groups",
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    nested(
                        "{ ",
                        &format!(
                            "?s <{EX}p> ?o {}",
                            format!("OPTIONAL {{ ?s <{EX}q> ?o }} ").repeat(1_500)
                        ),
                        " }",
                        n
                    )
                )
            },
        },
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
    for form in forms() {
        let query = (form.text)(1_000);
        assert_stack_refusal(&parse_on(&query, SMALL), form.name);
        // The valid neighbour: the very same text, with room for it.
        parse_on(&query, LARGE).unwrap_or_else(|e| {
            panic!("{} a thousand deep parses on a large stack: {e}", form.name)
        });
    }
}

#[test]
fn the_small_stack_still_parses_a_shallow_query() {
    // The refusal is about the stack this query needs, not about the thread: the same
    // small stack parses the one-level twin of every deep form.
    for form in forms().into_iter().filter(|f| !f.name.contains("spine")) {
        parse_on(&(form.text)(1), SMALL)
            .unwrap_or_else(|e| panic!("{} one level deep on a small stack: {e}", form.name));
    }
    // A long flat chain costs the parser no stack: its loop builds it.
    let chain = format!(
        "SELECT * WHERE {{ ?s <{EX}p> ?o FILTER(?o{}) }}",
        " + ?o".repeat(400)
    );
    parse_on(&chain, SMALL).unwrap_or_else(|e| panic!("a flat chain on a small stack: {e}"));
}

/// The largest `n` whose form parses (and is walked) with `bytes` left, found by
/// bisection, and the refusal one level deeper.
fn deepest(form: &Form, bytes: usize) -> (usize, Result<(), ParseError>) {
    let (mut parses, mut refused) = (1_usize, 50_000_usize);
    parse_on(&(form.text)(parses), bytes)
        .unwrap_or_else(|e| panic!("{}: one level parses with {bytes} bytes: {e}", form.name));
    while refused - parses > 1 {
        let mid = parses.midpoint(refused);
        if parse_on(&(form.text)(mid), bytes).is_ok() {
            parses = mid;
        } else {
            refused = mid;
        }
    }
    (parses, parse_on(&(form.text)(refused), bytes))
}

/// The real limit of every form on a 1 MiB and a 2 MiB stack: the deepest level that
/// parses is walked in full from the frame that parsed it — copied, compared, hashed,
/// formatted, rendered and dropped — and the next level is the typed stack refusal. The
/// walks run where the parse ran out of room, so an overflow in any of them would abort
/// this process here. Every form reaches past the 127 levels the removed count admitted
/// (a written sub-`SELECT` or `NOT EXISTS` level is two recursive levels; the spine
/// costs its combinator budget).
#[test]
fn the_deepest_tree_a_stack_parses_is_walked_there_and_one_level_more_is_refused() {
    for bytes in [1024 * 1024, 2 * 1024 * 1024] {
        for form in forms() {
            let (parses, refused) = deepest(&form, bytes);
            assert_stack_refusal(&refused, form.name);
            let floor = if form.name.contains("sub-SELECT") || form.name.contains("NOT EXISTS") {
                63
            } else if form.name.contains("spine") {
                1
            } else {
                127
            };
            assert!(
                parses >= floor,
                "{}: only {parses} levels parse with {bytes} bytes left",
                form.name
            );
        }
    }
}

#[test]
fn every_stack_between_the_margin_and_the_need_parses_or_refuses_typed() {
    // Wherever in the descent the stack runs out, the parse returns: it parses (a stack
    // with room) or it is the stack refusal (one without) — never an abort, which would
    // take the whole test process down, and never any other error. Each tree that parses
    // is walked in full from the same frame.
    let floor = purrdf_stack::MARGIN_BYTES;
    for form in forms() {
        let query = (form.text)(200);
        let mut parsed_at = None;
        for step in 0..48 {
            let bytes = floor + step * 64 * 1024;
            match parse_on(&query, bytes) {
                Ok(()) => {
                    parsed_at.get_or_insert(bytes);
                }
                refused => {
                    assert_stack_refusal(&refused, form.name);
                    assert!(
                        parsed_at.is_none(),
                        "{}: refused at {bytes} bytes after parsing with less",
                        form.name
                    );
                }
            }
        }
        assert!(
            parsed_at.is_some(),
            "{}: parses within {} KiB",
            form.name,
            (floor + 47 * 64 * 1024) / 1024
        );
    }
}

/// Triple terms nest as deep as the stack holds them, like every other construct: a
/// thousand levels in a pattern and in `VALUES` parse with every level present — the
/// height check reports all thousand, the oracle that no level was dropped — where the
/// removed count refused the 129th, and a thousand nested reifying triples parse too.
#[test]
fn triple_terms_nest_as_deep_as_the_stack_holds_them() {
    for (name, text) in [
        (
            "pattern",
            format!(
                "SELECT * WHERE {{ {} <{EX}q> ?z }}",
                nested(&format!("<<( ?s <{EX}p> "), "?o", " )>>", 1_000)
            ),
        ),
        (
            "VALUES",
            format!(
                "SELECT * WHERE {{ VALUES ?x {{ {} }} }}",
                nested(&format!("<<( <{EX}s> <{EX}p> "), "1", " )>>", 1_000)
            ),
        ),
    ] {
        let nesting = on_stack(LARGE, move || {
            let query = SparqlParser::new()
                .parse_query(&text)
                .unwrap_or_else(|e| panic!("{name}: a thousand nested triple terms parse: {e}"));
            let (Query::Select { pattern, .. }
            | Query::Ask { pattern, .. }
            | Query::Construct { pattern, .. }
            | Query::Describe { pattern, .. }) = &query;
            pattern
                .validate_height()
                .expect("the height check admits it")
        });
        assert_eq!(
            nesting, 1_000,
            "{name}: every level of the term is in the algebra"
        );
    }
    let reifying = format!(
        "SELECT * WHERE {{ {} <{EX}q> ?z }}",
        nested(&format!("<< ?s <{EX}p> "), "?o", " >>", 1_000)
    );
    let parsed = on_stack(LARGE, move || {
        SparqlParser::new()
            .parse_query(&reifying)
            .map(|q| format!("{q:?}"))
    })
    .expect("a thousand nested reifying triples parse");
    assert_eq!(parsed.matches("rdf-syntax-ns#reifies").count(), 1_000);
}

/// [`TermPattern::triple_term_nesting`] counts the triple terms of the longest chain,
/// through subjects and objects alike, and nothing for a term that is not one.
#[test]
fn triple_term_nesting_counts_the_longest_chain() {
    let leaf = TermPattern::Variable(purrdf_sparql_algebra::Variable::new("o"));
    let wrap = |subject: TermPattern, object: TermPattern| {
        TermPattern::Triple(Box::new(purrdf_sparql_algebra::TriplePattern {
            subject,
            predicate: purrdf_sparql_algebra::NamedNodePattern::NamedNode(
                purrdf_sparql_algebra::NamedNode::new(format!("{EX}p")).expect("IRI"),
            ),
            object,
        }))
    };
    assert_eq!(leaf.triple_term_nesting(), 0);
    let mut deep = leaf.clone();
    for _ in 0..5 {
        deep = wrap(leaf.clone(), deep);
    }
    assert_eq!(deep.triple_term_nesting(), 5, "down the objects");
    let subject_side = wrap(deep, leaf.clone());
    assert_eq!(subject_side.triple_term_nesting(), 6, "down a subject");
    let mut long = leaf.clone();
    for _ in 0..100_000 {
        long = wrap(leaf.clone(), long);
    }
    assert_eq!(
        long.triple_term_nesting(),
        100_000,
        "counted without recursion"
    );
    // Dropped level by level, so the test's stack never holds its drop.
    while let TermPattern::Triple(triple) = long {
        long = triple.object;
    }
}

/// A tall tree is held to the same measure when it is admitted as when it is parsed:
/// [`Query::validate`] and [`GraphPattern::validate_height`] accept it where the stack
/// holds its walks and refuse it, typed, where it does not.
#[test]
fn a_tall_tree_is_admitted_by_the_stack_it_would_be_walked_on() {
    fn heights(query: &Query) -> [(&'static str, Result<(), ParseError>); 2] {
        let (Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }) = query;
        let pattern: &GraphPattern = pattern;
        [
            ("Query::validate", query.validate()),
            (
                "GraphPattern::validate_height",
                pattern.validate_height().map(drop),
            ),
        ]
    }
    let query = on_stack(LARGE, || {
        SparqlParser::new()
            .parse_query(&format!(
                "SELECT * WHERE {{ ?s <{EX}p> ?o FILTER({}?o) }}",
                "!".repeat(2_000)
            ))
            .expect("two thousand negations parse on a large stack")
    });
    let copy = query.clone();
    for (what, admitted) in on_stack(LARGE, move || heights(&copy)) {
        admitted.unwrap_or_else(|e| panic!("{what} on a large stack: {e}"));
    }
    for (what, refused) in on_stack(SMALL, move || heights(&query)) {
        assert!(
            matches!(refused, Err(ParseError::StackExhausted { .. })),
            "{what} on a small stack: {refused:?}"
        );
    }
}
