// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The algebra's `Clone`, `==`, `Hash` and `Debug` walk the tree over work lists; this
//! pins each against a recursive reference.
//!
//! The reference is [`mirror`]: the same node types, field for field and name for name,
//! with `Box` and `Vec` for the edges and every trait **derived** — exactly what the
//! compiler writes for a recursive type. A tree is converted into its mirror
//! recursively, which is fine for the bounded depths used here. Then:
//!
//! * `{:?}` and `{:#?}` of a tree are byte-identical to its mirror's;
//! * a copy's mirror equals the original's mirror;
//! * two trees are `==` exactly when their mirrors are;
//! * equal trees hash equally, and no two unequal trees of the sample hash equally.
//!
//! The trees are every query the in-repository corpora parse to — the same corpora the
//! committed AST snapshot pins — and generated trees that reach every node kind and
//! variant, including shapes the parser never builds (hand-built algebra is public).

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use proptest::prelude::*;
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, ArithmeticOperator, BlankNode, Chain, Child,
    Expression, Function, GraphPattern, GroundTerm, GroundTriple, Literal, NamedNode,
    NamedNodePattern, NegatedPathElement, NonEmpty, OrderExpression, ParserOptions,
    PropertyFunctionCall, PropertyPathExpression, Query, SparqlParser, TermPattern, TriplePattern,
    Variable,
};

/// The node types with every trait derived: the recursive reference.
mod mirror {
    use purrdf_sparql_algebra::{
        AggregateFunction, ArithmeticOperator, BlankNode, Function, Literal, NamedNode,
        NamedNodePattern, NegatedPathElement, Variable,
    };

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum GraphPattern {
        Bgp {
            patterns: Vec<TriplePattern>,
        },
        Path {
            subject: TermPattern,
            path: PropertyPathExpression,
            object: TermPattern,
        },
        Join {
            left: Box<Self>,
            right: Box<Self>,
        },
        LeftJoin {
            left: Box<Self>,
            right: Box<Self>,
            expression: Option<Expression>,
        },
        Lateral {
            left: Box<Self>,
            right: Box<Self>,
        },
        Filter {
            expr: Expression,
            inner: Box<Self>,
        },
        Union {
            arms: Vec<Self>,
        },
        Graph {
            name: NamedNodePattern,
            inner: Box<Self>,
        },
        Extend {
            inner: Box<Self>,
            variable: Variable,
            expression: Expression,
        },
        Minus {
            left: Box<Self>,
            right: Box<Self>,
        },
        Service {
            name: NamedNodePattern,
            inner: Box<Self>,
            silent: bool,
        },
        Values {
            variables: Vec<Variable>,
            bindings: Vec<Vec<Option<GroundTerm>>>,
        },
        OrderBy {
            inner: Box<Self>,
            expression: Vec<OrderExpression>,
        },
        Project {
            inner: Box<Self>,
            variables: Vec<Variable>,
        },
        Distinct {
            inner: Box<Self>,
        },
        Reduced {
            inner: Box<Self>,
        },
        Slice {
            inner: Box<Self>,
            start: usize,
            length: Option<usize>,
        },
        Group {
            inner: Box<Self>,
            variables: Vec<Variable>,
            aggregates: Vec<(Variable, AggregateExpression)>,
        },
        PropertyFunction(PropertyFunctionCall),
        Unfold {
            inner: Box<Self>,
            expression: Expression,
            element: Variable,
            companion: Option<Variable>,
        },
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct PropertyFunctionCall {
        pub(crate) iri: String,
        pub(crate) subject_args: Vec<TermPattern>,
        pub(crate) object_args: Vec<TermPattern>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum PropertyPathExpression {
        NamedNode(NamedNode),
        Reverse(Box<Self>),
        Sequence(Vec<Self>),
        Alternative(Vec<Self>),
        ZeroOrMore(Box<Self>),
        OneOrMore(Box<Self>),
        ZeroOrOne(Box<Self>),
        NegatedPropertySet(Vec<NegatedPathElement>),
        Range {
            inner: Box<Self>,
            min: u32,
            max: Option<u32>,
        },
        Wildcard {
            namespace: Option<NamedNode>,
        },
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum Expression {
        NamedNode(NamedNode),
        Literal(Literal),
        Variable(Variable),
        Bound(Variable),
        Or(Vec<Self>),
        And(Vec<Self>),
        Equal(Box<Self>, Box<Self>),
        SameTerm(Box<Self>, Box<Self>),
        Greater(Box<Self>, Box<Self>),
        GreaterOrEqual(Box<Self>, Box<Self>),
        Less(Box<Self>, Box<Self>),
        LessOrEqual(Box<Self>, Box<Self>),
        Arithmetic(Box<Self>, Vec<(ArithmeticOperator, Self)>),
        UnaryPlus(Box<Self>),
        UnaryMinus(Box<Self>),
        Not(Box<Self>),
        In(Box<Self>, Vec<Self>),
        If(Box<Self>, Box<Self>, Box<Self>),
        Coalesce(Vec<Self>),
        FunctionCall(Function, Vec<Self>),
        Exists(Box<GraphPattern>),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum OrderExpression {
        Asc(Expression),
        Desc(Expression),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct AggregateExpression {
        pub(crate) function: AggregateFunction,
        pub(crate) args: Vec<Expression>,
        pub(crate) scalarvals: Vec<(String, Literal)>,
        pub(crate) order_by: Vec<OrderExpression>,
        pub(crate) distinct: bool,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum TermPattern {
        NamedNode(NamedNode),
        BlankNode(BlankNode),
        Literal(Literal),
        Variable(Variable),
        Triple(Box<TriplePattern>),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct TriplePattern {
        pub(crate) subject: TermPattern,
        pub(crate) predicate: NamedNodePattern,
        pub(crate) object: TermPattern,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum GroundTerm {
        NamedNode(NamedNode),
        Literal(Literal),
        Triple(Box<GroundTriple>),
        BlankNode(BlankNode),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct GroundTriple {
        pub(crate) subject: GroundTerm,
        pub(crate) predicate: NamedNode,
        pub(crate) object: GroundTerm,
    }
}

// ── conversion into the mirror (recursive: bounded depths only) ──────────────────────

fn m_pattern(p: &GraphPattern) -> mirror::GraphPattern {
    use mirror::GraphPattern as M;
    let b = |p: &GraphPattern| Box::new(m_pattern(p));
    match p {
        GraphPattern::Bgp { patterns } => M::Bgp {
            patterns: patterns.iter().map(m_triple).collect(),
        },
        GraphPattern::Path {
            subject,
            path,
            object,
        } => M::Path {
            subject: m_term(subject),
            path: m_path(path),
            object: m_term(object),
        },
        GraphPattern::Join { left, right } => M::Join {
            left: b(left),
            right: b(right),
        },
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => M::LeftJoin {
            left: b(left),
            right: b(right),
            expression: expression.as_ref().map(m_expr),
        },
        GraphPattern::Lateral { left, right } => M::Lateral {
            left: b(left),
            right: b(right),
        },
        GraphPattern::Filter { expr, inner } => M::Filter {
            expr: m_expr(expr),
            inner: b(inner),
        },
        GraphPattern::Union { arms } => M::Union {
            arms: arms.iter().map(m_pattern).collect(),
        },
        GraphPattern::Graph { name, inner } => M::Graph {
            name: name.clone(),
            inner: b(inner),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => M::Extend {
            inner: b(inner),
            variable: variable.clone(),
            expression: m_expr(expression),
        },
        GraphPattern::Minus { left, right } => M::Minus {
            left: b(left),
            right: b(right),
        },
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => M::Service {
            name: name.clone(),
            inner: b(inner),
            silent: *silent,
        },
        GraphPattern::Values {
            variables,
            bindings,
        } => M::Values {
            variables: variables.clone(),
            bindings: bindings
                .iter()
                .map(|row| row.iter().map(|cell| cell.as_ref().map(m_ground)).collect())
                .collect(),
        },
        GraphPattern::OrderBy { inner, expression } => M::OrderBy {
            inner: b(inner),
            expression: expression.iter().map(m_order).collect(),
        },
        GraphPattern::Project { inner, variables } => M::Project {
            inner: b(inner),
            variables: variables.clone(),
        },
        GraphPattern::Distinct { inner } => M::Distinct { inner: b(inner) },
        GraphPattern::Reduced { inner } => M::Reduced { inner: b(inner) },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => M::Slice {
            inner: b(inner),
            start: *start,
            length: *length,
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => M::Group {
            inner: b(inner),
            variables: variables.clone(),
            aggregates: aggregates
                .iter()
                .map(|(v, a)| (v.clone(), m_aggregate(a)))
                .collect(),
        },
        GraphPattern::PropertyFunction(call) => M::PropertyFunction(mirror::PropertyFunctionCall {
            iri: call.iri.clone(),
            subject_args: call.subject_args.iter().map(m_term).collect(),
            object_args: call.object_args.iter().map(m_term).collect(),
        }),
        GraphPattern::Unfold {
            inner,
            expression,
            element,
            companion,
        } => M::Unfold {
            inner: b(inner),
            expression: m_expr(expression),
            element: element.clone(),
            companion: companion.clone(),
        },
    }
}

fn m_expr(e: &Expression) -> mirror::Expression {
    use mirror::Expression as M;
    let b = |e: &Expression| Box::new(m_expr(e));
    let list = |l: &[Expression]| l.iter().map(m_expr).collect::<Vec<_>>();
    match e {
        Expression::NamedNode(n) => M::NamedNode(n.clone()),
        Expression::Literal(l) => M::Literal(l.clone()),
        Expression::Variable(v) => M::Variable(v.clone()),
        Expression::Bound(v) => M::Bound(v.clone()),
        Expression::Or(x) => M::Or(list(x)),
        Expression::And(x) => M::And(list(x)),
        Expression::Equal(a, c) => M::Equal(b(a), b(c)),
        Expression::SameTerm(a, c) => M::SameTerm(b(a), b(c)),
        Expression::Greater(a, c) => M::Greater(b(a), b(c)),
        Expression::GreaterOrEqual(a, c) => M::GreaterOrEqual(b(a), b(c)),
        Expression::Less(a, c) => M::Less(b(a), b(c)),
        Expression::LessOrEqual(a, c) => M::LessOrEqual(b(a), b(c)),
        Expression::Arithmetic(first, steps) => M::Arithmetic(
            b(first),
            steps.iter().map(|(op, x)| (*op, m_expr(x))).collect(),
        ),
        Expression::UnaryPlus(x) => M::UnaryPlus(b(x)),
        Expression::UnaryMinus(x) => M::UnaryMinus(b(x)),
        Expression::Not(x) => M::Not(b(x)),
        Expression::In(x, l) => M::In(b(x), list(l)),
        Expression::If(x, y, z) => M::If(b(x), b(y), b(z)),
        Expression::Coalesce(l) => M::Coalesce(list(l)),
        Expression::FunctionCall(f, l) => M::FunctionCall(f.clone(), list(l)),
        Expression::Exists(p) => M::Exists(Box::new(m_pattern(p))),
    }
}

fn m_path(p: &PropertyPathExpression) -> mirror::PropertyPathExpression {
    use mirror::PropertyPathExpression as M;
    let b = |p: &PropertyPathExpression| Box::new(m_path(p));
    match p {
        PropertyPathExpression::NamedNode(n) => M::NamedNode(n.clone()),
        PropertyPathExpression::Reverse(x) => M::Reverse(b(x)),
        PropertyPathExpression::Sequence(l) => M::Sequence(l.iter().map(m_path).collect()),
        PropertyPathExpression::Alternative(l) => M::Alternative(l.iter().map(m_path).collect()),
        PropertyPathExpression::ZeroOrMore(x) => M::ZeroOrMore(b(x)),
        PropertyPathExpression::OneOrMore(x) => M::OneOrMore(b(x)),
        PropertyPathExpression::ZeroOrOne(x) => M::ZeroOrOne(b(x)),
        PropertyPathExpression::NegatedPropertySet(l) => M::NegatedPropertySet(l.clone()),
        PropertyPathExpression::Range { inner, min, max } => M::Range {
            inner: b(inner),
            min: *min,
            max: *max,
        },
        PropertyPathExpression::Wildcard { namespace } => M::Wildcard {
            namespace: namespace.clone(),
        },
    }
}

fn m_order(o: &OrderExpression) -> mirror::OrderExpression {
    match o {
        OrderExpression::Asc(e) => mirror::OrderExpression::Asc(m_expr(e)),
        OrderExpression::Desc(e) => mirror::OrderExpression::Desc(m_expr(e)),
    }
}

fn m_aggregate(a: &AggregateExpression) -> mirror::AggregateExpression {
    mirror::AggregateExpression {
        function: a.function().clone(),
        args: a.args().iter().map(m_expr).collect(),
        scalarvals: a.scalarvals().to_vec(),
        order_by: a.order_by().iter().map(m_order).collect(),
        distinct: a.distinct,
    }
}

fn m_term(t: &TermPattern) -> mirror::TermPattern {
    use mirror::TermPattern as M;
    match t {
        TermPattern::NamedNode(n) => M::NamedNode(n.clone()),
        TermPattern::BlankNode(b) => M::BlankNode(b.clone()),
        TermPattern::Literal(l) => M::Literal(l.clone()),
        TermPattern::Variable(v) => M::Variable(v.clone()),
        TermPattern::Triple(t) => M::Triple(Box::new(m_triple(t))),
    }
}

fn m_triple(t: &TriplePattern) -> mirror::TriplePattern {
    mirror::TriplePattern {
        subject: m_term(&t.subject),
        predicate: t.predicate.clone(),
        object: m_term(&t.object),
    }
}

fn m_ground(g: &GroundTerm) -> mirror::GroundTerm {
    use mirror::GroundTerm as M;
    match g {
        GroundTerm::NamedNode(n) => M::NamedNode(n.clone()),
        GroundTerm::Literal(l) => M::Literal(l.clone()),
        GroundTerm::BlankNode(b) => M::BlankNode(b.clone()),
        GroundTerm::Triple(t) => M::Triple(Box::new(mirror::GroundTriple {
            subject: m_ground(&t.subject),
            predicate: t.predicate.clone(),
            object: m_ground(&t.object),
        })),
    }
}

fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Every single-tree check: both `Debug` forms byte-identical to the mirror's, and a
/// copy whose mirror equals the original's, equal and hashing equally to it.
fn assert_single(p: &GraphPattern) {
    let reference = m_pattern(p);
    assert_eq!(format!("{p:?}"), format!("{reference:?}"), "plain Debug");
    assert_eq!(format!("{p:#?}"), format!("{reference:#?}"), "pretty Debug");
    let copy = p.clone();
    assert_eq!(m_pattern(&copy), reference, "a copy mirrors its original");
    assert!(copy == *p, "a copy equals its original");
    assert_eq!(hash_of(&copy), hash_of(p), "a copy hashes as its original");
}

/// Every pairwise check over `trees`: `==` agrees with the mirrors', and only equal
/// trees share a hash.
fn assert_pairs(trees: &[GraphPattern]) {
    let mirrors: Vec<_> = trees.iter().map(m_pattern).collect();
    let hashes: Vec<_> = trees.iter().map(hash_of).collect();
    // Trees sharing a hash, then every pair compared: unequal hashes must mean
    // unequal trees, which the mirrors confirm.
    let mut by_hash: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, hash) in hashes.iter().enumerate() {
        by_hash.entry(*hash).or_default().push(i);
    }
    for group in by_hash.values() {
        for &i in group {
            for &j in group {
                assert!(
                    trees[i] == trees[j] && mirrors[i] == mirrors[j],
                    "trees {i} and {j} share a hash but are not equal"
                );
            }
        }
    }
    for i in 0..trees.len() {
        for j in 0..trees.len() {
            assert_eq!(
                trees[i] == trees[j],
                mirrors[i] == mirrors[j],
                "`==` of trees {i} and {j} disagrees with the derived reference"
            );
        }
    }
}

// ── the corpus ───────────────────────────────────────────────────────────────────────

const CORPORA: &[&str] = &[
    "crates/sparql-conformance/suite",
    "crates/sparql-conformance/corpus",
    "crates/sparql-algebra/tests/update",
    "vectors/sparql-cdt",
    "vectors/sparql-governors/cases",
    "queries",
    "generated/queries",
];

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a corpus directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rq") {
            out.push(path);
        }
    }
}

/// The root pattern of every corpus query that parses, in sorted path order.
fn corpus_patterns() -> Vec<GraphPattern> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for corpus in CORPORA {
        collect(&root.join(corpus), &mut files);
    }
    files.sort();
    let options = ParserOptions {
        extension_fn_namespaces: vec!["https://example.org/ext/".to_owned()],
        property_fn_namespaces: vec!["https://example.org/rel/".to_owned()],
        property_fn_iris: Vec::new(),
    };
    files
        .iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(path).ok()?;
            let query = SparqlParser::new()
                .with_base_iri("http://example.org/base/")
                .parse_query_with(&text, &options)
                .ok()?;
            Some(match query {
                Query::Select { pattern, .. }
                | Query::Construct { pattern, .. }
                | Query::Describe { pattern, .. }
                | Query::Ask { pattern, .. } => pattern,
            })
        })
        .collect()
}

/// Run `body` on a thread with room for the recursive reference.
fn with_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(body)
        .expect("the thread starts")
        .join()
        .expect("the checks passed");
}

#[test]
fn every_corpus_tree_matches_the_derived_reference() {
    with_stack(|| {
        let trees = corpus_patterns();
        assert!(
            trees.len() > 1000,
            "the corpora hold over a thousand queries"
        );
        for tree in &trees {
            assert_single(tree);
        }
        assert_pairs(&trees);
    });
}

// ── generated trees ──────────────────────────────────────────────────────────────────

fn nn(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://example.org/{local}"))
}

fn leaf_term() -> impl Strategy<Value = TermPattern> {
    prop_oneof![
        (0..3u8).prop_map(|k| TermPattern::Variable(Variable::new(format!("v{k}")))),
        (0..3u8).prop_map(|k| TermPattern::NamedNode(nn(&format!("n{k}")))),
        (0..2u8).prop_map(|k| TermPattern::BlankNode(BlankNode::new(format!("b{k}")))),
        (0..2u8).prop_map(|k| TermPattern::Literal(Literal::new_simple(format!("l\n{k}")))),
        Just(TermPattern::Literal(Literal::new_lang(
            "x",
            "en",
            Some(purrdf_sparql_algebra::BaseDirection::Rtl)
        ))),
    ]
}

fn term() -> impl Strategy<Value = TermPattern> {
    leaf_term().prop_recursive(3, 8, 2, |inner| {
        (inner.clone(), inner).prop_map(|(s, o)| {
            TermPattern::Triple(Child::new(TriplePattern {
                subject: s,
                predicate: NamedNodePattern::NamedNode(nn("p")),
                object: o,
            }))
        })
    })
}

fn ground() -> impl Strategy<Value = GroundTerm> {
    let leaf = prop_oneof![
        (0..3u8).prop_map(|k| GroundTerm::NamedNode(nn(&format!("g{k}")))),
        (0..2u8).prop_map(|k| GroundTerm::Literal(Literal::new_simple(format!("{k}")))),
        Just(GroundTerm::BlankNode(BlankNode::new("gb"))),
    ];
    leaf.prop_recursive(3, 8, 2, |inner| {
        inner.prop_map(|o| {
            GroundTerm::Triple(Child::new(GroundTriple {
                subject: GroundTerm::NamedNode(nn("s")),
                predicate: nn("p"),
                object: o,
            }))
        })
    })
}

fn triple() -> impl Strategy<Value = TriplePattern> {
    (term(), prop::bool::ANY, term()).prop_map(|(subject, by_var, object)| TriplePattern {
        subject,
        predicate: if by_var {
            NamedNodePattern::Variable(Variable::new("p"))
        } else {
            NamedNodePattern::NamedNode(nn("p"))
        },
        object,
    })
}

fn path() -> impl Strategy<Value = PropertyPathExpression> {
    let leaf = prop_oneof![
        (0..3u8).prop_map(|k| PropertyPathExpression::NamedNode(nn(&format!("q{k}")))),
        Just(PropertyPathExpression::NegatedPropertySet(vec![
            NegatedPathElement {
                predicate: nn("x"),
                inverse: true,
            }
        ])),
        Just(PropertyPathExpression::Wildcard { namespace: None }),
        Just(PropertyPathExpression::Wildcard {
            namespace: Some(nn("ns/"))
        }),
    ];
    leaf.prop_recursive(4, 24, 3, |inner| {
        prop_oneof![
            inner
                .clone()
                .prop_map(|a| PropertyPathExpression::Reverse(Child::new(a))),
            inner
                .clone()
                .prop_map(|a| PropertyPathExpression::ZeroOrMore(Child::new(a))),
            inner
                .clone()
                .prop_map(|a| PropertyPathExpression::OneOrMore(Child::new(a))),
            inner
                .clone()
                .prop_map(|a| PropertyPathExpression::ZeroOrOne(Child::new(a))),
            (inner.clone(), 0u32..3, prop::option::of(3u32..5)).prop_map(|(a, min, max)| {
                PropertyPathExpression::Range {
                    inner: Child::new(a),
                    min,
                    max,
                }
            }),
            (
                inner.clone(),
                inner.clone(),
                prop::collection::vec(inner.clone(), 0..2)
            )
                .prop_map(|(a, b, rest)| PropertyPathExpression::Sequence(Chain::new(a, b, rest))),
            (
                inner.clone(),
                inner.clone(),
                prop::collection::vec(inner, 0..2)
            )
                .prop_map(|(a, b, rest)| PropertyPathExpression::Alternative(
                    Chain::new(a, b, rest)
                )),
        ]
    })
}

/// A binary expression node's constructor.
type Operator = fn(Child<Expression>, Child<Expression>) -> Expression;

fn expression_with(pattern: BoxedStrategy<GraphPattern>) -> BoxedStrategy<Expression> {
    let leaf = prop_oneof![
        (0..3u8).prop_map(|k| Expression::Variable(Variable::new(format!("v{k}")))),
        (0..2u8).prop_map(|k| Expression::NamedNode(nn(&format!("e{k}")))),
        (0..2u8).prop_map(|k| Expression::Literal(Literal::new_simple(format!("{k}")))),
        Just(Expression::Bound(Variable::new("v0"))),
    ];
    leaf.prop_recursive(4, 24, 3, move |inner| {
        let binary: [Operator; 6] = [
            Expression::Equal,
            Expression::SameTerm,
            Expression::Greater,
            Expression::GreaterOrEqual,
            Expression::Less,
            Expression::LessOrEqual,
        ];
        prop_oneof![
            (
                inner.clone(),
                inner.clone(),
                prop::collection::vec(inner.clone(), 0..2)
            )
                .prop_map(|(a, b, rest)| Expression::Or(Chain::new(a, b, rest))),
            (
                inner.clone(),
                inner.clone(),
                prop::collection::vec(inner.clone(), 0..2)
            )
                .prop_map(|(a, b, rest)| Expression::And(Chain::new(a, b, rest))),
            (0..6usize, inner.clone(), inner.clone())
                .prop_map(move |(k, a, b)| binary[k](Child::new(a), Child::new(b))),
            (
                inner.clone(),
                prop::collection::vec(
                    (
                        prop_oneof![
                            Just(ArithmeticOperator::Add),
                            Just(ArithmeticOperator::Subtract),
                            Just(ArithmeticOperator::Multiply),
                            Just(ArithmeticOperator::Divide),
                        ],
                        inner.clone()
                    ),
                    1..3
                )
            )
                .prop_map(|(first, steps)| Expression::Arithmetic(
                    Child::new(first),
                    NonEmpty::try_from(steps).expect("one or more steps")
                )),
            inner
                .clone()
                .prop_map(|a| Expression::UnaryPlus(Child::new(a))),
            inner
                .clone()
                .prop_map(|a| Expression::UnaryMinus(Child::new(a))),
            inner.clone().prop_map(|a| Expression::Not(Child::new(a))),
            (inner.clone(), prop::collection::vec(inner.clone(), 0..3))
                .prop_map(|(a, l)| Expression::In(Child::new(a), l.into())),
            (inner.clone(), inner.clone(), inner.clone()).prop_map(|(a, b, c)| Expression::If(
                Child::new(a),
                Child::new(b),
                Child::new(c)
            )),
            prop::collection::vec(inner.clone(), 0..3).prop_map(|l| Expression::Coalesce(l.into())),
            prop::collection::vec(inner, 0..3)
                .prop_map(|l| Expression::FunctionCall(Function::Custom(nn("f")), l.into())),
            pattern
                .clone()
                .prop_map(|p| Expression::Exists(Child::new(p))),
        ]
    })
    .boxed()
}

fn aggregate(expr: BoxedStrategy<Expression>) -> impl Strategy<Value = AggregateExpression> {
    prop_oneof![
        Just(
            AggregateExpression::new(AggregateFunction::Count, vec![], vec![], vec![], true)
                .expect("COUNT(*) is valid")
        ),
        expr.clone().prop_map(|e| {
            AggregateExpression::new(
                AggregateFunction::GroupConcat,
                vec![e],
                vec![("separator".to_owned(), Literal::new_simple("|"))],
                vec![],
                false,
            )
            .expect("GROUP_CONCAT with a separator is valid")
        }),
        (expr.clone(), expr).prop_map(|(e, k)| {
            AggregateExpression::new(
                AggregateFunction::Fold,
                vec![e],
                vec![],
                vec![OrderExpression::Desc(k)],
                false,
            )
            .expect("FOLD with a sort key is valid")
        }),
    ]
}

fn pattern() -> BoxedStrategy<GraphPattern> {
    let leaf = prop_oneof![
        prop::collection::vec(triple(), 0..3).prop_map(|patterns| GraphPattern::Bgp { patterns }),
        (term(), path(), term()).prop_map(|(subject, path, object)| GraphPattern::Path {
            subject,
            path,
            object
        }),
        prop::collection::vec(prop::collection::vec(prop::option::of(ground()), 2), 0..3).prop_map(
            |bindings| GraphPattern::Values {
                variables: vec![Variable::new("a"), Variable::new("b")],
                bindings,
            }
        ),
        (
            prop::collection::vec(term(), 0..2),
            prop::collection::vec(term(), 0..2)
        )
            .prop_map(
                |(subject_args, object_args)| GraphPattern::PropertyFunction(
                    PropertyFunctionCall {
                        iri: "https://example.org/rel/f".to_owned(),
                        subject_args,
                        object_args,
                    }
                )
            ),
    ];
    leaf.prop_recursive(4, 32, 3, |inner| {
        let expr = expression_with(inner.clone().boxed());
        let pair = (inner.clone(), inner.clone());
        prop_oneof![
            pair.clone().prop_map(|(l, r)| GraphPattern::Join {
                left: Child::new(l),
                right: Child::new(r)
            }),
            (inner.clone(), inner.clone(), prop::option::of(expr.clone())).prop_map(
                |(l, r, expression)| GraphPattern::LeftJoin {
                    left: Child::new(l),
                    right: Child::new(r),
                    expression,
                }
            ),
            pair.clone().prop_map(|(l, r)| GraphPattern::Lateral {
                left: Child::new(l),
                right: Child::new(r)
            }),
            pair.prop_map(|(l, r)| GraphPattern::Minus {
                left: Child::new(l),
                right: Child::new(r)
            }),
            (expr.clone(), inner.clone()).prop_map(|(expr, i)| GraphPattern::Filter {
                expr,
                inner: Child::new(i)
            }),
            (
                inner.clone(),
                inner.clone(),
                prop::collection::vec(inner.clone(), 0..2)
            )
                .prop_map(|(a, b, rest)| GraphPattern::Union {
                    arms: Chain::new(a, b, rest)
                }),
            (inner.clone(), prop::bool::ANY).prop_map(|(i, v)| GraphPattern::Graph {
                name: if v {
                    NamedNodePattern::Variable(Variable::new("g"))
                } else {
                    NamedNodePattern::NamedNode(nn("g"))
                },
                inner: Child::new(i),
            }),
            (inner.clone(), expr.clone()).prop_map(|(i, expression)| GraphPattern::Extend {
                inner: Child::new(i),
                variable: Variable::new("x"),
                expression,
            }),
            (inner.clone(), prop::bool::ANY).prop_map(|(i, silent)| GraphPattern::Service {
                name: NamedNodePattern::NamedNode(nn("ep")),
                inner: Child::new(i),
                silent,
            }),
            (inner.clone(), prop::collection::vec(expr.clone(), 1..3)).prop_map(|(i, keys)| {
                GraphPattern::OrderBy {
                    inner: Child::new(i),
                    expression: keys
                        .into_iter()
                        .enumerate()
                        .map(|(k, e)| {
                            if k % 2 == 0 {
                                OrderExpression::Asc(e)
                            } else {
                                OrderExpression::Desc(e)
                            }
                        })
                        .collect(),
                }
            }),
            inner.clone().prop_map(|i| GraphPattern::Project {
                inner: Child::new(i),
                variables: vec![Variable::new("a")],
            }),
            inner.clone().prop_map(|i| GraphPattern::Distinct {
                inner: Child::new(i)
            }),
            inner.clone().prop_map(|i| GraphPattern::Reduced {
                inner: Child::new(i)
            }),
            (inner.clone(), 0usize..3, prop::option::of(0usize..3)).prop_map(
                |(i, start, length)| GraphPattern::Slice {
                    inner: Child::new(i),
                    start,
                    length,
                }
            ),
            (
                inner.clone(),
                prop::collection::vec(aggregate(expr.clone()), 0..2)
            )
                .prop_map(|(i, aggregates)| GraphPattern::Group {
                    inner: Child::new(i),
                    variables: vec![Variable::new("k")],
                    aggregates: aggregates
                        .into_iter()
                        .enumerate()
                        .map(|(k, a)| (Variable::new(format!("agg{k}")), a))
                        .collect(),
                }),
            (inner, expr, prop::option::of(Just(Variable::new("c")))).prop_map(
                |(i, expression, companion)| GraphPattern::Unfold {
                    inner: Child::new(i),
                    expression,
                    element: Variable::new("e"),
                    companion,
                }
            ),
        ]
    })
    .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// A generated tree formats, copies, compares and hashes exactly as the derived
    /// reference does.
    #[test]
    fn a_generated_tree_matches_the_derived_reference(tree in pattern()) {
        assert_single(&tree);
    }

    /// `==` of two generated trees — mostly unequal, sometimes equal — agrees with the
    /// derived reference, and only equal ones share a hash.
    #[test]
    fn generated_pairs_compare_as_the_derived_reference(
        trees in prop::collection::vec(pattern(), 2..6)
    ) {
        let mut trees = trees;
        let copy = trees[0].clone();
        trees.push(copy);
        assert_pairs(&trees);
    }
}
