// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The spec symbol table and its linker (`purrdf_shapes::spec`).
//!
//! Three claims, each executed rather than asserted in prose:
//!
//! 1. **The ratchet.** What the vendored W3C SHACL 1.2 vocabularies DECLARE and
//!    what the table IMPLEMENTS agree exactly — every component, every function,
//!    every key parameter and every parameter set — and the declared-vs-implemented
//!    gap is empty for functions and for components alike.
//! 2. **The linker's outcomes.** A built-in's bare declaration binds and indexes
//!    nothing; a built-in component's declared validators bind as alternatives the
//!    native implementation supersedes; a second definition, a kind or signature
//!    mismatch, a semantic statement on a built-in, an ill-formed validator and a key
//!    clash are refused. Every refusal is proven
//!    next to a VALID neighbour whose outcome differs from the refusal's.
//! 3. **The resolution report.** Every call site names what it bound to.
//!
//! Test IRIs live under `example.org`; every `sh:` / `shnex:` / `sparql:` term used
//! here is defined by the W3C specification that declares it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::function_resolution::FunctionBinding;
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::shapes::{__linked_declarations, Constraint};
use purrdf_shapes::spec::{Carrier, FunctionClass, SPEC_TEXT_OPTIONALITY, declared, implemented};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = r"
@prefix ex:     <http://example.org/ns#> .
@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:   <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix shnex:  <http://www.w3.org/ns/shacl-node-expr#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix xsd:    <http://www.w3.org/2001/XMLSchema#> .
";

/// The three vendored vocabulary files, byte for byte.
const VOCABULARY_FILES: [&str; 3] = [
    include_str!("../spec/shacl.ttl"),
    include_str!("../spec/shnex.ttl"),
    include_str!("../spec/shnex-sparql.ttl"),
];

const SPARQL_NS: &str = "http://www.w3.org/ns/sparql#";

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load(shapes_ttl: &str) -> Result<purrdf_shapes::shapes::Shapes, String> {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).map_err(String::from)
}

fn load_error(shapes_ttl: &str) -> String {
    load(shapes_ttl).expect_err("the shapes graph must be refused at load")
}

fn data_of(data_ttl: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parses")
}

fn validate(shapes_ttl: &str, data_ttl: &str) -> ValidationReport {
    let shapes = load(shapes_ttl).expect("shapes load");
    validate_dataset_with_shapes_graph(&data_of(data_ttl), &shapes, None).expect("validation runs")
}

/// The focus nodes a report names, sorted.
fn focus_nodes(report: &ValidationReport) -> Vec<String> {
    let mut out: Vec<String> = report
        .results
        .iter()
        .map(|r| r.focus_node.to_string())
        .collect();
    out.sort();
    out
}

fn linked(shapes_ttl: &str) -> purrdf_shapes::shapes::LinkedDeclarations {
    let dataset =
        parse_turtle_to_dataset(&format!("{PREFIXES}{shapes_ttl}"), None).expect("parses");
    __linked_declarations(&dataset).expect("the declarations link")
}

/// The merged vocabulary as ONE dataset.
fn vocabulary_dataset() -> Arc<RdfDataset> {
    let merged: String = VOCABULARY_FILES.concat();
    parse_turtle_to_dataset(&merged, None).expect("the merged vocabulary parses")
}

/// The number of function declarations in the raw vocabulary TEXT, counted by
/// scanning the files for the declaring `a <class>` lines — independently of the
/// RDF reader under test.
fn function_declarations_in_text() -> usize {
    VOCABULARY_FILES
        .iter()
        .flat_map(|text| text.lines())
        .filter(|line| {
            let line = line.trim();
            line == "a sh:NamedParameterExpressionFunction ;"
                || line == "a sh:ListParameterExpressionFunction ;"
                || line == "a sh:NodeExpressionFunction ;"
        })
        .count()
}

/// The number of component declarations in the raw vocabulary text.
fn component_declarations_in_text() -> usize {
    VOCABULARY_FILES
        .iter()
        .flat_map(|text| text.lines())
        .filter(|line| line.trim() == "a sh:ConstraintComponent ;")
        .count()
}

// ── 1. The ratchet ────────────────────────────────────────────────────────────

/// Every function the vocabularies declare binds to the table with the SAME class
/// and the SAME parameter set, key flags and optionality included (optionality
/// modulo the pinned spec-text ledger); every table row is declared.
#[test]
fn every_declared_function_binds_with_its_declared_signature() {
    let vocab = declared().expect("the vendored vocabularies read");
    let counted = function_declarations_in_text();
    assert_eq!(
        vocab.functions.len(),
        counted,
        "the RDF reader and the text scan disagree on how many functions are declared"
    );
    let table = implemented();
    let function_gap: BTreeSet<&str> = vocab
        .functions
        .keys()
        .map(String::as_str)
        .filter(|iri| table.function(iri).is_none())
        .collect();
    assert!(
        function_gap.is_empty(),
        "declared − implemented functions must be empty: {function_gap:?}"
    );
    let optionality_overrides: BTreeSet<(&str, &str)> =
        SPEC_TEXT_OPTIONALITY.iter().copied().collect();
    let mut overrides_used: BTreeSet<(&str, &str)> = BTreeSet::new();
    for (iri, declaration) in &vocab.functions {
        let (class, params) = table
            .function(iri)
            .unwrap_or_else(|| panic!("<{iri}> is declared but the engine does not bind it"));
        assert_eq!(class, declaration.class, "<{iri}>: declaring class");
        let implemented_params: BTreeMap<&str, (bool, bool)> = params
            .iter()
            .map(|p| (p.path, (p.key, p.optional)))
            .collect();
        let declared_params: BTreeMap<&str, (bool, bool)> = declaration
            .params
            .iter()
            .map(|p| (p.path.as_str(), (p.key, p.optional)))
            .collect();
        assert_eq!(
            implemented_params.keys().collect::<Vec<_>>(),
            declared_params.keys().collect::<Vec<_>>(),
            "<{iri}>: the parameter set"
        );
        for (path, &(key, optional)) in &implemented_params {
            let (declared_key, declared_optional) = declared_params[path];
            assert_eq!(key, declared_key, "<{iri}> <{path}>: sh:keyParameter");
            if optional != declared_optional {
                let entry = optionality_overrides
                    .get(&(iri.as_str(), *path))
                    .unwrap_or_else(|| {
                        panic!(
                            "<{iri}> <{path}>: optionality differs from the vocabulary and the \
                             spec-text ledger does not record it"
                        )
                    });
                overrides_used.insert(*entry);
            }
        }
    }
    assert_eq!(
        overrides_used, optionality_overrides,
        "every spec-text optionality entry must be a REAL disagreement with the vocabulary"
    );
    for row in table.functions() {
        assert!(
            vocab.functions.contains_key(row.iri()),
            "the table implements <{}>, which the vocabulary does not declare",
            row.iri()
        );
    }
    // The sparql: declarations are bound by the SPARQL lowering, not by table rows;
    // the two alias spellings the vocabulary uses are among them.
    let sparql_declared: BTreeSet<&str> = vocab
        .functions
        .keys()
        .filter_map(|iri| iri.strip_prefix(SPARQL_NS))
        .collect();
    for alias in table.sparql_aliases() {
        assert!(
            sparql_declared.contains(alias.local),
            "the alias sparql:{} is not a spelling the vocabulary declares",
            alias.local
        );
    }
    // Pinned from the files: 2 sh: + 22 shnex: named + shnex:conformsToShape +
    // shnex:EmptyExpression + 77 sparql:.
    assert_eq!(table.functions().len() + sparql_declared.len(), counted);
    assert_eq!(
        counted, 103,
        "the vendored vocabularies declare 103 functions"
    );
}

/// Every component the vocabulary declares is a table row with the SAME parameter
/// set and optionality, and the engine evaluates every one of them: the
/// declared-vs-implemented gap is EMPTY, and a row that stopped naming its
/// carrier in the parsed model would reopen it.
#[test]
fn every_declared_component_is_a_row_and_the_gap_is_exactly_pinned() {
    let vocab = declared().expect("the vendored vocabularies read");
    assert_eq!(
        vocab.components.len(),
        component_declarations_in_text(),
        "the RDF reader and the text scan disagree on how many components are declared"
    );
    assert_eq!(vocab.components.len(), 42);
    let table = implemented();
    let rows: BTreeSet<&str> = table
        .components()
        .iter()
        .map(purrdf_shapes::spec::ComponentRow::iri)
        .collect();
    let declared_iris: BTreeSet<&str> = vocab.components.keys().map(String::as_str).collect();
    assert_eq!(rows, declared_iris, "component rows == declared components");
    for row in table.components() {
        let declared_params: BTreeSet<(&str, bool)> = vocab.components[row.iri()]
            .iter()
            .map(|p| (p.path.as_str(), p.optional))
            .collect();
        let row_params: BTreeSet<(&str, bool)> =
            row.params().iter().map(|p| (p.path, p.optional)).collect();
        assert_eq!(row_params, declared_params, "<{}>: parameters", row.iri());
    }
    let implemented_iris: BTreeSet<&str> = table
        .components()
        .iter()
        .filter(|row| match row.carrier() {
            Carrier::Constraint(variants) => !variants.is_empty(),
            Carrier::ShapeField(field) => !field.is_empty(),
        })
        .map(purrdf_shapes::spec::ComponentRow::iri)
        .collect();
    let gap: BTreeSet<&str> = declared_iris
        .difference(&implemented_iris)
        .copied()
        .collect();
    assert!(
        gap.is_empty(),
        "declared − implemented components must be empty: {gap:?}"
    );
}

/// The table's canonical text is deterministic and names every row.
#[test]
fn the_table_renders_every_row_deterministically() {
    let table = implemented();
    let text = table.canonical_text();
    assert_eq!(text, implemented().canonical_text());
    for row in table.functions() {
        assert!(text.contains(row.iri()), "{} missing", row.iri());
    }
    for row in table.components() {
        assert!(text.contains(row.iri()), "{} missing", row.iri());
    }
    assert!(text.contains("sparql-alias plus = add"));
    assert!(text.contains("sparql-alias encode = encodeForUri"));
}

// ── 2. The linker: kept refusals, each beside a valid neighbour ──────────────

const BODYLESS_EX_F: &str = r"
ex:F a sh:ListParameterExpressionFunction ;
  sh:parameter [ sh:path shnex:arg0 ] .
ex:S a sh:NodeShape ; sh:targetNode ex:a ;
  sh:expression [ sparql:equals ( [ ex:F ( 2 ) ] 4 ) ] .
";

const BODIED_EX_F: &str = r"
ex:F a sh:ListParameterExpressionFunction ;
  sh:parameter [ sh:path shnex:arg0 ] ;
  sh:bodyExpression [ sparql:multiply ( [ shnex:arg 0 ] 2 ) ] .
ex:S a sh:NodeShape ; sh:targetNode ex:a, ex:b ;
  sh:expression [ sparql:equals ( [ ex:F ( [ sh:path ex:n ] ) ] 4 ) ] .
";

#[test]
fn a_bodiless_custom_function_is_refused() {
    let error = load_error(BODYLESS_EX_F);
    assert!(error.contains("declares 0 sh:bodyExpression"), "{error}");
}

#[test]
fn a_bodied_custom_function_loads_and_evaluates() {
    let report = validate(BODIED_EX_F, "ex:a ex:n 2 . ex:b ex:n 3 .");
    assert_eq!(
        focus_nodes(&report),
        vec!["<http://example.org/ns#b>".to_owned()],
        "2*2 = 4 conforms, 3*2 = 6 does not"
    );
}

const BODYLESS_NOT_A_BUILTIN: &str = r"
sh:NotABuiltin a sh:NamedParameterExpressionFunction ;
  sh:parameter [ sh:path ex:notABuiltinKey ; sh:keyParameter true ] .
";

/// The W3C SHACL 1.2 declaration of `sh:SPARQLExprExpression`, verbatim.
const SPARQL_EXPR_DECLARATION: &str = r#"
sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
  rdfs:label "SPARQL expr expression"@en ;
  rdfs:comment "The class of node expressions based on SPARQL expressions (sh:sparqlExpr)."@en ;
  rdfs:isDefinedBy sh: ;
  rdfs:subClassOf sh:NamedParameterExpression,
  sh:SPARQLExecutable ;
  sh:parameter sh:SPARQLExprExpression-prefixes,
  sh:SPARQLExprExpression-sparqlExpr .

sh:SPARQLExprExpression-prefixes a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:description "The prefixes that shall be applied before parsing the SPARQL query that gets derived from the sh:sparqlExpr expression. The object should define those prefixes using sh:declare."@en ;
  sh:name "prefixes"@en ;
  sh:nodeKind sh:BlankNodeOrIRI ;
  sh:path sh:prefixes .

sh:SPARQLExprExpression-sparqlExpr a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:datatype xsd:string ;
  sh:description "The SPARQL expression that is executed during evaluation of this node expression."@en ;
  sh:keyParameter true ;
  sh:name "SPARQL expr"@en ;
  sh:path sh:sparqlExpr .
"#;

#[test]
fn a_bodiless_non_builtin_in_the_sh_namespace_is_refused() {
    let error = load_error(BODYLESS_NOT_A_BUILTIN);
    assert!(
        error.contains("NotABuiltin") && error.contains("declares 0 sh:bodyExpression"),
        "nativeness is decided by implementation, not namespace: {error}"
    );
}

#[test]
fn the_builtin_sparql_expr_declaration_loads() {
    let shapes = load(SPARQL_EXPR_DECLARATION).expect("the W3C declaration binds natively");
    assert!(
        shapes.node_shapes.is_empty(),
        "a declaration is not a shape"
    );
}

/// Two custom named-parameter functions keyed `(ex:F key, ex:G key)`.
fn two_named_functions(f_key: &str, g_key: &str) -> String {
    format!(
        "ex:F a sh:NamedParameterExpressionFunction ;
           sh:parameter [ sh:path {f_key} ; sh:keyParameter true ] ;
           sh:bodyExpression [ shnex:arg {f_key} ] .
         ex:G a sh:NamedParameterExpressionFunction ;
           sh:parameter [ sh:path {g_key} ; sh:keyParameter true ] ;
           sh:bodyExpression [ shnex:arg {g_key} ] ."
    )
}

#[test]
fn colliding_custom_keys_are_refused_and_distinct_keys_load() {
    let error = load_error(&two_named_functions("ex:k", "ex:k"));
    assert!(error.contains("claimed by both"), "{error}");
    let linked = linked(&two_named_functions("ex:k", "ex:j"));
    assert_eq!(
        linked.by_key_parameter("http://example.org/ns#k"),
        Some("http://example.org/ns#F")
    );
    assert_eq!(
        linked.by_key_parameter("http://example.org/ns#j"),
        Some("http://example.org/ns#G")
    );
}

// ── 2. The linker: new refusals, each beside a valid neighbour ───────────────

/// A custom named-parameter function keyed by a built-in's key (or its AF
/// spelling) could never be called: the call site dispatches to the built-in.
#[test]
fn a_custom_key_that_is_a_builtin_key_is_refused() {
    for key in ["shnex:count", "sh:count"] {
        let error = load_error(&format!(
            "ex:MyCount a sh:NamedParameterExpressionFunction ;
               sh:parameter [ sh:path {key} ; sh:keyParameter true ] ;
               sh:bodyExpression [ shnex:count [ shnex:arg {key} ] ] ."
        ));
        assert!(
            error.contains(
                "The key parameters of all node expression functions (including the \
                            built-in ones from the shnex: namespace) must be disjoint."
            ),
            "{key}: {error}"
        );
    }
}

#[test]
fn a_custom_key_of_its_own_evaluates() {
    let shapes = r"
        ex:MyCount a sh:NamedParameterExpressionFunction ;
          sh:parameter [ sh:path ex:myCount ; sh:keyParameter true ] ;
          sh:bodyExpression [ shnex:count [ shnex:arg ex:myCount ] ] .
        ex:S a sh:NodeShape ; sh:targetNode ex:a, ex:b ;
          sh:expression [ sparql:equals ( [ ex:myCount [ shnex:pathValues ex:p ] ] 2 ) ] .
    ";
    let report = validate(shapes, "ex:a ex:p 1, 2 . ex:b ex:p 1 .");
    assert_eq!(
        focus_nodes(&report),
        vec!["<http://example.org/ns#b>".to_owned()],
        "ex:a has two ex:p values, ex:b one"
    );
}

/// A custom LIST-parameter function's own IRI is its call key, so a built-in key
/// there is the same clash.
#[test]
fn a_custom_list_function_named_by_a_builtin_key_is_refused() {
    let error = load_error(
        "sh:count a sh:ListParameterExpressionFunction ;
           sh:parameter [ sh:path shnex:arg0 ] ;
           sh:bodyExpression [ shnex:arg 0 ] .",
    );
    assert!(error.contains("must be disjoint"), "{error}");
    // The neighbour: the same declaration under an IRI of its own.
    let ok = linked(
        "ex:count a sh:ListParameterExpressionFunction ;
           sh:parameter [ sh:path shnex:arg0 ] ;
           sh:bodyExpression [ shnex:arg 0 ] .",
    );
    assert!(ok.custom_function("http://example.org/ns#count"));
}

#[test]
fn a_builtin_function_given_a_body_is_a_duplicate_definition() {
    let error = load_error(&format!(
        "{SPARQL_EXPR_DECLARATION}
         sh:SPARQLExprExpression sh:bodyExpression [ shnex:var \"focusNode\" ] ."
    ));
    assert!(
        error.contains("duplicate definition") && error.contains("SPARQLExprExpression"),
        "{error}"
    );
    // The bare declaration binds (see `the_builtin_sparql_expr_declaration_loads`)
    // and — the observing half — indexes nothing.
    let bare = linked(SPARQL_EXPR_DECLARATION);
    assert!(!bare.custom_function("http://www.w3.org/ns/shacl#SPARQLExprExpression"));
}

const MIN_COUNT_DECLARATION: &str = r"
sh:MinCountConstraintComponent a sh:ConstraintComponent ;
  sh:parameter sh:MinCountConstraintComponent-minCount .
sh:MinCountConstraintComponent-minCount a sh:Parameter ;
  sh:path sh:minCount ; sh:datatype xsd:integer ; sh:maxCount 1 .
";

const MIN_COUNT_SHAPE: &str = r"
ex:S a sh:NodeShape ; sh:targetNode ex:a, ex:b ;
  sh:property [ sh:path ex:p ; sh:minCount 1 ] .
";

/// The alternatives declared for `sh:MinCountConstraintComponent` below: a SELECT
/// property validator that reports EVERY focus node, and an ASK validator that calls a
/// function this engine does not have. Either, if it ran, would change the report.
const MIN_COUNT_ALTERNATIVES: &str = r#"
sh:MinCountConstraintComponent
  sh:propertyValidator [ a sh:SPARQLSelectValidator ; sh:select "SELECT $this WHERE { }" ] ;
  sh:validator ex:askAlternative .
ex:askAlternative a sh:SPARQLAskValidator ;
  sh:ask "ASK { FILTER (<http://example.org/ns#notAFunction>(?value)) }" .
"#;

/// A built-in given validators binds NATIVELY: the declared validators are
/// alternative implementations the native one supersedes (SHACL 1.2 SPARQL
/// Extensions, "Validators": a constraint uses "one of the values"). The report is the
/// native one — identical to the report without the alternatives — although the
/// SELECT alternative would flag ex:a as well and the ASK alternative would fail on
/// an unknown function.
#[test]
fn a_builtin_component_given_validators_binds_natively() {
    let data = "ex:a ex:p 1 .";
    let with = validate(
        &format!("{MIN_COUNT_DECLARATION}{MIN_COUNT_ALTERNATIVES}{MIN_COUNT_SHAPE}"),
        data,
    );
    let without = validate(&format!("{MIN_COUNT_DECLARATION}{MIN_COUNT_SHAPE}"), data);
    assert_eq!(
        focus_nodes(&with),
        vec!["<http://example.org/ns#b>".to_owned()]
    );
    assert_eq!(summary(&with), summary(&without));
    let linked = linked(&format!("{MIN_COUNT_DECLARATION}{MIN_COUNT_ALTERNATIVES}"));
    assert_eq!(linked.registered_components, Vec::<String>::new());
    let alternatives: Vec<(String, String, &str)> = linked
        .alternative_validators
        .iter()
        .map(|a| {
            (
                a.component.clone(),
                a.attachment.clone(),
                a.language.label(),
            )
        })
        .collect();
    assert_eq!(
        alternatives,
        vec![
            (
                sh_iri("MinCountConstraintComponent"),
                sh_iri("propertyValidator"),
                "sparql-select"
            ),
            (
                sh_iri("MinCountConstraintComponent"),
                sh_iri("validator"),
                "sparql-ask"
            ),
        ]
    );
}

/// Every result as (focus, path, value, component, severity, messages) — the report
/// minus the shape's blank-node label, which differs between two parses.
fn summary(report: &ValidationReport) -> Vec<String> {
    let mut out: Vec<String> = report
        .results
        .iter()
        .map(|r| {
            format!(
                "{} {:?} {:?} {} {:?} {:?}",
                r.focus_node,
                r.result_path,
                r.value,
                r.source_constraint_component,
                r.severity,
                r.messages
            )
        })
        .collect();
    out.sort();
    out
}

fn sh_iri(local: &str) -> String {
    format!("http://www.w3.org/ns/shacl#{local}")
}

/// A built-in's declaration that ALSO states a parameter the built-in does not have
/// is still refused, alternatives or not; the neighbour without the foreign
/// parameter is the test above.
#[test]
fn a_builtin_with_validators_and_a_foreign_parameter_is_a_signature_mismatch() {
    let error = load_error(&format!(
        "{MIN_COUNT_DECLARATION}{MIN_COUNT_ALTERNATIVES}
         sh:MinCountConstraintComponent sh:parameter [ sh:path ex:extra ] .
         {MIN_COUNT_SHAPE}"
    ));
    assert!(
        error.contains("signature mismatch") && error.contains("http://example.org/ns#extra"),
        "{error}"
    );
}

/// A query stated on the built-in component ITSELF, not on a validator, is a second
/// definition; the same query on a validator is an alternative and loads.
#[test]
fn a_query_on_a_builtin_component_itself_is_a_duplicate_definition() {
    let error = load_error(&format!(
        "{MIN_COUNT_DECLARATION}
         sh:MinCountConstraintComponent sh:ask \"ASK {{ }}\" .
         {MIN_COUNT_SHAPE}"
    ));
    assert!(
        error.contains("duplicate definition") && error.contains("MinCountConstraintComponent"),
        "{error}"
    );
    load(&format!(
        "{MIN_COUNT_DECLARATION}
         sh:MinCountConstraintComponent sh:validator [ a sh:SPARQLAskValidator ; sh:ask \"ASK {{ }}\" ] .
         {MIN_COUNT_SHAPE}"
    ))
    .expect("the same query on a validator is an alternative");
}

/// `sh:severity` on a built-in's declaration would state something the native
/// implementation does not honour, so it is refused. The neighbour carries every
/// annotation the linker accepts — `sh:message`, `sh:labelTemplate`, a non-validating
/// `sh:name`, and non-SHACL `rdfs:label` — and validates exactly as the bare
/// declaration does: the component's `sh:message` belongs to the SPARQL validation
/// protocol the native implementation supersedes, so the native result carries no
/// message.
#[test]
fn a_semantic_statement_on_a_builtin_declaration_is_refused() {
    let error = load_error(&format!(
        "{MIN_COUNT_DECLARATION}
         sh:MinCountConstraintComponent sh:severity sh:Warning .
         {MIN_COUNT_SHAPE}"
    ));
    assert!(
        error.contains("MinCountConstraintComponent") && error.contains("shacl#severity"),
        "{error}"
    );
    let annotated = validate(
        &format!(
            "{MIN_COUNT_DECLARATION}
             sh:MinCountConstraintComponent
               sh:message \"Fewer than {{$minCount}} values\" ;
               sh:labelTemplate \"Must have at least {{$minCount}} values\" ;
               sh:name \"min count\" ;
               rdfs:label \"Min count\" .
             {MIN_COUNT_SHAPE}"
        ),
        "ex:a ex:p 1 .",
    );
    let bare = validate(
        &format!("{MIN_COUNT_DECLARATION}{MIN_COUNT_SHAPE}"),
        "ex:a ex:p 1 .",
    );
    assert_eq!(
        focus_nodes(&annotated),
        vec!["<http://example.org/ns#b>".to_owned()]
    );
    assert_eq!(summary(&annotated), summary(&bare));
    assert_eq!(
        annotated.results[0].messages.len(),
        0,
        "{:?}",
        annotated.results
    );
}

/// An alternative must still be a well-formed validator of its attachment. A SHACL-JS
/// validator is not ("The values of sh:validator must be ASK-based validators"), nor
/// is an ASK validator under `sh:propertyValidator` ("The values of
/// sh:propertyValidator must be SELECT-based validators"), nor an ASK validator whose
/// query does not parse; each is refused. The neighbours — the SPARQL validator of the
/// right form, with a parsable query calling an unknown function — load (see
/// `a_builtin_component_given_validators_binds_natively`).
#[test]
fn an_ill_formed_alternative_on_a_builtin_is_refused() {
    let js = load_error(&format!(
        "{MIN_COUNT_DECLARATION}
         sh:MinCountConstraintComponent sh:validator [
           a sh:JSValidator ; sh:jsFunctionName \"validateMinCount\" ] .
         {MIN_COUNT_SHAPE}"
    ));
    assert!(
        js.contains("sh:JSValidator") && js.contains("SHACL JavaScript Extensions"),
        "{js}"
    );
    let wrong_form = load_error(&format!(
        "{MIN_COUNT_DECLARATION}
         sh:MinCountConstraintComponent sh:propertyValidator [
           a sh:SPARQLAskValidator ; sh:ask \"ASK {{ }}\" ] .
         {MIN_COUNT_SHAPE}"
    ));
    assert!(
        wrong_form.contains("requires SELECT validators"),
        "{wrong_form}"
    );
    let unparsable = load_error(&format!(
        "{MIN_COUNT_DECLARATION}
         sh:MinCountConstraintComponent sh:validator [
           a sh:SPARQLAskValidator ; sh:ask \"ASK {{\" ] .
         {MIN_COUNT_SHAPE}"
    ));
    assert!(unparsable.contains("unparsable query"), "{unparsable}");
}

/// A CUSTOM component with a SHACL-JS validator is refused even when no shape uses
/// it: the node is ill-formed whatever uses it. The neighbour — the same component
/// with only its SPARQL validator — loads and runs that validator.
#[test]
fn a_custom_component_with_a_javascript_validator_is_refused() {
    const COMPONENT: &str = r#"
ex:EqualsOne a sh:ConstraintComponent ;
  sh:parameter [ sh:path ex:one ] ;
  sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (?value = 1) }" ] .
"#;
    let error = load_error(&format!(
        "{COMPONENT}
         ex:EqualsOne sh:validator [ a sh:JSValidator ; sh:jsFunctionName \"equalsOne\" ] ."
    ));
    assert!(error.contains("sh:JSValidator"), "{error}");
    let report = validate(
        &format!(
            "{COMPONENT}
             ex:S a sh:NodeShape ; sh:targetNode 1, 2 ; ex:one true ."
        ),
        "",
    );
    assert_eq!(
        focus_nodes(&report),
        vec!["\"2\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned()]
    );
}

/// The bare declaration binds, and sh:minCount keeps its NATIVE semantics: ex:b
/// (no ex:p) violates, ex:a conforms — a row that differs from the refusal's.
#[test]
fn the_bare_builtin_component_declaration_binds_natively() {
    let report = validate(
        &format!("{MIN_COUNT_DECLARATION}{MIN_COUNT_SHAPE}"),
        "ex:a ex:p 1 .",
    );
    assert_eq!(
        focus_nodes(&report),
        vec!["<http://example.org/ns#b>".to_owned()]
    );
    assert_eq!(
        linked(MIN_COUNT_DECLARATION).registered_components,
        Vec::<String>::new(),
        "a built-in component's declaration registers no custom component"
    );
}

#[test]
fn a_list_builtin_declared_as_named_is_a_kind_mismatch() {
    let error = load_error("sparql:abs a sh:NamedParameterExpressionFunction .");
    assert!(
        error.contains("kind mismatch") && error.contains("sparql#abs"),
        "{error}"
    );
}

#[test]
fn a_list_builtin_declared_as_list_binds_and_evaluates() {
    let shapes = r"
        sparql:abs a sh:ListParameterExpressionFunction .
        ex:S a sh:NodeShape ; sh:targetNode ex:a, ex:b ;
          sh:expression [ sparql:equals ( [ sparql:abs ( [ sh:path ex:n ] ) ] 42 ) ] .
    ";
    let report = validate(shapes, "ex:a ex:n -42 . ex:b ex:n -41 .");
    assert_eq!(
        focus_nodes(&report),
        vec!["<http://example.org/ns#b>".to_owned()]
    );
    let bound = linked("sparql:abs a sh:ListParameterExpressionFunction .");
    assert_eq!(bound.custom_functions, Vec::<String>::new());
    assert_eq!(bound.native_list_functions, vec![format!("{SPARQL_NS}abs")]);
}

#[test]
fn a_builtin_component_declared_as_a_function_is_a_kind_mismatch() {
    let error = load_error(
        "sh:MinCountConstraintComponent a sh:NamedParameterExpressionFunction ;
           sh:parameter [ sh:path ex:k ; sh:keyParameter true ] .",
    );
    assert!(error.contains("kind mismatch"), "{error}");
    // Neighbour: the component declared as a component binds.
    load(MIN_COUNT_DECLARATION).expect("the component declaration binds");
}

#[test]
fn a_builtin_function_declared_as_a_component_is_a_kind_mismatch() {
    let error = load_error("shnex:CountExpression a sh:ConstraintComponent .");
    assert!(error.contains("kind mismatch"), "{error}");
    // Neighbour: declared under its own class, it binds.
    load(
        "shnex:CountExpression a sh:NamedParameterExpressionFunction ;
           sh:parameter [ sh:path shnex:count ; sh:keyParameter true ] .",
    )
    .expect("the function declaration binds");
}

#[test]
fn a_builtin_declaration_stating_a_foreign_parameter_is_a_signature_mismatch() {
    let error = load_error(&format!(
        "{SPARQL_EXPR_DECLARATION}
         sh:SPARQLExprExpression sh:parameter [ sh:path ex:extra ] ."
    ));
    assert!(error.contains("signature mismatch"), "{error}");
    // Neighbour: a declaration stating FEWER of the built-in's parameters is an
    // incomplete description, not a contradiction, and binds.
    load(
        "sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
           sh:parameter [ sh:path sh:sparqlExpr ; sh:keyParameter true ] .",
    )
    .expect("a subset of the signature binds");
}

#[test]
fn a_builtin_declaration_keying_a_non_key_parameter_is_a_signature_mismatch() {
    let error = load_error(
        "sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
           sh:parameter [ sh:path sh:prefixes ; sh:keyParameter true ] .",
    );
    assert!(error.contains("signature mismatch"), "{error}");
    // Neighbour: the W3C spelling states sh:prefixes with no sh:keyParameter.
    load(SPARQL_EXPR_DECLARATION).expect("the W3C declaration binds");
}

#[test]
fn a_builtin_redefined_as_a_sparql_function_is_a_duplicate_definition() {
    let error = load_error(
        "sparql:abs a sh:SPARQLFunction ;
           sh:parameter [ sh:path ex:x ] ;
           sh:select \"SELECT (ABS($x) AS ?r) WHERE {}\" .",
    );
    assert!(error.contains("duplicate definition"), "{error}");
    // Neighbour: the same SPARQL function under an IRI of its own loads.
    load(
        "ex:abs a sh:SPARQLFunction ;
           sh:parameter [ sh:path ex:x ] ;
           sh:select \"SELECT (ABS($x) AS ?r) WHERE {}\" .",
    )
    .expect("a SPARQL function under its own IRI loads");
}

// ── sh:uniqueValuesFor, the last declared component, is native ─────────────

const UNIQUE_VALUES_FOR_DECLARATION: &str = r"
sh:UniqueValuesForConstraintComponent a sh:ConstraintComponent ;
  sh:parameter sh:UniqueValuesForConstraintComponent-uniqueValuesFor .
sh:UniqueValuesForConstraintComponent-uniqueValuesFor a sh:Parameter ;
  sh:path sh:uniqueValuesFor ; sh:nodeKind sh:BlankNodeOrIRI .
";

const UNIQUE_VALUES_FOR_SHAPE: &str = r"
ex:S a sh:NodeShape ; sh:targetClass ex:Record ; sh:uniqueValuesFor ex:id .
";

/// The bare W3C declaration of `sh:UniqueValuesForConstraintComponent` binds to
/// the native evaluator and registers nothing: a shape using the parameter is
/// evaluated natively with or without the declaration, and the two data rows
/// below differ, so the parameter is honoured rather than dropped.
#[test]
fn the_unique_values_for_declaration_binds_natively() {
    for shapes in [
        UNIQUE_VALUES_FOR_SHAPE.to_owned(),
        format!("{UNIQUE_VALUES_FOR_DECLARATION}{UNIQUE_VALUES_FOR_SHAPE}"),
    ] {
        let clash = validate(
            &shapes,
            "ex:a a ex:Record ; ex:id \"1\" . ex:b a ex:Record ; ex:id \"1\" .",
        );
        assert_eq!(
            focus_nodes(&clash),
            vec![
                "<http://example.org/ns#a>".to_owned(),
                "<http://example.org/ns#b>".to_owned()
            ]
        );
        let distinct = validate(
            &shapes,
            "ex:a a ex:Record ; ex:id \"1\" . ex:b a ex:Record ; ex:id \"2\" .",
        );
        assert!(distinct.conforms, "{:?}", distinct.results);
    }
    assert_eq!(
        linked(UNIQUE_VALUES_FOR_DECLARATION).registered_components,
        Vec::<String>::new()
    );
}

/// A shapes graph that supplies its own validator for the native component binds
/// the component natively: the validator's `FILTER (true)` would pass every value,
/// and the native `sh:uniqueValuesFor` still reports the clash.
#[test]
fn a_validator_on_unique_values_for_is_a_superseded_alternative() {
    let shapes = format!(
        r#"{UNIQUE_VALUES_FOR_DECLARATION}
        sh:UniqueValuesForConstraintComponent sh:validator [
          a sh:SPARQLAskValidator ;
          sh:ask """ASK {{ FILTER (true) }}"""
        ] .
        {UNIQUE_VALUES_FOR_SHAPE}"#
    );
    let clash = validate(
        &shapes,
        "ex:a a ex:Record ; ex:id \"1\" . ex:b a ex:Record ; ex:id \"1\" .",
    );
    assert_eq!(
        focus_nodes(&clash),
        vec![
            "<http://example.org/ns#a>".to_owned(),
            "<http://example.org/ns#b>".to_owned()
        ]
    );
}

/// A component this engine evaluates natively is a native IRI: its bare W3C
/// declaration binds, and a validator the shapes graph also supplies for it is an
/// alternative the native `sh:singleLine` supersedes — the alternative's
/// `FILTER (true)` would pass the two-line value the native component reports.
#[test]
fn a_validator_on_a_native_component_is_a_superseded_alternative() {
    const SINGLE_LINE_DECLARATION: &str = r"
sh:SingleLineConstraintComponent a sh:ConstraintComponent ;
  sh:parameter sh:SingleLineConstraintComponent-singleLine .
sh:SingleLineConstraintComponent-singleLine a sh:Parameter ;
  sh:path sh:singleLine ; sh:datatype xsd:boolean ; sh:maxCount 1 .
";
    const SINGLE_LINE_SHAPE: &str = r"
ex:S a sh:NodeShape ; sh:targetNode ex:a ;
  sh:property [ sh:path ex:text ; sh:singleLine true ] .
";
    let shapes = format!(
        r#"{SINGLE_LINE_DECLARATION}
        sh:SingleLineConstraintComponent sh:validator [
          a sh:SPARQLAskValidator ;
          sh:ask """ASK {{ FILTER (true) }}"""
        ] .
        {SINGLE_LINE_SHAPE}"#
    );
    assert_eq!(
        focus_nodes(&validate(&shapes, "ex:a ex:text \"one\\ntwo\" .")),
        vec!["<http://example.org/ns#a>".to_owned()]
    );
    assert!(validate(&shapes, "ex:a ex:text \"one two\" .").conforms);
    assert_eq!(linked(&shapes).registered_components, Vec::<String>::new());
    assert_eq!(linked(&shapes).alternative_validators.len(), 1);
}

// ── 5. The index stays empty ─────────────────────────────────────────────────

/// The whole merged vocabulary indexes no custom function, claims no key
/// parameter, registers no component — and binds exactly the built-in
/// list-parameter functions it declares.
#[test]
fn the_merged_vocabulary_indexes_and_registers_nothing() {
    let linked = __linked_declarations(&vocabulary_dataset()).expect("the vocabulary links");
    assert_eq!(
        linked.custom_functions,
        Vec::<String>::new(),
        "{:?}",
        linked.custom_functions
    );
    assert!(!linked.custom_function("http://www.w3.org/ns/shacl#SPARQLExprExpression"));
    assert_eq!(
        linked.by_key_parameter("http://www.w3.org/ns/shacl#sparqlExpr"),
        None
    );
    assert_eq!(linked.custom_key_parameters, Vec::<(String, String)>::new());
    assert_eq!(
        linked.registered_components,
        Vec::<String>::new(),
        "{:?}",
        linked.registered_components
    );
    let vocab = declared().expect("vocabularies read");
    let list: Vec<String> = vocab
        .functions
        .iter()
        .filter(|(_, f)| f.class == FunctionClass::ListParameter)
        .map(|(iri, _)| iri.clone())
        .collect();
    assert_eq!(linked.native_list_functions, list);
    assert_eq!(list.len(), 78, "shnex:conformsToShape + 77 sparql:");
}

#[test]
fn the_sparql_expr_declaration_indexes_nothing() {
    let linked = linked(SPARQL_EXPR_DECLARATION);
    assert!(!linked.custom_function("http://www.w3.org/ns/shacl#SPARQLExprExpression"));
    assert_eq!(
        linked.by_key_parameter("http://www.w3.org/ns/shacl#sparqlExpr"),
        None
    );
    assert_eq!(linked.registered_components, Vec::<String>::new());
}

/// The whole merged vocabulary also loads as a shapes graph, with no shape: its
/// `sh:Parameter` nodes stay inert.
#[test]
fn the_merged_vocabulary_loads_with_no_shape() {
    let shapes = purrdf_shapes::shapes::from_dataset(&vocabulary_dataset())
        .expect("the merged vocabulary loads");
    assert!(shapes.node_shapes.is_empty());
}

// ── 3. The function-resolution report ────────────────────────────────────────

#[test]
fn every_call_site_names_what_it_bound_to() {
    let shapes = load(
        r#"
        ex:f a sh:ListParameterExpressionFunction ;
          sh:parameter [ sh:path shnex:arg0 ] ;
          sh:bodyExpression [ sparql:abs ( [ shnex:arg 0 ] ) ] .
        ex:g a sh:SPARQLFunction ;
          sh:parameter [ sh:path ex:x ] ;
          sh:select "SELECT (ABS($x) AS ?r) WHERE {}" .
        ex:Target a sh:NodeShape ; sh:targetNode ex:a .
        ex:S a sh:NodeShape ; sh:targetNode ex:a ;
          sh:expression [ sparql:equals ( [ ex:f ( 1 ) ] 1 ) ] ;
          sh:expression [ sparql:equals ( [ ex:g ( 1 ) ] 1 ) ] ;
          sh:expression [ sparql:equals ( [ ex:h ( 1 ) ] 1 ) ] ;
          sh:expression [ sh:sparqlExpr "true" ] ;
          sh:expression [ shnex:conformsToShape ( sh:this ex:Target ) ] .
        "#,
    )
    .expect("shapes load");
    let report = shapes.function_resolution();
    let expect = [
        (format!("{SPARQL_NS}equals"), FunctionBinding::Native),
        (format!("{SPARQL_NS}abs"), FunctionBinding::Native),
        (
            "http://example.org/ns#f".to_owned(),
            FunctionBinding::Custom,
        ),
        (
            "http://example.org/ns#g".to_owned(),
            FunctionBinding::SparqlRegistered,
        ),
        (
            "http://example.org/ns#h".to_owned(),
            FunctionBinding::HostExtension,
        ),
        (
            "http://www.w3.org/ns/shacl#SPARQLExprExpression".to_owned(),
            FunctionBinding::Native,
        ),
        (
            "http://www.w3.org/ns/shacl-node-expr#conformsToShape".to_owned(),
            FunctionBinding::Native,
        ),
    ];
    for (function, binding) in &expect {
        assert_eq!(
            report.bindings_of(function),
            BTreeSet::from([*binding]),
            "{function}"
        );
    }
    // The call inside ex:f's BODY is reported under the calling site.
    assert!(
        report
            .sites()
            .any(|site| site.function == format!("{SPARQL_NS}abs")
                && site.owner.contains("via <http://example.org/ns#f>")),
        "the body of a custom function is walked"
    );
    assert!(
        !shapes.node_shapes.iter().any(|s| s
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::Component { .. }))),
        "no component instance was made from a function declaration"
    );
}

// ── 6. Alternatives are reported, never findings ─────────────────────────────

fn lint_report(shapes_ttl: &str) -> purrdf_shapes::lint::LintReport {
    let document =
        purrdf_shapes::text_ingest::parse_turtle_document(&format!("{PREFIXES}{shapes_ttl}"), None)
            .expect("parses");
    purrdf_shapes::lint::lint(
        &document.dataset,
        &document.prefixes,
        None,
        None,
        &purrdf_shapes::ShapesImports::new(),
    )
    .expect("shacl-shacl.ttl loads and validates")
}

/// `shapes lint` names every alternative a built-in's declaration carries, with the
/// native implementation superseding it, and counts none as a finding: the report
/// with the alternatives has exactly the findings of the report without them. A
/// graph the loader refuses reports `validators unavailable`.
#[test]
fn lint_reports_superseded_alternatives_without_findings() {
    let with = lint_report(&format!(
        "{MIN_COUNT_DECLARATION}{MIN_COUNT_ALTERNATIVES}{MIN_COUNT_SHAPE}"
    ));
    let without = lint_report(&format!("{MIN_COUNT_DECLARATION}{MIN_COUNT_SHAPE}"));
    assert_eq!(with.load_error(), None, "{}", with.render());
    assert_eq!(with.findings(), without.findings());
    assert_eq!(with.alternative_validators().map(<[_]>::len), Some(2));
    assert_eq!(without.alternative_validators().map(<[_]>::len), Some(0));
    let text = with.render();
    assert!(
        text.contains(
            "validators 2\n\
             alternative <http://www.w3.org/ns/shacl#MinCountConstraintComponent> \
             <http://www.w3.org/ns/shacl#propertyValidator> _:"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "alternative <http://www.w3.org/ns/shacl#MinCountConstraintComponent> \
             <http://www.w3.org/ns/shacl#validator> <http://example.org/ns#askAlternative> \
             sparql-ask superseded-by-native\n"
        ),
        "{text}"
    );
    assert!(without.render().contains("validators 0\n"));
    let refused = lint_report(&format!(
        "{MIN_COUNT_DECLARATION}
         sh:MinCountConstraintComponent sh:severity sh:Warning ."
    ));
    assert!(refused.load_error().is_some());
    assert_eq!(refused.alternative_validators(), None);
    assert!(refused.render().contains("validators unavailable\n"));
}

/// The neighbour of the alternative that calls an unknown function: a SELECTED
/// validator calling one is a hard error — at validation, where the host's function
/// registry is in scope (a shapes graph is parsed once and validated under whatever
/// registry each caller installs, so the load has none to consult) — never a silently
/// false FILTER. The superseded alternative in
/// `a_builtin_component_given_validators_binds_natively` calls the same kind of
/// function and validates, because it never runs.
#[test]
fn a_selected_validator_calling_an_unknown_function_fails_validation() {
    let shapes = load(
        r#"
ex:C a sh:ConstraintComponent ;
  sh:parameter [ sh:path ex:c ] ;
  sh:validator [ a sh:SPARQLAskValidator ;
                 sh:ask "ASK { FILTER (<http://example.org/ns#notAFunction>(?value)) }" ] .
ex:S a sh:NodeShape ; sh:targetNode ex:a ; ex:c true .
"#,
    )
    .expect("the validator's query is well-formed");
    let error = validate_dataset_with_shapes_graph(&data_of(""), &shapes, None)
        .expect_err("the selected validator cannot run");
    assert!(
        error.contains("http://example.org/ns#notAFunction"),
        "{error}"
    );
}

// ── 7. A component that is also a shape ──────────────────────────────────────

/// A node may be both a custom constraint component and a shape (TOSH's
/// `tosh:MemberShapeConstraintComponent` carries `sh:targetClass`). Its `sh:parameter`
/// and `sh:validator` are read by the component registry, so they are not silently
/// ignored and the shape is not refused for carrying them — and both roles run: the
/// shape reports the `ex:Thing` without a label, the component reports the empty
/// string.
#[test]
fn a_component_that_is_also_a_shape_plays_both_roles() {
    let shapes = r#"
ex:NonEmpty a sh:ConstraintComponent ;
  sh:parameter [ sh:path ex:nonEmpty ] ;
  sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK { FILTER (STRLEN(STR(?value)) > 0) }" ] ;
  sh:targetClass ex:Thing ;
  sh:property [ sh:path ex:label ; sh:minCount 1 ] .
ex:S a sh:NodeShape ; sh:targetNode "", "x" ; ex:nonEmpty true .
"#;
    let report = validate(
        shapes,
        "ex:t a ex:Thing . ex:u a ex:Thing ; ex:label \"u\" .",
    );
    let mut seen: Vec<(String, String)> = report
        .results
        .iter()
        .map(|r| {
            (
                r.focus_node.to_string(),
                r.source_constraint_component.as_str().to_owned(),
            )
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![
            (
                "\"\"".to_owned(),
                "http://example.org/ns#NonEmpty".to_owned()
            ),
            (
                "<http://example.org/ns#t>".to_owned(),
                sh_iri("MinCountConstraintComponent")
            ),
        ]
    );
}

/// The neighbour: a shape that is NOT a constraint component carrying `sh:validator`
/// is still refused — nothing would read the validator there.
#[test]
fn a_validator_on_a_plain_shape_is_refused() {
    let error = load_error(
        r#"ex:S a sh:NodeShape ; sh:targetNode ex:a ;
             sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK { }" ] ."#,
    );
    assert!(
        error.contains("shacl#validator") && error.contains("not a property of a shape"),
        "{error}"
    );
}
