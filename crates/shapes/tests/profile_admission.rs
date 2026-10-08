// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native, dated Appendix A admission expectations and admissible neighbors.

use purrdf_shapes::{AdmissionReason, QueryPurpose, ShaclProfile};
use purrdf_sparql_algebra::{Query, SparqlParser};

const DATED: [ShaclProfile; 2] = [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918];

fn parse(text: &str) -> Query {
    SparqlParser::new()
        .parse_query(text)
        .expect("fixture is legal SPARQL")
}

fn check(profile: ShaclProfile, text: &str) -> Result<(), purrdf_shapes::AdmissionRefusal> {
    profile.admit_query(QueryPurpose::SelectConstraint, &parse(text), &[])
}

fn role_query(purpose: QueryPurpose, body: &str) -> Query {
    let text = match purpose {
        QueryPurpose::AskValidator | QueryPurpose::AskTarget => format!("ASK {{ {body} }}"),
        QueryPurpose::ConstructRule | QueryPurpose::GlobalConstructRule => {
            format!("CONSTRUCT {{ $this <http://example.org/p> ?v }} WHERE {{ {body} }}")
        }
        _ => format!("SELECT $this WHERE {{ {body} }}"),
    };
    parse(&text)
}

#[test]
fn identities_are_exact_and_name_each_required_edition() {
    assert_eq!(ShaclProfile::default(), ShaclProfile::LEGACY);
    for profile in [ShaclProfile::LEGACY, DATED[0], DATED[1]] {
        assert_eq!(ShaclProfile::from_id(profile.id()).unwrap(), profile);
        assert_eq!(profile.to_string(), profile.id());
    }
    for id in [
        "",
        "LEGACY",
        " legacy",
        "shacl-rec-2017-07-21",
        "shacl-wd-2026-09-17",
    ] {
        let error = ShaclProfile::from_id(id).expect_err("no date or spelling fallback");
        assert_eq!(error.id(), id);
    }
    assert_eq!(ShaclProfile::LEGACY.core_specification(), None);
    assert_eq!(ShaclProfile::LEGACY.sparql_specification(), None);
    assert_eq!(ShaclProfile::LEGACY.xpath_specification(), None);
    assert_eq!(
        DATED[0].core_specification(),
        Some("https://www.w3.org/TR/2017/REC-shacl-20170720/")
    );
    assert_eq!(
        DATED[0].core_specification(),
        DATED[0].sparql_specification()
    );
    assert_eq!(
        DATED[0].xpath_specification(),
        Some("https://www.w3.org/TR/2010/REC-xpath-functions-20101214/")
    );
    assert_eq!(
        DATED[1].core_specification(),
        Some("https://www.w3.org/TR/2026/WD-shacl12-core-20260917/")
    );
    assert_eq!(
        DATED[1].sparql_specification(),
        Some("https://www.w3.org/TR/2026/WD-shacl12-sparql-20260918/")
    );
    assert_eq!(
        DATED[1].xpath_specification(),
        Some("https://www.w3.org/TR/2017/REC-xpath-functions-31-20170321/")
    );
}

#[test]
fn local_values_differs_by_date_and_prebound_values_are_refused() {
    let local = "SELECT $this WHERE { $this ?p ?v VALUES ?local { 1 2 } }";
    for profile in [ShaclProfile::LEGACY, DATED[0]] {
        let refusal = check(profile, local).unwrap_err();
        assert_eq!(refusal.reason(), AdmissionReason::Values);
        assert_eq!(refusal.variable(), None);
        assert_eq!(refusal.profile(), profile);
        assert_eq!(refusal.purpose(), QueryPurpose::SelectConstraint);
    }
    check(DATED[1], local).expect("draft local VALUES is an admitted neighbor");
    for profile in DATED {
        let refusal = check(
            profile,
            "SELECT $this WHERE { VALUES $this { <http://example.org/a> } }",
        )
        .unwrap_err();
        assert_eq!(refusal.reason(), AdmissionReason::Values);
        assert_eq!(refusal.variable(), (profile == DATED[1]).then_some("this"));
        check(
            profile,
            "SELECT $this WHERE { $this ?p ?local FILTER(?local IN (1, 2)) }",
        )
        .expect("a FILTER over a local variable is admissible");
    }
}

#[test]
fn minus_has_optional_and_not_exists_neighbors() {
    for profile in DATED {
        assert_eq!(
            check(
                profile,
                "SELECT $this WHERE { $this ?p ?v MINUS { $this ?q ?v } }"
            )
            .unwrap_err()
            .reason(),
            AdmissionReason::Minus
        );
        for neighbor in [
            "SELECT $this WHERE { $this ?p ?v OPTIONAL { $this ?q ?other } }",
            "SELECT $this WHERE { $this ?p ?v FILTER NOT EXISTS { $this ?q ?v } }",
        ] {
            check(profile, neighbor).expect("neighbor is admissible");
        }
    }
}

#[test]
fn every_role_refuses_service_and_admits_a_local_graph() {
    let roles = [
        QueryPurpose::SelectConstraint,
        QueryPurpose::SelectValidator,
        QueryPurpose::AskValidator,
        QueryPurpose::ConstructRule,
        QueryPurpose::GlobalConstructRule,
        QueryPurpose::Function,
        QueryPurpose::TargetType,
        QueryPurpose::SelectTarget,
        QueryPurpose::AskTarget,
        QueryPurpose::SelectExpression,
        QueryPurpose::ScalarExpression,
    ];
    for profile in [ShaclProfile::LEGACY, DATED[0], DATED[1]] {
        for purpose in roles {
            let refused = role_query(
                purpose,
                "SERVICE <http://example.org/endpoint> { $this ?p ?v }",
            );
            let refusal = profile.admit_query(purpose, &refused, &[]).unwrap_err();
            assert_eq!(
                refusal.reason(),
                AdmissionReason::Service,
                "{profile} {purpose:?}"
            );
            let admitted = role_query(purpose, "GRAPH <http://example.org/local> { $this ?p ?v }");
            profile
                .admit_query(purpose, &admitted, &[])
                .expect("local GRAPH is admissible");
        }
    }
}

#[test]
fn assignments_use_actual_role_bindings() {
    for profile in DATED {
        for (purpose, variable, parameters) in [
            (QueryPurpose::SelectConstraint, "this", &[][..]),
            (QueryPurpose::SelectValidator, "argument", &["argument"][..]),
            (QueryPurpose::AskValidator, "value", &[][..]),
            (QueryPurpose::AskTarget, "this", &[][..]),
            (QueryPurpose::Function, "argument", &["argument"][..]),
            (QueryPurpose::TargetType, "argument", &["argument"][..]),
            (QueryPurpose::ConstructRule, "this", &[][..]),
            (QueryPurpose::SelectExpression, "this", &[][..]),
            (QueryPurpose::ScalarExpression, "this", &[][..]),
        ] {
            let refused = role_query(purpose, &format!("BIND(true AS ?{variable})"));
            let refusal = profile
                .admit_query(purpose, &refused, parameters)
                .unwrap_err();
            assert_eq!(refusal.reason(), AdmissionReason::Assignment);
            assert_eq!(refusal.variable(), Some(variable));
            let admitted = role_query(purpose, &format!("BIND(?{variable} AS ?local)"));
            profile
                .admit_query(purpose, &admitted, parameters)
                .expect("using a pre-binding is admissible");
        }
        for purpose in [
            QueryPurpose::Function,
            QueryPurpose::TargetType,
            QueryPurpose::SelectTarget,
        ] {
            let query = role_query(purpose, "BIND(true AS ?this)");
            profile
                .admit_query(purpose, &query, &[])
                .expect("this is produced, not pre-bound, for this role");
        }
    }
}

#[test]
fn optional_rec_context_is_restricted_for_assignment_but_not_projection() {
    for variable in ["shapesGraph", "currentShape"] {
        let query = format!("SELECT $this WHERE {{ BIND(true AS ?{variable}) }}");
        let refusal = check(DATED[0], &query).unwrap_err();
        assert_eq!(refusal.reason(), AdmissionReason::Assignment);
        assert_eq!(refusal.variable(), Some(variable));
        check(DATED[1], &query).expect("draft removed this potential shape-context binding");
    }
    let query = "SELECT $this WHERE { { SELECT $this WHERE { $this ?p ?v } } }";
    for profile in DATED {
        check(profile, query).expect("optional shape context need not be projected");
    }
}

#[test]
fn rec_subqueries_project_role_bindings_and_draft_allows_hidden_bindings() {
    for query in [
        "SELECT $this WHERE { { SELECT ?v WHERE { $this ?p ?v } } }",
        "SELECT $this WHERE { { SELECT * WHERE { FILTER(BOUND($this)) } } }",
    ] {
        let refusal = check(DATED[0], query).unwrap_err();
        assert_eq!(refusal.reason(), AdmissionReason::SubqueryProjection);
        assert_eq!(refusal.variable(), Some("this"));
        check(DATED[1], query).expect("draft permits this hidden subquery binding");
    }
    for query in [
        "SELECT $this WHERE { { SELECT $this WHERE { $this ?p ?v } } }",
        "SELECT $this WHERE { { SELECT * WHERE { $this ?p ?v } } }",
        "SELECT ?v WHERE { $this ?p ?v }",
    ] {
        for profile in DATED {
            check(profile, query).expect("projected or outer-projection neighbor is admissible");
        }
    }
    let hidden = parse("ASK { { SELECT $this WHERE { $this ?p ?value } } }");
    let refusal = DATED[0]
        .admit_query(QueryPurpose::AskValidator, &hidden, &[])
        .unwrap_err();
    assert_eq!(refusal.variable(), Some("value"));
    DATED[1]
        .admit_query(QueryPurpose::AskValidator, &hidden, &[])
        .unwrap();
    let projected = parse("ASK { { SELECT $this ?value WHERE { $this ?p ?value } } }");
    DATED[0]
        .admit_query(QueryPurpose::AskValidator, &projected, &[])
        .unwrap();
}

#[test]
fn function_and_target_parameters_have_their_own_projection_and_values_policy() {
    for purpose in [QueryPurpose::Function, QueryPurpose::TargetType] {
        let hidden = parse("SELECT $this WHERE { { SELECT $this WHERE { $this ?p ?argument } } }");
        let refusal = DATED[0]
            .admit_query(purpose, &hidden, &["argument"])
            .unwrap_err();
        assert_eq!(refusal.reason(), AdmissionReason::SubqueryProjection);
        assert_eq!(refusal.variable(), Some("argument"));
        DATED[1]
            .admit_query(purpose, &hidden, &["argument"])
            .unwrap();
        let projected =
            parse("SELECT $this WHERE { { SELECT ?argument WHERE { $this ?p ?argument } } }");
        DATED[0]
            .admit_query(purpose, &projected, &["argument"])
            .unwrap();

        let values = parse("SELECT $this WHERE { VALUES ?argument { 1 } }");
        for profile in DATED {
            assert_eq!(
                profile
                    .admit_query(purpose, &values, &["argument"])
                    .unwrap_err()
                    .reason(),
                AdmissionReason::Values
            );
            let local = parse("SELECT $this WHERE { $this ?p ?argument FILTER(?argument = 1) }");
            profile.admit_query(purpose, &local, &["argument"]).unwrap();
        }
    }
}

#[test]
fn a_parameter_named_like_optional_context_remains_required() {
    for purpose in [
        QueryPurpose::SelectValidator,
        QueryPurpose::Function,
        QueryPurpose::TargetType,
    ] {
        for name in ["shapesGraph", "currentShape"] {
            let hidden = parse("SELECT $this WHERE { { SELECT $this WHERE { $this ?p ?v } } }");
            let refusal = DATED[0].admit_query(purpose, &hidden, &[name]).unwrap_err();
            assert_eq!(refusal.reason(), AdmissionReason::SubqueryProjection);
            assert_eq!(refusal.variable(), Some(name));
            let projected = parse(&format!(
                "SELECT $this WHERE {{ {{ SELECT $this ?{name} WHERE {{ $this ?p ?{name} }} }} }}"
            ));
            DATED[0].admit_query(purpose, &projected, &[name]).unwrap();
            DATED[1].admit_query(purpose, &hidden, &[name]).unwrap();
        }
    }
}

#[test]
fn genuinely_unbound_bodies_follow_each_dated_scope_and_keep_legacy_behavior() {
    for purpose in [
        QueryPurpose::Function,
        QueryPurpose::TargetType,
        QueryPurpose::SelectTarget,
    ] {
        for (text, reason) in [
            (
                "SELECT $this WHERE { VALUES $this { <http://example.org/a> } }",
                AdmissionReason::Values,
            ),
            (
                "SELECT $this WHERE { $this ?p ?v MINUS { $this ?q ?v } }",
                AdmissionReason::Minus,
            ),
        ] {
            let query = parse(text);
            assert_eq!(
                DATED[0]
                    .admit_query(purpose, &query, &[])
                    .unwrap_err()
                    .reason(),
                reason
            );
            DATED[1].admit_query(purpose, &query, &[]).unwrap();
            ShaclProfile::LEGACY
                .admit_query(purpose, &query, &[])
                .unwrap();
        }
        for profile in DATED {
            profile
                .admit_query(purpose, &parse("SELECT $this WHERE { $this ?p ?v }"), &[])
                .unwrap();
        }
    }
}

#[test]
fn wrong_query_form_has_a_typed_refusal_and_correct_form_neighbor() {
    for profile in DATED {
        for purpose in [
            QueryPurpose::SelectConstraint,
            QueryPurpose::AskValidator,
            QueryPurpose::ConstructRule,
            QueryPurpose::Function,
        ] {
            let refusal = profile
                .admit_query(purpose, &parse("DESCRIBE $this WHERE { $this ?p ?v }"), &[])
                .unwrap_err();
            assert_eq!(refusal.reason(), AdmissionReason::QueryForm);
            assert_eq!(refusal.variable(), None);
            profile
                .admit_query(purpose, &role_query(purpose, "$this ?p ?v"), &[])
                .unwrap();
        }
    }
}

#[test]
fn aggregate_operands_cannot_hide_restrictions_under_either_dated_law() {
    for profile in DATED {
        for (body, reason) in [
            (
                "SERVICE <http://example.org/endpoint> { $this ?p ?v }",
                AdmissionReason::Service,
            ),
            ("$this ?p ?v MINUS { $this ?q ?v }", AdmissionReason::Minus),
        ] {
            let text = format!(
                "SELECT $this (COUNT(IF(EXISTS {{ {body} }}, 1, 0)) AS ?count) WHERE {{ $this ?p ?v }} GROUP BY $this"
            );
            assert_eq!(check(profile, &text).unwrap_err().reason(), reason);
        }
        check(profile, "SELECT $this (COUNT(IF(EXISTS { GRAPH <http://example.org/local> { $this ?p ?v } }, 1, 0)) AS ?count) WHERE { $this ?p ?v } GROUP BY $this").unwrap();
        let values = parse(
            "SELECT $this (COUNT(IF(EXISTS { VALUES ?argument { 1 } }, 1, 0)) AS ?count) WHERE { $this ?p ?v } GROUP BY $this",
        );
        assert_eq!(
            profile
                .admit_query(QueryPurpose::SelectValidator, &values, &["argument"])
                .unwrap_err()
                .reason(),
            AdmissionReason::Values
        );
        let neighbor = parse(
            "SELECT $this (COUNT(IF(EXISTS { ?argument ?p ?other FILTER(?argument = 1) }, 1, 0)) AS ?count) WHERE { $this ?p ?v } GROUP BY $this",
        );
        profile
            .admit_query(QueryPurpose::SelectValidator, &neighbor, &["argument"])
            .unwrap();
    }
}

#[test]
fn aggregate_order_expressions_are_audited() {
    for profile in DATED {
        let refused = "SELECT $this (FOLD(?v ORDER BY EXISTS { SERVICE <http://example.org/endpoint> { $this ?p ?other } }) AS ?items) WHERE { $this ?p ?v } GROUP BY $this";
        assert_eq!(
            check(profile, refused).unwrap_err().reason(),
            AdmissionReason::Service
        );
        let admitted = "SELECT $this (FOLD(?v ORDER BY EXISTS { GRAPH <http://example.org/local> { $this ?p ?other } }) AS ?items) WHERE { $this ?p ?v } GROUP BY $this";
        check(profile, admitted).unwrap();
    }
}

#[test]
fn projection_and_unfold_aliases_cannot_assign_a_potential_binding() {
    for profile in DATED {
        for query in [
            "SELECT (true AS $this) WHERE { ?s ?p ?v }",
            "SELECT $this WHERE { ?s ?p ?list UNFOLD(?list AS $this) }",
        ] {
            let refusal = check(profile, query).unwrap_err();
            assert_eq!(refusal.reason(), AdmissionReason::Assignment);
            assert_eq!(refusal.variable(), Some("this"));
        }
        for query in [
            "SELECT (true AS ?local) WHERE { ?s ?p ?v }",
            "SELECT $this WHERE { ?s ?p ?list UNFOLD(?list AS ?local) }",
        ] {
            check(profile, query).unwrap();
        }
    }
}
