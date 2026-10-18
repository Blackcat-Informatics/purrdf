// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deep borrowed preparation and dependency walks, independent of AST teardown.

use purrdf_core::RdfDatasetBuilder;
use purrdf_shex::ast::{Schema, Shape, ShapeDecl, ShapeExpr, TripleConstraint, TripleExpr};
use purrdf_shex::{StructureError, check_structure, parse_shexc, validate};

const SHAPE: &str = "http://example.org/S";
const MISSING: &str = "http://example.org/Missing";
const PREDICATE: &str = "http://example.org/p";

#[test]
fn large_neighbourhood_backtracks_without_using_the_native_call_stack() {
    let schema = parse_shexc(
        "PREFIX ex: <http://example.org/> ex:S { ex:p .* ; ex:p . }",
        None,
    )
    .unwrap();
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/focus").unwrap();
    let predicate = builder.intern_iri(PREDICATE).unwrap();
    for value in 0..10_000 {
        let object = builder
            .intern_iri(&format!("http://example.org/value/{value}"))
            .unwrap();
        builder.push_quad(subject, predicate, object, None).unwrap();
    }
    let data = builder.freeze().unwrap();
    let map = [(
        purrdf_core::TermValue::iri("http://example.org/focus"),
        purrdf_shex::ShapeSelector::Label(SHAPE.to_owned()),
    )];
    let report =
        purrdf_stack::on_stack_scoped(256 * 1024, || validate(&schema, &data, &map)).unwrap();
    assert_eq!(report.entries.len(), 1);
    assert_eq!(
        report.entries[0].status,
        purrdf_shex::ConformanceStatus::Conformant
    );
}

#[test]
fn deep_dependency_walk_keeps_missing_references_and_odd_negation() {
    // Construction and owned AST teardown are outside the small-stack check.
    // This specifically qualifies the borrowed admission walk, not AST Drop.
    purrdf_stack::on_stack(16 * 1024 * 1024, || {
        for negative in [false, true] {
            let mut expr = ShapeExpr::Ref(if negative { SHAPE } else { MISSING }.to_owned());
            for _ in 0..10_001 {
                expr = if negative {
                    ShapeExpr::Not(Box::new(expr))
                } else {
                    ShapeExpr::And(vec![expr])
                };
            }
            let schema = Schema {
                shapes: vec![ShapeDecl {
                    id: SHAPE.to_owned(),
                    expr,
                }],
                ..Schema::default()
            };
            let errors =
                purrdf_stack::on_stack_scoped(256 * 1024, || check_structure(&schema).unwrap_err())
                    .unwrap();
            let expected = if negative {
                vec![
                    StructureError::ReferenceCycle(SHAPE.to_owned()),
                    StructureError::NegatedCycle(SHAPE.to_owned()),
                ]
            } else {
                vec![StructureError::DanglingShapeRef(MISSING.to_owned())]
            };
            assert_eq!(errors, expected);
        }
    })
    .unwrap();
}

#[test]
fn deep_cross_kind_walk_keeps_extra_and_inverse_polarity() {
    purrdf_stack::on_stack(16 * 1024 * 1024, || {
        for inverse in [false, true] {
            let mut expr = ShapeExpr::Ref(SHAPE.to_owned());
            for level in 0..10_000 {
                expr = ShapeExpr::Shape(Shape {
                    // Exactly one EXTRA reversal, at the outermost shape.
                    extra: if level == 9_999 {
                        vec![PREDICATE.to_owned()]
                    } else {
                        vec![]
                    },
                    expression: Some(TripleExpr::TripleConstraint(TripleConstraint {
                        predicate: PREDICATE.to_owned(),
                        inverse: Some(inverse),
                        value_expr: Some(Box::new(expr)),
                        ..TripleConstraint::default()
                    })),
                    ..Shape::default()
                });
            }
            let schema = Schema {
                shapes: vec![ShapeDecl {
                    id: SHAPE.to_owned(),
                    expr,
                }],
                ..Schema::default()
            };
            let result =
                purrdf_stack::on_stack_scoped(256 * 1024, || check_structure(&schema)).unwrap();
            if inverse {
                assert!(result.is_ok());
            } else {
                assert_eq!(
                    result,
                    Err(vec![StructureError::NegatedCycle(SHAPE.to_owned())])
                );
            }
        }
    })
    .unwrap();
}

#[test]
fn deep_preparation_census_runs_on_the_caller_small_stack() {
    purrdf_stack::on_stack(16 * 1024 * 1024, || {
        let mut expr = ShapeExpr::Shape(Shape::default());
        for _ in 0..10_000 {
            expr = ShapeExpr::And(vec![expr]);
        }
        let schema = Schema {
            shapes: vec![ShapeDecl {
                id: SHAPE.to_owned(),
                expr,
            }],
            ..Schema::default()
        };
        let data = RdfDatasetBuilder::new().freeze().unwrap();
        let prepared = purrdf_stack::on_stack_scoped(256 * 1024, || {
            // An empty shape map still prepares the complete schema. Evaluating
            // a deep shape is a different operation, not claimed by this test.
            validate(&schema, &data, &[])
        })
        .unwrap();
        assert_eq!(prepared.entries, [] as [purrdf_shex::ResultEntry; 0]);
    })
    .unwrap();
}

#[test]
fn repeated_inclusion_is_checked_under_each_enclosing_shape() {
    let schema = parse_shexc(
        "PREFIX ex: <http://example.org/>\n\
         ex:S ({ &ex:Shared } AND EXTRA ex:p { &ex:Shared })\n\
         ex:T { $ex:Shared ex:p @ex:S }",
        None,
    )
    .unwrap();
    assert_eq!(
        check_structure(&schema),
        Err(vec![StructureError::NegatedCycle(SHAPE.to_owned())])
    );
}
