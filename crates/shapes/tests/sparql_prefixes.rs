// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `PREFIX` header of a SHACL-SPARQL query, against SHACL 1.2 SPARQL
//! Extensions, "Prefix Declarations for SPARQL Queries":
//!
//! > A SHACL processor collects a set of prefix mappings as the union of all
//! > individual prefix mappings that are values of the SPARQL property path
//! > `sh:prefixes/(^owl:versionIRI?/owl:imports)*/sh:declare` of the SPARQL-based
//! > constraint or validator. [...] If such a collection of prefix declarations
//! > contains multiple different namespaces for the same value of `sh:prefix`, then
//! > the shapes graph is ill-formed. (Note that SHACL processors MAY ignore prefix
//! > declarations that are never reached). If a SPARQL query has no value for
//! > `sh:prefixes` then the system will use those prefix declarations from the
//! > shapes graph that are values of `sh:declare` at a SHACL instance of
//! > `owl:Ontology`, `sh:DataGraph`, `sh:ShapesGraph`, or `sh:RulesGraph`.
//!
//! Every fixture validates one focus node whose `ex:property` is
//! `<http://example.org/target#Value>` with the query
//! `FILTER (?value = t:Value)`, so the OBSERVABLE is which namespace `t:` was bound
//! to: `target#` reports exactly one violation, and every other binding reports
//! none. Each treatment sits beside a control whose report differs from it.

use purrdf_shapes::engine::{parse_shapes, validate_graphs};
use purrdf_shapes::{ShapesError, ShapesImportError};

const PREFIXES: &str = "
@prefix ex:   <http://example.org/ns#> .
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
";

const DATA: &str = "<http://example.org/ns#Focus> <http://example.org/ns#property> <http://example.org/target#Value> .\n";

const TARGET: &str = "http://example.org/target#";
const OTHER: &str = "http://example.org/other#";

/// The shape: one `sh:sparql` constraint `ex:C1` whose query names `t:Value`,
/// with `c1_extra` added to the constraint node.
fn shape(c1_extra: &str) -> String {
    format!(
        r#"
ex:Shape a sh:NodeShape ;
    sh:targetNode ex:Focus ;
    sh:sparql ex:C1 .
ex:C1 {c1_extra}
    sh:select """
        SELECT $this ?value WHERE {{
            $this ex:property ?value .
            FILTER (?value = t:Value)
        }}
    """ .
"#
    )
}

/// A `sh:declare` object binding `t` to `namespace`.
fn declare_t(namespace: &str) -> String {
    format!(r#"[ sh:prefix "t" ; sh:namespace "{namespace}"^^xsd:anyURI ]"#)
}

/// The rendered `sh:value` of every result of validating [`DATA`] against `shapes`.
fn violations(shapes: &str) -> Vec<String> {
    let report = validate_graphs(DATA, shapes, None).expect("the shapes graph loads");
    let values: Vec<String> = report
        .results
        .iter()
        .map(|result| {
            result
                .value
                .as_ref()
                .map_or_else(String::new, ToString::to_string)
        })
        .collect();
    assert_eq!(report.conforms, values.is_empty());
    values
}

/// Exactly the one violation a `t:` bound to `target#` reports.
fn target_violation() -> Vec<String> {
    vec![format!("<{TARGET}Value>")]
}

fn load_error(shapes: &str) -> String {
    match parse_shapes(shapes, None) {
        Ok(_) => panic!("the shapes graph must be refused:\n{shapes}"),
        Err(error) => error.to_string(),
    }
}

// ── A PREFIX inside a query literal stays in that query ─────────────────────────

/// The regression: a `PREFIX` line inside ONE constraint's query literal was read
/// as a document prefix by a line scan of the source text, and prepended to EVERY
/// other query — so `ex:C1`, which declares `t:` nowhere, silently resolved it to
/// the other constraint's namespace. It is now an undeclared prefix in `ex:C1`,
/// and the shapes graph is refused, as SHACL-SPARQL requires of a query that does
/// not parse.
#[test]
fn a_prefix_in_one_query_literal_does_not_reach_another_query() {
    let shapes = format!(
        r#"{PREFIXES}
{}
ex:Shape sh:sparql ex:C2 .
ex:C2 sh:select """
PREFIX t: <{TARGET}>
SELECT $this ?value WHERE {{ $this ex:property ?value . FILTER (?value = t:Nothing) }}
""" .
"#,
        shape("")
    );
    let error = load_error(&shapes);
    assert!(
        error.contains("unparsable sh:select") && error.contains("undeclared prefix \"t\""),
        "ex:C1 names an undeclared prefix: {error}"
    );

    // The valid neighbour: ex:C1 declares `t:` itself, and ex:C2's own prologue binds
    // `t:` to a namespace that matches nothing — so the one violation is ex:C1's, and
    // ex:C2's prologue binding reached no query but its own.
    let shapes = format!(
        r#"{PREFIXES}
{}
ex:Decls sh:declare {} .
ex:Shape sh:sparql ex:C2 .
ex:C2 sh:select """
PREFIX t: <{OTHER}>
SELECT $this ?value WHERE {{ $this ex:property ?value . FILTER (?value = t:Value) }}
""" .
"#,
        shape("sh:prefixes ex:Decls ;"),
        declare_t(TARGET),
    );
    assert_eq!(violations(&shapes), target_violation());
}

// ── Implicit declarations ────────────────────────────────────────────────────────

/// A query with no `sh:prefixes` takes the `sh:declare` values of a SHACL instance
/// of `sh:ShapesGraph`, and they outrank the document's own `@prefix` fallback: the
/// document binds `t:` to `other#`, the declaration to `target#`, and the spec's
/// binding is the one reported. The control — the same document with no
/// declaration — resolves `t:` through the fallback and reports nothing.
#[test]
fn a_shapes_graph_declaration_supplies_a_query_without_prefixes() {
    let control = format!("{PREFIXES}@prefix t: <{OTHER}> .\n{}", shape(""));
    assert_eq!(violations(&control), Vec::<String>::new());

    for holder in [
        "ex:Graph a sh:ShapesGraph .",
        "ex:Graph a owl:Ontology .",
        "ex:Graph a sh:DataGraph .",
        "ex:Graph a sh:RulesGraph .",
        "ex:Graph a ex:GraphClass . ex:GraphClass rdfs:subClassOf sh:ShapesGraph .",
    ] {
        let shapes = format!(
            "{control}\n{holder}\nex:Graph sh:declare {} .\n",
            declare_t(TARGET)
        );
        assert_eq!(violations(&shapes), target_violation(), "{holder}");
    }

    // A node that is not a SHACL instance of any of the four classes supplies
    // nothing, so the fallback stands.
    let shapes = format!(
        "{control}\nex:Graph a rdfs:Resource ; sh:declare {} .\n",
        declare_t(TARGET)
    );
    assert_eq!(violations(&shapes), Vec::<String>::new());
}

/// With a value for `sh:prefixes`, the implicit declarations are not used: the
/// query's collection is what `sh:prefixes` reaches, which does not bind `t:`.
#[test]
fn a_query_with_prefixes_does_not_use_the_implicit_declarations() {
    let shapes = format!(
        "{PREFIXES}{}\nex:Graph a sh:ShapesGraph ; sh:declare {} .\n\
         ex:Decls sh:declare [ sh:prefix \"ex\" ; sh:namespace \"http://example.org/ns#\" ] .\n",
        shape("sh:prefixes ex:Decls ;"),
        declare_t(TARGET)
    );
    let error = load_error(&shapes);
    assert!(error.contains("undeclared prefix \"t\""), "{error}");
}

// ── The sh:prefixes path ─────────────────────────────────────────────────────────

/// `sh:prefixes/(^owl:versionIRI?/owl:imports)*/sh:declare`: the version IRI a query
/// names is navigated back to its graph, whose imports are followed.
#[test]
fn the_prefixes_path_follows_version_iris_and_imports() {
    let graph = |headers: &str| {
        format!(
            "{PREFIXES}{}\nex:G owl:versionIRI ex:V1 ; owl:imports ex:Q .\n\
             ex:Q owl:imports ex:R .\nex:R sh:declare {} .\n{headers}",
            shape("sh:prefixes ex:V1 ;"),
            declare_t(TARGET)
        )
    };
    // `ex:R` is a node this document describes with `sh:declare`, so the import of it is
    // in hand; `ex:Q` declares no prefix, and is in hand by its ontology header.
    assert_eq!(
        violations(&graph("ex:Q a owl:Ontology .\n")),
        target_violation()
    );
    // The neighbour: without `ex:Q`'s header the import of it names an ontology nothing in
    // hand declares, and the shapes graph is refused rather than read without it. `ex:R` is
    // not named: its `sh:declare` is what resolves it.
    let Err(ShapesError::Imports(ShapesImportError::Unresolved { iris })) =
        parse_shapes(&graph(""), None)
    else {
        panic!("an import of an ontology the shapes graph does not hold is refused");
    };
    assert_eq!(iris, ["http://example.org/ns#Q"]);
}

// ── Conflicts ────────────────────────────────────────────────────────────────────

/// Two different namespaces for `t` among the declarations a query reaches make
/// the shapes graph ill-formed — through `sh:prefixes` and its imports, and among
/// the implicit declarations alike. The same binding declared twice is one mapping.
#[test]
fn a_conflicting_prefix_is_refused_and_an_agreeing_one_loads() {
    let explicit = |second: &str| {
        format!(
            "{PREFIXES}{}\nex:P sh:declare {} ; owl:imports ex:Q .\nex:Q sh:declare {} .\n",
            shape("sh:prefixes ex:P ;"),
            declare_t(TARGET),
            declare_t(second)
        )
    };
    let implicit = |second: &str| {
        format!(
            "{PREFIXES}{}\nex:G1 a sh:ShapesGraph ; sh:declare {} .\n\
             ex:G2 a owl:Ontology ; sh:declare {} .\n",
            shape(""),
            declare_t(TARGET),
            declare_t(second)
        )
    };
    for build in [&explicit as &dyn Fn(&str) -> String, &implicit] {
        let error = load_error(&build(OTHER));
        assert!(
            error.contains("ill-formed")
                && error.contains("\"t\"")
                && error.contains(&format!("<{TARGET}>"))
                && error.contains(&format!("<{OTHER}>")),
            "the refusal names the prefix and both namespaces: {error}"
        );
        assert_eq!(violations(&build(TARGET)), target_violation());
    }
}

/// Only REACHED declarations are checked: a conflicting declaration no query reaches
/// — on a node that is not a graph, or among implicit declarations when every query
/// has `sh:prefixes` — leaves the shapes graph loadable.
#[test]
fn an_unreached_conflict_is_not_read() {
    let shapes = format!(
        "{PREFIXES}{}\nex:G a sh:ShapesGraph ; sh:declare {} .\n\
         ex:Elsewhere a rdfs:Resource ; sh:declare {} .\n",
        shape(""),
        declare_t(TARGET),
        declare_t(OTHER)
    );
    assert_eq!(violations(&shapes), target_violation());

    let shapes = format!(
        "{PREFIXES}{}\nex:P sh:declare {} .\n\
         ex:G1 a sh:ShapesGraph ; sh:declare {} .\nex:G2 a sh:ShapesGraph ; sh:declare {} .\n",
        shape("sh:prefixes ex:P ;"),
        declare_t(TARGET),
        declare_t(TARGET),
        declare_t(OTHER)
    );
    assert_eq!(violations(&shapes), target_violation());
}

/// A reached `sh:declare` value that is not a prefix declaration is refused rather
/// than skipped; the well-formed neighbour of each is [`declare_t`] itself, which
/// every test above loads.
#[test]
fn a_malformed_reached_declaration_is_refused() {
    for (declaration, expected) in [
        (r#"[ sh:prefix "t" ]"#, "0 sh:namespace values"),
        (
            &*format!(r#"[ sh:prefix "t", "u" ; sh:namespace "{TARGET}" ]"#),
            "2 sh:prefix values",
        ),
        (
            &*format!(r#"[ sh:prefix ex:t ; sh:namespace "{TARGET}" ]"#),
            "literals of datatype xsd:string",
        ),
        (
            r#"[ sh:prefix "t" ; sh:namespace 42 ]"#,
            "xsd:anyURI or xsd:string",
        ),
        (r#""t""#, "is not a prefix declaration"),
    ] {
        let shapes = format!(
            "{PREFIXES}{}\nex:P sh:declare {declaration} .\n",
            shape("sh:prefixes ex:P ;")
        );
        let error = load_error(&shapes);
        assert!(
            error.contains("ill-formed") && error.contains(expected),
            "{declaration}: {error}"
        );
    }
    let error = load_error(&format!("{PREFIXES}{}", shape(r#"sh:prefixes "ex" ;"#)));
    assert!(error.contains("not an IRI or a blank node"), "{error}");
}

// ── The document fallback ───────────────────────────────────────────────────────

/// A document that redeclares `t:` falls back to its FINAL binding — the one map
/// the codec reports for the whole document. The control reverses the two
/// directives and reports nothing.
#[test]
fn a_redeclared_document_prefix_falls_back_to_its_last_binding() {
    let redeclared = |first: &str, last: &str| {
        format!(
            "{PREFIXES}@prefix t: <{first}> .\n{}\n@prefix t: <{last}> .\nex:Other ex:p t:x .\n",
            shape("")
        )
    };
    assert_eq!(violations(&redeclared(OTHER, TARGET)), target_violation());
    assert_eq!(violations(&redeclared(TARGET, OTHER)), Vec::<String>::new());
}
