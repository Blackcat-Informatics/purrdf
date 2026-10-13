// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every `*_with_xpath_regex` entry point of the shared host boundary: `None` is its
//! compatibility twin's exact answer, each dated law shows a behaviour only it has, and a
//! resource the law withholds refuses the request beside the admitted neighbour at the
//! bound.

use purrdf_core::xsd_regex::xpath::{Error, Profile, Resource};
use purrdf_validate::xpath_regex::parse_profile;
use purrdf_validate::{
    EntailRequest, ExprSelector, NodeExprRequest, RulesRequest, SarifOptions, SelectedProductError,
    ShapesProductRefusal, XPathValidationError, apply_rules_to_ntriples,
    apply_rules_to_ntriples_with_xpath_regex, entail_to_ntriples,
    entail_to_ntriples_with_xpath_regex, eval_node_expr, eval_node_expr_with_xpath_regex,
    pack_shapes_product, validate_changes_to_sarif_string_with_shapes_graph,
    validate_changes_to_sarif_string_with_xpath_regex, validate_to_sarif_string_with_shapes_graph,
    validate_to_sarif_string_with_xpath_regex, validate_with_rebuilt_shapes_product,
    validate_with_rebuilt_shapes_product_with_xpath_regex, validate_with_shapes_product,
    validate_with_shapes_product_with_xpath_regex,
};

/// Non-capturing groups are XPath 3.1 only; under 2.0 the pattern is ill-formed.
const NONCAPTURING: &str = "(?:a)b";
/// Both dated laws define backreferences; the compatibility law refuses them.
const BACKREFERENCE: &str = r"^(a)\1$";
const PATTERN_COMPONENT: &str = "PatternConstraintComponent";

fn data(value: &str) -> String {
    format!("<http://example.org/n> <http://example.org/p> {value:?} .\n")
}

fn pattern_shapes(pattern: &str) -> String {
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <http://example.org/> .\n\
         ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ;\n\
           sh:property [ sh:path ex:p ; sh:pattern {pattern:?} ] .\n"
    )
}

fn rule_shapes(pattern: &str) -> String {
    let construct = format!(
        "CONSTRUCT {{ $this <http://example.org/hit> ?v }} \
         WHERE {{ $this <http://example.org/p> ?v FILTER(REGEX(?v, {pattern:?})) }}"
    );
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <http://example.org/> .\n\
         ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ;\n\
           sh:rule [ a sh:SPARQLRule ; sh:construct {construct:?} ] .\n"
    )
}

fn rule_set(pattern: &str) -> String {
    format!(
        "PREFIX : <http://example.org/>\n\
         RULE {{ ?s :hit ?v }} WHERE {{ ?s :p ?v FILTER(REGEX(?v, {pattern:?})) }}"
    )
}

fn expression_shapes(pattern: &str) -> String {
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n\
         _:e shnex:filterShape [ sh:pattern {pattern:?} ] ; shnex:nodes [ shnex:var \"focusNode\" ] .\n"
    )
}

/// The law-distinguishing cases: `(pattern, value, matches under 2.0, under 3.1)`.
const CASES: [(&str, &str, bool, bool); 2] = [
    (NONCAPTURING, "ab", false, true),
    (BACKREFERENCE, "aa", true, true),
];

fn conforms(sarif: &str) -> bool {
    let log = purrdf_lex::json::read(sarif).expect("SARIF JSON");
    log["runs"][0]["properties"]["shaclConforms"] == purrdf_lex::json::Value::Bool(true)
}

fn sarif(pattern: &str, value: &str, law: Option<Profile>) -> Result<String, XPathValidationError> {
    validate_to_sarif_string_with_xpath_regex(
        &pattern_shapes(pattern),
        None,
        None,
        &data(value),
        &SarifOptions::default(),
        &[],
        law,
    )
}

fn change(
    pattern: &str,
    value: &str,
    law: Option<Profile>,
) -> Result<(String, purrdf_validate::ChangeScope), XPathValidationError> {
    validate_changes_to_sarif_string_with_xpath_regex(
        &pattern_shapes(pattern),
        None,
        None,
        "<http://example.org/other> <http://example.org/q> \"x\" .\n",
        Some(&data(value)),
        None,
        &SarifOptions::default(),
        &[],
        law,
    )
}

#[test]
fn whole_graph_and_change_validation_follow_the_selected_law() {
    for (pattern, value, xpath20, xpath31) in CASES {
        for (law, expected) in [(Profile::Xpath20, xpath20), (Profile::Xpath31, xpath31)] {
            let log = sarif(pattern, value, Some(law)).unwrap();
            assert_eq!(conforms(&log), expected, "{pattern} under {}", law.name());
            assert_eq!(log.contains(PATTERN_COMPONENT), !expected);
            let (log, scope) = change(pattern, value, Some(law)).unwrap();
            assert_eq!(
                scope,
                purrdf_validate::ChangeScope::Bounded { focus_nodes: 1 }
            );
            assert_eq!(
                conforms(&log),
                expected,
                "change {pattern} under {}",
                law.name()
            );
        }
    }
    // Unselected is the compatibility entry's exact log, which refuses the backreference
    // both dated laws match.
    let unselected = sarif(BACKREFERENCE, "aa", None).unwrap();
    assert_eq!(
        unselected,
        validate_to_sarif_string_with_shapes_graph(
            &pattern_shapes(BACKREFERENCE),
            None,
            None,
            &data("aa"),
            &SarifOptions::default(),
            &[],
        )
        .unwrap()
    );
    assert!(!conforms(&unselected));
    assert_eq!(
        change(BACKREFERENCE, "aa", None).unwrap(),
        validate_changes_to_sarif_string_with_shapes_graph(
            &pattern_shapes(BACKREFERENCE),
            None,
            None,
            "<http://example.org/other> <http://example.org/q> \"x\" .\n",
            Some(&data("aa")),
            None,
            &SarifOptions::default(),
            &[],
        )
        .unwrap()
    );
}

#[test]
fn a_validation_resource_refusal_writes_no_log_and_the_neighbour_does() {
    // One byte over the 64 KiB production source bound, and exactly the bound.
    let (over, at) = ("a".repeat(64 * 1024 + 1), "a".repeat(64 * 1024));
    for law in Profile::ALL {
        for refused in [
            sarif(&over, "a", Some(law)).map(|_| ()),
            change(&over, "a", Some(law)).map(|_| ()),
        ] {
            assert!(matches!(
                refused,
                Err(XPathValidationError::Pattern(Error::Resource(refusal)))
                    if refusal.resource == Resource::PatternBytes
            ));
        }
        assert!(!conforms(&sarif(&at, "a", Some(law)).unwrap()));
        assert!(!conforms(&change(&at, "a", Some(law)).unwrap().0));
    }
    // A shapes refusal keeps the compatibility entry's own error.
    assert!(matches!(
        validate_to_sarif_string_with_xpath_regex(
            "@@@ not turtle",
            None,
            None,
            &data("a"),
            &SarifOptions::default(),
            &[],
            Some(Profile::Xpath31),
        ),
        Err(XPathValidationError::Shapes(_))
    ));
}

#[test]
fn product_validation_follows_the_selected_law() {
    for (pattern, value, xpath20, xpath31) in CASES {
        let product = pack_shapes_product(&pattern_shapes(pattern), None, &[]).unwrap();
        for (law, expected) in [(Profile::Xpath20, xpath20), (Profile::Xpath31, xpath31)] {
            for log in [
                validate_with_shapes_product_with_xpath_regex(
                    &product,
                    &data(value),
                    None,
                    &SarifOptions::default(),
                    Some(law),
                )
                .unwrap(),
                validate_with_rebuilt_shapes_product_with_xpath_regex(
                    &product,
                    &data(value),
                    None,
                    &SarifOptions::default(),
                    Some(law),
                )
                .unwrap(),
            ] {
                assert_eq!(conforms(&log), expected, "{pattern} under {}", law.name());
            }
        }
    }
    let product = pack_shapes_product(&pattern_shapes(BACKREFERENCE), None, &[]).unwrap();
    assert_eq!(
        validate_with_shapes_product_with_xpath_regex(
            &product,
            &data("aa"),
            None,
            &SarifOptions::default(),
            None
        )
        .unwrap(),
        validate_with_shapes_product(&product, &data("aa"), &SarifOptions::default()).unwrap()
    );
    assert_eq!(
        validate_with_rebuilt_shapes_product_with_xpath_regex(
            &product,
            &data("aa"),
            None,
            &SarifOptions::default(),
            None
        )
        .unwrap(),
        validate_with_rebuilt_shapes_product(&product, &data("aa"), &SarifOptions::default())
            .unwrap()
    );
}

#[test]
fn a_product_refusal_keeps_its_dimension_and_a_law_refusal_is_its_own() {
    // One byte over the 64 KiB production source bound, and exactly the bound.
    let (over, at) = ("a".repeat(64 * 1024 + 1), "a".repeat(64 * 1024));
    let product = pack_shapes_product(&pattern_shapes(&over), None, &[]).unwrap();
    let neighbour = pack_shapes_product(&pattern_shapes(&at), None, &[]).unwrap();
    let wrong = [0_u8; 32];
    for law in Profile::ALL {
        assert!(matches!(
            validate_with_shapes_product_with_xpath_regex(
                &product,
                &data("a"),
                None,
                &SarifOptions::default(),
                Some(law),
            ),
            Err(SelectedProductError::Law(XPathValidationError::Pattern(Error::Resource(refusal))))
                if refusal.resource == Resource::PatternBytes
        ));
        assert!(!conforms(
            &validate_with_rebuilt_shapes_product_with_xpath_regex(
                &neighbour,
                &data("a"),
                None,
                &SarifOptions::default(),
                Some(law),
            )
            .unwrap()
        ));
        // The identity is compared before any validation, as on the compatibility path.
        let Err(SelectedProductError::Product(refusal)) =
            validate_with_shapes_product_with_xpath_regex(
                &neighbour,
                &data("a"),
                Some(&wrong),
                &SarifOptions::default(),
                Some(law),
            )
        else {
            panic!("a product that is not the expected one is refused");
        };
        assert_eq!(refusal.dimension_label(), Some("shapes-graph"));
        // A malformed data graph is the dimensionless refusal it always was.
        assert!(matches!(
            validate_with_shapes_product_with_xpath_regex(
                &neighbour,
                "not n-triples",
                None,
                &SarifOptions::default(),
                Some(law),
            ),
            Err(SelectedProductError::Product(ShapesProductRefusal::Shapes(
                _
            )))
        ));
    }
}

fn entail(
    pattern: &str,
    value: &str,
    law: Option<Profile>,
) -> Result<String, XPathValidationError> {
    let shapes = rule_shapes(pattern);
    let data = data(value);
    let request = EntailRequest {
        shapes_ttl: &shapes,
        shapes_base: None,
        shapes_graph: None,
        data_nt: &data,
        imports: &[],
        max_term_generating_rounds: None,
        max_generated_terms: None,
        max_stored_facts: None,
        max_join_steps: None,
        host: purrdf_validate::RulesHost::Rust,
    };
    let selected = entail_to_ntriples_with_xpath_regex(&request, law)?;
    if law.is_none() {
        assert_eq!(selected, entail_to_ntriples(&request).unwrap());
    }
    Ok(selected.ntriples)
}

fn rules(
    pattern: &str,
    value: &str,
    srl: bool,
    law: Option<Profile>,
) -> Result<String, XPathValidationError> {
    let shapes = rule_shapes(pattern);
    let rule_set = rule_set(pattern);
    let data = data(value);
    let request = RulesRequest {
        data_nt: &data,
        shapes_ttl: (!srl).then_some(shapes.as_str()),
        srl: srl.then_some(rule_set.as_str()),
        imports: &[],
        ..RulesRequest::default()
    };
    let selected = apply_rules_to_ntriples_with_xpath_regex(&request, law)?;
    if law.is_none() {
        assert_eq!(selected, apply_rules_to_ntriples(&request).unwrap());
    }
    Ok(selected.inferred_ntriples)
}

fn node_expr(
    pattern: &str,
    value: &str,
    law: Option<Profile>,
) -> Result<Vec<String>, XPathValidationError> {
    let shapes = expression_shapes(pattern);
    let focus = format!("{value:?}");
    let request = NodeExprRequest {
        shapes_ttl: &shapes,
        shapes_base: None,
        data_nt: "",
        expr: ExprSelector::Node("_:e"),
        focus: &focus,
        scope: &[],
        imports: &[],
    };
    let selected = eval_node_expr_with_xpath_regex(&request, law)?;
    if law.is_none() {
        assert_eq!(selected, eval_node_expr(&request).unwrap());
    }
    Ok(selected.outputs)
}

#[test]
fn rules_entailment_and_node_expressions_follow_the_selected_law() {
    let hit = "<http://example.org/hit>";
    for (pattern, value, xpath20, xpath31) in CASES {
        for (law, expected) in [(Profile::Xpath20, xpath20), (Profile::Xpath31, xpath31)] {
            let name = law.name();
            assert_eq!(
                entail(pattern, value, Some(law)).unwrap().contains(hit),
                expected,
                "entail {pattern} under {name}"
            );
            for srl in [false, true] {
                assert_eq!(
                    !rules(pattern, value, srl, Some(law)).unwrap().is_empty(),
                    expected,
                    "rules (srl: {srl}) {pattern} under {name}"
                );
            }
            assert_eq!(
                node_expr(pattern, value, Some(law)).unwrap(),
                if expected {
                    vec![format!("{value:?}")]
                } else {
                    Vec::new()
                },
                "node expression {pattern} under {name}"
            );
        }
    }
    // Unselected is the compatibility answer, which refuses the backreference.
    assert!(!entail(BACKREFERENCE, "aa", None).unwrap().contains(hit));
    assert_eq!(rules(BACKREFERENCE, "aa", false, None).unwrap(), "");
    assert_eq!(rules(BACKREFERENCE, "aa", true, None).unwrap(), "");
    assert_eq!(
        node_expr(BACKREFERENCE, "aa", None).unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn a_rules_or_expression_resource_refusal_answers_nothing_and_the_neighbour_answers() {
    // One byte over the 64 KiB production source bound, and exactly the bound.
    let (over, at) = ("a".repeat(64 * 1024 + 1), "a".repeat(64 * 1024));
    for law in Profile::ALL {
        for refused in [
            entail(&over, "a", Some(law)).map(|_| ()),
            rules(&over, "a", false, Some(law)).map(|_| ()),
            rules(&over, "a", true, Some(law)).map(|_| ()),
        ] {
            assert!(matches!(
                refused,
                Err(XPathValidationError::Query(diagnostic))
                    if diagnostic.code == Resource::PatternBytes.code()
            ));
        }
        assert!(matches!(
            node_expr(&over, "a", Some(law)),
            Err(XPathValidationError::Pattern(Error::Resource(refusal)))
                if refusal.resource == Resource::PatternBytes
        ));
        assert!(!entail(&at, "a", Some(law)).unwrap().contains("hit"));
        assert_eq!(rules(&at, "a", false, Some(law)).unwrap(), "");
        assert_eq!(rules(&at, "a", true, Some(law)).unwrap(), "");
        assert_eq!(
            node_expr(&at, "a", Some(law)).unwrap(),
            Vec::<String>::new()
        );
    }
    // A request the compatibility run refuses is refused identically.
    let neither = RulesRequest {
        data_nt: "",
        ..RulesRequest::default()
    };
    let Err(XPathValidationError::Shapes(selected)) =
        apply_rules_to_ntriples_with_xpath_regex(&neither, Some(Profile::Xpath31))
    else {
        panic!("a request with no rule source is refused");
    };
    assert_eq!(
        selected.to_string(),
        apply_rules_to_ntriples(&neither).unwrap_err().to_string()
    );
}

#[test]
fn only_an_exact_name_selects_a_law() {
    for name in ["xpath-3.1", "XPATH-3.1-2017-03-21", ""] {
        assert!(parse_profile(Some(name)).is_err(), "{name:?}");
    }
    assert_eq!(
        parse_profile(Some("xpath-3.1-2017-03-21")),
        Ok(Some(Profile::Xpath31))
    );
    assert_eq!(
        parse_profile(Some("xpath-2.0-2010-12-14")),
        Ok(Some(Profile::Xpath20))
    );
}
