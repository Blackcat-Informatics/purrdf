// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! How deeply a request nests is bounded by memory, not by the stack of the thread that
//! parses it.
//!
//! The parser keeps what encloses the cursor on heap-allocated stacks, and every walk
//! over the tree it returns runs over a work list, so a request nested a hundred
//! thousand levels deep — in any construct of the query or update grammar — parses on
//! a thread spawned with a 128 KiB stack, and the tree it built is formatted and
//! dropped on that same thread. A stack overflow aborts the whole test process, so
//! every assertion reached is itself the proof that nothing overflowed.
//!
//! Each family's algebra is checked against an oracle that sees every level: a marker
//! its `Debug` form holds once per written level (or a fixed number of times, where a
//! level builds no node of its own), so a truncated or dropped level fails the count.

use purrdf_testkit::text::nested as wrapped;
use std::fmt::Write as _;

use purrdf_sparql_algebra::{
    Expression, GraphPattern, ParseError, ParserOptions, Query, SparqlParser,
};

/// The stack every deep parse below runs on.
const SMALL_STACK: usize = 128 * 1024;

/// How many levels every family is written.
const LEVELS: usize = 100_000;

const EX_P: &str = "<http://example.org/p>";

/// The property-function namespace the argument-list family declares.
const PF_NS: &str = "http://example.org/pf/";

/// What a family's text is parsed as.
#[derive(Clone, Copy)]
enum Form {
    Query,
    Update,
    /// A query under a property-function namespace ([`PF_NS`]).
    QueryWithPropertyFunctions,
}

/// One construct of the query or update grammar, written `levels` deep.
struct Family {
    /// What the family is, for assertion messages.
    name: &'static str,
    form: Form,
    /// The request, written `levels` deep.
    text: fn(usize) -> String,
    /// A substring of the parsed algebra's `Debug` form, and how often it occurs once
    /// `levels` levels were parsed.
    marker: &'static str,
    occurrences: fn(usize) -> usize,
}

impl Family {
    /// Parse the family `levels` deep and return its algebra's `Debug` form; the tree is
    /// dropped before this returns.
    fn parse(&self, levels: usize) -> Result<String, ParseError> {
        let text = (self.text)(levels);
        match self.form {
            Form::Query => SparqlParser::new()
                .parse_query(&text)
                .map(|q| format!("{q:?}")),
            Form::Update => SparqlParser::new()
                .parse_update(&text)
                .map(|u| format!("{u:?}")),
            Form::QueryWithPropertyFunctions => {
                let options = ParserOptions {
                    property_fn_namespaces: vec![PF_NS.to_owned()],
                    ..ParserOptions::default()
                };
                SparqlParser::new()
                    .parse_query_with(&text, &options)
                    .map(|q| format!("{q:?}"))
            }
        }
    }
}

/// Every construct that nests, in queries and updates.
#[allow(
    clippy::too_many_lines,
    reason = "one entry per construct; splitting the table would scatter it"
)]
fn families() -> Vec<Family> {
    vec![
        Family {
            name: "group graph pattern",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped("{ ?s <http://example.org/g> ?o ", "", " }", n)
                )
            },
            marker: "<http://example.org/g>",
            occurrences: |n| n,
        },
        Family {
            name: "OPTIONAL",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped("?s ?p ?o OPTIONAL { ", "?s ?p ?innermost", " }", n)
                )
            },
            marker: "LeftJoin {",
            occurrences: |n| n,
        },
        Family {
            name: "LATERAL",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped("?s ?p ?o LATERAL { ", "?s ?p ?innermost", " }", n)
                )
            },
            marker: "Lateral {",
            occurrences: |n| n,
        },
        Family {
            name: "MINUS",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped("?s ?p ?o MINUS { ", "?s ?p ?innermost", " }", n)
                )
            },
            marker: "Minus {",
            occurrences: |n| n,
        },
        Family {
            name: "GRAPH",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped(
                        "?s ?p ?o GRAPH <http://example.org/g> { ",
                        "?s ?p ?innermost",
                        " }",
                        n
                    )
                )
            },
            marker: "Graph {",
            occurrences: |n| n,
        },
        Family {
            name: "SERVICE",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped(
                        "?s ?p ?o SERVICE <http://example.org/ep> { ",
                        "?s ?p ?innermost",
                        " }",
                        n
                    )
                )
            },
            marker: "Service {",
            occurrences: |n| n,
        },
        Family {
            name: "UNION arm",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped("{ ?s ?p ?o } UNION { ", "?s ?p ?innermost", " }", n)
                )
            },
            marker: "Union {",
            occurrences: |n| n,
        },
        Family {
            name: "sub-SELECT",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped("{ SELECT * WHERE { ", "?s ?p ?innermost", " } }", n)
                )
            },
            marker: "Project {",
            occurrences: |n| n + 1,
        },
        // Each sub-SELECT's `WHERE` group is directly the next sub-SELECT, with no group
        // of its own around it: `{ SELECT * WHERE { SELECT * WHERE … } }`.
        Family {
            name: "sub-SELECT as the WHERE group",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped("{ SELECT * WHERE ", "{ ?s ?p ?innermost }", " }", n)
                )
            },
            marker: "Project {",
            occurrences: |n| n + 1,
        },
        Family {
            name: "FILTER EXISTS",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped("?s ?p ?o FILTER EXISTS { ", "?s ?p ?innermost", " }", n)
                )
            },
            marker: "Exists(",
            occurrences: |n| n,
        },
        Family {
            name: "FILTER NOT EXISTS",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped("?s ?p ?o FILTER NOT EXISTS { ", "?s ?p ?innermost", " }", n)
                )
            },
            marker: "Not(Exists(",
            occurrences: |n| n,
        },
        Family {
            name: "bracketted expression",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ FILTER({}) }}",
                    wrapped("(", "?innermost", ")", n)
                )
            },
            // Brackets build no node: the oracle is that the innermost operand survived.
            marker: "?innermost",
            occurrences: |_| 1,
        },
        Family {
            name: "logical negation",
            form: Form::Query,
            text: |n| format!("SELECT * WHERE {{ FILTER({}?innermost) }}", "!".repeat(n)),
            marker: "Not(",
            occurrences: |n| n,
        },
        Family {
            name: "unary minus",
            form: Form::Query,
            text: |n| format!("SELECT * WHERE {{ FILTER({}?innermost) }}", "- ".repeat(n)),
            marker: "UnaryMinus(",
            occurrences: |n| n,
        },
        Family {
            name: "unary plus",
            form: Form::Query,
            text: |n| format!("SELECT * WHERE {{ FILTER({}?innermost) }}", "+ ".repeat(n)),
            marker: "UnaryPlus(",
            occurrences: |n| n,
        },
        Family {
            name: "function call",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ FILTER({}) }}",
                    wrapped("<http://example.org/fn>(", "?innermost", ")", n)
                )
            },
            marker: "Custom(<http://example.org/fn>)",
            occurrences: |n| n,
        },
        Family {
            name: "built-in call",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ FILTER({}) }}",
                    wrapped("STR(", "?innermost", ")", n)
                )
            },
            marker: "FunctionCall(Str,",
            occurrences: |n| n,
        },
        Family {
            name: "IN list",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ FILTER({}) }}",
                    wrapped("?x IN (", "?innermost", ")", n)
                )
            },
            marker: "In(Variable(?x)",
            occurrences: |n| n,
        },
        Family {
            name: "operator levels under brackets",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ FILTER({}) }}",
                    wrapped("(?x || ?x && ?x != ?x + ?x * ", "?innermost", ")", n)
                )
            },
            marker: "Or(",
            occurrences: |n| n,
        },
        Family {
            name: "expression triple term",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ FILTER(?x = {}) }}",
                    wrapped("<<( ?s ?p ", "?innermost", " )>>", n)
                )
            },
            marker: "FunctionCall(Triple,",
            occurrences: |n| n,
        },
        Family {
            name: "aggregate over nested calls",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT (SUM({}) AS ?total) WHERE {{ ?s ?p ?x }}",
                    wrapped("<http://example.org/fn>(", "?x", ")", n)
                )
            },
            marker: "Custom(<http://example.org/fn>)",
            occurrences: |n| n,
        },
        Family {
            name: "interleaved call, brackets, negation and EXISTS",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} }}",
                    wrapped(
                        "?s ?p ?o FILTER(<http://example.org/fn>((!EXISTS { ",
                        "?s ?p ?innermost",
                        " }))) ",
                        n
                    )
                )
            },
            marker: "Exists(",
            occurrences: |n| n,
        },
        Family {
            name: "property-path group",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s {} ?o }}",
                    wrapped("(", "<http://example.org/innermost>+", ")", n)
                )
            },
            marker: "OneOrMore(",
            occurrences: |_| 1,
        },
        Family {
            name: "property-path group inside chains",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s {} ?o }}",
                    wrapped(
                        "<http://example.org/p>/(",
                        "<http://example.org/innermost>",
                        ")|<http://example.org/q>",
                        n
                    )
                )
            },
            marker: "Union {",
            occurrences: |n| n,
        },
        Family {
            name: "inverse path",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s {} ?o }}",
                    wrapped("^(", "<http://example.org/innermost>+", ")", n)
                )
            },
            marker: "Reverse(",
            occurrences: |n| n,
        },
        Family {
            name: "negated property set in path groups",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s {} ?o }}",
                    wrapped(
                        "!(<http://example.org/p>|^<http://example.org/q>)/(",
                        "!<http://example.org/innermost>",
                        ")",
                        n
                    )
                )
            },
            marker: "NegatedPropertySet(",
            occurrences: |n| n + 1,
        },
        Family {
            name: "blank-node property list",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s {} }}",
                    wrapped(
                        "<http://example.org/p> [ ",
                        "<http://example.org/p> ?innermost",
                        " ]",
                        n
                    )
                )
            },
            marker: "<http://example.org/p>",
            occurrences: |n| n + 1,
        },
        Family {
            name: "collection",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s {EX_P} {} }}",
                    wrapped("( ", "?innermost", " )", n)
                )
            },
            marker: "rdf-syntax-ns#first",
            occurrences: |n| n,
        },
        Family {
            name: "triple term",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} <http://example.org/q> ?z }}",
                    wrapped("<<( ?s <http://example.org/p> ", "?innermost", " )>>", n)
                )
            },
            marker: "<http://example.org/p>",
            occurrences: |n| n,
        },
        Family {
            name: "reifying triple",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ {} <http://example.org/q> ?z }}",
                    wrapped("<< ?s <http://example.org/p> ", "?innermost", " >>", n)
                )
            },
            marker: "rdf-syntax-ns#reifies",
            occurrences: |n| n,
        },
        Family {
            name: "annotation block",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s {EX_P} ?o {} }}",
                    wrapped("{| <http://example.org/a> ?innermost ", "", " |}", n)
                )
            },
            // Each block asserts `R <a> ?innermost`, and each block but the first
            // reifies the previous one's `<a>` triple.
            marker: "<http://example.org/a>",
            occurrences: |n| 2 * n - 1,
        },
        Family {
            name: "property-function argument triple term",
            form: Form::QueryWithPropertyFunctions,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ ?s <{PF_NS}f> {} }}",
                    wrapped("<<( ?s <http://example.org/p> ", "?innermost", " )>>", n)
                )
            },
            marker: "<http://example.org/p>",
            occurrences: |n| n,
        },
        Family {
            name: "VALUES triple term",
            form: Form::Query,
            text: |n| {
                format!(
                    "SELECT * WHERE {{ VALUES ?x {{ {} }} }}",
                    wrapped(
                        "<<( <http://example.org/s> <http://example.org/p> ",
                        "1",
                        " )>>",
                        n
                    )
                )
            },
            marker: "<http://example.org/p>",
            occurrences: |n| n,
        },
        Family {
            name: "CONSTRUCT template collection",
            form: Form::Query,
            text: |n| {
                format!(
                    "CONSTRUCT {{ ?s {EX_P} {} }} WHERE {{ ?s ?p ?innermost }}",
                    wrapped("( ", "?innermost", " )", n)
                )
            },
            marker: "rdf-syntax-ns#first",
            occurrences: |n| n,
        },
        Family {
            name: "INSERT DATA blank-node property list",
            form: Form::Update,
            text: |n| {
                format!(
                    "INSERT DATA {{ <http://example.org/s> {} }}",
                    wrapped(
                        "<http://example.org/p> [ ",
                        "<http://example.org/p> 1",
                        " ]",
                        n
                    )
                )
            },
            marker: "<http://example.org/p>",
            occurrences: |n| n + 1,
        },
        Family {
            name: "INSERT DATA collection",
            form: Form::Update,
            text: |n| {
                format!(
                    "INSERT DATA {{ <http://example.org/s> {EX_P} {} }}",
                    wrapped("( ", "1", " )", n)
                )
            },
            marker: "rdf-syntax-ns#first",
            occurrences: |n| n,
        },
        Family {
            name: "INSERT DATA triple term",
            form: Form::Update,
            text: |n| {
                format!(
                    "INSERT DATA {{ {} <http://example.org/q> 1 }}",
                    wrapped(
                        "<<( <http://example.org/s> <http://example.org/p> ",
                        "1",
                        " )>>",
                        n
                    )
                )
            },
            marker: "<http://example.org/p>",
            occurrences: |n| n,
        },
        Family {
            name: "update WHERE expression",
            form: Form::Update,
            text: |n| {
                format!(
                    "DELETE {{ ?s ?p ?o }} WHERE {{ ?s ?p ?o FILTER({}?innermost) }}",
                    "!".repeat(n)
                )
            },
            marker: "Not(",
            occurrences: |n| n,
        },
        Family {
            name: "update WHERE group",
            form: Form::Update,
            text: |n| {
                format!(
                    "DELETE {{ ?s ?p ?o }} WHERE {{ {} }}",
                    wrapped("?s ?p ?o OPTIONAL { ", "?s ?p ?innermost", " }", n)
                )
            },
            marker: "LeftJoin {",
            occurrences: |n| n,
        },
    ]
}

/// A wholly linear path is lowered without consuming the thread's stack, even
/// when its syntax is nested. Every edge survives a long sequence, and a hundred
/// thousand inversions cancel by parity instead of leaving recursive path nodes.
#[test]
fn linear_paths_a_hundred_thousand_deep_lower_on_a_small_stack() {
    purrdf_stack::on_stack(SMALL_STACK, || {
        for inversions in [LEVELS, LEVELS + 1] {
            let text = format!(
                "SELECT * WHERE {{ ?s {} ?o }}",
                wrapped("^(", "<http://example.org/innermost>", ")", inversions)
            );
            let Query::Select { pattern, .. } = SparqlParser::new()
                .parse_query(&text)
                .expect("a nested linear inverse parses")
            else {
                panic!("a SELECT");
            };
            let GraphPattern::Project { inner, .. } = pattern else {
                panic!("the projection");
            };
            let GraphPattern::Bgp { patterns } = inner.into_inner() else {
                panic!("inversions lower into one data triple");
            };
            assert_eq!(patterns.len(), 1);
            let names = if inversions % 2 == 0 { ["s", "o"] } else { ["o", "s"] };
            for (term, name) in [(&patterns[0].subject, names[0]), (&patterns[0].object, names[1])] {
                assert!(matches!(term, purrdf_sparql_algebra::TermPattern::Variable(v) if v.as_str() == name));
            }
        }
        let sequence = (0..LEVELS)
            .map(|i| format!("<http://example.org/p{i}>"))
            .collect::<Vec<_>>()
            .join("/");
        let Query::Select { pattern, .. } = SparqlParser::new()
            .parse_query(&format!("SELECT * WHERE {{ ?s {sequence} ?o }}"))
            .expect("a flat linear sequence parses")
        else {
            panic!("a SELECT");
        };
        let GraphPattern::Project { inner, variables } = pattern else {
            panic!("the projection");
        };
        assert_eq!(variables.len(), 2, "every join point stays hidden");
        let GraphPattern::Bgp { patterns } = inner.into_inner() else {
            panic!("the entire sequence is one BGP");
        };
        assert_eq!(patterns.len(), LEVELS);
        for (i, triple) in patterns.iter().enumerate() {
            assert!(matches!(&triple.predicate,
                purrdf_sparql_algebra::NamedNodePattern::NamedNode(p)
                if p.as_str() == format!("http://example.org/p{i}")));
            if let Some(next) = patterns.get(i + 1) {
                assert_eq!(triple.object, next.subject, "edge {i} stays connected");
            }
        }
    })
    .expect("spawn a small-stack thread");
}

/// A right-nested sequence lowers every edge without recursion, just as a flat
/// sequence does, and keeps the anonymous join points outside the projection.
#[test]
fn a_linear_sequence_a_hundred_thousand_levels_deep_stays_connected() {
    purrdf_stack::on_stack(SMALL_STACK, || {
        let sequence = wrapped(
            "<http://example.org/p>/(",
            "<http://example.org/innermost>",
            ")",
            LEVELS,
        );
        let Query::Select { pattern, .. } = SparqlParser::new()
            .parse_query(&format!("SELECT * WHERE {{ ?s {sequence} ?o }}"))
            .expect("a deeply nested linear sequence parses")
        else {
            panic!("a SELECT");
        };
        let GraphPattern::Project { inner, variables } = pattern else {
            panic!("the projection");
        };
        assert_eq!(variables.len(), 2, "every join point stays hidden");
        let GraphPattern::Bgp { patterns } = inner.into_inner() else {
            panic!("the entire nested sequence is one BGP");
        };
        assert_eq!(patterns.len(), LEVELS + 1);
        for (i, triple) in patterns.iter().enumerate() {
            let expected = if i == LEVELS { "innermost" } else { "p" };
            assert!(matches!(&triple.predicate,
                purrdf_sparql_algebra::NamedNodePattern::NamedNode(p)
                if p.as_str() == format!("http://example.org/{expected}")));
            if let Some(next) = patterns.get(i + 1) {
                assert_eq!(triple.object, next.subject, "edge {i} stays connected");
            }
        }
    })
    .expect("spawn a small-stack thread");
}

/// Every construct of the grammar parses a hundred thousand levels deep on a 128 KiB
/// stack, with every level present in the algebra, and the tree is formatted and
/// dropped on that stack too.
#[test]
fn every_construct_parses_a_hundred_thousand_levels_deep_on_a_small_stack() {
    purrdf_stack::on_stack(SMALL_STACK, || {
        for family in families() {
            let algebra = family.parse(LEVELS).unwrap_or_else(|error| {
                panic!("{} written {LEVELS} deep must parse: {error}", family.name)
            });
            assert_eq!(
                algebra.matches(family.marker).count(),
                (family.occurrences)(LEVELS),
                "{}: the parsed algebra holds every one of its {LEVELS} levels",
                family.name
            );
        }
    })
    .expect("spawn a small-stack thread");
}

/// The same families, one level deep, on the same small stack: nothing about the stack
/// changes what a shallow request parses to.
#[test]
fn every_construct_one_level_deep_parses_to_the_same_algebra_on_any_stack() {
    let here: Vec<String> = families()
        .iter()
        .map(|family| family.parse(1).expect("one level parses"))
        .collect();
    let small = purrdf_stack::on_stack(SMALL_STACK, || {
        families()
            .iter()
            .map(|family| family.parse(1).expect("one level parses"))
            .collect::<Vec<_>>()
    })
    .expect("spawn a small-stack thread");
    assert_eq!(here, small);
}

/// A request of sibling elements, by name, written `n` elements long.
type Spine = (&'static str, fn(usize) -> String);

/// A run of sibling elements builds a spine as tall as the run is long: a hundred
/// thousand `OPTIONAL`s, dot-separated complex paths, projection expressions, or an
/// `OPTIONAL` run inside one `UNION` arm parse on a 128 KiB stack, and the tree is
/// copied, compared and dropped there.
#[test]
fn a_hundred_thousand_sibling_elements_parse_on_a_small_stack() {
    let spines: [Spine; 4] = [
        ("OPTIONAL siblings", |n| {
            let mut body = String::from("SELECT * WHERE { ?s <https://example.org/p> ?o ");
            for _ in 0..n {
                body.push_str("OPTIONAL { ?s <https://example.org/q> ?r } ");
            }
            body.push('}');
            body
        }),
        ("dot-separated complex paths", |n| {
            let mut body = String::from("SELECT * WHERE { ");
            for i in 0..n {
                let _ = write!(
                    body,
                    "<https://example.org/s{i}> <https://example.org/p>+ <https://example.org/o{i}> . "
                );
            }
            body.push('}');
            body
        }),
        ("projection expressions", |n| {
            let mut body = String::from("SELECT ");
            for i in 0..n {
                let _ = write!(body, "(1 AS ?v{i}) ");
            }
            body.push_str("WHERE { }");
            body
        }),
        ("an OPTIONAL run inside one UNION arm", |n| {
            let mut body = String::from("SELECT * WHERE { { ?s <https://example.org/p> ?o }");
            for arm in 0..100 {
                body.push_str(" UNION { ?s <https://example.org/p> ?o");
                if arm == 50 {
                    body.push_str(&" OPTIONAL { ?s <https://example.org/q> ?o }".repeat(n));
                }
                body.push_str(" }");
            }
            body.push('}');
            body
        }),
    ];
    purrdf_stack::on_stack(SMALL_STACK, move || {
        for (name, spine) in spines {
            let parsed = SparqlParser::new()
                .parse_query(&spine(LEVELS))
                .unwrap_or_else(|error| panic!("{name}: {LEVELS} parse: {error}"));
            let copy = parsed.clone();
            assert_eq!(copy, parsed, "{name}: a copy compares equal");
            drop(copy);
            drop(parsed);
        }
    })
    .expect("spawn a small-stack thread");
}

/// Aggregates do not nest in SPARQL, and a hundred thousand nested aggregate calls are
/// refused for that — the typed [`ParseError::Unsupported`] of an aggregate inside an
/// aggregate's argument, raised on a 128 KiB stack — while an aggregate over a hundred
/// thousand nested calls parses (see the families above).
#[test]
fn nested_aggregates_are_refused_as_nested_aggregates_at_any_depth() {
    purrdf_stack::on_stack(SMALL_STACK, || {
        for (open, close) in [("SUM(", ")"), ("AGG(<http://example.org/agg>, ", ")")] {
            let text = format!(
                "SELECT ({} AS ?total) WHERE {{ ?s ?p ?x }}",
                wrapped(open, "?x", close, LEVELS)
            );
            let error = SparqlParser::new()
                .parse_query(&text)
                .expect_err("a nested aggregate is refused");
            assert!(
                matches!(&error, ParseError::Unsupported(reason)
                    if reason == "aggregate outside GROUP BY / SELECT / HAVING context"),
                "{open}: {error}"
            );
        }
        // The valid neighbour: one aggregate, its argument a plain variable.
        let flat = SparqlParser::new()
            .parse_query("SELECT (SUM(?x) AS ?total) WHERE { ?s ?p ?x }")
            .expect("one aggregate parses");
        assert!(format!("{flat:?}").contains("Sum"));
    })
    .expect("spawn a small-stack thread");
}

/// The deepest `FILTER` expression of a hundred thousand nested brackets is the
/// innermost operand, reached without any bracket building a node.
#[test]
fn brackets_a_hundred_thousand_deep_build_no_node() {
    purrdf_stack::on_stack(SMALL_STACK, || {
        let text = format!(
            "SELECT * WHERE {{ FILTER({}) }}",
            wrapped("(", "?innermost", ")", LEVELS)
        );
        let Query::Select { pattern, .. } = SparqlParser::new()
            .parse_query(&text)
            .expect("the brackets parse")
        else {
            panic!("a SELECT");
        };
        let GraphPattern::Project { inner, .. } = pattern else {
            panic!("the projection");
        };
        let GraphPattern::Filter { expr, .. } = inner.into_inner() else {
            panic!("the FILTER");
        };
        assert!(matches!(expr, Expression::Variable(v) if v.as_str() == "innermost"));
    })
    .expect("spawn a small-stack thread");
}
