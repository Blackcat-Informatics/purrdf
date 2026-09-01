// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unit tests for query shape maps: parsing the compact `{FOCUS p _}` /
//! `{_ p FOCUS}` syntax, resolving selectors against the data graph
//! (with dedup + deterministic order), and validating the expansion.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_shex::{
    ConformanceStatus, NodeSelector, ShapeSelector, ShexError, ValidationOptions, parse_shape_map,
    parse_shexc, resolve_imports, resolve_shape_map, validate, validate_shape_map,
};

const P1: &str = "http://a.example/p1";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const CLASS: &str = "http://a.example/C";

fn iri(s: &str) -> TermValue {
    TermValue::iri(s)
}

/// s1 <p1> o1 ; s2 <p1> o2 ; s1 <p1> o3 ; s3 a C
fn data() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let mut arc = |s: &str, p: &str, o: &str| {
        let s = b.intern_iri(s);
        let p = b.intern_iri(p);
        let o = b.intern_iri(o);
        b.push_quad(s, p, o, None);
    };
    arc("http://a.example/s1", P1, "http://a.example/o1");
    arc("http://a.example/s2", P1, "http://a.example/o2");
    arc("http://a.example/s1", P1, "http://a.example/o3");
    arc("http://a.example/s3", RDF_TYPE, CLASS);
    b.freeze().expect("freeze")
}

fn nodes(map_src: &str, data: &RdfDataset) -> Vec<TermValue> {
    let map = parse_shape_map(map_src, None).expect("shape map parses");
    resolve_shape_map(&map, data)
        .into_iter()
        .map(|(node, _)| node)
        .collect()
}

#[test]
fn parses_query_and_explicit_forms() {
    let map = parse_shape_map(
        &format!("<http://a.example/s1>@<http://a.example/S>, {{FOCUS <{P1}> _}}@START"),
        None,
    )
    .expect("parses");
    assert_eq!(map.0.len(), 2);
    assert_eq!(
        map.0[0].node,
        NodeSelector::Node(iri("http://a.example/s1"))
    );
    assert_eq!(
        map.0[0].shape,
        ShapeSelector::Label("http://a.example/S".to_owned())
    );
    assert_eq!(
        map.0[1].node,
        NodeSelector::SubjectOf {
            predicate: P1.to_owned(),
            object: None,
        }
    );
    assert_eq!(map.0[1].shape, ShapeSelector::Start);
}

#[test]
fn focus_subject_selects_subjects_deduped() {
    let data = data();
    // s1 appears twice in the data but must be selected once.
    let got = nodes(&format!("{{FOCUS <{P1}> _}}@<http://a.example/S>"), &data);
    assert_eq!(
        got,
        vec![iri("http://a.example/s1"), iri("http://a.example/s2")]
    );
}

#[test]
fn focus_object_selects_objects() {
    let data = data();
    let got = nodes(&format!("{{_ <{P1}> FOCUS}}@<http://a.example/S>"), &data);
    assert_eq!(
        got,
        vec![
            iri("http://a.example/o1"),
            iri("http://a.example/o2"),
            iri("http://a.example/o3"),
        ]
    );
}

#[test]
fn focus_typed_subjects() {
    let data = data();
    let got = nodes(
        &format!("{{FOCUS a <{CLASS}>}}@<http://a.example/S>"),
        &data,
    );
    assert_eq!(got, vec![iri("http://a.example/s3")]);
}

#[test]
fn anchored_subject_selects_its_objects() {
    let data = data();
    let got = nodes(
        &format!("{{<http://a.example/s1> <{P1}> FOCUS}}@<http://a.example/S>"),
        &data,
    );
    assert_eq!(
        got,
        vec![iri("http://a.example/o1"), iri("http://a.example/o3")]
    );
}

#[test]
fn unknown_predicate_selects_nothing() {
    let data = data();
    let got = nodes(
        "{FOCUS <http://a.example/absent> _}@<http://a.example/S>",
        &data,
    );
    assert!(got.is_empty());
}

// ── RDF-1.2 quoted-triple term parsing ──────────────────────────────────────

fn parse_node(src: &str) -> TermValue {
    let map = parse_shape_map(&format!("{src}@START"), None).expect("term parses");
    match &map.0[0].node {
        NodeSelector::Node(value) => value.clone(),
        other => panic!("expected a concrete node, got {other:?}"),
    }
}

#[test]
fn parses_quoted_triple_term() {
    let got = parse_node("<< <http://a.example/s> <http://a.example/p> <http://a.example/o> >>");
    assert_eq!(
        got,
        TermValue::Triple {
            s: Box::new(iri("http://a.example/s")),
            p: Box::new(iri("http://a.example/p")),
            o: Box::new(iri("http://a.example/o")),
        }
    );
}

#[test]
fn parses_quoted_triple_term_tolerates_extra_whitespace() {
    let got =
        parse_node("<<   <http://a.example/s>\t<http://a.example/p>\n\n<http://a.example/o>   >>");
    assert_eq!(
        got,
        TermValue::Triple {
            s: Box::new(iri("http://a.example/s")),
            p: Box::new(iri("http://a.example/p")),
            o: Box::new(iri("http://a.example/o")),
        }
    );
}

#[test]
fn parses_nested_quoted_triple_term() {
    let got = parse_node(
        "<< << <http://a.example/s> <http://a.example/p> <http://a.example/o> >> <http://a.example/p2> <http://a.example/o2> >>",
    );
    let inner = TermValue::Triple {
        s: Box::new(iri("http://a.example/s")),
        p: Box::new(iri("http://a.example/p")),
        o: Box::new(iri("http://a.example/o")),
    };
    assert_eq!(
        got,
        TermValue::Triple {
            s: Box::new(inner),
            p: Box::new(iri("http://a.example/p2")),
            o: Box::new(iri("http://a.example/o2")),
        }
    );
}

#[test]
fn parses_quoted_triple_with_blank_and_literal_positions() {
    let got = parse_node(r#"<< _:b1 <http://a.example/p> "lit"@en >>"#);
    assert_eq!(
        got,
        TermValue::Triple {
            s: Box::new(TermValue::blank("b1")),
            p: Box::new(iri("http://a.example/p")),
            o: Box::new(TermValue::lang_literal("lit", "en")),
        }
    );
}

#[test]
fn quoted_triple_term_requires_closing_delimiter() {
    let err = parse_shape_map(
        "<< <http://a.example/s> <http://a.example/p> <http://a.example/o> @START",
        None,
    );
    assert!(err.is_err());
}

/// A relative `<iri>` in a shape map with no base is refused with the same shared
/// code the schema parser reports, rather than becoming a selector that silently
/// matches nothing and yields a clean, empty result map.
#[test]
fn relative_selector_with_no_base_is_refused() {
    let err = parse_shape_map("<alice>@<http://a.example/S>", None).unwrap_err();
    assert!(err.to_string().contains("iri-relative-no-base"), "{err}");
    let map = parse_shape_map(
        "<alice>@<http://a.example/S>",
        Some("http://a.example/dir/"),
    )
    .expect("a base in scope resolves the selector");
    assert_eq!(
        map.0[0].node,
        NodeSelector::Node(iri("http://a.example/dir/alice"))
    );
}

#[test]
fn resolve_then_validate() {
    let data = data();
    let schema = parse_shexc("<http://a.example/S> {}", None).expect("schema");
    let map =
        parse_shape_map(&format!("{{FOCUS <{P1}> _}}@<http://a.example/S>"), None).expect("map");
    let associations = resolve_shape_map(&map, &data);
    assert_eq!(associations.len(), 2);
    let result = validate(&schema, &data, &associations);
    assert!(result.all_conformant());
    assert!(
        result
            .entries
            .iter()
            .all(|e| e.status == ConformanceStatus::Conformant)
    );
}

// ── A shape the schema does not declare ────────────────────────────────────────
//
// Undefined in both specifications: ShEx 2.1 §5.7's reference requirement binds a
// `shapeExprRef` written INSIDE a schema, and `satisfies` is defined only where the
// label resolves to a shape expression; the ShapeMap specification is silent, and
// its `status` vocabulary is conformant/nonconformant with no third value. The
// no-optionality/hard-fail doctrine decides it as a caller error.

#[test]
fn a_map_naming_an_undeclared_shape_is_refused_not_answered_nonconformant() {
    let data = data();
    let schema = parse_shexc("<http://a.example/S> {}", None).expect("schema");

    let err = validate_shape_map(
        &schema,
        &data,
        "<http://a.example/s1>@<http://a.example/Missing>",
        None,
        &ValidationOptions::default(),
    )
    .expect_err("an undeclared shape is refused");
    assert!(matches!(err, ShexError::UnknownShape(_)), "{err:?}");
    assert!(
        err.to_string()
            .contains("<http://a.example/Missing>, which the schema does not declare"),
        "the refusal names the selector as the ShapeMap grammar writes it: {err}"
    );

    // The declared label on the same schema and map still validates.
    let ok = validate_shape_map(
        &schema,
        &data,
        "<http://a.example/s1>@<http://a.example/S>",
        None,
        &ValidationOptions::default(),
    )
    .expect("a declared label decides normally");
    assert!(ok.all_conformant());
}

/// `START` against a schema with no `start` is the same mistake and is refused too.
#[test]
fn a_map_naming_start_on_a_startless_schema_is_refused() {
    let data = data();
    let startless = parse_shexc("<http://a.example/S> {}", None).expect("schema");
    let err = validate_shape_map(
        &startless,
        &data,
        "<http://a.example/s1>@START",
        None,
        &ValidationOptions::default(),
    )
    .expect_err("no start shape to decide against");
    assert!(matches!(err, ShexError::UnknownShape(_)), "{err:?}");
    assert!(err.to_string().contains("START"), "{err}");

    let with_start = parse_shexc("start = {}", None).expect("schema with a start");
    let ok = validate_shape_map(
        &with_start,
        &data,
        "<http://a.example/s1>@START",
        None,
        &ValidationOptions::default(),
    )
    .expect("a declared start decides normally");
    assert!(ok.all_conformant());
}

/// The refusal is decided from the MAP and the SCHEMA alone, so a selector that
/// happens to select no node is refused exactly as one that selects many is.
///
/// Otherwise the mistake would be invisible precisely when the result shape map is
/// empty — the case an operator is least able to tell from "nothing matched".
#[test]
fn an_undeclared_shape_is_refused_even_when_the_selector_matches_nothing() {
    let data = data();
    let schema = parse_shexc("<http://a.example/S> {}", None).expect("schema");
    let err = validate_shape_map(
        &schema,
        &data,
        "{FOCUS <http://a.example/nothingHasThis> _}@<http://a.example/Missing>",
        None,
        &ValidationOptions::default(),
    )
    .expect_err("refused before the selector is expanded");
    assert!(matches!(err, ShexError::UnknownShape(_)), "{err:?}");
}

/// A label an IMPORTED schema declares counts as declared, because
/// `resolve_imports` folds the closure in before validation.
#[test]
fn a_label_from_the_import_closure_counts_as_declared() {
    let data = data();
    let root = parse_shexc(
        "IMPORT <http://a.example/imported>\n<http://a.example/S> {}",
        None,
    )
    .expect("root schema");
    let imported = parse_shexc("<http://a.example/Inherited> {}", None).expect("imported schema");
    let folded = resolve_imports(root, &|_| Ok(imported.clone())).expect("import folds");

    let ok = validate_shape_map(
        &folded,
        &data,
        "<http://a.example/s1>@<http://a.example/Inherited>",
        None,
        &ValidationOptions::default(),
    )
    .expect("an imported label is declared");
    assert!(ok.all_conformant());
}
