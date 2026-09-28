// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every whole-tree operation over the algebra runs in the same machine stack however
//! deep the tree is.
//!
//! Each shape is built a million levels deep, by a loop rather than by the parser, and
//! then copied, compared, hashed, formatted, serialized and dropped on a thread whose
//! whole stack is 128 KiB. A recursive walk spends at least a few dozen bytes of stack
//! per level, so any operation that still recursed would overflow that stack many
//! times over and abort the test process: every assertion reached is itself the proof
//! that nothing recursed.
//!
//! The shapes cover every kind of owning edge ([`purrdf_sparql_algebra::Child`],
//! [`purrdf_sparql_algebra::Chain`], [`purrdf_sparql_algebra::Args`],
//! [`purrdf_sparql_algebra::NonEmpty`]) and every node kind that can nest: patterns
//! (an `OPTIONAL` spine, a `UNION` nest, `EXISTS` alternating with `FILTER`),
//! expressions (nested calls, nested unary operators, bracketted arithmetic, a `||`
//! nest), property paths (sequences nested through brackets) and quoted triple terms
//! (in a pattern and in a `VALUES` cell).
//!
//! The drop is also checked for completeness: building and dropping a tree inside one
//! counting-allocator window leaves no live byte behind.

use std::fmt::Write as _;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_sparql_algebra::{
    ArithmeticOperator, Chain, Child, Expression, Function, GraphPattern, GroundTerm, GroundTriple,
    Literal, NamedNode, NamedNodePattern, NonEmpty, PropertyPathExpression, TermPattern,
    TriplePattern, Variable, pattern_to_select_query,
};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// How deep every shape is built.
const LEVELS: usize = 1_000_000;

/// The whole stack of the thread every operation runs on.
const STACK: usize = 128 * 1024;

/// One tree a million levels deep at a time: they are large.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn var(name: &str) -> Variable {
    Variable::new(name)
}

fn iri(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://example.org/{local}"))
}

fn empty() -> GraphPattern {
    GraphPattern::Bgp {
        patterns: Vec::new(),
    }
}

fn one_triple() -> GraphPattern {
    GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(var("s")),
            predicate: NamedNodePattern::NamedNode(iri("p")),
            object: TermPattern::Variable(var("o")),
        }],
    }
}

fn filtered(expr: Expression) -> GraphPattern {
    GraphPattern::Filter {
        expr,
        inner: Child::new(one_triple()),
    }
}

/// Counts what is written to it, keeping nothing.
#[derive(Default)]
struct Counted {
    bytes: usize,
}

impl std::fmt::Write for Counted {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.bytes += s.len();
        Ok(())
    }
}

fn hash_of(pattern: &GraphPattern) -> u64 {
    let mut hasher = DefaultHasher::new();
    pattern.hash(&mut hasher);
    hasher.finish()
}

/// Build the tree `build` makes and walk it every whole-tree way, on a thread whose
/// whole stack is [`STACK`], inside one counting-allocator window: when the tree and its
/// copy are dropped, no live byte is left. Returns how many bytes `{:?}` wrote and the
/// serialized text's length.
fn walk_every_way(build: fn() -> GraphPattern) -> (usize, usize) {
    let _serial = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::thread::Builder::new()
        .name("deep-tree".to_owned())
        .stack_size(STACK)
        .spawn(move || {
            let window = CurrentThreadWindow::open();
            let tree = build();
            let copy = tree.clone();
            assert!(tree == copy, "a copy equals its original");
            assert_eq!(hash_of(&tree), hash_of(&copy), "equal trees hash equally");
            let mut debug = Counted::default();
            write!(debug, "{tree:?}").expect("counting cannot fail");
            let mut copied = Counted::default();
            write!(copied, "{copy:?}").expect("counting cannot fail");
            assert_eq!(debug.bytes, copied.bytes);
            let text = pattern_to_select_query(&tree).len();
            let built = window.sample();
            drop(copy);
            drop(tree);
            let left = window.close();
            assert!(built.retained_bytes > 0, "the tree was built");
            assert_eq!(left.retained_bytes, 0, "the drop freed every node");
            (debug.bytes, text)
        })
        .expect("the thread starts")
        .join()
        .expect("no walk overflowed the thread's stack")
}

/// `OPTIONAL` a million times: a `LeftJoin` spine down its left operand.
fn optional_spine() -> GraphPattern {
    let mut pattern = one_triple();
    for _ in 0..LEVELS {
        pattern = GraphPattern::LeftJoin {
            left: Child::new(pattern),
            right: Child::new(empty()),
            expression: None,
        };
    }
    pattern
}

/// `{ … } UNION { … }` nested down the second arm.
fn union_nest() -> GraphPattern {
    let mut pattern = one_triple();
    for _ in 0..LEVELS {
        pattern = GraphPattern::Union {
            arms: Chain::new(empty(), pattern, []),
        };
    }
    pattern
}

/// `FILTER EXISTS { FILTER EXISTS { … } }`: patterns and expressions alternating.
fn exists_nest() -> GraphPattern {
    let mut pattern = one_triple();
    for _ in 0..LEVELS {
        pattern = GraphPattern::Filter {
            expr: Expression::Exists(Child::new(pattern)),
            inner: Child::new(empty()),
        };
    }
    pattern
}

/// `ABS(ABS(… ?o …))`: nested calls, each argument list an [`purrdf_sparql_algebra::Args`].
fn nested_calls() -> GraphPattern {
    let mut expr = Expression::Variable(var("o"));
    for _ in 0..LEVELS {
        expr = Expression::FunctionCall(Function::Abs, vec![expr].into());
    }
    filtered(expr)
}

/// `!-!-… ?o`: nested unary operators.
fn nested_unary() -> GraphPattern {
    let mut expr = Expression::Variable(var("o"));
    for level in 0..LEVELS {
        expr = if level % 2 == 0 {
            Expression::UnaryMinus(Child::new(expr))
        } else {
            Expression::Not(Child::new(expr))
        };
    }
    filtered(expr)
}

/// `?o - (?o - (?o - …))`: arithmetic nested through its steps, bracketted when
/// written.
fn bracketted_arithmetic() -> GraphPattern {
    let mut expr = Expression::Variable(var("o"));
    for _ in 0..LEVELS {
        expr = Expression::Arithmetic(
            Child::new(Expression::Variable(var("o"))),
            NonEmpty::new((ArithmeticOperator::Subtract, expr)),
        );
    }
    filtered(expr)
}

/// `?a || (?a || (?a || …))`: a `||` nested through its second operand.
fn or_nest() -> GraphPattern {
    let mut expr = Expression::Variable(var("a"));
    for _ in 0..LEVELS {
        expr = Expression::Or(Chain::new(Expression::Variable(var("a")), expr, []));
    }
    filtered(expr)
}

/// `<p>/(<p>/(<p>/…))`: path sequences nested through brackets.
fn nested_path_sequences() -> GraphPattern {
    let mut path = PropertyPathExpression::NamedNode(iri("p"));
    for _ in 0..LEVELS {
        path = PropertyPathExpression::Sequence(Chain::new(
            PropertyPathExpression::NamedNode(iri("p")),
            path,
            [],
        ));
    }
    GraphPattern::Path {
        subject: TermPattern::Variable(var("s")),
        path,
        object: TermPattern::Variable(var("o")),
    }
}

/// `<<( ?s <p> <<( ?s <p> … )>> )>>` in a pattern's object.
fn nested_triple_terms() -> GraphPattern {
    let mut term = TermPattern::Variable(var("o"));
    for _ in 0..LEVELS {
        term = TermPattern::Triple(Child::new(TriplePattern {
            subject: TermPattern::Variable(var("s")),
            predicate: NamedNodePattern::NamedNode(iri("p")),
            object: term,
        }));
    }
    GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(var("s")),
            predicate: NamedNodePattern::NamedNode(iri("r")),
            object: term,
        }],
    }
}

/// `VALUES ?v { <<( <s> <p> <<( <s> <p> … )>> )>> }`: ground triple terms nested in a
/// `VALUES` cell.
fn nested_ground_triple_terms() -> GraphPattern {
    let mut term = GroundTerm::Literal(Literal::new_simple("x"));
    for _ in 0..LEVELS {
        term = GroundTerm::Triple(Child::new(GroundTriple {
            subject: GroundTerm::NamedNode(iri("s")),
            predicate: iri("p"),
            object: term,
        }));
    }
    GraphPattern::Values {
        variables: vec![var("v")],
        bindings: vec![vec![Some(term)]],
    }
}

macro_rules! deep_tests {
    ($($name:ident => $build:ident;)+) => {$(
        #[test]
        fn $name() {
            let (debug_bytes, text_bytes) = walk_every_way($build);
            assert!(debug_bytes > LEVELS, "every level is written by Debug");
            assert!(text_bytes > LEVELS, "every level is serialized");
        }
    )+};
}

deep_tests! {
    an_optional_spine_is_walked_without_recursion => optional_spine;
    a_union_nest_is_walked_without_recursion => union_nest;
    an_exists_nest_is_walked_without_recursion => exists_nest;
    nested_calls_are_walked_without_recursion => nested_calls;
    nested_unary_operators_are_walked_without_recursion => nested_unary;
    bracketted_arithmetic_is_walked_without_recursion => bracketted_arithmetic;
    an_or_nest_is_walked_without_recursion => or_nest;
    nested_path_sequences_are_walked_without_recursion => nested_path_sequences;
    nested_triple_terms_are_walked_without_recursion => nested_triple_terms;
    nested_ground_triple_terms_are_walked_without_recursion => nested_ground_triple_terms;
}
