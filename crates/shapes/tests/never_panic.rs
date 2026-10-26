// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! "Reject malformed, never panic" property gate (T7) for the purrdf-shapes
//! shapes frontend.
//!
//! `engine::parse_shapes` parses untrusted SHACL Turtle; given arbitrary input it
//! must return `Ok`/`Err`, never panic. Inputs are bounded so a superlinear
//! parse cannot become a spurious timeout. See `crates/rdf/tests/never_panic.rs`
//! for the contract rationale.

use purrdf_lex::json::{Number, Object, Value};
use purrdf_shapes::engine::parse_shapes;
use purrdf_shapes::json_schema::Namespaces;
use purrdf_shapes::{
    LinkmlDocument, SchemaDatatypeMap, SchemaImportConfig, import_json_schema, import_linkml,
    parse_linkml,
};
use purrdf_testkit::prop::prelude::*;

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

const NODE_EXPR_PREFIXES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
     @prefix ex: <http://example.org/ns#> .\n";

#[test]
fn recursive_shape_references_are_failures_instead_of_empty_shapes() {
    for body in [
        "ex:S a sh:NodeShape; sh:node ex:S .",
        "_:s a sh:NodeShape; sh:node _:s .",
        "ex:S a sh:NodeShape; sh:node ex:T . ex:T sh:node ex:S .",
        "ex:S a sh:NodeShape; sh:not ex:S .",
        "ex:S a sh:NodeShape; sh:and (ex:S) .",
        "ex:S a sh:NodeShape; sh:or (ex:S) .",
        "ex:S a sh:NodeShape; sh:xone (ex:S) .",
        "ex:S a sh:NodeShape; sh:property ex:P . ex:P sh:path ex:p; sh:node ex:S .",
        "ex:P a sh:PropertyShape; sh:path ex:p; sh:property ex:P .",
        "ex:P a sh:PropertyShape; sh:path ex:p; sh:node ex:P .",
        "ex:S a sh:NodeShape; sh:property ex:P . ex:P sh:path ex:p; sh:qualifiedValueShape ex:S; sh:qualifiedMinCount 1 .",
    ] {
        let error = parse_shapes(&format!("{NODE_EXPR_PREFIXES}{body}"), None)
            .expect_err("a cyclic shape must not be replaced with an unconstrained shape");
        assert!(
            error.to_string().contains("cyclic SHACL shape reference"),
            "{body}: {error}"
        );
    }
}

#[test]
fn shared_shape_references_are_not_cycles() {
    let shapes = parse_shapes(
        &format!(
            "{NODE_EXPR_PREFIXES}\
            ex:S a sh:NodeShape; sh:and (ex:Left ex:Right) .\
            ex:Left sh:node ex:Shared . ex:Right sh:node ex:Shared .\
            ex:Shared sh:nodeKind sh:IRI ."
        ),
        None,
    )
    .expect("the in-flight guard must release each completed reference");
    assert!(!shapes.node_shapes.is_empty());
}

#[test]
fn recursion_failure_names_component_parameters_and_polarity() {
    use purrdf_shapes::{RecursionPolarity, ShapesError};
    for (body, expected) in [
        ("ex:S sh:node ex:S .", RecursionPolarity::Positive),
        ("ex:S sh:not ex:S .", RecursionPolarity::Negative),
        ("ex:S sh:xone (ex:S) .", RecursionPolarity::Both),
        ("ex:S sh:targetWhere ex:S .", RecursionPolarity::Target),
        ("ex:S sh:memberShape ex:S .", RecursionPolarity::Positive),
        ("ex:S sh:someValue ex:S .", RecursionPolarity::Positive),
        (
            "ex:S sh:path ex:p; sh:reifierShape ex:S .",
            RecursionPolarity::Positive,
        ),
        (
            "ex:S sh:path ex:p; sh:qualifiedValueShape ex:S; sh:qualifiedMinCount 1 .",
            RecursionPolarity::Positive,
        ),
        (
            "ex:S sh:path ex:p; sh:qualifiedValueShape ex:S; sh:qualifiedMaxCount 1 .",
            RecursionPolarity::Negative,
        ),
        (
            "ex:S sh:path ex:p; sh:qualifiedValueShape ex:S; sh:qualifiedMinCount 1; sh:qualifiedMaxCount 2 .",
            RecursionPolarity::Both,
        ),
    ] {
        let error = parse_shapes(&format!("{NODE_EXPR_PREFIXES}{body}"), None)
            .expect_err("recursive catalogs require explicit semantics");
        let ShapesError::Recursion(refusal) = error else {
            panic!("{body}: expected a typed recursive-component refusal, got {error}");
        };
        assert_eq!(refusal.members(), ["<http://example.org/ns#S>"]);
        assert_eq!(refusal.edges().len(), 1, "{body}: {refusal:?}");
        assert_eq!(refusal.edges()[0].polarity, expected, "{body}");
    }
}

#[test]
fn recursive_component_refusal_is_independent_of_statement_order() {
    let a = "ex:A sh:node ex:B . ex:B sh:not ex:A .";
    let b = "ex:B sh:not ex:A . ex:A sh:node ex:B .";
    let error = |body| parse_shapes(&format!("{NODE_EXPR_PREFIXES}{body}"), None).unwrap_err();
    assert_eq!(error(a), error(b));
}

#[test]
fn recursion_analysis_uses_all_shapes_including_parameter_declarations() {
    let error = parse_shapes(
        &format!(
            "{NODE_EXPR_PREFIXES}\
             ex:C a sh:ConstraintComponent; sh:parameter ex:P .\
             ex:P sh:path ex:argument; sh:node ex:P ."
        ),
        None,
    )
    .expect_err("the refers graph includes shapes that are not explicit-target candidates");
    let purrdf_shapes::ShapesError::Recursion(refusal) = error else {
        panic!("expected the complete specification census to detect recursion: {error}");
    };
    assert_eq!(refusal.members(), ["<http://example.org/ns#P>"]);
}

#[test]
fn disjoint_qualified_siblings_contribute_the_opposite_count_polarity() {
    use purrdf_shapes::{RecursionPolarity, ShapesError};
    for (bounds, expected) in [
        ("sh:qualifiedMinCount 1", RecursionPolarity::Negative),
        ("sh:qualifiedMaxCount 1", RecursionPolarity::Positive),
        (
            "sh:qualifiedMinCount 1; sh:qualifiedMaxCount 2",
            RecursionPolarity::Both,
        ),
    ] {
        let error = parse_shapes(
            &format!(
                "{NODE_EXPR_PREFIXES}\
                 ex:Root sh:property ex:P, ex:Q .\
                 ex:P sh:path ex:p; sh:qualifiedValueShape ex:S;\
                     sh:qualifiedValueShapesDisjoint true; {bounds} .\
                 ex:Q sh:path ex:q; sh:qualifiedValueShape ex:T; sh:qualifiedMinCount 1 .\
                 ex:S sh:nodeKind sh:IRI . ex:T sh:node ex:P ."
            ),
            None,
        )
        .unwrap_err();
        let ShapesError::Recursion(refusal) = error else {
            panic!("expected the implicit sibling dependency to close the cycle: {error}");
        };
        assert_eq!(
            refusal.members(),
            ["<http://example.org/ns#P>", "<http://example.org/ns#T>"]
        );
        let sibling = refusal
            .edges()
            .iter()
            .find(|edge| edge.parameter == purrdf_iri::vocab::sh::QUALIFIED_VALUE_SHAPES_DISJOINT)
            .expect("the implicit edge must identify its disjoint-count origin");
        assert_eq!(sibling.polarity, expected, "{bounds}");
    }
}

#[test]
fn lower_stratum_negation_is_not_recursive() {
    parse_shapes(
        &format!("{NODE_EXPR_PREFIXES}ex:S sh:not ex:T . ex:T sh:nodeKind sh:IRI ."),
        None,
    )
    .expect("a negative reference outside an SCC is stratified");
}

#[test]
fn long_recursive_catalog_is_analyzed_before_model_expansion() {
    use std::fmt::Write as _;
    let mut document = NODE_EXPR_PREFIXES.to_owned();
    for index in 0..2000 {
        writeln!(document, "ex:S{index} sh:node ex:S{} .", (index + 1) % 2000).unwrap();
    }
    let error = purrdf_stack::on_stack(1024 * 1024, move || parse_shapes(&document, None))
        .unwrap()
        .expect_err("the complete cycle must be detected without unfolding it");
    let purrdf_shapes::ShapesError::Recursion(refusal) = error else {
        panic!("expected static SCC refusal rather than native stack exhaustion: {error}");
    };
    assert_eq!(refusal.members().len(), 2000);
    assert_eq!(refusal.edges().len(), 2000);
}

#[test]
fn deep_acyclic_shape_loading_refuses_before_native_stack_exhaustion() {
    for property in [false, true] {
        let mut document = NODE_EXPR_PREFIXES.to_owned();
        document.push_str("ex:S0 a sh:NodeShape .\n");
        for index in 0..2000 {
            use std::fmt::Write as _;
            if property {
                writeln!(
                    document,
                    "ex:S{index} sh:path ex:p; sh:property ex:S{} .",
                    index + 1
                )
                .unwrap();
            } else {
                writeln!(document, "ex:S{index} sh:node ex:S{} .", index + 1).unwrap();
            }
        }
        if property {
            document.push_str("ex:S2000 sh:path ex:p .\n");
        }
        let error = purrdf_stack::on_stack(1024 * 1024, move || {
            parse_shapes(&document, None).map_err(|error| error.to_string())
        })
        .unwrap()
        .expect_err("the deep graph must return a physical refusal");
        assert!(error.contains("stack margin"), "{error}");
    }
}

#[test]
fn deep_acyclic_paths_and_expressions_refuse_before_stack_exhaustion() {
    use std::fmt::Write as _;
    for expression in [false, true] {
        let mut document = NODE_EXPR_PREFIXES.to_owned();
        document.push_str(if expression {
            "ex:S a sh:NodeShape; sh:expression ex:E0 .\n"
        } else {
            "ex:S a sh:NodeShape; sh:property [ sh:path _:p0 ] .\n"
        });
        for index in 0..2000 {
            if expression {
                writeln!(document, "ex:E{index} sh:union (ex:E{}) .", index + 1).unwrap();
            } else {
                writeln!(document, "_:p{index} sh:inversePath _:p{} .", index + 1).unwrap();
            }
        }
        if !expression {
            document.push_str("_:p2000 sh:inversePath ex:p .\n");
        }
        let error = purrdf_stack::on_stack(1024 * 1024, move || {
            parse_shapes(&document, None).map_err(|error| error.to_string())
        })
        .unwrap()
        .expect_err("the deep graph must return a physical refusal");
        assert!(error.contains("stack margin"), "{error}");
    }
}

#[test]
fn stack_refusal_is_not_a_conformance_or_validation_report() {
    use purrdf_shapes::constraints::conforms;
    use purrdf_shapes::data::ShaclData;
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::term::NamedNode;
    use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
    use std::sync::Arc;

    let shapes = Arc::new(parse_shapes(
        &format!("{NODE_EXPR_PREFIXES}ex:S a sh:NodeShape; sh:targetNode ex:alice; sh:nodeKind sh:IRI ."),
        None,
    ).unwrap());
    let data = parse_turtle_to_dataset(
        &format!("{NODE_EXPR_PREFIXES}ex:alice ex:p ex:object ."),
        None,
    )
    .unwrap();
    let prepared = PreparedShapes::new(Arc::clone(&shapes));
    let validator = prepared.bind_shared_dataset(Arc::clone(&data)).unwrap();
    let complete = prepared
        .bind_complete_shared_dataset(Arc::clone(&data))
        .unwrap();
    let store = ShaclData::new(Arc::clone(&data), data, None);
    let focus = NamedNode::new_unchecked("http://example.org/ns#alice").into_term();

    // Reserve the available stack through the production admission API, then
    // release it before inspecting results. No actual stack overflow is induced.
    let reserve = purrdf_stack::reserve(purrdf_stack::remaining());
    let conformance = conforms(&store, &focus, &shapes.node_shapes[0]);
    let report = validator.validate();
    let complete_report = complete.validate();
    drop(reserve);
    let conformance = conformance.unwrap_err();
    assert_eq!(
        conformance.failure_class(),
        purrdf_shapes::FailureClass::Processor
    );
    assert!(conformance.to_string().contains("stack margin"));
    assert!(report.unwrap_err().contains("stack margin"));
    assert!(
        complete_report
            .unwrap_err()
            .to_string()
            .contains("stack margin")
    );
    assert!(conforms(&store, &focus, &shapes.node_shapes[0]).unwrap());
    assert!(validator.validate().unwrap().conforms);
    assert!(complete.validate().is_ok());
}

#[test]
fn preparation_refusal_cannot_become_an_empty_path_or_product() {
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::product::{ProductDimension, ShapesProfile};
    use purrdf_shapes::shapes::Path;
    use purrdf_shapes::term::{NamedNode, Term};
    use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
    use std::sync::Arc;

    let shapes = Arc::new(
        parse_shapes(
            &format!("{NODE_EXPR_PREFIXES}ex:S sh:targetNode ex:a; sh:nodeKind sh:IRI ."),
            None,
        )
        .unwrap(),
    );
    let data =
        parse_turtle_to_dataset(&format!("{NODE_EXPR_PREFIXES}ex:a ex:p ex:b ."), None).unwrap();
    let focus = Term::NamedNode(NamedNode::new_unchecked("http://example.org/ns#a"));
    let path = Path::Predicate(NamedNode::new_unchecked("http://example.org/ns#p"));
    let reserve = purrdf_stack::reserve(purrdf_stack::remaining());
    let refused = PreparedShapes::new(Arc::clone(&shapes));
    let bound = refused.bind_shared_dataset(Arc::clone(&data)).err();
    let product = refused.to_product(&ShapesProfile::CORE);
    let terms = purrdf_shapes::path::eval(data.as_ref(), &focus, &path);
    let ids = purrdf_shapes::path::eval_ids(data.as_ref(), &focus, &path);
    drop(reserve);

    assert!(matches!(
        bound.unwrap(),
        purrdf_shapes::ShapesError::Preparation(purrdf_shapes::PreparationRefusal::StackMargin)
    ));
    let product_error = product.unwrap_err();
    assert_eq!(product_error.dimension(), ProductDimension::DepthLimit);
    assert_eq!(
        product_error.preparation_refusal(),
        Some(&purrdf_shapes::PreparationRefusal::StackMargin)
    );
    assert!(
        std::error::Error::source(&product_error)
            .unwrap()
            .downcast_ref::<purrdf_shapes::PreparationRefusal>()
            .is_some()
    );
    assert!(matches!(
        terms.unwrap_err(),
        purrdf_shapes::ShapesError::Preparation(purrdf_shapes::PreparationRefusal::StackMargin)
    ));
    assert!(matches!(
        ids.unwrap_err(),
        purrdf_shapes::ShapesError::Preparation(purrdf_shapes::PreparationRefusal::StackMargin)
    ));
    // A refused preparation cannot later publish a made-up empty class catalog.
    assert!(refused.to_product(&ShapesProfile::CORE).is_err());
    let fresh = PreparedShapes::new(shapes);
    assert!(
        fresh
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    assert!(fresh.to_product(&ShapesProfile::CORE).is_ok());
    assert_eq!(
        purrdf_shapes::path::eval(data.as_ref(), &focus, &path)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn deep_built_models_refuse_preparation_on_a_small_stack() {
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::expression::NodeExpr;
    use purrdf_shapes::shapes::{Constraint, Path};
    use purrdf_shapes::term::NamedNode;
    use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
    use std::sync::Arc;

    // Keep the owned deep model and its eventual destruction on the outer stack;
    // the inner stack only borrows the graph during the actual preparation.
    purrdf_stack::on_stack(32 * 1024 * 1024, || {
        for family in 0..3 {
            let mut shapes = parse_shapes(
                &format!("{NODE_EXPR_PREFIXES}ex:S sh:targetNode ex:a; sh:nodeKind sh:IRI ."),
                None,
            )
            .unwrap();
            match family {
                0 => {
                    let leaf = shapes.node_shapes[0].clone();
                    let mut nested = leaf.clone();
                    for _ in 0..2000 {
                        let mut parent = leaf.clone();
                        parent.constraints = vec![Constraint::Node(Box::new(nested))];
                        nested = parent;
                    }
                    shapes.node_shapes = vec![nested];
                }
                1 => {
                    let mut path =
                        Path::Predicate(NamedNode::new_unchecked("http://example.org/ns#p"));
                    for _ in 0..2000 {
                        path = Path::ZeroOrOne(Box::new(path));
                    }
                    shapes.node_shapes[0].constraints = vec![Constraint::Equals(path)];
                }
                _ => {
                    let mut expr = NodeExpr::This;
                    for _ in 0..2000 {
                        expr = NodeExpr::Distinct(Box::new(expr));
                    }
                    shapes.node_shapes[0].constraints = vec![Constraint::Expression {
                        expr,
                        messages: Vec::new(),
                        severity: None,
                    }];
                }
            }
            let shapes = Arc::new(shapes);
            let data = parse_turtle_to_dataset("", None).unwrap();
            let error = purrdf_stack::on_stack_scoped(1024 * 1024, || {
                // The frame starts above the canonical margin, with 32 KiB
                // available for descent. This exercises refusal inside the walk;
                // a 2,000-node chain can legitimately fit on the full 1 MiB.
                let reserve = purrdf_stack::reserve(
                    purrdf_stack::remaining()
                        .saturating_sub(purrdf_stack::MARGIN_BYTES + 32 * 1024),
                );
                assert!(!purrdf_stack::is_low(), "the root frame must be admitted");
                let error = PreparedShapes::new(Arc::clone(&shapes))
                    .bind_shared_dataset(Arc::clone(&data))
                    .expect_err("deep preparation must refuse before it exhausts the thread")
                    .to_string();
                drop(reserve);
                error
            })
            .unwrap();
            assert!(error.contains("stack margin"), "family {family}: {error}");
        }
    })
    .unwrap();
}

#[test]
fn shallow_path_copy_and_drop_add_no_work_list_allocations() {
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_shapes::shapes::Path;
    use purrdf_shapes::term::NamedNode;

    let first = "http://example.org/ns#a";
    let second = "http://example.org/ns#b";
    let path = Path::Sequence(vec![
        Path::Predicate(NamedNode::new_unchecked(first)),
        Path::Inverse(Box::new(Path::Predicate(NamedNode::new_unchecked(second)))),
    ]);
    let window = CurrentThreadWindow::open();
    let copied = path.clone();
    let copied_allocations = window.close();
    assert_eq!(copied_allocations.allocations, 4);
    assert_eq!(
        copied_allocations.requested_bytes,
        u64::try_from(3 * size_of::<Path>() + first.len() + second.len()).unwrap()
    );
    assert_eq!(format!("{path:?}"), format!("{copied:?}"));
    let window = CurrentThreadWindow::open();
    drop(copied);
    assert_eq!(window.close().allocations, 0);
}

/// A cyclic SHACL-AF node expression must be a hard `Err`, whatever node kind
/// spells the cycle.
///
/// Rust stack exhaustion ABORTS the process and is not catchable, so an
/// unguarded node-expression cycle is not a "panic" this suite could observe —
/// the test binary reaching its own assertions is the proof that the parse
/// terminated. Both cases below are reachable from ordinary Turtle: a named
/// node referring to itself through `sh:union`, and a labelled blank node doing
/// the same.
#[test]
fn cyclic_node_expression_graph_is_hard_error() {
    let named = parse_shapes(
        &format!(
            "{NODE_EXPR_PREFIXES}\
         ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:expression ex:E .\n\
         ex:E sh:union ( ex:E ) .\n"
        ),
        None,
    )
    .expect_err("a named-node expression cycle must be rejected")
    .to_string();
    assert!(
        named.contains("cyclic"),
        "error must name the cycle, got: {named}"
    );

    let blank = parse_shapes(
        &format!(
            "{NODE_EXPR_PREFIXES}\
         ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:expression _:e .\n\
         _:e sh:union ( _:e ) .\n"
        ),
        None,
    )
    .expect_err("a blank-node expression cycle must be rejected")
    .to_string();
    assert!(
        blank.contains("cyclic"),
        "error must name the cycle, got: {blank}"
    );
}

/// The cycle guard is a STACK of in-flight parses, not a visited set: a node
/// expression DAG that reaches the same named sub-expression through two
/// disjoint branches is not a cycle and must still parse.
///
/// This is the anti-regression guard for extending the guard beyond blank
/// nodes — a visited-set implementation would wrongly reject this document.
#[test]
fn shared_named_sub_expression_is_not_a_cycle() {
    let shapes = parse_shapes(
        &format!(
            "{NODE_EXPR_PREFIXES}\
         ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:expression ex:Top .\n\
         ex:Top sh:union ( ex:Left ex:Right ) .\n\
         ex:Left sh:union ( ex:Shared ) .\n\
         ex:Right sh:union ( ex:Shared ) .\n\
         ex:Shared sh:union ( ex:a ) .\n"
        ),
        None,
    )
    .expect("a shared sub-expression reached twice is a DAG, not a cycle");
    assert_eq!(shapes.node_shapes.len(), 1);
}

/// Structure-aware SHACL Turtle: real `sh:` shape fragments interleaved with
/// noise, to reach the shape-graph interpreter, not just the Turtle lexer.
fn structured_shapes() -> impl Strategy<Value = String> {
    prop::sample::interleaved(SHAPES_FRAGMENTS.to_vec(), 24)
}

/// The fragments [`structured_shapes`] interleaves.
const SHAPES_FRAGMENTS: [&str; 13] = [
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <https://example.org/> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "ex:S a sh:NodeShape ; sh:targetClass ex:C .\n",
    "ex:S sh:property [ sh:path ex:p ; sh:minCount 1 ] .\n",
    "ex:S sh:property [ sh:path ex:p ; sh:datatype xsd:string ] .\n",
    "ex:S sh:property [ sh:path ex:p ; sh:pattern \"^a+$\" ] .\n",
    "ex:S sh:property [ sh:path ex:p ; sh:minCount \"notanint\" ] .\n",
    "ex:S sh:node ex:S .\n",
    "ex:S sh:property [ sh:path [ sh:inversePath ex:p ] ] .\n",
    "\u{0}\u{1}",
    "ex:S a sh:NodeShape ; sh:property",
    "@prefix sh:",
];

fn arbitrary_json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|value| Value::Number(Number::from(value))),
        prop::string::regex(".{0,64}").prop_map(Value::String),
    ];
    leaf.prop_recursive(5, 128, 8, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..8).prop_map(Value::Array),
            prop::collection::btree_map(prop::string::regex(".{0,24}"), inner, 0..8)
                .prop_map(|entries| { Value::Object(entries.into_iter().collect::<Object>()) }),
        ]
    })
}

fn schema_import_config() -> SchemaImportConfig {
    use purrdf_xsd::datatype::XSD_NS as XSD;
    let namespaces = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )
    .expect("valid test namespace");
    let datatypes = SchemaDatatypeMap::new(
        format!("{XSD}string"),
        format!("{XSD}boolean"),
        format!("{XSD}integer"),
        format!("{XSD}decimal"),
        format!("{XSD}dateTime"),
        format!("{XSD}date"),
        format!("{XSD}time"),
        format!("{XSD}anyURI"),
    )
    .expect("valid test datatypes");
    SchemaImportConfig::new(namespaces, datatypes)
}

prop_test! {
    #![prop_config(Config { cases: 256, ..Config::default() })]

    #[test]
    fn parse_shapes_never_panics_raw(data in prop::collection::bytes(4096)) {
        if let Ok(text) = std::str::from_utf8(&data) {
            let _ = parse_shapes(text, None);
        }
    }

    #[test]
    fn parse_shapes_never_panics_structured(text in structured_shapes()) {
        let _ = parse_shapes(&text, None);
    }

    #[test]
    fn import_json_schema_never_panics(value in arbitrary_json()) {
        let input = purrdf_lex::json::write_compact(&value);
        let _ = import_json_schema(&input, &schema_import_config());
    }

    #[test]
    fn parse_linkml_never_panics(data in prop::collection::bytes(4096)) {
        if let Ok(text) = std::str::from_utf8(&data) {
            let _ = parse_linkml(text);
        }
    }

    #[test]
    fn import_linkml_never_panics(section in 0_usize..4, value in arbitrary_json()) {
        let mut document = purrdf_lex::json::read(r#"{
            "id": "https://example.org/schema/linkml",
            "name": "Generated-Schema",
            "metamodel_version": "1.11.0",
            "prefixes": {
                "ex": "https://example.org/",
                "linkml": "https://w3id.org/linkml/"
            },
            "default_prefix": "ex",
            "classes": {},
            "enums": {},
            "slots": {},
            "types": {}
        }"#)
        .expect("literal");
        let section_name = ["classes", "enums", "slots", "types"][section];
        document[section_name]["Generated"] = value;
        if let Ok(document) = LinkmlDocument::from_value(document) {
            let _ = import_linkml(&document, &schema_import_config());
        }
    }
}
