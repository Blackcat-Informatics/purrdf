// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Walking the algebra tree without recursion.
//!
//! [`NodeRef`] is the one view every walk over a finished tree shares: a borrowed node
//! of any of the algebra's recursive kinds, and [`NodeRef::for_each_child`], the one
//! enumeration of what each kind of node owns. Every whole-tree operation of this
//! crate — `Clone`, `==`, `Hash`, `Debug`, the drop, validation, the retained-size
//! count — reads the tree through it, over an explicit work list, so none of them
//! needs more machine stack for a taller tree.
//!
//! [`walk_pre_post`] and [`fold_post_order`] are the two walks built on it, for
//! analyses outside this crate: the first visits every node before and after its
//! children, the second computes one value per node from its children's values.

use crate::algebra::{AggregateExpression, Expression, GraphPattern, OrderExpression};
use crate::algebra::{Function, PropertyPathExpression};
use crate::ast::{GroundTerm, NamedNodePattern, TermPattern, TriplePattern, Variable};
use crate::worklist::WorkList;

/// A borrowed node of the algebra tree, of any of its recursive kinds.
///
/// A node's children are the nodes it owns, in the order its fields are declared
/// ([`NodeRef::for_each_child`]). Everything else a node holds — variables, IRIs,
/// literals, operators, flags — is a leaf of that node, not a child.
#[derive(Clone, Copy, Debug)]
pub enum NodeRef<'a> {
    /// A graph pattern.
    Pattern(&'a GraphPattern),
    /// An expression.
    Expr(&'a Expression),
    /// A property path.
    Path(&'a PropertyPathExpression),
    /// A triple pattern: a basic graph pattern's element, or a quoted triple term's
    /// triple. Its children are its subject and object terms.
    Triple(&'a TriplePattern),
    /// A term in a pattern. Only a quoted triple term has a child, its triple.
    Term(&'a TermPattern),
    /// A `VALUES` cell. Only a quoted triple term has children, its subject and
    /// object.
    Ground(&'a GroundTerm),
    /// An `ORDER BY` key: its child is its expression.
    Order(&'a OrderExpression),
    /// A `GROUP BY` aggregate: its children are its arguments, then its own `ORDER BY`
    /// keys.
    Aggregate(&'a AggregateExpression),
}

impl NodeRef<'_> {
    /// Visit the variable leaves this node owns directly. Recursive children are
    /// visited separately by the shared walk, so a complete variable census stays
    /// stack safe and includes bindings, graph names and expression-only references.
    pub fn for_each_variable(self, mut visit: impl FnMut(&Variable)) {
        let named = |name: &NamedNodePattern, visit: &mut dyn FnMut(&Variable)| {
            if let NamedNodePattern::Variable(variable) = name {
                visit(variable);
            }
        };
        match self {
            Self::Pattern(pattern) => match pattern {
                GraphPattern::Graph { name, .. } | GraphPattern::Service { name, .. } => {
                    named(name, &mut visit);
                }
                GraphPattern::Extend { variable, .. } => visit(variable),
                GraphPattern::Unfold {
                    element, companion, ..
                } => {
                    visit(element);
                    if let Some(variable) = companion {
                        visit(variable);
                    }
                }
                GraphPattern::Values { variables, .. }
                | GraphPattern::Project { variables, .. } => {
                    variables.iter().for_each(&mut visit);
                }
                GraphPattern::Group {
                    variables,
                    aggregates,
                    ..
                } => {
                    variables.iter().for_each(&mut visit);
                    for (variable, _) in aggregates {
                        visit(variable);
                    }
                }
                GraphPattern::Bgp { .. }
                | GraphPattern::Path { .. }
                | GraphPattern::Join { .. }
                | GraphPattern::Lateral { .. }
                | GraphPattern::Minus { .. }
                | GraphPattern::Union { .. }
                | GraphPattern::LeftJoin { .. }
                | GraphPattern::Filter { .. }
                | GraphPattern::OrderBy { .. }
                | GraphPattern::Distinct { .. }
                | GraphPattern::Reduced { .. }
                | GraphPattern::Slice { .. }
                | GraphPattern::PropertyFunction(_) => {}
            },
            Self::Expr(Expression::Variable(variable) | Expression::Bound(variable))
            | Self::Term(TermPattern::Variable(variable)) => visit(variable),
            Self::Triple(triple) => named(&triple.predicate, &mut visit),
            Self::Expr(
                Expression::NamedNode(_)
                | Expression::Literal(_)
                | Expression::Or(_)
                | Expression::And(_)
                | Expression::Coalesce(_)
                | Expression::Arithmetic(..)
                | Expression::Equal(..)
                | Expression::SameTerm(..)
                | Expression::Greater(..)
                | Expression::GreaterOrEqual(..)
                | Expression::Less(..)
                | Expression::LessOrEqual(..)
                | Expression::Not(_)
                | Expression::UnaryPlus(_)
                | Expression::UnaryMinus(_)
                | Expression::In(..)
                | Expression::If(..)
                | Expression::FunctionCall(..)
                | Expression::Exists(_),
            )
            | Self::Term(
                TermPattern::NamedNode(_)
                | TermPattern::BlankNode(_)
                | TermPattern::Literal(_)
                | TermPattern::Triple(_),
            )
            | Self::Path(_)
            | Self::Ground(_)
            | Self::Order(_)
            | Self::Aggregate(_) => {}
        }
    }

    /// Call `visit` with each child of this node, in the order the node's fields are
    /// declared.
    ///
    /// The enumeration is exhaustive over every variant of every kind, with no
    /// wildcard arm: a variant added later does not compile until its children are
    /// named here, and every walk built on this function then sees them.
    pub fn for_each_child(self, mut visit: impl FnMut(Self)) {
        use Expression as E;
        use GraphPattern as G;
        use PropertyPathExpression as P;
        match self {
            Self::Pattern(pattern) => match pattern {
                G::Bgp { patterns } => patterns.iter().for_each(|t| visit(Self::Triple(t))),
                G::Path {
                    subject,
                    path,
                    object,
                } => {
                    visit(Self::Term(subject));
                    visit(Self::Path(path));
                    visit(Self::Term(object));
                }
                G::Join { left, right } | G::Lateral { left, right } | G::Minus { left, right } => {
                    visit(Self::Pattern(left));
                    visit(Self::Pattern(right));
                }
                G::LeftJoin {
                    left,
                    right,
                    expression,
                } => {
                    visit(Self::Pattern(left));
                    visit(Self::Pattern(right));
                    if let Some(expression) = expression {
                        visit(Self::Expr(expression));
                    }
                }
                G::Filter { expr, inner } => {
                    visit(Self::Expr(expr));
                    visit(Self::Pattern(inner));
                }
                G::Union { arms } => arms.iter().for_each(|arm| visit(Self::Pattern(arm))),
                G::Extend {
                    inner, expression, ..
                }
                | G::Unfold {
                    inner, expression, ..
                } => {
                    visit(Self::Pattern(inner));
                    visit(Self::Expr(expression));
                }
                G::Values { bindings, .. } => bindings
                    .iter()
                    .flatten()
                    .flatten()
                    .for_each(|cell| visit(Self::Ground(cell))),
                G::OrderBy { inner, expression } => {
                    visit(Self::Pattern(inner));
                    for key in expression {
                        visit(Self::Order(key));
                    }
                }
                G::Graph { inner, .. }
                | G::Service { inner, .. }
                | G::Project { inner, .. }
                | G::Distinct { inner }
                | G::Reduced { inner }
                | G::Slice { inner, .. } => visit(Self::Pattern(inner)),
                G::Group {
                    inner, aggregates, ..
                } => {
                    visit(Self::Pattern(inner));
                    for (_, aggregate) in aggregates {
                        visit(Self::Aggregate(aggregate));
                    }
                }
                G::PropertyFunction(call) => call
                    .subject_args
                    .iter()
                    .chain(&call.object_args)
                    .for_each(|term| visit(Self::Term(term))),
            },
            Self::Expr(expr) => match expr {
                E::NamedNode(_) | E::Literal(_) | E::Variable(_) | E::Bound(_) => {}
                E::Or(operands) | E::And(operands) => {
                    operands.iter().for_each(|e| visit(Self::Expr(e)));
                }
                E::Coalesce(operands) | E::FunctionCall(_, operands) => {
                    operands.iter().for_each(|e| visit(Self::Expr(e)));
                }
                E::Arithmetic(first, steps) => {
                    visit(Self::Expr(first));
                    steps.iter().for_each(|(_, e)| visit(Self::Expr(e)));
                }
                E::Equal(a, b)
                | E::SameTerm(a, b)
                | E::Greater(a, b)
                | E::GreaterOrEqual(a, b)
                | E::Less(a, b)
                | E::LessOrEqual(a, b) => {
                    visit(Self::Expr(a));
                    visit(Self::Expr(b));
                }
                E::UnaryPlus(x) | E::UnaryMinus(x) | E::Not(x) => visit(Self::Expr(x)),
                E::In(x, list) => {
                    visit(Self::Expr(x));
                    list.iter().for_each(|e| visit(Self::Expr(e)));
                }
                E::If(a, b, c) => {
                    visit(Self::Expr(a));
                    visit(Self::Expr(b));
                    visit(Self::Expr(c));
                }
                E::Exists(pattern) => visit(Self::Pattern(pattern)),
            },
            Self::Path(path) => match path {
                P::NamedNode(_) | P::NegatedPropertySet(_) | P::Wildcard { .. } => {}
                P::Reverse(x)
                | P::ZeroOrMore(x)
                | P::OneOrMore(x)
                | P::ZeroOrOne(x)
                | P::Range { inner: x, .. } => visit(Self::Path(x)),
                P::Sequence(elements) | P::Alternative(elements) => {
                    elements.iter().for_each(|e| visit(Self::Path(e)));
                }
            },
            Self::Triple(triple) => {
                visit(Self::Term(&triple.subject));
                visit(Self::Term(&triple.object));
            }
            Self::Term(term) => match term {
                TermPattern::NamedNode(_)
                | TermPattern::BlankNode(_)
                | TermPattern::Literal(_)
                | TermPattern::Variable(_) => {}
                TermPattern::Triple(triple) => visit(Self::Triple(triple)),
            },
            Self::Ground(term) => match term {
                GroundTerm::NamedNode(_) | GroundTerm::Literal(_) | GroundTerm::BlankNode(_) => {}
                GroundTerm::Triple(triple) => {
                    visit(Self::Ground(&triple.subject));
                    visit(Self::Ground(&triple.object));
                }
            },
            Self::Order(OrderExpression::Asc(expr) | OrderExpression::Desc(expr)) => {
                visit(Self::Expr(expr));
            }
            Self::Aggregate(aggregate) => {
                aggregate.args().iter().for_each(|e| visit(Self::Expr(e)));
                aggregate
                    .order_by()
                    .iter()
                    .for_each(|key| visit(Self::Order(key)));
            }
        }
    }

    /// Whether this node owns no other node.
    #[must_use]
    pub fn is_leaf(self) -> bool {
        let mut leaf = true;
        self.for_each_child(|_| leaf = false);
        leaf
    }

    /// Whether this node is a quoted triple term: a pattern or `VALUES` triple term,
    /// or a `TRIPLE(…)` call, which builds one around its arguments.
    #[must_use]
    pub fn is_triple_term(self) -> bool {
        matches!(
            self,
            Self::Term(TermPattern::Triple(_))
                | Self::Ground(GroundTerm::Triple(_))
                | Self::Expr(Expression::FunctionCall(Function::Triple, _))
        )
    }
}

/// Which side of a node [`walk_pre_post`] is visiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visit {
    /// Before any of the node's children.
    Enter,
    /// After all of the node's children.
    Exit,
}

/// What a [`walk_pre_post`] visitor asks of the walk after entering a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// Walk into the node's children.
    Descend,
    /// Skip the node's children; the node is still exited.
    Skip,
    /// End the whole walk here; no further node is entered or exited.
    Stop,
}

/// Visit every node under `root`, depth first, children in declaration order: each
/// node once on [`Visit::Enter`] before its children and once on [`Visit::Exit`] after
/// them.
///
/// The visitor's answer on [`Visit::Enter`] steers the walk ([`Flow`]); its answer on
/// [`Visit::Exit`] is read only for [`Flow::Stop`]. The walk keeps its own work list,
/// so it needs no more machine stack for a taller tree. Returns `false` when the
/// visitor stopped it.
pub fn walk_pre_post<'a>(
    root: NodeRef<'a>,
    mut visit: impl FnMut(Visit, NodeRef<'a>) -> Flow,
) -> bool {
    enum Step<'a> {
        Enter(NodeRef<'a>),
        Exit(NodeRef<'a>),
    }
    let mut stack: WorkList<Step<'a>, 32> = WorkList::with(Step::Enter(root));
    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(node) => match visit(Visit::Enter, node) {
                Flow::Stop => return false,
                Flow::Skip => stack.push(Step::Exit(node)),
                Flow::Descend => {
                    stack.push(Step::Exit(node));
                    let first = stack.len();
                    node.for_each_child(|child| stack.push(Step::Enter(child)));
                    stack.reverse_top(stack.len() - first);
                }
            },
            Step::Exit(node) => {
                if visit(Visit::Exit, node) == Flow::Stop {
                    return false;
                }
            }
        }
    }
    true
}

/// Compute one value per node under `root`, each from the node and its children's
/// values, and return the root's.
///
/// `combine` is called once per node, after every child's value exists, with those
/// values in declaration order. The walk keeps its own work list and its own stack
/// of computed values, so it needs no more machine stack for a taller tree.
pub fn fold_post_order<'a, R>(
    root: NodeRef<'a>,
    mut combine: impl FnMut(NodeRef<'a>, &mut dyn Iterator<Item = R>) -> R,
) -> R {
    enum Step<'a> {
        Enter(NodeRef<'a>),
        Exit(NodeRef<'a>, usize),
    }
    let mut stack: WorkList<Step<'a>, 32> = WorkList::with(Step::Enter(root));
    let mut values: WorkList<R, 16> = WorkList::new();
    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(node) => {
                let exit = stack.len();
                stack.push(Step::Exit(node, 0));
                node.for_each_child(|child| stack.push(Step::Enter(child)));
                let count = stack.len() - exit - 1;
                stack.set(exit, Step::Exit(node, count));
                stack.reverse_top(count);
            }
            Step::Exit(node, count) => {
                // The children's values are the top `count` entries, the last
                // child's on top; reversed, they pop in declaration order.
                values.reverse_top(count);
                let mut left = count;
                let mut kids = core::iter::from_fn(|| {
                    (left > 0).then(|| {
                        left -= 1;
                        values.pop().expect("a child's value is computed")
                    })
                });
                let value = combine(node, &mut kids);
                // Values `combine` left unread are dropped here, before the next.
                kids.for_each(drop);
                values.push(value);
            }
        }
    }
    values
        .pop()
        .expect("the root's value is the last one computed")
}
