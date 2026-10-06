// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every synchronous entry point that evaluates `REGEX`/`REPLACE` or a SHACL pattern,
//! driven end to end on the native build under each dated law and under none.
//!
//! Two behaviours tell the laws apart. A non-capturing group `(?:a)b` is grammar only
//! under XPath 3.1, so XPath 2.0 refuses it as a pattern error. A backreference
//! `^(a)\1$` matches `"aa"` under both dated laws, while the compatibility regex refuses
//! backreferences. A JavaScript error is a wasm import that cannot be built natively,
//! so every refusal here is read at the layer beneath the binding that throws it.

use crate::AsyncOperationKind;
use crate::dataset::Dataset;
use crate::jsonld::CompiledJsonLdContext;
use crate::operation::{JobOutcome, JobRun, OperationInput};
use crate::query::QueryEngine;
use crate::shacl::requests;

use super::parse;

const XPATH_20: &str = "xpath-2.0-2010-12-14";
const XPATH_31: &str = "xpath-3.1-2017-03-21";

/// Each selection a caller can make, beside what it should answer: `(law, non-capturing
/// group matches, backreference matches)`.
const LAWS: [(Option<&str>, bool, bool); 3] = [
    (None, true, false),
    (Some(XPATH_20), false, true),
    (Some(XPATH_31), true, true),
];

/// The neighbours of a stable name that select nothing.
const REFUSED: [&str; 3] = ["xpath-3.1", "XPATH-3.1-2017-03-21", ""];

const DATA: &str = "<http://example.org/s> <http://example.org/p> \"aa\" .\n\
                    <http://example.org/s> <http://example.org/q> \"ab\" .\n";

const ASK_NON_CAPTURING: &str = r#"ASK { FILTER(REGEX("ab", "(?:a)b")) }"#;
const ASK_BACKREFERENCE: &str = r#"ASK { FILTER(REGEX("aa", "^(a)\\1$")) }"#;
/// One row per value the backreference matches, so a matching law answers one row.
const SELECT_BACKREFERENCE: &str = r#"SELECT ?o WHERE { ?s ?p ?o FILTER(REGEX(?o, "^(a)\\1$")) }"#;
const CONSTRUCT_BACKREFERENCE: &str = r#"CONSTRUCT { ?s <http://example.org/matched> ?o }
    WHERE { ?s ?p ?o FILTER(REGEX(?o, "^(a)\\1$")) }"#;
const DESCRIBE_NON_CAPTURING: &str =
    r#"DESCRIBE ?s WHERE { ?s ?p ?o FILTER(REGEX(?o, "(?:a)b")) }"#;
const INSERT_BACKREFERENCE: &str = r#"INSERT { ?s <http://example.org/matched> ?o }
    WHERE { ?s ?p ?o FILTER(REGEX(?o, "^(a)\\1$")) }"#;
/// `REPLACE` under the law: `"b"` where the backreference matched, unbound where it
/// was refused.
const SELECT_REPLACE: &str = r#"SELECT ?r WHERE { BIND(REPLACE("aa", "^(a)\\1$", "b") AS ?r) }"#;

const JSONLD_OPTIONS: &str =
    r#"{"version":1,"mode":"context","prefixes":{"ex":"http://example.org/"}}"#;

fn data() -> Dataset {
    Dataset::parse(DATA, "ntriples", None).expect("the fixture parses")
}

fn owned(law: Option<&str>) -> Option<String> {
    law.map(str::to_owned)
}

/// The SPARQL Results JSON boolean of an ASK document.
fn srj_boolean(document: &str) -> bool {
    match (
        document.contains("\"boolean\":true") || document.contains("\"boolean\": true"),
        document.contains("\"boolean\":false") || document.contains("\"boolean\": false"),
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => panic!("not an ASK document: {document}"),
    }
}

#[test]
fn every_ask_entry_evaluates_under_the_selected_law() {
    let engine = QueryEngine::new();
    let dataset = data();
    for (law, non_capturing, backreference) in LAWS {
        for (query, expected) in [
            (ASK_NON_CAPTURING, non_capturing),
            (ASK_BACKREFERENCE, backreference),
        ] {
            let what = format!("{law:?} {query}");
            assert_eq!(
                engine.ask(&dataset, query, None, owned(law)).expect(&what),
                expected,
                "ask {what}"
            );
            assert_eq!(
                engine
                    .query(&dataset, query, None, owned(law))
                    .expect(&what)
                    .boolean(),
                Some(expected),
                "query {what}"
            );
            assert_eq!(
                srj_boolean(
                    &engine
                        .query_raw(&dataset, query, None, None, None, None, owned(law))
                        .expect(&what)
                ),
                expected,
                "queryRaw {what}"
            );
            assert_eq!(
                srj_boolean(&dataset.query(query, None, owned(law)).expect(&what)),
                expected,
                "Dataset.query {what}"
            );
            let mut outcome = engine
                .query_governed(
                    &dataset,
                    query,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    owned(law),
                )
                .expect(&what);
            assert_eq!(
                outcome.take_result().expect("complete").boolean(),
                Some(expected),
                "queryGoverned {what}"
            );
            let mut entailed = engine
                .query_entailment_governed(
                    &dataset,
                    query,
                    None,
                    "rdfs",
                    None,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    owned(law),
                )
                .expect(&what);
            assert_eq!(
                entailed
                    .take_outcome()
                    .expect("the closure completed")
                    .take_result()
                    .expect("the query completed")
                    .boolean(),
                Some(expected),
                "queryEntailmentGoverned {what}"
            );
        }
    }
}

#[test]
fn every_row_and_graph_entry_evaluates_under_the_selected_law() {
    let engine = QueryEngine::new();
    let dataset = data();
    for (law, non_capturing, backreference) in LAWS {
        let what = format!("{law:?}");
        let rows = usize::from(backreference);
        assert_eq!(
            engine
                .select(&dataset, SELECT_BACKREFERENCE, None, owned(law))
                .expect(&what)
                .row_count(),
            rows,
            "select {what}"
        );
        let replaced = engine
            .select(&dataset, SELECT_REPLACE, None, owned(law))
            .expect(&what)
            .next_row()
            .expect("one row")
            .get("r");
        assert_eq!(
            replaced.is_some(),
            backreference,
            "REPLACE under {what} binds only where the backreference is grammar"
        );
        assert_eq!(
            engine
                .construct(&dataset, CONSTRUCT_BACKREFERENCE, None, owned(law))
                .expect(&what)
                .size(),
            rows,
            "construct {what}"
        );
        assert_eq!(
            engine
                .describe(&dataset, DESCRIBE_NON_CAPTURING, None, owned(law))
                .expect(&what)
                .size()
                > 0,
            non_capturing,
            "describe {what}"
        );
        let configured = engine
            .query_raw_configured(
                &dataset,
                CONSTRUCT_BACKREFERENCE,
                None,
                "jsonld",
                JSONLD_OPTIONS,
                owned(law),
            )
            .expect(&what);
        assert_eq!(
            configured.contains("ex:matched"),
            backreference,
            "queryRawConfigured {what}: {configured}"
        );
        let context = CompiledJsonLdContext::new(JSONLD_OPTIONS).expect("context compiles");
        let contextual = engine
            .query_raw_with_context(
                &dataset,
                CONSTRUCT_BACKREFERENCE,
                None,
                "jsonld",
                &context,
                None,
                owned(law),
            )
            .expect(&what);
        assert_eq!(contextual, configured, "queryRawWithContext {what}");
        let explained = engine
            .explain_query(&dataset, SELECT_BACKREFERENCE, None, owned(law))
            .expect(&what);
        let filter = explained
            .lines()
            .find(|line| line.contains(" Filter fuel="))
            .unwrap_or_else(|| panic!("the filter is measured: {explained}"));
        assert!(
            filter.contains(&format!("rows={rows} ")),
            "explainQuery {what}: {filter}"
        );
    }
}

#[test]
fn both_update_entries_apply_under_the_selected_law() {
    let engine = QueryEngine::new();
    for (law, _, backreference) in LAWS {
        let what = format!("{law:?}");
        let expected = 2 + usize::from(backreference);
        let mut dataset = data();
        engine
            .update(&mut dataset, INSERT_BACKREFERENCE, None, owned(law))
            .expect(&what);
        assert_eq!(dataset.size(), expected, "update {what}");
        let mut dataset = data();
        let outcome = engine
            .update_governed(
                &mut dataset,
                INSERT_BACKREFERENCE,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                owned(law),
            )
            .expect(&what);
        assert!(outcome.is_applied(), "updateGoverned {what}");
        assert_eq!(dataset.size(), expected, "updateGoverned {what}");
    }
}

/// A pattern past the production source bound is an operational failure of the whole
/// request under either dated law — never an unbound, `false` or truncated answer, and
/// never a governed outcome — carrying the resource's own code. The pattern one byte
/// shorter, exactly at the bound, is admitted and answers.
#[test]
fn a_native_resource_refusal_is_the_operation_s_error() {
    let engine = std::rc::Rc::new(purrdf_sparql_eval::NativeSparqlEngine::new());
    let frozen = data().view().freeze().expect("freezes");
    let ask = |bytes: usize| format!("ASK {{ FILTER(REGEX(\"aa\", \"{}\")) }}", "a".repeat(bytes));
    for law in [XPATH_20, XPATH_31] {
        let profile = parse(Some(law)).expect("a stable name");
        for kind in [
            AsyncOperationKind::Query,
            AsyncOperationKind::Raw,
            AsyncOperationKind::Governed,
            AsyncOperationKind::Explain,
        ] {
            let run = |sparql: String| {
                let mut input = OperationInput::new(
                    kind,
                    &engine,
                    std::sync::Arc::clone(&frozen),
                    sparql,
                    None,
                );
                input.xpath_regex = profile;
                input.execute(&JobRun::offline(None))
            };
            let refused = run(ask(64 * 1024 + 1)).expect_err("the oversized pattern is refused");
            assert_eq!(refused.code(), "xpath-pattern-bytes", "{law} {kind:?}");
            assert!(
                refused.rendered().contains("limit 65536"),
                "{law} {kind:?}: {}",
                refused.rendered()
            );
            match run(ask(64 * 1024)) {
                Ok(JobOutcome::Query(_) | JobOutcome::Raw(_) | JobOutcome::Governed(_)) => {}
                other => panic!("{law} {kind:?}: the bounded pattern answers, got {other:?}"),
            }
        }
    }
}

// ── SHACL ──────────────────────────────────────────────────────────────────────────

/// A Core `sh:pattern` and a SHACL-SPARQL constraint whose `REGEX` reports the focus
/// node when the backreference matches.
fn shapes(pattern: &str) -> String {
    format!(
        r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:PatternShape a sh:NodeShape ;
  sh:targetNode ex:n ;
  sh:property [ sh:path ex:v ; sh:pattern "{pattern}" ] .
"#
    )
}

const SPARQL_SHAPES: &str = r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:SparqlShape a sh:NodeShape ;
  sh:targetNode ex:n ;
  sh:sparql [ sh:select """SELECT $this WHERE { FILTER(REGEX("aa", "^(a)\\\\1$")) }""" ] .
"#;

const NON_CAPTURING: &str = "(?:a)b";
const BACKREFERENCE: &str = "^(a)\\\\1$";

fn value(literal: &str) -> String {
    format!("<http://example.org/n> <http://example.org/v> \"{literal}\" .\n")
}

/// Whether a SARIF log reports a conforming validation.
fn conforms(sarif: &str) -> bool {
    let compact: String = sarif.split_whitespace().collect();
    match (
        compact.contains("\"shaclConforms\":true"),
        compact.contains("\"shaclConforms\":false"),
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => panic!("no verdict in {sarif}"),
    }
}

fn validate(
    shapes: &str,
    data: &str,
    law: Option<&str>,
) -> Result<String, purrdf_validate::XPathValidationError> {
    requests::validate_to_sarif(
        shapes.to_owned(),
        data.to_owned(),
        None,
        None,
        None,
        None,
        None,
        None,
        owned(law),
    )
}

fn validate_changes(
    shapes: &str,
    added: &str,
    law: Option<&str>,
) -> Result<crate::shacl::ShaclChangeValidation, purrdf_validate::XPathValidationError> {
    requests::validate_changes_to_sarif(
        shapes.to_owned(),
        String::new(),
        Some(added.to_owned()),
        None,
        None,
        None,
        None,
        None,
        owned(law),
    )
}

/// The four product entries, over one product, under `law`.
fn product_validations(
    shapes: &str,
    data: &str,
    law: Option<&str>,
) -> [Result<String, crate::shacl::ShaclProductRefusal>; 4] {
    let product = crate::shacl::pack_product_impl(shapes, None, &[], &[], None).expect("packed");
    let identity = crate::shacl::product_explain_impl(&product)
        .expect("explained")
        .lines()
        .find_map(|line| line.strip_prefix("identity-digest ").map(ToOwned::to_owned))
        .expect("an identity digest");
    let data = data.to_owned();
    [
        requests::product_validate_to_sarif(product.clone(), data.clone(), owned(law)),
        requests::product_validate_to_sarif_rebuild(product.clone(), data.clone(), owned(law)),
        requests::product_validate_to_sarif_expecting(
            product.clone(),
            data.clone(),
            identity.clone(),
            owned(law),
        ),
        requests::product_validate_to_sarif_rebuild_expecting(product, data, identity, owned(law)),
    ]
}

#[test]
fn every_shacl_validation_entry_evaluates_under_the_selected_law() {
    for (law, non_capturing, backreference) in LAWS {
        for (pattern, literal, expected) in [
            (NON_CAPTURING, "ab", non_capturing),
            (BACKREFERENCE, "aa", backreference),
        ] {
            let what = format!("{law:?} {pattern}");
            let shapes = shapes(pattern);
            let sarif = validate(&shapes, &value(literal), law).expect(&what);
            assert_eq!(conforms(&sarif), expected, "shaclValidateToSarif {what}");
            let change = validate_changes(&shapes, &value(literal), law).expect(&what);
            assert!(
                change.bounded(),
                "{what}: a Core pattern has a bounded footprint"
            );
            assert_eq!(
                conforms(&change.sarif()),
                expected,
                "shaclValidateChangesToSarif {what}"
            );
            for (entry, validated) in product_validations(&shapes, &value(literal), law)
                .into_iter()
                .enumerate()
            {
                let sarif =
                    validated.unwrap_or_else(|refusal| panic!("{what} #{entry}: {refusal:?}"));
                assert_eq!(conforms(&sarif), expected, "product entry #{entry} {what}");
            }
        }
        // A SHACL-SPARQL constraint's REGEX runs under the same law: the focus node is
        // reported exactly where the backreference matches.
        let what = format!("{law:?} sh:sparql");
        let data = value("x");
        assert_eq!(
            conforms(&validate(SPARQL_SHAPES, &data, law).expect(&what)),
            !backreference,
            "shaclValidateToSarif {what}"
        );
        let change = validate_changes(SPARQL_SHAPES, &data, law).expect(&what);
        assert!(
            !change.bounded(),
            "{what}: SPARQL text has no bounded footprint"
        );
        assert_eq!(conforms(&change.sarif()), !backreference, "changes {what}");
        for (entry, validated) in product_validations(SPARQL_SHAPES, &data, law)
            .into_iter()
            .enumerate()
        {
            let sarif = validated.unwrap_or_else(|refusal| panic!("{what} #{entry}: {refusal:?}"));
            assert_eq!(
                conforms(&sarif),
                !backreference,
                "product entry #{entry} {what}"
            );
        }
    }
}

/// With nothing that tells the laws apart, a selected validation reports byte for byte
/// what the compatibility one does: the selection changes the pattern law and nothing
/// else about the shared validation boundary.
#[test]
fn a_selected_shacl_validation_is_otherwise_the_compatibility_one() {
    let shapes = shapes("^a+$");
    for literal in ["aa", "ab"] {
        let data = value(literal);
        let compatibility = validate(&shapes, &data, None).expect("compatibility");
        let change = validate_changes(&shapes, &data, None).expect("compatibility");
        let products = product_validations(&shapes, &data, None);
        for law in [XPATH_20, XPATH_31] {
            assert_eq!(
                validate(&shapes, &data, Some(law)).expect(law),
                compatibility
            );
            let selected = validate_changes(&shapes, &data, Some(law)).expect(law);
            assert_eq!(selected.sarif(), change.sarif(), "{law}");
            assert_eq!(selected.focus_nodes(), change.focus_nodes(), "{law}");
            for (selected, compatibility) in product_validations(&shapes, &data, Some(law))
                .into_iter()
                .zip(&products)
            {
                assert_eq!(
                    selected.expect(law),
                    *compatibility.as_ref().expect("compatibility")
                );
            }
        }
    }
}

/// An oversized `sh:pattern` is the validation's error under either law, with no report,
/// identified by the resource's own code exactly as a SPARQL entry's refusal is: the
/// code the synchronous entry's `Error` carries as `code`, and a product refusal's
/// `code` beside its absent dimension. The pattern exactly at the bound validates.
#[test]
fn a_shacl_resource_refusal_is_the_validation_s_error() {
    use crate::shacl::{selected_error_message, selected_refusal_code};
    let oversized = shapes(&"a".repeat(64 * 1024 + 1));
    let bounded = shapes(&"a".repeat(64 * 1024));
    let data = value("aa");
    for law in [XPATH_20, XPATH_31] {
        let refused = validate(&oversized, &data, Some(law)).expect_err("refused");
        assert_eq!(
            selected_refusal_code(&refused),
            Some("xpath-pattern-bytes"),
            "{law}"
        );
        let message = selected_error_message(&refused);
        assert!(
            message.starts_with("xpath-pattern-bytes"),
            "{law}: {message}"
        );
        let refused = validate_changes(&oversized, &data, Some(law))
            .map(|change| change.sarif())
            .expect_err("refused");
        assert_eq!(
            selected_refusal_code(&refused),
            Some("xpath-pattern-bytes"),
            "{law}"
        );
        let message = selected_error_message(&refused);
        assert!(
            message.starts_with("xpath-pattern-bytes"),
            "{law}: {message}"
        );
        for refusal in product_validations(&oversized, &data, Some(law)) {
            let refusal = refusal.expect_err("refused");
            assert_eq!(refusal.dimension(), None, "{law}");
            assert_eq!(
                refusal.code().as_deref(),
                Some("xpath-pattern-bytes"),
                "{law}"
            );
            assert!(refusal.message().contains("xpath-pattern-bytes"), "{law}");
        }
        assert!(!conforms(&validate(&bounded, &data, Some(law)).expect(law)));
        assert!(!conforms(
            &validate_changes(&bounded, &data, Some(law))
                .expect(law)
                .sarif()
        ));
        for validated in product_validations(&bounded, &data, Some(law)) {
            assert!(!conforms(&validated.expect(law)));
        }
    }
}

/// A SHACL-SPARQL constraint whose `REGEX` pattern is past the source bound is refused
/// by the query it runs; that diagnostic is identified by the resource's own code too.
#[test]
fn a_shacl_sparql_resource_refusal_carries_the_resource_code() {
    let sparql = |bytes: usize| {
        format!(
            r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:SparqlShape a sh:NodeShape ;
  sh:targetNode ex:n ;
  sh:sparql [ sh:select """SELECT $this WHERE {{ FILTER(REGEX("aa", "{}")) }}""" ] .
"#,
            "a".repeat(bytes)
        )
    };
    let data = value("x");
    for law in [XPATH_20, XPATH_31] {
        let refused = validate(&sparql(64 * 1024 + 1), &data, Some(law)).expect_err("refused");
        assert_eq!(
            crate::shacl::selected_refusal_code(&refused),
            Some("xpath-pattern-bytes"),
            "{law}: {refused}"
        );
        for refusal in product_validations(&sparql(64 * 1024 + 1), &data, Some(law)) {
            assert_eq!(
                refusal.expect_err("refused").code().as_deref(),
                Some("xpath-pattern-bytes"),
                "{law}"
            );
        }
        assert!(conforms(
            &validate(&sparql(64 * 1024), &data, Some(law)).expect(law)
        ));
    }
}

/// A name that selects no law is refused by every SHACL entry before anything is
/// validated, in the words every surface uses; the exact name beside it is accepted.
#[test]
fn every_shacl_entry_refuses_a_name_that_selects_no_law() {
    let shapes = shapes(NON_CAPTURING);
    let data = value("ab");
    for name in REFUSED {
        let expected = parse(Some(name)).expect_err(name);
        assert_eq!(
            crate::shacl::selected_error_message(
                &validate(&shapes, &data, Some(name)).expect_err(name)
            ),
            expected
        );
        assert_eq!(
            crate::shacl::selected_error_message(
                &validate_changes(&shapes, &data, Some(name))
                    .map(|change| change.sarif())
                    .expect_err(name)
            ),
            expected
        );
        for refusal in product_validations(&shapes, &data, Some(name)) {
            let refusal = refusal.expect_err(name);
            assert_eq!(refusal.dimension(), None, "{name:?}");
            assert_eq!(refusal.code(), None, "{name:?}");
            assert_eq!(refusal.message(), expected);
        }
    }
    assert!(conforms(
        &validate(&shapes, &data, Some(XPATH_31)).expect("accepted")
    ));
    assert!(conforms(
        &validate_changes(&shapes, &data, Some(XPATH_31))
            .expect("accepted")
            .sarif()
    ));
    for validated in product_validations(&shapes, &data, Some(XPATH_31)) {
        assert!(conforms(&validated.expect("accepted")));
    }
}

/// A SPARQL rule inferring `?this ex:hit ?v` for each `ex:p` value `pattern` matches.
fn rule_shapes(pattern: &str) -> String {
    let construct = format!(
        "CONSTRUCT {{ $this <http://example.org/hit> ?v }} \
         WHERE {{ $this <http://example.org/p> ?v FILTER(REGEX(?v, \"{}\")) }}",
        pattern.replace('\\', "\\\\")
    );
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <http://example.org/> .\n\
         ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ;\n  \
         sh:rule [ a sh:SPARQLRule ; sh:construct \"{}\" ] .\n",
        construct.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

/// A filter expression `_:e` keeping the focus node when `pattern` matches it.
fn expression_shapes(pattern: &str) -> String {
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n\
         _:e shnex:filterShape [ sh:pattern \"{}\" ] ; shnex:nodes [ shnex:var \"focusNode\" ] .\n",
        pattern.replace('\\', "\\\\")
    )
}

/// `shaclEntail`, `shaclApplyRules` and `shaclEvalNodeExpr` over `pattern`, each read as
/// "did the pattern match `value`?", or the entry's error: the resource code it is
/// identified by, if any, and its message.
type ToolVerdict = Result<bool, (Option<&'static str>, String)>;

fn tool_verdicts(pattern: &str, value: &str, law: Option<&str>) -> [ToolVerdict; 3] {
    let data = format!("<http://example.org/s> <http://example.org/p> \"{value}\" .\n");
    let rules = rule_shapes(pattern);
    let message = |error: purrdf_validate::XPathValidationError| {
        (
            crate::shacl::selected_refusal_code(&error),
            crate::shacl::selected_error_message(&error),
        )
    };
    let hit = "<http://example.org/hit>";
    [
        requests::entail(
            rules.clone(),
            data.clone(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            owned(law),
        )
        .map(|entailment| entailment.ntriples().contains(hit))
        .map_err(message),
        requests::apply_rules(
            data.clone(),
            Some(rules),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            owned(law),
        )
        .map(|inference| inference.inferred().contains(hit))
        .map_err(message),
        requests::eval_node_expr(
            expression_shapes(pattern),
            data,
            Some("_:e".to_owned()),
            format!("\"{value}\""),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            owned(law),
        )
        .map(|outcome| !outcome.outputs().is_empty())
        .map_err(message),
    ]
}

/// Entailment, rules and node expressions evaluate their patterns under the selected
/// law; a resource refusal is the entry's error beside the admitted neighbour at the
/// bound; and a name that selects no law is refused beside its accepted exact neighbour.
#[test]
fn every_shapes_graph_tool_evaluates_under_the_selected_law() {
    for (law, non_capturing, backreference) in LAWS {
        for verdict in tool_verdicts(NON_CAPTURING, "ab", law) {
            assert_eq!(verdict, Ok(non_capturing), "{law:?}");
        }
        for verdict in tool_verdicts(r"^(a)\1$", "aa", law) {
            assert_eq!(verdict, Ok(backreference), "{law:?}");
        }
    }
    for law in [XPATH_20, XPATH_31] {
        for verdict in tool_verdicts(&"a".repeat(64 * 1024 + 1), "a", Some(law)) {
            let (code, message) = verdict.expect_err("refused");
            assert_eq!(code, Some("xpath-pattern-bytes"), "{law}: {message}");
            assert!(message.contains("xpath-pattern-bytes"), "{law}: {message}");
        }
        for verdict in tool_verdicts(&"a".repeat(64 * 1024), "a", Some(law)) {
            assert_eq!(verdict, Ok(false), "{law}");
        }
    }
    for name in REFUSED {
        let expected = parse(Some(name)).expect_err(name);
        for verdict in tool_verdicts("a", "a", Some(name)) {
            assert_eq!(verdict, Err((None, expected.clone())), "{name:?}");
        }
    }
    for verdict in tool_verdicts("a", "a", Some(XPATH_31)) {
        assert_eq!(verdict, Ok(true));
    }
}
