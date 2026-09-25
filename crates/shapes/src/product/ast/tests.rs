// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Tests for the declarative-AST codec.
//!
//! Deliberately function-only: the product-model census scans every
//! `crates/shapes/src/**/*.rs` file on its own, and a `struct` or a `type` alias
//! declared here would be censused as part of the model it is testing.
//!
//! Every refusal below is executed alongside a NEIGHBOURING VALID case. A
//! refusal is a claim, and an over-refusal — rejecting input that is actually
//! valid — hides perfectly: every test still passes, and the strictness looks
//! correct right up until a user writes the shapes graph that should load and
//! doesn't.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use super::{
    AstReader, AstWriter, MAX_DEPTH, MAX_SPECULATIVE_ELEMENTS, TAGS_ANNOTATED_CONSTRAINT,
    TAGS_ARG_KEY, TAGS_CLOSED_MODE, TAGS_COMPONENT_VALIDATOR, TAGS_CONSTRAINT, TAGS_CUSTOM_FN_KIND,
    TAGS_FN_CALL, TAGS_NODE_EXPR, TAGS_NODE_KIND, TAGS_PATH, TAGS_RULE_BODY, TAGS_SEVERITY,
    TAGS_SHAPE_ARG, TAGS_TARGET, TAGS_TERM, decode_ast, encode_ast_derived, speculative_capacity,
    write_varint,
};
use crate::expression::{
    ArgKey, CustomFnKind, CustomFunction, FnCall, NodeExpr, ShapeArg, sparql_ns_lowering,
};
use crate::model::{BoxRoleVocab, sparql_ns};
use crate::product::{ProductDimension, ShapesProductError};
use crate::report::Severity;
use crate::rules::{OrderKey, Rule, RuleBody, RuleGraph, RuleSetDeclaration};
use crate::shapes::{
    AnnotatedConstraint, ClosedMode, ClosedTypeIndex, ComponentValidator, Constraint,
    ConstraintAnnotation, NodeKindValue, Path, PropertyShape, Shape, Shapes, SparqlTargetType,
    Target, TargetTypeParam,
};
use crate::term::{Literal, NamedNode, Term, Triple};

// ── Fixture vocabulary (example.org, per the repository's fixture rule) ─────────

/// An `example.org` IRI.
fn ex(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("https://example.org/{local}"))
}

/// An `example.org` IRI as a term.
fn ex_term(local: &str) -> Term {
    Term::NamedNode(ex(local))
}

/// The IRI of the fixture custom node-expression function.
fn fixture_fn_iri() -> String {
    ex("fn/average").as_str().to_owned()
}

/// A custom node-expression function declaration with no body installed.
fn fixture_fn() -> Arc<CustomFunction> {
    Arc::new(CustomFunction {
        iri: ex("fn/average"),
        kind: CustomFnKind::ListParameter,
        params: vec![ArgKey::Index(0), ArgKey::Index(1)],
        required: 1,
        body: OnceLock::new(),
    })
}

/// The same declaration with a SELF-RECURSIVE body installed: the body calls the
/// very function it belongs to, which is what makes the value graph cyclic.
fn fixture_recursive_fn() -> Arc<CustomFunction> {
    let func = fixture_fn();
    func.body
        .set(NodeExpr::CustomCall {
            func: Arc::clone(&func),
            args: vec![(ArgKey::Index(0), NodeExpr::Arg(ArgKey::Index(0)))],
        })
        .expect("the fixture body installs once");
    func
}

/// A node shape with no constraints, targets, properties or rules.
fn leaf_shape(id: &str) -> Shape {
    Shape {
        id: ex_term(id),
        targets: Vec::new(),
        constraints: Vec::new(),
        property_shapes: Vec::new(),
        severity: Severity::Violation,
        messages: vec![],
        constraint_annotations: vec![],
        deactivated: false,
        box_roles: Vec::new(),
        rules: Vec::new(),
    }
}

/// A `sparql:<NAME>` call built exactly the way the shapes parser builds one:
/// the surface text is rendered through the §5 lowering table at construction.
fn sparql_call(local: &str, args: Vec<NodeExpr>) -> FnCall {
    let iri = NamedNode::new_unchecked(format!("{}{local}", sparql_ns::NS));
    let form = sparql_ns_lowering(local).expect("the fixture names a callable SPARQL 1.2 function");
    let expr = form
        .render(iri.as_str(), args.len())
        .expect("the fixture supplies the arity the form takes");
    FnCall::Sparql { iri, expr, args }
}

// ── Sample vectors: ONE value per tag, IN TAG ORDER ─────────────────────────────
//
// These vectors ARE the codec's tag declaration. `roundtrip_tags` asserts each
// sample's first encoded byte is its index, so a sample in the wrong slot fails,
// and that the vector's length equals the declared tag count, so a variant that
// was given no tag fails.

/// One [`Term`] per tag.
fn sample_terms() -> Vec<Term> {
    vec![
        ex_term("node"),
        Term::BlankNode("b0".to_owned()),
        Term::Literal(Literal::new_simple_literal("plain")),
        Term::Triple(Box::new(Triple::new(ex_term("s"), ex("p"), ex_term("o")))),
    ]
}

/// Every lexical shape an RDF literal takes — not a tag space, but the four
/// constructor paths [`AstReader::literal`] chooses between.
fn sample_literals() -> Vec<Literal> {
    vec![
        Literal::new_simple_literal("plain"),
        Literal::new_typed_literal("42", ex("Count")),
        Literal::new_language_tagged_literal_unchecked("hello", "en"),
        Literal::new_directional_language_tagged_literal_unchecked(
            "مرحبا",
            "ar",
            ::purrdf::RdfTextDirection::Rtl,
        ),
        Literal::new_directional_language_tagged_literal_unchecked(
            "hello",
            "en",
            ::purrdf::RdfTextDirection::Ltr,
        ),
    ]
}

/// One [`Severity`] per tag.
fn sample_severities() -> Vec<Severity> {
    vec![
        Severity::Violation,
        Severity::Warning,
        Severity::Info,
        Severity::Debug,
        Severity::Trace,
        Severity::Other(ex("Critical")),
    ]
}

/// One [`NodeKindValue`] per tag.
fn sample_node_kinds() -> Vec<NodeKindValue> {
    vec![
        NodeKindValue::Iri,
        NodeKindValue::BlankNode,
        NodeKindValue::Literal,
        NodeKindValue::BlankNodeOrIri,
        NodeKindValue::BlankNodeOrLiteral,
        NodeKindValue::IriOrLiteral,
        NodeKindValue::TripleTerm,
    ]
}

/// One [`Path`] per tag.
fn sample_paths() -> Vec<Path> {
    vec![
        Path::Predicate(ex("p")),
        Path::Inverse(Box::new(Path::Predicate(ex("parent")))),
        Path::Sequence(vec![Path::Predicate(ex("a")), Path::Predicate(ex("b"))]),
        Path::Alternative(vec![Path::Predicate(ex("a")), Path::Predicate(ex("b"))]),
        Path::ZeroOrMore(Box::new(Path::Predicate(ex("next")))),
        Path::OneOrMore(Box::new(Path::Predicate(ex("next")))),
        Path::ZeroOrOne(Box::new(Path::Predicate(ex("next")))),
    ]
}

/// One [`Target`] per tag.
fn sample_targets() -> Vec<Target> {
    vec![
        Target::Class(ex("Person")),
        Target::SubjectsOf(ex("name")),
        Target::ObjectsOf(ex("knows")),
        Target::Node(ex_term("alice")),
        Target::ImplicitClass(ex_term("Person")),
        Target::Sparql {
            select: "SELECT ?this WHERE { ?this a <https://example.org/Person> }".to_owned(),
            substitutions: vec![("kind".to_owned(), ex_term("Manager"))],
        },
        Target::NodeExpression(NodeExpr::Path(Path::Predicate(ex("pointsAt")))),
        Target::Where(Box::new(leaf_shape("Adult"))),
    ]
}

/// One [`ComponentValidator`] per tag.
fn sample_component_validators() -> Vec<ComponentValidator> {
    vec![
        ComponentValidator::Ask {
            ask: "ASK { $this ?p ?o }".to_owned(),
        },
        ComponentValidator::Select {
            select: "SELECT $this WHERE { $this ?p ?o }".to_owned(),
        },
    ]
}

/// One [`ArgKey`] per tag.
fn sample_arg_keys() -> Vec<ArgKey> {
    vec![
        ArgKey::Index(3),
        ArgKey::Named(ex("average").as_str().to_owned()),
    ]
}

/// One [`CustomFnKind`] per tag.
fn sample_custom_fn_kinds() -> Vec<CustomFnKind> {
    vec![CustomFnKind::ListParameter, CustomFnKind::NamedParameter]
}

/// One [`RuleBody`] per tag.
fn sample_rule_bodies() -> Vec<RuleBody> {
    vec![
        RuleBody::Triple {
            subject: None,
            predicate: Some(NodeExpr::Constant(ex_term("derived"))),
            object: Some(NodeExpr::Path(Path::Predicate(ex("source")))),
        },
        RuleBody::Sparql {
            construct: "CONSTRUCT { $this <https://example.org/p> $v } WHERE {}".to_owned(),
            parameters: vec![("v".to_owned(), ex_term("value"))],
        },
    ]
}

/// One [`ShapeArg`] per tag.
fn sample_shape_args() -> Vec<ShapeArg> {
    vec![
        ShapeArg::Named(Box::new(leaf_shape("NamedShape"))),
        ShapeArg::Computed {
            expr: Box::new(NodeExpr::Path(Path::Predicate(ex("kind")))),
            shapes: Arc::new(OnceLock::new()),
        },
    ]
}

/// One [`FnCall`] per tag.
fn sample_fn_calls() -> Vec<FnCall> {
    vec![
        FnCall::Builtin {
            iri: ex("builtin"),
            args: vec![NodeExpr::This],
        },
        FnCall::UserDefined {
            iri: ex("userFunction"),
            args: vec![NodeExpr::This, NodeExpr::Empty],
        },
        sparql_call("strlen", vec![NodeExpr::This]),
    ]
}

/// One [`NodeExpr`] per tag, in tag order.
fn sample_node_exprs(func: &Arc<CustomFunction>) -> Vec<NodeExpr> {
    let this = || Box::new(NodeExpr::This);
    vec![
        NodeExpr::Constant(ex_term("constant")),
        NodeExpr::This,
        NodeExpr::Path(Path::Predicate(ex("p"))),
        NodeExpr::Filter {
            nodes: this(),
            shape: Box::new(leaf_shape("FilterShape")),
        },
        NodeExpr::Union(vec![NodeExpr::This, NodeExpr::Empty]),
        NodeExpr::Intersection(vec![NodeExpr::This]),
        NodeExpr::If {
            cond: this(),
            then: this(),
            els: Box::new(NodeExpr::Empty),
        },
        NodeExpr::Count {
            distinct: true,
            of: this(),
        },
        NodeExpr::Distinct(this()),
        NodeExpr::Min(this()),
        NodeExpr::Max(this()),
        NodeExpr::Sum(this()),
        NodeExpr::Limit { of: this(), n: 5 },
        NodeExpr::Offset { of: this(), n: 2 },
        NodeExpr::OrderBy {
            of: this(),
            key: Box::new(NodeExpr::Path(Path::Predicate(ex("rank")))),
            descending: true,
        },
        NodeExpr::Exists(this()),
        NodeExpr::Call(FnCall::Builtin {
            iri: ex("builtin"),
            args: vec![NodeExpr::This],
        }),
        NodeExpr::Arg(ArgKey::Index(0)),
        NodeExpr::CustomCall {
            func: Arc::clone(func),
            args: vec![(ArgKey::Index(0), NodeExpr::This)],
        },
        NodeExpr::Empty,
        NodeExpr::Var("value".to_owned()),
        NodeExpr::List(vec![ex_term("a"), ex_term("b")]),
        NodeExpr::PathValues {
            path: Path::Predicate(ex("p")),
            focus: this(),
        },
        NodeExpr::Concat(vec![NodeExpr::This, NodeExpr::This]),
        NodeExpr::Remove {
            nodes: this(),
            remove: Box::new(NodeExpr::Empty),
        },
        NodeExpr::FlatMap {
            nodes: this(),
            map: Box::new(NodeExpr::Path(Path::Predicate(ex("child")))),
        },
        NodeExpr::FindFirst {
            nodes: this(),
            shape: Box::new(leaf_shape("FirstShape")),
        },
        NodeExpr::MatchAll {
            nodes: this(),
            shape: Box::new(leaf_shape("AllShape")),
        },
        NodeExpr::InstancesOf(Box::new(NodeExpr::Constant(Term::NamedNode(ex("Person"))))),
        NodeExpr::NodesMatching(Box::new(leaf_shape("MatchingShape"))),
        NodeExpr::ConformsToShape {
            node: this(),
            shape: ShapeArg::Named(Box::new(leaf_shape("ConformsShape"))),
        },
        NodeExpr::Select {
            query: "SELECT ?x WHERE { $this <https://example.org/p> ?x }".to_owned(),
            variable: "x".to_owned(),
            key: "sh:select",
        },
    ]
}

/// One [`Constraint`] per tag, in tag order.
/// A two-type `sh:closed sh:ByTypes` index, built out of canonical order.
fn sample_type_index() -> Arc<ClosedTypeIndex> {
    Arc::new(
        ClosedTypeIndex::from_entries(vec![
            (ex_term("Sub"), vec![ex("sub"), ex("root")]),
            (ex_term("Root"), vec![ex("root")]),
        ])
        .expect("distinct types"),
    )
}

/// One [`ClosedMode`] per tag.
fn sample_closed_modes() -> Vec<ClosedMode> {
    vec![
        ClosedMode::Declared,
        ClosedMode::ByTypes(sample_type_index()),
    ]
}

fn sample_constraints(func: &Arc<CustomFunction>) -> Vec<Constraint> {
    vec![
        Constraint::Class(vec![ex("Person"), ex("Agent")]),
        Constraint::Datatype(vec![ex("integer")]),
        Constraint::NodeKind(vec![NodeKindValue::IriOrLiteral]),
        Constraint::MinCount(1),
        Constraint::MaxCount(5),
        Constraint::In(vec![ex_term("a"), ex_term("b")]),
        Constraint::HasValue(ex_term("v")),
        Constraint::Pattern {
            regex: "^[A-Z]".to_owned(),
            flags: Some("i".to_owned()),
            compiled: Arc::new(OnceLock::new()),
        },
        Constraint::MinLength(3),
        Constraint::MaxLength(255),
        Constraint::UniqueLang(true),
        Constraint::LanguageIn(vec!["en".to_owned(), "fr".to_owned()]),
        Constraint::Not(Box::new(leaf_shape("NotShape"))),
        Constraint::Closed {
            ignored: vec![ex("ignored")],
            mode: ClosedMode::ByTypes(sample_type_index()),
        },
        Constraint::MinInclusive(ex_term("zero")),
        Constraint::MaxInclusive(ex_term("hundred")),
        Constraint::MinExclusive(ex_term("zero")),
        Constraint::MaxExclusive(ex_term("hundred")),
        Constraint::And(vec![leaf_shape("A"), leaf_shape("B")]),
        Constraint::Or(vec![leaf_shape("A")]),
        Constraint::Xone(vec![leaf_shape("A"), leaf_shape("B")]),
        Constraint::Node(Box::new(leaf_shape("NodeShape"))),
        Constraint::Sparql {
            select: "SELECT $this WHERE { $this ?p ?o }".to_owned(),
            messages: vec![Literal::new_simple_literal("no")],
            severity: Some(Severity::Warning),
            // Two annotations in canonical order, one with a variable and defaults,
            // so the result-annotation codec is exercised past the empty list.
            annotations: vec![
                crate::shapes::ResultAnnotation {
                    property: ex("seen"),
                    variable: Some("seen".to_owned()),
                    default_values: vec![
                        Term::Literal(Literal::new_simple_literal("unknown")),
                        ex_term("never"),
                    ],
                },
                crate::shapes::ResultAnnotation {
                    property: ex("time"),
                    variable: Some("when".to_owned()),
                    default_values: vec![],
                },
            ],
        },
        // A composite path, so the pair tags are exercised past the IRI form.
        Constraint::Equals(Path::Sequence(vec![
            Path::Inverse(Box::new(Path::Predicate(ex("p")))),
            Path::ZeroOrMore(Box::new(Path::Predicate(ex("q")))),
        ])),
        Constraint::Disjoint(Path::Predicate(ex("p"))),
        Constraint::LessThan(Path::Predicate(ex("p"))),
        Constraint::LessThanOrEquals(Path::Predicate(ex("p"))),
        Constraint::QualifiedValueShape {
            shape: Box::new(leaf_shape("Qualified")),
            siblings: vec![leaf_shape("Sibling")],
            min_count: Some(1),
            max_count: None,
            disjoint: true,
        },
        Constraint::Expression {
            expr: NodeExpr::Exists(Box::new(NodeExpr::Path(Path::Predicate(ex("p"))))),
            messages: vec![],
            severity: Some(Severity::Info),
        },
        Constraint::NodeByExpression {
            expr: NodeExpr::CustomCall {
                func: Arc::clone(func),
                args: vec![(ArgKey::Index(0), NodeExpr::This)],
            },
            shapes: Arc::new(OnceLock::new()),
            messages: vec![Literal::new_simple_literal("shape")],
            severity: None,
        },
        Constraint::Component {
            component: ex("MyConstraintComponent"),
            source_shape: ex_term("SourceShape"),
            bindings: vec![("limit".to_owned(), ex_term("ten"))],
            validator: ComponentValidator::Ask {
                ask: "ASK { $this ?p ?o }".to_owned(),
            },
            messages: vec![],
            severity: None,
            // No variable: only the default applies.
            annotations: vec![crate::shapes::ResultAnnotation {
                property: ex("origin"),
                variable: None,
                default_values: vec![ex_term("component")],
            }],
        },
        Constraint::MinListLength(1),
        Constraint::MaxListLength(4),
        Constraint::UniqueMembers(true),
        Constraint::MemberShape(Box::new(leaf_shape("MemberShape"))),
        Constraint::SingleLine(true),
        Constraint::RootClass(vec![ex("RootA"), ex("RootB")]),
        Constraint::SomeValue(Box::new(leaf_shape("SomeValue"))),
        Constraint::SubsetOf(Path::Alternative(vec![
            Path::Predicate(ex("p")),
            Path::ZeroOrOne(Box::new(Path::Predicate(ex("q")))),
        ])),
        Constraint::UniqueValuesFor {
            properties: vec![ex("notation"), ex("scheme")],
            shape: ex_term("SchemeShape"),
            targets: vec![
                Target::Class(ex("Concept")),
                Target::SubjectsOf(ex("notation")),
            ],
        },
    ]
}

// ── Codec harness ───────────────────────────────────────────────────────────────

/// A writer whose declaration index holds exactly the fixture function.
fn test_writer() -> AstWriter {
    let mut index = BTreeMap::new();
    index.insert(fixture_fn_iri(), 0u64);
    AstWriter::new(index)
}

/// A reader whose declaration table holds exactly the fixture function.
fn test_reader(bytes: &[u8]) -> AstReader<'_> {
    let mut reader = AstReader::new(bytes);
    reader.functions = vec![fixture_fn()];
    reader
}

/// Decode `bytes` with one [`AstReader`] method, answering the value and how many
/// bytes it consumed.
///
/// The indirection through a byte slice is deliberate: a bound written over
/// `&mut AstReader<'_>` would quantify over the reader's OWN borrow lifetime as
/// well as the closure's, and no method item satisfies that. Taking the bytes
/// keeps exactly one lifetime in play.
macro_rules! reader_fn {
    ($method:ident) => {
        |bytes: &[u8]| {
            let mut reader = test_reader(bytes);
            let value = reader.$method()?;
            Ok((value, reader.pos))
        }
    };
}

/// Decode `bytes` with one [`AstReader`] method, discarding the value.
macro_rules! reader_probe {
    ($method:ident) => {
        Box::new(|bytes: &[u8]| {
            let mut reader = test_reader(bytes);
            reader.$method()?;
            Ok(())
        })
    };
}

/// Drive one tag space through both halves of RULE 1's behavioural check.
///
/// The VALID half: every declared tag encodes to its own index, decodes, consumes
/// exactly its bytes, and re-encodes byte-identically. The INVALID half, run
/// immediately after on the same decoder: the first tag past the declared space
/// is refused as an unsupported capability rather than panicking or being skipped.
fn roundtrip_tags<T>(
    label: &str,
    declared: u8,
    samples: &[T],
    write: impl Fn(&mut AstWriter, &T) -> Result<(), ShapesProductError>,
    read: impl Fn(&[u8]) -> Result<(T, usize), ShapesProductError>,
) {
    assert_eq!(
        samples.len(),
        usize::from(declared),
        "{label}: the sample vector must carry exactly one value per declared tag",
    );

    for (index, sample) in samples.iter().enumerate() {
        let mut writer = test_writer();
        write(&mut writer, sample).unwrap_or_else(|error| panic!("{label}[{index}]: {error}"));
        let bytes = writer.out;
        assert_eq!(
            bytes.first().copied(),
            Some(index as u8),
            "{label}[{index}] must be written under tag {index}: the sample vector is the tag \
             declaration, so a sample in the wrong slot is a tag space that does not say what the \
             codec does",
        );

        let (value, consumed) =
            read(&bytes).unwrap_or_else(|error| panic!("{label}[{index}]: {error}"));
        assert_eq!(
            consumed,
            bytes.len(),
            "{label}[{index}] must consume exactly the bytes it was written as",
        );

        let mut again = test_writer();
        write(&mut again, &value).expect("a decoded value re-encodes");
        assert_eq!(
            again.out, bytes,
            "{label}[{index}] is not byte-stable across a round trip",
        );
    }

    let bytes = [declared];
    let error = read(&bytes).err().unwrap_or_else(|| {
        panic!("{label}: tag {declared} is past the declared space and must be refused")
    });
    assert_eq!(
        error.dimension(),
        ProductDimension::UnsupportedCapability,
        "{label}: an unknown tag names a capability this build lacks, not a malformed byte \
         string: {error}",
    );
}

/// A `Shapes` carrying `node_shapes` and nothing else caller-supplied.
fn shapes_of(node_shapes: Vec<Shape>) -> Shapes {
    Shapes {
        node_shapes,
        ..Shapes::default()
    }
}

/// Encode, decode, and re-encode from the decoded parts.
///
/// The returned pair is `(original bytes, re-encoded bytes)`. [`Shape`],
/// [`Constraint`], [`Path`] and [`NodeExpr`] are deliberately not `PartialEq`, so
/// the bytes ARE the equality relation — see this module's parent documentation.
fn round_trip(shapes: &Shapes) -> (Vec<u8>, Vec<u8>, Shapes) {
    let bytes = encode_ast_derived(shapes).expect("the fixture encodes");
    let parts = decode_ast(&bytes).expect("the fixture decodes");
    let rebuilt = Shapes {
        node_shapes: parts.node_shapes,
        box_role_vocab: parts.box_role_vocab,
        target_types: parts.target_types,
        shapes_graph: parts.shapes_graph,
        ..Shapes::default()
    };
    let again = encode_ast_derived(&rebuilt).expect("the decoded parts re-encode");
    (bytes, again, rebuilt)
}

/// The whole-model fixture: every [`Constraint`] variant, every [`NodeExpr`]
/// variant, every [`Target`] and [`Path`] form, both rule bodies, both rule
/// schedules, and a self-recursive custom function.
fn full_fixture() -> Shapes {
    let func = fixture_recursive_fn();

    let property = PropertyShape {
        id: ex_term("PropertyShape"),
        path: Path::Sequence(sample_paths()),
        values: None,
        default_value: None,
        constraints: sample_constraints(&func),
        property_shapes: vec![PropertyShape {
            id: ex_term("NestedProperty"),
            path: Path::Predicate(ex("nested")),
            values: Some(NodeExpr::Path(Path::Predicate(ex("computed")))),
            default_value: Some(NodeExpr::Constant(Term::Literal(
                Literal::new_simple_literal("fallback"),
            ))),
            constraints: vec![Constraint::MinCount(1)],
            property_shapes: Vec::new(),
            reifier_shapes: Vec::new(),
            reification_required: false,
            severity: Severity::Warning,
            messages: vec![Literal::new_simple_literal("nested")],
            constraint_annotations: vec![],
            deactivated: true,
            box_roles: vec![ex("role/abox")],
        }],
        reifier_shapes: vec![leaf_shape("ReifierShape")],
        reification_required: true,
        severity: Severity::Other(ex("Critical")),
        messages: vec![],
        constraint_annotations: vec![
            ConstraintAnnotation {
                constraint: AnnotatedConstraint::Constraint(0),
                severity: Some(Severity::Debug),
                messages: vec![
                    Literal::new_language_tagged_literal_unchecked("erste", "de"),
                    Literal::new_simple_literal("first"),
                ],
            },
            ConstraintAnnotation {
                constraint: AnnotatedConstraint::Constraint(2),
                severity: None,
                messages: vec![Literal::new_simple_literal("third")],
            },
            ConstraintAnnotation {
                constraint: AnnotatedConstraint::Reifier,
                severity: Some(Severity::Trace),
                messages: vec![],
            },
        ],
        deactivated: false,
        box_roles: vec![ex("role/tbox"), ex("role/rbox")],
    };

    let expression_constraints: Vec<Constraint> = sample_node_exprs(&func)
        .into_iter()
        .map(|expr| Constraint::Expression {
            expr,
            messages: vec![],
            severity: None,
        })
        .collect();

    let call_constraints: Vec<Constraint> = sample_fn_calls()
        .into_iter()
        .map(|call| Constraint::Expression {
            expr: NodeExpr::Call(call),
            messages: vec![],
            severity: None,
        })
        .collect();

    let arg_constraints: Vec<Constraint> = sample_shape_args()
        .into_iter()
        .map(|shape| Constraint::Expression {
            expr: NodeExpr::ConformsToShape {
                node: Box::new(NodeExpr::This),
                shape,
            },
            messages: vec![],
            severity: None,
        })
        .collect();

    let literal_constraints: Vec<Constraint> = sample_literals()
        .into_iter()
        .map(|literal| Constraint::HasValue(Term::Literal(literal)))
        .collect();

    let term_constraints: Vec<Constraint> = sample_terms()
        .into_iter()
        .map(Constraint::HasValue)
        .collect();

    let kind_constraints: Vec<Constraint> = sample_node_kinds()
        .into_iter()
        .map(|kind| Constraint::NodeKind(vec![kind]))
        .collect();

    let validator_constraints: Vec<Constraint> = sample_component_validators()
        .into_iter()
        .map(|validator| Constraint::Component {
            component: ex("MyConstraintComponent"),
            source_shape: ex_term("SourceShape"),
            bindings: Vec::new(),
            validator,
            messages: vec![],
            severity: None,
            annotations: vec![],
        })
        .collect();

    let severity_constraints: Vec<Constraint> = sample_severities()
        .into_iter()
        .map(|severity| Constraint::Sparql {
            select: "SELECT $this WHERE { $this ?p ?o }".to_owned(),
            messages: vec![],
            severity: Some(severity),
            annotations: vec![],
        })
        .collect();

    let arg_key_constraints: Vec<Constraint> = sample_arg_keys()
        .into_iter()
        .map(|key| Constraint::Expression {
            expr: NodeExpr::Arg(key),
            messages: vec![],
            severity: None,
        })
        .collect();

    let mut rules = Vec::new();
    for (index, body) in sample_rule_bodies().into_iter().enumerate() {
        for (offset, run_once) in [true, false].into_iter().enumerate() {
            rules.push(Rule {
                id: ex_term(&format!("rule/{index}-{offset}")),
                body: body.clone(),
                conditions: vec![leaf_shape("Condition")],
                layer: if offset == 1 {
                    Some(OrderKey::new(3.0))
                } else {
                    None
                },
                order: if offset == 0 {
                    Some(OrderKey::new(1.5))
                } else {
                    None
                },
                run_once,
                deactivated: offset == 1,
                expected_predicates: if offset == 0 {
                    vec![ex("expected")]
                } else {
                    Vec::new()
                },
                processors: if offset == 0 {
                    vec![ex_term("processor")]
                } else {
                    Vec::new()
                },
            });
        }
    }

    let mut constraints = sample_constraints(&func);
    constraints.extend(expression_constraints);
    constraints.extend(call_constraints);
    constraints.extend(arg_constraints);
    constraints.extend(literal_constraints);
    constraints.extend(term_constraints);
    constraints.extend(kind_constraints);
    constraints.extend(validator_constraints);
    constraints.extend(severity_constraints);
    constraints.extend(arg_key_constraints);

    let shape = Shape {
        id: ex_term("EverythingShape"),
        targets: sample_targets(),
        constraints,
        property_shapes: vec![property],
        severity: Severity::Info,
        messages: vec![Literal::new_simple_literal("everything")],
        constraint_annotations: vec![ConstraintAnnotation {
            constraint: AnnotatedConstraint::Constraint(1),
            severity: Some(Severity::Other(ex("Critical"))),
            messages: vec![],
        }],
        deactivated: false,
        box_roles: vec![ex("role/cbox")],
        rules,
    };

    let mut target_types = BTreeMap::new();
    target_types.insert(
        ex("TargetTypeB").as_str().to_owned(),
        SparqlTargetType {
            id: ex_term("TargetTypeB"),
            params: Vec::new(),
            select: "SELECT ?this WHERE { ?this a ?class }".to_owned(),
        },
    );
    target_types.insert(
        ex("TargetTypeA").as_str().to_owned(),
        SparqlTargetType {
            id: ex_term("TargetTypeA"),
            params: vec![
                TargetTypeParam {
                    predicate: ex("class"),
                    var: "class".to_owned(),
                },
                TargetTypeParam {
                    predicate: ex("depth"),
                    var: "depth".to_owned(),
                },
            ],
            select: "SELECT ?this WHERE { ?this a $class . ?this <https://example.org/d> $depth }"
                .to_owned(),
        },
    );

    Shapes {
        node_shapes: vec![shape, leaf_shape("Second")],
        box_role_vocab: Some(BoxRoleVocab::for_namespace("https://example.org/box#")),
        target_types,
        shapes_graph: Some("https://example.org/shapes".to_owned()),
        ..Shapes::default()
    }
}

// ── Tag spaces: every known tag decodes, the first unknown one refuses ──────────

/// Each tagged type's VALID case (every declared tag encodes, decodes and
/// re-encodes byte-identically), paired with its INVALID one (the first tag past
/// the space is an `unsupported-capability` refusal).
#[test]
fn every_known_tag_decodes() {
    let func = fixture_fn();

    roundtrip_tags(
        "Term",
        TAGS_TERM,
        &sample_terms(),
        AstWriter::term,
        reader_fn!(term),
    );
    roundtrip_tags(
        "Severity",
        TAGS_SEVERITY,
        &sample_severities(),
        |writer, value| {
            writer.severity(value);
            Ok(())
        },
        reader_fn!(severity),
    );
    roundtrip_tags(
        "NodeKindValue",
        TAGS_NODE_KIND,
        &sample_node_kinds(),
        |writer, value| {
            writer.node_kind(value);
            Ok(())
        },
        reader_fn!(node_kind),
    );
    roundtrip_tags(
        "Path",
        TAGS_PATH,
        &sample_paths(),
        AstWriter::path,
        reader_fn!(path),
    );
    roundtrip_tags(
        "Target",
        TAGS_TARGET,
        &sample_targets(),
        AstWriter::target,
        reader_fn!(target),
    );
    roundtrip_tags(
        "ComponentValidator",
        TAGS_COMPONENT_VALIDATOR,
        &sample_component_validators(),
        |writer, value| {
            writer.component_validator(value);
            Ok(())
        },
        reader_fn!(component_validator),
    );
    roundtrip_tags(
        "ArgKey",
        TAGS_ARG_KEY,
        &sample_arg_keys(),
        |writer, value| {
            writer.arg_key(value);
            Ok(())
        },
        reader_fn!(arg_key),
    );
    roundtrip_tags(
        "CustomFnKind",
        TAGS_CUSTOM_FN_KIND,
        &sample_custom_fn_kinds(),
        |writer, value| {
            writer.custom_fn_kind(*value);
            Ok(())
        },
        reader_fn!(custom_fn_kind),
    );
    roundtrip_tags(
        "RuleBody",
        TAGS_RULE_BODY,
        &sample_rule_bodies(),
        AstWriter::rule_body,
        reader_fn!(rule_body),
    );
    roundtrip_tags(
        "ShapeArg",
        TAGS_SHAPE_ARG,
        &sample_shape_args(),
        AstWriter::shape_arg,
        reader_fn!(shape_arg),
    );
    roundtrip_tags(
        "FnCall",
        TAGS_FN_CALL,
        &sample_fn_calls(),
        AstWriter::fn_call,
        reader_fn!(fn_call),
    );
    roundtrip_tags(
        "NodeExpr",
        TAGS_NODE_EXPR,
        &sample_node_exprs(&func),
        AstWriter::node_expr,
        reader_fn!(node_expr),
    );
    roundtrip_tags(
        "Constraint",
        TAGS_CONSTRAINT,
        &sample_constraints(&func),
        AstWriter::constraint,
        reader_fn!(constraint),
    );
    roundtrip_tags(
        "ClosedMode",
        TAGS_CLOSED_MODE,
        &sample_closed_modes(),
        AstWriter::closed_mode,
        reader_fn!(closed_mode),
    );
}

/// The invalid neighbour of every case above, stated on its own so the refusal is
/// visible as a named test rather than only as the tail of a loop.
#[test]
fn unknown_variant_tag_is_unsupported_capability() {
    // One tag past each declared space, through the same decoders the valid cases
    // above drive.
    type Probe = Box<dyn Fn(&[u8]) -> Result<(), ShapesProductError>>;
    let probes: Vec<(&str, u8, Probe)> = vec![
        ("Term", TAGS_TERM, reader_probe!(term)),
        ("Severity", TAGS_SEVERITY, reader_probe!(severity)),
        ("NodeKindValue", TAGS_NODE_KIND, reader_probe!(node_kind)),
        ("Path", TAGS_PATH, reader_probe!(path)),
        ("Target", TAGS_TARGET, reader_probe!(target)),
        ("Constraint", TAGS_CONSTRAINT, reader_probe!(constraint)),
        ("ClosedMode", TAGS_CLOSED_MODE, reader_probe!(closed_mode)),
        ("NodeExpr", TAGS_NODE_EXPR, reader_probe!(node_expr)),
        ("FnCall", TAGS_FN_CALL, reader_probe!(fn_call)),
        ("ShapeArg", TAGS_SHAPE_ARG, reader_probe!(shape_arg)),
        ("ArgKey", TAGS_ARG_KEY, reader_probe!(arg_key)),
        ("RuleBody", TAGS_RULE_BODY, reader_probe!(rule_body)),
        (
            "ComponentValidator",
            TAGS_COMPONENT_VALIDATOR,
            reader_probe!(component_validator),
        ),
        (
            "CustomFnKind",
            TAGS_CUSTOM_FN_KIND,
            reader_probe!(custom_fn_kind),
        ),
    ];

    for (label, tag, decode) in probes {
        let bytes = [tag];
        let error = decode(&bytes)
            .err()
            .unwrap_or_else(|| panic!("{label}: tag {tag} must be refused"));
        assert_eq!(
            error.dimension(),
            ProductDimension::UnsupportedCapability,
            "{label}: {error}",
        );
        assert!(
            error.message().contains("prepare the product")
                || error.message().contains("re-prepare the product"),
            "{label}: the refusal must name the fix: {error}",
        );
    }

    // The SAME shape at the whole-product level, so the refusal is reachable from
    // the public surface and not only from the private decoders.
    let bytes =
        encode_ast_derived(&shapes_of(vec![leaf_shape("Only")])).expect("the fixture encodes");
    decode_ast(&bytes).expect("the unmodified product decodes");
    let mut corrupted = bytes;
    // Byte 0 is the declaration count (0), byte 1 the node-shape count (1), byte 2
    // the first shape's id TERM tag — raise it past the tag space.
    corrupted[2] = TAGS_TERM;
    let error = decode_ast(&corrupted).expect_err("an unknown term tag must be refused");
    assert_eq!(error.dimension(), ProductDimension::UnsupportedCapability);
}

// ── The equality witness ────────────────────────────────────────────────────────

/// `encode(decode(b)) == b` over a fixture exercising every [`Constraint`] and
/// every [`NodeExpr`] variant.
#[test]
fn encode_decode_encode_is_identity() {
    let shapes = full_fixture();
    let (bytes, again, rebuilt) = round_trip(&shapes);
    assert!(
        bytes.len() > 1_000,
        "the fixture is substantial: {} bytes",
        bytes.len()
    );
    assert_eq!(
        bytes, again,
        "the bytes ARE the equality relation these types do not implement; a difference here is a \
         field the decoder did not restore or the encoder did not write",
    );

    // Encoding is a pure function of value content: the second pass over the
    // REBUILT model reproduces the first, and a third over the same model
    // reproduces it again.
    let (third, fourth, _) = round_trip(&rebuilt);
    assert_eq!(third, bytes);
    assert_eq!(fourth, bytes);

    // The fixture is not vacuous: it really does carry every arm.
    assert_eq!(
        sample_constraints(&fixture_fn()).len(),
        usize::from(TAGS_CONSTRAINT)
    );
    assert_eq!(
        sample_node_exprs(&fixture_fn()).len(),
        usize::from(TAGS_NODE_EXPR)
    );
}

/// Every lexical shape of an RDF literal survives, and the one impossible shape
/// is refused.
#[test]
fn literal_shapes_round_trip() {
    for (index, literal) in sample_literals().into_iter().enumerate() {
        let datatype = literal.datatype_str().to_owned();
        let language = literal.language().map(ToOwned::to_owned);
        let direction = literal.direction();

        let mut writer = test_writer();
        writer.literal(&literal);
        let bytes = writer.out;
        let mut reader = test_reader(&bytes);
        let decoded = reader
            .literal()
            .unwrap_or_else(|error| panic!("[{index}]: {error}"));

        assert_eq!(decoded.value(), literal.value());
        assert_eq!(decoded.datatype_str(), datatype);
        assert_eq!(decoded.language().map(ToOwned::to_owned), language);
        assert_eq!(decoded.direction(), direction);
    }

    // The invalid neighbour: a base direction with no language tag is not an RDF
    // 1.2 term, and admitting it would mint one.
    let mut writer = test_writer();
    writer.text("value");
    writer.text("https://example.org/Datatype");
    writer.flag(false); // no language
    writer.flag(true); // but a direction
    writer.tag(0);
    let bytes = writer.out;
    let mut reader = test_reader(&bytes);
    let error = reader
        .literal()
        .expect_err("a direction without a language must be refused");
    assert_eq!(error.dimension(), ProductDimension::Malformed);

    // The valid neighbour, one byte away: the same literal WITH its language tag.
    let mut writer = test_writer();
    writer.literal(&Literal::new_directional_language_tagged_literal_unchecked(
        "value",
        "en",
        ::purrdf::RdfTextDirection::Ltr,
    ));
    let bytes = writer.out;
    let mut reader = test_reader(&bytes);
    let decoded = reader.literal().expect("a directional literal decodes");
    assert_eq!(decoded.language(), Some("en"));
}

// ── Bounded recursion ───────────────────────────────────────────────────────────

/// A path of `levels` nested `sh:inversePath` wrappers around one predicate.
fn inverse_chain(levels: usize) -> Path {
    let mut path = Path::Predicate(ex("p"));
    for _ in 0..levels {
        path = Path::Inverse(Box::new(path));
    }
    path
}

/// The bytes of an inverse chain, built without the encoder so the DECODER's
/// ceiling can be probed past the point the encoder would write.
fn inverse_chain_bytes(levels: usize) -> Vec<u8> {
    let mut bytes = vec![1u8; levels];
    bytes.push(0);
    let iri = ex("p");
    bytes.push(u8::try_from(iri.as_str().len()).expect("the fixture IRI is short"));
    bytes.extend_from_slice(iri.as_str().as_bytes());
    bytes
}

/// A node shape nested `levels` deep through `sh:node`.
fn nested_shape(levels: usize) -> Shape {
    let mut shape = leaf_shape("Deep");
    for _ in 0..levels {
        shape = Shape {
            constraints: vec![Constraint::Node(Box::new(shape))],
            ..leaf_shape("Deep")
        };
    }
    shape
}

/// Past the ceiling, BOTH directions refuse with a typed error instead of driving
/// the native stack into an abort no caller could have handled.
#[test]
fn deep_nesting_refuses_with_depth_limit() {
    let ceiling = usize::try_from(MAX_DEPTH).expect("the ceiling fits in a usize");

    // Encoding: `ceiling` wrappers is `ceiling + 1` nested frames.
    let mut writer = test_writer();
    let error = writer
        .path(&inverse_chain(ceiling))
        .expect_err("encoding past the ceiling must refuse");
    assert_eq!(error.dimension(), ProductDimension::DepthLimit);
    assert!(
        error.message().contains("flatten the shapes graph"),
        "the refusal must name the fix: {error}",
    );

    // Decoding the same shape, driven from bytes the encoder would never write.
    let bytes = inverse_chain_bytes(ceiling);
    let mut reader = test_reader(&bytes);
    let error = reader
        .path()
        .expect_err("decoding past the ceiling must refuse");
    assert_eq!(error.dimension(), ProductDimension::DepthLimit);

    // And from the public surface, over the mutually recursive shape/constraint
    // cycle rather than a single-type chain.
    let deep = shapes_of(vec![nested_shape(4 * ceiling)]);
    let error = encode_ast_derived(&deep).expect_err("a deeply nested shapes graph must refuse");
    assert_eq!(error.dimension(), ProductDimension::DepthLimit);
}

/// The neighbouring VALID case: nesting up to and including the ceiling still
/// encodes, decodes and re-encodes byte-identically.
///
/// This is the half that matters most. A ceiling that refused one level early
/// would leave every test above green while quietly rejecting shapes graphs that
/// are perfectly legal — the mirror of a silent drop, and far harder to see.
#[test]
fn nesting_one_below_the_ceiling_decodes() {
    let ceiling = usize::try_from(MAX_DEPTH).expect("the ceiling fits in a usize");

    for levels in [ceiling - 2, ceiling - 1] {
        let path = inverse_chain(levels);
        let mut writer = test_writer();
        writer
            .path(&path)
            .unwrap_or_else(|error| panic!("{levels} levels must encode: {error}"));
        let bytes = writer.out;
        assert_eq!(bytes, inverse_chain_bytes(levels));

        let mut reader = test_reader(&bytes);
        let decoded = reader
            .path()
            .unwrap_or_else(|error| panic!("{levels} levels must decode: {error}"));
        assert_eq!(reader.pos, bytes.len());

        let mut again = test_writer();
        again.path(&decoded).expect("a decoded path re-encodes");
        assert_eq!(again.out, bytes);
    }

    // The public surface too, with the mutual shape/constraint recursion.
    let shapes = shapes_of(vec![nested_shape(10)]);
    let (bytes, round_tripped, _) = round_trip(&shapes);
    assert_eq!(bytes, round_tripped);
}

// ── `sh:order` canonicality ─────────────────────────────────────────────────────

/// A shapes graph whose single rule carries `order`.
fn shapes_with_order(order: Option<OrderKey>) -> Shapes {
    shapes_of(vec![Shape {
        rules: vec![Rule {
            id: ex_term("rule"),
            body: RuleBody::Sparql {
                construct: "CONSTRUCT { $this <https://example.org/p> 1 } WHERE {}".to_owned(),
                parameters: Vec::new(),
            },
            conditions: Vec::new(),
            layer: None,
            order,
            run_once: false,
            deactivated: false,
            expected_predicates: Vec::new(),
            processors: Vec::new(),
        }],
        ..leaf_shape("Ordered")
    }])
}

/// `-0.0` and `+0.0` are one number, so they must be one byte form.
#[test]
fn order_key_negative_zero_is_canonical() {
    let negative = encode_ast_derived(&shapes_with_order(Some(OrderKey::new(-0.0))))
        .expect("a negative-zero order encodes");
    let positive = encode_ast_derived(&shapes_with_order(Some(OrderKey::new(0.0))))
        .expect("a zero order encodes");
    assert_eq!(
        negative, positive,
        "`-0.0` and `+0.0` are two IEEE-754 encodings of ONE number; two byte forms would make one \
         shapes graph two products, and the scheduler makes equal orders mean one stratum",
    );

    let parts = decode_ast(&negative).expect("it decodes");
    let value = parts.node_shapes[0].rules[0]
        .order
        .expect("the order survives")
        .value();
    assert!(value == 0.0 && !value.is_sign_negative(), "got {value}");

    // Neighbouring values are still distinct — canonicalizing zero must not
    // collapse anything else.
    let one = encode_ast_derived(&shapes_with_order(Some(OrderKey::new(1.0)))).expect("encodes");
    let minus_one =
        encode_ast_derived(&shapes_with_order(Some(OrderKey::new(-1.0)))).expect("encodes");
    assert_ne!(one, minus_one);
    assert_ne!(one, positive);

    // Absent encodes distinctly from present-and-zero.
    let absent = encode_ast_derived(&shapes_with_order(None)).expect("encodes");
    assert_ne!(absent, positive);
    assert!(
        decode_ast(&absent).expect("decodes").node_shapes[0].rules[0]
            .order
            .is_none()
    );

    // The invalid neighbour: a key with no ordering value at all.
    let error = encode_ast_derived(&shapes_with_order(Some(OrderKey::new(f64::NAN))))
        .expect_err("a non-finite order must be refused");
    assert_eq!(error.dimension(), ProductDimension::Malformed);
    let error = encode_ast_derived(&shapes_with_order(Some(OrderKey::new(f64::INFINITY))))
        .expect_err("a non-finite order must be refused");
    assert_eq!(error.dimension(), ProductDimension::Malformed);
}

// ── The cyclic function graph ───────────────────────────────────────────────────

/// A self-recursive body encodes FINITELY, as a DAG of indices.
#[test]
fn custom_function_cycle_encodes_as_dag() {
    let func = fixture_recursive_fn();
    let shapes = shapes_of(vec![Shape {
        constraints: vec![
            Constraint::Expression {
                expr: NodeExpr::CustomCall {
                    func: Arc::clone(&func),
                    args: vec![(ArgKey::Index(0), NodeExpr::This)],
                },
                messages: vec![],
                severity: None,
            },
            // A SECOND call site of the SAME declaration: the table must still
            // hold one entry, and both sites must reference it by index.
            Constraint::Expression {
                expr: NodeExpr::CustomCall {
                    func: Arc::clone(&func),
                    args: vec![(ArgKey::Index(1), NodeExpr::Empty)],
                },
                messages: vec![],
                severity: None,
            },
        ],
        ..leaf_shape("Recursive")
    }]);

    // Terminating at all is the claim: a tree walk over this value would recurse
    // until the stack was gone.
    let bytes = encode_ast_derived(&shapes).expect("a cyclic function graph encodes");
    let parts = decode_ast(&bytes).expect("it decodes");

    assert_eq!(
        parts.custom_functions.len(),
        1,
        "two call sites of one declaration must share ONE table entry",
    );
    let decoded = &parts.custom_functions[0];
    assert_eq!(decoded.iri.as_str(), func.iri.as_str());
    assert_eq!(decoded.kind, func.kind);
    assert_eq!(decoded.params, func.params);
    assert_eq!(decoded.required, func.required);

    // The cycle is restored, not flattened: the decoded body calls the decoded
    // function itself, through the very same handle.
    let body = decoded.body().expect("the body is installed");
    match body {
        NodeExpr::CustomCall { func: callee, .. } => assert!(
            Arc::ptr_eq(callee, decoded),
            "the body must call the SAME handle, not a copy: a copy would give the recursion a \
             second, divergent definition",
        ),
        other => panic!("the body is not the call it was written as: {other:?}"),
    }

    let rebuilt = Shapes {
        node_shapes: parts.node_shapes,
        ..Shapes::default()
    };
    assert_eq!(
        encode_ast_derived(&rebuilt).expect("re-encodes"),
        bytes,
        "the DAG is byte-stable",
    );

    // A declaration whose body was never installed is a distinct, legal state.
    let bodiless = shapes_of(vec![Shape {
        constraints: vec![Constraint::Expression {
            expr: NodeExpr::CustomCall {
                func: fixture_fn(),
                args: Vec::new(),
            },
            messages: vec![],
            severity: None,
        }],
        ..leaf_shape("Bodiless")
    }]);
    let bytes = encode_ast_derived(&bodiless).expect("a bodiless declaration encodes");
    let parts = decode_ast(&bytes).expect("it decodes");
    assert!(parts.custom_functions[0].body.get().is_none());
    assert_ne!(bytes, encode_ast_derived(&shapes).expect("encodes"));
}

/// Two SEPARATE declarations of one function IRI are refused rather than unified,
/// paired with the valid case they are one byte away from.
#[test]
fn two_declarations_of_one_function_iri_are_refused() {
    let call = |func: Arc<CustomFunction>| Constraint::Expression {
        expr: NodeExpr::CustomCall {
            func,
            args: Vec::new(),
        },
        messages: vec![],
        severity: None,
    };

    // INVALID: two handles, same IRI. Picking one would give half the call sites
    // a body they were never written against.
    let first = fixture_fn();
    let second = fixture_fn();
    assert!(!Arc::ptr_eq(&first, &second));
    let clashing = shapes_of(vec![Shape {
        constraints: vec![call(first), call(second)],
        ..leaf_shape("Clashing")
    }]);
    let error =
        encode_ast_derived(&clashing).expect_err("two declarations of one IRI must be refused");
    assert_eq!(error.dimension(), ProductDimension::Malformed);

    // VALID: the same two call sites sharing ONE declaration — which is what the
    // parser actually produces, and what a refusal here must not touch.
    let shared = fixture_fn();
    let sharing = shapes_of(vec![Shape {
        constraints: vec![call(Arc::clone(&shared)), call(shared)],
        ..leaf_shape("Sharing")
    }]);
    let bytes = encode_ast_derived(&sharing).expect("shared declarations encode");
    assert_eq!(
        decode_ast(&bytes).expect("decodes").custom_functions.len(),
        1
    );
}

// ── `sh:SPARQLTargetType` ───────────────────────────────────────────────────────

/// A target type restores with byte-identical `select` text and parameter order.
#[test]
fn sparql_target_type_round_trips() {
    let shapes = full_fixture();
    let bytes = encode_ast_derived(&shapes).expect("encodes");
    let parts = decode_ast(&bytes).expect("decodes");

    assert_eq!(parts.target_types.len(), 2);
    let key = ex("TargetTypeA").as_str().to_owned();
    let original = &shapes.target_types[&key];
    let decoded = &parts.target_types[&key];

    assert_eq!(
        decoded.select, original.select,
        "the query text is byte-identical"
    );
    assert_eq!(decoded.id.to_string(), original.id.to_string());
    let decoded_params: Vec<(&str, &str)> = decoded
        .params
        .iter()
        .map(|param| (param.predicate.as_str(), param.var.as_str()))
        .collect();
    assert_eq!(
        decoded_params,
        vec![
            ("https://example.org/class", "class"),
            ("https://example.org/depth", "depth"),
        ],
        "parameter ORDER is the call order of the target type and must not be re-sorted",
    );

    // The table is written key-sorted, not in insertion order: the fixture inserts
    // `TargetTypeB` first, and the bytes must not say so.
    let mut reversed = BTreeMap::new();
    for (iri, target_type) in &shapes.target_types {
        reversed.insert(iri.clone(), target_type.clone());
    }
    let same = Shapes {
        target_types: reversed,
        ..full_fixture()
    };
    assert_eq!(encode_ast_derived(&same).expect("encodes"), bytes);

    // A target type with no parameters is a distinct, legal declaration.
    let empty_key = ex("TargetTypeB").as_str().to_owned();
    assert!(parts.target_types[&empty_key].params.is_empty());
}

// ── Rules ───────────────────────────────────────────────────────────────────────

/// A run-once rule and an iterating rule survive a round trip, and are distinguishable.
#[test]
fn run_once_round_trips() {
    let mut encodings = Vec::new();
    for run_once in [true, false] {
        let shapes = shapes_of(vec![Shape {
            rules: vec![Rule {
                id: ex_term("rule"),
                body: RuleBody::Sparql {
                    construct: "CONSTRUCT { $this <https://example.org/p> 1 } WHERE {}".to_owned(),
                    parameters: Vec::new(),
                },
                conditions: vec![leaf_shape("Condition")],
                layer: Some(OrderKey::new(1.0)),
                order: Some(OrderKey::new(2.0)),
                run_once,
                deactivated: false,
                expected_predicates: Vec::new(),
                processors: Vec::new(),
            }],
            ..leaf_shape("Scheduled")
        }]);
        let bytes = encode_ast_derived(&shapes).expect("encodes");
        let parts = decode_ast(&bytes).expect("decodes");
        assert_eq!(
            parts.node_shapes[0].rules[0].run_once, run_once,
            "a run-once rule and an iterating rule run on different schedules; collapsing them \
             would change what the inferences contain",
        );
        encodings.push(bytes);
    }
    assert_ne!(
        encodings[0], encodings[1],
        "the two schedules must not share a byte form",
    );
}

/// The global rules, the rule sets and the rules entailment declaration round-trip.
#[test]
fn rule_graph_round_trips() {
    let shapes = Shapes {
        rules: RuleGraph {
            global_rules: vec![Rule {
                id: ex_term("global"),
                body: RuleBody::Triple {
                    subject: Some(NodeExpr::Constant(ex_term("s"))),
                    predicate: Some(NodeExpr::Constant(ex_term("p"))),
                    object: None,
                },
                conditions: Vec::new(),
                layer: Some(OrderKey::new(-1.0)),
                order: None,
                run_once: true,
                deactivated: false,
                expected_predicates: vec![ex("expected")],
                processors: vec![ex_term("processor")],
            }],
            rule_sets: vec![RuleSetDeclaration {
                id: ex("set"),
                rules: vec![ex_term("global")],
                includes: vec![ex("other-set")],
                processors: vec![ex_term("processor")],
            }],
            entailment: true,
        },
        ..shapes_of(vec![leaf_shape("S")])
    };
    let bytes = encode_ast_derived(&shapes).expect("encodes");
    let parts = decode_ast(&bytes).expect("decodes");
    assert_eq!(parts.rules.global_rules.len(), 1);
    assert!(parts.rules.global_rules[0].run_once);
    assert_eq!(
        parts.rules.global_rules[0].processors,
        [ex_term("processor")]
    );
    assert_eq!(parts.rules.rule_sets, shapes.rules.rule_sets);
    assert!(parts.rules.entailment);
    let without = Shapes {
        rules: RuleGraph::default(),
        ..shapes_of(vec![leaf_shape("S")])
    };
    assert_ne!(
        encode_ast_derived(&without).expect("encodes"),
        bytes,
        "the rule graph reaches the byte form"
    );
}

// ── The box-role vocabulary ─────────────────────────────────────────────────────

/// Absent encodes distinctly from present-and-empty: the box-role feature being
/// INACTIVE is not the same claim as it being active over empty IRIs.
#[test]
fn box_role_vocab_round_trips() {
    let absent = shapes_of(vec![leaf_shape("S")]);
    assert!(absent.box_role_vocab.is_none());

    let present_empty = Shapes {
        box_role_vocab: Some(BoxRoleVocab {
            graph_box_role: String::new(),
            box_abox: String::new(),
            box_tbox: String::new(),
            box_rbox: String::new(),
            box_cbox: String::new(),
            box_config_box: String::new(),
        }),
        ..shapes_of(vec![leaf_shape("S")])
    };
    let populated = Shapes {
        box_role_vocab: Some(BoxRoleVocab::for_namespace("https://example.org/box#")),
        ..shapes_of(vec![leaf_shape("S")])
    };

    let absent_bytes = encode_ast_derived(&absent).expect("encodes");
    let empty_bytes = encode_ast_derived(&present_empty).expect("encodes");
    let populated_bytes = encode_ast_derived(&populated).expect("encodes");

    assert_ne!(
        absent_bytes, empty_bytes,
        "`None` means the feature is inactive and PurRDF mints no vocabulary of its own; \
         `Some(empty)` means the caller supplied one whose terms happen to be empty. Collapsing \
         them would fabricate a default.",
    );
    assert_ne!(empty_bytes, populated_bytes);

    assert!(
        decode_ast(&absent_bytes)
            .expect("decodes")
            .box_role_vocab
            .is_none()
    );
    let empty = decode_ast(&empty_bytes)
        .expect("decodes")
        .box_role_vocab
        .expect("present");
    assert_eq!(empty.graph_box_role, "");
    assert_eq!(empty.box_config_box, "");

    let restored = decode_ast(&populated_bytes)
        .expect("decodes")
        .box_role_vocab
        .expect("present");
    assert_eq!(
        restored,
        BoxRoleVocab::for_namespace("https://example.org/box#")
    );
}

// ── `sparql:<NAME>` re-lowering ─────────────────────────────────────────────────

/// Every `SparqlCallForm` the §5 lowering table can produce is re-derived at
/// decode, byte-identically to what the parser rendered — and a name the table
/// does not answer is refused rather than guessed.
#[test]
fn sparql_call_forms_relower_byte_identically() {
    // One local name per `SparqlCallForm` variant, in the enum's declaration
    // order: Call, Infix, Prefix, Membership, Ebv.
    let names = ["strlen", "add", "unary-minus", "in", "ebv"];
    let arities = [1usize, 2, 1, 2, 1];

    for (name, arity) in names.iter().zip(arities) {
        let args: Vec<NodeExpr> = (0..arity).map(|_| NodeExpr::This).collect();
        let call = sparql_call(name, args);
        let rendered = match &call {
            FnCall::Sparql { expr, .. } => expr.clone(),
            other => panic!("the fixture built the wrong call: {other:?}"),
        };

        let mut writer = test_writer();
        writer.fn_call(&call).expect("encodes");
        let bytes = writer.out;
        let mut reader = test_reader(&bytes);
        let decoded = reader.fn_call().expect("decodes");
        match &decoded {
            FnCall::Sparql { expr, iri, args } => {
                assert_eq!(
                    *expr, rendered,
                    "`{name}` must re-lower to the text the parser rendered",
                );
                assert_eq!(iri.as_str(), format!("{}{name}", sparql_ns::NS));
                assert_eq!(args.len(), arity);
            }
            other => panic!("the decoder built the wrong call: {other:?}"),
        }

        let mut again = test_writer();
        again.fn_call(&decoded).expect("re-encodes");
        assert_eq!(again.out, bytes);
    }

    // The INVALID neighbour: a `sparql:` IRI the lowering table refuses (an
    // aggregate is not a scalar function) is an unsupported capability, not a
    // silently empty call.
    let mut writer = test_writer();
    writer.tag(2);
    writer.named_node(&NamedNode::new_unchecked(format!(
        "{}agg-sum",
        sparql_ns::NS
    )));
    writer.count(1);
    writer.node_expr(&NodeExpr::This).expect("encodes");
    let bytes = writer.out;
    let mut reader = test_reader(&bytes);
    let error = reader
        .fn_call()
        .expect_err("an unlowerable SPARQL name must be refused");
    assert_eq!(error.dimension(), ProductDimension::UnsupportedCapability);

    // …and an IRI outside the SPARQL 1.2 term vocabulary under the same tag.
    let mut writer = test_writer();
    writer.tag(2);
    writer.named_node(&ex("notSparql"));
    writer.count(0);
    let bytes = writer.out;
    let mut reader = test_reader(&bytes);
    let error = reader
        .fn_call()
        .expect_err("a non-`sparql:` IRI must be refused");
    assert_eq!(error.dimension(), ProductDimension::UnsupportedCapability);
}

/// The `NodeExpr::Select` key is a closed two-value set: both spellings survive,
/// and a third tag is an unsupported capability.
#[test]
fn select_key_round_trips_and_a_third_spelling_refuses() {
    for (tag, key) in [(0u8, "sh:select"), (1, "sh:sparqlExpr")] {
        let expr = NodeExpr::Select {
            query: "SELECT ?x WHERE {}".to_owned(),
            variable: "x".to_owned(),
            key,
        };
        let mut writer = test_writer();
        writer.node_expr(&expr).expect("encodes");
        let bytes = writer.out;
        assert_eq!(
            bytes.last().copied(),
            Some(tag),
            "the key is the trailing tag of a Select expression",
        );
        let mut reader = test_reader(&bytes);
        match reader.node_expr().expect("decodes") {
            NodeExpr::Select { key: decoded, .. } => assert_eq!(decoded, key),
            other => panic!("wrong arm: {other:?}"),
        }
    }

    // INVALID: a third tag names a spelling this build does not implement.
    let mut writer = test_writer();
    writer
        .node_expr(&NodeExpr::Select {
            query: "SELECT ?x WHERE {}".to_owned(),
            variable: "x".to_owned(),
            key: "sh:select",
        })
        .expect("encodes");
    let mut bytes = writer.out;
    let last = bytes.len() - 1;
    bytes[last] = 2;
    let mut reader = test_reader(&bytes);
    let error = reader.node_expr().expect_err("a third key must be refused");
    assert_eq!(error.dimension(), ProductDimension::UnsupportedCapability);

    // INVALID on the encode side too: a key literal this build does not know.
    let mut writer = test_writer();
    let error = writer
        .node_expr(&NodeExpr::Select {
            query: "SELECT ?x WHERE {}".to_owned(),
            variable: "x".to_owned(),
            key: "sh:somethingElse",
        })
        .expect_err("an unknown key spelling must be refused");
    assert_eq!(error.dimension(), ProductDimension::UnsupportedCapability);
}

// ── Structural refusals, each with its valid neighbour ──────────────────────────

/// Trailing bytes are refused: ignoring them is how a partial write passes for a
/// whole one.
#[test]
fn trailing_bytes_are_malformed() {
    let shapes = full_fixture();
    let bytes = encode_ast_derived(&shapes).expect("encodes");

    // VALID: the exact bytes.
    decode_ast(&bytes).expect("the exact byte string decodes");

    // INVALID: one byte more.
    let mut extended = bytes.clone();
    extended.push(0);
    let error = decode_ast(&extended).expect_err("trailing bytes must be refused");
    assert_eq!(error.dimension(), ProductDimension::Malformed);

    // INVALID: one byte fewer — a truncation, named as such.
    let truncated = &bytes[..bytes.len() - 1];
    let error = decode_ast(truncated).expect_err("a truncated product must be refused");
    assert!(
        matches!(
            error.dimension(),
            ProductDimension::Truncated | ProductDimension::Malformed
        ),
        "a short read is Truncated or Malformed, got {error}",
    );
}

/// A count larger than the bytes that could hold it is refused before it becomes
/// an allocation, and a HONEST large count is not.
#[test]
fn an_impossible_sequence_length_is_refused_but_an_honest_one_is_not() {
    // INVALID: a declaration table of `u64::MAX` entries in a four-byte product.
    let bytes = [0xffu8, 0xff, 0xff, 0x7f];
    let error = decode_ast(&bytes).expect_err("an impossible count must be refused");
    assert_eq!(error.dimension(), ProductDimension::Malformed);

    // VALID: a genuinely large-ish sequence that the bytes DO carry. A ceiling
    // that refused this would be over-refusal, not strictness.
    let shapes = shapes_of((0..512).map(|i| leaf_shape(&format!("S{i}"))).collect());
    let bytes = encode_ast_derived(&shapes).expect("encodes");
    assert_eq!(
        decode_ast(&bytes).expect("decodes").node_shapes.len(),
        512,
        "a large but honest sequence must load",
    );
}

/// A count the bytes DO permit is still not a licence to reserve memory for it.
///
/// [`AstReader::count`] bounds a declared count by the bytes that follow, which is
/// a bound in the wrong unit: an element encodes in as little as one byte but
/// occupies `size_of` the decoded value in memory, so a modest artifact can
/// honestly declare a sequence whose unclamped reservation is orders of magnitude
/// larger than the artifact itself. [`speculative_capacity`] is what closes that
/// gap, and this fixes it in place.
///
/// The valid neighbour matters as much as the refusal: the clamp caps the
/// SPECULATION, not the capacity, so a sequence longer than the clamp that the
/// stream genuinely carries must still decode in full. A clamp that truncated
/// would be a silent drop wearing the costume of a security bound.
#[test]
fn an_oversized_sequence_count_does_not_reserve_for_itself() {
    // The reservation rule itself: honest small counts are reserved exactly, and
    // nothing above the ceiling is reserved eagerly.
    assert_eq!(speculative_capacity(0), 0);
    assert_eq!(speculative_capacity(7), 7);
    assert_eq!(
        speculative_capacity(MAX_SPECULATIVE_ELEMENTS),
        MAX_SPECULATIVE_ELEMENTS
    );
    assert_eq!(
        speculative_capacity(usize::MAX),
        MAX_SPECULATIVE_ELEMENTS,
        "a declared count must never become the reservation it asks for",
    );

    // INVALID: a ~100 KB product whose top-level `node_shapes` count is 100_000.
    // `count()` accepts it — 100_000 elements really could be spelled in the
    // 100_000 bytes that follow — so the only thing standing between this byte
    // string and a `Vec<Shape>` reservation of 100_000 * size_of::<Shape>() is
    // the clamp. The elements are `0xff`, which is no `Term` tag this build
    // knows, so the very first one refuses.
    const HOSTILE: usize = 100_000;
    assert!(
        HOSTILE * size_of::<Shape>() > 16 * 1024 * 1024,
        "the fixture must describe a reservation large enough to be worth refusing",
    );
    let mut bytes = vec![0u8]; // field 1: an empty declaration table.
    let mut count = Vec::new();
    write_varint(&mut count, HOSTILE as u64);
    bytes.extend_from_slice(&count); // field 3: the node-shape count.
    bytes.resize(bytes.len() + HOSTILE, 0xff);
    let error = decode_ast(&bytes).expect_err("an unreadable element must refuse");
    assert_eq!(
        error.dimension(),
        ProductDimension::UnsupportedCapability,
        "0xff is not a Term tag this build implements, got {error}",
    );

    // VALID: a sequence LONGER than the clamp that the bytes really do carry.
    let honest = MAX_SPECULATIVE_ELEMENTS + 1;
    let shapes = shapes_of((0..honest).map(|i| leaf_shape(&format!("S{i}"))).collect());
    let bytes = encode_ast_derived(&shapes).expect("the fixture encodes");
    assert_eq!(
        decode_ast(&bytes).expect("decodes").node_shapes.len(),
        honest,
        "the clamp caps the reservation, never the sequence",
    );
}

/// A boolean spelled as anything but 0 or 1 means the reader is no longer at a
/// field boundary; both legal spellings still decode.
#[test]
fn a_non_boolean_byte_is_malformed() {
    // VALID: both spellings.
    for value in [false, true] {
        let mut writer = test_writer();
        writer.flag(value);
        let bytes = writer.out;
        let mut reader = test_reader(&bytes);
        assert_eq!(reader.flag().expect("decodes"), value);
    }

    // INVALID: any other byte.
    let bytes = [2u8];
    let mut reader = test_reader(&bytes);
    let error = reader.flag().expect_err("2 is not a boolean");
    assert_eq!(error.dimension(), ProductDimension::Malformed);
}

/// The shared shape index really is ONE handle: every `sh:nodeByExpression`
/// constraint and every computed shape argument in a decoded product points at
/// the same cell, so filling it once reaches all of them.
#[test]
fn the_decoded_shape_index_is_one_shared_handle() {
    let shapes = shapes_of(vec![Shape {
        constraints: vec![
            Constraint::NodeByExpression {
                expr: NodeExpr::This,
                shapes: Arc::new(OnceLock::new()),
                messages: vec![],
                severity: None,
            },
            Constraint::NodeByExpression {
                expr: NodeExpr::Empty,
                shapes: Arc::new(OnceLock::new()),
                messages: vec![],
                severity: None,
            },
            Constraint::Expression {
                expr: NodeExpr::ConformsToShape {
                    node: Box::new(NodeExpr::This),
                    shape: ShapeArg::Computed {
                        expr: Box::new(NodeExpr::Path(Path::Predicate(ex("kind")))),
                        shapes: Arc::new(OnceLock::new()),
                    },
                },
                messages: vec![],
                severity: None,
            },
        ],
        ..leaf_shape("Indexed")
    }]);

    let bytes = encode_ast_derived(&shapes).expect("encodes");
    let parts = decode_ast(&bytes).expect("decodes");

    let mut handles: Vec<&Arc<OnceLock<::purrdf::FastMap<Term, Shape>>>> = Vec::new();
    for constraint in &parts.node_shapes[0].constraints {
        match constraint {
            Constraint::NodeByExpression { shapes, .. } => handles.push(shapes),
            Constraint::Expression {
                expr:
                    NodeExpr::ConformsToShape {
                        shape: ShapeArg::Computed { shapes, .. },
                        ..
                    },
                ..
            } => handles.push(shapes),
            other => panic!("unexpected constraint: {other:?}"),
        }
    }
    assert_eq!(handles.len(), 3);
    for handle in &handles {
        assert!(
            Arc::ptr_eq(handle, &parts.shape_index),
            "every site must share the ONE index `AstParts::shape_index` hands out; a per-site \
             default would type-check and leave every constraint with a private, permanently \
             empty table",
        );
        assert!(
            handle.get().is_none(),
            "the index is filled by a later stage"
        );
    }

    // Filling it once reaches every site, which is the whole point of sharing it.
    parts
        .shape_index
        .set(::purrdf::FastMap::default())
        .expect("the index fills once");
    for handle in &handles {
        assert!(handle.get().is_some());
    }
}

/// An empty shapes graph is a legal product, not an error.
#[test]
fn an_empty_shapes_graph_round_trips() {
    let shapes = Shapes::default();
    let bytes = encode_ast_derived(&shapes).expect("an empty shapes graph encodes");
    let parts = decode_ast(&bytes).expect("it decodes");
    assert!(parts.node_shapes.is_empty());
    assert!(parts.target_types.is_empty());
    assert!(parts.custom_functions.is_empty());
    assert!(parts.box_role_vocab.is_none());
    assert!(parts.shapes_graph.is_none());

    let rebuilt = Shapes {
        node_shapes: parts.node_shapes,
        ..Shapes::default()
    };
    assert_eq!(encode_ast_derived(&rebuilt).expect("re-encodes"), bytes);
}

// ── `sh:closed sh:ByTypes` ──────────────────────────────────────────────────────

/// Two `sh:closed sh:ByTypes` constraints carrying one index decode to ONE shared
/// index, as the parse that wrote them shared one; a different index stays apart.
#[test]
fn equal_type_indexes_decode_to_one_shared_index() {
    let shared = sample_type_index();
    let other = Arc::new(
        ClosedTypeIndex::from_entries(vec![(ex_term("Other"), vec![ex("other")])])
            .expect("one type"),
    );
    let mut writer = test_writer();
    for index in [&shared, &shared, &other] {
        writer
            .closed_mode(&ClosedMode::ByTypes(Arc::clone(index)))
            .expect("encodes");
    }
    let bytes = writer.out;
    let mut reader = test_reader(&bytes);
    let decoded: Vec<Arc<ClosedTypeIndex>> = (0..3)
        .map(|_| match reader.closed_mode().expect("decodes") {
            ClosedMode::ByTypes(index) => index,
            ClosedMode::Declared => panic!("a ByTypes mode decodes as ByTypes"),
        })
        .collect();
    assert!(Arc::ptr_eq(&decoded[0], &decoded[1]));
    assert!(!Arc::ptr_eq(&decoded[1], &decoded[2]));
    assert_eq!(*decoded[0], *shared);
    assert_eq!(*decoded[2], *other);
}

/// A type index out of canonical order is refused as malformed rather than
/// re-sorted, so a product's bytes stay the canonical form of what they decode
/// to; the same entries in canonical order decode.
#[test]
fn a_non_canonical_type_index_is_malformed_and_the_canonical_one_decodes() {
    let encode = |entries: &[(&str, &[&str])]| {
        let mut writer = test_writer();
        writer.tag(1);
        writer.count(entries.len());
        for (ty, properties) in entries {
            writer.term(&ex_term(ty)).expect("an IRI encodes");
            writer.count(properties.len());
            for property in *properties {
                writer.named_node(&ex(property));
            }
        }
        writer.out
    };
    let unsorted = encode(&[("Sub", &["sub"]), ("Root", &["root"])]);
    let error = test_reader(&unsorted)
        .closed_mode()
        .expect_err("out-of-order types are refused");
    assert_eq!(error.dimension(), ProductDimension::Malformed);
    let empty = encode(&[("Root", &[])]);
    assert_eq!(
        test_reader(&empty)
            .closed_mode()
            .expect_err("an empty property list is refused")
            .dimension(),
        ProductDimension::Malformed
    );
    let canonical = encode(&[("Root", &["root"]), ("Sub", &["root", "sub"])]);
    assert!(matches!(
        test_reader(&canonical).closed_mode(),
        Ok(ClosedMode::ByTypes(_))
    ));
}

/// A per-constraint annotation list the shapes parser could not have written is
/// refused as malformed — out of range, out of order, a reifier annotation on a
/// node shape, an override of nothing — and the canonical list beside each
/// decodes; the tag past `AnnotatedConstraint`'s space is an unknown capability.
#[test]
fn a_non_canonical_constraint_annotation_list_is_malformed_and_the_canonical_one_decodes() {
    let encode = |entries: &[(Option<usize>, Option<Severity>, Option<&str>)]| {
        let mut writer = test_writer();
        writer.count(entries.len());
        for (index, severity, message) in entries {
            match index {
                Some(index) => {
                    writer.tag(0);
                    writer.count(*index);
                }
                None => writer.tag(1),
            }
            writer.opt_severity(severity.as_ref());
            let messages: Vec<Literal> = message
                .iter()
                .map(|m| Literal::new_simple_literal(*m))
                .collect();
            writer.messages(&messages);
        }
        writer.out
    };
    let decode = |bytes: &[u8], constraints: usize, property_shape: bool| {
        test_reader(bytes).constraint_annotations(constraints, property_shape)
    };
    let warning = || Some(Severity::Warning);

    let canonical = encode(&[(Some(0), warning(), None), (None, None, Some("m"))]);
    let decoded = decode(&canonical, 1, true).expect("the canonical list decodes");
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[1].constraint, AnnotatedConstraint::Reifier);

    for (label, bytes, constraints, property_shape) in [
        (
            "past the constraints",
            encode(&[(Some(1), warning(), None)]),
            1,
            true,
        ),
        (
            "out of order",
            encode(&[(Some(1), warning(), None), (Some(0), warning(), None)]),
            2,
            true,
        ),
        (
            "a reifier on a node shape",
            encode(&[(None, warning(), None)]),
            1,
            false,
        ),
        (
            "overriding nothing",
            encode(&[(Some(0), None, None)]),
            1,
            true,
        ),
    ] {
        assert_eq!(
            decode(&bytes, constraints, property_shape)
                .expect_err(label)
                .dimension(),
            ProductDimension::Malformed,
            "{label}"
        );
    }

    let mut unknown = test_writer();
    unknown.count(1);
    unknown.tag(TAGS_ANNOTATED_CONSTRAINT);
    assert_eq!(
        decode(&unknown.out, 1, true)
            .expect_err("an unknown AnnotatedConstraint tag is refused")
            .dimension(),
        ProductDimension::UnsupportedCapability
    );
}

/// A message set out of canonical order, repeated, or holding a literal SHACL
/// does not permit as an `sh:message` is malformed; the canonical set of tagged
/// messages beside it decodes with every tag intact.
#[test]
fn a_non_canonical_message_set_is_malformed_and_the_canonical_one_decodes() {
    let encode = |messages: &[Literal]| {
        let mut writer = test_writer();
        writer.count(messages.len());
        for message in messages {
            writer.literal(message);
        }
        writer.out
    };
    let en = Literal::new_language_tagged_literal_unchecked("Too many", "en");
    let de = Literal::new_language_tagged_literal_unchecked("Zu viele", "de");
    let decoded = test_reader(&encode(&[en.clone(), de.clone()]))
        .messages()
        .expect("the canonical set decodes");
    assert_eq!(decoded, vec![en.clone(), de.clone()]);
    for (label, bytes) in [
        ("out of order", encode(&[de, en.clone()])),
        ("repeated", encode(&[en.clone(), en])),
        (
            "not a message literal",
            encode(&[Literal::new_typed_literal("1", ex("Datatype"))]),
        ),
    ] {
        assert_eq!(
            test_reader(&bytes).messages().expect_err(label).dimension(),
            ProductDimension::Malformed,
            "{label}"
        );
    }
}
