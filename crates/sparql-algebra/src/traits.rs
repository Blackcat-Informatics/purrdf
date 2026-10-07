// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `Clone`, `==`, `Hash` and `Debug` for the recursive node types, over work lists.
//!
//! `==`, `Hash` and `Debug` all read a node as its **script**: the flat sequence of
//! tokens `#[derive(Debug)]` would print for it — variant names, field names, list
//! lengths, leaf values — with each child left as a token of its own, expanded in
//! turn from the same work list. So:
//!
//! * `Debug` prints the script with the derive's exact punctuation, in both the plain
//!   (`{:?}`) and the pretty (`{:#?}`) form, indenting the pretty form the way the
//!   standard library's builders do;
//! * `==` compares two scripts token by token, which is the derive's comparison: the
//!   same variant, then the same fields in order;
//! * `Hash` feeds every variant name, list length and leaf value in pre-order, which
//!   determines the tree, so unequal trees feed unequal sequences.
//!
//! `Clone` builds the copy bottom-up ([`crate::owned::clone_tree`]).

use core::fmt;
use core::hash::{Hash, Hasher};

use crate::algebra::{
    AggregateExpression, AggregateFunction, ArithmeticOperator, Expression, Function, GraphPattern,
    NegatedPathElement, OrderExpression, PropertyPathExpression,
};
use crate::ast::{
    BlankNode, GroundTerm, Literal, NamedNode, NamedNodePattern, TermPattern, TriplePattern,
    Variable,
};
use crate::owned::{Owned, clone_tree, shallow_ground, shallow_term};
use crate::walk::NodeRef;
use crate::worklist::WorkList;

/// A script's work list: shallow scripts stay inline.
type Tokens<'a> = WorkList<Tok<'a>, 48>;

/// A value a node holds that is not itself a node.
#[derive(Clone, Copy)]
enum Leaf<'a> {
    Var(&'a Variable),
    Vars(&'a [Variable]),
    OptVar(&'a Option<Variable>),
    Named(&'a NamedNode),
    OptNamed(&'a Option<NamedNode>),
    Lit(&'a Literal),
    Blank(&'a BlankNode),
    NamedPattern(&'a NamedNodePattern),
    Func(&'a Function),
    AggFunc(&'a AggregateFunction),
    Op(ArithmeticOperator),
    Bool(bool),
    Usize(usize),
    OptUsize(Option<usize>),
    U32(u32),
    OptU32(Option<u32>),
    Str(&'a str),
    Negated(&'a [NegatedPathElement]),
    Scalarvals(&'a [(String, Literal)]),
    VarPairs(&'a [(Variable, Variable)]),
}

/// Apply `$body` to the value inside any [`Leaf`], bound as `$x`.
macro_rules! each_leaf {
    ($leaf:expr, $x:ident => $body:expr) => {
        match $leaf {
            Leaf::Var($x) => $body,
            Leaf::Vars($x) => $body,
            Leaf::VarPairs($x) => $body,
            Leaf::OptVar($x) => $body,
            Leaf::Named($x) => $body,
            Leaf::OptNamed($x) => $body,
            Leaf::Lit($x) => $body,
            Leaf::Blank($x) => $body,
            Leaf::NamedPattern($x) => $body,
            Leaf::Func($x) => $body,
            Leaf::AggFunc($x) => $body,
            Leaf::Op($x) => $body,
            Leaf::Bool($x) => $body,
            Leaf::Usize($x) => $body,
            Leaf::OptUsize($x) => $body,
            Leaf::U32($x) => $body,
            Leaf::OptU32($x) => $body,
            Leaf::Str($x) => $body,
            Leaf::Negated($x) => $body,
            Leaf::Scalarvals($x) => $body,
        }
    };
}

impl Leaf<'_> {
    const fn tag(self) -> u8 {
        match self {
            Self::Var(_) => 0,
            Self::Vars(_) => 1,
            Self::OptVar(_) => 2,
            Self::Named(_) => 3,
            Self::OptNamed(_) => 4,
            Self::Lit(_) => 5,
            Self::Blank(_) => 6,
            Self::NamedPattern(_) => 7,
            Self::Func(_) => 8,
            Self::AggFunc(_) => 9,
            Self::Op(_) => 10,
            Self::Bool(_) => 11,
            Self::Usize(_) => 12,
            Self::OptUsize(_) => 13,
            Self::U32(_) => 14,
            Self::OptU32(_) => 15,
            Self::Str(_) => 16,
            Self::Negated(_) => 17,
            Self::Scalarvals(_) => 18,
            Self::VarPairs(_) => 19,
        }
    }

    fn same(self, other: Self) -> bool {
        match (self, other) {
            (Self::Var(a), Self::Var(b)) => a == b,
            (Self::Vars(a), Self::Vars(b)) => a == b,
            (Self::VarPairs(a), Self::VarPairs(b)) => a == b,
            (Self::OptVar(a), Self::OptVar(b)) => a == b,
            (Self::Named(a), Self::Named(b)) => a == b,
            (Self::OptNamed(a), Self::OptNamed(b)) => a == b,
            (Self::Lit(a), Self::Lit(b)) => a == b,
            (Self::Blank(a), Self::Blank(b)) => a == b,
            (Self::NamedPattern(a), Self::NamedPattern(b)) => a == b,
            (Self::Func(a), Self::Func(b)) => a == b,
            (Self::AggFunc(a), Self::AggFunc(b)) => a == b,
            (Self::Op(a), Self::Op(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Usize(a), Self::Usize(b)) => a == b,
            (Self::OptUsize(a), Self::OptUsize(b)) => a == b,
            (Self::U32(a), Self::U32(b)) => a == b,
            (Self::OptU32(a), Self::OptU32(b)) => a == b,
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::Negated(a), Self::Negated(b)) => a == b,
            (Self::Scalarvals(a), Self::Scalarvals(b)) => a == b,
            _ => false,
        }
    }

    fn feed<H: Hasher>(self, state: &mut H) {
        self.tag().hash(state);
        each_leaf!(self, x => x.hash(state));
    }
}

impl fmt::Debug for Leaf<'_> {
    /// The leaf's own `Debug`, in the form `f` asks for.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        each_leaf!(self, x => fmt::Debug::fmt(x, f))
    }
}

/// One token of a node's script.
type Tok<'a> = purrdf_lex::walk::Tok<NodeRef<'a>, Leaf<'a>>;

/// Whether two non-node tokens are the same token: the derive's comparison of one
/// script step.
fn same_tok(left: Tok<'_>, right: Tok<'_>) -> bool {
    match (left, right) {
        (Tok::Struct(a), Tok::Struct(b))
        | (Tok::Field(a), Tok::Field(b))
        | (Tok::Tuple(a), Tok::Tuple(b))
        | (Tok::Unit(a), Tok::Unit(b)) => a == b,
        (Tok::List(a), Tok::List(b)) => a == b,
        (Tok::EndStruct, Tok::EndStruct)
        | (Tok::EndTuple, Tok::EndTuple)
        | (Tok::EndList, Tok::EndList) => true,
        (Tok::Leaf(a), Tok::Leaf(b)) => a.same(b),
        _ => false,
    }
}

/// Builds a node's script, in order, onto a buffer.
struct Script<'s, 'a> {
    out: &'s mut Tokens<'a>,
}

impl<'a> Script<'_, 'a> {
    fn push(&mut self, tok: Tok<'a>) -> &mut Self {
        self.out.push(tok);
        self
    }

    fn leaf(&mut self, leaf: Leaf<'a>) -> &mut Self {
        self.push(Tok::Leaf(leaf))
    }

    fn node(&mut self, node: NodeRef<'a>) -> &mut Self {
        self.push(Tok::Node(node))
    }

    fn field(&mut self, name: &'static str) -> &mut Self {
        self.push(Tok::Field(name))
    }

    fn strukt(&mut self, name: &'static str, body: impl FnOnce(&mut Self)) {
        self.push(Tok::Struct(name));
        body(self);
        self.push(Tok::EndStruct);
    }

    fn tuple(&mut self, name: &'static str, body: impl FnOnce(&mut Self)) {
        self.push(Tok::Tuple(name));
        body(self);
        self.push(Tok::EndTuple);
    }

    fn list<I>(&mut self, items: I, mut entry: impl FnMut(&mut Self, I::Item))
    where
        I: IntoIterator,
        I::IntoIter: ExactSizeIterator,
    {
        let items = items.into_iter();
        self.push(Tok::List(items.len()));
        for item in items {
            entry(self, item);
        }
        self.push(Tok::EndList);
    }

    fn nodes<T: 'a>(&mut self, items: &'a [T], node: impl Fn(&'a T) -> NodeRef<'a>) {
        self.list(items, |s, item| {
            s.node(node(item));
        });
    }

    fn option_node(&mut self, node: Option<NodeRef<'a>>) {
        match node {
            None => {
                self.push(Tok::Unit("None"));
            }
            Some(node) => self.tuple("Some", |s| {
                s.node(node);
            }),
        }
    }

    fn tuple_leaf(&mut self, name: &'static str, leaf: Leaf<'a>) {
        self.tuple(name, |s| {
            s.leaf(leaf);
        });
    }

    fn tuple_node(&mut self, name: &'static str, node: NodeRef<'a>) {
        self.tuple(name, |s| {
            s.node(node);
        });
    }

    fn tuple_nodes(&mut self, name: &'static str, a: NodeRef<'a>, b: NodeRef<'a>) {
        self.tuple(name, |s| {
            s.node(a).node(b);
        });
    }

    /// The script `#[derive(Debug)]` prints for `node`, children as tokens.
    fn of(&mut self, node: NodeRef<'a>) {
        match node {
            NodeRef::Pattern(pattern) => self.pattern(pattern),
            NodeRef::Expr(expr) => self.expr(expr),
            NodeRef::Path(path) => self.path(path),
            NodeRef::Triple(triple) => self.triple(triple),
            NodeRef::Term(term) => self.term(term),
            NodeRef::Ground(term) => self.ground(term),
            NodeRef::Order(key) => match key {
                OrderExpression::Asc(e) => self.tuple_node("Asc", NodeRef::Expr(e)),
                OrderExpression::Desc(e) => self.tuple_node("Desc", NodeRef::Expr(e)),
            },
            NodeRef::Aggregate(aggregate) => self.aggregate(aggregate),
        }
    }

    fn triple(&mut self, triple: &'a TriplePattern) {
        self.strukt("TriplePattern", |s| {
            s.field("subject").node(NodeRef::Term(&triple.subject));
            s.field("predicate")
                .leaf(Leaf::NamedPattern(&triple.predicate));
            s.field("object").node(NodeRef::Term(&triple.object));
        });
    }

    fn term(&mut self, term: &'a TermPattern) {
        match term {
            TermPattern::NamedNode(n) => self.tuple_leaf("NamedNode", Leaf::Named(n)),
            TermPattern::BlankNode(b) => self.tuple_leaf("BlankNode", Leaf::Blank(b)),
            TermPattern::Literal(l) => self.tuple_leaf("Literal", Leaf::Lit(l)),
            TermPattern::Variable(v) => self.tuple_leaf("Variable", Leaf::Var(v)),
            TermPattern::Triple(t) => self.tuple_node("Triple", NodeRef::Triple(t)),
        }
    }

    fn ground(&mut self, term: &'a GroundTerm) {
        match term {
            GroundTerm::NamedNode(n) => self.tuple_leaf("NamedNode", Leaf::Named(n)),
            GroundTerm::Literal(l) => self.tuple_leaf("Literal", Leaf::Lit(l)),
            GroundTerm::Triple(t) => self.tuple("Triple", |s| {
                s.strukt("GroundTriple", |s| {
                    s.field("subject").node(NodeRef::Ground(&t.subject));
                    s.field("predicate").leaf(Leaf::Named(&t.predicate));
                    s.field("object").node(NodeRef::Ground(&t.object));
                });
            }),
            GroundTerm::BlankNode(b) => self.tuple_leaf("BlankNode", Leaf::Blank(b)),
        }
    }

    fn aggregate(&mut self, aggregate: &'a AggregateExpression) {
        self.strukt("AggregateExpression", |s| {
            s.field("function").leaf(Leaf::AggFunc(&aggregate.function));
            s.field("args");
            s.nodes(&aggregate.args, NodeRef::Expr);
            s.field("scalarvals")
                .leaf(Leaf::Scalarvals(&aggregate.scalarvals));
            s.field("order_by");
            s.nodes(&aggregate.order_by, NodeRef::Order);
            s.field("distinct").leaf(Leaf::Bool(aggregate.distinct));
        });
    }

    fn pattern(&mut self, pattern: &'a GraphPattern) {
        use GraphPattern as G;
        let binary = |s: &mut Self, name, left: &'a GraphPattern, right: &'a GraphPattern| {
            s.strukt(name, |s| {
                s.field("left").node(NodeRef::Pattern(left));
                s.field("right").node(NodeRef::Pattern(right));
            });
        };
        let inner_only = |s: &mut Self, name, inner: &'a GraphPattern| {
            s.strukt(name, |s| {
                s.field("inner").node(NodeRef::Pattern(inner));
            });
        };
        match pattern {
            G::Bgp { patterns } => self.strukt("Bgp", |s| {
                s.field("patterns");
                s.nodes(patterns, NodeRef::Triple);
            }),
            G::Path {
                subject,
                path,
                object,
            } => self.strukt("Path", |s| {
                s.field("subject").node(NodeRef::Term(subject));
                s.field("path").node(NodeRef::Path(path));
                s.field("object").node(NodeRef::Term(object));
            }),
            G::Join { left, right } => binary(self, "Join", left, right),
            G::LeftJoin {
                left,
                right,
                expression,
            } => self.strukt("LeftJoin", |s| {
                s.field("left").node(NodeRef::Pattern(left));
                s.field("right").node(NodeRef::Pattern(right));
                s.field("expression");
                s.option_node(expression.as_ref().map(NodeRef::Expr));
            }),
            G::Lateral { left, right } => binary(self, "Lateral", left, right),
            G::Apply {
                left,
                right,
                policy,
            } => self.strukt("Apply", |s| {
                s.field("left").node(NodeRef::Pattern(left));
                s.field("right").node(NodeRef::Pattern(right));
                s.field("policy");
                s.strukt("ApplicationPolicy", |s| {
                    s.field("dataset_required")
                        .leaf(Leaf::Bool(policy.dataset_required));
                    s.field("row_pipeline")
                        .leaf(Leaf::Bool(policy.row_pipeline));
                    s.field("reduced_adjacent")
                        .leaf(Leaf::Bool(policy.reduced_adjacent));
                    s.field("group_domain");
                    if let Some(domain) = &policy.group_domain {
                        s.tuple("Some", |s| {
                            s.leaf(Leaf::Vars(domain));
                        });
                    } else {
                        s.push(Tok::Unit("None"));
                    }
                    s.field("inputs").leaf(Leaf::VarPairs(&policy.inputs));
                    s.field("optional");
                    if let Some(optional) = &policy.optional {
                        s.tuple("Some", |s| {
                            s.strukt("OptionalApplication", |s| {
                                s.field("retry_inputs")
                                    .leaf(Leaf::VarPairs(&optional.retry_inputs));
                                s.field("forget_marker")
                                    .leaf(Leaf::Var(&optional.forget_marker));
                            });
                        });
                    } else {
                        s.push(Tok::Unit("None"));
                    }
                });
            }),
            G::Filter { expr, inner } => self.strukt("Filter", |s| {
                s.field("expr").node(NodeRef::Expr(expr));
                s.field("inner").node(NodeRef::Pattern(inner));
            }),
            G::Union { arms } => self.strukt("Union", |s| {
                s.field("arms");
                s.nodes(arms, NodeRef::Pattern);
            }),
            G::Graph { name, inner } => self.strukt("Graph", |s| {
                s.field("name").leaf(Leaf::NamedPattern(name));
                s.field("inner").node(NodeRef::Pattern(inner));
            }),
            G::Extend {
                inner,
                variable,
                expression,
            } => self.strukt("Extend", |s| {
                s.field("inner").node(NodeRef::Pattern(inner));
                s.field("variable").leaf(Leaf::Var(variable));
                s.field("expression").node(NodeRef::Expr(expression));
            }),
            G::Minus { left, right } => binary(self, "Minus", left, right),
            G::Service {
                name,
                inner,
                silent,
            } => self.strukt("Service", |s| {
                s.field("name").leaf(Leaf::NamedPattern(name));
                s.field("inner").node(NodeRef::Pattern(inner));
                s.field("silent").leaf(Leaf::Bool(*silent));
            }),
            G::Values {
                variables,
                bindings,
            } => self.strukt("Values", |s| {
                s.field("variables").leaf(Leaf::Vars(variables));
                s.field("bindings");
                s.list(bindings, |s, row| {
                    s.list(row, |s, cell| {
                        s.option_node(cell.as_ref().map(NodeRef::Ground));
                    });
                });
            }),
            G::OrderBy { inner, expression } => self.strukt("OrderBy", |s| {
                s.field("inner").node(NodeRef::Pattern(inner));
                s.field("expression");
                s.nodes(expression, NodeRef::Order);
            }),
            G::Project { inner, variables } => self.strukt("Project", |s| {
                s.field("inner").node(NodeRef::Pattern(inner));
                s.field("variables").leaf(Leaf::Vars(variables));
            }),
            G::Distinct { inner } => inner_only(self, "Distinct", inner),
            G::Reduced { inner } => inner_only(self, "Reduced", inner),
            G::Slice {
                inner,
                start,
                length,
            } => self.strukt("Slice", |s| {
                s.field("inner").node(NodeRef::Pattern(inner));
                s.field("start").leaf(Leaf::Usize(*start));
                s.field("length").leaf(Leaf::OptUsize(*length));
            }),
            G::Group {
                inner,
                variables,
                aggregates,
            } => self.strukt("Group", |s| {
                s.field("inner").node(NodeRef::Pattern(inner));
                s.field("variables").leaf(Leaf::Vars(variables));
                s.field("aggregates");
                s.list(aggregates, |s, (output, aggregate)| {
                    s.tuple("", |s| {
                        s.leaf(Leaf::Var(output))
                            .node(NodeRef::Aggregate(aggregate));
                    });
                });
            }),
            G::PropertyFunction(call) => self.tuple("PropertyFunction", |s| {
                s.strukt("PropertyFunctionCall", |s| {
                    s.field("iri").leaf(Leaf::Str(&call.iri));
                    s.field("subject_args");
                    s.nodes(&call.subject_args, NodeRef::Term);
                    s.field("object_args");
                    s.nodes(&call.object_args, NodeRef::Term);
                });
            }),
            G::Unfold {
                inner,
                expression,
                element,
                companion,
            } => self.strukt("Unfold", |s| {
                s.field("inner").node(NodeRef::Pattern(inner));
                s.field("expression").node(NodeRef::Expr(expression));
                s.field("element").leaf(Leaf::Var(element));
                s.field("companion").leaf(Leaf::OptVar(companion));
            }),
        }
    }

    fn expr(&mut self, expr: &'a Expression) {
        use Expression as E;
        let pair = |s: &mut Self, name, a: &'a Expression, b: &'a Expression| {
            s.tuple_nodes(name, NodeRef::Expr(a), NodeRef::Expr(b));
        };
        match expr {
            E::NamedNode(n) => self.tuple_leaf("NamedNode", Leaf::Named(n)),
            E::Literal(l) => self.tuple_leaf("Literal", Leaf::Lit(l)),
            E::Variable(v) => self.tuple_leaf("Variable", Leaf::Var(v)),
            E::Bound(v) => self.tuple_leaf("Bound", Leaf::Var(v)),
            E::Or(operands) => self.tuple("Or", |s| s.nodes(operands, NodeRef::Expr)),
            E::And(operands) => self.tuple("And", |s| s.nodes(operands, NodeRef::Expr)),
            E::Equal(a, b) => pair(self, "Equal", a, b),
            E::SameTerm(a, b) => pair(self, "SameTerm", a, b),
            E::Greater(a, b) => pair(self, "Greater", a, b),
            E::GreaterOrEqual(a, b) => pair(self, "GreaterOrEqual", a, b),
            E::Less(a, b) => pair(self, "Less", a, b),
            E::LessOrEqual(a, b) => pair(self, "LessOrEqual", a, b),
            E::Arithmetic(first, steps) => self.tuple("Arithmetic", |s| {
                s.node(NodeRef::Expr(first));
                s.list(steps.iter(), |s, (op, operand)| {
                    s.tuple("", |s| {
                        s.leaf(Leaf::Op(*op)).node(NodeRef::Expr(operand));
                    });
                });
            }),
            E::UnaryPlus(x) => self.tuple_node("UnaryPlus", NodeRef::Expr(x)),
            E::UnaryMinus(x) => self.tuple_node("UnaryMinus", NodeRef::Expr(x)),
            E::Not(x) => self.tuple_node("Not", NodeRef::Expr(x)),
            E::In(x, list) => self.tuple("In", |s| {
                s.node(NodeRef::Expr(x));
                s.nodes(list, NodeRef::Expr);
            }),
            E::If(a, b, c) => self.tuple("If", |s| {
                s.node(NodeRef::Expr(a))
                    .node(NodeRef::Expr(b))
                    .node(NodeRef::Expr(c));
            }),
            E::Coalesce(list) => self.tuple("Coalesce", |s| s.nodes(list, NodeRef::Expr)),
            E::FunctionCall(function, args) => self.tuple("FunctionCall", |s| {
                s.leaf(Leaf::Func(function));
                s.nodes(args, NodeRef::Expr);
            }),
            E::Exists(pattern) => self.tuple_node("Exists", NodeRef::Pattern(pattern)),
        }
    }

    fn path(&mut self, path: &'a PropertyPathExpression) {
        use PropertyPathExpression as P;
        match path {
            P::NamedNode(n) => self.tuple_leaf("NamedNode", Leaf::Named(n)),
            P::Reverse(x) => self.tuple_node("Reverse", NodeRef::Path(x)),
            P::Sequence(elements) => self.tuple("Sequence", |s| s.nodes(elements, NodeRef::Path)),
            P::Alternative(elements) => {
                self.tuple("Alternative", |s| s.nodes(elements, NodeRef::Path));
            }
            P::ZeroOrMore(x) => self.tuple_node("ZeroOrMore", NodeRef::Path(x)),
            P::OneOrMore(x) => self.tuple_node("OneOrMore", NodeRef::Path(x)),
            P::ZeroOrOne(x) => self.tuple_node("ZeroOrOne", NodeRef::Path(x)),
            P::NegatedPropertySet(elements) => {
                self.tuple_leaf("NegatedPropertySet", Leaf::Negated(elements));
            }
            P::Range { inner, min, max } => self.strukt("Range", |s| {
                s.field("inner").node(NodeRef::Path(inner));
                s.field("min").leaf(Leaf::U32(*min));
                s.field("max").leaf(Leaf::OptU32(*max));
            }),
            P::Wildcard { namespace } => self.strukt("Wildcard", |s| {
                s.field("namespace").leaf(Leaf::OptNamed(namespace));
            }),
        }
    }
}

/// Replace the `Node` token on top of `stack` — already popped — by its script, so
/// the script's first token is popped next.
fn expand<'a>(node: NodeRef<'a>, stack: &mut Tokens<'a>) {
    let before = stack.len();
    Script { out: stack }.of(node);
    stack.reverse_top(stack.len() - before);
}

/// `a == b`, as `#[derive(PartialEq)]` answers it, over two work lists.
pub(crate) fn nodes_eq(a: NodeRef<'_>, b: NodeRef<'_>) -> bool {
    let (mut left, mut right) = (Tokens::with(Tok::Node(a)), Tokens::with(Tok::Node(b)));
    loop {
        match (left.pop(), right.pop()) {
            (None, None) => return true,
            (Some(Tok::Node(a)), Some(Tok::Node(b))) => {
                expand(a, &mut left);
                expand(b, &mut right);
            }
            (Some(a), Some(b)) if same_tok(a, b) => {}
            _ => return false,
        }
    }
}

/// Feed the tree under `node` into `state`: in pre-order, every variant name, list
/// length and leaf value. Field names and closing tokens are implied by the variant
/// and the lengths, so they are not fed.
pub(crate) fn node_hash<H: Hasher>(node: NodeRef<'_>, state: &mut H) {
    let mut stack = Tokens::with(Tok::Node(node));
    while let Some(tok) = stack.pop() {
        match tok {
            Tok::Node(node) => expand(node, &mut stack),
            Tok::Struct(name) | Tok::Tuple(name) | Tok::Unit(name) => name.hash(state),
            Tok::List(len) => len.hash(state),
            Tok::Leaf(leaf) => leaf.feed(state),
            Tok::Field(_) | Tok::EndStruct | Tok::EndTuple | Tok::EndList => {}
        }
    }
}

/// `{:?}` / `{:#?}` exactly as `#[derive(Debug)]` writes them, over a work list.
pub(crate) fn node_debug(node: NodeRef<'_>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    purrdf_lex::walk::write_debug(f, node, |node, out: &mut Tokens<'_>| {
        Script { out }.of(node);
    })
}

/// `Clone`, `PartialEq`, `Eq`, `Hash` and `Debug` for a node type, each over a work
/// list rather than recursion.
macro_rules! iterative_traits {
    ($($ty:ty => $kind:ident $(, leaf: $leaf:expr)?;)+) => {$(
        impl Clone for $ty {
            fn clone(&self) -> Self {
                $(if let Some(copy) = ($leaf)(self) {
                    return copy;
                })?
                match clone_tree::<false>(NodeRef::$kind(self), GraphPattern::clone) {
                    Owned::$kind(copy) => copy,
                    _ => unreachable!("a copy is of the kind it copies"),
                }
            }
        }

        impl PartialEq for $ty {
            fn eq(&self, other: &Self) -> bool {
                nodes_eq(NodeRef::$kind(self), NodeRef::$kind(other))
            }
        }

        impl Eq for $ty {}

        impl Hash for $ty {
            fn hash<H: Hasher>(&self, state: &mut H) {
                node_hash(NodeRef::$kind(self), state);
            }
        }

        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                node_debug(NodeRef::$kind(self), f)
            }
        }
    )+};
}

iterative_traits! {
    GraphPattern => Pattern;
    Expression => Expr;
    PropertyPathExpression => Path;
    TermPattern => Term, leaf: |term: &TermPattern| {
        (!matches!(term, TermPattern::Triple(_))).then(|| shallow_term(term))
    };
    GroundTerm => Ground, leaf: |term: &GroundTerm| {
        (!matches!(term, GroundTerm::Triple(_))).then(|| shallow_ground(term))
    };
}
