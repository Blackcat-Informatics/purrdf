// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned nodes on a work list: the iterative drop, and the iterative clone that
//! builds a copy bottom-up.

use crate::algebra::{
    AggregateExpression, Expression, GraphPattern, OrderExpression, PropertyFunctionCall,
    PropertyPathExpression,
};
use crate::ast::{GroundTerm, GroundTriple, TermPattern, TriplePattern};
use crate::tree::{Args, Chain, Child, NonEmpty, Subtree};
use crate::walk::NodeRef;
use crate::worklist::WorkList;

/// How many dismantled nodes the drop keeps inline before its work list spills.
pub(crate) const DROP_INLINE: usize = 8;

/// The drop's work list.
pub(crate) type DropWork = WorkList<Owned, DROP_INLINE>;

/// An owned node of any of the algebra's recursive kinds: an entry of the work list
/// the iterative drop dismantles, or a copy the iterative clone has finished.
#[derive(Debug)]
pub enum Owned {
    /// A graph pattern.
    Pattern(GraphPattern),
    /// An expression.
    Expr(Expression),
    /// A property path.
    Path(PropertyPathExpression),
    /// A triple pattern.
    Triple(TriplePattern),
    /// A term in a pattern.
    Term(TermPattern),
    /// A `VALUES` cell.
    Ground(GroundTerm),
    /// An `ORDER BY` key.
    Order(OrderExpression),
    /// A `GROUP BY` aggregate.
    Aggregate(AggregateExpression),
}

/// Dismantle `work`: take each node's children onto the list before the node itself
/// is dropped, until nothing is left. Every node dropped here owns nothing by then,
/// so no drop it runs recurses.
pub(crate) fn reclaim(work: &mut DropWork) {
    while let Some(mut node) = work.pop() {
        take_children(&mut node, work);
    }
}

pub(crate) fn release_pattern(node: GraphPattern, work: &mut DropWork) {
    work.push(Owned::Pattern(node));
}

pub(crate) fn release_expr(node: Expression, work: &mut DropWork) {
    // The leaf expressions own no node: dropping them here recurses nowhere.
    if !matches!(
        node,
        Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Variable(_)
            | Expression::Bound(_)
    ) {
        work.push(Owned::Expr(node));
    }
}

pub(crate) fn release_path(node: PropertyPathExpression, work: &mut DropWork) {
    if !matches!(
        node,
        PropertyPathExpression::NamedNode(_)
            | PropertyPathExpression::NegatedPropertySet(_)
            | PropertyPathExpression::Wildcard { .. }
    ) {
        work.push(Owned::Path(node));
    }
}

pub(crate) fn release_ground(node: GroundTerm, work: &mut DropWork) {
    if matches!(node, GroundTerm::Triple(_)) {
        work.push(Owned::Ground(node));
    }
}

fn take_child<T: Subtree + purrdf_lex::walk::Dismantle>(child: &mut Child<T>, work: &mut DropWork) {
    if let Some(node) = child.take() {
        (*node).release(work);
    }
}

fn take_chain<T: Subtree>(chain: &mut Chain<T>, work: &mut DropWork) {
    for node in chain.take_nodes() {
        node.release(work);
    }
}

fn take_non_empty<T: Subtree>(list: &mut NonEmpty<T>, work: &mut DropWork) {
    for node in list.take_nodes() {
        node.release(work);
    }
}

fn take_args<T: Subtree>(list: &mut Args<T>, work: &mut DropWork) {
    for node in list.take_nodes() {
        node.release(work);
    }
}

fn take_term(term: &mut TermPattern, work: &mut DropWork) {
    if let TermPattern::Triple(triple) = term {
        take_child(triple, work);
    }
}

fn take_triple(triple: &mut TriplePattern, work: &mut DropWork) {
    take_term(&mut triple.subject, work);
    take_term(&mut triple.object, work);
}

fn take_ground(term: &mut GroundTerm, work: &mut DropWork) {
    if let GroundTerm::Triple(triple) = term {
        take_child(triple, work);
    }
}

fn take_order(key: &mut OrderExpression, work: &mut DropWork) {
    let (OrderExpression::Asc(expr) | OrderExpression::Desc(expr)) = key;
    take_expr(expr, work);
}

fn take_aggregate(aggregate: &mut AggregateExpression, work: &mut DropWork) {
    for arg in core::mem::take(&mut aggregate.args) {
        release_expr(arg, work);
    }
    for key in &mut aggregate.order_by {
        take_order(key, work);
    }
}

/// Move every node `node` owns onto `work`, leaving `node` owning none. Nodes held
/// inline (a `FILTER`'s expression, a path pattern's path, a triple's terms) are
/// emptied in place rather than moved.
fn take_children(node: &mut Owned, work: &mut DropWork) {
    match node {
        Owned::Pattern(pattern) => take_pattern(pattern, work),
        Owned::Expr(expr) => take_expr(expr, work),
        Owned::Path(path) => take_path(path, work),
        Owned::Triple(triple) => take_triple(triple, work),
        Owned::Term(term) => take_term(term, work),
        Owned::Ground(term) => take_ground(term, work),
        Owned::Order(key) => take_order(key, work),
        Owned::Aggregate(aggregate) => take_aggregate(aggregate, work),
    }
}

fn take_pattern(pattern: &mut GraphPattern, work: &mut DropWork) {
    use GraphPattern as G;
    match pattern {
        G::Bgp { patterns } => patterns.iter_mut().for_each(|t| take_triple(t, work)),
        G::Path {
            subject,
            path,
            object,
        } => {
            take_term(subject, work);
            take_path(path, work);
            take_term(object, work);
        }
        G::Join { left, right } | G::Lateral { left, right } | G::Minus { left, right } => {
            take_child(left, work);
            take_child(right, work);
        }
        G::LeftJoin {
            left,
            right,
            expression,
        } => {
            take_child(left, work);
            take_child(right, work);
            if let Some(expression) = expression.take() {
                release_expr(expression, work);
            }
        }
        G::Filter { expr, inner } => {
            take_expr(expr, work);
            take_child(inner, work);
        }
        G::Union { arms } => take_chain(arms, work),
        G::Extend {
            inner, expression, ..
        }
        | G::Unfold {
            inner, expression, ..
        } => {
            take_child(inner, work);
            take_expr(expression, work);
        }
        G::Values { bindings, .. } => bindings
            .iter_mut()
            .flatten()
            .flatten()
            .for_each(|cell| take_ground(cell, work)),
        G::OrderBy { inner, expression } => {
            take_child(inner, work);
            for key in expression.iter_mut() {
                take_order(key, work);
            }
        }
        G::Graph { inner, .. }
        | G::Service { inner, .. }
        | G::Project { inner, .. }
        | G::Distinct { inner }
        | G::Reduced { inner }
        | G::Slice { inner, .. } => take_child(inner, work),
        G::Group {
            inner, aggregates, ..
        } => {
            take_child(inner, work);
            for (_, aggregate) in aggregates.iter_mut() {
                take_aggregate(aggregate, work);
            }
        }
        G::PropertyFunction(call) => call
            .subject_args
            .iter_mut()
            .chain(&mut call.object_args)
            .for_each(|term| take_term(term, work)),
    }
}

fn take_expr(expr: &mut Expression, work: &mut DropWork) {
    use Expression as E;
    match expr {
        E::NamedNode(_) | E::Literal(_) | E::Variable(_) | E::Bound(_) => {}
        E::Or(operands) | E::And(operands) => take_chain(operands, work),
        E::Arithmetic(first, steps) => {
            take_child(first, work);
            take_non_empty(steps, work);
        }
        E::Equal(a, b)
        | E::SameTerm(a, b)
        | E::Greater(a, b)
        | E::GreaterOrEqual(a, b)
        | E::Less(a, b)
        | E::LessOrEqual(a, b) => {
            take_child(a, work);
            take_child(b, work);
        }
        E::UnaryPlus(x) | E::UnaryMinus(x) | E::Not(x) => take_child(x, work),
        E::In(x, list) => {
            take_child(x, work);
            take_args(list, work);
        }
        E::If(a, b, c) => {
            take_child(a, work);
            take_child(b, work);
            take_child(c, work);
        }
        E::Coalesce(list) | E::FunctionCall(_, list) => take_args(list, work),
        E::Exists(pattern) => take_child(pattern, work),
    }
}

fn take_path(path: &mut PropertyPathExpression, work: &mut DropWork) {
    use PropertyPathExpression as P;
    match path {
        P::NamedNode(_) | P::NegatedPropertySet(_) | P::Wildcard { .. } => {}
        P::Reverse(x)
        | P::ZeroOrMore(x)
        | P::OneOrMore(x)
        | P::ZeroOrOne(x)
        | P::Range { inner: x, .. } => take_child(x, work),
        P::Sequence(elements) | P::Alternative(elements) => take_chain(elements, work),
    }
}

/// The finished copies of one node's children, in declaration order, handed out by
/// kind as the node's copy is assembled.
struct Kids<'k> {
    copies: &'k mut CopyStack,
}

macro_rules! kid {
    ($($method:ident => $variant:ident: $ty:ty;)+) => {$(
        fn $method(&mut self) -> $ty {
            match self.copies.pop() {
                Some(Owned::$variant(node)) => node,
                _ => unreachable!("a copy is assembled from its children's copies in order"),
            }
        }
    )+};
}

impl Kids<'_> {
    kid! {
        pattern => Pattern: GraphPattern;
        expr => Expr: Expression;
        path => Path: PropertyPathExpression;
        triple => Triple: TriplePattern;
        term => Term: TermPattern;
        ground => Ground: GroundTerm;
        order => Order: OrderExpression;
        aggregate => Aggregate: AggregateExpression;
    }

    fn patterns<I: IntoIterator>(&mut self, of: I) -> Vec<GraphPattern> {
        of.into_iter().map(|_| self.pattern()).collect()
    }

    fn exprs<I: IntoIterator>(&mut self, of: I) -> Vec<Expression> {
        of.into_iter().map(|_| self.expr()).collect()
    }

    fn paths<I: IntoIterator>(&mut self, of: I) -> Vec<PropertyPathExpression> {
        of.into_iter().map(|_| self.path()).collect()
    }

    fn terms<I: IntoIterator>(&mut self, of: I) -> Vec<TermPattern> {
        of.into_iter().map(|_| self.term()).collect()
    }

    fn orders<I: IntoIterator>(&mut self, of: I) -> Vec<OrderExpression> {
        of.into_iter().map(|_| self.order()).collect()
    }
}

/// A copy of the tree under `root`, built bottom-up over a work list: each node's
/// copy is assembled from its shallow fields and its children's finished copies.
pub(crate) fn clone_tree(root: NodeRef<'_>) -> Owned {
    enum Step<'a> {
        Enter(NodeRef<'a>),
        Exit(NodeRef<'a>, usize),
    }
    let mut stack: WorkList<Step<'_>, 32> = WorkList::with(Step::Enter(root));
    let mut copies = CopyStack::new();
    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(node) => {
                if let Some(copy) = clone_leaf(node) {
                    copies.push(copy);
                    continue;
                }
                let exit = stack.len();
                stack.push(Step::Exit(node, 0));
                node.for_each_child(|child| stack.push(Step::Enter(child)));
                let count = stack.len() - exit - 1;
                stack.set(exit, Step::Exit(node, count));
                stack.reverse_top(count);
            }
            Step::Exit(node, count) => {
                // The children's copies are the top `count` entries, the last child's
                // on top; reversed, they pop in declaration order.
                copies.reverse_top(count);
                let before = copies.len();
                let copy = assemble(
                    node,
                    &mut Kids {
                        copies: &mut copies,
                    },
                );
                debug_assert_eq!(before - copies.len(), count, "every child's copy is used");
                copies.push(copy);
            }
        }
    }
    copies
        .pop()
        .expect("the root's copy is the last one assembled")
}

/// The finished copies waiting to be assembled into their parents'.
type CopyStack = WorkList<Owned, 16>;

/// The copy of a node that owns no other node, made directly; `None` for any other.
fn clone_leaf(node: NodeRef<'_>) -> Option<Owned> {
    Some(match node {
        NodeRef::Expr(
            expr @ (Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Variable(_)
            | Expression::Bound(_)),
        ) => Owned::Expr(shallow_expr(expr)),
        NodeRef::Term(term) if !matches!(term, TermPattern::Triple(_)) => {
            Owned::Term(shallow_term(term))
        }
        NodeRef::Ground(term) if !matches!(term, GroundTerm::Triple(_)) => {
            Owned::Ground(shallow_ground(term))
        }
        _ => return None,
    })
}

pub(crate) fn shallow_expr(expr: &Expression) -> Expression {
    match expr {
        Expression::NamedNode(n) => Expression::NamedNode(n.clone()),
        Expression::Literal(l) => Expression::Literal(l.clone()),
        Expression::Variable(v) => Expression::Variable(v.clone()),
        Expression::Bound(v) => Expression::Bound(v.clone()),
        _ => unreachable!("only a leaf expression is copied shallowly"),
    }
}

pub(crate) fn shallow_term(term: &TermPattern) -> TermPattern {
    match term {
        TermPattern::NamedNode(n) => TermPattern::NamedNode(n.clone()),
        TermPattern::BlankNode(b) => TermPattern::BlankNode(b.clone()),
        TermPattern::Literal(l) => TermPattern::Literal(l.clone()),
        TermPattern::Variable(v) => TermPattern::Variable(v.clone()),
        TermPattern::Triple(_) => unreachable!("only a leaf term is copied shallowly"),
    }
}

pub(crate) fn shallow_ground(term: &GroundTerm) -> GroundTerm {
    match term {
        GroundTerm::NamedNode(n) => GroundTerm::NamedNode(n.clone()),
        GroundTerm::Literal(l) => GroundTerm::Literal(l.clone()),
        GroundTerm::BlankNode(b) => GroundTerm::BlankNode(b.clone()),
        GroundTerm::Triple(_) => unreachable!("only a leaf term is copied shallowly"),
    }
}

/// The copy of `node`: its shallow fields cloned, its children taken from `kids`
/// in the order [`NodeRef::for_each_child`] names them. Struct literals below list
/// their fields in that same order, since that is the order they are evaluated in.
fn assemble(node: NodeRef<'_>, kids: &mut Kids<'_>) -> Owned {
    match node {
        NodeRef::Pattern(pattern) => Owned::Pattern(assemble_pattern(pattern, kids)),
        NodeRef::Expr(expr) => Owned::Expr(assemble_expr(expr, kids)),
        NodeRef::Path(path) => Owned::Path(assemble_path(path, kids)),
        NodeRef::Triple(triple) => Owned::Triple(TriplePattern {
            subject: kids.term(),
            predicate: triple.predicate.clone(),
            object: kids.term(),
        }),
        NodeRef::Term(term) => Owned::Term(match term {
            TermPattern::Triple(_) => TermPattern::Triple(Child::new(kids.triple())),
            leaf => shallow_term(leaf),
        }),
        NodeRef::Ground(term) => Owned::Ground(match term {
            GroundTerm::Triple(triple) => GroundTerm::Triple(Child::new(GroundTriple {
                subject: kids.ground(),
                predicate: triple.predicate.clone(),
                object: kids.ground(),
            })),
            leaf => shallow_ground(leaf),
        }),
        NodeRef::Order(key) => Owned::Order(match key {
            OrderExpression::Asc(_) => OrderExpression::Asc(kids.expr()),
            OrderExpression::Desc(_) => OrderExpression::Desc(kids.expr()),
        }),
        NodeRef::Aggregate(aggregate) => Owned::Aggregate(AggregateExpression {
            function: aggregate.function.clone(),
            args: kids.exprs(&aggregate.args),
            scalarvals: aggregate.scalarvals.clone(),
            order_by: kids.orders(&aggregate.order_by),
            distinct: aggregate.distinct,
        }),
    }
}

fn assemble_pattern(pattern: &GraphPattern, kids: &mut Kids<'_>) -> GraphPattern {
    use GraphPattern as G;
    match pattern {
        G::Bgp { patterns } => G::Bgp {
            patterns: patterns.iter().map(|_| kids.triple()).collect(),
        },
        G::Path { .. } => G::Path {
            subject: kids.term(),
            path: kids.path(),
            object: kids.term(),
        },
        G::Join { .. } => G::Join {
            left: kids.pattern().into(),
            right: kids.pattern().into(),
        },
        G::LeftJoin { expression, .. } => G::LeftJoin {
            left: kids.pattern().into(),
            right: kids.pattern().into(),
            expression: expression.as_ref().map(|_| kids.expr()),
        },
        G::Lateral { .. } => G::Lateral {
            left: kids.pattern().into(),
            right: kids.pattern().into(),
        },
        G::Filter { .. } => G::Filter {
            expr: kids.expr(),
            inner: kids.pattern().into(),
        },
        G::Union { arms } => G::Union {
            arms: Chain::from_vec_unchecked(kids.patterns(arms.iter())),
        },
        G::Graph { name, .. } => G::Graph {
            name: name.clone(),
            inner: kids.pattern().into(),
        },
        G::Extend { variable, .. } => G::Extend {
            inner: kids.pattern().into(),
            variable: variable.clone(),
            expression: kids.expr(),
        },
        G::Minus { .. } => G::Minus {
            left: kids.pattern().into(),
            right: kids.pattern().into(),
        },
        G::Service { name, silent, .. } => G::Service {
            name: name.clone(),
            inner: kids.pattern().into(),
            silent: *silent,
        },
        G::Values {
            variables,
            bindings,
        } => G::Values {
            variables: variables.clone(),
            bindings: bindings
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| cell.as_ref().map(|_| kids.ground()))
                        .collect()
                })
                .collect(),
        },
        G::OrderBy { expression, .. } => G::OrderBy {
            inner: kids.pattern().into(),
            expression: kids.orders(expression),
        },
        G::Project { variables, .. } => G::Project {
            inner: kids.pattern().into(),
            variables: variables.clone(),
        },
        G::Distinct { .. } => G::Distinct {
            inner: kids.pattern().into(),
        },
        G::Reduced { .. } => G::Reduced {
            inner: kids.pattern().into(),
        },
        G::Slice { start, length, .. } => G::Slice {
            inner: kids.pattern().into(),
            start: *start,
            length: *length,
        },
        G::Group {
            variables,
            aggregates,
            ..
        } => G::Group {
            inner: kids.pattern().into(),
            variables: variables.clone(),
            aggregates: aggregates
                .iter()
                .map(|(output, _)| (output.clone(), kids.aggregate()))
                .collect(),
        },
        G::PropertyFunction(call) => G::PropertyFunction(PropertyFunctionCall {
            iri: call.iri.clone(),
            subject_args: kids.terms(&call.subject_args),
            object_args: kids.terms(&call.object_args),
        }),
        G::Unfold {
            element, companion, ..
        } => G::Unfold {
            inner: kids.pattern().into(),
            expression: kids.expr(),
            element: element.clone(),
            companion: companion.clone(),
        },
    }
}

fn assemble_expr(expr: &Expression, kids: &mut Kids<'_>) -> Expression {
    use Expression as E;
    macro_rules! binary {
        ($variant:ident) => {
            E::$variant(kids.expr().into(), kids.expr().into())
        };
    }
    match expr {
        E::NamedNode(_) | E::Literal(_) | E::Variable(_) | E::Bound(_) => shallow_expr(expr),
        E::Or(operands) => E::Or(Chain::from_vec_unchecked(kids.exprs(operands.iter()))),
        E::And(operands) => E::And(Chain::from_vec_unchecked(kids.exprs(operands.iter()))),
        E::Equal(..) => binary!(Equal),
        E::SameTerm(..) => binary!(SameTerm),
        E::Greater(..) => binary!(Greater),
        E::GreaterOrEqual(..) => binary!(GreaterOrEqual),
        E::Less(..) => binary!(Less),
        E::LessOrEqual(..) => binary!(LessOrEqual),
        E::Arithmetic(_, steps) => {
            let first = kids.expr().into();
            let steps = steps.iter().map(|(op, _)| (*op, kids.expr())).collect();
            E::Arithmetic(first, NonEmpty::from_vec_unchecked(steps))
        }
        E::UnaryPlus(_) => E::UnaryPlus(kids.expr().into()),
        E::UnaryMinus(_) => E::UnaryMinus(kids.expr().into()),
        E::Not(_) => E::Not(kids.expr().into()),
        E::In(_, list) => {
            let tested = kids.expr().into();
            E::In(tested, kids.exprs(list.iter()).into())
        }
        E::If(..) => E::If(kids.expr().into(), kids.expr().into(), kids.expr().into()),
        E::Coalesce(list) => E::Coalesce(kids.exprs(list.iter()).into()),
        E::FunctionCall(function, args) => {
            E::FunctionCall(function.clone(), kids.exprs(args.iter()).into())
        }
        E::Exists(_) => E::Exists(kids.pattern().into()),
    }
}

fn assemble_path(path: &PropertyPathExpression, kids: &mut Kids<'_>) -> PropertyPathExpression {
    use PropertyPathExpression as P;
    match path {
        P::NamedNode(n) => P::NamedNode(n.clone()),
        P::Reverse(_) => P::Reverse(kids.path().into()),
        P::Sequence(elements) => {
            P::Sequence(Chain::from_vec_unchecked(kids.paths(elements.iter())))
        }
        P::Alternative(elements) => {
            P::Alternative(Chain::from_vec_unchecked(kids.paths(elements.iter())))
        }
        P::ZeroOrMore(_) => P::ZeroOrMore(kids.path().into()),
        P::OneOrMore(_) => P::OneOrMore(kids.path().into()),
        P::ZeroOrOne(_) => P::ZeroOrOne(kids.path().into()),
        P::NegatedPropertySet(elements) => P::NegatedPropertySet(elements.clone()),
        P::Range { min, max, .. } => P::Range {
            inner: kids.path().into(),
            min: *min,
            max: *max,
        },
        P::Wildcard { namespace } => P::Wildcard {
            namespace: namespace.clone(),
        },
    }
}
