// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The **substituted** plan, retained across runs of one prepared execution.
//!
//! [`PreparedExecution`](crate::PreparedExecution) caches the parse and the
//! admission. It did not cache the *rewrite*: every run cloned the admitted algebra
//! and walked it again to push each pre-bound constant into the patterns that can
//! carry it and to plant the `VALUES` seed. For a SHACL validation that is once per
//! focus node, for a tree that is the same tree every time except for the cells the
//! caller's values sit in. `docs/design/purrdf-change-path-allocations.md` names the
//! artifact that closes it: *"the substituted shape decided once per query and per
//! pre-bound variable name, with only the values written per focus node."*
//!
//! This module is that artifact. A [`PrebindMemo`] holds the rewritten [`Query`] and
//! a list of [`Target`]s — the positions in it that hold a caller-supplied value —
//! so a run writes cells into a retained tree instead of building a new one.
//!
//! # Why the memo is keyed on value SHAPES and not on names alone
//!
//! The design note states the soundness premise: the pushdown's boundary is *"a
//! function of the query and the names of the pre-bound variables, both constant
//! across focus nodes."* That is true of the boundary — [`crate::substitute`]'s
//! descent stops at an `OPTIONAL`'s and a `MINUS`'s right arm by matching on the
//! `GraphPattern` variant, never on a value — but it is NOT the whole story about
//! which cells exist, and the difference is already live in the crate:
//! `term_pattern_from_ground` refuses a [`GroundTerm::BlankNode`], because a blank in
//! a pattern is an anonymous variable rather than a request to match that blank. So a
//! blank-node focus node is bound through `VALUES` rows alone — the seed, and the
//! one-row driver a property-function call naming it is given — while an IRI one is
//! written into the leaves themselves. The pushed set and the seed set are different
//! sets, and which positions exist depends on which of a small, closed set of
//! *shapes* each value has.
//!
//! [`ValueShape`] is that closed set, and it is the memo's key alongside the lane.
//! A run whose values have the memo's shapes writes into the retained tree; a run
//! whose values do not takes the ordinary per-run rewrite, unchanged. That is how a
//! focus set mixing IRI and blank-node nodes cannot corrupt the memo: the blank ones
//! never reach it.
//!
//! # Why a memo is never believed, only checked
//!
//! A memo that answers *differently* from the rewrite it stands in for would be a
//! silent wrong answer — the worst failure this path has. So the target list is not
//! derived by a second reading of [`crate::substitute`]'s rules, which would be a
//! reimplementation free to drift from them. It is **observed**: [`PrebindMemo::build`]
//! runs the real rewrite several times over, varying one parameter at a time, and
//! takes the cells that MOVED. A cell the rewrite does not write cannot move, and a
//! cell it writes must. The memo is then accepted only if replaying a different value
//! set into it reproduces, node for node, the tree the real rewrite produces for that
//! same value set. A memo that fails that check is discarded and the run takes the
//! ordinary path.

use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, GroundTerm, GroundTriple,
    Literal, NamedNode, NamedNodePattern, OrderExpression, Query, TermPattern, TriplePattern,
    Variable,
};
use purrdf_sparql_algebra::{Args, Child};

use crate::engine::ShaclPrebinding;
use crate::substitute::{
    apply_probes, apply_shacl_probes, expression_from_ground, has_repeated_variable,
    named_node_from_ground, term_pattern_from_ground,
};

/// The already-grounded pre-binding list both halves of the rewrite consume.
type Probes = [(Variable, GroundTerm)];

/// Everything about a pre-bound VALUE that the rewrite branches on.
///
/// The rewrite reads a value in exactly four places, and every one of them asks a
/// question this enum answers:
///
/// * `term_pattern_from_ground` — may the constant be written into a triple-pattern
///   position at all? No for a blank node, and no for a quoted triple with a blank
///   anywhere inside it.
/// * `expression_from_ground` — has the constant an expression form? Only an IRI or a
///   literal has one; a blank node and a quoted triple ride the `VALUES` seed.
/// * `substitute_in_term_pattern` — a property function's argument takes an IRI or a
///   literal as a written constant; a blank node there is bound by the one-row
///   `VALUES` that drives the call instead, which is a `VALUES` cell like the seed's.
/// * `substitute_in_named_node_pattern` — a `GRAPH`/`SERVICE` name takes an IRI and
///   nothing else.
///
/// Two values with the same shape therefore produce the same TREE, differing only in
/// the cells they occupy, which is exactly the premise a memo needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueShape {
    /// An IRI: pushed into patterns, substituted in expressions, and the one shape a
    /// `GRAPH` name accepts.
    Iri,
    /// A literal: pushed into patterns and substituted in expressions, but never a
    /// graph name.
    Literal,
    /// Bound through `VALUES` rows and written into no pattern — a blank node, or a
    /// quoted triple with a blank node somewhere inside it. The seed carries it, and so
    /// does the one-row driver of any property-function call that names it.
    SeedOnly,
    /// A quoted triple every position of which is writable into a pattern.
    ///
    /// Carried as its own shape rather than folded into [`Self::Iri`] because it is
    /// the one shape a memo REFUSES, and refusing it needs a name. Writing a quoted
    /// triple into a term position changes how many nested term positions that
    /// position contains, and the memo's cell numbering counts term positions — so
    /// the numbering would become a function of the VALUE rather than of the query
    /// and the parameter names, which is the property the whole artifact rests on.
    /// Such a run takes the ordinary rewrite and answers identically; it is a memo
    /// that is declined, not a query that is refused.
    NestedTriple,
}

impl ValueShape {
    /// The shape of one grounded pre-binding.
    fn of(ground: &GroundTerm) -> Self {
        match ground {
            GroundTerm::NamedNode(_) => Self::Iri,
            GroundTerm::Literal(_) => Self::Literal,
            GroundTerm::BlankNode(_) => Self::SeedOnly,
            // Asked of the real predicate rather than re-derived: a quoted triple is
            // writable exactly when `term_pattern_from_ground` says it is, and that
            // function recurses through every nested position looking for a blank.
            GroundTerm::Triple(_) => {
                if term_pattern_from_ground(ground).is_some() {
                    Self::NestedTriple
                } else {
                    Self::SeedOnly
                }
            }
        }
    }
}

/// One position in the substituted algebra that holds a caller-supplied value.
#[derive(Clone, Copy, Debug)]
struct Target {
    /// The position's ordinal in [`walk_query`]'s fixed traversal order.
    cell: u32,
    /// Which pre-binding's value belongs here.
    parameter: u32,
}

/// A position the rewrite can write a value into, borrowed for writing.
///
/// Four variants because the rewrite writes four different things: a term into a
/// pattern position, a cell of a `VALUES` row, a constant expression, and a
/// `GRAPH`/`SERVICE` name.
enum Cell<'a> {
    /// A triple pattern's, property path's or property-function argument's term.
    Term(&'a mut TermPattern),
    /// One cell of one `VALUES` row.
    Ground(&'a mut Option<GroundTerm>),
    /// An expression node, which the SHACL lane replaces wholesale.
    Expr(&'a mut Expression),
    /// A `GRAPH` or `SERVICE` name.
    GraphName(&'a mut NamedNodePattern),
}

impl Cell<'_> {
    /// An owned copy of what this position currently holds, for differencing two
    /// rewrites of the same query.
    fn snapshot(&self) -> CellValue {
        match self {
            Self::Term(term) => CellValue::Term((*term).clone()),
            Self::Ground(slot) => CellValue::Ground((*slot).clone()),
            Self::Expr(expr) => CellValue::Expr((*expr).clone()),
            Self::GraphName(name) => CellValue::GraphName((*name).clone()),
        }
    }
}

/// [`Cell`]'s contents, owned. Build-time only: differencing two trees needs both
/// readings at once, and only one of them can be a live borrow.
#[derive(Clone, Debug, PartialEq)]
enum CellValue {
    /// See [`Cell::Term`].
    Term(TermPattern),
    /// See [`Cell::Ground`].
    Ground(Option<GroundTerm>),
    /// See [`Cell::Expr`].
    Expr(Expression),
    /// See [`Cell::GraphName`].
    GraphName(NamedNodePattern),
}

impl CellValue {
    /// Whether a real rewrite could ever write THIS WHOLE position as one atomic
    /// replacement, rather than only ever recursing through it to reach positions
    /// further in.
    ///
    /// Wildcard-free, and it has to mirror exactly where [`walk_term`] descends and
    /// which expressions [`for_each_child_slot`] gives children: a position this
    /// says is a leaf is exactly a position the walk does not descend past, and a
    /// position this says is not is exactly one it does.
    ///
    /// This exists because [`Cell::snapshot`] clones a position's WHOLE subtree,
    /// not just its own immediate content — so when a leaf changes, every
    /// ancestor's snapshot changes too, purely because the leaf sits inside it.
    /// Without this check, [`PrebindMemo::build`]'s diff would record every one of
    /// those ancestors as ITS OWN [`Target`] — a `TermPattern::Triple` whose
    /// nested `$this` moved, or the `Equal`/`Bound` wrapping a pre-bound variable
    /// in a `FILTER` — even though the real rewrite (`probe_term_pattern`,
    /// `substitute_in_expression`) never overwrites any of those wholesale; it
    /// only ever recurses through them to the `TermPattern::Variable` or
    /// `Expression::Variable`/`Expression::Bound` leaf inside. Writing a replay
    /// value into such an ancestor would replace that whole subtree — deleting
    /// the very leaf position the recursion exists to reach, which is exactly
    /// what made the traversal in [`write_values`] find fewer positions than were
    /// recorded for it.
    fn is_leaf(&self) -> bool {
        match self {
            Self::Term(TermPattern::Triple(_)) => false,
            Self::Term(_) | Self::Ground(_) | Self::GraphName(_) => true,
            Self::Expr(expr) => matches!(
                expr,
                Expression::Variable(_)
                    | Expression::Bound(_)
                    | Expression::NamedNode(_)
                    | Expression::Literal(_)
            ),
        }
    }
}

/// What a caller's value becomes at `cell`.
///
/// Every arm is the SAME constructor [`crate::substitute`] writes with, so a replay
/// cannot spell a constant differently from the rewrite it stands in for.
///
/// # Panics
///
/// If `ground`'s [`ValueShape`] is not the shape the target was recorded under. A
/// target for a term position exists only because `term_pattern_from_ground`
/// answered `Some` when the memo was built, and [`PrebindMemo::matches`] refuses a
/// run whose shapes differ from the build's — so this cannot fire without one of
/// those two having gone wrong, and quietly leaving the previous run's value in the
/// cell is precisely the silent wrong answer the memo must not be able to produce.
fn write_cell(cell: Cell<'_>, ground: &GroundTerm) {
    match cell {
        Cell::Term(term) => {
            *term = term_pattern_from_ground(ground)
                .expect("a term target is only recorded for a value shape a pattern can carry");
        }
        Cell::Ground(slot) => *slot = Some(ground.clone()),
        Cell::Expr(expr) => {
            *expr = expression_from_ground(ground)
                .expect("an expression target is only recorded for an IRI or a literal");
        }
        Cell::GraphName(name) => {
            *name = NamedNodePattern::NamedNode(
                named_node_from_ground(ground)
                    .expect("a graph-name target is only recorded for an IRI"),
            );
        }
    }
}

/// Run the pre-binding rewrite `lane` names, over already-grounded probes.
///
/// The memo calls THIS rather than a copy of it, so "what the memo stands in for" and
/// "what the engine would otherwise have run" are the same two function calls.
pub(crate) fn rewrite(
    query: Query,
    lane: ShaclPrebinding,
    probes: Vec<(Variable, GroundTerm)>,
) -> Query {
    match lane {
        ShaclPrebinding::Applied => apply_shacl_probes(query, probes),
        ShaclPrebinding::None => apply_probes(query, probes),
    }
}

/// Visit every position in `query` a pre-binding rewrite can write a value into, in
/// one fixed order, handing each a running ordinal.
///
/// **This is the only enumeration of those positions.** Recording a target and
/// replaying into it both go through here, so the two cannot disagree about which
/// position ordinal `n` is: they are literally the same traversal. A `GraphPattern`
/// or `Expression` variant added later fails to compile here — the matches of
/// [`visit_own_cells`] and [`for_each_child_slot`] are wildcard-free — rather than
/// silently shifting every ordinal after it.
///
/// # What makes the ordinals stable across runs
///
/// An ordinal is assigned to a position, never to a node reached THROUGH one. A
/// written value is a leaf in every case the memo admits ([`ValueShape::NestedTriple`]
/// is declined for exactly this reason), and the position it replaces — a
/// `TermPattern::Variable`, an `Expression::Variable`, an `Expression::Bound`, a
/// `VALUES` cell, a `NamedNodePattern::Variable` — is a leaf too. So writing a value
/// changes what a position holds and never how many positions there are.
fn walk_query(query: &mut Query, visit: &mut dyn FnMut(u32, Cell<'_>)) -> u32 {
    let mut index = 0;
    match query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => walk_pattern(pattern, &mut index, visit),
    }
    index
}

/// [`walk_query`]'s graph-pattern walk.
///
/// The arms of [`visit_own_cells`] and [`for_each_child_slot`] together mirror
/// `crate::substitute::substitute_in_graph_pattern`'s arms one for one, plus the term
/// positions of the three leaves that walk does not enter because the PUSHDOWN half of
/// the rewrite owns them (`Bgp`, `Path`) and the cells of a `VALUES` block, which is
/// where the seed's row lives.
///
/// # How the walk moves through the tree
///
/// The walk keeps its own work list and needs no more machine stack for a deeper
/// tree. Each node is **moved out** of its slot — a placeholder that allocates nothing
/// stands in the slot meanwhile — and entered: its own cells are visited in place, then
/// every child slot is emptied the same way onto the work list, in numbering order,
/// below an exit step for the node. When the node's exit step is reached every child has
/// come back through the value stack, in the same order, and is written back into the
/// slot it came from; the node itself then joins the value stack for ITS parent. The
/// root is written back last. A tree walked this way is the same tree afterwards, cell
/// for cell, apart from what the visitor wrote.
///
/// Moving is what lets an aggregate be walked at all: an [`AggregateExpression`]'s
/// expressions are reachable only through its consuming `into_parts`, so the walk holds
/// the parts it took apart while their expressions are out, and rebuilds the aggregate
/// through the checked constructor when they return. The placeholder is a `COUNT(*)`,
/// whose empty argument list allocates nothing.
fn walk_pattern(root: &mut GraphPattern, index: &mut u32, visit: &mut dyn FnMut(u32, Cell<'_>)) {
    let mut steps: smallvec::SmallVec<[Step; 16]> = smallvec::smallvec![Step::Enter(
        Node::Pattern(std::mem::replace(root, pattern_placeholder()))
    )];
    let mut returned: smallvec::SmallVec<[Node; 16]> = smallvec::SmallVec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(node) => {
                let mut shell = match node {
                    Node::Pattern(mut pattern) => {
                        visit_own_cells(&mut pattern, index, visit);
                        Shell::Pattern(pattern)
                    }
                    // The node itself is numbered BEFORE its children, because the SHACL
                    // lane replaces a whole `Expression::Variable` or `Expression::Bound`
                    // node rather than editing one.
                    Node::Expression(mut expression) => {
                        let here = *index;
                        *index += 1;
                        visit(here, Cell::Expr(&mut expression));
                        Shell::Expression(expression)
                    }
                    Node::Aggregate(aggregate) => {
                        let (function, args, scalarvals, order_by, distinct) =
                            aggregate.into_parts();
                        Shell::Aggregate {
                            function,
                            args,
                            scalarvals,
                            order_by,
                            distinct,
                        }
                    }
                };
                // The children go on the work list in numbering order, first child on
                // top, and the node's exit step beneath them all.
                let first = steps.len();
                for_each_child_slot(&mut shell, &mut |slot| steps.push(Step::Enter(take(slot))));
                let children = steps.len() - first;
                steps[first..].reverse();
                steps.insert(first, Step::Exit { shell, children });
            }
            Step::Exit {
                mut shell,
                children,
            } => {
                let first = returned.len() - children;
                {
                    let mut back = returned.drain(first..);
                    for_each_child_slot(&mut shell, &mut |slot| {
                        put(
                            slot,
                            back.next()
                                .expect("every child taken out of a node comes back to it"),
                        );
                    });
                    let extra = back.next();
                    assert!(
                        extra.is_none(),
                        "a node takes back exactly the children it gave out"
                    );
                }
                returned.push(match shell {
                    Shell::Pattern(pattern) => Node::Pattern(pattern),
                    Shell::Expression(expression) => Node::Expression(expression),
                    Shell::Aggregate {
                        function,
                        args,
                        scalarvals,
                        order_by,
                        distinct,
                    } => Node::Aggregate(
                        AggregateExpression::new(function, args, scalarvals, order_by, distinct)
                            .expect(
                                "visiting an argument changes no argument count, so arity stays \
                                 valid",
                            ),
                    ),
                });
            }
        }
    }
    match returned.pop() {
        Some(Node::Pattern(pattern)) => *root = pattern,
        _ => unreachable!("the root pattern is the last node handed back"),
    }
}

/// One step of [`walk_pattern`]'s work list.
enum Step {
    /// Enter a node taken out of its slot.
    Enter(Node),
    /// Write a node's `children` back into it from the value stack, and hand the node
    /// back to its own parent.
    Exit { shell: Shell, children: usize },
}

/// A node moved out of its slot: on its way into the walk, or back to its slot.
enum Node {
    Pattern(GraphPattern),
    Expression(Expression),
    Aggregate(AggregateExpression),
}

/// A node whose children are out being walked.
enum Shell {
    Pattern(GraphPattern),
    Expression(Expression),
    /// An aggregate taken apart with `into_parts`, its expressions out being walked.
    Aggregate {
        function: AggregateFunction,
        args: Vec<Expression>,
        scalarvals: Vec<(String, Literal)>,
        order_by: Vec<OrderExpression>,
        distinct: bool,
    },
}

/// One child slot of a node.
enum Slot<'a> {
    Pattern(&'a mut GraphPattern),
    Expression(&'a mut Expression),
    Aggregate(&'a mut AggregateExpression),
}

/// A pattern with no cells and no children, left in a slot whose node is out being
/// walked. Allocates nothing.
fn pattern_placeholder() -> GraphPattern {
    GraphPattern::Bgp {
        patterns: Vec::new(),
    }
}

/// An expression with no children, left in a slot whose node is out being walked.
/// Allocates nothing.
fn expression_placeholder() -> Expression {
    Expression::Coalesce(Args::new())
}

/// Take the node out of `slot`, leaving a placeholder.
fn take(slot: Slot<'_>) -> Node {
    match slot {
        Slot::Pattern(pattern) => Node::Pattern(std::mem::replace(pattern, pattern_placeholder())),
        Slot::Expression(expression) => {
            Node::Expression(std::mem::replace(expression, expression_placeholder()))
        }
        Slot::Aggregate(aggregate) => Node::Aggregate(std::mem::replace(aggregate, count_star())),
    }
}

/// Put `node` back into `slot`, the slot it was taken from.
fn put(slot: Slot<'_>, node: Node) {
    match (slot, node) {
        (Slot::Pattern(slot), Node::Pattern(pattern)) => *slot = pattern,
        (Slot::Expression(slot), Node::Expression(expression)) => *slot = expression,
        (Slot::Aggregate(slot), Node::Aggregate(aggregate)) => *slot = aggregate,
        (Slot::Pattern(_) | Slot::Expression(_) | Slot::Aggregate(_), _) => {
            unreachable!("children come back in the order their slots are enumerated")
        }
    }
}

/// Visit the cells a pattern node holds ITSELF, before any child: a `Bgp`'s, a `Path`'s
/// and a property-function call's term positions, a `VALUES` block's cells, and a
/// `GRAPH`/`SERVICE` name. Every other variant holds only children.
///
/// Wildcard-free: a `GraphPattern` variant added later fails to compile here rather
/// than silently shifting every ordinal after it.
fn visit_own_cells(
    pattern: &mut GraphPattern,
    index: &mut u32,
    visit: &mut dyn FnMut(u32, Cell<'_>),
) {
    match pattern {
        GraphPattern::Bgp { patterns } => {
            for triple in patterns.iter_mut() {
                walk_term(&mut triple.subject, index, visit);
                // The predicate is a `NamedNodePattern` that neither half of the
                // rewrite writes: the pushdown visits subject and object only, and
                // the expression walk does not enter a `Bgp` at all.
                walk_term(&mut triple.object, index, visit);
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            walk_term(subject, index, visit);
            walk_term(object, index, visit);
        }
        GraphPattern::Values { bindings, .. } => {
            for row in bindings.iter_mut() {
                for slot in row.iter_mut() {
                    let here = *index;
                    *index += 1;
                    visit(here, Cell::Ground(slot));
                }
            }
        }
        GraphPattern::Graph { name, .. } | GraphPattern::Service { name, .. } => {
            let here = *index;
            *index += 1;
            visit(here, Cell::GraphName(name));
        }
        GraphPattern::PropertyFunction(call) => {
            for term in call
                .subject_args
                .iter_mut()
                .chain(call.object_args.iter_mut())
            {
                walk_term(term, index, visit);
            }
        }
        GraphPattern::Join { .. }
        | GraphPattern::Lateral { .. }
        | GraphPattern::Minus { .. }
        | GraphPattern::Union { .. }
        | GraphPattern::LeftJoin { .. }
        | GraphPattern::Filter { .. }
        | GraphPattern::Extend { .. }
        | GraphPattern::Unfold { .. }
        | GraphPattern::OrderBy { .. }
        | GraphPattern::Project { .. }
        | GraphPattern::Distinct { .. }
        | GraphPattern::Reduced { .. }
        | GraphPattern::Slice { .. }
        | GraphPattern::Group { .. } => {}
    }
}

/// Call `f` with every child slot of `shell`, in numbering order.
///
/// A pattern's children are its sub-patterns and the expressions it evaluates, in the
/// order the rewrite reads them; a `GROUP BY`'s aggregates are children of their own,
/// each holding its arguments and then its own `ORDER BY` keys. An expression's
/// children are its operands, left to right, and the pattern inside an `EXISTS`.
///
/// Wildcard-free, both matches: a variant added later fails to compile here.
fn for_each_child_slot(shell: &mut Shell, f: &mut dyn FnMut(Slot<'_>)) {
    match shell {
        Shell::Pattern(pattern) => match pattern {
            GraphPattern::Bgp { .. }
            | GraphPattern::Path { .. }
            | GraphPattern::Values { .. }
            | GraphPattern::PropertyFunction(_) => {}
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Minus { left, right } => {
                f(Slot::Pattern(left));
                f(Slot::Pattern(right));
            }
            GraphPattern::Union { arms } => {
                for arm in arms.iter_mut() {
                    f(Slot::Pattern(arm));
                }
            }
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                f(Slot::Pattern(left));
                f(Slot::Pattern(right));
                if let Some(expression) = expression {
                    f(Slot::Expression(expression));
                }
            }
            GraphPattern::Filter { expr, inner } => {
                f(Slot::Expression(expr));
                f(Slot::Pattern(inner));
            }
            GraphPattern::Graph { inner, .. } | GraphPattern::Service { inner, .. } => {
                f(Slot::Pattern(inner));
            }
            GraphPattern::Extend {
                inner, expression, ..
            }
            | GraphPattern::Unfold {
                inner, expression, ..
            } => {
                f(Slot::Pattern(inner));
                f(Slot::Expression(expression));
            }
            GraphPattern::OrderBy { inner, expression } => {
                f(Slot::Pattern(inner));
                for order in expression.iter_mut() {
                    f(Slot::Expression(order_key(order)));
                }
            }
            GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => f(Slot::Pattern(inner)),
            GraphPattern::Group {
                inner, aggregates, ..
            } => {
                f(Slot::Pattern(inner));
                for (_, aggregate) in aggregates.iter_mut() {
                    f(Slot::Aggregate(aggregate));
                }
            }
        },
        Shell::Expression(expression) => match expression {
            Expression::Variable(_)
            | Expression::Bound(_)
            | Expression::NamedNode(_)
            | Expression::Literal(_) => {}
            Expression::Or(operands) | Expression::And(operands) => {
                for operand in operands.iter_mut() {
                    f(Slot::Expression(operand));
                }
            }
            Expression::Arithmetic(first, steps) => {
                f(Slot::Expression(first));
                for (_, operand) in steps.iter_mut() {
                    f(Slot::Expression(operand));
                }
            }
            Expression::Equal(left, right)
            | Expression::SameTerm(left, right)
            | Expression::Greater(left, right)
            | Expression::GreaterOrEqual(left, right)
            | Expression::Less(left, right)
            | Expression::LessOrEqual(left, right) => {
                f(Slot::Expression(left));
                f(Slot::Expression(right));
            }
            Expression::UnaryPlus(inner)
            | Expression::UnaryMinus(inner)
            | Expression::Not(inner) => {
                f(Slot::Expression(inner));
            }
            Expression::In(target, list) => {
                f(Slot::Expression(target));
                for item in list.iter_mut() {
                    f(Slot::Expression(item));
                }
            }
            Expression::If(cond, then_expr, else_expr) => {
                f(Slot::Expression(cond));
                f(Slot::Expression(then_expr));
                f(Slot::Expression(else_expr));
            }
            Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
                for item in list.iter_mut() {
                    f(Slot::Expression(item));
                }
            }
            Expression::Exists(inner) => f(Slot::Pattern(inner)),
        },
        Shell::Aggregate { args, order_by, .. } => {
            for arg in args.iter_mut() {
                f(Slot::Expression(arg));
            }
            for order in order_by.iter_mut() {
                f(Slot::Expression(order_key(order)));
            }
        }
    }
}

/// The expression a sort key orders by.
fn order_key(order: &mut OrderExpression) -> &mut Expression {
    match order {
        OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => expr,
    }
}

/// `COUNT(*)`: the one aggregate whose argument list may be empty, so the one that
/// can stand in a slot while the aggregate that was there is taken apart and walked,
/// without allocating.
fn count_star() -> AggregateExpression {
    AggregateExpression::new(
        AggregateFunction::Count,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect("COUNT(*) is the spec's one empty-exprlist aggregate")
}

/// [`walk_query`]'s term-position walk.
///
/// A quoted triple's own subject and object are positions too — the pushdown descends
/// into them — so they are numbered as well, subject first, each with everything
/// nested in it before the other. Its predicate is not: neither half of the rewrite
/// writes there. The term is visited BEFORE its nesting is read, so what is descended
/// into is the term as the visitor left it. The walk keeps its own work list, so a
/// deeper nesting needs no more machine stack.
fn walk_term(term: &mut TermPattern, index: &mut u32, visit: &mut dyn FnMut(u32, Cell<'_>)) {
    let mut pending: smallvec::SmallVec<[&mut TermPattern; 8]> = smallvec::smallvec![term];
    while let Some(term) = pending.pop() {
        let here = *index;
        *index += 1;
        visit(here, Cell::Term(&mut *term));
        if let TermPattern::Triple(triple) = term {
            let TriplePattern {
                subject, object, ..
            } = &mut **triple;
            pending.push(object);
            pending.push(subject);
        }
    }
}

/// Every position's contents, in [`walk_query`]'s order.
fn cell_snapshots(query: &mut Query) -> Vec<CellValue> {
    let mut cells = Vec::new();
    walk_query(query, &mut |_, cell| cells.push(cell.snapshot()));
    cells
}

/// Write each target's parameter value into the tree, and report how many positions
/// the traversal found.
///
/// Allocation-free: the traversal builds nothing, and every constant it writes is
/// `Arc<str>`-backed, so a write is a refcount bump.
fn write_values(query: &mut Query, targets: &[Target], probes: &Probes) -> u32 {
    let mut next = 0;
    let count = walk_query(query, &mut |index, cell| {
        // `targets` is sorted by `cell` and holds each ordinal at most once, and
        // `walk_query` yields ordinals in increasing order, so one forward cursor
        // matches them all with no search.
        if let Some(target) = targets.get(next)
            && target.cell == index
        {
            write_cell(cell, &probes[target.parameter as usize].1);
            next += 1;
        }
    });
    assert!(
        next == targets.len(),
        "the substituted plan retained by a prepared execution reached {next} of its {} value \
         positions: the traversal that records a position and the traversal that writes one \
         have diverged, so cells still carrying an earlier run's values would be evaluated as \
         if they carried this one's",
        targets.len()
    );
    count
}

/// A term of the same [`ValueShape`] as `ground` and never equal to it.
///
/// This is how [`PrebindMemo::build`] finds the cells a parameter occupies: rewrite
/// once with the caller's values, rewrite again with ONE parameter moved, and take
/// the positions that moved with it. Derived from the caller's own term rather than
/// minted, so the probe is a term the caller already brought and no vocabulary is
/// invented for it; it is longer than what it derives from, so it is always distinct;
/// and it keeps the term KIND, so the tree it produces has the same shape.
///
/// It never escapes: the trees it appears in are read for their cell positions and
/// dropped inside `build`.
fn moved(ground: &GroundTerm) -> GroundTerm {
    /// One step of the walk: move a term, or assemble a quoted triple from the two
    /// moved components on top of the value stack, under this predicate.
    enum Step<'a> {
        Move(&'a GroundTerm),
        Assemble(&'a NamedNode),
    }
    let mut steps: Vec<Step<'_>> = vec![Step::Move(ground)];
    let mut built: Vec<GroundTerm> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Move(GroundTerm::NamedNode(node)) => {
                built.push(GroundTerm::NamedNode(moved_node(node)));
            }
            Step::Move(GroundTerm::Literal(literal)) => {
                built.push(GroundTerm::Literal(match literal.language() {
                    Some(language) => Literal::new_lang(
                        format!("{}a", literal.value()),
                        language,
                        literal.direction(),
                    ),
                    None => Literal::new_typed(
                        format!("{}a", literal.value()),
                        literal.datatype().clone(),
                    ),
                }));
            }
            Step::Move(GroundTerm::BlankNode(blank)) => {
                built.push(GroundTerm::BlankNode(
                    purrdf_sparql_algebra::BlankNode::new(format!("{}a", blank.as_str())),
                ));
            }
            // Every position moves, so a quoted triple whose writability came from a
            // nested blank keeps it and a fully-writable one stays fully writable. The
            // subject is moved first, then the object.
            Step::Move(GroundTerm::Triple(triple)) => {
                steps.push(Step::Assemble(&triple.predicate));
                steps.push(Step::Move(&triple.object));
                steps.push(Step::Move(&triple.subject));
            }
            Step::Assemble(predicate) => {
                let object = built.pop().expect("a quoted triple's object is moved");
                let subject = built.pop().expect("a quoted triple's subject is moved");
                built.push(GroundTerm::Triple(Child::new(GroundTriple {
                    subject,
                    predicate: moved_node(predicate),
                    object,
                })));
            }
        }
    }
    built
        .pop()
        .expect("the root's moved term is the last one built")
}

/// [`moved`], for an IRI.
fn moved_node(node: &NamedNode) -> NamedNode {
    NamedNode::new_unchecked(format!("{}a", node.as_str()))
}

/// The substituted algebra of one prepared execution, retained across its runs.
#[derive(Debug)]
pub(crate) struct PrebindMemo {
    /// Which of the two rewrites produced [`Self::query`]. The SHACL lane and the
    /// plain lane build DIFFERENT trees from the same query, so a run on the other
    /// lane must not read this one.
    lane: ShaclPrebinding,
    /// The [`ValueShape`] each parameter had when the tree was built, positionally.
    shapes: Box<[ValueShape]>,
    /// The rewritten query, holding the most recently bound values.
    query: Query,
    /// Where those values live, sorted by [`Target::cell`] and each ordinal at most
    /// once.
    targets: Box<[Target]>,
    /// How many positions [`walk_query`] found when the targets were recorded.
    ///
    /// Re-checked on every replay. If a written value ever changed the number of
    /// positions — the hazard [`ValueShape::NestedTriple`] is declined for — every
    /// ordinal after it would shift and the replay would write into the wrong cells.
    /// This turns that into a panic instead of a wrong answer.
    cells: u32,
    /// The numbered tree of [`Self::query`], shared by every run that reads it. A run
    /// only rewrites cells, in place, so the tree's nodes keep their addresses; an
    /// attached expression or `EXISTS` body holding a cell is compiled or prepared per
    /// run, since its constants are the run's values.
    plan: crate::plan::PlanCache,
}

impl PrebindMemo {
    /// The [`ValueShape`] of each probe, as a memo key.
    pub(crate) fn shapes_of(probes: &Probes) -> Box<[ValueShape]> {
        probes
            .iter()
            .map(|(_, ground)| ValueShape::of(ground))
            .collect()
    }

    /// Whether `shapes` is the shape list `probes` would produce, without building
    /// one to find out. Called on every run, so it allocates nothing.
    pub(crate) fn shapes_match(shapes: &[ValueShape], probes: &Probes) -> bool {
        shapes.len() == probes.len()
            && shapes
                .iter()
                .zip(probes)
                .all(|(shape, (_, ground))| *shape == ValueShape::of(ground))
    }

    /// Whether this memo answers for `lane` and `probes`.
    pub(crate) fn matches(&self, lane: ShaclPrebinding, probes: &Probes) -> bool {
        self.lane == lane && Self::shapes_match(&self.shapes, probes)
    }

    /// The retained tree, with `probes`' values written into it.
    ///
    /// # Panics
    ///
    /// If the traversal no longer reaches every recorded position, or reaches a
    /// different number of positions than it did when they were recorded. Both mean
    /// the retained tree holds cells from an earlier run that this run's values were
    /// meant to replace, which is a wrong answer rather than a slow one.
    pub(crate) fn bind(&mut self, probes: &Probes) -> (&Query, &crate::plan::PlanCache) {
        debug_assert!(
            Self::shapes_match(&self.shapes, probes),
            "a memo is only ever bound after `matches` agreed to it"
        );
        let cells = write_values(&mut self.query, &self.targets, probes);
        assert!(
            cells == self.cells,
            "the substituted plan retained by a prepared execution now has {cells} value \
             positions where it had {}: a bound value has changed the SHAPE of the tree rather \
             than only its cells, so every position after it is misnumbered",
            self.cells
        );
        (&self.query, &self.plan)
    }

    /// The tree as it stands, for the run that built it.
    pub(crate) fn query(&self) -> &Query {
        &self.query
    }

    /// The plan cache kept for [`Self::query`].
    pub(crate) const fn plan(&self) -> &crate::plan::PlanCache {
        &self.plan
    }

    /// Build a memo for `prepared` under `lane` and the shapes of `probes`, or `None`
    /// if this query, lane and shape list are outside what a memo can represent.
    ///
    /// # How the targets are found
    ///
    /// By **observation of the real rewrite**, never by a second reading of its rules.
    /// One tree is built from `probes`; then, for each parameter in turn, another is
    /// built from `probes` with that one parameter's value [`moved`]. The positions
    /// whose contents differ are that parameter's, because a position the rewrite
    /// does not write cannot differ and a position it writes with that value must.
    /// Varying one parameter at a time is what makes the attribution unambiguous —
    /// with every value moved at once, two parameters bound to the SAME term (a
    /// SHACL `$this` and `$value` on a self-referential path, say) could not be told
    /// apart.
    ///
    /// # What makes it safe to use afterwards
    ///
    /// The last step replays the all-moved value set into a copy of the first tree
    /// and compares it against the tree the real rewrite builds for that same value
    /// set. They must be equal node for node. That is the check that closes the gap
    /// the differencing cannot close on its own: if this module's traversal failed to
    /// VISIT some position the rewrite writes, no amount of differencing would ever
    /// have noticed, but the replayed tree would carry the first value set's term
    /// there and the comparison fails. A memo that fails it is discarded.
    pub(crate) fn build(prepared: &Query, lane: ShaclPrebinding, probes: &Probes) -> Option<Self> {
        // A repeated pre-bound name keeps `apply_probes`' own per-variable path, which
        // builds a different tree; nothing on the prepared door can produce one
        // (`prepare_execution` refuses a repeated parameter), and a memo that assumed
        // the combined-seed shape for it would be describing the wrong rewrite.
        if has_repeated_variable(probes) {
            return None;
        }
        let shapes = Self::shapes_of(probes);
        if shapes.contains(&ValueShape::NestedTriple) {
            return None;
        }

        let mut base = rewrite(prepared.clone(), lane, probes.to_vec());
        let base_cells = cell_snapshots(&mut base);

        let moved_probes: Vec<(Variable, GroundTerm)> = probes
            .iter()
            .map(|(variable, ground)| (variable.clone(), moved(ground)))
            .collect();

        let mut targets = Vec::new();
        for (parameter, (_, ground)) in moved_probes.iter().enumerate() {
            let mut one_moved = probes.to_vec();
            one_moved[parameter].1 = ground.clone();
            let mut tree = rewrite(prepared.clone(), lane, one_moved);
            let cells = cell_snapshots(&mut tree);
            if cells.len() != base_cells.len() {
                return None;
            }
            for (cell, (here, there)) in base_cells.iter().zip(&cells).enumerate() {
                // A changed ANCESTOR of the real write site changes too — its
                // snapshot is the whole subtree — but it is never itself a
                // position the rewrite overwrites, only one it recurses through.
                // See [`CellValue::is_leaf`].
                if here != there && here.is_leaf() {
                    targets.push(Target {
                        cell: u32::try_from(cell).ok()?,
                        parameter: u32::try_from(parameter).ok()?,
                    });
                }
            }
        }
        targets.sort_unstable_by_key(|target| target.cell);
        // One position cannot hold two parameters' values. If two parameters both
        // claim one, the attribution is ambiguous and the memo would write whichever
        // of them the cursor reached first.
        if targets.windows(2).any(|pair| pair[0].cell == pair[1].cell) {
            return None;
        }

        let mut replayed = base.clone();
        let cells = write_values(&mut replayed, &targets, &moved_probes);
        if replayed != rewrite(prepared.clone(), lane, moved_probes) {
            return None;
        }
        // `replayed` holds a different value in every cell `base` does, so an attached
        // expression or `EXISTS` body the two disagree on is one a run rewrites.
        let plan = crate::plan::PlanCache::varying(
            crate::eval::query_pattern(&base),
            crate::eval::query_pattern(&replayed),
        );
        Some(Self {
            lane,
            shapes,
            query: base,
            targets: targets.into_boxed_slice(),
            cells,
            plan,
        })
    }
}

#[cfg(test)]
mod tests;

/// The loop-based walks checked against their recursive forms: [`walk_query`]'s cell
/// numbering and its write-back over generated queries — every pattern and expression
/// variant, nested `EXISTS`, quoted triples, aggregates with their own sort keys — and
/// [`moved`] over generated ground terms; and one query whose `FILTER` nests a hundred
/// thousand `!` operators, walked on a thread with a 128 KiB stack.
#[cfg(test)]
mod iterative_walk_tests {
    use purrdf_sparql_algebra::{
        AggregateExpression, AggregateFunction, Args, ArithmeticOperator, BlankNode, Chain, Child,
        Expression, Function, GraphPattern, GroundTerm, GroundTriple, Literal, NamedNode,
        NamedNodePattern, NonEmpty, OrderExpression, PropertyFunctionCall, PropertyPathExpression,
        Query, QueryDataset, TermPattern, TriplePattern, Variable,
    };

    use super::{Cell, CellValue, count_star, moved, moved_node, walk_query};
    use crate::test_rng::splitmix64_next;

    const EX: &str = "http://example.org/";

    /// The stack the deep case runs on.
    const SMALL_STACK: usize = 128 * 1024;

    /// How many `!` operators the deep case nests.
    const DEPTH: usize = 100_000;

    // ── The recursive references ────────────────────────────────────────────────────

    /// [`walk_query`], written as the recursion it replaces.
    fn walk_query_ref(query: &mut Query, visit: &mut dyn FnMut(u32, Cell<'_>)) -> u32 {
        let mut index = 0;
        match query {
            Query::Select { pattern, .. }
            | Query::Construct { pattern, .. }
            | Query::Describe { pattern, .. }
            | Query::Ask { pattern, .. } => walk_pattern_ref(pattern, &mut index, visit),
        }
        index
    }

    fn walk_pattern_ref(
        pattern: &mut GraphPattern,
        index: &mut u32,
        visit: &mut dyn FnMut(u32, Cell<'_>),
    ) {
        match pattern {
            GraphPattern::Bgp { patterns } => {
                for triple in patterns.iter_mut() {
                    walk_term_ref(&mut triple.subject, index, visit);
                    walk_term_ref(&mut triple.object, index, visit);
                }
            }
            GraphPattern::Path {
                subject, object, ..
            } => {
                walk_term_ref(subject, index, visit);
                walk_term_ref(object, index, visit);
            }
            GraphPattern::Values { bindings, .. } => {
                for row in bindings.iter_mut() {
                    for slot in row.iter_mut() {
                        let here = *index;
                        *index += 1;
                        visit(here, Cell::Ground(slot));
                    }
                }
            }
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Minus { left, right } => {
                walk_pattern_ref(left, index, visit);
                walk_pattern_ref(right, index, visit);
            }
            GraphPattern::Union { arms } => {
                for arm in arms.iter_mut() {
                    walk_pattern_ref(arm, index, visit);
                }
            }
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                walk_pattern_ref(left, index, visit);
                walk_pattern_ref(right, index, visit);
                if let Some(expression) = expression {
                    walk_expression_ref(expression, index, visit);
                }
            }
            GraphPattern::Filter { expr, inner } => {
                walk_expression_ref(expr, index, visit);
                walk_pattern_ref(inner, index, visit);
            }
            GraphPattern::Graph { name, inner } | GraphPattern::Service { name, inner, .. } => {
                let here = *index;
                *index += 1;
                visit(here, Cell::GraphName(name));
                walk_pattern_ref(inner, index, visit);
            }
            GraphPattern::Extend {
                inner, expression, ..
            }
            | GraphPattern::Unfold {
                inner, expression, ..
            } => {
                walk_pattern_ref(inner, index, visit);
                walk_expression_ref(expression, index, visit);
            }
            GraphPattern::OrderBy { inner, expression } => {
                walk_pattern_ref(inner, index, visit);
                for order in expression.iter_mut() {
                    walk_order_ref(order, index, visit);
                }
            }
            GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => walk_pattern_ref(inner, index, visit),
            GraphPattern::PropertyFunction(call) => {
                for term in call
                    .subject_args
                    .iter_mut()
                    .chain(call.object_args.iter_mut())
                {
                    walk_term_ref(term, index, visit);
                }
            }
            GraphPattern::Group {
                inner, aggregates, ..
            } => {
                walk_pattern_ref(inner, index, visit);
                walk_aggregates_ref(aggregates, index, visit);
            }
        }
    }

    fn walk_aggregates_ref(
        aggregates: &mut Vec<(Variable, AggregateExpression)>,
        index: &mut u32,
        visit: &mut dyn FnMut(u32, Cell<'_>),
    ) {
        let mut taken = std::mem::take(aggregates);
        for entry in &mut taken {
            let (function, mut args, scalarvals, mut order_by, distinct) =
                std::mem::replace(&mut entry.1, count_star()).into_parts();
            for arg in &mut args {
                walk_expression_ref(arg, index, visit);
            }
            for order in &mut order_by {
                walk_order_ref(order, index, visit);
            }
            entry.1 = AggregateExpression::new(function, args, scalarvals, order_by, distinct)
                .expect("visiting an argument changes no argument count");
        }
        *aggregates = taken;
    }

    fn walk_order_ref(
        order: &mut OrderExpression,
        index: &mut u32,
        visit: &mut dyn FnMut(u32, Cell<'_>),
    ) {
        match order {
            OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
                walk_expression_ref(expr, index, visit);
            }
        }
    }

    fn walk_term_ref(
        term: &mut TermPattern,
        index: &mut u32,
        visit: &mut dyn FnMut(u32, Cell<'_>),
    ) {
        let here = *index;
        *index += 1;
        visit(here, Cell::Term(term));
        if let TermPattern::Triple(triple) = term {
            walk_term_ref(&mut triple.subject, index, visit);
            walk_term_ref(&mut triple.object, index, visit);
        }
    }

    fn walk_expression_ref(
        expr: &mut Expression,
        index: &mut u32,
        visit: &mut dyn FnMut(u32, Cell<'_>),
    ) {
        let here = *index;
        *index += 1;
        visit(here, Cell::Expr(expr));
        match expr {
            Expression::Variable(_)
            | Expression::Bound(_)
            | Expression::NamedNode(_)
            | Expression::Literal(_) => {}
            Expression::Or(operands) | Expression::And(operands) => {
                for operand in operands.iter_mut() {
                    walk_expression_ref(operand, index, visit);
                }
            }
            Expression::Arithmetic(first, steps) => {
                walk_expression_ref(first, index, visit);
                for (_, operand) in steps.iter_mut() {
                    walk_expression_ref(operand, index, visit);
                }
            }
            Expression::Equal(left, right)
            | Expression::SameTerm(left, right)
            | Expression::Greater(left, right)
            | Expression::GreaterOrEqual(left, right)
            | Expression::Less(left, right)
            | Expression::LessOrEqual(left, right) => {
                walk_expression_ref(left, index, visit);
                walk_expression_ref(right, index, visit);
            }
            Expression::UnaryPlus(inner)
            | Expression::UnaryMinus(inner)
            | Expression::Not(inner) => {
                walk_expression_ref(inner, index, visit);
            }
            Expression::In(target, list) => {
                walk_expression_ref(target, index, visit);
                for item in list.iter_mut() {
                    walk_expression_ref(item, index, visit);
                }
            }
            Expression::If(cond, then_expr, else_expr) => {
                walk_expression_ref(cond, index, visit);
                walk_expression_ref(then_expr, index, visit);
                walk_expression_ref(else_expr, index, visit);
            }
            Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
                for item in list.iter_mut() {
                    walk_expression_ref(item, index, visit);
                }
            }
            Expression::Exists(inner) => walk_pattern_ref(inner, index, visit),
        }
    }

    /// [`moved`], written as the recursion it replaces.
    fn moved_ref(ground: &GroundTerm) -> GroundTerm {
        match ground {
            GroundTerm::NamedNode(node) => GroundTerm::NamedNode(moved_node(node)),
            GroundTerm::Literal(literal) => GroundTerm::Literal(match literal.language() {
                Some(language) => Literal::new_lang(
                    format!("{}a", literal.value()),
                    language,
                    literal.direction(),
                ),
                None => {
                    Literal::new_typed(format!("{}a", literal.value()), literal.datatype().clone())
                }
            }),
            GroundTerm::BlankNode(blank) => {
                GroundTerm::BlankNode(BlankNode::new(format!("{}a", blank.as_str())))
            }
            GroundTerm::Triple(triple) => GroundTerm::Triple(Child::new(GroundTriple {
                subject: moved_ref(&triple.subject),
                predicate: moved_node(&triple.predicate),
                object: moved_ref(&triple.object),
            })),
        }
    }

    // ── A deterministic choice sequence ────────────────────────────────────────────

    /// The choices one generated shape is built from: a SplitMix64 counter stream, so
    /// a seed names a shape.
    struct Choices(u64);

    impl Choices {
        /// One choice in `0..bound`.
        fn pick(&mut self, bound: u64) -> u64 {
            splitmix64_next(&mut self.0) % bound
        }

        /// One even choice.
        fn coin(&mut self) -> bool {
            self.pick(2) == 1
        }
    }

    fn nn(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("{EX}{local}"))
    }

    fn var(name: &str) -> Variable {
        Variable::new(name)
    }

    /// One ground term, nesting quoted triples while `depth` allows.
    fn gen_ground(choices: &mut Choices, depth: usize) -> GroundTerm {
        match choices.pick(if depth == 0 { 4 } else { 5 }) {
            0 => GroundTerm::NamedNode(nn(&format!("g{}", choices.pick(3)))),
            1 => GroundTerm::Literal(Literal::new_simple(format!("l{}", choices.pick(3)))),
            2 => GroundTerm::Literal(Literal::new_lang(
                format!("t{}", choices.pick(3)),
                "en",
                None,
            )),
            3 => GroundTerm::BlankNode(BlankNode::new(format!("b{}", choices.pick(3)))),
            _ => GroundTerm::Triple(Child::new(GroundTriple {
                subject: gen_ground(choices, depth - 1),
                predicate: nn("gp"),
                object: gen_ground(choices, depth - 1),
            })),
        }
    }

    /// One term position, nesting quoted triples while `depth` allows.
    fn gen_term(choices: &mut Choices, depth: usize) -> TermPattern {
        match choices.pick(if depth == 0 { 4 } else { 5 }) {
            0 => TermPattern::Variable(var(&format!("v{}", choices.pick(4)))),
            1 => TermPattern::NamedNode(nn(&format!("n{}", choices.pick(3)))),
            2 => TermPattern::Literal(Literal::new_simple(format!("l{}", choices.pick(3)))),
            3 => TermPattern::BlankNode(BlankNode::new(format!("b{}", choices.pick(3)))),
            _ => TermPattern::Triple(Child::new(TriplePattern {
                subject: gen_term(choices, depth - 1),
                predicate: gen_predicate(choices),
                object: gen_term(choices, depth - 1),
            })),
        }
    }

    fn gen_predicate(choices: &mut Choices) -> NamedNodePattern {
        if choices.coin() {
            NamedNodePattern::Variable(var("p"))
        } else {
            NamedNodePattern::NamedNode(nn("p"))
        }
    }

    /// One expression, at most `budget` operators over its leaves.
    fn gen_expression(choices: &mut Choices, budget: &mut usize) -> Expression {
        if *budget == 0 {
            return match choices.pick(4) {
                0 => Expression::Variable(var(&format!("v{}", choices.pick(4)))),
                1 => Expression::Bound(var(&format!("v{}", choices.pick(4)))),
                2 => Expression::NamedNode(nn("c")),
                _ => Expression::Literal(Literal::new_simple("k")),
            };
        }
        *budget -= 1;
        match choices.pick(11) {
            0 => Expression::Or(Chain::new(
                gen_expression(choices, budget),
                gen_expression(choices, budget),
                [],
            )),
            1 => Expression::And(Chain::new(
                gen_expression(choices, budget),
                gen_expression(choices, budget),
                [gen_expression(choices, budget)],
            )),
            2 => Expression::Arithmetic(
                Child::new(gen_expression(choices, budget)),
                NonEmpty::from_parts(
                    (ArithmeticOperator::Add, gen_expression(choices, budget)),
                    [(
                        ArithmeticOperator::Multiply,
                        gen_expression(choices, budget),
                    )],
                ),
            ),
            3 => Expression::Equal(
                Child::new(gen_expression(choices, budget)),
                Child::new(gen_expression(choices, budget)),
            ),
            4 => Expression::Not(Child::new(gen_expression(choices, budget))),
            5 => Expression::In(
                Child::new(gen_expression(choices, budget)),
                Args::from(vec![
                    gen_expression(choices, budget),
                    gen_expression(choices, budget),
                ]),
            ),
            6 => Expression::If(
                Child::new(gen_expression(choices, budget)),
                Child::new(gen_expression(choices, budget)),
                Child::new(gen_expression(choices, budget)),
            ),
            7 => Expression::Coalesce(Args::from(vec![gen_expression(choices, budget)])),
            8 => Expression::FunctionCall(
                Function::Str,
                Args::from(vec![gen_expression(choices, budget)]),
            ),
            9 => Expression::Exists(Child::new(gen_pattern(choices, budget))),
            _ => Expression::UnaryMinus(Child::new(gen_expression(choices, budget))),
        }
    }

    /// One aggregate over generated expressions: a `COUNT(*)`, a `SUM`, or a `FOLD`
    /// with its own sort key.
    fn gen_aggregate(choices: &mut Choices, budget: &mut usize) -> AggregateExpression {
        match choices.pick(3) {
            0 => count_star(),
            1 => AggregateExpression::new(
                AggregateFunction::Sum,
                vec![gen_expression(choices, budget)],
                Vec::new(),
                Vec::new(),
                choices.coin(),
            )
            .expect("SUM takes one argument"),
            _ => AggregateExpression::new(
                AggregateFunction::Fold,
                vec![gen_expression(choices, budget)],
                Vec::new(),
                vec![
                    OrderExpression::Desc(gen_expression(choices, budget)),
                    OrderExpression::Asc(gen_expression(choices, budget)),
                ],
                false,
            )
            .expect("FOLD takes one argument and its own sort keys"),
        }
    }

    /// A leaf pattern: a basic graph pattern, a path, a `VALUES` block, or a call.
    fn gen_leaf(choices: &mut Choices) -> GraphPattern {
        match choices.pick(4) {
            0 => GraphPattern::Bgp {
                patterns: (0..choices.pick(3))
                    .map(|_| TriplePattern {
                        subject: gen_term(choices, 2),
                        predicate: gen_predicate(choices),
                        object: gen_term(choices, 2),
                    })
                    .collect(),
            },
            1 => GraphPattern::Path {
                subject: gen_term(choices, 1),
                path: PropertyPathExpression::NamedNode(nn("q")),
                object: gen_term(choices, 1),
            },
            2 => GraphPattern::Values {
                variables: vec![var("v0"), var("v1")],
                bindings: (0..choices.pick(3))
                    .map(|_| {
                        vec![
                            choices.coin().then(|| gen_ground(choices, 1)),
                            choices.coin().then(|| gen_ground(choices, 1)),
                        ]
                    })
                    .collect(),
            },
            _ => GraphPattern::PropertyFunction(PropertyFunctionCall {
                iri: format!("{EX}rel"),
                subject_args: vec![gen_term(choices, 1)],
                object_args: vec![gen_term(choices, 1), gen_term(choices, 1)],
            }),
        }
    }

    /// One generated pattern, at most `budget` operators over its leaves.
    fn gen_pattern(choices: &mut Choices, budget: &mut usize) -> GraphPattern {
        if *budget == 0 {
            return gen_leaf(choices);
        }
        *budget -= 1;
        match choices.pick(17) {
            0 => GraphPattern::Join {
                left: Child::new(gen_pattern(choices, budget)),
                right: Child::new(gen_pattern(choices, budget)),
            },
            1 => GraphPattern::Lateral {
                left: Child::new(gen_pattern(choices, budget)),
                right: Child::new(gen_pattern(choices, budget)),
            },
            2 => GraphPattern::Minus {
                left: Child::new(gen_pattern(choices, budget)),
                right: Child::new(gen_pattern(choices, budget)),
            },
            3 => GraphPattern::Union {
                arms: Chain::new(
                    gen_pattern(choices, budget),
                    gen_pattern(choices, budget),
                    [],
                ),
            },
            4 => GraphPattern::LeftJoin {
                left: Child::new(gen_pattern(choices, budget)),
                right: Child::new(gen_pattern(choices, budget)),
                expression: choices.coin().then(|| gen_expression(choices, budget)),
            },
            5 => GraphPattern::Filter {
                expr: gen_expression(choices, budget),
                inner: Child::new(gen_pattern(choices, budget)),
            },
            6 => GraphPattern::Graph {
                name: gen_predicate(choices),
                inner: Child::new(gen_pattern(choices, budget)),
            },
            7 => GraphPattern::Service {
                name: gen_predicate(choices),
                inner: Child::new(gen_pattern(choices, budget)),
                silent: choices.coin(),
            },
            8 => GraphPattern::Extend {
                inner: Child::new(gen_pattern(choices, budget)),
                variable: var("x"),
                expression: gen_expression(choices, budget),
            },
            9 => GraphPattern::Unfold {
                inner: Child::new(gen_pattern(choices, budget)),
                expression: gen_expression(choices, budget),
                element: var("e"),
                companion: choices.coin().then(|| var("i")),
            },
            10 => GraphPattern::OrderBy {
                inner: Child::new(gen_pattern(choices, budget)),
                expression: (0..choices.pick(3))
                    .map(|_| {
                        if choices.coin() {
                            OrderExpression::Asc(gen_expression(choices, budget))
                        } else {
                            OrderExpression::Desc(gen_expression(choices, budget))
                        }
                    })
                    .collect(),
            },
            11 => GraphPattern::Project {
                inner: Child::new(gen_pattern(choices, budget)),
                variables: vec![var("v0")],
            },
            12 => GraphPattern::Distinct {
                inner: Child::new(gen_pattern(choices, budget)),
            },
            13 => GraphPattern::Reduced {
                inner: Child::new(gen_pattern(choices, budget)),
            },
            14 => GraphPattern::Slice {
                inner: Child::new(gen_pattern(choices, budget)),
                start: 1,
                length: Some(2),
            },
            15 => GraphPattern::Group {
                inner: Child::new(gen_pattern(choices, budget)),
                variables: vec![var("v0")],
                aggregates: (0..choices.pick(3))
                    .map(|n| (var(&format!("a{n}")), gen_aggregate(choices, budget)))
                    .collect(),
            },
            _ => gen_leaf(choices),
        }
    }

    /// One generated `SELECT` query.
    fn gen_query(seed: u64) -> Query {
        let mut choices = Choices(seed);
        let mut budget = 2 + choices.pick(9) as usize;
        Query::Select {
            pattern: gen_pattern(&mut choices, &mut budget),
            dataset: QueryDataset::default(),
            base_iri: None,
            version: None,
        }
    }

    /// A whole-query walk: the loop, or the recursion it is checked against.
    type Walk = fn(&mut Query, &mut dyn FnMut(u32, Cell<'_>)) -> u32;

    /// Every cell of `query` in numbering order, as `(ordinal, contents)`, by `walk`.
    fn numbered(query: &mut Query, walk: Walk) -> (u32, Vec<(u32, CellValue)>) {
        let mut cells = Vec::new();
        let count = walk(query, &mut |index, cell| {
            cells.push((index, cell.snapshot()));
        });
        (count, cells)
    }

    /// A visitor that writes into every kind of cell a rewrite writes: a term variable
    /// becomes an IRI, an `UNDEF` cell a value, a variable expression a literal, a
    /// variable graph name an IRI.
    fn write_every_cell(_: u32, cell: Cell<'_>) {
        match cell {
            Cell::Term(term) => {
                if matches!(term, TermPattern::Variable(_)) {
                    *term = TermPattern::NamedNode(nn("written"));
                }
            }
            Cell::Ground(slot) => {
                if slot.is_none() {
                    *slot = Some(GroundTerm::NamedNode(nn("written")));
                }
            }
            Cell::Expr(expr) => {
                if matches!(expr, Expression::Variable(_) | Expression::Bound(_)) {
                    *expr = Expression::Literal(Literal::new_simple("written"));
                }
            }
            Cell::GraphName(name) => {
                if matches!(name, NamedNodePattern::Variable(_)) {
                    *name = NamedNodePattern::NamedNode(nn("written"));
                }
            }
        }
    }

    /// Run `body` on a fresh thread with [`SMALL_STACK`] of stack.
    fn on_small_stack<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(body)
            .expect("spawn")
            .join()
            .expect("the 128 KiB thread returned")
    }

    // ── The checks ─────────────────────────────────────────────────────────────────

    /// Over two hundred generated queries, the loop numbers the same cells in the same
    /// order as the recursion, reports the same count, and hands the tree back
    /// unchanged.
    #[test]
    fn the_loop_numbers_the_cells_the_recursion_numbers() {
        for seed in 0..200_u64 {
            let original = gen_query(seed);
            let mut looped = original.clone();
            let mut recursed = original.clone();
            let by_loop = numbered(&mut looped, walk_query);
            let by_recursion = numbered(&mut recursed, walk_query_ref);
            assert_eq!(by_loop, by_recursion, "seed {seed}: {original:?}");
            assert_eq!(
                by_loop.0 as usize,
                by_loop.1.len(),
                "seed {seed}: one ordinal per cell"
            );
            assert_eq!(
                looped, original,
                "seed {seed}: a reading walk changes nothing"
            );
            assert_eq!(recursed, original, "seed {seed}");
        }
    }

    /// Over the same queries, a visitor that writes into every cell leaves the loop's
    /// tree equal to the recursion's, node for node.
    #[test]
    fn the_loop_writes_back_what_the_recursion_writes() {
        for seed in 0..200_u64 {
            let original = gen_query(seed);
            let mut looped = original.clone();
            let mut recursed = original.clone();
            let looped_count = walk_query(&mut looped, &mut write_every_cell);
            let recursed_count = walk_query_ref(&mut recursed, &mut write_every_cell);
            assert_eq!(looped_count, recursed_count, "seed {seed}");
            assert_eq!(looped, recursed, "seed {seed}: {original:?}");
        }
    }

    /// Over generated ground terms, quoted triples nested inside, the loop moves what
    /// the recursion moves.
    #[test]
    fn the_loop_moves_what_the_recursion_moves() {
        for seed in 0..200_u64 {
            let mut choices = Choices(seed);
            let ground = gen_ground(&mut choices, 4);
            assert_eq!(
                moved(&ground),
                moved_ref(&ground),
                "seed {seed}: {ground:?}"
            );
        }
    }

    /// A `FILTER` nesting a hundred thousand `!` operators over `?v` is numbered on a
    /// thread whose stack holds a few hundred frames of any recursion: one cell per
    /// operator, one for the variable under them, then the two term positions of the
    /// one triple pattern the filter reads — and a writing walk over the same query
    /// leaves the variable under the operators written and the operators in place.
    #[test]
    fn a_hundred_thousand_nested_operators_are_numbered_on_a_128_kib_thread() {
        on_small_stack(|| {
            let mut expression = Expression::Variable(var("v"));
            for _ in 0..DEPTH {
                expression = Expression::Not(Child::new(expression));
            }
            let mut query = Query::Select {
                pattern: GraphPattern::Filter {
                    expr: expression,
                    inner: Child::new(GraphPattern::Bgp {
                        patterns: vec![TriplePattern {
                            subject: TermPattern::Variable(var("s")),
                            predicate: NamedNodePattern::NamedNode(nn("p")),
                            object: TermPattern::Variable(var("o")),
                        }],
                    }),
                },
                dataset: QueryDataset::default(),
                base_iri: None,
                version: None,
            };
            let expected = u32::try_from(DEPTH + 1 + 2).expect("fits");

            let mut seen = 0_u32;
            let counted = walk_query(&mut query, &mut |index, _| {
                assert_eq!(index, seen, "ordinals run in order");
                seen += 1;
            });
            assert_eq!(counted, expected);
            assert_eq!(seen, expected);

            let written = walk_query(&mut query, &mut write_every_cell);
            assert_eq!(written, expected);
            let Query::Select {
                pattern: GraphPattern::Filter { expr, inner },
                ..
            } = &query
            else {
                unreachable!("the query keeps its shape");
            };
            let mut levels = 0_usize;
            let mut at = expr;
            while let Expression::Not(inner) = at {
                levels += 1;
                at = inner;
            }
            assert_eq!(levels, DEPTH);
            assert_eq!(at, &Expression::Literal(Literal::new_simple("written")));
            assert_eq!(
                &**inner,
                &GraphPattern::Bgp {
                    patterns: vec![TriplePattern {
                        subject: TermPattern::NamedNode(nn("written")),
                        predicate: NamedNodePattern::NamedNode(nn("p")),
                        object: TermPattern::NamedNode(nn("written")),
                    }],
                }
            );
        });
    }
}
