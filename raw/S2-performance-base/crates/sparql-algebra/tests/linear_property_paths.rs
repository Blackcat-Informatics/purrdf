// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Linear property paths are ordinary conjunctive triple patterns, with hidden
//! join points, while the other path operators and template grammar stay distinct.

#[path = "support/patterns.rs"]
mod patterns;

use std::collections::BTreeSet;

use patterns::where_body;
use purrdf_sparql_algebra::{
    GraphPattern, ParserOptions, Query, SparqlParser, TermPattern, pattern_to_select_query,
    pattern_to_select_query_with_options,
};

const PREFIX: &str = "PREFIX ex: <http://example.org/> ";

/// A complete SELECT's body, under the supplied relation recognition rules.
fn body(text: &str, options: &ParserOptions) -> GraphPattern {
    let Query::Select { pattern, .. } = SparqlParser::new()
        .parse_query_with(&format!("{PREFIX}{text}"), options)
        .expect("the query parses")
    else {
        panic!("a SELECT query");
    };
    where_body(&pattern)
}

/// The in-scope variables and the body of a parsed SELECT.
fn projected(text: &str) -> (Vec<String>, GraphPattern) {
    let Query::Select { pattern, .. } = SparqlParser::new()
        .parse_query(&format!("{PREFIX}{text}"))
        .expect("the query parses")
    else {
        panic!("a SELECT query");
    };
    let GraphPattern::Project { variables, inner } = pattern else {
        panic!("a SELECT has a projection");
    };
    (
        variables.iter().map(|v| v.as_str().to_owned()).collect(),
        inner.into_inner(),
    )
}

#[test]
fn linear_paths_and_their_expansions_are_the_same_bgp() {
    let b0 = "_:__purrdf_anon_0";
    let b1 = "_:__purrdf_anon_1";
    let b2 = "_:__purrdf_anon_2";
    let b3 = "_:__purrdf_anon_3";
    for (path, triples) in [
        ("ex:p/ex:q", format!("?s ex:p {b0} . {b0} ex:q ?o")),
        ("^ex:p", "?o ex:p ?s".to_owned()),
        ("^(^ex:p)", "?s ex:p ?o".to_owned()),
        ("^(ex:p/ex:q)", format!("{b0} ex:q ?s . ?o ex:p {b0}")),
        ("^(^(ex:p/ex:q))", format!("?s ex:p {b0} . {b0} ex:q ?o")),
        (
            "ex:p/(ex:q/^ex:r)/^(ex:t/^ex:u)",
            format!(
                "?s ex:p {b0} . {b0} ex:q {b1} . {b2} ex:r {b1} . \
                 {b2} ex:u {b3} . ?o ex:t {b3}"
            ),
        ),
        ("a/ex:p", format!("?s a {b0} . {b0} ex:p ?o")),
    ] {
        let query = format!("SELECT * WHERE {{ ?s {path} ?o . ?s a ?ts . ?o a ?to }}");
        let expanded = format!("SELECT * WHERE {{ {triples} . ?s a ?ts . ?o a ?to }}");
        let actual = body(&query, &ParserOptions::default());
        assert!(
            matches!(&actual, GraphPattern::Bgp { .. }),
            "{path}: {actual:?}"
        );
        assert_eq!(actual, body(&expanded, &ParserOptions::default()), "{path}");
    }
}

#[test]
fn every_object_and_path_has_its_own_collision_safe_hidden_join() {
    let (variables, pattern) = projected(
        "SELECT * WHERE { ?s ex:p/ex:q ?o, ?z ; ex:r/ex:t ?end . \
         [] ex:other _:__purrdf_anon_0 }",
    );
    assert_eq!(variables, ["s", "o", "z", "end"]);
    let GraphPattern::Bgp { patterns } = pattern else {
        panic!("all the triples share one BGP");
    };
    assert_eq!(patterns.len(), 7);
    let joins: BTreeSet<_> = [0, 2, 4]
        .map(|i| {
            let TermPattern::BlankNode(blank) = &patterns[i].object else {
                panic!("each sequence has a hidden intermediate");
            };
            assert_eq!(patterns[i].object, patterns[i + 1].subject);
            assert_ne!(patterns[i].object, patterns[6].subject);
            assert_ne!(patterns[i].object, patterns[6].object);
            blank.as_str()
        })
        .into_iter()
        .collect();
    assert_eq!(joins.len(), 3);
    assert!(
        joins
            .iter()
            .all(|label| label.starts_with("___purrdf_anon_"))
    );
}

#[test]
fn inverse_projection_is_the_normalized_triples_visible_order() {
    let (variables, pattern) = projected("SELECT * WHERE { ?s ^ex:p ?o }");
    assert_eq!(variables, ["o", "s"]);
    assert_eq!(
        pattern,
        body("SELECT * WHERE { ?o ex:p ?s }", &ParserOptions::default())
    );
}

#[test]
fn a_non_linear_element_preserves_the_whole_path_without_minting_join_points() {
    for path in [
        "ex:p/ex:q*",
        "ex:p/ex:q+",
        "ex:p/ex:q?",
        "ex:p/ex:q{1,1}",
        "ex:p/(ex:q|ex:r*)",
        "ex:p/(ex:q|ex:r?)",
        "ex:p/!(ex:q|^ex:r)",
        "^(ex:p/ex:q+)",
    ] {
        let original = body(
            &format!("SELECT * WHERE {{ ?s {path} ?o }}"),
            &ParserOptions::default(),
        );
        assert!(matches!(&original, GraphPattern::Path { .. }), "{path}");
        let pattern = body(
            &format!("SELECT * WHERE {{ ?s {path} ?o . ?x ex:r/ex:t ?y }}"),
            &ParserOptions::default(),
        );
        let GraphPattern::Join { left, right } = pattern else {
            panic!("the complete non-linear path is joined onto the data BGP");
        };
        assert_eq!(*right, original, "{path}");
        assert_eq!(
            *left,
            body(
                "SELECT * WHERE { ?x ex:r _:__purrdf_anon_0 . _:__purrdf_anon_0 ex:t ?y }",
                &ParserOptions::default(),
            ),
            "{path}: rejection emitted no partial triples or fresh blanks"
        );
    }
}

#[test]
fn linear_paths_remain_forbidden_in_every_template_form() {
    for text in [
        "CONSTRUCT { ?s ex:p/ex:q ?o } WHERE { }",
        "CONSTRUCT WHERE { ?s ^ex:p ?o }",
        "CONSTRUCT { GRAPH ex:g { ?s ex:p/ex:q ?o } } WHERE { }",
        "CONSTRUCT { ?s ex:r [ ex:p/ex:q ?o ] } WHERE { }",
    ] {
        let error = SparqlParser::new()
            .parse_query(&format!("{PREFIX}{text}"))
            .expect_err("a template cannot assert a path");
        assert!(
            error.to_string().contains("property paths"),
            "{text}: {error}"
        );
    }
    for text in [
        "INSERT DATA { ex:s ex:p/ex:q ex:o }",
        "DELETE DATA { ex:s ^ex:p ex:o }",
        "INSERT { ?s ex:p/ex:q ?o } WHERE { ?s ex:r ?o }",
        "DELETE { ?s ^ex:p ?o } WHERE { ?s ex:r ?o }",
        "DELETE WHERE { ?s ex:p/ex:q ?o }",
        "INSERT DATA { GRAPH ex:g { ex:s ex:p/ex:q ex:o } }",
        "DELETE WHERE { GRAPH ex:g { ?s ^ex:p ?o } }",
        "INSERT DATA { ex:s ex:r [ ex:p/ex:q ex:o ] }",
    ] {
        let error = SparqlParser::new()
            .parse_update(&format!("{PREFIX}{text}"))
            .expect_err("a template cannot assert a path");
        assert!(
            error.to_string().contains("property paths"),
            "{text}: {error}"
        );
    }
}

#[test]
fn linear_path_lowering_does_not_enable_annotation_or_triple_term_path_syntax() {
    for text in [
        "SELECT * WHERE { ?s ex:p/ex:q ?o {| ex:a ex:v |} }",
        "SELECT * WHERE { ?s ^ex:p ?o ~ ex:r }",
        "SELECT * WHERE { <<( ?s ex:p/ex:q ?o )>> ex:a ex:v }",
    ] {
        assert!(
            SparqlParser::new()
                .parse_query(&format!("{PREFIX}{text}"))
                .is_err(),
            "{text}"
        );
    }
}

#[test]
fn both_serializers_keep_data_predicates_distinct_under_namespace_and_exact_registries() {
    for options in [
        ParserOptions::default(),
        ParserOptions {
            property_fn_namespaces: vec!["http://example.org/".to_owned()],
            ..ParserOptions::default()
        },
        ParserOptions {
            property_fn_iris: vec!["http://example.org/p".to_owned()],
            ..ParserOptions::default()
        },
    ] {
        let pattern = body("SELECT * WHERE { ?s ex:p/ex:q ?o . ?x ^ex:p ?y }", &options);
        let independent = pattern_to_select_query(&pattern);
        let configured = pattern_to_select_query_with_options(&pattern, &options);
        assert_eq!(independent.matches("^(^").count(), 3);
        let expected = if options.property_fn_namespaces.is_empty() {
            if options.property_fn_iris.is_empty() {
                0
            } else {
                2
            }
        } else {
            3
        };
        assert_eq!(configured.matches("^(^").count(), expected);
        for text in [independent, configured] {
            let reparsed = body(&text, &options);
            assert_eq!(reparsed, pattern, "{text}");
            assert_eq!(
                pattern_to_select_query(&reparsed),
                pattern_to_select_query(&pattern)
            );
        }
    }
}

#[test]
fn serializer_modes_have_deterministic_predicate_spellings() {
    let pattern = body("SELECT * WHERE { ?s ^ex:p ?o }", &ParserOptions::default());
    let protected = "SELECT * WHERE { ?o ^(^<http://example.org/p>) ?s . }";
    assert_eq!(pattern_to_select_query(&pattern), protected);
    assert_eq!(
        pattern_to_select_query_with_options(&pattern, &ParserOptions::default()),
        "SELECT * WHERE { ?o <http://example.org/p> ?s . }"
    );
    let options = ParserOptions {
        property_fn_iris: vec!["http://example.org/p".to_owned()],
        ..ParserOptions::default()
    };
    assert_eq!(
        pattern_to_select_query_with_options(&pattern, &options),
        protected
    );
}

#[test]
fn compact_serialization_uses_exact_registry_identity_and_keeps_calls_and_quoted_terms() {
    let options = ParserOptions {
        property_fn_iris: vec!["http://example.org/p".to_owned()],
        ..ParserOptions::default()
    };
    let pattern = body(
        "SELECT * WHERE { ?s ^ex:p ?o . ?a ^ex:pa ?b . \
         ?s ex:q <<( ?x ex:p ?y )>> . ?o ex:p ?z }",
        &options,
    );
    let compact = pattern_to_select_query_with_options(&pattern, &options);
    assert_eq!(compact.matches("^(^").count(), 1);
    assert!(compact.contains("?b <http://example.org/pa> ?a ."));
    assert!(compact.contains("<<( ?x <http://example.org/p> ?y )>>"));
    assert!(compact.contains("?o <http://example.org/p> ?z ."));
    for text in [compact, pattern_to_select_query(&pattern)] {
        assert_eq!(body(&text, &options), pattern, "{text}");
    }
}

#[test]
fn both_serializer_modes_protect_registry_collisions_inside_modifiers_and_exists() {
    for options in [
        ParserOptions {
            property_fn_namespaces: vec!["http://example.org/".to_owned()],
            ..ParserOptions::default()
        },
        ParserOptions {
            property_fn_iris: vec!["http://example.org/p".to_owned()],
            ..ParserOptions::default()
        },
    ] {
        for query in [
            "SELECT ?o WHERE { ?s ex:p/ex:q ?o FILTER EXISTS { ?o ^ex:p ?z } } \
             ORDER BY ?o LIMIT 2",
            "SELECT * WHERE { { SELECT ?o WHERE { ?s ex:p/ex:q ?o } ORDER BY ?o LIMIT 2 } }",
            "SELECT * WHERE { ?s ex:p/ex:q ?o OPTIONAL { ?o ^ex:p ?z } \
             FILTER NOT EXISTS { ?o ex:p/ex:q ?end } }",
        ] {
            let pattern = body(query, &options);
            for text in [
                pattern_to_select_query(&pattern),
                pattern_to_select_query_with_options(&pattern, &options),
            ] {
                assert_eq!(body(&text, &options), pattern, "{query}: {text}");
            }
        }
    }
}

#[test]
fn update_display_protects_where_predicates_and_keeps_template_predicates_plain() {
    let update = SparqlParser::new()
        .parse_update(&format!(
            "{PREFIX}INSERT {{ ?s ex:p ?o }} WHERE {{ ?s ^ex:p ?o }}"
        ))
        .expect("an update WHERE admits an inverse path");
    let text = update.to_string();
    assert!(
        text.contains("INSERT { ?s <http://example.org/p> ?o . }"),
        "{text}"
    );
    assert!(
        text.contains("?o ^(^<http://example.org/p>) ?s ."),
        "{text}"
    );
    assert_eq!(
        SparqlParser::new().parse_update(&text).expect("re-parse"),
        update
    );
}
