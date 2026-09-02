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
    assert_eq!(got, [] as [_; 0]);
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

// ── Prefixed names ─────────────────────────────────────────────────────────────
//
// The ShapeMap grammar's `[136s] iri` production is IRIREF alone — the
// `| prefixedName` alternative is commented out in the specification's source, as
// are `shapeSpec`'s ATPNAME arms — and the one paragraph mentioning prefixes says
// resolving shape references against the schema's context is "common practice"
// that "this specification does not specify". So `ex:S1` is refused, and the
// refusal has to say WHY rather than leave a user guessing at a syntax error.

/// Every IRI position names the real reason for rejecting a prefixed name.
#[test]
fn a_prefixed_name_is_refused_with_the_grammar_reason_in_every_iri_position() {
    for map_src in [
        // shape label
        "<http://a.example/s1>@ex:S1",
        // node
        "ex:alice@<http://a.example/S>",
        // predicate, in both triple-pattern directions
        "{FOCUS ex:p _}@<http://a.example/S>",
        "{_ ex:p FOCUS}@<http://a.example/S>",
        // datatype of a literal node
        r#""7"^^xsd:integer@<http://a.example/S>"#,
    ] {
        let err = parse_shape_map(map_src, Some("http://a.example/"))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("is a prefixed name") && err.contains("IRIREF only"),
            "`{map_src}` must name the grammar reason: {err}"
        );
        assert!(
            err.contains("does not") && err.contains("specify"),
            "`{map_src}` must say the spec declined to define the prefix map: {err}"
        );
        assert!(
            err.contains("Write the IRI in full"),
            "`{map_src}` must name the remedy: {err}"
        );
    }
}

/// A literal's `^^` datatype is an IRI position like any other.
///
/// It was the one caller that did not pre-check the opening `<`, so `parse_iri` read the
/// datatype's first character as the bracket and ran to end-of-input: `"7"^^xsd:integer`
/// reported `unterminated IRI`, blaming the wrong thing entirely. The bracket check now
/// lives in `parse_iri`, so a non-IRI datatype is diagnosed where it goes wrong.
#[test]
fn a_literal_datatype_is_diagnosed_as_an_iri_position() {
    let prefixed = parse_shape_map(r#""7"^^xsd:integer@<http://a.example/S>"#, None)
        .unwrap_err()
        .to_string();
    assert!(
        prefixed.contains("`xsd:integer` is a prefixed name"),
        "the datatype's real defect is named: {prefixed}"
    );
    assert!(
        !prefixed.contains("unterminated"),
        "and it is no longer blamed on a runaway bracket scan: {prefixed}"
    );

    let bare = parse_shape_map(r#""7"^^integer@<http://a.example/S>"#, None)
        .unwrap_err()
        .to_string();
    assert!(
        bare.contains("expected an IRI in angle brackets"),
        "a bare datatype token names the IRI position too: {bare}"
    );

    // The admitted form is unchanged.
    let ok = parse_shape_map(
        r#""7"^^<http://www.w3.org/2001/XMLSchema#integer>@<http://a.example/S>"#,
        None,
    )
    .expect("an angle-bracketed datatype parses");
    assert_eq!(
        ok.0[0].node,
        NodeSelector::Node(TermValue::typed_literal(
            "7",
            "http://www.w3.org/2001/XMLSchema#integer"
        ))
    );
}

/// The refusal quotes the prefixed name it rejected, including the empty-prefix
/// form (`PNAME_NS ::= PN_PREFIX? ':'`) and a bare namespace.
#[test]
fn the_refusal_quotes_the_offending_prefixed_name() {
    for (map_src, quoted) in [
        ("<http://a.example/s1>@ex:S1", "`ex:S1`"),
        ("<http://a.example/s1>@:S1", "`:S1`"),
        ("<http://a.example/s1>@ex:", "`ex:`"),
    ] {
        let err = parse_shape_map(map_src, None).unwrap_err().to_string();
        assert!(err.contains(quoted), "`{map_src}`: {err}");
    }
}

/// A prefixed name whose prefix spells a keyword is a prefixed name, not the
/// keyword followed by junk — which is why the check runs BEFORE `START` and `a`.
#[test]
fn a_keyword_shaped_prefix_is_read_as_a_prefixed_name() {
    for map_src in [
        "<http://a.example/s1>@START:S",
        "{FOCUS a:p _}@<http://a.example/S>",
    ] {
        let err = parse_shape_map(map_src, None).unwrap_err().to_string();
        assert!(
            err.contains("is a prefixed name"),
            "`{map_src}` must not be read as the keyword plus trailing input: {err}"
        );
    }
}

/// The forms the grammar DOES admit keep working, unchanged — the detector must
/// not swallow `_:label`, `START`, `a`, or an angle-bracketed IRI.
#[test]
fn the_admitted_forms_are_untouched_by_the_prefixed_name_check() {
    let start = parse_shape_map("<http://a.example/s1>@START", None).expect("START still parses");
    assert_eq!(start.0[0].shape, ShapeSelector::Start);

    // `_:label` is BLANK_NODE_LABEL, not a prefixed name, in every term position.
    let blank = parse_shape_map("_:b1@<http://a.example/S>", None).expect("a blank node parses");
    assert_eq!(blank.0[0].node, NodeSelector::Node(TermValue::blank("b1")));
    let anchored = parse_shape_map(&format!("{{_:b1 <{P1}> FOCUS}}@<http://a.example/S>"), None)
        .expect("a blank-node subject anchor parses");
    assert!(matches!(
        anchored.0[0].node,
        NodeSelector::ObjectOf {
            subject: Some(TermValue::Blank { .. }),
            ..
        }
    ));

    // `a` is still rdf:type, and a relative IRI still resolves against the base.
    let typed = parse_shape_map(
        "{FOCUS a <http://a.example/C>}@<S>",
        Some("http://a.example/"),
    )
    .expect("`a` and a relative shape label parse");
    assert_eq!(
        typed.0[0].shape,
        ShapeSelector::Label("http://a.example/S".to_owned())
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
