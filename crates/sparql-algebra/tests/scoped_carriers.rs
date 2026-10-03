// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Text carriers preserve existential ownership and refuse concrete blank cells.

#[path = "support/patterns.rs"]
mod patterns;

use purrdf_sparql_algebra::walk::{Flow, NodeRef, Visit, walk_pre_post};
use purrdf_sparql_algebra::{
    BlankNode, Child, GraphPattern, GraphUpdateOperation, GroundTerm, GroundTriple, Literal,
    NamedNodePattern, ParseError, ParserOptions, PropertyFunctionCall, Query, QueryDataset,
    SparqlParser, TermPattern, TriplePattern, Update, Variable, try_pattern_to_select_query,
    try_pattern_to_select_query_with_options,
};

fn raw(label: &str) -> TermPattern {
    TermPattern::BlankNode(BlankNode::new(label))
}

fn bgp(subject: TermPattern, object: TermPattern) -> GraphPattern {
    GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject,
            predicate: NamedNodePattern::NamedNode(patterns::example("p")),
            object,
        }],
    }
}

fn values(value: GroundTerm) -> GraphPattern {
    GraphPattern::Values {
        variables: vec![Variable::new("binding")],
        bindings: vec![vec![Some(value)]],
    }
}

fn blank_labels(pattern: &GraphPattern) -> Vec<String> {
    let mut labels = Vec::new();
    walk_pre_post(NodeRef::Pattern(pattern), |visit, node| {
        if let (Visit::Enter, NodeRef::Term(TermPattern::BlankNode(blank))) = (visit, node) {
            labels.push(blank.as_str().to_owned());
        }
        Flow::Descend
    });
    labels
}

fn parse(text: &str) -> GraphPattern {
    let Query::Select { pattern, .. } = SparqlParser::new()
        .parse_query(text)
        .expect("a checked carrier reparses")
    else {
        panic!("SELECT carrier");
    };
    pattern
}

#[test]
fn concrete_blank_cells_are_legal_algebra_and_refused_before_rendering() {
    let pattern = values(GroundTerm::BlankNode(BlankNode::new("dataset.node")));
    let query = Query::Select {
        pattern: pattern.clone(),
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    };
    query
        .validate()
        .expect("injected dataset blanks are legal algebra");
    let error = try_pattern_to_select_query(&pattern).expect_err("no concrete blank text carrier");
    assert!(matches!(&error, ParseError::Unsupported(_)));
    let reason = error.to_string();
    assert!(reason.contains("dataset.node"));
    assert!(reason.contains("DataBlockValue"));
    assert!(reason.contains("execute the injected algebra locally"));
    assert_eq!(
        error,
        try_pattern_to_select_query_with_options(&pattern, &ParserOptions::default())
            .expect_err("the configured carrier has the same admission")
    );

    let operation = GraphUpdateOperation::DeleteInsert {
        delete: vec![],
        insert: vec![],
        pattern: Box::new(pattern),
        using: vec![],
        with: None,
    };
    assert_eq!(
        error,
        operation
            .try_to_sparql()
            .expect_err("checked operation refuses")
    );
    let update = Update {
        operations: vec![operation],
        base_iri: None,
        version: None,
    };
    assert_eq!(
        error,
        update.try_to_sparql().expect_err("checked request refuses")
    );
}

#[test]
fn quoted_ground_cells_are_admitted_until_a_concrete_blank_occurs() {
    let quoted = |object| {
        GroundTerm::Triple(Child::new(GroundTriple {
            subject: GroundTerm::NamedNode(patterns::example("subject")),
            predicate: patterns::example("predicate"),
            object,
        }))
    };
    let accepted = values(quoted(quoted(GroundTerm::Literal(Literal::new_simple(
        "value",
    )))));
    let carrier =
        try_pattern_to_select_query(&accepted).expect("ground triple terms are legal cells");
    parse(&carrier);
    let rejected = values(quoted(quoted(GroundTerm::BlankNode(BlankNode::new(
        "concrete",
    )))));
    assert!(matches!(
        try_pattern_to_select_query(&rejected),
        Err(ParseError::Unsupported(_))
    ));
    let existential = bgp(
        TermPattern::Triple(Child::new(TriplePattern {
            subject: raw("existential"),
            predicate: NamedNodePattern::NamedNode(patterns::example("predicate")),
            object: TermPattern::Variable(Variable::new("inside")),
        })),
        TermPattern::Variable(Variable::new("outside")),
    );
    parse(
        &try_pattern_to_select_query(&existential).expect("quoted match blanks remain existential"),
    );
}

#[test]
fn independent_union_owners_receive_distinct_labels_deterministically() {
    let pattern = GraphPattern::Join {
        left: Child::new(GraphPattern::Union {
            arms: vec![
                bgp(TermPattern::Variable(Variable::new("s")), raw("spelling")),
                bgp(TermPattern::Variable(Variable::new("s")), raw("spelling")),
            ]
            .try_into()
            .expect("two UNION arms"),
        }),
        right: Child::new(bgp(
            raw("spelling"),
            TermPattern::Variable(Variable::new("o")),
        )),
    };
    let carrier = try_pattern_to_select_query(&pattern).expect("independent blanks have a carrier");
    let cloned_pattern = pattern.clone();
    assert_eq!(
        carrier,
        try_pattern_to_select_query(&cloned_pattern).expect("allocation order is irrelevant")
    );
    assert_eq!(
        carrier,
        try_pattern_to_select_query(&pattern).expect("source remains unchanged")
    );
    let rebuilt = parse(&carrier);
    let labels = blank_labels(&rebuilt);
    assert_eq!(labels.len(), 3);
    let distinct: std::collections::BTreeSet<_> = labels.iter().collect();
    assert_eq!(
        distinct.len(),
        3,
        "UNION arms and following BGP have independent owners"
    );
    assert!(!carrier.contains("_:spelling"));
}

#[test]
fn positive_join_leaves_keep_one_blank_and_parsed_text_keeps_existing_bytes() {
    let pattern = GraphPattern::Join {
        left: Child::new(bgp(
            TermPattern::Variable(Variable::new("s")),
            raw("shared"),
        )),
        right: Child::new(bgp(
            raw("shared"),
            TermPattern::Variable(Variable::new("o")),
        )),
    };
    let carrier =
        try_pattern_to_select_query(&pattern).expect("positive leaves share one blank owner");
    assert_eq!(carrier.matches("_:shared").count(), 2);
    assert!(carrier.starts_with("SELECT * WHERE"));
    let rebuilt = parse(&carrier);
    let labels = blank_labels(&rebuilt);
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0], labels[1]);
    let source =
        parse("SELECT * WHERE { ?s <http://example.org/p> _:b . _:b <http://example.org/p> ?o }");
    let body = patterns::where_body(&source);
    let first = try_pattern_to_select_query(&body).expect("parsed body renders");
    assert_eq!(
        first,
        try_pattern_to_select_query(&patterns::where_body(&parse(&first)))
            .expect("stable parsed bytes")
    );
}

#[test]
fn a_shared_owner_crossing_required_braces_uses_unprojected_collision_free_names() {
    let pattern = GraphPattern::Join {
        left: Child::new(bgp(
            TermPattern::Variable(Variable::new("__purrdf_hidden_0")),
            raw("shared"),
        )),
        right: Child::new(GraphPattern::PropertyFunction(PropertyFunctionCall {
            iri: patterns::example("call").as_str().to_owned(),
            subject_args: vec![raw("shared")],
            object_args: vec![TermPattern::Variable(Variable::new("o"))],
        })),
    };
    let options = ParserOptions {
        property_fn_iris: vec![patterns::example("call").as_str().to_owned()],
        ..ParserOptions::default()
    };
    let carrier = try_pattern_to_select_query_with_options(&pattern, &options)
        .expect("shared positive owner has a carrier");
    assert!(carrier.starts_with("SELECT ?__purrdf_hidden_0 ?o WHERE"));
    assert_eq!(carrier.matches("?___purrdf_hidden_0").count(), 2);
    assert!(!carrier.contains("_:shared"));
    let Query::Select {
        pattern: GraphPattern::Project { variables, .. },
        ..
    } = SparqlParser::new()
        .parse_query_with(&carrier, &options)
        .expect("call carrier reparses")
    else {
        panic!("explicit visible projection");
    };
    assert_eq!(
        variables,
        [Variable::new("__purrdf_hidden_0"), Variable::new("o")]
    );
}

#[test]
fn canonical_source_endpoints_reconcile_raw_siblings_through_generated_union() {
    let hidden = Variable::hidden_blank("shared");
    let pattern = GraphPattern::Join {
        left: Child::new(GraphPattern::Union {
            arms: vec![
                bgp(
                    TermPattern::Variable(Variable::new("s")),
                    TermPattern::Variable(hidden.clone()),
                ),
                bgp(
                    TermPattern::Variable(Variable::new("s")),
                    TermPattern::Variable(hidden),
                ),
            ]
            .try_into()
            .expect("two UNION arms"),
        }),
        right: Child::new(bgp(
            raw("shared"),
            TermPattern::Variable(Variable::new("o")),
        )),
    };
    let carrier =
        try_pattern_to_select_query(&pattern).expect("source marked identity remains shared");
    assert_eq!(carrier.matches("?__purrdf_hidden_0").count(), 3);
    assert!(!carrier.contains("_:shared"));
    let rebuilt = parse(&carrier);
    let GraphPattern::Project { variables, .. } = rebuilt else {
        panic!("visible projection");
    };
    assert_eq!(variables, [Variable::new("s"), Variable::new("o")]);
}

#[test]
fn authored_blank_alias_lookalikes_and_quoted_endpoints_cannot_capture() {
    let quoted = || {
        TermPattern::Triple(Child::new(TriplePattern {
            subject: raw("__purrdf_scoped_blank_0"),
            predicate: NamedNodePattern::NamedNode(patterns::example("quoted")),
            object: TermPattern::Variable(Variable::new("inside")),
        }))
    };
    let pattern = GraphPattern::Union {
        arms: vec![
            bgp(quoted(), TermPattern::Variable(Variable::new("outside"))),
            bgp(quoted(), TermPattern::Variable(Variable::new("outside"))),
        ]
        .try_into()
        .expect("two UNION arms"),
    };
    let carrier = try_pattern_to_select_query(&pattern).expect("quoted independent owners render");
    assert!(carrier.contains("_:___purrdf_scoped_blank_0"));
    assert!(carrier.contains("_:___purrdf_scoped_blank_1"));
    let rebuilt = parse(&carrier);
    let labels = blank_labels(&rebuilt);
    assert_eq!(labels.len(), 2);
    assert_ne!(labels[0], labels[1]);
}

#[test]
fn zero_column_and_deep_positive_carriers_preserve_witness_policy() {
    let pattern = GraphPattern::Join {
        left: Child::new(bgp(
            raw("shared"),
            TermPattern::NamedNode(patterns::example("object")),
        )),
        right: Child::new(GraphPattern::PropertyFunction(PropertyFunctionCall {
            iri: patterns::example("call").as_str().to_owned(),
            subject_args: vec![raw("shared")],
            object_args: vec![],
        })),
    };
    let options = ParserOptions {
        property_fn_iris: vec![patterns::example("call").as_str().to_owned()],
        ..ParserOptions::default()
    };
    let carrier = try_pattern_to_select_query_with_options(&pattern, &options)
        .expect("zero-column witness carrier");
    assert!(carrier.starts_with("SELECT (1 AS ?__purrdf_hidden_0unit) WHERE"));
    SparqlParser::new()
        .parse_query_with(&carrier, &options)
        .expect("unit carrier reparses");

    purrdf_stack::on_stack(128 * 1024, || {
        let mut pattern = bgp(TermPattern::Variable(Variable::new("s")), raw("shared"));
        for _ in 0..10_000 {
            pattern = GraphPattern::Join {
                left: Child::new(pattern),
                right: Child::new(bgp(
                    raw("shared"),
                    TermPattern::Variable(Variable::new("o")),
                )),
            };
        }
        let carrier =
            try_pattern_to_select_query(&pattern).expect("deep ownership scan is iterative");
        assert_eq!(carrier.matches("_:shared").count(), 10_001);
        parse(&carrier);
    })
    .expect("bounded stack thread completes");
}

#[test]
fn opaque_raw_labels_receive_injective_legal_aliases_and_keep_repeated_identity() {
    let labels = [
        "two words",
        "",
        "end.",
        "-start",
        "\0bgp:witness",
        "a:b",
        "__purrdf_scoped_blank_0",
        "café",
    ];
    let pattern = GraphPattern::Bgp {
        patterns: labels
            .iter()
            .flat_map(|label| {
                let triple = TriplePattern {
                    subject: raw(label),
                    predicate: NamedNodePattern::NamedNode(patterns::example("p")),
                    object: TermPattern::Variable(Variable::new("visible")),
                };
                [triple.clone(), triple]
            })
            .collect(),
    };
    let carrier = try_pattern_to_select_query(&pattern)
        .expect("opaque existential identities have legal aliases");
    assert!(!carrier.contains('\0'));
    assert_eq!(carrier.matches("_:__purrdf_scoped_blank_0").count(), 2);
    assert_eq!(carrier.matches("_:café").count(), 2);
    let cloned_pattern = pattern.clone();
    assert_eq!(
        carrier,
        try_pattern_to_select_query(&cloned_pattern).expect("fresh allocation preserves names")
    );
    assert_eq!(
        blank_labels(&pattern),
        labels
            .into_iter()
            .flat_map(|label| [label.to_owned(), label.to_owned()])
            .collect::<Vec<_>>()
    );
    let rebuilt = parse(&carrier);
    let rebuilt_labels = blank_labels(&rebuilt);
    assert_eq!(rebuilt_labels.len(), labels.len() * 2);
    let (pairs, remainder) = rebuilt_labels.as_chunks::<2>();
    assert!(
        remainder.is_empty(),
        "every source identity has two occurrences"
    );
    for pair in pairs {
        assert_eq!(
            pair[0], pair[1],
            "one source existential keeps one identity"
        );
    }
    let distinct: std::collections::BTreeSet<_> = rebuilt_labels.iter().collect();
    assert_eq!(
        distinct.len(),
        labels.len(),
        "different opaque source identities cannot capture"
    );
    assert!(
        rebuilt_labels
            .iter()
            .all(|label| purrdf_lex::terminals::is_valid_blank_node_label(label))
    );
}
