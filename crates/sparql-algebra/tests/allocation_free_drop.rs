// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Failed construction must be able to destroy every algebra owning edge without
//! requesting more allocator storage, even on the smallest supported test stack.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_sparql_algebra::{
    Args, ArithmeticOperator, Chain, Child, Expression, Function, GraphPattern, NonEmpty,
    PropertyPathExpression, Variable,
};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn leaf() -> Expression {
    Expression::Variable(Variable::new("x"))
}

fn assert_drop_allocates_nothing<T>(value: T) {
    let window = CurrentThreadWindow::open();
    drop(value);
    let measured = window.close();
    assert_eq!(
        measured.allocations, 0,
        "destruction requested fresh storage"
    );
    assert!(
        measured.retained_bytes <= 0,
        "destruction retained new storage"
    );
}

#[test]
fn heterogeneous_box_and_list_continuations_drop_without_allocating() {
    purrdf_stack::on_stack(128 * 1024, || {
        let mut graph = GraphPattern::empty_bgp();
        for _ in 0..100_000 {
            graph = GraphPattern::Union {
                arms: Chain::new(
                    GraphPattern::empty_bgp(),
                    GraphPattern::Filter {
                        expr: Expression::Exists(Child::new(graph)),
                        inner: Child::new(GraphPattern::empty_bgp()),
                    },
                    [],
                ),
            };
        }
        assert_drop_allocates_nothing(graph);

        let mut expr = leaf();
        for _ in 0..100_000 {
            expr = Expression::FunctionCall(
                Function::Abs,
                vec![Expression::Arithmetic(
                    Child::new(leaf()),
                    NonEmpty::new((ArithmeticOperator::Add, expr)),
                )]
                .into(),
            );
        }
        assert_drop_allocates_nothing(expr);
    })
    .expect("the small-stack thread starts");
}

#[test]
fn property_path_chains_and_wide_branches_drop_without_allocating() {
    purrdf_stack::on_stack(128 * 1024, || {
        let mut path = PropertyPathExpression::NegatedPropertySet(Vec::new());
        for _ in 0..100_000 {
            path = PropertyPathExpression::Alternative(Chain::new(
                PropertyPathExpression::NegatedPropertySet(Vec::new()),
                PropertyPathExpression::Reverse(Child::new(path)),
                [],
            ));
        }
        assert_drop_allocates_nothing(path);
        let args: Args<_> = (0..8192)
            .map(|_| Expression::If(Child::new(leaf()), Child::new(leaf()), Child::new(leaf())))
            .collect();
        assert_drop_allocates_nothing(args);
    })
    .expect("the small-stack thread starts");
}

struct Refuse;
impl Admission for Refuse {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes == 0 {
            Ok(())
        } else {
            Err(StorageError::AdmissionFailed)
        }
    }
}

#[test]
fn refused_child_construction_destroys_its_whole_input_without_allocating() {
    purrdf_stack::on_stack(128 * 1024, || {
        let mut graph = GraphPattern::empty_bgp();
        for _ in 0..100_000 {
            graph = GraphPattern::Union {
                arms: Chain::new(GraphPattern::empty_bgp(), graph, []),
            };
        }
        let mut refusal = Refuse;
        let mut memory = Memory::new(&mut refusal);
        let window = CurrentThreadWindow::open();
        let result = Child::try_new(graph, &mut memory);
        assert!(matches!(result, Err(StorageError::AdmissionFailed)));
        assert_eq!(memory.admitted_bytes(), 0);
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        assert!(
            measured.retained_bytes < 0,
            "refusal destroyed the owned input"
        );
    })
    .expect("the small-stack thread starts");
}

use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, GroundTerm, GroundTriple, NamedNode, NamedNodePattern,
    OrderExpression, PropertyFunctionCall, TermPattern, TriplePattern,
};

fn assert_constructed_storage_is_released<T>(construct: impl FnOnce() -> T) {
    // Probe windows cannot nest. Compare separate construction/destruction
    // windows so every raw array and shared lexical owner must be released.
    let construction = CurrentThreadWindow::open();
    let value = construct();
    let constructed = construction.close();
    assert!(
        constructed.retained_bytes > 0,
        "the fixture retained owned storage"
    );
    let destruction = CurrentThreadWindow::open();
    drop(value);
    let destroyed = destruction.close();
    assert_eq!(
        destroyed.allocations, 0,
        "destruction requested fresh storage"
    );
    assert_eq!(
        destroyed.retained_bytes, -constructed.retained_bytes,
        "destruction leaked constructed storage"
    );
}

fn quoted_pattern(depth: usize) -> TermPattern {
    let predicate = NamedNodePattern::Variable(Variable::new("predicate"));
    let mut term = TermPattern::Variable(Variable::new("quoted"));
    for at in 0..depth {
        let sibling = TermPattern::Variable(Variable::new("sibling"));
        let (subject, object) = if at % 2 == 0 {
            (term, sibling)
        } else {
            (sibling, term)
        };
        term = TermPattern::Triple(Child::new(TriplePattern {
            subject,
            predicate: predicate.clone(),
            object,
        }));
    }
    term
}

fn quoted_ground(depth: usize) -> GroundTerm {
    let predicate = NamedNode::new_unchecked("https://example.org/predicate");
    let mut term = GroundTerm::NamedNode(NamedNode::new_unchecked("https://example.org/quoted"));
    for at in 0..depth {
        let sibling =
            GroundTerm::NamedNode(NamedNode::new_unchecked("https://example.org/sibling"));
        let (subject, object) = if at % 2 == 0 {
            (term, sibling)
        } else {
            (sibling, term)
        };
        term = GroundTerm::Triple(Child::new(GroundTriple {
            subject,
            predicate: predicate.clone(),
            object,
        }));
    }
    term
}

#[test]
fn group_aggregate_argument_and_order_vectors_drop_without_allocating() {
    purrdf_stack::on_stack(128 * 1024, || {
        assert_constructed_storage_is_released(|| {
            let mut graph = GraphPattern::empty_bgp();
            for at in 0..100_000 {
                // Alternate the continuation between FOLD's raw argument array
                // and its independent raw sort-key array. The second aggregate
                // still has to be destroyed after the deep branch returns.
                let (args, order) = if at % 2 == 0 {
                    (
                        vec![Expression::Exists(Child::new(graph)), leaf()],
                        vec![OrderExpression::Asc(leaf()), OrderExpression::Desc(leaf())],
                    )
                } else {
                    (
                        vec![leaf(), leaf()],
                        vec![
                            OrderExpression::Asc(leaf()),
                            OrderExpression::Desc(Expression::Exists(Child::new(graph))),
                        ],
                    )
                };
                let fold = AggregateExpression::new(
                    AggregateFunction::Fold,
                    args,
                    Vec::new(),
                    order,
                    false,
                )
                .expect("two arguments and sort keys form a valid FOLD");
                let count = AggregateExpression::new(
                    AggregateFunction::Count,
                    vec![leaf()],
                    Vec::new(),
                    Vec::new(),
                    true,
                )
                .expect("COUNT accepts one expression");
                graph = GraphPattern::Group {
                    inner: Child::new(GraphPattern::empty_bgp()),
                    variables: vec![Variable::new("key")],
                    aggregates: vec![
                        (Variable::new("fold"), fold),
                        (Variable::new("count"), count),
                    ],
                };
            }
            graph
        });
    })
    .expect("the small-stack thread starts");
}

#[test]
fn order_by_raw_keys_resume_after_deep_branches_without_allocating() {
    purrdf_stack::on_stack(128 * 1024, || {
        assert_constructed_storage_is_released(|| {
            let mut graph = GraphPattern::empty_bgp();
            for _ in 0..100_000 {
                graph = GraphPattern::OrderBy {
                    inner: Child::new(GraphPattern::empty_bgp()),
                    expression: vec![
                        OrderExpression::Asc(leaf()),
                        OrderExpression::Desc(Expression::Exists(Child::new(graph))),
                        OrderExpression::Asc(leaf()),
                    ],
                };
            }
            graph
        });
        assert_constructed_storage_is_released(|| GraphPattern::OrderBy {
            inner: Child::new(GraphPattern::empty_bgp()),
            expression: (0..8192)
                .map(|_| {
                    OrderExpression::Asc(Expression::Exists(Child::new(GraphPattern::empty_bgp())))
                })
                .collect(),
        });
    })
    .expect("the small-stack thread starts");
}

#[test]
fn values_raw_rows_and_quoted_ground_triples_drop_without_allocating() {
    purrdf_stack::on_stack(128 * 1024, || {
        assert_constructed_storage_is_released(|| GraphPattern::Values {
            variables: vec![Variable::new("first"), Variable::new("second")],
            bindings: vec![
                vec![None, Some(quoted_ground(1))],
                vec![Some(quoted_ground(100_000)), None],
                vec![Some(quoted_ground(1)), Some(quoted_ground(1))],
            ],
        });
        assert_constructed_storage_is_released(|| GraphPattern::Values {
            variables: vec![Variable::new("first"), Variable::new("second")],
            bindings: (0..8192)
                .map(|_| vec![Some(quoted_ground(1)), None])
                .collect(),
        });
    })
    .expect("the small-stack thread starts");
}

#[test]
fn bgp_raw_pattern_vectors_and_quoted_triples_drop_without_allocating() {
    purrdf_stack::on_stack(128 * 1024, || {
        assert_constructed_storage_is_released(|| GraphPattern::Bgp {
            patterns: vec![
                TriplePattern {
                    subject: quoted_pattern(1),
                    predicate: NamedNodePattern::Variable(Variable::new("predicate")),
                    object: quoted_pattern(1),
                },
                TriplePattern {
                    subject: quoted_pattern(100_000),
                    predicate: NamedNodePattern::Variable(Variable::new("predicate")),
                    object: quoted_pattern(1),
                },
                TriplePattern {
                    subject: quoted_pattern(1),
                    predicate: NamedNodePattern::Variable(Variable::new("predicate")),
                    object: quoted_pattern(1),
                },
            ],
        });
        assert_constructed_storage_is_released(|| GraphPattern::Bgp {
            patterns: (0..8192)
                .map(|_| TriplePattern {
                    subject: quoted_pattern(1),
                    predicate: NamedNodePattern::Variable(Variable::new("predicate")),
                    object: quoted_pattern(1),
                })
                .collect(),
        });
    })
    .expect("the small-stack thread starts");
}

#[test]
fn property_function_raw_term_arguments_drop_without_allocating() {
    purrdf_stack::on_stack(128 * 1024, || {
        assert_constructed_storage_is_released(|| {
            GraphPattern::PropertyFunction(PropertyFunctionCall {
                iri: "https://example.org/relation".into(),
                subject_args: vec![
                    quoted_pattern(1),
                    quoted_pattern(100_000),
                    quoted_pattern(1),
                ],
                object_args: vec![
                    quoted_pattern(1),
                    quoted_pattern(100_000),
                    quoted_pattern(1),
                ],
            })
        });
        assert_constructed_storage_is_released(|| {
            GraphPattern::PropertyFunction(PropertyFunctionCall {
                iri: "https://example.org/relation".into(),
                subject_args: (0..8192).map(|_| quoted_pattern(1)).collect(),
                object_args: (0..8192).map(|_| quoted_pattern(1)).collect(),
            })
        });
    })
    .expect("the small-stack thread starts");
}
